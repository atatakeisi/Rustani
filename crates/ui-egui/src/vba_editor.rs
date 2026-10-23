//! Workbook VBA editor. Execution and validation belong to engine commands.

use std::fmt::Write as _;

use egui::{RichText, TextEdit};
use serde_json::{Value as Json, json};

use crate::{SheetApp, theme};

const SAMPLE: &str = "Sub MakeSummary()\n    Range(\"E1\").Value = \"Summary\"\n    Range(\"E2\").Formula = \"=SUM(B2:B10)\"\n    Range(\"E1\").Font.Bold = True\n    Debug.Print \"Summary created in E1:E2\"\nEnd Sub\n";
const SOURCE_LIMIT: usize = 1_048_576;

#[derive(Clone, Debug)]
struct Module {
    name: String,
    source: String,
    procedures: Vec<String>,
    compile_error: Option<String>,
    imported: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct VbaEditor {
    owner: Option<u64>,
    modules: Vec<Module>,
    name: String,
    source: String,
    saved_name: String,
    saved_source: String,
    procedure: String,
    stored_origin: bool,
    trusted: bool,
    preserved_vba: bool,
    import_error: Option<String>,
    status: Json,
    error: Option<String>,
    close_requested: bool,
    overwrite_requested: bool,
}

impl VbaEditor {
    pub(crate) fn new(app: &mut SheetApp, params: &Json) -> Self {
        let mut editor = Self {
            owner: app.view_key().map(|key| key.0),
            modules: vec![],
            name: "Module1".into(),
            source: String::new(),
            saved_name: "Module1".into(),
            saved_source: String::new(),
            procedure: "Main".into(),
            stored_origin: false,
            trusted: false,
            preserved_vba: false,
            import_error: None,
            status: json!({"state": "idle"}),
            error: None,
            close_requested: false,
            overwrite_requested: false,
        };
        editor.refresh(app);
        if let Some(name) = params.get("module").and_then(Json::as_str) {
            editor.select(name);
        } else if let Some(name) = editor.modules.first().map(|m| m.name.clone()) {
            editor.select(&name);
        }
        for key in ["name", "source", "procedure"] {
            if let Some(value) = params.get(key) {
                editor.set_value(key, value);
            }
        }
        if let Ok(status) = app.run("automate.vbaStatus", json!({})) {
            editor.status = status;
        }
        editor
    }

    pub(crate) fn set_value(&mut self, key: &str, value: &Json) {
        let Some(text) = value.as_str() else { return };
        match key {
            "name" => self.name = text.to_string(),
            "source" => self.source = text.chars().take(SOURCE_LIMIT).collect(),
            "procedure" => self.procedure = text.to_string(),
            _ => {}
        }
    }

    fn same_workbook(&self, app: &SheetApp) -> bool {
        self.owner.is_some() && self.owner == app.view_key().map(|key| key.0)
    }

    fn dirty(&self) -> bool {
        self.name != self.saved_name || self.source != self.saved_source
    }

    fn running(&self) -> bool {
        self.status.get("state").and_then(Json::as_str) == Some("running")
    }

    pub(crate) fn refresh(&mut self, app: &mut SheetApp) {
        match app.run("automate.vbaModules", json!({})) {
            Ok(value) => {
                self.trusted = value.get("trusted").and_then(Json::as_bool).unwrap_or(false);
                self.preserved_vba = value.get("preservedVba").and_then(Json::as_bool).unwrap_or(false);
                self.import_error = value.get("importError").and_then(Json::as_str).map(str::to_string);
                self.modules = value
                    .get("modules")
                    .and_then(Json::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| {
                                Some(Module {
                                    name: item.get("name")?.as_str()?.to_string(),
                                    source: item.get("source")?.as_str()?.to_string(),
                                    imported: item.get("origin").and_then(Json::as_str) == Some("project"),
                                    compile_error: item.get("compileError").filter(|value| !value.is_null()).map(|value| {
                                        if let (Some(line), Some(message)) =
                                            (value.get("line").and_then(Json::as_u64), value.get("message").and_then(Json::as_str))
                                        {
                                            format!("Line {line}: {message}")
                                        } else {
                                            value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string())
                                        }
                                    }),
                                    procedures: item
                                        .get("procedures")
                                        .and_then(Json::as_array)
                                        .map(|items| items.iter().filter_map(Json::as_str).map(str::to_string).collect())
                                        .unwrap_or_default(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            }
            Err(error) => self.error = Some(crate::clean_error(&error)),
        }
    }

    fn select(&mut self, name: &str) {
        if let Some(module) = self.modules.iter().find(|m| m.name.eq_ignore_ascii_case(name)) {
            self.name.clone_from(&module.name);
            self.source.clone_from(&module.source);
            self.saved_name.clone_from(&module.name);
            self.saved_source.clone_from(&module.source);
            self.procedure = module.procedures.first().cloned().unwrap_or_default();
            self.stored_origin = true;
            self.error = None;
        }
    }

    fn new_module(&mut self) {
        let mut suffix = self.modules.len().saturating_add(1);
        let mut name = format!("Module{suffix}");
        while self.modules.iter().any(|module| module.name.eq_ignore_ascii_case(&name)) {
            let next = suffix.saturating_add(1);
            if next == suffix {
                return;
            }
            suffix = next;
            name = format!("Module{suffix}");
        }
        self.name = name;
        self.source.clear();
        self.saved_name.clone_from(&self.name);
        self.saved_source.clear();
        self.procedure = "Main".into();
        self.stored_origin = false;
        self.error = None;
    }

    fn save(&mut self, app: &mut SheetApp, replace: bool) -> bool {
        if !self.same_workbook(app) || self.running() {
            return false;
        }
        if !replace
            && !self.name.eq_ignore_ascii_case(&self.saved_name)
            && self.modules.iter().any(|module| module.name.eq_ignore_ascii_case(&self.name))
        {
            self.overwrite_requested = true;
            return false;
        }
        match app.run("automate.saveVbaModule", json!({"name": self.name, "source": self.source})) {
            Ok(value) => {
                if let Some(name) = value.get("name").and_then(Json::as_str) {
                    self.name = name.to_string();
                }
                self.saved_name.clone_from(&self.name);
                self.saved_source.clone_from(&self.source);
                self.stored_origin = true;
                self.error = None;
                self.overwrite_requested = false;
                self.refresh(app);
                true
            }
            Err(error) => {
                self.error = Some(crate::clean_error(&error));
                false
            }
        }
    }

    fn start(&mut self, app: &mut SheetApp) {
        if !self.same_workbook(app) || self.running() {
            return;
        }
        self.refresh(app);
        if !self.trusted {
            return;
        }
        self.error = None;
        // All execution goes through the engine's workbook trust check. Drafts can run
        // without being silently saved to the workbook.
        let params = if self.stored_origin && !self.dirty() {
            json!({"module": self.name, "procedure": self.procedure})
        } else {
            json!({"source": self.source, "procedure": self.procedure})
        };
        match app.run("automate.startVba", params) {
            Ok(status) => self.status = status,
            Err(error) => {
                self.error = Some(crate::clean_error(&error));
                if let Ok(status) = app.run("automate.vbaStatus", json!({})) {
                    self.status = status;
                }
            }
        }
    }

    fn stop(&mut self, app: &mut SheetApp) {
        if self.running() {
            match app.run("automate.stopVba", json!({})) {
                Ok(status) => self.status = status,
                Err(error) => self.error = Some(crate::clean_error(&error)),
            }
        }
    }

    /// Moving to recorded scripts must not leave hidden VBA running.
    pub(crate) fn pause_for_tab(&mut self, app: &mut SheetApp) {
        self.stop(app);
    }

    /// Closing always stops execution. Unsaved source stays available until the user
    /// explicitly saves or closes without saving.
    pub(crate) fn request_close(&mut self, app: &mut SheetApp) -> bool {
        self.stop(app);
        if self.running() {
            return false;
        }
        if self.dirty() {
            self.close_requested = true;
            false
        } else {
            true
        }
    }

    pub(crate) fn discard_edits(&mut self) {
        self.name.clone_from(&self.saved_name);
        self.source.clone_from(&self.saved_source);
        self.overwrite_requested = false;
    }

    pub(crate) fn show(&mut self, app: &mut SheetApp, ui: &mut egui::Ui, open: &mut bool) {
        let tokens = theme::Tokens::get(ui.ctx());
        let same_workbook = self.same_workbook(app);
        if !same_workbook {
            self.stop(app);
            ui.colored_label(tokens.danger, "Return to the original workbook to edit or run this module.");
        }
        ui.label("Edit or paste VBA, then choose a procedure and Run. Unsupported code reports an error.");
        if self.preserved_vba {
            ui.weak("The original VBA project is preserved in .xlsm files. Excel VBA compatibility is still incomplete.");
        }
        if self.modules.iter().any(|module| module.name == self.saved_name && module.imported) {
            ui.weak("Saving source updates GridCraft's copy; the original Excel macro project is preserved.");
        }
        if let Some(error) = &self.import_error {
            ui.colored_label(tokens.danger, format!("VBA source import: {error}"));
        }
        if !self.dirty()
            && let Some(error) = self.modules.iter().find(|module| module.name == self.saved_name).and_then(|module| module.compile_error.as_ref())
        {
            ui.colored_label(tokens.danger, format!("Module error: {error}"));
        }
        let running = self.running();
        // Stop is handled before the cooperative execution slice at the end of this frame.
        ui.horizontal(|ui| {
            if ui.add_enabled(same_workbook && !running && self.trusted && !self.source.trim().is_empty(), egui::Button::new("Run VBA")).clicked() {
                self.start(app);
            }
            if ui.add_enabled(same_workbook && running, egui::Button::new("Stop VBA")).clicked() {
                self.stop(app);
            }
            let state = self.status.get("state").and_then(Json::as_str).unwrap_or("idle");
            let line = self.status.get("line").and_then(Json::as_u64).unwrap_or(0);
            let steps = self.status.get("steps").and_then(Json::as_u64).unwrap_or(0);
            ui.label(format!("{state} · line {line} · {steps} steps"));
        });
        if !self.trusted {
            ui.horizontal_wrapped(|ui| {
                ui.weak("Macros are disabled until you trust this workbook.");
                if ui.add_enabled(same_workbook, egui::Button::new("Trust Scripts")).clicked() {
                    match app.run("automate.trustScripts", json!({"trusted": true})) {
                        Ok(_) => self.refresh(app),
                        Err(error) => self.error = Some(crate::clean_error(&error)),
                    }
                }
            });
        }
        ui.separator();
        let editable = same_workbook && !self.running();
        let dirty = self.dirty();
        ui.add_enabled_ui(editable, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!dirty, |ui| {
                    let mut selected = self.saved_name.clone();
                    egui::ComboBox::from_id_salt("vba_module")
                        .width(160.0)
                        .selected_text(if self.stored_origin { self.saved_name.as_str() } else { "New module" })
                        .show_ui(ui, |ui| {
                            for module in &self.modules {
                                ui.selectable_value(&mut selected, module.name.clone(), &module.name);
                            }
                        });
                    if selected != self.saved_name {
                        self.select(&selected);
                    }
                    if ui.button("New module").clicked() {
                        self.new_module();
                    }
                });
                if ui.add_enabled(!self.source.is_empty() || dirty, egui::Button::new("Save module")).clicked() {
                    self.save(app, false);
                }
                if dirty {
                    ui.colored_label(tokens.accent, "Unsaved changes");
                    if ui.button("Discard edits").clicked() {
                        self.discard_edits();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Module name:");
                ui.add(TextEdit::singleline(&mut self.name).desired_width(150.0).char_limit(128));
                ui.label("Procedure:");
                ui.add(TextEdit::singleline(&mut self.procedure).desired_width(150.0).char_limit(128));
                let procedures = procedure_suggestions(&self.source);
                if !procedures.is_empty() {
                    egui::ComboBox::from_id_salt("vba_procedure").selected_text("Choose…").show_ui(ui, |ui| {
                        for name in procedures {
                            ui.selectable_value(&mut self.procedure, name.clone(), name);
                        }
                    });
                }
            });
            if self.overwrite_requested {
                ui.horizontal(|ui| {
                    ui.colored_label(tokens.danger, format!("Replace module \"{}\"?", self.name));
                    if ui.button("Replace module").clicked() {
                        self.save(app, true);
                    }
                    if ui.button("Cancel replacement").clicked() {
                        self.overwrite_requested = false;
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(self.source.trim().is_empty(), egui::Button::new("Insert summary example")).clicked() {
                    self.source = SAMPLE.into();
                    self.procedure = "MakeSummary".into();
                }
                ui.weak("The example writes to E1:E2 only when you press Run.");
            });
            let height = (ui.ctx().content_rect().height() * 0.32).clamp(130.0, 300.0);
            let line_count = self.source.lines().count().saturating_add(usize::from(self.source.ends_with('\n'))).max(1);
            let mut numbers = String::new();
            for line in 1..=line_count {
                if line > 1 {
                    numbers.push('\n');
                }
                let _ = write!(numbers, "{line}");
            }
            egui::ScrollArea::both().id_salt("vba_source_scroll").max_height(height).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.add(egui::Label::new(RichText::new(numbers).monospace().color(tokens.text_dim)).selectable(false));
                    ui.add(
                        TextEdit::multiline(&mut self.source)
                            .id_salt("vba_source")
                            .code_editor()
                            .desired_width((ui.available_width() - 8.0).max(240.0))
                            .desired_rows(10)
                            .char_limit(SOURCE_LIMIT),
                    );
                });
            });
        });
        ui.separator();
        if let Some(error) = &self.error {
            ui.colored_label(tokens.danger, error);
        }
        if let Some(error) = self.status.get("error").filter(|error| error.is_object()) {
            let line = error.get("line").and_then(Json::as_u64).unwrap_or(0);
            let message = error.get("message").and_then(Json::as_str).unwrap_or("Execution failed");
            ui.colored_label(tokens.danger, format!("Line {line}: {message}"));
        }
        egui::ScrollArea::vertical().id_salt("vba_output").max_height(100.0).stick_to_bottom(true).show(ui, |ui| {
            if let Some(log) = self.status.get("log").and_then(Json::as_array) {
                for entry in log.iter().filter_map(Json::as_str) {
                    ui.monospace(entry);
                }
            }
        });
        if self.close_requested {
            ui.horizontal_wrapped(|ui| {
                ui.label("Save changes before closing?");
                if ui.add_enabled(editable, egui::Button::new("Save and close")).clicked() && self.save(app, false) {
                    *open = false;
                }
                if ui.button("Close without saving").clicked() {
                    self.discard_edits();
                    *open = false;
                }
                if ui.button("Keep editing").clicked() {
                    self.close_requested = false;
                }
            });
        }
        if same_workbook && self.running() {
            match app.run("automate.pollVba", json!({"budget": 1000})) {
                Ok(status) => self.status = status,
                Err(error) => self.error = Some(crate::clean_error(&error)),
            }
            ui.ctx().request_repaint();
        }
    }
}

/// Suggestions only: the engine is the authority on syntax and callable procedures.
fn procedure_suggestions(source: &str) -> Vec<String> {
    let mut procedures = vec![];
    for line in source.lines() {
        let mut line = line.trim();
        for _ in 0..3 {
            let Some((keyword, rest)) = line.split_once(char::is_whitespace) else { break };
            if ["public", "private", "static"].iter().any(|prefix| keyword.eq_ignore_ascii_case(prefix)) {
                line = rest.trim_start();
            } else {
                break;
            }
        }
        let Some((keyword, rest)) = line.split_once(char::is_whitespace) else { continue };
        if !keyword.eq_ignore_ascii_case("sub") {
            continue;
        }
        let name = rest.trim().split(['(', ' ', '\t', ':']).next().unwrap_or("");
        if !name.is_empty() && !procedures.iter().any(|existing: &String| existing.eq_ignore_ascii_case(name)) {
            procedures.push(name.to_string());
        }
        if procedures.len() >= 1000 {
            break;
        }
    }
    procedures
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> SheetApp {
        let mut session = gridcraft_engine::Session::new();
        session.new_workbook();
        SheetApp::new(session, Default::default())
    }

    #[test]
    fn editor_starts_without_changing_cells_and_close_keeps_a_dirty_draft() {
        let mut app = app();
        let mut editor = VbaEditor::new(&mut app, &json!({"source": SAMPLE, "procedure": "MakeSummary"}));
        assert!(editor.dirty());
        assert!(!app.session.active().unwrap().is_dirty());
        assert!(!editor.request_close(&mut app));
        assert!(editor.close_requested);
        assert_eq!(editor.source, SAMPLE);
    }

    #[test]
    fn switching_workbooks_prevents_saving_a_draft_to_the_wrong_file() {
        let mut app = app();
        let mut editor = VbaEditor::new(&mut app, &json!({"source": SAMPLE}));
        app.session.new_workbook();
        assert!(!editor.same_workbook(&app));
        assert!(!editor.save(&mut app, false));
        assert_eq!(app.run("automate.vbaModules", json!({})).unwrap()["modules"], json!([]));
    }

    #[test]
    fn procedure_choices_include_multiple_procedures_without_treating_comments_as_code() {
        assert_eq!(procedure_suggestions("' Sub Ignored()\nPublic Sub First()\nEnd Sub\nsub second()\nEnd Sub\n"), vec!["First", "second"]);
    }
}
