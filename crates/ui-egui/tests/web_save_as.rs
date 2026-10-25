//! Web Save As asks for a filename before handing bytes to the host.

use std::sync::{Arc, Mutex};

use egui::{Event, ImeEvent, Key, Modifiers, accesskit::Role, vec2};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::{Session, core::CellRef, core::Value};
use gridcraft_ui_egui::{Services, SheetApp, control};
use serde_json::{Value as Json, json};

type Harness = egui_kittest::Harness<'static, SheetApp>;
type Downloads = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

fn fixture() -> (Harness, Downloads) {
    let downloads = Downloads::default();
    let sink = downloads.clone();
    let mut session = Session::new();
    session.new_workbook();
    session.run("selection.set", json!({"cell": "C3"})).unwrap();
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        SheetApp::new(
            session,
            Services { download: Some(Box::new(move |name, bytes| sink.lock().unwrap().push((name.into(), bytes.to_vec())))), ..Default::default() },
        ),
    );
    h.run_steps(6);
    (h, downloads)
}

fn open(h: &mut Harness) {
    h.state_mut().run("file.saveAs", json!({})).unwrap();
    h.run_steps(6);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("saveAs"));
}

fn type_name(h: &mut Harness, name: &str) {
    h.input_mut().events.push(Event::Text(name.into()));
    h.run_steps(2);
}

fn set_name(h: &mut Harness, name: &str) {
    h.state_mut().dialog.as_mut().unwrap().values.insert("name".into(), json!(name));
    h.run_steps(2);
}

fn confirm(h: &mut Harness) -> Json {
    let ctx = h.ctx.clone();
    match control::handle(h.state_mut(), &ctx, "ui.dialog.confirm", &json!({})) {
        control::Outcome::Done(result) => result,
        _ => panic!("unexpected screenshot"),
    }
}

#[test]
fn initial_focus_replaces_the_name_and_enter_downloads_the_cell_draft_once() {
    let (mut h, downloads) = fixture();
    h.input_mut().events.push(Event::Text("123".into()));
    h.run_steps(2);
    assert!(h.state().editor.is_some());
    open(&mut h);
    assert!(h.state().editor.is_none(), "opening Save As commits the cell draft");
    assert!(downloads.lock().unwrap().is_empty(), "opening the dialog does not download");
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(egui::Id::new("gridcraft.save_as.name")));
    type_name(&mut h, "2026年度売上");
    assert_eq!(h.state().dialog.as_ref().unwrap().values["name"], "2026年度売上");
    let saved = h.state().session.active().unwrap().saved.clone();
    h.key_press(Key::Enter);
    h.run_steps(6);
    assert!(h.state().dialog.is_none());
    let files = downloads.lock().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "2026年度売上.xlsx");
    let (wb, _) = gridcraft_engine::io::open_bytes(&files[0].0, &files[0].1).unwrap();
    assert_eq!(wb.active().unwrap().value(CellRef::new(2, 2)), Value::Number(123.0));
    let doc = h.state().session.active().unwrap();
    assert_eq!(doc.display_title(), "2026年度売上.xlsx");
    assert!(doc.path.is_none(), "browser downloads do not create a local path");
    assert!(doc.is_dirty(), "dispatch is not confirmation that a browser wrote the file");
    assert!(Arc::ptr_eq(&saved, &doc.saved));
    assert_eq!(doc.selection.active, CellRef::new(2, 2), "Enter must not move the grid selection");
}

#[test]
fn save_button_preserves_an_existing_extension_and_reopens_with_the_new_title() {
    let (mut h, downloads) = fixture();
    open(&mut h);
    type_name(&mut h, "  PWGridCraft.XLSX  ");
    h.get_by_role_and_label(Role::Button, "Save").click();
    h.run_steps(4);
    assert_eq!(downloads.lock().unwrap()[0].0, "PWGridCraft.XLSX");
    assert_eq!(h.state().session.active().unwrap().title, "PWGridCraft.XLSX");
    open(&mut h);
    assert_eq!(h.state().dialog.as_ref().unwrap().values["name"], "PWGridCraft.xlsx");
    assert_eq!(downloads.lock().unwrap().len(), 1);
}

#[test]
fn cancel_and_escape_leave_the_document_name_and_downloads_alone() {
    let (mut h, downloads) = fixture();
    let title = h.state().session.active().unwrap().title.clone();
    for escape in [false, true] {
        open(&mut h);
        type_name(&mut h, "discard this name");
        if escape {
            h.key_press(Key::Escape);
        } else {
            h.get_by_role_and_label(Role::Button, "Cancel").click();
        }
        h.run_steps(4);
        assert!(h.state().dialog.is_none());
        assert_eq!(h.state().session.active().unwrap().title, title);
        assert!(downloads.lock().unwrap().is_empty());
    }
}

#[test]
fn invalid_names_keep_the_dialog_open_and_allow_retry() {
    let (mut h, downloads) = fixture();
    open(&mut h);
    let title = h.state().session.active().unwrap().title.clone();
    for name in ["", "  ", "../outside", "folder\\outside", "bad\0name"] {
        set_name(&mut h, name);
        let result = confirm(&mut h);
        assert!(result.get("error").is_some(), "{name:?}: {result}");
        assert!(h.state().dialog.is_some(), "{name:?}");
        assert_eq!(h.state().session.active().unwrap().title, title);
        assert!(downloads.lock().unwrap().is_empty());
    }
    set_name(&mut h, "Valid");
    assert_eq!(confirm(&mut h)["ok"], true);
    assert!(h.state().dialog.is_none());
    assert_eq!(downloads.lock().unwrap().len(), 1);
}

#[test]
fn stale_dialog_does_not_download_or_rename_another_workbook() {
    let (mut h, downloads) = fixture();
    open(&mut h);
    let original = h.state().session.active().unwrap().uid;
    h.state_mut().session.new_workbook();
    assert_ne!(h.state().session.active().unwrap().uid, original);
    let title = h.state().session.active().unwrap().title.clone();
    set_name(&mut h, "Other workbook");
    assert!(confirm(&mut h).get("error").is_some());
    assert!(downloads.lock().unwrap().is_empty());
    assert_eq!(h.state().session.active().unwrap().title, title);
}

#[test]
fn native_save_as_still_invokes_the_platform_picker() {
    let (mut h, downloads) = fixture();
    let picked = Arc::new(Mutex::new(Vec::new()));
    let capture = picked.clone();
    h.state_mut().services.pick_save = Some(Box::new(move |suggested| {
        capture.lock().unwrap().push(suggested.to_string());
        None
    }));
    h.state_mut().run("file.saveAs", json!({})).unwrap();
    h.run_steps(3);
    assert_eq!(picked.lock().unwrap().len(), 1);
    assert!(h.state().dialog.is_none());
    assert!(downloads.lock().unwrap().is_empty());
}

#[test]
fn encoding_a_workbook_programmatically_does_not_open_a_dialog_or_download() {
    let (mut h, downloads) = fixture();
    let result = h.state_mut().run("file.saveBytes", json!({"format": "xlsx", "name": "Programmatic.xlsx"})).unwrap();
    assert_eq!(result["name"], "Programmatic.xlsx");
    assert!(h.state().dialog.is_none());
    assert!(downloads.lock().unwrap().is_empty());
}

fn enter_event() -> Event {
    Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }
}

#[test]
fn ime_commit_enter_in_either_event_order_does_not_save_before_the_next_enter() {
    for key_first in [false, true] {
        let (mut h, downloads) = fixture();
        open(&mut h);
        h.input_mut().events.push(Event::Ime(ImeEvent::Preedit { text: "売上".into(), active_range_chars: None }));
        h.run_steps(2);
        let commit = Event::Ime(ImeEvent::Commit("売上".into()));
        let events = if key_first { vec![enter_event(), commit] } else { vec![commit, enter_event()] };
        h.input_mut().events.extend(events);
        h.run_steps(2);
        assert!(downloads.lock().unwrap().is_empty(), "IME Enter is not Save");
        assert_eq!(h.state().dialog.as_ref().unwrap().values["name"], "売上");
        h.key_press(Key::Enter);
        h.run_steps(4);
        assert_eq!(downloads.lock().unwrap().len(), 1);
        assert_eq!(downloads.lock().unwrap()[0].0, "売上.xlsx");
    }
}

#[test]
fn ime_cancellation_escape_does_not_close_the_filename_dialog() {
    let (mut h, downloads) = fixture();
    open(&mut h);
    h.input_mut().events.push(Event::Ime(ImeEvent::Preedit { text: "仮".into(), active_range_chars: None }));
    h.run_steps(2);
    h.input_mut().events.extend([
        Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
        Event::Ime(ImeEvent::Preedit { text: "".into(), active_range_chars: None }),
    ]);
    h.run_steps(2);
    assert!(h.state().dialog.is_some());
    assert!(downloads.lock().unwrap().is_empty());
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
}

#[test]
fn japanese_dialog_labels_and_empty_name_error_are_translated() {
    let (mut h, downloads) = fixture();
    h.state_mut().ui.language = gridcraft_ui_egui::i18n::Language::Ja;
    open(&mut h);
    assert!(h.query_by_label("ファイル名:").is_some());
    assert!(h.query_by_role_and_label(Role::Button, "保存").is_some());
    assert!(h.query_by_role_and_label(Role::Button, "キャンセル").is_some());
    type_name(&mut h, " ");
    h.get_by_role_and_label(Role::Button, "保存").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_some());
    assert!(downloads.lock().unwrap().is_empty());
    assert!(h.query_by_label("ファイル名を入力してください。").is_some());
}
