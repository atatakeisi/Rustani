//! Macro entry points and recorded-script actions through the real UI.
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;
fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1000.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.run_steps(4);
    h
}

fn click_control(h: &Harness, label: &str, status: bool) {
    if status {
        h.get_all_by_label(label).max_by(|a, b| a.rect().center().y.total_cmp(&b.rect().center().y)).unwrap().click();
    } else {
        h.get_all_by_label(label).min_by(|a, b| a.rect().center().y.total_cmp(&b.rect().center().y)).unwrap().click();
    }
}

#[test]
fn status_opens_macros_and_view_and_automate_have_recording_controls() {
    let mut h = harness();
    assert!(h.state_mut().session.execute("view.macros", json!({})).unwrap().is_array());
    assert!(h.state().dialog.is_none());
    click_control(&h, "Macros", false);
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().unwrap().name(), "macros");
    h.state_mut().dialog = None;
    h.run_steps(2);
    click_control(&h, "Macros", true);
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().unwrap().name(), "macros");
    h.state_mut().dialog = None;
    for tab in ["View", "Automate"] {
        h.state_mut().ui.ribbon_tab = tab.into();
        h.run_steps(3);
        assert!(h.query_all_by_label("Macros").count() >= 2, "status bar and {tab}");
        assert!(h.query_all_by_label("Record Macro").count() >= 2, "status bar and {tab}");
    }
    h.get_by_label("VBA Editor").click();
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().unwrap().name(), "macros");
    assert!(h.query_by_label("Run VBA").is_some());
}

#[test]
fn status_record_and_stop_save_vba_and_open_its_editor() {
    for status in [false, true] {
        let mut h = harness();
        click_control(&h, "Record Macro", status);
        h.run_steps(2);
        assert!(h.state().session.recording.is_some());
        h.state_mut().session.execute("selection.set", json!({"cell":"B2"})).unwrap();
        h.state_mut().begin_edit(Some("42".into()), false);
        h.run_steps(2);
        click_control(&h, "Stop Recording", status);
        h.run_steps(3);
        assert!(h.state().session.recording.is_none());
        assert!(h.state().editor.is_none(), "stopping commits the pending cell first");
        assert!(matches!(h.state().dialog.as_ref().unwrap().name(), "macros" | "vbaEditor"));
        assert!(h.query_by_label("Run VBA").is_some());
        let modules = h.state_mut().session.execute("automate.vbaModules", json!({})).unwrap();
        assert_eq!(modules["modules"][0]["procedures"], json!(["Macro1"]));
        assert!(modules["modules"][0]["source"].as_str().unwrap().contains("42"), "the pending edit is part of the recording");
    }
}

#[test]
fn japanese_quick_access_and_status_record_controls_keep_the_same_actions() {
    let mut h = harness();
    h.state_mut().ui.language = gridcraft_ui_egui::i18n::Language::Ja;
    h.run_steps(3);
    assert_eq!(h.query_all_by_label("マクロ").count(), 1);
    assert_eq!(h.query_all_by_label("マクロの記録").count(), 1);
    click_control(&h, "マクロの記録", false);
    h.run_steps(2);
    assert!(h.state().session.recording.is_some());
    assert_eq!(h.query_all_by_label("記録終了").count(), 2);
    click_control(&h, "記録終了", true);
    h.run_steps(3);
    assert!(h.state().session.recording.is_none());
    assert!(h.query_by_label("VBA を実行").is_some());
}

#[test]
fn recorded_scripts_have_a_delete_action() {
    let mut h = harness();
    h.state_mut().session.execute("automate.newScript", json!({"name":"Remove me","commands":[]})).unwrap();
    h.state_mut().open_dialog("macros", json!({}));
    h.run_steps(3);
    h.get_by_label("Recorded Scripts").click();
    h.run_steps(2);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert!(h.state_mut().session.execute("automate.allScripts", json!({})).unwrap().as_array().unwrap().is_empty());
}
