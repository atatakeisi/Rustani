//! The canvas editor menu changes its own Unicode text and keeps TextEdit undo history.
use std::{cell::RefCell, rc::Rc, sync::Arc};

use egui::{Event, Key, Modifiers, PointerButton, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use gridcraft_engine::Session;
use gridcraft_ui_egui::{ClipboardData, ClipboardInbox, Services, SheetApp};
use serde_json::json;

const ORIGINAL: &str = "Sub Japanese()\n    Range(\"A1\").Value = \"日本語\"\nEnd Sub\n";
const PASTED: &str = "Sub Pasted()\n    Debug.Print \"貼り付け\"\nEnd Sub\n";
type Harness = egui_kittest::Harness<'static, SheetApp>;
type Requests = Rc<RefCell<Vec<ClipboardInbox>>>;
type WorkbookSnapshot = (Arc<gridcraft_engine::model::Workbook>, gridcraft_engine::Selection, usize);

fn async_harness() -> (Harness, Requests) {
    let requests = Requests::default();
    let received = requests.clone();
    let mut session = Session::new();
    session.new_workbook();
    session.execute("cell.set", json!({"cell":"A1","input":"grid must remain 日本語🙂"})).unwrap();
    session.execute("selection.set", json!({"range":"D4,E5"})).unwrap();
    session.execute("edit.copy", json!({"range":"A1"})).unwrap();
    let services = Services { paste_async: Some(Box::new(move |inbox| received.borrow_mut().push(inbox))), ..Default::default() };
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 1000.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, services),
    );
    h.run_steps(4);
    h.state_mut().open_dialog("vbaEditor", json!({"source":ORIGINAL,"procedure":"Japanese"}));
    h.run_steps(3);
    assert_eq!(text(&h), ORIGINAL);
    (h, requests)
}

fn workbook_snapshot(h: &Harness) -> WorkbookSnapshot {
    let d = h.state().session.doc().unwrap();
    (d.wb.clone(), d.selection.clone(), d.undo.len())
}

fn assert_workbook_unchanged(h: &Harness, before: &WorkbookSnapshot) {
    let d = h.state().session.doc().unwrap();
    assert_eq!(d.wb, before.0, "VBA paste must not change workbook cells, pictures or module metadata");
    assert_eq!(d.selection, before.1, "VBA paste must not move the worksheet selection");
    assert_eq!(d.undo.len(), before.2, "source text uses TextEdit undo, not worksheet Undo");
}

fn deliver(requests: &Requests, text: &str) {
    let inbox = requests.borrow_mut().pop().expect("the context-menu click must invoke the async clipboard service");
    *inbox.lock().unwrap() = Some(Ok(ClipboardData::Text { text: text.to_string(), html: Some("<p>HTML must not enter the VBA source</p>".into()) }));
}

fn dispatch_frames(h: &mut Harness, count: usize) {
    for _ in 0..count {
        step(h);
    }
}

fn begin_async_paste(h: &mut Harness, requests: &Requests) {
    action(h, "Select All");
    step(h);
    action(h, "Paste");
    assert_eq!(requests.borrow().len(), 1, "Paste is requested by the actual right-click menu");
    assert!(h.state().dialog.is_some());
    assert!(h.ctx.memory(|m| m.has_focus(egui::Id::new("gridcraft.vba_source"))), "the source is the pending request's editor");
}

fn close_editor_without_saving(h: &mut Harness) {
    h.key_press(Key::Escape);
    h.run_steps(3);
    h.get_by_label("Close without saving").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
}
fn text(h: &Harness) -> String {
    h.get_by_role(Role::MultilineTextInput).accesskit_node().document_range().text()
}
fn step(h: &mut Harness) {
    let mut raw = egui::RawInput::default();
    h.state_mut().raw_input_hook(&mut raw);
    h.input_mut().events.extend(raw.events);
    h.step();
}
fn menu(h: &mut Harness) {
    let pos = h.get_by_role(Role::MultilineTextInput).rect().center();
    h.input_mut().events.push(Event::PointerMoved(pos));
    step(h);
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        step(h);
    }
}
fn action(h: &mut Harness, label: &str) {
    menu(h);
    // The popup is drawn after the modal dialog; the earlier matching Paste belongs to the
    // disabled ribbon. Text assertions below verify that the editor receives every action.
    h.get_all_by_label(label).last().unwrap().click();
    h.run_steps(2);
}
fn undo(h: &mut Harness) {
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(2);
}
#[test]
fn vba_context_menu_select_all_copy_cut_paste_and_undo_operate_on_source_only() {
    let mut session = Session::new();
    session.new_workbook();
    let mut app = SheetApp::new(session, Default::default());
    app.services.paste_text = Some(Box::new(|| Some(PASTED.to_string())));
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 1000.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h.state_mut().open_dialog("vbaEditor", json!({"source":ORIGINAL,"procedure":"Japanese"}));
    h.run_steps(3);
    assert_eq!(text(&h), ORIGINAL);
    action(&mut h, "Select All");
    step(&mut h);
    action(&mut h, "Copy");
    step(&mut h);
    assert!(h.output().platform_output.commands.iter().any(|command| matches!(command,egui::OutputCommand::CopyText(text) if text==ORIGINAL)));
    assert_eq!(text(&h), ORIGINAL);
    action(&mut h, "Cut");
    step(&mut h);
    assert_eq!(text(&h), "");
    undo(&mut h);
    assert_eq!(text(&h), ORIGINAL);
    action(&mut h, "Select All");
    step(&mut h);
    action(&mut h, "Paste");
    step(&mut h);
    assert_eq!(text(&h), PASTED, "popup open: {}; focus: {:?}", egui::Popup::is_any_open(&h.ctx), h.ctx.memory(|memory| memory.focused()));
    undo(&mut h);
    assert_eq!(text(&h), ORIGINAL);
    assert!(h.state().session.doc().unwrap().wb.active().unwrap().cells.is_empty(), "editor events never paste into the grid");
}

#[test]
fn vba_right_click_async_paste_arrives_in_the_modal_source_and_text_undo_restores_unicode() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    dispatch_frames(&mut h, 4);
    assert_eq!(text(&h), ORIGINAL, "the asynchronous request must leave the selected source intact while waiting");
    assert_workbook_unchanged(&h, &before);

    const UNICODE: &str = "Sub Pasted()\n    Debug.Print \"貼り付け🙂e\u{301}\"\nEnd Sub\n";
    deliver(&requests, UNICODE);
    dispatch_frames(&mut h, 4);
    assert!(h.state().dialog.is_some(), "the reply must reach the editor while its modal dialog remains open");
    assert_eq!(text(&h), UNICODE, "the selected Unicode source is replaced once, with plain clipboard text");
    assert_workbook_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(text(&h), ORIGINAL, "one TextEdit Undo restores the complete previous Unicode source");
    assert_workbook_unchanged(&h, &before);
}

#[test]
fn vba_async_paste_is_discarded_after_the_source_dialog_closes_and_reopens() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    close_editor_without_saving(&mut h);
    const REOPENED: &str = "Sub Reopened()\n    Debug.Print \"新しい下書き🙂\"\nEnd Sub\n";
    h.state_mut().open_dialog("vbaEditor", json!({"source":REOPENED,"procedure":"Reopened"}));
    h.run_steps(3);
    h.get_by_role(Role::MultilineTextInput).click();
    h.run_steps(2);
    assert!(h.ctx.memory(|m| m.has_focus(egui::Id::new("gridcraft.vba_source"))));
    deliver(&requests, "stale reply from the closed editor");
    dispatch_frames(&mut h, 4);
    assert_eq!(text(&h), REOPENED, "reusing the source widget id must not reuse a closed editor's pending paste");
    assert_workbook_unchanged(&h, &before);
}

#[test]
fn vba_async_paste_is_discarded_when_another_text_field_takes_focus() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    h.get_all_by_role(Role::TextInput).find(|node| node.value().as_deref() == Some("Japanese")).unwrap().click();
    h.run_steps(2);
    assert!(!h.ctx.memory(|m| m.has_focus(egui::Id::new("gridcraft.vba_source"))));
    deliver(&requests, "must not become the procedure name");
    dispatch_frames(&mut h, 4);
    assert_eq!(text(&h), ORIGINAL);
    assert!(h.get_all_by_role(Role::TextInput).any(|node| node.value().as_deref() == Some("Japanese")), "the procedure field is unchanged");
    assert_workbook_unchanged(&h, &before);
    // Returning to the source after the reply was rejected must not replay it later.
    h.get_by_role(Role::MultilineTextInput).click();
    dispatch_frames(&mut h, 3);
    assert_eq!(text(&h), ORIGINAL);
    assert_workbook_unchanged(&h, &before);
}

#[test]
fn vba_async_paste_restores_a_focusless_live_draft_without_redirecting_to_the_grid() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    let source = egui::Id::new("gridcraft.vba_source");
    h.ctx.memory_mut(|memory| memory.surrender_focus(source));
    assert_eq!(h.ctx.memory(|memory| memory.focused()), None);
    deliver(&requests, PASTED);
    dispatch_frames(&mut h, 4);
    assert_eq!(text(&h), PASTED, "a browser focus gap must not lose the live draft's requested paste");
    assert!(h.ctx.memory(|memory| memory.has_focus(source)));
    assert_workbook_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(text(&h), ORIGINAL);
    assert_workbook_unchanged(&h, &before);
}

#[test]
fn vba_async_reply_polled_before_paint_cannot_leak_into_a_recreated_dialog() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    deliver(&requests, "stale text polled before the old dialog was painted");
    let ctx = h.ctx.clone();
    h.state_mut().logic(&ctx);
    assert_eq!(text(&h), ORIGINAL, "polling alone has not drawn or edited the source");

    // Drop/recreate the dialog between the logic and paint phases. The existing close/reopen
    // test exercises real close controls; this case isolates the otherwise unpainted interval.
    h.state_mut().dialog = None;
    const RECREATED: &str = "Sub Fresh()\n    Debug.Print \"別の下書き🙂\"\nEnd Sub\n";
    h.state_mut().open_dialog("vbaEditor", json!({"source":RECREATED,"procedure":"Fresh"}));
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("gridcraft.vba_source")));
    dispatch_frames(&mut h, 4);
    assert!(h.state().dialog.is_some());
    assert_eq!(text(&h), RECREATED, "a reply already polled for the old draft must not become a global Paste event");
    assert_workbook_unchanged(&h, &before);
}

#[test]
fn vba_async_permission_error_preserves_source_and_workbook_and_leaves_the_app_usable() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    let inbox = requests.borrow_mut().pop().unwrap();
    *inbox.lock().unwrap() = Some(Err("Clipboard permission denied".into()));
    dispatch_frames(&mut h, 4);
    assert_workbook_unchanged(&h, &before);
    if h.state().message.is_some() {
        h.key_press(Key::Enter);
        h.run_steps(3);
        assert!(h.state().message.is_none(), "the optional permission message can be dismissed normally");
    }
    assert!(h.state().dialog.is_some());
    assert_eq!(text(&h), ORIGINAL);
    // The same source menu still operates after the rejected browser request.
    action(&mut h, "Select All");
    step(&mut h);
    action(&mut h, "Copy");
    step(&mut h);
    assert!(h.output().platform_output.commands.iter().any(|command| matches!(command,egui::OutputCommand::CopyText(text) if text==ORIGINAL)));
    assert_eq!(text(&h), ORIGINAL);
    assert_workbook_unchanged(&h, &before);
}

#[test]
fn reopening_a_different_vba_draft_does_not_inherit_the_previous_drafts_text_undo() {
    let (mut h, requests) = async_harness();
    let before = workbook_snapshot(&h);
    begin_async_paste(&mut h, &requests);
    deliver(&requests, PASTED);
    dispatch_frames(&mut h, 4);
    assert_eq!(text(&h), PASTED, "the first draft has an actual async-paste undo entry");
    close_editor_without_saving(&mut h);

    const SECOND: &str = "Sub SecondDraft()\n    Debug.Print \"別のソース🙂\"\nEnd Sub\n";
    h.state_mut().open_dialog("vbaEditor", json!({"source":SECOND,"procedure":"SecondDraft"}));
    h.run_steps(3);
    assert_eq!(text(&h), SECOND);
    h.get_by_role(Role::MultilineTextInput).click();
    h.run_steps(2);
    undo(&mut h);
    assert_eq!(text(&h), SECOND, "Undo in a fresh draft must not restore source from the discarded draft sharing its widget id");
    assert_workbook_unchanged(&h, &before);
}
