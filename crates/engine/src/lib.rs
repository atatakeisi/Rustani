//! The GridCraft engine façade.
//!
//! Every user-visible action is a command with a stable id (`cell.set`, `home.bold`,
//! `data.sortAscending`…) and JSON parameters. The egui UI, the CLI, the control channel and MCP
//! all go through [`Session::execute`], so every action is scriptable and journaled.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

pub mod catalog;
pub mod cf;
mod chart_svg;
mod clip_html;
pub mod cmd;
pub mod display;
pub mod fill;
pub mod io;
pub mod links;
pub mod outline;
pub mod paper;
pub mod pivot;
pub mod pivot_calc;
pub mod sample;
pub mod selection;
pub mod tables;
pub mod vba;

use std::sync::Arc;

use gridcraft_calc::Calc;
use gridcraft_core::{CellRef, RangeRef};
use gridcraft_model::Workbook;
use serde::Serialize;
use serde_json::Value as Json;

pub use cmd::{CommandInfo, CommandSpec, command_specs, find_command, register_test_command};
pub use gridcraft_calc as calc;
pub use gridcraft_core as core;
pub use gridcraft_formula as formula;
pub use gridcraft_model as model;
pub use gridcraft_numfmt as numfmt;
pub use selection::Selection;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("unknown command `{0}`")]
    UnknownCommand(String),
    #[error("command `{0}` is not available right now: {1}")]
    Disabled(String, String),
    #[error("invalid parameters for `{cmd}`: {msg}")]
    BadParams { cmd: String, msg: String },
    #[error("no workbook open")]
    NoDocument,
    #[error("{0}")]
    Other(String),
    #[error("internal error in `{0}` (the workbook was kept as it was): {1}")]
    Internal(String, String),
}

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub label: String,
    pub wb: Arc<Workbook>,
    pub selection: Selection,
    pub selected_sheets: Vec<usize>,
    pub sheet_selection_anchor: usize,
    /// Estimate of storage this snapshot retains beyond the next workbook version.
    pub retained_bytes: usize,
}

/// The clipboard: a block of cells (with formulas and styles) or marching-ants reference.
#[derive(Clone, Debug)]
pub struct Clipboard {
    /// Source workbook snapshot, sheet and range.
    pub wb: Arc<Workbook>,
    pub sheet: usize,
    pub range: RangeRef,
    /// The range as copied, before whole rows/columns were trimmed to the used range
    /// (Insert Copied Cells inserts whole lines for a row or column copy).
    pub selected: RangeRef,
    pub cut: bool,
    /// Document uid of the source (a cut only moves within the same workbook).
    pub doc_uid: u64,
    /// Plain text (tab-separated) for the system clipboard.
    pub text: String,
    /// A multi-area copy: the areas as selected (for the marquee). `wb` then holds them slid
    /// together into one block at `range`. Empty for a single-range copy.
    pub areas: Vec<RangeRef>,
}

/// In-cell edit mode as shown in the status bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Mode {
    #[default]
    Ready,
    Enter,
    Edit,
    Point,
}

#[derive(Clone, Debug)]
pub struct DocState {
    pub wb: Arc<Workbook>,
    pub calc: Calc,
    pub selection: Selection,
    pub undo: Vec<HistoryEntry>,
    pub redo: Vec<HistoryEntry>,
    pub path: Option<String>,
    pub saved: Arc<Workbook>,
    pub revision: u64,
    pub uid: u64,
    pub title: String,
    /// Per-sheet selection memory.
    pub sheet_selections: Vec<Selection>,
    /// Visible worksheet indices selected as a group, including the active worksheet.
    pub selected_sheets: Vec<usize>,
    /// Fixed starting worksheet for Shift-click tab selection.
    pub sheet_selection_anchor: usize,
    /// Allow Edit Ranges unlocked with their password in this session: (sheet name, title).
    pub unlocked_ranges: Vec<(String, String)>,
    /// The format `path`'s content was recognised as when its extension doesn't name one (an
    /// XLSX called `budget.dat`): Save writes it back in that format.
    pub content_kind: Option<io::FileKind>,
    /// Whether the scripts stored in this workbook may run (`automate.runScript`). A workbook
    /// that arrives already carrying scripts (opened from a file) starts untrusted until the
    /// user trusts it in this session (`automate.trustScripts`); one with no scripts starts
    /// trusted, since any script it gains later was written here.
    pub scripts_trusted: bool,
}

static NEXT_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
const UNDO_LIMIT: usize = 100;
/// Past this many entries the journal drops its oldest half.
const JOURNAL_MAX_ENTRIES: usize = 10_000;
/// Past this many bytes (entries serialized as JSON) the journal drops its oldest entries until
/// it is under half. Entries keep their full params so they replay (recorded scripts,
/// `engine.journal`), and a `file.open` holds the whole file as base64: without a size cap a
/// few opens of a large workbook keep tens of MB alive.
pub(crate) const JOURNAL_MAX_BYTES: usize = 64 * 1024 * 1024;
const UNDO_BYTES_LIMIT: usize = 128 * 1024 * 1024;

fn history_retained_bytes(older: &Workbook, newer: &Workbook) -> usize {
    let mut bytes = std::mem::size_of::<Workbook>();
    for (index, sheet) in older.sheets.iter().enumerate() {
        // Moving or inserting sheets must not charge unchanged sheets at a new index.
        if newer.sheets.iter().any(|next| Arc::ptr_eq(sheet, next)) {
            continue;
        }
        bytes = bytes.saturating_add(std::mem::size_of::<gridcraft_model::Sheet>());
        let empty = gridcraft_model::CellStore::new();
        let next = newer.sheets.get(index).map_or(&empty, |next| &next.cells);
        bytes = bytes.saturating_add(sheet.cells.retained_bytes_against(next));
    }
    bytes
}

fn push_history(history: &mut Vec<HistoryEntry>, label: String, wb: Arc<Workbook>, next: &Workbook, selection: Selection) {
    push_history_with_budget(history, label, wb, next, selection, UNDO_BYTES_LIMIT);
}

fn push_history_with_budget(
    history: &mut Vec<HistoryEntry>,
    label: String,
    wb: Arc<Workbook>,
    next: &Workbook,
    selection: Selection,
    byte_budget: usize,
) {
    let retained_bytes = history_retained_bytes(&wb, next);
    let active = wb.active_sheet;
    history.push(HistoryEntry { label, wb, selection, selected_sheets: vec![active], sheet_selection_anchor: active, retained_bytes });
    let mut total = history.iter().fold(0usize, |sum, entry| sum.saturating_add(entry.retained_bytes));
    // Always preserve the latest action, even if that one edit exceeds the budget.
    while history.len() > 1 && (history.len() > UNDO_LIMIT || total > byte_budget) {
        total = total.saturating_sub(history.remove(0).retained_bytes);
    }
}

impl DocState {
    pub fn new(mut wb: Workbook, path: Option<String>, title: String) -> DocState {
        let mut calc = Calc::new();
        calc.recalc_all(&mut wb);
        let scripts_trusted = !cmd::extra::has_scripts(&wb) && !vba::has_modules(&wb) && wb.vba_project.is_none();
        let wb = Arc::new(wb);
        let sel = Selection::at(wb.active().map(|s| s.view_active).unwrap_or_default());
        let active_sheet = wb.active_sheet;
        DocState {
            saved: wb.clone(),
            wb,
            calc,
            selection: sel,
            undo: vec![],
            redo: vec![],
            path,
            revision: 1,
            uid: NEXT_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            title,
            sheet_selections: vec![],
            selected_sheets: vec![active_sheet],
            sheet_selection_anchor: active_sheet,
            unlocked_ranges: vec![],
            content_kind: None,
            scripts_trusted,
        }
    }
    pub fn is_dirty(&self) -> bool {
        !Arc::ptr_eq(&self.wb, &self.saved)
    }
    pub fn sheet_index(&self) -> usize {
        self.wb.active_sheet
    }
    pub fn normalize_sheet_selection(&mut self) {
        let active = self.wb.active_sheet;
        self.selected_sheets.retain(|&i| self.wb.sheet(i).is_some_and(|s| s.visibility == gridcraft_model::Visibility::Visible));
        if self.wb.sheet(active).is_some_and(|s| s.visibility == gridcraft_model::Visibility::Visible) && !self.selected_sheets.contains(&active) {
            self.selected_sheets.push(active);
        }
        self.selected_sheets.sort_unstable();
        self.selected_sheets.dedup();
        if !self.selected_sheets.contains(&self.sheet_selection_anchor) {
            self.sheet_selection_anchor = active;
        }
    }
    /// Records the view state that is saved with the file (`sheetView topLeftCell`, the active
    /// cell): the first visible cell of the scrolling area per sheet index in `tops`, and the
    /// active sheet's active cell. View state never makes the workbook dirty.
    pub fn store_view(&mut self, tops: &[(usize, CellRef)]) {
        let active = self.selection.active;
        let si = self.wb.active_sheet;
        let changed = tops.iter().any(|(i, c)| self.wb.sheets.get(*i).is_some_and(|sh| sh.view_top_left != *c))
            || self.wb.sheets.get(si).is_some_and(|sh| sh.view_active != active);
        if !changed {
            return;
        }
        let dirty = self.is_dirty();
        let wb = Arc::make_mut(&mut self.wb);
        for (i, c) in tops {
            if let Some(sh) = wb.sheets.get_mut(*i)
                && sh.view_top_left != *c
            {
                Arc::make_mut(sh).view_top_left = *c;
            }
        }
        if let Some(sh) = wb.sheets.get_mut(si)
            && sh.view_active != active
        {
            Arc::make_mut(sh).view_active = active;
        }
        if !dirty {
            self.saved = self.wb.clone();
        }
    }
    pub fn display_title(&self) -> String {
        self.path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.title.clone())
    }
}

/// Engine-side preferences.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Prefs {
    /// After pressing Enter, move the selection: "down", "right", "up", "left", "none".
    pub enter_direction: String,
    pub autocomplete: bool,
    pub user_name: String,
    pub default_font: String,
    pub default_font_size: f32,
    pub sheets_in_new_workbook: usize,
    pub r1c1: bool,
    /// Words added with Add to Dictionary.
    pub user_dictionary: Vec<String>,
    /// View › Show › Formula Bar (an application option in Excel, not saved in the file).
    pub formula_bar: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            enter_direction: "down".into(),
            autocomplete: true,
            user_name: "GridCraft User".into(),
            default_font: gridcraft_model::DEFAULT_FONT.into(),
            default_font_size: gridcraft_model::DEFAULT_FONT_SIZE,
            sheets_in_new_workbook: 1,
            r1c1: false,
            user_dictionary: vec![],
            formula_bar: true,
        }
    }
}

/// Requests from commands to the UI (open a dialog, show a message…).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum UiRequest {
    Dialog(String, Json),
    Message(String),
    /// Start editing the active cell (F2) with optional initial text.
    EditCell(Option<String>),
    OpenUrl(String),
}

/// Width in points of one line of text drawn in a cell font, measured by a front end that has
/// the font the grid draws with. With `exact` false the front end may answer with a cheaper
/// measure that is never narrower (AutoFit asks for it on columns with many distinct values).
/// Commands call it while they run, so a front end must not run commands while holding a lock
/// the measure takes (egui's: see `theme::cell_text_width` in the UI crate).
pub type TextWidth = Arc<dyn Fn(&str, &model::Font, bool) -> f32 + Send + Sync>;

#[derive(Default)]
pub struct Session {
    pub(crate) vba: vba::State,
    pub(crate) recorder: crate::cmd::extra::recorder::Recorder,
    pub(crate) execute_depth: usize,
    docs: Vec<DocState>,
    active: usize,
    pub clipboard: Option<Clipboard>,
    pub prefs: Prefs,
    pub mode: Mode,
    /// Commands run so far, without their passwords ([`cmd::journal_params`]).
    pub journal: Vec<(String, Json)>,
    /// Serialized size of each journal entry, measured once when it is pushed, and their sum.
    journal_sizes: Vec<usize>,
    journal_bytes: usize,
    pub ui_requests: Vec<UiRequest>,
    /// Format painter source style (sticky when double-clicked).
    pub format_painter: Option<(Arc<Workbook>, usize, RangeRef, bool)>,
    /// Find state (last search).
    pub last_find: Option<Json>,
    /// View › Show › Ruler is off (rulers in Page Layout view; an application option).
    pub hide_ruler: bool,
    /// Journal position where action recording started.
    pub recording: Option<usize>,
    /// Windows hidden with View › Hide.
    pub hidden_windows: Vec<usize>,
    /// Draw tab tool: "" (select), "pen" or "eraser"; pen colour and width.
    pub draw_tool: String,
    pub draw_color: String,
    pub draw_width: f32,
    /// Review › Show Comments: the Comments pane is open.
    pub comments_pane: bool,
    /// Review › Notes › Show All Notes: every note is displayed. An application display setting
    /// (like Excel's comment indicator option), so it's not saved in the file.
    pub show_all_notes: bool,
    /// Measures cell text for AutoFit Column Width. Without it (CLI, MCP) AutoFit uses
    /// [`cmd::format::approx_text_width`].
    pub text_width: Option<TextWidth>,
}

impl Session {
    pub fn new() -> Session {
        Session::default()
    }
    pub fn documents(&self) -> &[DocState] {
        &self.docs
    }
    pub fn active_index(&self) -> usize {
        self.active
    }
    pub fn set_active(&mut self, i: usize) {
        if i < self.docs.len() {
            self.active = i;
        }
    }
    pub fn active(&self) -> Option<&DocState> {
        self.docs.get(self.active)
    }
    pub fn active_mut(&mut self) -> Option<&mut DocState> {
        self.docs.get_mut(self.active)
    }
    pub fn doc(&self) -> Result<&DocState> {
        self.active().ok_or(EngineError::NoDocument)
    }
    pub fn doc_mut(&mut self) -> Result<&mut DocState> {
        self.active_mut().ok_or(EngineError::NoDocument)
    }
    /// Refuse starting a cell edit with the same protection rules and message used when an edit
    /// commits, including Allow Edit Ranges unlocked in this document's session.
    pub fn check_cell_editable(&self, sheet: usize, cell: CellRef) -> Result<()> {
        let d = self.doc()?;
        let sh = d.wb.sheet(sheet).ok_or_else(|| EngineError::Other("no such sheet".into()))?;
        if sh.is_protected()
            && d.wb.styles.get(sh.style_id(cell)).protection.locked
            && let Some(message) = cmd::locked_cell_error(sh, cell, &d.unlocked_ranges)
        {
            return Err(EngineError::Other(message));
        }
        Ok(())
    }
    pub fn add_document(&mut self, d: DocState) -> usize {
        self.docs.push(d);
        self.active = self.docs.len() - 1;
        self.active
    }
    pub fn close_document(&mut self, i: usize) {
        if i < self.docs.len() {
            self.docs.remove(i);
            if self.active >= self.docs.len() {
                self.active = self.docs.len().saturating_sub(1);
            }
        }
    }
    /// A new blank workbook titled `BookN`.
    pub fn new_workbook(&mut self) -> usize {
        let n = (1..).find(|n| !self.docs.iter().any(|d| d.title == format!("Book{n}") && d.path.is_none())).unwrap_or(1);
        let mut wb = Workbook::new();
        for _ in 1..self.prefs.sheets_in_new_workbook.clamp(1, 255) {
            let name = wb.next_sheet_name();
            wb.sheets.push(Arc::new(gridcraft_model::Sheet::new(name)));
        }
        self.add_document(DocState::new(wb, None, format!("Book{n}")))
    }

    /// Journals a command, then trims the oldest entries past either cap. A recording in progress
    /// keeps pointing at the same entry (the steps the trim dropped are lost).
    fn push_journal(&mut self, id: &str, params: Json) {
        let size = |e: &(String, Json)| serde_json::to_vec(e).map_or(0, |v| v.len());
        // `journal` is public: if it was changed from outside, measure it again.
        if self.journal_sizes.len() != self.journal.len() {
            self.journal_sizes = self.journal.iter().map(size).collect();
            self.journal_bytes = self.journal_sizes.iter().sum();
        }
        let entry = (id.to_string(), params);
        let entry_size = size(&entry);
        self.journal.push(entry);
        self.journal_sizes.push(entry_size);
        self.journal_bytes = self.journal_bytes.saturating_add(entry_size);
        let len = self.journal.len();
        let mut n = if len > JOURNAL_MAX_ENTRIES { len - JOURNAL_MAX_ENTRIES / 2 } else { 0 };
        let mut kept = self.journal_bytes.saturating_sub(self.journal_sizes.iter().take(n).sum());
        if self.journal_bytes > JOURNAL_MAX_BYTES {
            // The newest entry stays even when it is larger than the cap on its own.
            while kept > JOURNAL_MAX_BYTES / 2 && n + 1 < len {
                kept = kept.saturating_sub(self.journal_sizes.get(n).copied().unwrap_or(0));
                n += 1;
            }
        }
        if n > 0 {
            self.journal.drain(..n);
            self.journal_sizes.drain(..n);
            self.journal_bytes = kept;
            self.recording = self.recording.map(|r| r.saturating_sub(n));
        }
    }

    pub fn commands(&self) -> Vec<CommandInfo> {
        command_specs().iter().map(|c| c.info(self)).collect()
    }

    /// Runs a command by id. Undoable commands record a history step when the workbook changed.
    /// Panics inside commands are caught and reported; the workbook is restored.
    pub fn execute(&mut self, id: &str, params: Json) -> Result<Json> {
        let spec = find_command(id).ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;
        if let Err(why) = (spec.enabled)(self) {
            return Err(EngineError::Disabled(id.to_string(), why));
        }
        let before = self.active().map(|d| (d.uid, d.wb.clone(), d.selection.clone(), d.selected_sheets.clone(), d.sheet_selection_anchor));
        let params = if params.is_null() { Json::Object(Default::default()) } else { params };
        let top_level = self.execute_depth == 0;
        let record_before = if top_level && self.recording.is_some() { self.active().cloned() } else { None };
        self.execute_depth = self.execute_depth.saturating_add(1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.should_group_command(id, &params) { self.execute_grouped(spec, &params) } else { (spec.run)(self, &params) }
        }));
        self.execute_depth = self.execute_depth.saturating_sub(1);
        let result = match result {
            Ok(r) => r,
            Err(p) => {
                let msg =
                    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "panic".into());
                // Restore the workbook as it was.
                if let Some((uid, wb, sel, sheets, anchor)) = &before
                    && let Some(d) = self.docs.iter_mut().find(|d| d.uid == *uid)
                {
                    d.wb = wb.clone();
                    d.selection = sel.clone();
                    d.selected_sheets = sheets.clone();
                    d.sheet_selection_anchor = *anchor;
                    d.calc.rebuild(&d.wb);
                }
                return Err(EngineError::Internal(id.to_string(), msg));
            }
        };
        if result.is_err() {
            // A command may have completed an internal step before a later validation failed.
            // Never publish that partial workbook, add history, or end copy mode on an error.
            if let Some((uid, wb, sel, sheets, anchor)) = &before
                && let Some(d) = self.docs.iter_mut().find(|d| d.uid == *uid)
            {
                if !Arc::ptr_eq(&d.wb, wb) {
                    d.wb = wb.clone();
                    d.calc.rebuild(&d.wb);
                }
                d.selection = sel.clone();
                d.selected_sheets = sheets.clone();
                d.sheet_selection_anchor = *anchor;
            }
            return result;
        }
        if top_level && spec.journal && result.is_ok() {
            self.push_journal(id, cmd::journal_params(&params));
        }
        if spec.undoable
            && let Some((uid, wb, sel, sheets, anchor)) = before
            && let Some(d) = self.docs.iter_mut().find(|d| d.uid == uid)
            && !Arc::ptr_eq(&d.wb, &wb)
        {
            push_history(&mut d.undo, spec.label.trim_end_matches('…').to_string(), wb, &d.wb, sel);
            if let Some(entry) = d.undo.last_mut() {
                entry.selected_sheets = sheets;
                entry.sheet_selection_anchor = anchor;
            }
            d.redo.clear();
            d.revision += 1;
            d.normalize_sheet_selection();
            // A successful edit ends copy mode, while repeated pastes of a copied block keep it.
            let repeated_copy = matches!(id, "edit.paste" | "edit.pasteSpecial") && self.clipboard.as_ref().is_some_and(|clip| !clip.cut);
            if !repeated_copy {
                self.clipboard = None;
            }
        }
        if top_level && result.is_ok() {
            if let Some(d) = self.active_mut() {
                d.normalize_sheet_selection();
            }
            crate::cmd::extra::recorder::capture(self, id, &params, record_before.as_ref());
        }
        result
    }

    fn should_group_command(&self, id: &str, params: &Json) -> bool {
        if self.active().is_none_or(|d| d.selected_sheets.len() < 2) || params.get("sheet").is_some() {
            return false;
        }
        if ["cell", "range", "at"]
            .iter()
            .any(|key| params.get(*key).and_then(Json::as_str).is_some_and(|s| gridcraft_core::split_sheet(s).0.is_some()))
        {
            return false;
        }
        matches!(
            id,
            "cell.set"
                | "range.setValues"
                | "range.fill"
                | "edit.paste"
                | "edit.pasteSpecial"
                | "edit.clearAll"
                | "edit.clearContents"
                | "edit.clearFormats"
                | "edit.clearComments"
                | "edit.clearHyperlinks"
                | "edit.fillDown"
                | "edit.fillUp"
                | "edit.fillRight"
                | "edit.fillLeft"
                | "edit.autoFill"
                | "edit.fillSeries"
                | "formulas.autoSum"
                | "home.insertRows"
                | "home.insertColumns"
                | "home.deleteRows"
                | "home.deleteColumns"
                | "home.insertCells"
                | "home.deleteCells"
        ) || cmd::format::specs().iter().any(|spec| spec.id == id)
    }

    /// Run existing command implementations against one private workbook. Individual commits
    /// and protection checks stay inside the staging session; publishing once creates one Undo.
    fn execute_grouped(&mut self, spec: &CommandSpec, params: &Json) -> Result<Json> {
        let original = self.doc()?.clone();
        let mut sheets = original.selected_sheets.clone();
        sheets.sort_unstable();
        sheets.dedup();
        if sheets.len() > 256 {
            return Err(EngineError::Other("Too many grouped sheets for one edit (maximum 256).".into()));
        }
        if matches!(spec.id, "edit.paste" | "edit.pasteSpecial") && self.clipboard.as_ref().is_some_and(|clip| clip.cut) {
            return Err(EngineError::Other("Cut cells cannot be pasted into grouped sheets.".into()));
        }
        self.check_group_budget(spec.id, params, &sheets)?;
        let mut params = params.clone();
        // A toolbar toggle derives one new state from the active sheet and applies that state
        // to the entire group, including sheets whose previous formatting differs.
        if params.get("on").is_none() {
            let st = cmd::format::active_style(self)?;
            let current = match spec.id {
                "home.bold" => Some(st.font.bold),
                "home.italic" => Some(st.font.italic),
                "home.strikethrough" => Some(st.font.strike),
                "home.superscript" => Some(st.font.vert == gridcraft_model::VertAlign::Superscript),
                "home.subscript" => Some(st.font.vert == gridcraft_model::VertAlign::Subscript),
                "home.wrapText" => Some(st.align.wrap),
                "home.shrinkToFit" => Some(st.align.shrink),
                "home.lockCell" => Some(st.protection.locked),
                "home.underline" | "home.doubleUnderline" => {
                    let desired =
                        if spec.id == "home.doubleUnderline" { "double" } else { params.get("style").and_then(Json::as_str).unwrap_or("single") };
                    Some(
                        st.font.underline
                            == match desired {
                                "double" => gridcraft_model::Underline::Double,
                                "singleAccounting" => gridcraft_model::Underline::SingleAccounting,
                                "doubleAccounting" => gridcraft_model::Underline::DoubleAccounting,
                                "none" => gridcraft_model::Underline::None,
                                _ => gridcraft_model::Underline::Single,
                            },
                    )
                }
                _ => None,
            };
            if let Some(current) = current {
                params["on"] = Json::Bool(!current);
            }
        }
        let mut stage = Session::new();
        stage.prefs = self.prefs.clone();
        stage.text_width = self.text_width.clone();
        stage.clipboard = self.clipboard.clone();
        stage.execute_depth = 1;
        stage.add_document(original.clone());
        let mut active_selection = original.selection.clone();
        let mut active_requests = Vec::new();
        let mut changed = false;
        let mut result = Json::Null;
        for sheet in sheets {
            let d = stage.doc_mut()?;
            if d.wb.sheet(sheet).is_none_or(|s| s.visibility != gridcraft_model::Visibility::Visible) {
                return Err(EngineError::Other("A grouped sheet is no longer visible.".into()));
            }
            Arc::make_mut(&mut d.wb).active_sheet = sheet;
            d.selection = original.selection.clone();
            d.selected_sheets = vec![sheet];
            d.sheet_selection_anchor = sheet;
            let before = d.wb.clone();
            let response = (spec.run)(&mut stage, &params)?;
            changed |= !Arc::ptr_eq(&stage.doc()?.wb, &before);
            let requests = stage.take_ui_requests();
            if sheet == original.wb.active_sheet {
                active_selection = stage.doc()?.selection.clone();
                active_requests = requests;
                result = response;
            }
        }
        let mut staged = stage.doc()?.clone();
        Arc::make_mut(&mut staged.wb).active_sheet = original.wb.active_sheet;
        let d = self.doc_mut()?;
        if changed {
            d.wb = staged.wb;
            d.calc = staged.calc;
        }
        d.selection = active_selection;
        self.ui_requests.extend(active_requests);
        Ok(result)
    }

    fn check_group_budget(&self, id: &str, params: &Json, sheets: &[usize]) -> Result<()> {
        const MAX_WORK: u64 = 5_000_000;
        const MAX_BYTES: u64 = 64 * 1024 * 1024;
        let d = self.doc()?;
        let ranges = if id == "edit.autoFill" {
            vec![params.get("target").and_then(Json::as_str).and_then(RangeRef::parse).ok_or_else(|| cmd::bad(id, "missing or invalid target"))?]
        } else {
            cmd::target_ranges(self, params)?
        };
        if ranges.len() > 4096 {
            return Err(EngineError::Other("Too many selected areas for one grouped edit.".into()));
        }
        let mut work = 0u64;
        let mut bytes = 0u64;
        for &sheet in sheets {
            let sh = d.wb.sheet(sheet).ok_or_else(|| EngineError::Other("A grouped sheet no longer exists.".into()))?;
            if matches!(id, "edit.paste" | "edit.pasteSpecial") {
                let (cells, owned) = cmd::edit::grouped_paste_budget(self, params, id, sheet)?;
                work = work.saturating_add(cells);
                bytes = bytes.saturating_add(owned);
            } else if id == "cell.set" {
                work = work.saturating_add(if params.get("array").and_then(Json::as_bool) == Some(true) { d.selection.current().count() } else { 1 });
                bytes = bytes
                    .saturating_add(params.get("input").or_else(|| params.get("value")).and_then(Json::as_str).map_or(0, |s| s.len() as u64 * 4));
            } else if id == "range.setValues" {
                if let Some(rows) = params.get("values").and_then(Json::as_array) {
                    for row in rows {
                        let values: &[Json] = row.as_array().map(Vec::as_slice).unwrap_or_else(|| std::slice::from_ref(row));
                        work = work.saturating_add(values.len() as u64);
                        bytes = values.iter().fold(bytes, |sum, value| sum.saturating_add(value.as_str().map_or(0, |s| s.len() as u64 * 4)));
                    }
                }
            } else if matches!(
                id,
                "home.insertRows" | "home.insertColumns" | "home.deleteRows" | "home.deleteColumns" | "home.insertCells" | "home.deleteCells"
            ) {
                // Line commands accept comma-separated rows/cols separately from `range`.
                let areas = ["rows", "cols"]
                    .iter()
                    .filter_map(|key| params.get(*key).and_then(Json::as_str))
                    .map(|text| text.split(',').count())
                    .max()
                    .unwrap_or(ranges.len());
                if areas > 4096 {
                    return Err(EngineError::Other("Too many selected areas for one grouped edit.".into()));
                }
                work = work.saturating_add((sh.cells.len() as u64).saturating_add(1).saturating_mul(areas as u64));
            } else {
                for range in &ranges {
                    let count = if matches!(id, "edit.fillDown" | "edit.fillUp" | "edit.fillRight" | "edit.fillLeft") {
                        if range.is_full_cols() || range.is_full_rows() {
                            sh.used_range().and_then(|used| range.intersection(&used)).map_or(0, |r| r.count())
                        } else {
                            range.count()
                        }
                    } else if matches!(id, "range.fill" | "edit.autoFill" | "edit.fillSeries" | "formulas.autoSum") {
                        range.count()
                    } else if range.is_full_rows() || range.is_full_cols() {
                        let lines = if range.is_full_rows() && !range.is_full_cols() { range.height() } else { range.width() };
                        u64::from(lines).saturating_add(sh.cells.iter_range(*range).count() as u64)
                    } else {
                        range.count()
                    };
                    work = work.saturating_add(count);
                    if matches!(id, "edit.fillDown" | "edit.fillUp" | "edit.fillRight" | "edit.fillLeft" | "edit.autoFill" | "edit.fillSeries") {
                        let effective = if range.is_full_cols() || range.is_full_rows() {
                            sh.used_range().and_then(|used| range.intersection(&used))
                        } else {
                            Some(*range)
                        };
                        if let Some(mut area) = effective {
                            if id == "edit.fillDown" && area.height() == 1 {
                                area.start.row = area.start.row.saturating_sub(1);
                            }
                            if id == "edit.fillRight" && area.width() == 1 {
                                area.start.col = area.start.col.saturating_sub(1);
                            }
                            let max_formula = sh
                                .cells
                                .iter_range(area)
                                .filter_map(|(_, cell)| cell.formula.as_ref())
                                .map(|f| f.text.len() as u64)
                                .max()
                                .unwrap_or(0);
                            bytes = bytes.saturating_add(max_formula.saturating_mul(4).saturating_mul(count));
                        }
                    }
                    if id == "range.fill"
                        && let Some(input) = params.get("input").and_then(Json::as_str)
                    {
                        // Plain text cells share their parsed value; each translated formula owns
                        // its expression/text, so account for its full materialized output.
                        bytes = bytes.saturating_add((input.len() as u64).saturating_mul(if input.starts_with('=') {
                            count.saturating_mul(4)
                        } else {
                            1
                        }));
                    }
                }
            }
            let max_work = if matches!(id, "edit.paste" | "edit.pasteSpecial") { 4_000_000 } else { MAX_WORK };
            if work > max_work || bytes > MAX_BYTES {
                return Err(EngineError::Other(
                    "The grouped edit is too large (5000000 operations, or 4000000 pasted cells, and 64 MiB of owned data).".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn run(&mut self, id: &str, params: Json) -> std::result::Result<Json, String> {
        self.execute(id, params).map_err(|e| e.to_string())
    }

    pub fn take_ui_requests(&mut self) -> Vec<UiRequest> {
        std::mem::take(&mut self.ui_requests)
    }

    /// Runs a command for a caller with no UI (MCP, CLI): queued UI requests are dropped, and a command that only
    /// queued a file dialog because it got no file (`file.open`, `file.saveAs`, `file.save` on a workbook with no
    /// file yet, `data.getData`) is an error instead of a silent success.
    pub fn execute_headless(&mut self, id: &str, params: Json) -> Result<Json> {
        let r = self.execute(id, params);
        let requests = self.take_ui_requests();
        let asked_for_file = requests.iter().any(|q| matches!(q, UiRequest::Dialog(name, _) if name == "open" || name == "saveAs"));
        if r.is_ok() && asked_for_file {
            let what = match id {
                "file.open" => "`path` or `base64`",
                "data.getData" | "data.fromTextCsv" => "`path` or `text`",
                _ => "`path`",
            };
            let msg = format!("missing {what}: programmatic calls never open dialogs");
            return Err(EngineError::BadParams { cmd: id.to_string(), msg });
        }
        r
    }
}

/// Convenience: the active workbook's active sheet.
pub fn active_sheet(s: &Session) -> Option<&gridcraft_model::Sheet> {
    s.active().and_then(|d| d.wb.active())
}

pub fn cell(r: u32, c: u32) -> CellRef {
    CellRef::new(r, c)
}

#[cfg(test)]
mod tests;
