# Gemini Audit Report: Commercial Excel Operational Parity, Root-Cause Analysis & UI Macros Entry Point

- **Date**: 2026-10-24 (Updated with Root-Cause / Rust-WASM-egui Architectural Analysis & Macros UI Audit)
- **Scope**:
  1. User-reported multi-range paste gap & systematic audit of operational differences vs. commercial Microsoft Excel
  2. Root-cause analysis of **why** these differences occurred and **Rust / WASM / egui technical pitfalls** that must be solved when fixing them
  3. Missing **Macros (マクロ)** button on the top Quick Access Toolbar (`title_bar`) and `View` (`表示`) tab, plus broken `view.macros` UI routing
- **Constraint Compliance**: Read-only code inspection; no `cargo` / `rustc` / `trunk` execution; no source code modified.

---

## Part 1: Root-Cause Analysis — Why Did These Differences Occur, and What Rust/WASM/egui Pitfalls Exist?

Investigating the architecture across `crates/engine` and `crates/ui-egui` revealed **five structural reasons** why these gaps existed from the original baseline (`ac8da7d4`), including specific **Rust borrow-checker, coordinate-shift, and WebAssembly async constraints** that must be handled carefully during implementation:

### Root Cause A: `cargo xtask parity` Only Measured Command ID Registration ("Breadth"), Not UI Placement or Multi-Range Depth
- **Evidence**: `xtask/src/parity.rs:1-4` and `crates/engine/src/catalog.rs:1-4` define parity as `gridcraft_engine::find_command(id).is_some()`.
- **Consequence**:
  - As long as a command ID was registered in `crates/engine/src/cmd/*.rs`, `cargo xtask parity` reported 100% implementation—even if:
    1. The command was never drawn on the corresponding Ribbon tab in `crates/ui-egui/src/ribbon.rs` (e.g., `view.macros` is in `CATALOG` under `View -> Macros` and in `keytip_table/generated.rs:344-346` as `WM` / `WMV`, but **never drawn in `ribbon.rs:view()`**!).
    2. The command implementation called `target_range(s, p)` (`ranges.first()`) or `selection.current()` (`ranges.last()`) instead of `target_ranges(s, p)` (`selection.ranges`).

### Root Cause B: Rust Borrow Checker Split Between `Session` (`clipboard`) and `DocState` (`wb`, `selection`, `undo`)
- **Evidence**: `crates/engine/src/lib.rs:293-300` places `pub clipboard: Option<Clipboard>` on `Session`, while `wb` and `selection` live inside `DocState` (`s.docs[s.active]`).
- **Rust Pitfall**:
  - In `crates/engine/src/cmd/mod.rs:320-327`:
    ```rust
    pub(crate) fn edit<R>(s: &mut Session, f: impl FnOnce(&mut Ctx) -> Result<R>) -> Result<R> {
        let d = s.doc_mut()?;
        commit(d, |cx| { ... })
    }
    ```
  - Because `s.doc_mut()` mutably borrows `s`, closures inside `edit(s, |cx| ...)` cannot access `s.clipboard`. Consequently, the original author only cleared `s.clipboard` via a manual `cancel_copy_mode(s)` helper in `sheet.rs:266` after `edit()` returned, leaving `s.clipboard` alive across all cell edits (`cell.set`, `range.fill`, `edit.clear*`, `edit.fill*`, etc.).
- **Clean Rust Solution**:
  - Clear `s.clipboard` in `Session::execute` (`crates/engine/src/lib.rs:459-468`)—or at the end of `cmd::edit(s, ...)` after `commit(d, ...)` drops its `&mut DocState` borrow—whenever an undoable command mutates the workbook (`!Arc::ptr_eq(&d.wb, &wb)`), **except** when `id` is `"edit.paste"` or `"edit.pasteSpecial"` with a non-cut clipboard (`cut == false`), because Excel keeps marching ants across repeated pastes of a copied range until another edit occurs.

### Root Cause C: Coordinate Drift & Cumulative Overflow in Multi-Area Structural Edits (`sheet.rs`)
- **Evidence**: `crates/engine/src/cmd/sheet.rs:318-349` (`delete_areas`).
- **Rust / Algorithmic Pitfall**:
  - The doc comment on `delete_areas` (`sheet.rs:318`) literally states:
    `/// The areas a line delete/insert targets as (first line, count), merged and sorted from the last line to the first so deleting one never shifts the next.`
  - However, only `delete_lines` called `delete_areas`; `insert_lines` (`sheet.rs:273`) still called `lines()` (`selection.current()`).
  - **Trap when fixing `insert_lines`, `insert_cells`, and `delete_cells` for multi-area selections**:
    1. **Reverse-order execution is mandatory**: In `CellStore` (`BTreeMap`-backed), shifting row 2 down by 1 immediately increments all row indices `>= 2`. If a multi-area selection `(Row 2, Row 5)` is processed top-to-bottom, the second insert happens at Row 5 (which was originally Row 4!) instead of Row 6. Sorting and merging spans and iterating **in reverse (`.rev()`, highest row/col to lowest)** prevents coordinate drift.
    2. **Same-row/col deduplication**: If a user selects `B2` and `D2` (`Cmd+Click`) and runs `home.insertRows`, merging row spans `(2..=2, 2..=2)` -> `(2, 1)` ensures 1 row is inserted at row 2, not 2 rows (matching Excel).
    3. **Cumulative overflow check before mutation**: In `insert_lines` (`sheet.rs:283-288`), the check that non-blank cells in `used_range()` are not pushed past `MAX_ROWS` / `MAX_COLS` must sum the `count` of **all** inserted spans at or before `u.end`, not just check each span in isolation.

### Root Cause D: WebAssembly Browser Sandbox Clipboard (`Promise` vs. Synchronous `Services::paste_text`)
- **Evidence**: `crates/ui-egui/src/lib.rs:193-196` and `apps/gridcraft-web/src/web.rs:191-243`.
- **Rust + WASM Pitfall**:
  - `Services::paste_text` is a synchronous closure `Option<Box<dyn FnMut() -> Option<String>>>`.
  - On Desktop, OS clipboard reads are synchronous. On Web (`wasm32-unknown-unknown`), the browser's `navigator.clipboard.readText()` is **asynchronous (`js_sys::Promise`)** and cannot block the single-threaded WASM stack. That is why `services.paste_text` was left `None` on Web, causing the Ribbon/Context-Menu "Paste" button to ignore external OS clipboard text.
- **Clean Rust/WASM Solution**:
  - Mirror `open_async` / `pick_picture_async` in `Services`: add `pub paste_async: Option<Box<dyn Fn()>>` on `Services`. On Web, when the user clicks the Ribbon or Context-Menu "Paste" button and `services.paste_text` is `None`, invoke `paste_async` (which calls `navigator.clipboard.readText()` in JS/WASM and enqueues `egui::Event::Paste(text)` into `app.synthetic` + `ctx.request_repaint()`, or falls back to `edit.paste` if internal `s.clipboard` is newer or permission is denied).

### Root Cause E: `egui::Modifiers::COMMAND` vs. `Modifiers::CTRL` on macOS
- **Evidence**: `crates/ui-egui/src/formula_bar.rs:258` and `crates/ui-egui/src/grid.rs:2046`.
- **egui Pitfall**:
  - `egui` maps `Modifiers::COMMAND` to `Cmd` on macOS and `Ctrl` on Windows/Linux.
  - However, in commercial Excel for Mac, **`Control + Enter`** (as well as `Cmd + Enter`) is the standard shortcut to commit an edit across all selected cells (`range.fill`). Checking only `Modifiers::COMMAND` in `formula_bar.rs:258` broke `Control + Enter` on macOS.

---

## Part 2: Missing Macros (マクロ) Button in Initial UI Bar / View Tab & Broken `view.macros` Routing

### 0. [HIGH] Macros Button Missing from Top Quick Access Toolbar (`title_bar`) and `View` (`表示`) Tab, and `view.macros` Fails to Open Dialog
- **Location**:
  - `crates/ui-egui/src/ribbon.rs:38-115` (`title_bar` — top Quick Access Toolbar)
  - `crates/ui-egui/src/ribbon.rs:2253-2392` (`view` — `表示` tab)
  - `crates/ui-egui/src/ribbon.rs:2394-2418` (`automate` — `自動化` tab)
  - `crates/engine/src/cmd/extra.rs:150` (`view.macros`)
  - `crates/ui-egui/src/lib.rs:532-557` (`run_or_alert`)
- **What the User Noticed**:
  - The user asked: *"Didn't there use to be a Macros button on the very first operation icon bar (UI)?"*
- **Actual Code State**:
  1. **Top Quick Access Toolbar (`title_bar`, `ribbon.rs:52-115`)**: Only shows `AutoSave`, `Home`, `Save`, `Undo`, `Redo`, and `More commands` (which only lists `New`, `Open`, `Save As`, `Sort A to Z`, `Calculate Now`, `Print`). There is **no Macros (`Icon::Script`) button** on the top bar or in `More commands`.
  2. **Initial `Home` (`ホーム`) tab & `View` (`表示`) tab**: Neither has a `Macros` button. In commercial Excel, `View` (`表示`) always has the **`Macros` (`マクロ`)** button at the far right, and `catalog.rs:271` + `keytip_table/generated.rs:344-346` (`Alt -> W -> M -> V`) explicitly declare `"view.macros"` under `View -> Macros`. Yet `ribbon.rs:view()` omitted it completely!
  3. **Broken `view.macros` UI routing**: Even if a user triggers `"view.macros"` via KeyTips (`Alt, W, M, V`) or Command Search (`commandSearch`), `extra.rs:150` defines `view.macros` as a headless query (`all_scripts`) that returns a JSON array and **never opens the `"macros"` dialog**!
  4. **Hidden on the 9th Tab (`Automate` / `自動化`)**: Currently, the only button that opens `app.open_dialog("macros", json!({}))` is on the far-right 9th tab `Automate` (`自動化`, `ribbon.rs:2395`). Furthermore, `automate()` (`ribbon.rs:2394-2418`) does not even show buttons for `Record Actions` (`automate.recordActions`) or direct `VBA Editor` (`automate.newScript`).
- **Recommendation**:
  1. Add a **`Macros` (`Icon::Script`, tooltip `"Macros (Alt+F8)"`)** quick-access button to `title_bar` (`crates/ui-egui/src/ribbon.rs:76`, right next to Save / Undo / Redo, and/or in `More commands`), so it is immediately accessible from the very first screen without switching tabs.
  2. Add the **`Macros` (`big_button(ui, Icon::Script, "Macros", ...)` with `control_keytip(..., "view.macros")`)** button to the **`View` (`表示`)** tab (`crates/ui-egui/src/ribbon.rs:2391`), matching commercial Excel and `catalog.rs:271`.
  3. In `SheetApp::run_or_alert` (`crates/ui-egui/src/lib.rs:532`), intercept `"view.macros"` (and `"automate.allScripts"`) to call `self.open_dialog("macros", json!({}))`, so KeyTips (`Alt, W, M, V`) and Command Search open the Macros / VBA Editor dialog while programmatic callers (`session.run("view.macros", ...)`) still receive the JSON script list.

### 0b. [HIGH] "Record Macro" (マクロの記録) Button Missing from Ribbon & Status Bar, and Recording Does Not Generate VBA Code into the VBA Editor
- **Location**:
  - `crates/ui-egui/src/ribbon.rs:2394-2418` (`automate` tab) & `2253-2392` (`view` tab)
  - `crates/ui-egui/src/tabs.rs:260-296` (`status_bar` — bottom-left next to `"Ready"` / `"準備完了"`)
  - `crates/ui-egui/src/keytips/keytip_table/generated.rs:345` (`WMR` mapped to `view.macros` instead of `automate.recordActions`)
  - `crates/engine/src/cmd/extra.rs:969-980` (`record` — `automate.recordActions`)
- **What the User Noticed**:
  - The user asked: *"In commercial Excel, besides the Macro icon, isn't there a 'Record Macro' button and feature that records operations and automatically writes them into the Macro VBA code screen?"*
- **Why This Difference Occurred (Architectural History)**:
  1. **No UI Button for Macro Recording Anywhere**:
     - `automate.recordActions` exists in the engine (`crates/engine/src/cmd/extra.rs:185`), but there is **no button** for it on the `Automate` (`自動化`) tab (`ribbon.rs:2394-2418`), **no button** on the `View` (`表示`) tab (`ribbon.rs:2253-2392`), and **no Record/Stop Macro indicator button** next to `"Ready"` (`準備完了`) on the bottom-left status bar (`tabs.rs:266-272`).
     - Even KeyTip `Alt, W, M, R` (`generated.rs:345`, commented `// Excel: Record Macro`) was wired to `view.macros` instead of `automate.recordActions`!
  2. **Disconnected Two-Generation Macro Architecture (`_sc_scripts` JSON vs. `vba/` Interpreter)**:
     - Originally, before the VBA interpreter (`crates/engine/src/vba/`) and `VbaEditor` (`crates/ui-egui/src/vba_editor.rs`) were built, `automate.recordActions` (`extra.rs:969-980`) was implemented as a simple JSON journal dump (`[{"command": "selection.set", ...}, {"command": "cell.set", ...}]`) stored in `_sc_scripts`.
     - Later, when the real VBA engine and `VbaEditor` were added, `automate.recordActions` was never updated to emit VBA source code (`Sub Macro1() ... End Sub`) into a VBA module!
     - As a result, in commercial Excel, recording a macro writes VBA statements (`Range("B2").Select`, `ActiveCell.FormulaR1C1 = ...`, etc.) into `Module1` so the user can view and edit the recorded macro in the VBA Editor; in GridCraft, recording neither has a button nor writes anything to the VBA Editor.
- **Clean Rust Solution (No Technical Blockers)**:
  1. **UI Buttons**:
     - Add a **「マクロの記録 / 記録終了」(`Record Macro` / `Stop Recording`, toggling `app.session.recording.is_some()`)** button to the `Automate` (`自動化`) tab (`ribbon.rs:2395`), the `View` (`表示`) tab Macros area, and the bottom-left `status_bar` (`tabs.rs:272`, right next to `準備完了` / `Ready`, showing `●` / `■` like Excel).
     - Fix KeyTip `WMR` (`generated.rs:345`) to invoke `automate.recordActions`.
  2. **Automatic Command-to-VBA Code Generation (`journal_to_vba`)**:
     - In `crates/engine/src/cmd/extra.rs:969-980` (`record`), when recording stops (or live as commands are recorded into `s.journal`), in addition to saving the recorded script, translate the recorded `(command_id, params)` entries into valid VBA statements supported by `crates/engine/src/vba/` and append/save a `Sub <MacroName>() ... End Sub` into `Module1` (via `crate::vba::save_module`):
       - `selection.set` (`cell` / `range`) -> `Range("...").Select`
       - `cell.set` (`value` starting with `=`) -> `Range("...").Formula = "..."`
       - `cell.set` (`value` literal) -> `Range("...").Value = ...`
       - `range.fill` / `range.setValues` -> `Range("...").Value = ...`
       - `edit.clearContents` / `edit.clear` -> `Selection.ClearContents` / `Selection.Clear`
       - `sheet.activate` -> `Worksheets("...").Activate`
       - `home.insertSheet` -> `Worksheets.Add`
       - `sheet.rename` -> `ActiveSheet.Name = "..."`
       - `home.insertRows` / `home.deleteRows` -> `Selection.EntireRow.Insert` / `Selection.EntireRow.Delete`
       - `home.insertColumns` / `home.deleteColumns` -> `Selection.EntireColumn.Insert` / `Selection.EntireColumn.Delete`
       - Any other command -> comment or fallback comment `' [command_id] ...` so the generated `Sub` is always valid, runnable, and editable in `VbaEditor`!

---

## Part 3: Complete List of Commercial Excel Operational Differences (12 Items)

### 1. [HIGH] Copy Mode (`s.clipboard`) Never Clears on Cell Edits, Causing `Enter` to Unexpectedly Paste Old Clipboard Data
- **Location**: `crates/ui-egui/src/grid.rs:3038`, `crates/engine/src/lib.rs:459-468`
- **Problem**: `s.clipboard` is only cleared on `Escape` and row/col/cell insert/delete. If a user copies a cell (`Cmd+C`), edits several other cells, and later presses `Enter` on the grid (not editing) expecting to move down 1 row, `grid.rs:3038` sees `app.session.clipboard.is_some()` and **overwrites the active cell with the old clipboard**!
- **Fix**: Clear `s.clipboard` in `Session::execute` whenever an undoable command modifies `d.wb` (`!Arc::ptr_eq(&d.wb, &wb)`), except for non-cut `edit.paste` / `edit.pasteSpecial`.

### 2. [HIGH] Fill Down / Right / Up / Left (`Cmd+D` / `Cmd+R`, `edit.fillDown` etc.) Ignores Multi-Area Selections
- **Location**: `crates/engine/src/cmd/edit.rs:1581-1614` (`fill_dir`)
- **Problem**: Uses `target_range(s, p)?` (`ranges.first()`), ignoring all other selected ranges in `selection.ranges`.
- **Fix**: Iterate over `target_ranges(s, p)?` in `fill_dir` and apply `fill_one_dir` to every selected area in one `edit(s, ...)` step.

### 3. [HIGH] `Ctrl + Enter` (Mac & Windows Excel Standard for Multi-Cell Commit) Fails on macOS
- **Location**: `crates/ui-egui/src/formula_bar.rs:258`, `crates/ui-egui/src/grid.rs:3044`
- **Problem**: `formula_bar.rs:258` only checks `egui::Modifiers::COMMAND` (`Cmd+Enter` on Mac) and ignores `egui::Modifiers::CTRL` (`Ctrl+Enter`). Meanwhile `grid.rs:3044` treats `Ctrl+Enter` when not editing as `begin_edit`.
- **Fix**: Accept both `Modifiers::COMMAND` and `Modifiers::CTRL` for `Key::Enter` in `formula_bar.rs:258` (and in `suspend_ime_boundary_keys`), and remove `m.ctrl` from `grid.rs:3044`.

### 4. [HIGH] `Backspace` vs. `Delete` on Multi-Cell Selection Clears Everything Instead of Editing Active Cell
- **Location**: `crates/ui-egui/src/grid.rs:3055-3061`
- **Problem**: Pressing `Backspace` on a multi-cell selection calls `edit.clearContents` on all selected cells instead of clearing only the active cell and entering edit mode (`app.begin_edit(Some(String::new()), false)`) while preserving the multi-cell selection.
- **Fix**: Keep `Key::Delete` as `edit.clearContents` on the full selection, and change `Key::Backspace` (without modifiers, when no object is selected) to start editing the active cell with `Some(String::new())` while keeping `selection.ranges`.

### 5. [MEDIUM-HIGH] Deselecting / Clicking Inside an Existing Range with `Cmd+Click` / `Ctrl+Click` Creates Overlapping Selections
- **Location**: `crates/ui-egui/src/grid.rs:2890-2911` (`toggled_ranges`, `select_ranges`)
- **Problem**: `toggled_ranges` only checks exact string equality `ranges.contains(&clicked)`. Clicking `A3` with `Cmd` held while `A1:A5` is selected appends `"A3"` (`"A1:A5,A3"`), creating an overlapping selection that causes subsequent multi-area paste (`paste_plan` at `edit.rs:883-888`) to error with `"That command cannot be used on overlapping selections."`.
- **Fix**: Operate on `Vec<RangeRef>`: if the clicked range/cell intersects an existing `RangeRef`, split or subtract the clicked cell/range from the existing `RangeRef` (up to 4 non-overlapping rectangles: above, below, left, right) so `Cmd+Click` deselects cells inside a range just like Excel 2019/365 and never creates overlapping areas.

### 6. [MEDIUM-HIGH] Insert Rows / Columns & Insert / Delete Cells Ignore Multi-Area Selections
- **Location**:
  - `crates/engine/src/cmd/sheet.rs:273-316` (`insert_lines`)
  - `crates/engine/src/cmd/sheet.rs:483-556` (`insert_cells`, `delete_cells`)
  - `crates/ui-egui/src/ribbon.rs:2660` (`Key::Plus | Key::Equals if shift`)
- **Problem**: `delete_lines` supports multi-area selections via `delete_areas` (`sheet.rs:321-349`), but `insert_lines` only reads `selection.current()`, `insert_cells` / `delete_cells` only read `target_range()` (`ranges.first()`), and `ribbon.rs:2660` blocks `Ctrl+Shift+=` when multiple full rows/columns are selected.
- **Fix**: Use `delete_areas` in `insert_lines` (checking cumulative inserted line count against `MAX_ROWS` / `MAX_COLS` and `used_range()`), process `target_ranges(s, p)?` sorted in reverse along the shift axis in `insert_cells` / `delete_cells`, and remove `.filter(|_| single_area(app))` from `ribbon.rs:2660`.

### 7. [MEDIUM-HIGH] `Shift + Space` (`selection.row`) and `Ctrl + Space` (`selection.column`) Drop Multi-Area Selections
- **Location**: `crates/engine/src/cmd/edit.rs:540-554` (`select_row`, `select_col`)
- **Problem**: Both functions read only `let r = d.selection.current();` and overwrite `d.selection.ranges` with a single range. In Excel, `Shift+Space` / `Ctrl+Space` expands **every** area in `d.selection.ranges` to full rows / full columns.
- **Fix**: Map all `d.selection.ranges` to `RangeRef::rows(r.start.row, r.end.row)` / `RangeRef::cols(r.start.col, r.end.col)`.

### 8. [MEDIUM] Replace (`edit.replace`) Ignores Multi-Cell Selection Scope and Does Not Advance on Single Replace
- **Location**: `crates/engine/src/cmd/edit.rs:1784-1817` (`replace`)
- **Problem**:
  1. `all: true` ("Replace All") always replaces across the entire sheet, ignoring a multi-cell selection. In Excel, if more than 1 cell is selected (`!d.selection.is_single_cell()`), "Replace All" replaces **only within `d.selection.ranges`**.
  2. `all: false` ("Replace") replaces at `d.selection.active` but does not advance to the next match.

### 9. [MEDIUM] AutoSum (`Alt+=`) and Define Name Ignore Multi-Area Selections
- **Location**: `crates/engine/src/cmd/formulas.rs:86-89` (`auto_sum`), `287-292` (`define_name`)
- **Problem**: Both inspect only `d.selection.current()` instead of iterating `d.selection.ranges` (for AutoSum) or joining all areas with commas (for Define Name).

### 10. [MEDIUM] Format Painter (`edit.formatPainter`) Single-Click vs. Double-Click Conflict in `ribbon.rs`
- **Location**: `crates/ui-egui/src/ribbon.rs:675-683`
- **Problem**:
  ```rust
  if fp.double_clicked() {
      act(app, "edit.formatPainter", json!({"sticky": true}));
  } else if fp.clicked() {
      if fp_on {
          app.session.format_painter = None;
      } else {
          act(app, "edit.formatPainter", json!({}));
      }
  }
  ```
  In `egui`, the first click of a double-click fires `fp.clicked()` on frame 1 (which sets `fp_on = true` with `sticky = false`), and on the second click, if `double_clicked()` and `clicked()` interact across frames or when clicking again, verify that double-clicking reliably leaves `format_painter` armed with `sticky = true` (`format_painter.3 == true`) rather than toggling it off!

### 11. [MEDIUM] Web Ribbon / Context-Menu "Paste" Ignores External OS Clipboard
- **Location**: `crates/ui-egui/src/lib.rs:487-491`, `apps/gridcraft-web/src/web.rs:191-243`
- **Fix**: Wire asynchronous browser clipboard read (`navigator.clipboard.readText()`) or cached clipboard text when `services.paste_text` is `None`.

### 12. [LOW-MEDIUM] `F2` / Formula Bar Click Drops IME `platform_output.ime` for 1–2 Frames
- **Location**: `crates/ui-egui/src/formula_bar.rs:43-55`, `390-406`, `crates/ui-egui/src/lib.rs:952-995` (`begin_edit`)
- **Fix**: Request focus on `gridcraft.formula_bar` or `gridcraft.cell_editor` before `TextEdit::show` runs on the opening frame of `begin_edit`.

### 13. [MEDIUM] Right-Click Does Nothing in VBA Code Editor (`vba_editor.rs`) and Formula Bar / Cell Editor (`formula_bar.rs`), and Recorded Scripts Lack a Delete Button (`dialogs.rs`)
- **Location**:
  - `crates/ui-egui/src/vba_editor.rs:390-397` (`TextEdit::multiline(&mut self.source)`)
  - `crates/ui-egui/src/formula_bar.rs:390-399` (`edit.show(ui)`)
  - `crates/ui-egui/src/dialogs.rs:1834-1859` (`macros` -> `Recorded Scripts` tab)
- **Root Cause**:
  1. `eframe` suppresses the browser's default DOM `contextmenu` on `<canvas id="gridcraft_canvas">`, while `egui::TextEdit` does not provide a built-in right-click menu unless `.context_menu(|ui| ...)` is explicitly attached to its `Response`. As a result, right-clicking inside the VBA code editor (`vba_editor.rs:390-397`) or formula bar (`formula_bar.rs:390-399`) does nothing at all.
  2. For "Paste" inside that right-click menu on Web (`wasm32`), `Cmd+V` works because the browser fires a DOM `paste` event (`egui::Event::Paste`), whereas clicking a UI menu item requires reading the clipboard via `services.paste_text` (desktop) or `services.paste_async` (`navigator.clipboard.readText()` on web) and inserting at the current `TextEditState` cursor range (or pushing `egui::Event::Paste` while keeping focus on `vba_source`).
  3. In `dialogs.rs:1834-1859`, the `Recorded Scripts` tab only renders a `Run script` (`スクリプトを実行`) button and lacks a `Delete` (`削除`) button calling `automate.deleteScript` (`crates/engine/src/cmd/extra.rs:214`).
- **Fix**:
  - Attach `.context_menu(|ui| ...)` with localized **Cut (`切り取り`) / Copy (`コピー`) / Paste (`貼り付け`) / Select All (`すべて選択`)** actions to the `TextEdit` response in `vba_editor.rs:390-397` (and `formula_bar.rs:390-399`), wired to `TextEditState` cursor range + `ui.ctx().copy_text(...)` + `paste_text` / `paste_async`.
  - Add a localized `Delete` (`削除`) button in `dialogs.rs:1850-1856` calling `automate.deleteScript`.

### 14. [HIGH] Formula Entry Errors & Dynamic Engine Errors Display in English (`tr_error` in `i18n.rs`)
- **Location**:
  - `crates/engine/src/cmd/edit.rs:195-221` (`parse_typed_formula`, `check_arg_counts`)
  - `crates/formula/src/parser.rs:62-250`, `crates/formula/src/lexer.rs:97-150` (`ParseError`, `LexError`)
  - `crates/ui-egui/src/i18n.rs:222-229` (`Language::tr_error`)
  - `crates/ui-egui/src/i18n/ja.rs`
- **Root Cause**:
  - When a user enters a malformed formula or wrong argument count, `edit.rs:198` and `214` format dynamic strings:
    - `"There's a problem with this formula: {e}"` (where `{e}` is e.g. `"expected expression, found end of formula"`, `"unexpected `,`"`, `"unterminated text"`, etc.)
    - `"{NAME} has too few arguments. Use {SIGNATURE}"` / `"{NAME} has too many arguments. Use {SIGNATURE}"`
  - However, `Language::tr_error` (`i18n.rs:222-229`) only performs an exact static string lookup `self.tr(&message)`. Any error with dynamic parameters (`{e}`, `{NAME}`, `{SIGNATURE}`) misses the lookup table and is shown in raw English.
- **Fix**:
  1. In `Language::tr_error` (`crates/ui-egui/src/i18n.rs:222-229`), add structured pattern matching before falling back to raw text:
     - Strip `"There's a problem with this formula: "` -> translate prefix (`"この数式には問題があります: "`) and translate the inner `ParseError` reason (`"unexpected end of formula"` -> `"数式が途中で終わっています"`, `"unterminated text"` -> `"文字列が閉じられていません"`, `"expected {a}, found {b}"` -> `"{a} が必要ですが、{b} が見つかりました"`, `"unexpected {t}"` -> `"予期しない {t} です"`, etc.).
     - Match `"{name} has too few arguments. Use {sig}"` -> `"この関数に対して、入力されている引数が少なすぎます ({name})。使用方法: {sig}"`
     - Match `"{name} has too many arguments. Use {sig}"` -> `"この関数に対して、入力されている引数が多すぎます ({name})。使用方法: {sig}"`
  2. Add missing static worksheet error messages to `ja.rs` (e.g., `"You can't change part of an array."`, `"You can't do that to a merged cell."`, `"That command cannot be used on multiple selections."`, `"That command cannot be used on overlapping selections."`, and data validation default alerts).

### 15. [HIGH] Picture Insertion ("Place in Cell" vs "Place over Cells") & Broken Resize Handles (`ribbon.rs`, `chartview.rs`, `grid.rs`)
- **Location**:
  - `crates/ui-egui/src/ribbon.rs:1516-1526` (`Insert -> Pictures` popup)
  - `crates/ui-egui/src/cell_pictures.rs:110-125` (`CellPicture` in-cell rendering)
  - `crates/ui-egui/src/chartview.rs:335-350, 493-532` (floating `Picture` rendering & `hit()`)
  - `crates/ui-egui/src/grid.rs:2004-2007, 2130-2161, 2500-2508, 3269-3420` (resize handles, drag logic, context menus)
  - `crates/ui-egui/src/app.rs:3006-3043` (`open_picture_file`)
- **Root Cause**:
  1. **Confusing default & missing conversion between `Place in Cell` (`セルに配置`) and `Place over Cells` (`セルの上に配置`)**:
     - `ribbon.rs:1520` lists **「セルに配置 (`Place in Cell`, `placement: "cell"`)」** first in the `挿入 → 画像` popup. When a user selects it (expecting normal Excel picture insertion), the screenshot is stored in `sh.cell_pictures` (`cell_pictures.rs:110-120`), shrunk to fit inside the tiny `64px × 20px` cell box, and has **no resize frame or handles** (it only scales if the row/column headers are dragged).
     - Unlike Excel 365, right-clicking a cell picture (`grid.rs:3269`) has no **「セルの上に配置 (`Place over Cells`)」** action (nor an on-selection pop-out button) to convert it into a floating resizable image, and right-clicking a floating image (`grid.rs:3397`) has no **「セルに配置 (`Place in Cell`)」** action.
  2. **7 out of 8 Resize Handles on Floating Pictures/Charts/Shapes Are Non-Functional (`chartview.rs:493-500` vs `grid.rs:2005, 2152`)**:
     - `chartview.rs:493-500` draws **8 white handle circles** (`left_top`, `center_top`, `right_top`, `left_center`, `right_center`, `left_bottom`, `center_bottom`, `right_bottom`) when an object is selected (`selected_chart == Some(id)`).
     - However, `grid.rs:2005` (hover cursor) and `grid.rs:2152` (drag start) **ONLY check `rect.right_bottom()`** (`(p - rect.right_bottom()).length() < OBJECT_RESIZE_RADIUS`)! Dragging any of the other 7 handles moves the image instead of resizing it.
  3. **Corner Handle Hit-Test & Auto-Selection Bugs on Floating Pictures (`chartview.rs:521`, `app.rs:3006-3043`)**:
     - In `chartview.rs:521`, `sh.images` hit-testing checks `r.contains(p)` *without* `.expand(...)` (whereas `sh.charts` at line 515 uses `r.expand(4.0).contains(p)`). Because the corner handle circle extends 4–8px outside `r`, hovering the outer half of the bottom-right handle misses `chartview::hit` completely, so neither the resize cursor (`CursorIcon::ResizeNwSe`) nor resize drag activates!
     - When inserting a floating picture (`insert.picture` with `placement: "overCells"`), `app.selected_chart` is not set to the new picture's `id` (unlike `insert.chart` in `dialogs.rs:2577`), so resize handles do not appear until the user explicitly clicks the image.
  4. **Direct Clipboard Screenshot Paste (`Cmd+V` / `Ctrl+V` with image data)**:
     - On Web/Desktop, pressing `Cmd+V` after taking an OS screenshot does not check clipboard image items (`clipboardData.items` with `image/png`) to insert a floating picture (`placement: "overCells"`).
- **Fix**:
  1. **Support all 8 resize handles** (`NW`, `N`, `NE`, `E`, `SE`, `S`, `SW`, `W`) in `grid.rs:2004-2007`, `2150-2160`, and `2500-2508` with matching cursor icons (`ResizeNwSe`, `ResizeNeSw`, `ResizeHorizontal`, `ResizeVertical`) and proper anchor/size updates (`x, y, width, height`).
  2. **Expand hit-test & auto-select**: Change `chartview.rs:521` and `527` to `r.expand(8.0).contains(p)` (matching `OBJECT_RESIZE_RADIUS = 8.0`), and auto-select newly inserted floating pictures/shapes (`app.selected_chart = Some(new_id)`).
  3. **Add conversion & clearer placement UI**:
     - Add right-click menu items to convert between **「セルの上に配置 (`Place over Cells`)」** and **「セルに配置 (`Place in Cell`)」** in `grid.rs:3269` (cell context menu when active cell has a `CellPicture`) and `grid.rs:3397` (`object_menu`).
     - In `ribbon.rs:1516-1526`, place **「セルの上に配置 (`Place over Cells`)」** (resizable floating image) first or clearly label both options, and wire Web `paste` event `image/*` blobs to `insert.picture` (`placement: "overCells"`).

### 16. [HIGH] Multi-Sheet Selection (`Shift+Click` / `Cmd+Click` / `Ctrl+Click` on Sheet Tabs) & "Select All Sheets" Are Unimplemented (`tabs.rs`, `DocState`)
- **Location**:
  - `crates/ui-egui/src/tabs.rs:84-90, 126-243` (`sheet_tabs`, `activate_sheet`, `"Select All Sheets"`)
  - `crates/engine/src/lib.rs:101-124` (`DocState`)
  - `crates/model/src/workbook.rs:141-150` (`Workbook::active_sheet`)
  - `crates/engine/src/cmd/sheet.rs:746-800, 926-977` (`delete_sheet`, `tab_color`, `hide_sheet`, `activate`)
- **What the User Noticed**:
  - The user added sheets with the `+` button and tried to select non-adjacent sheets with `Ctrl+Click` and a contiguous range of sheets with `Shift+Click` on the bottom Sheet tab bar, and asked whether this is related to the earlier cell multi-selection modifier behavior.
- **Root Cause**:
  1. **Multi-Sheet Selection (`selected_sheets` / Grouped Sheets) Is Completely Unimplemented in Both UI and Engine**:
     - `Workbook` (`workbook.rs:150`) and `DocState` (`engine/src/lib.rs:101-124`) only store a single `active_sheet: usize` and have **no `selected_sheets: Vec<usize>`** field at all!
     - In `tabs.rs:174-176`, clicking any sheet tab unconditionally runs `activate_sheet(app, i)` -> `sheet.activate` without checking `ui.input(|i| i.modifiers.shift)` or `modifiers.command` / `modifiers.ctrl`!
     - Even more tellingly, in the sheet tab right-click menu (`tabs.rs:239-241`), **"Select All Sheets" (`すべてのシートを選択`)** is literally a stub that only calls `ui.close()` and does nothing:
       ```rust
       if ui.button(app.ui.language.tr("Select All Sheets")).clicked() {
           ui.close();
       }
       ```
     - Nor is there an **"Ungroup Sheets" (`シートのグループ解除`)** menu item when multiple sheets are selected.
  2. **Bonus macOS Modifier Note (Shared with Cell Selection)**:
     - For **cells** (`grid.rs:2046-2049, 2369-2376`), `Shift+Click` (contiguous range) and **`Cmd+Click`** (`⌘+クリック`, non-adjacent cells) *do* work on macOS, whereas **`Ctrl+Click`** (`control_click = ctx.os().is_mac() && mods.ctrl && !mods.mac_cmd`) is intentionally treated as a macOS right-click (`secondary_clicked`) that opens the context menu.
     - However, for **sheet tabs** (`tabs.rs:174`), **neither `Shift+Click`, `Cmd+Click`, nor `Ctrl+Click` is implemented at all**!
- **Fix**:
  1. **Add `selected_sheets: Vec<usize>` to `DocState` (or `Workbook`)**:
     - Maintain `selected_sheets: Vec<usize>` (always containing `active_sheet`, defaulting to `vec![active_sheet]`).
     - Add engine commands or parameters on `sheet.activate`:
       - Normal click on sheet `i`: sets `active_sheet = i` and resets `selected_sheets = vec![i]` (unless `i` is clicked to ungroup, which also resets to `vec![i]`).
       - `Shift + Click` on sheet `i`: selects all visible sheets between `active_sheet` and `i` (`min..=max`), keeping `active_sheet` (or setting `active_sheet = i` with anchor).
       - `Cmd + Click` (and `Ctrl + Click` on Windows/Linux, or `Cmd + Click` on Mac) on sheet `i`: toggles `i` in `selected_sheets` (never allowing `selected_sheets` to become empty; if `active_sheet` is deselected, move `active_sheet` to another selected sheet).
       - `"Select All Sheets"` (`tabs.rs:239`): sets `selected_sheets` to all visible sheet indices, and show `"Ungroup Sheets"` (`シートのグループ解除`) in the tab context menu when `selected_sheets.len() > 1`.
  2. **Visual Highlight, 3-D Formula Pointing & Grouped Operations**:
     - In `tabs.rs:126-171`, highlight every tab in `selected_sheets` with the active/grouped background (`t.tab_active`) and underline band so the user clearly sees all selected sheets (and show `[Group]` / `[グループ]` in the title bar like Excel).
     - In formula point mode (`app.editor.as_ref().is_some_and(|e| e.can_point())`), `Shift + Click` on a sheet tab (`tabs.rs:28-30`) + clicking a cell (`grid.rs:2323-2327`) should emit a 3-D span reference `Sheet1:Sheet3!A1` across the selected sheet span.
     - Apply sheet tab commands (`home.deleteSheet`, `sheet.tabColor`, `sheet.hide`, `sheet.move`, and cell edits/formatting in group mode) across all `selected_sheets`.

### 17. [HIGH] Safari WASM Panic on Right-Click -> Paste Inside VBA Editor (`vba_editor.rs`, `clipboard.rs`)
- **Location**:
  - `crates/ui-egui/src/vba_editor.rs:413-431` (`code_editor`)
  - `crates/ui-egui/src/clipboard.rs` (Web clipboard read / paste handling)
- **What the User Noticed**:
  - On Safari (`http://127.0.0.1:8784/?sample=sales&lang=ja`), opening the VBA Editor (`View` -> `Macros` -> `Visual Basic`), right-clicking inside the code editor area, and selecting the browser's **"貼り付け (Paste)"** menu item triggers an immediate WASM panic overlay (`RuntimeError: Unreachable code should not be executed` inside `egui::containers::scroll_area::State::load` / `vba_editor::show` / `eframe`).
  - Pasting via keyboard (`Cmd+V`) works normally.
- **Root Cause & Fix**:
  1. Assign an explicit, stable `id_salt("vba_code_editor_text")` to `egui::TextEdit::multiline(&mut buf)` and `egui::ScrollArea::both().id_salt("vba_code_scroll")` in `vba_editor.rs:413-424` so widget state IDs never collide with `ScrollArea` state during Safari native context-menu focus/paste event dispatch.
  2. Provide an in-app right-click context menu (`resp.context_menu(|ui| ...)`) on the VBA code editor with **Cut (`切り取り`) / Copy (`コピー`) / Paste (`貼り付け`) / Select All (`すべて選択`)** wired to `app.clipboard_text` and Web async clipboard fallback (`navigator.clipboard.readText()`), while guarding against re-entrant `RefCell` borrows or corrupted `ScrollArea` state bytes when a native browser `paste` event arrives.

### 18. [HIGH] `More Functions` (`その他の関数`) on the `Formulas` (`数式`) Tab Hardcodes `"Statistical"` and Hides `Engineering`, `Information`, `Database`, `Compatibility`, `Web`, and `AddIn` (`ribbon.rs`)
- **Location**:
  - `crates/ui-egui/src/ribbon.rs:1934-1964` (`tab_formulas` -> `Function Library`)
  - `crates/engine/src/cmd/formulas.rs:180-232` (`function_list`)
- **What the User Noticed**:
  - The user asked whether all functions on the `Formulas` (`数式`) tab work and can be verified.
- **Root Cause**:
  - While all 400+ functions in `gridcraft_functions::all()` and 33 special forms in `formulas::function_list()` are implemented and pass all unit tests, `ribbon.rs:1941` defines `("More\nFunctions", "Statistical", "formulas.moreFunctions")` with `cat = "Statistical"` hardcoded!
  - Consequently, clicking `More Functions` (`その他の関数`) only lists `Statistical` (`統計`) functions and completely hides **`Engineering` (`エンジニアリング`, e.g., `HEX2DEC`, `BIN2DEC`, `CONVERT`, `IMABS`)**, **`Information` (`情報`, e.g., `ISBLANK`, `ISNUMBER`, `CELL`, `INFO`, `SHEET`, `SHEETS`, `TYPE`)**, **`Database` (`データベース`, e.g., `DSUM`, `DAVERAGE`, `DCOUNT`, `DGET`)**, **`Compatibility` (`互換性`)**, **`Web` (`ENCODEURL`)**, and **`AddIn` (`EUROCONVERT`)** from the Ribbon.
- **Fix**:
  - Replace the single-category popup on `More Functions` (`その他の関数`) in `ribbon.rs:1941-1963` with a categorized submenu matching commercial Excel:
    - `Statistical` (`統計`)
    - `Engineering` (`エンジニアリング`)
    - `Information` (`情報`)
    - `Database` (`データベース`)
    - `Compatibility` (`互換性`)
    - `Web` (`Web`) / `Add-in and Automation` (`アドインとオートメーション`)
  - Also add `"More Functions…"` (`その他の関数…`) at the bottom of each category dropdown to open `Dialog::InsertFunction` pre-filtered to that category.

### 19. [MEDIUM] `AutoSum` (`オートSUM`) on the `Formulas` (`数式`) Tab Shows a Dropdown Arrow (`true`) But Has No Dropdown Menu (`ribbon.rs`)
- **Location**:
  - `crates/ui-egui/src/ribbon.rs:1929-1933` (`Formulas` tab `AutoSum`) vs `crates/ui-egui/src/ribbon.rs:1469-1485` (`Home` tab `AutoSum`)
- **Root Cause**:
  - On the `Home` (`ホーム`) tab (`ribbon.rs:1469-1485`), `AutoSum` opens a popup menu with `Sum` (`合計`), `Average` (`平均`), `Count Numbers` (`数値の個数`), `Max` (`最大値`), `Min` (`最小値`), and `More Functions…` (`その他の関数…`).
  - On the `Formulas` (`数式`) tab (`ribbon.rs:1929-1933`), `big_button(ui, Icon::Sum, "AutoSum", "AutoSum", true)` draws the dropdown arrow (`true`), but clicking it immediately runs `formulas.autoSum` (`SUM` only) with no dropdown menu.
- **Fix**:
  - Wire the `Formulas` tab `AutoSum` button (`ribbon.rs:1929-1933`) to the same dropdown menu (`Sum`, `Average`, `Count Numbers`, `Max`, `Min`, `More Functions…`) as the `Home` tab (`ribbon.rs:1469-1485`).

### 20. [MEDIUM] Selecting a Function from a Ribbon Category Dropdown Overwrites an In-Progress Formula Instead of Inserting at the Caret (`ribbon.rs`, `formulas.rs`, `dialogs.rs`)
- **Location**:
  - `crates/ui-egui/src/ribbon.rs:1957-1959`
  - `crates/engine/src/cmd/formulas.rs:167-172` (`insert_function`)
  - `crates/ui-egui/src/dialogs.rs:2137-2145` (`insert_function` dialog)
- **Root Cause**:
  - In `dialogs.rs:2138-2143` (`Insert Function` dialog), if the user is already editing a formula starting with `=` (e.g., `=10+`), selecting a function appends `NAME(` -> `=10+NAME(`.
  - However, in `ribbon.rs:1958`, selecting a function from any Ribbon category dropdown (`Financial`, `Logical`, `Text`, `Date & Time`, `Lookup & Reference`, `Math & Trig`, `More Functions`) calls `formulas.insertFunction`, which unconditionally sends `UiRequest::EditCell(Some("=NAME("))` (`formulas.rs:170`), wiping out the user's in-progress formula in `app.editor`.
- **Fix**:
  - When `app.editor` is already active and editing a formula (`e.text.starts_with('=')`), selecting a function from a Ribbon category dropdown (or `UiRequest::EditCell` when `formulas.insertFunction` is called) should insert `NAME(` at the current cursor/caret position (or append if at the end) rather than overwriting the entire formula text with `=NAME(`.

### 21. [HIGH] Web/デスクトップでの「名前を付けて保存 (Save As)」時にファイル名入力ダイアログを表示する (`crates/ui-egui/src/dialogs.rs`, `crates/ui-egui/src/lib.rs`, `crates/ui-egui/src/backstage.rs`)
- **Location**:
  - `crates/ui-egui/src/lib.rs:854-870` (`open_dialog("saveAs", ...)`)
  - `crates/ui-egui/src/dialogs.rs` (`save_as` / 新規 `Dialog::SaveAs` またはファイル名プロンプト)
  - `crates/ui-egui/src/backstage.rs:360-390` (`Backstage::show` -> Save As ページ)
- **What the User Noticed**:
  - Web版（Safari等）において、Backstage（「ファイル」タブ）やリボンから「名前を付けて保存（Save As）」をクリックすると、ファイル名を入力・変更する余地がなく、現在のブック名（`Book1.xlsx` や `Quarterly Sales Dashboard.xlsx`）のまま即座に自動ダウンロードされてしまう。
  - ユーザーが任意のファイル名（例: `PWGridCraft.xlsx` や `2026年度売上.xlsx`）に変更して保存できない。
- **Root Cause**:
  - `crates/ui-egui/src/lib.rs:854-870` の `match name { "saveAs" => ... }` において、`services.pick_save` が存在しない環境（Web/WASM版）では、ダイアログを出さずに即座に `file.saveBytes` を呼び出し、`services.download(&name, &b)` を実行してしまっている。
  - また、Backstage の `Save As` 画面でもファイル名入力フィールドが存在しない、あるいはダイアログ連携が不足している。
- **Fix**:
  1. **ファイル名入力ダイアログ（Save As Dialog）の実装**:
     - `services.pick_save` が利用できない環境（Web環境）、あるいは「名前を付けて保存」が呼び出された際、モーダルダイアログ（例: `Dialog::SaveAs`）を表示する。
     - ダイアログ内には以下を配置：
       - 現在のファイル名（`.xlsx` 拡張子付きまたは拡張子なしの入力欄、初期値はアクティブなブック名、例: `Book1`）。入力欄に初期フォーカスを当てる。
       - ファイル形式（`Excel ワークブック (*.xlsx)` / 将来拡張用の選択、または固定表示）。
       - 「保存 (Save)」ボタン（Enterキー対応）および「キャンセル (Cancel)」ボタン（Escキー対応）。
     - 保存ボタン押下時：
       - 入力されたファイル名が空でないことを確認し、拡張子 `.xlsx` がなければ自動補完する。
       - 確定したファイル名で `file.saveBytes`（Web版なら `services.download(custom_name, bytes)`、デスクトップなら指定パスへの保存）を実行する。
       - ドキュメントのアクティブタイトル（`d.title` や表示名）を入力された名称に更新する。
  2. **回帰テスト**:
     - Web環境（`download` サービスモック）において、「名前を付けて保存」をトリガーした際にダイアログが開き、任意のファイル名を入力して確定すると、そのカスタムファイル名でダウンロードペイロードが発行されることを検証するユニット/UIテストを追加する。
