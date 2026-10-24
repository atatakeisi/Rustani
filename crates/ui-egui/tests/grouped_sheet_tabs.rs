//! Real tab clicks preserve the platform modifier conventions and expose grouping menus.
use egui::{Event, Modifiers, PointerButton, os::OperatingSystem};
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;
fn harness(os: OperatingSystem) -> Harness {
    let mut s = Session::new();
    s.new_workbook();
    for name in ["Two", "Three", "Four"] {
        s.execute("home.insertSheet", json!({"name":name})).unwrap();
    }
    s.execute("sheet.activate", json!({"sheet":0})).unwrap();
    let mut h = Harness::builder().with_os(os).with_size(egui::vec2(1500.0, 900.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(s, Default::default()),
    );
    h.run_steps(4);
    h
}
fn click(h: &mut Harness, name: &str, button: PointerButton, modifiers: Modifiers) {
    let pos = h.get_by_label(name).rect().center();
    h.input_mut().events.extend([Event::ModifiersChanged(modifiers), Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton { pos, button, pressed, modifiers });
        h.run_steps(1);
    }
    h.run_steps(2);
}
#[test]
fn additive_and_shift_click_group_tabs_then_plain_click_ungroups() {
    for (os, mods) in [
        (OperatingSystem::Mac, Modifiers { mac_cmd: true, command: true, ..Modifiers::NONE }),
        (OperatingSystem::Windows, Modifiers { ctrl: true, command: true, ..Modifiers::NONE }),
        (OperatingSystem::Nix, Modifiers { ctrl: true, command: true, ..Modifiers::NONE }),
    ] {
        let mut h = harness(os);
        click(&mut h, "Three", PointerButton::Primary, mods);
        assert_eq!(h.state().session.doc().unwrap().selected_sheets, vec![0, 2]);
        assert_eq!(h.state().session.doc().unwrap().wb.active_sheet, 2);
        click(&mut h, "Four", PointerButton::Primary, Modifiers::SHIFT);
        assert_eq!(h.state().session.doc().unwrap().selected_sheets, vec![0, 1, 2, 3]);
        click(&mut h, "Two", PointerButton::Primary, Modifiers::NONE);
        assert_eq!(h.state().session.doc().unwrap().selected_sheets, vec![1]);
    }
}
#[test]
fn context_menu_can_select_all_visible_sheets_and_ungroup() {
    let mut h = harness(OperatingSystem::Mac);
    h.state_mut().session.execute("sheet.hide", json!({"sheet":2})).unwrap();
    h.run_steps(3);
    click(&mut h, "Sheet1", PointerButton::Primary, Modifiers { ctrl: true, command: true, ..Modifiers::NONE });
    assert_eq!(h.state().session.doc().unwrap().selected_sheets, vec![0]);
    h.get_by_label("Select All Sheets").click();
    h.run_steps(3);
    assert_eq!(h.state().session.doc().unwrap().selected_sheets, vec![0, 1, 3]);
    click(&mut h, "Two", PointerButton::Secondary, Modifiers::NONE);
    h.get_by_label("Ungroup Sheets").click();
    h.run_steps(3);
    assert_eq!(h.state().session.doc().unwrap().selected_sheets, vec![1]);
}
