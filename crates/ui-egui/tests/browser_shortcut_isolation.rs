//! Browser/system shortcuts must not fall through to unrelated spreadsheet commands.
use egui::{Event, Key, Modifiers, os::OperatingSystem};
use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
};
use gridcraft_ui_egui::{SheetApp, i18n::Language};
use serde_json::json;
use std::sync::Arc;

type Harness = egui_kittest::Harness<'static, SheetApp>;
const CMD: Modifiers = Modifiers { mac_cmd: true, command: true, ..Modifiers::NONE };
const CTRL: Modifiers = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };

fn harness(os: OperatingSystem) -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("cell.set", json!({"cell":"B2","input":"42"})).unwrap();
    session.execute("selection.set", json!({"cell":"H8"})).unwrap();
    let mut app = SheetApp::new(session, Default::default());
    app.ui.language = Language::Ja;
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.ctx.set_os(os);
    h.run_steps(3);
    h
}
fn press(h: &mut Harness, key: Key, modifiers: Modifiers, text: Option<&str>, text_first: bool) {
    let down = Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers };
    if text_first {
        if let Some(text) = text {
            h.input_mut().events.push(Event::Text(text.into()));
        }
        h.input_mut().events.push(down);
    } else {
        h.input_mut().events.push(down);
        if let Some(text) = text {
            h.input_mut().events.push(Event::Text(text.into()));
        }
    }
    h.input_mut().events.push(Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers });
    h.run_steps(3);
}
fn assert_unchanged(h: &Harness, before: &Arc<gridcraft_engine::model::Workbook>, history: usize, docs: usize) {
    let app = h.state();
    let doc = app.session.active().unwrap();
    assert!(Arc::ptr_eq(before, &doc.wb), "reserved shortcut mutated workbook");
    assert_eq!(doc.undo.len(), history);
    assert_eq!(app.session.documents().len(), docs);
    assert_eq!(doc.selection.current().a1(), "H8");
    assert!(app.editor.is_none(), "shortcut character leaked into cell editor");
    assert!(app.dialog.is_none() && app.message.is_none() && app.backstage.is_none());
    assert!(doc.wb.active().unwrap().tables.is_empty());
    assert_eq!(doc.wb.active().unwrap().value(CellRef::parse("B2").unwrap()), Value::number(42.0));
}

#[test]
fn safari_option_command_t_never_creates_a_table_or_types_a_character() {
    // Distinct browser delivery cases: key only, text after key, and text before key.
    for (text, text_first) in [(None, false), (Some("†"), false), (Some("†"), true)] {
        let mut h = harness(OperatingSystem::Mac);
        let before = h.state().session.active().unwrap().wb.clone();
        let history = h.state().session.active().unwrap().undo.len();
        let docs = h.state().session.documents().len();
        press(&mut h, Key::T, Modifiers { alt: true, ..CMD }, text, text_first);
        assert_unchanged(&h, &before, history, docs);
    }
}

#[test]
fn extra_modifiers_do_not_trigger_plain_command_letter_actions() {
    // Every case has a different destructive/formatting/dialog action if modifiers are ignored.
    for (os, modifiers) in [(OperatingSystem::Mac, Modifiers { alt: true, ..CMD }), (OperatingSystem::Windows, Modifiers { alt: true, ..CTRL })] {
        let mut h = harness(os);
        let before = h.state().session.active().unwrap().wb.clone();
        let history = h.state().session.active().unwrap().undo.len();
        let docs = h.state().session.documents().len();
        for key in [
            Key::T,
            Key::B,
            Key::I,
            Key::U,
            Key::S,
            Key::O,
            Key::N,
            Key::W,
            Key::P,
            Key::K,
            Key::F,
            Key::H,
            Key::G,
            Key::A,
            Key::D,
            Key::E,
            Key::Z,
            Key::Y,
        ] {
            press(&mut h, key, modifiers, None, false);
            assert_unchanged(&h, &before, history, docs);
        }
    }
}

#[test]
fn browser_shift_chords_do_not_become_plain_formatting_or_fill_commands() {
    let mut h = harness(OperatingSystem::Windows);
    let before = h.state().session.active().unwrap().wb.clone();
    let history = h.state().session.active().unwrap().undo.len();
    let docs = h.state().session.documents().len();
    for key in [Key::B, Key::I, Key::U, Key::D, Key::R, Key::O, Key::N, Key::W, Key::P, Key::K, Key::H, Key::A, Key::E, Key::Y] {
        press(&mut h, key, Modifiers { shift: true, ..CTRL }, None, false);
        assert_unchanged(&h, &before, history, docs);
    }
}

#[test]
fn intended_table_bold_and_altgr_input_remain_available() {
    for (os, modifiers) in [(OperatingSystem::Mac, CMD), (OperatingSystem::Windows, CTRL)] {
        let mut h = harness(os);
        press(&mut h, Key::T, modifiers, None, false);
        assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().tables.len(), 1);
        assert!(h.state().editor.is_none());
        press(&mut h, Key::Z, modifiers, None, false);
        assert!(h.state().session.active().unwrap().wb.active().unwrap().tables.is_empty());
        press(&mut h, Key::B, modifiers, None, false);
        let doc = h.state().session.active().unwrap();
        assert!(doc.wb.styles.get(doc.wb.active().unwrap().style_id(CellRef::parse("H8").unwrap())).font.bold);
    }
    // Windows AltGr is real text input, unlike macOS Command+Option shortcuts.
    let mut h = harness(OperatingSystem::Windows);
    press(&mut h, Key::A, Modifiers { alt: true, ..CTRL }, Some("ä"), false);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "ä");
}
