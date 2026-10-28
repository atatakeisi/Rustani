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

#[test]
fn mac_control_command_is_not_plain_control_or_command() {
    let mut h = harness(OperatingSystem::Mac);
    let before = h.state().session.active().unwrap().wb.clone();
    let history = h.state().session.active().unwrap().undo.len();
    let docs = h.state().session.documents().len();
    // Control+Command has a separate meaning (e.g. system shortcuts). Paste Special's
    // explicitly supported Control+Command+V is checked separately as a positive case.
    for key in [Key::E, Key::L, Key::Num6, Key::Quote, Key::F3] {
        press(&mut h, key, Modifiers { ctrl: true, ..CMD }, None, false);
        assert_unchanged(&h, &before, history, docs);
    }
    press(&mut h, Key::V, Modifiers { ctrl: true, ..CMD }, None, false);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("pasteSpecial"));
    assert!(Arc::ptr_eq(&before, &h.state().session.active().unwrap().wb));
}

#[test]
fn held_new_workbook_key_fires_once_but_navigation_can_repeat() {
    let mut h = harness(OperatingSystem::Mac);
    let docs = h.state().session.documents().len();
    for repeat in [false, true] {
        // Two distinct events of one key hold, not repeated test runs.
        h.input_mut().events.push(Event::Key { key: Key::N, physical_key: Some(Key::N), pressed: true, repeat, modifiers: CMD });
        h.run_steps(2);
    }
    assert_eq!(h.state().session.documents().len(), docs + 1, "holding Command+N must not create additional workbooks");
    for repeat in [false, true] {
        h.input_mut().events.push(Event::Key {
            key: Key::ArrowDown,
            physical_key: Some(Key::ArrowDown),
            pressed: true,
            repeat,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }
    assert_eq!(h.state().session.active().unwrap().selection.active.a1(), "A3", "arrow auto-repeat remains available");
}

#[test]
fn focused_editors_dialogs_and_filter_menu_own_command_keys() {
    use egui_kittest::kittest::Queryable;
    for surface in ["cell", "formula", "find", "filter", "vba"] {
        let mut h = harness(OperatingSystem::Mac);
        match surface {
            "cell" | "formula" => h.state_mut().begin_edit(Some("editing".into()), surface == "formula"),
            "find" => h.state_mut().open_dialog("find", json!({})),
            "filter" => {
                h.state_mut().run("range.setValues", json!({"range":"H8:H10","values":[["Header"],["Value"],["Other"]]})).unwrap();
                h.state_mut().run("data.filter", json!({"range":"H8:H10"})).unwrap();
                press(&mut h, Key::ArrowDown, Modifiers::ALT, None, false);
            }
            _ => h.state_mut().open_dialog("vbaEditor", json!({"source":"Sub Main()\nEnd Sub"})),
        }
        h.run_steps(3);
        let before = h.state().session.active().unwrap().wb.clone();
        let selection = h.state().session.active().unwrap().selection.clone();
        let history = h.state().session.active().unwrap().undo.len();
        let docs = h.state().session.documents().len();
        let editor = h.state().editor.as_ref().map(|e| e.text.clone());
        for (key, modifiers) in [(Key::T, CMD), (Key::T, Modifiers { alt: true, ..CMD }), (Key::N, CMD)] {
            press(&mut h, key, modifiers, None, false);
            assert!(Arc::ptr_eq(&before, &h.state().session.active().unwrap().wb), "{surface}: workbook changed");
            assert_eq!(h.state().session.active().unwrap().selection, selection, "{surface}: selection changed");
            assert_eq!(h.state().session.active().unwrap().undo.len(), history);
            assert_eq!(h.state().session.documents().len(), docs);
            assert_eq!(h.state().editor.as_ref().map(|e| e.text.clone()), editor);
        }
        if surface == "filter" {
            h.input_mut().events.push(Event::Text("Value".into()));
            h.run_steps(2);
            assert!(h.query_by_label("Other").is_none());
            assert!(h.state().editor.is_none());
        }
    }
}

#[test]
fn unfocused_window_does_not_accept_delayed_grid_keys_or_paste() {
    let mut h = harness(OperatingSystem::Mac);
    let before = h.state().session.active().unwrap().wb.clone();
    let history = h.state().session.active().unwrap().undo.len();
    let docs = h.state().session.documents().len();
    h.input_mut().focused = false;
    h.input_mut().events.extend([
        Event::WindowFocused(false),
        Event::Key { key: Key::T, physical_key: Some(Key::T), pressed: true, repeat: false, modifiers: CMD },
        Event::Paste("late text".into()),
        Event::Key { key: Key::T, physical_key: Some(Key::T), pressed: false, repeat: false, modifiers: CMD },
    ]);
    h.run_steps(2);
    assert_unchanged(&h, &before, history, docs);
    h.input_mut().focused = true;
    h.input_mut().events.push(Event::WindowFocused(true));
    h.run_steps(2);
    press(&mut h, Key::T, CMD, None, false);
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().tables.len(), 1, "input works again after refocusing");
}

fn held_key(h: &mut Harness, key: Key, modifiers: Modifiers) {
    for repeat in [false, true] {
        h.input_mut().events.push(Event::Key { key, physical_key: Some(key), pressed: true, repeat, modifiers });
        h.run_steps(2);
    }
    h.input_mut().events.push(Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers });
    h.run_steps(1);
}

#[test]
fn held_actions_are_single_shot_and_intentional_repeats_still_work() {
    let mut h = harness(OperatingSystem::Mac);
    held_key(&mut h, Key::B, CMD);
    let doc = h.state().session.active().unwrap();
    assert!(
        doc.wb.styles.get(doc.wb.active().unwrap().style_id(CellRef::parse("H8").unwrap())).font.bold,
        "holding Bold must not toggle it off again"
    );
    held_key(&mut h, Key::T, CMD);
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().tables.len(), 1);
    assert!(h.state().message.is_none(), "holding Table must not try to create an overlapping table");
    let sheets = h.state().session.active().unwrap().wb.sheets.len();
    held_key(&mut h, Key::F11, Modifiers::SHIFT);
    assert_eq!(h.state().session.active().unwrap().wb.sheets.len(), sheets + 1);
    h.state_mut().run("file.new", json!({})).unwrap();
    let docs = h.state().session.documents().len();
    held_key(&mut h, Key::W, CMD);
    assert_eq!(h.state().session.documents().len(), docs - 1);
    assert!(h.state().dialog.is_none(), "a repeated Close must not reach the previous unsaved workbook");
    // Two different edits must still be undoable with a held Undo key.
    h.state_mut().run("cell.set", json!({"cell":"A1","input":"1"})).unwrap();
    h.state_mut().run("cell.set", json!({"cell":"A2","input":"2"})).unwrap();
    held_key(&mut h, Key::Z, CMD);
    let sh = h.state().session.active().unwrap().wb.active().unwrap();
    assert_eq!(sh.value(CellRef::new(0, 0)), Value::Empty);
    assert_eq!(sh.value(CellRef::new(1, 0)), Value::Empty);
    h.input_mut().events.push(Event::Text("a".into()));
    h.run_steps(2);
    h.input_mut().events.extend([
        Event::Key { key: Key::A, physical_key: Some(Key::A), pressed: true, repeat: true, modifiers: Modifiers::NONE },
        Event::Text("a".into()),
    ]);
    h.run_steps(2);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "aa", "ordinary text auto-repeat remains available");
}
