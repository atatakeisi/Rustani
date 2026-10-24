//! The canvas editor menu changes its own Unicode text and keeps TextEdit undo history.
use egui::{Event, Key, Modifiers, PointerButton, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

const ORIGINAL: &str = "Sub Japanese()\n    Range(\"A1\").Value = \"日本語\"\nEnd Sub\n";
const PASTED: &str = "Sub Pasted()\n    Debug.Print \"貼り付け\"\nEnd Sub\n";
type Harness = egui_kittest::Harness<'static, SheetApp>;
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
