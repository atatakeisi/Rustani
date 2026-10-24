//! A Shift-click on sheet tabs chooses a 3D formula reference without grouping later edits.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, accesskit::Role, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
};
use gridcraft_ui_egui::{SheetApp, grid::Geo};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;
const FIRST: &str = "First Sheet";
const LAST: &str = "Last's Sheet";
const SPAN: &str = "'First Sheet:Last''s Sheet'!";

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("home.insertSheet", json!({"name": "Middle"})).unwrap();
    session.execute("home.insertSheet", json!({"name": LAST})).unwrap();
    session.execute("sheet.rename", json!({"sheet": 0, "name": FIRST})).unwrap();
    for sheet in 0..3 {
        session.execute("sheet.activate", json!({"sheet": sheet})).unwrap();
        session.execute("cell.set", json!({"cell": "A1", "input": (sheet + 1).to_string()})).unwrap();
    }
    session.execute("sheet.activate", json!({"sheet": 0})).unwrap();
    session.execute("selection.set", json!({"cell": "F8"})).unwrap();
    let mut h = Harness::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.run_steps(4);
    h.state_mut().begin_edit(Some("=SUM(".into()), false);
    h.run_steps(2);
    h
}

fn click(h: &mut Harness, pos: Pos2, modifiers: Modifiers) {
    h.input_mut().events.push(Event::ModifiersChanged(modifiers));
    for pressed in [true, false] {
        h.input_mut().events.extend([Event::PointerMoved(pos), Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }]);
        h.step();
    }
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn tab(h: &mut Harness, name: &str, modifiers: Modifiers) {
    let pos = h.get_by_role_and_label(Role::Button, name).rect().center();
    click(h, pos, modifiers);
}

fn cell(h: &Harness, address: &str) -> Pos2 {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll).cell_rect(sh, CellRef::parse(address).unwrap()).center()
}

#[test]
fn shift_tab_then_cell_inserts_a_quoted_span_and_calculates_on_the_origin_sheet() {
    let mut h = harness();
    tab(&mut h, LAST, Modifiers::SHIFT);
    assert_eq!(h.state().session.doc().unwrap().selected_sheets, [2], "point mode navigates without grouping writes");
    let at = cell(&h, "A1");
    click(&mut h, at, Modifiers::NONE);
    assert_eq!(h.state().editor.as_ref().unwrap().text, format!("=SUM({SPAN}A1"));
    h.input_mut().events.push(Event::Text(")".into()));
    h.run_steps(2);
    assert_eq!(h.state().editor.as_ref().unwrap().text, format!("=SUM({SPAN}A1)"));
    h.key_press(Key::Enter);
    h.run_steps(2);
    assert!(h.state().editor.is_none(), "Enter commits through the visible formula editor on another sheet");
    let doc = h.state().session.doc().unwrap();
    assert_eq!(doc.wb.active_sheet, 0);
    assert_eq!(doc.wb.sheet(0).unwrap().value(CellRef::parse("F8").unwrap()), Value::number(6.0));
    assert!(doc.wb.sheet(2).unwrap().cell(CellRef::parse("F8").unwrap()).is_none());
}

#[test]
fn arrows_and_header_extension_keep_the_span_and_a_plain_tab_clears_it() {
    let mut h = harness();
    tab(&mut h, LAST, Modifiers::SHIFT);
    let at = cell(&h, "A1");
    click(&mut h, at, Modifiers::NONE);
    h.key_press(Key::ArrowRight);
    h.run_steps(2);
    assert_eq!(h.state().editor.as_ref().unwrap().text, format!("=SUM({SPAN}B1"));
    for (column, modifiers) in [(2, Modifiers::NONE), (4, Modifiers::SHIFT)] {
        let app = h.state();
        let sh = app.session.doc().unwrap().wb.active().unwrap();
        let geo = Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
        let at = pos2(geo.x(sh, column) + sh.col_width(column) * geo.z * 0.5, geo.rect.top() + geo.header_h * 0.5);
        click(&mut h, at, modifiers);
    }
    assert_eq!(h.state().editor.as_ref().unwrap().text, format!("=SUM({SPAN}C:E"));
    tab(&mut h, "Middle", Modifiers::NONE);
    let at = cell(&h, "A1");
    click(&mut h, at, Modifiers::NONE);
    assert_eq!(h.state().editor.as_ref().unwrap().text, "=SUM(Middle!A1");
}

#[test]
fn dragging_a_cell_reference_keeps_the_explicit_sheet_span() {
    let mut h = harness();
    tab(&mut h, LAST, Modifiers::SHIFT);
    let start = cell(&h, "C3");
    let middle = cell(&h, "D4");
    let end = cell(&h, "E5");
    h.input_mut().events.extend([
        Event::PointerMoved(start),
        Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
    ]);
    h.step();
    h.input_mut().events.push(Event::PointerMoved(middle));
    h.step();
    assert_eq!(h.state().editor.as_ref().unwrap().text, format!("=SUM({SPAN}C3:D4"));
    h.input_mut().events.push(Event::PointerButton { pos: end, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert_eq!(h.state().editor.as_ref().unwrap().text, format!("=SUM({SPAN}C3:E5"));
    assert_eq!(h.state().session.doc().unwrap().selected_sheets, [2]);
}
