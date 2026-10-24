use egui::{OutputCommand, accesskit::Role};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{Services, SheetApp};

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = Harness::builder().with_size(egui::vec2(1200., 800.)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Services { paste_text: Some(Box::new(|| Some("貼り付け🙂".into()))), ..Default::default() }),
    );
    h.run_steps(4);
    h.state_mut().begin_edit(Some("日本🙂".into()), true);
    h.run_steps(3);
    h
}

fn menu(h: &mut Harness) {
    h.get_all_by_role(Role::TextInput)
        .find(|node| node.value().as_deref() == h.state().editor.as_ref().map(|e| e.text.as_str()))
        .unwrap()
        .click_secondary();
    h.run_steps(2);
}

fn dispatch(h: &mut Harness) {
    // Production shells deliver the queue in raw_input_hook before the next frame.
    let mut raw = egui::RawInput::default();
    h.state_mut().raw_input_hook(&mut raw);
    h.input_mut().events.extend(raw.events);
    h.step();
}

#[test]
fn formula_menu_edits_unicode_selection_and_uses_the_text_undo_history() {
    let mut h = harness();
    menu(&mut h);
    h.get_by_label("Select All").click();
    h.run_steps(2);
    menu(&mut h);
    h.get_by_label("Copy").click();
    h.run_steps(2);
    dispatch(&mut h);
    assert!(
        h.output().platform_output.commands.iter().any(|command| matches!(command,OutputCommand::CopyText(text) if text=="日本🙂")),
        "{:?}; {:?}",
        h.output().platform_output.commands,
        h.state().editor
    );
    menu(&mut h);
    h.get_by_label("Cut").click();
    h.run_steps(2);
    dispatch(&mut h);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "");
    menu(&mut h);
    h.get_all_by_label("Paste").last().unwrap().click();
    h.run_steps(2);
    dispatch(&mut h);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "貼り付け🙂");
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().cells.len(), 0, "text menu must not paste into the sheet");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_ne!(h.state().editor.as_ref().unwrap().text, "貼り付け🙂");
}
