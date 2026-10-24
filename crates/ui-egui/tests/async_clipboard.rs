use std::{cell::RefCell, rc::Rc};

use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
};
use gridcraft_ui_egui::{ClipboardData, ClipboardInbox, Services, SheetApp};
use serde_json::json;

fn harness() -> (egui_kittest::Harness<'static, SheetApp>, Rc<RefCell<Vec<ClipboardInbox>>>) {
    let requests = Rc::new(RefCell::new(Vec::new()));
    let received = requests.clone();
    let mut session = Session::new();
    session.new_workbook();
    let services = Services { paste_async: Some(Box::new(move |inbox| received.borrow_mut().push(inbox))), ..Default::default() };
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200., 800.)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, services),
    );
    h.run_steps(4);
    (h, requests)
}

fn deliver(requests: &Rc<RefCell<Vec<ClipboardInbox>>>, data: Result<ClipboardData, String>) {
    *requests.borrow_mut().pop().unwrap().lock().unwrap() = Some(data);
}

fn value(h: &egui_kittest::Harness<'static, SheetApp>, cell: &str) -> Value {
    h.state().session.active().unwrap().wb.active().unwrap().value(CellRef::parse(cell).unwrap())
}

#[test]
fn ribbon_async_paste_waits_and_applies_rich_data_to_every_area() {
    let (mut h, requests) = harness();
    h.state_mut().run("selection.set", json!({"range":"A1,C3"})).unwrap();
    h.state_mut().paste_from_clipboard();
    h.run_steps(2);
    assert_eq!(value(&h, "A1"), Value::Empty);
    deliver(
        &requests,
        Ok(ClipboardData::Text { text: "¥1,200".into(), html: Some("<table><tr><td x:num=\"1200\">¥1,200</td></tr></table>".into()) }),
    );
    h.run_steps(2);
    assert_eq!(value(&h, "A1"), Value::Number(1200.));
    assert_eq!(value(&h, "C3"), Value::Number(1200.));
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(value(&h, "A1"), Value::Empty);
    assert_eq!(value(&h, "C3"), Value::Empty);
}

#[test]
fn clipboard_response_cannot_paste_into_a_changed_destination() {
    let (mut h, requests) = harness();
    h.state_mut().paste_from_clipboard();
    h.state_mut().run("selection.set", json!({"cell":"B2"})).unwrap();
    deliver(&requests, Ok(ClipboardData::Text { text: "stale".into(), html: None }));
    h.run_steps(2);
    assert_eq!(value(&h, "A1"), Value::Empty);
    assert_eq!(value(&h, "B2"), Value::Empty);
}

#[test]
fn pending_grid_paste_does_not_replace_a_cell_after_text_editing_starts() {
    let (mut h, requests) = harness();
    h.state_mut().paste_from_clipboard();
    h.state_mut().begin_edit(Some("編集中".into()), false);
    deliver(&requests, Ok(ClipboardData::Text { text: "stale".into(), html: None }));
    h.run_steps(2);
    assert_eq!(value(&h, "A1"), Value::Empty);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "編集中");
}

#[test]
fn denied_browser_read_falls_back_to_internal_copy() {
    let (mut h, requests) = harness();
    h.state_mut().run("cell.set", json!({"cell":"A1","value":7})).unwrap();
    h.state_mut().run("edit.copy", json!({})).unwrap();
    h.state_mut().run("selection.set", json!({"cell":"C3"})).unwrap();
    h.state_mut().paste_from_clipboard();
    deliver(&requests, Err("permission denied".into()));
    h.run_steps(2);
    assert_eq!(value(&h, "C3"), Value::Number(7.));
    assert!(h.state().session.clipboard.is_some());
}

#[test]
fn clipboard_image_is_inserted_selected_and_undoable() {
    let (mut h, requests) = harness();
    let image = image::RgbaImage::from_pixel(4, 3, image::Rgba([255, 70, 40, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    h.state_mut().paste_from_clipboard();
    deliver(&requests, Ok(ClipboardData::Image(bytes.into_inner())));
    h.run_steps(2);
    let sh = h.state().session.active().unwrap().wb.active().unwrap();
    assert_eq!(sh.images.len(), 1);
    assert_eq!(h.state().selected_chart, Some(sh.images[0].id));
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert!(h.state().session.active().unwrap().wb.active().unwrap().images.is_empty());
}

#[test]
fn menu_macros_routes_to_dialog_without_changing_headless_query() {
    let (mut h, _) = harness();
    assert!(h.state_mut().run("view.macros", json!({})).unwrap().is_array());
    assert!(h.state().dialog.is_none());
    h.state_mut().run_or_alert("view.macros", json!({}));
    assert!(h.state().dialog.is_some());
}
