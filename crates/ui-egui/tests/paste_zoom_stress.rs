//! Regression scenarios for repeated copy/paste and large-font rendering.

use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
};
use gridcraft_ui_egui::{ClipboardData, ClipboardInbox, Services, SheetApp};
use serde_json::json;
use std::{cell::RefCell, rc::Rc};

#[test]
fn max_font_and_zoom_render_on_a_small_atlas() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(2.0).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.input_mut().max_texture_side = Some(1024);
    h.state_mut().run("cell.set", json!({"cell":"A1", "input":"9999999999"})).unwrap();
    h.state_mut().run("home.fontSize", json!({"range":"A1", "size":409})).unwrap();
    h.state_mut().run("view.zoom", json!({"percent":400})).unwrap();
    h.state_mut().run("view.interfaceScale", json!({"percent":200})).unwrap();
    h.run_steps(3);
    h.state_mut().run("home.autofitColumnWidth", json!({"cols":"A:A"})).unwrap();
    h.state_mut().begin_edit(Some("9999999999".into()), false);
    h.run_steps(3);
    let doc = h.state().session.active().unwrap();
    let sh = doc.wb.active().unwrap();
    assert_eq!(doc.wb.styles.get(sh.style_id(CellRef::parse("A1").unwrap())).font.size, 409.0, "rendering does not change saved formatting");
}

#[test]
fn large_imported_shape_text_renders_on_a_small_atlas() {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("insert.shape", json!({"kind":"rectangle", "at":"A1", "text":"WWW 123"})).unwrap();
    let wb = std::sync::Arc::make_mut(&mut session.active_mut().unwrap().wb);
    wb.sheet_mut(wb.active_sheet).unwrap().shapes[0].format.font_size = Some(4000.0);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(2.0).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.input_mut().max_texture_side = Some(1024);
    h.state_mut().run("view.zoom", json!({"percent":400})).unwrap();
    h.state_mut().run("view.interfaceScale", json!({"percent":200})).unwrap();
    h.run_steps(3);
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().shapes[0].format.font_size, Some(4000.0));
}

#[test]
fn delayed_pastes_keep_the_latest_target_during_rapid_selection_and_zoom() {
    let requests: Rc<RefCell<Vec<ClipboardInbox>>> = Rc::new(RefCell::new(Vec::new()));
    let capture = requests.clone();
    let mut session = Session::new();
    session.new_workbook();
    let services = Services { paste_async: Some(Box::new(move |inbox| capture.borrow_mut().push(inbox))), ..Default::default() };
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, services),
    );
    h.run_steps(2);
    for i in 0..100 {
        let row = i + 1;
        h.state_mut().run("selection.set", json!({"cell":format!("B{row}")})).unwrap();
        h.state_mut().paste_from_clipboard();
        h.state_mut().run("selection.set", json!({"range":format!("D{row},F{row}")})).unwrap();
        h.state_mut().paste_from_clipboard();
        h.state_mut().run("view.zoom", json!({"percent":if i % 2 == 0 { 10 } else { 400 }})).unwrap();
        h.state_mut().run("view.interfaceScale", json!({"percent":if i % 2 == 0 { 75 } else { 200 }})).unwrap();
        let items = requests.borrow();
        *items[2 * i + 1].lock().unwrap() = Some(Ok(ClipboardData::Text { text: "42".into(), html: None }));
        drop(items);
        h.run_steps(2);
        // An older browser response arriving after the newer one must not paste anywhere.
        *requests.borrow()[2 * i].lock().unwrap() = Some(Ok(ClipboardData::Text { text: "999".into(), html: None }));
        h.run_steps(2);
        let sheet = h.state().session.active().unwrap().wb.active().unwrap();
        assert_eq!(sheet.value(CellRef::parse(&format!("B{row}")).unwrap()), Value::Empty);
        for col in ["D", "F"] {
            assert_eq!(sheet.value(CellRef::parse(&format!("{col}{row}")).unwrap()), Value::number(42.0));
        }
    }
}

#[test]
fn repeated_clipboard_events_paste_copied_numbers_across_zoom_changes() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.state_mut().run("cell.set", json!({"cell":"A1", "input":"73"})).unwrap();
    h.input_mut().events.push(egui::Event::Copy);
    h.run_steps(2);
    for row in 1..=100 {
        h.state_mut().run("selection.set", json!({"range":format!("D{row},F{row}")})).unwrap();
        h.state_mut().run("view.zoom", json!({"percent":if row % 2 == 0 { 10 } else { 400 }})).unwrap();
        h.input_mut().events.push(egui::Event::Paste("73\n".into()));
        h.run_steps(2);
        let sheet = h.state().session.active().unwrap().wb.active().unwrap();
        for col in ["D", "F"] {
            assert_eq!(sheet.value(CellRef::parse(&format!("{col}{row}")).unwrap()), Value::number(73.0));
        }
        assert!(h.state().message.is_none(), "unexpected error at row {row}");
    }
}

#[test]
fn copy_outline_survives_source_deletion_sheet_switch_and_zoom() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.state_mut().run("cell.set", json!({"cell":"A1", "input":"55"})).unwrap();
    h.state_mut().run("edit.copy", json!({"range":"A1"})).unwrap();
    h.run_steps(2);
    h.state_mut().run("view.zoom", json!({"percent":400})).unwrap();
    h.run_steps(2);
    h.state_mut().run("edit.clearContents", json!({"range":"A1"})).unwrap();
    h.run_steps(2);
    h.state_mut().run("home.insertSheet", json!({"name":"Other"})).unwrap();
    h.state_mut().run("view.zoom", json!({"percent":10})).unwrap();
    h.run_steps(2);
    assert!(h.state().message.is_none());
}

#[test]
fn home_survives_extreme_window_resizing() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.state_mut().run("ui.backstage", json!({"page":"home"})).unwrap();
    for size in [egui::vec2(1200.0, 800.0), egui::vec2(300.0, 180.0), egui::vec2(80.0, 45.0), egui::vec2(1920.0, 1080.0)] {
        h.set_size(size);
        h.run_steps(2);
    }
}
