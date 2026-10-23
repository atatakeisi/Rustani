//! The VBA editor preserves drafts, requires explicit execution, and remains stoppable.

use egui_kittest::kittest::{NodeT, Queryable};
use gridcraft_engine::{Session, core::CellRef};
use gridcraft_ui_egui::SheetApp;
use gridcraft_ui_egui::control::{Outcome, handle};
use serde_json::{Value as Json, json};

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 900.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    harness.run_steps(4);
    harness
}

fn status(harness: &mut Harness) -> Json {
    harness.state_mut().run("automate.vbaStatus", json!({})).unwrap()
}

#[test]
fn japanese_controls_execute_and_save_the_same_vba_and_switch_back_to_english() {
    let mut h = harness();
    h.state_mut().ui.language = gridcraft_ui_egui::i18n::Language::Ja;
    h.state_mut().open_dialog("macros", json!({}));
    h.run_steps(4);
    assert!(h.query_by_label("記録したスクリプト").is_some());
    h.get_by_label("集計のサンプルを挿入").click();
    h.run_steps(3);
    h.get_by_label("VBA を実行").click();
    h.run_steps(5);
    assert_eq!(status(&mut h)["state"], "completed");
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().value(CellRef::parse("E1").unwrap()).display(), "Summary");
    h.get_by_label("モジュールを保存").click();
    h.run_steps(3);
    let modules = h.state_mut().run("automate.vbaModules", json!({})).unwrap();
    assert!(modules["modules"][0]["source"].as_str().unwrap().starts_with("Sub MakeSummary()"));
    h.state_mut().ui.language = gridcraft_ui_egui::i18n::Language::En;
    h.run_steps(3);
    assert!(h.query_by_label("Run VBA").is_some());
    assert!(h.query_by_label("Save module").is_some());
    assert_eq!(status(&mut h)["state"], "completed");
}

#[test]
fn example_insertion_is_explicit_and_execution_reports_output() {
    let mut h = harness();
    h.state_mut().open_dialog("macros", json!({}));
    h.run_steps(4);
    h.get_by_label("Insert summary example").click();
    h.run_steps(3);
    assert!(!h.state().session.active().unwrap().is_dirty(), "pasting the example must not run it");
    h.get_by_label("Run VBA").click();
    h.run_steps(5);
    let status = status(&mut h);
    assert_eq!(status["state"], "completed", "{status}");
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().value(CellRef::parse("E1").unwrap()).display(), "Summary");
    assert!(status["log"].as_array().unwrap().iter().any(|line| line.as_str().is_some_and(|text| text.contains("Summary created"))));
}

#[test]
fn stop_remains_responsive_while_a_procedure_is_running() {
    let mut h = harness();
    h.state_mut().open_dialog("vbaEditor", json!({"source": "Sub Spin()\nDo\nLoop\nEnd Sub", "procedure": "Spin"}));
    h.run_steps(3);
    h.get_by_label("Run VBA").click();
    h.run_steps(3);
    assert_eq!(status(&mut h)["state"], "running");
    h.get_by_label("Stop VBA").click();
    h.run_steps(3);
    let stopped = status(&mut h);
    assert_eq!(stopped["state"], "stopped", "{stopped}");
    h.run_steps(5);
    assert_eq!(status(&mut h)["steps"], stopped["steps"]);
    assert!(!h.state().session.active().unwrap().is_dirty(), "stopped execution must not commit staged edits");
}

#[test]
fn closing_an_editor_stops_execution_and_preserves_the_unsaved_draft() {
    let mut h = harness();
    h.state_mut().open_dialog("vbaEditor", json!({"source": "Sub Spin()\nDo\nLoop\nEnd Sub", "procedure": "Spin"}));
    h.run_steps(3);
    h.get_by_label("Run VBA").click();
    h.run_steps(3);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert_eq!(status(&mut h)["state"], "stopped");
    assert!(h.state().dialog.is_some());
    h.get_by_label("Keep editing").click();
    h.run_steps(3);
    assert!(h.query_by_label("Unsaved changes").is_some());
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    h.get_by_label("Close without saving").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert_eq!(h.state_mut().run("automate.vbaModules", json!({})).unwrap()["modules"], json!([]));
}

#[test]
fn stored_modules_require_explicit_trust_before_execution() {
    let mut h = harness();
    h.state_mut().run("automate.saveVbaModule", json!({"name": "Saved", "source": "Sub Main()\nRange(\"A1\").Value = 7\nEnd Sub"})).unwrap();
    h.state_mut().run("automate.trustScripts", json!({"trusted": false})).unwrap();
    h.state_mut().open_dialog("vbaEditor", json!({"module": "Saved"}));
    h.run_steps(3);
    assert!(h.query_by_label("Trust Scripts").is_some());
    assert!(h.get_by_label("Run VBA").accesskit_node().is_disabled());
    h.get_by_label("Trust Scripts").click();
    h.run_steps(3);
    h.get_by_label("Run VBA").click();
    h.run_steps(3);
    assert_eq!(status(&mut h)["state"], "completed");
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().value(CellRef::parse("A1").unwrap()).display(), "7");
}

#[test]
fn a_new_draft_in_an_untrusted_workbook_keeps_the_trust_action_available() {
    let mut h = harness();
    h.state_mut().run("automate.trustScripts", json!({"trusted": false})).unwrap();
    h.state_mut().open_dialog("vbaEditor", json!({"source": "Sub Main()\nRange(\"A1\").Value = 8\nEnd Sub", "procedure": "Main"}));
    h.run_steps(3);
    assert!(h.query_by_label("Trust Scripts").is_some());
    assert!(h.get_by_label("Run VBA").accesskit_node().is_disabled());
    h.get_by_label("Trust Scripts").click();
    h.run_steps(3);
    h.get_by_label("Run VBA").click();
    h.run_steps(3);
    assert_eq!(status(&mut h)["state"], "completed");
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().value(CellRef::parse("A1").unwrap()).display(), "8");
}

#[test]
fn invalid_source_can_be_saved_for_further_editing_without_running_it() {
    let mut h = harness();
    let source = "Sub NeedsWork()\nRange(\"A1\").Value =\nEnd Sub";
    h.state_mut().open_dialog("vbaEditor", json!({"name": "Draft", "source": source, "procedure": "NeedsWork"}));
    h.run_steps(3);
    h.get_by_label("Save module").click();
    h.run_steps(3);
    let modules = h.state_mut().run("automate.vbaModules", json!({})).unwrap();
    assert!(modules["modules"].as_array().unwrap().iter().any(|module| module["name"] == "Draft" && module["source"] == source));
    assert!(h.state().session.active().unwrap().wb.active().unwrap().value(CellRef::parse("A1").unwrap()).is_empty());
    assert_ne!(status(&mut h)["state"], "running");
}

#[test]
fn new_script_menu_opens_the_editor_without_changing_programmatic_commands() {
    let mut h = harness();
    assert!(h.state_mut().run("automate.newScript", json!({})).is_err());
    assert!(h.state().dialog.is_none());
    h.state_mut().run_or_alert("automate.newScript", json!({}));
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("vbaEditor"));
    assert!(h.query_by_label("Run VBA").is_some());
}

#[test]
fn replacing_the_dialog_stops_execution_and_requires_resolving_the_draft() {
    let mut h = harness();
    h.state_mut().open_dialog("vbaEditor", json!({"source": "Sub Spin()\nDo\nLoop\nEnd Sub", "procedure": "Spin"}));
    h.run_steps(3);
    h.get_by_label("Run VBA").click();
    h.run_steps(3);
    h.state_mut().open_dialog("zoom", json!({}));
    h.run_steps(3);
    assert_eq!(status(&mut h)["state"], "stopped");
    assert_eq!(h.state().dialog.as_ref().unwrap().name(), "vbaEditor");
    assert!(h.query_by_label("Save changes before closing?").is_some());
    h.get_by_label("Close without saving").click();
    h.run_steps(3);
    h.state_mut().open_dialog("zoom", json!({}));
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().unwrap().name(), "zoom");
}

#[test]
fn control_cancellation_stops_execution_and_requires_explicit_draft_discard() {
    let mut h = harness();
    h.state_mut().open_dialog("vbaEditor", json!({"source": "Sub Spin()\nDo\nLoop\nEnd Sub", "procedure": "Spin"}));
    h.run_steps(3);
    h.get_by_label("Run VBA").click();
    h.run_steps(3);
    let ctx = h.ctx.clone();
    let result = handle(h.state_mut(), &ctx, "ui.dialog.cancel", &json!({}));
    assert!(matches!(result, Outcome::Done(value) if value["ok"] == false));
    assert_eq!(status(&mut h)["state"], "stopped");
    assert!(h.state().dialog.is_some());
    let result = handle(h.state_mut(), &ctx, "ui.dialog.cancel", &json!({"discardEdits": true}));
    assert!(matches!(result, Outcome::Done(value) if value["ok"] == true));
    assert!(h.state().dialog.is_none());
}
