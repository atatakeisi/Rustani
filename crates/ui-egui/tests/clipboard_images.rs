//! Browser image reads retain the paste gesture's worksheet and yield to text editors.

use egui::Event;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{ImagePasteInbox, ImagePasteTargetState, Services, SheetApp};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> (Harness, ImagePasteInbox, ImagePasteTargetState) {
    let inbox = ImagePasteInbox::default();
    let target = ImagePasteTargetState::default();
    let published = target.clone();
    let mut session = Session::new();
    session.new_workbook();
    let services = Services { clipboard_images: Some(inbox.clone()), ..Default::default() };
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        move |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
            // The web shell publishes only after UI and host mutations, before the next gesture.
            *published.lock().unwrap() = app.image_paste_target(ui.ctx());
        },
        SheetApp::new(session, services),
    );
    h.run_steps(4);
    (h, inbox, target)
}

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(4, 3, image::Rgba([24, 150, 110, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn image_event_inserts_at_the_captured_selection_and_is_selected_and_undoable() {
    let (mut h, inbox, target) = harness();
    h.state_mut().run("selection.set", json!({"cell": "D6"})).unwrap();
    h.run_steps(2);
    let gesture = target.lock().unwrap().clone().unwrap();
    assert_eq!(gesture.selection.active.a1(), "D6");
    let bytes = png();
    inbox.lock().unwrap().push((gesture, bytes.clone()));
    h.run_steps(2);
    let sh = h.state().session.doc().unwrap().wb.active().unwrap();
    assert_eq!(sh.images.len(), 1);
    assert_eq!(*sh.images[0].data, bytes);
    assert_eq!(sh.images[0].anchor.cell.a1(), "D6");
    assert_eq!(h.state().selected_chart, Some(sh.images[0].id));
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert!(h.state().session.doc().unwrap().wb.active().unwrap().images.is_empty());
}

#[test]
fn image_response_does_not_follow_a_changed_cell_sheet_workbook_or_input_owner() {
    for change in 0..7 {
        let (mut h, inbox, target) = harness();
        let gesture = target.lock().unwrap().clone().unwrap();
        match change {
            0 => {
                h.state_mut().run("selection.set", json!({"cell": "B2"})).unwrap();
            }
            1 => {
                h.state_mut().run("home.insertSheet", json!({"name": "Other"})).unwrap();
            }
            2 => {
                h.state_mut().run("file.new", json!({})).unwrap();
            }
            3 => h.state_mut().begin_edit(Some("draft".into()), false),
            4 => h.state_mut().open_dialog("find", json!({})),
            5 => h.state_mut().message = Some(("Rustani".into(), "A test message".into())),
            _ => h.state_mut().name_box = Some("A1".into()),
        }
        h.run_steps(2);
        let undo = h.state().session.doc().unwrap().undo.len();
        inbox.lock().unwrap().push((gesture, png()));
        h.run_steps(2);
        assert!(h.state().session.documents().iter().all(|doc| doc.wb.sheets.iter().all(|sheet| sheet.images.is_empty())), "change {change}");
        assert_eq!(h.state().session.doc().unwrap().undo.len(), undo, "rejected response adds no undo, change {change}");
        assert!(inbox.lock().unwrap().is_empty());
    }
}

#[test]
fn editing_leaves_the_browser_text_flavour_available() {
    let (mut h, inbox, target) = harness();
    h.state_mut().begin_edit(Some("draft".into()), false);
    h.run_steps(2);
    assert!(target.lock().unwrap().is_none(), "the DOM image listener must return without canceling the ordinary paste event");
    h.input_mut().events.push(Event::Paste(" / pasted text".into()));
    h.run_steps(2);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "draft / pasted text");
    assert!(inbox.lock().unwrap().is_empty());
    assert!(h.state().session.doc().unwrap().wb.active().unwrap().images.is_empty());
}

#[test]
fn target_published_after_ui_tracks_current_selection_and_modal_ownership() {
    let (mut h, _, target) = harness();
    let original = target.lock().unwrap().clone().unwrap();
    h.state_mut().run("selection.set", json!({"range": "C3,E5"})).unwrap();
    h.run_steps(2);
    let next = target.lock().unwrap().clone().unwrap();
    assert_eq!(next.uid, original.uid);
    assert_eq!(next.sheet, original.sheet);
    assert_eq!(next.selection.a1(), "C3,E5");
    h.state_mut().open_dialog("find", json!({}));
    h.run_steps(2);
    assert!(target.lock().unwrap().is_none());
}
