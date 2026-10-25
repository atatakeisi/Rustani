//! Function menus edit the current TextEdit draft without committing or replacing its context.
use std::sync::Arc;

use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Shape, TextEdit, accesskit::Role};
use egui_kittest::kittest::Queryable;
use gridcraft_engine::{Selection, Session, core::CellRef, formula::FormulaLocale, model::Workbook};
use gridcraft_ui_egui::{SheetApp, i18n::Language};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;
type GridSnapshot = (Arc<Workbook>, Selection, usize);
type EditorContext = (usize, CellRef, FormulaLocale, bool, bool);

fn harness() -> Harness {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("cell.set", json!({"cell":"A1","input":"grid remains unchanged 日本語🙂"})).unwrap();
    s.execute("cell.set", json!({"cell":"B3","input":"original cell"})).unwrap();
    s.execute("selection.set", json!({"range":"B3"})).unwrap();
    let mut app = SheetApp::new(s, Default::default());
    app.ui.language = Language::En;
    app.ui.ribbon_tab = "Formulas".into();
    let mut h = Harness::builder().with_step_dt(0.02).with_size(egui::vec2(1500.0, 950.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

fn begin(h: &mut Harness, text: &str, from_formula_bar: bool) {
    h.state_mut().begin_edit(Some(text.into()), from_formula_bar);
    h.run_steps(3);
    assert_eq!(draft(h), text);
}

fn draft(h: &Harness) -> &str {
    &h.state().editor.as_ref().expect("function insertion keeps editing open").text
}

fn context(h: &Harness) -> EditorContext {
    let e = h.state().editor.as_ref().unwrap();
    (e.sheet, e.cell, e.locale, e.from_formula_bar, e.enter_mode)
}

fn editor_id(h: &Harness) -> Id {
    let e = h.state().editor.as_ref().unwrap();
    if e.from_formula_bar || h.state().session.doc().unwrap().wb.active_sheet != e.sheet {
        Id::new("gridcraft.formula_bar")
    } else {
        Id::new("gridcraft.cell_editor")
    }
}

fn select_chars(h: &mut Harness, start: usize, end: usize) {
    let id = editor_id(h);
    let mut state = TextEdit::load_state(&h.ctx, id).expect("the visible source editor has TextEdit state");
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(start), egui::text::CCursor::new(end))));
    state.store(&h.ctx, id);
    let e = h.state_mut().editor.as_mut().unwrap();
    e.caret = end;
    e.request_focus = false;
    e.sync_caret = false;
    e.caret_to_end = false;
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.run_steps(1);
}

fn grid_snapshot(h: &Harness) -> GridSnapshot {
    let d = h.state().session.doc().unwrap();
    (d.wb.clone(), d.selection.clone(), d.undo.len())
}

fn assert_grid_unchanged(h: &Harness, before: &GridSnapshot) {
    let d = h.state().session.doc().unwrap();
    assert_eq!(d.wb, before.0, "inserting a function must not commit a draft into the worksheet");
    assert_eq!(d.selection, before.1);
    assert_eq!(d.undo.len(), before.2, "function insertion belongs to TextEdit's Undo history");
    assert!(h.state().message.is_none(), "incomplete formula drafts must not be submitted for validation");
}

fn click_at(h: &mut Harness, pos: Pos2) {
    for event in [
        Event::PointerMoved(pos),
        Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.input_mut().events.push(event);
        h.run_steps(1);
    }
    h.run_steps(2);
}

// The ribbon's hand-painted large buttons have no text accessibility node. Click their
// displayed labels, while function-menu entries and dialog fields use their real widgets.
fn painted_text_position(h: &Harness, label: &str) -> Pos2 {
    fn find(shape: &Shape, clip: egui::Rect, label: &str) -> Option<Pos2> {
        match shape {
            Shape::Text(t) if t.galley.text() == label && clip.contains(t.visual_bounding_rect().center()) => Some(t.visual_bounding_rect().center()),
            Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, clip, label)),
            _ => None,
        }
    }
    h.output()
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, shape.clip_rect, label))
        .unwrap_or_else(|| panic!("painted control not found: {label}"))
}

fn ribbon_if(h: &mut Harness) {
    let pos = painted_text_position(h, "Logical");
    click_at(h, pos);
    h.get_by_role_and_label(Role::Button, "IF").click();
    h.run_steps(4);
}

fn undo(h: &mut Harness) {
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
}

#[test]
fn ribbon_function_appends_to_each_editor_and_one_text_undo_restores_the_existing_draft() {
    for from_formula_bar in [false, true] {
        let mut h = harness();
        begin(&mut h, "=10+", from_formula_bar);
        select_chars(&mut h, 4, 4);
        let before = grid_snapshot(&h);
        let origin = context(&h);
        ribbon_if(&mut h);
        assert_eq!(draft(&h), "=10+IF(");
        assert_eq!(h.state().editor.as_ref().unwrap().caret, 7);
        assert_eq!(context(&h), origin);
        assert_grid_unchanged(&h, &before);
        undo(&mut h);
        assert_eq!(draft(&h), "=10+", "one Undo removes only the inserted function");
        assert_grid_unchanged(&h, &before);
    }
}

#[test]
fn ribbon_function_uses_the_caret_instead_of_the_end_of_the_formula() {
    let mut h = harness();
    begin(&mut h, "=10+20", false);
    select_chars(&mut h, 4, 4);
    let before = grid_snapshot(&h);
    ribbon_if(&mut h);
    assert_eq!(draft(&h), "=10+IF(20");
    assert_eq!(h.state().editor.as_ref().unwrap().caret, 7);
    assert_grid_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(draft(&h), "=10+20");
}

#[test]
fn inserted_function_redoes_as_one_text_edit_in_both_editor_widgets() {
    for from_formula_bar in [false, true] {
        let mut h = harness();
        begin(&mut h, "=10+20", from_formula_bar);
        select_chars(&mut h, 4, 4);
        let before = grid_snapshot(&h);
        let origin = context(&h);
        ribbon_if(&mut h);
        assert_eq!(draft(&h), "=10+IF(20");
        undo(&mut h);
        assert_eq!(draft(&h), "=10+20");
        h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        h.run_steps(3);
        assert_eq!(draft(&h), "=10+IF(20", "one Redo restores the same insertion, including its untouched suffix");
        assert_eq!(h.state().editor.as_ref().unwrap().caret, 7);
        assert_eq!(context(&h), origin);
        assert_grid_unchanged(&h, &before);
    }
}

#[test]
fn ribbon_function_replaces_a_unicode_selection_using_character_positions() {
    let mut h = harness();
    const ORIGINAL: &str = "=\"保持🙂\"&\"日本語\"+20";
    const SELECTED: &str = "\"日本語\"";
    begin(&mut h, ORIGINAL, true);
    let start = ORIGINAL[..ORIGINAL.find(SELECTED).unwrap()].chars().count();
    select_chars(&mut h, start, start + SELECTED.chars().count());
    let before = grid_snapshot(&h);
    let origin = context(&h);
    ribbon_if(&mut h);
    assert_eq!(draft(&h), "=\"保持🙂\"&IF(+20");
    assert_eq!(h.state().editor.as_ref().unwrap().caret, start + 3);
    assert_eq!(context(&h), origin);
    assert_grid_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(draft(&h), ORIGINAL);
    let restored = TextEdit::load_state(&h.ctx, editor_id(&h)).unwrap().cursor.char_range().unwrap();
    let indices: [usize; 2] = restored.sorted_cursors().map(|cursor| cursor.index.into());
    assert_eq!(indices, [start, start + SELECTED.chars().count()], "Undo restores the original Unicode selection as well as its text");
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    h.run_steps(3);
    assert_eq!(draft(&h), "=\"保持🙂\"&IF(+20");
    assert_eq!(h.state().editor.as_ref().unwrap().caret, start + 3);
    assert_grid_unchanged(&h, &before);
}

#[test]
fn nonformula_draft_becomes_a_formula_and_one_undo_restores_the_uncommitted_text() {
    let mut h = harness();
    begin(&mut h, "未確定の文字🙂", true);
    select_chars(&mut h, 2, 2);
    let before = grid_snapshot(&h);
    let origin = context(&h);
    ribbon_if(&mut h);
    assert_eq!(draft(&h), "=IF(");
    assert_eq!(context(&h), origin);
    assert_grid_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(draft(&h), "未確定の文字🙂");
    assert_grid_unchanged(&h, &before);
}

#[test]
fn insert_function_dialog_uses_the_preserved_selection_and_the_same_text_undo_path() {
    let mut h = harness();
    begin(&mut h, "=10+20", true);
    select_chars(&mut h, 4, 6);
    let before = grid_snapshot(&h);
    let origin = context(&h);
    let pos = painted_text_position(&h, "Function");
    click_at(&mut h, pos);
    assert!(h.state().dialog.is_some());
    h.input_mut().events.push(Event::Text("SUM".into()));
    h.run_steps(3);
    h.get_by_role_and_label(Role::Button, &format!("{:<18} {}", "SUM", "Math & Trig")).click();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(4);
    assert!(h.state().dialog.is_none());
    assert_eq!(draft(&h), "=10+SUM(");
    assert_eq!(context(&h), origin);
    assert_grid_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(draft(&h), "=10+20");
    assert_grid_unchanged(&h, &before);
    let restored = TextEdit::load_state(&h.ctx, editor_id(&h)).unwrap().cursor.char_range().unwrap();
    let indices: [usize; 2] = restored.sorted_cursors().map(|cursor| cursor.index.into());
    assert_eq!(indices, [4, 6], "the dialog preserves the formula selection while its search field owns focus");
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    h.run_steps(3);
    assert_eq!(draft(&h), "=10+SUM(");
    assert_grid_unchanged(&h, &before);
}

#[test]
fn insertion_uses_the_edits_captured_formula_locale_after_the_interface_language_changes() {
    let mut h = harness();
    h.state_mut().run("app.language.set", json!({"language":"es"})).unwrap();
    begin(&mut h, "=10+", true);
    h.state_mut().run("app.language.set", json!({"language":"en"})).unwrap();
    h.run_steps(4);
    select_chars(&mut h, 4, 4);
    let before = grid_snapshot(&h);
    let origin = context(&h);
    assert_eq!(origin.2, FormulaLocale::Es);
    ribbon_if(&mut h);
    assert_eq!(draft(&h), "=10+SI(", "the open draft keeps Spanish formula names despite the English UI");
    assert_eq!(context(&h), origin);
    assert_grid_unchanged(&h, &before);
    undo(&mut h);
    assert_eq!(draft(&h), "=10+");
}

#[test]
fn insertion_keeps_the_origin_cell_when_formula_point_mode_has_visited_another_sheet() {
    let mut h = harness();
    h.state_mut().run("home.insertSheet", json!({"name":"Other"})).unwrap();
    h.state_mut().run("sheet.activate", json!({"sheet":0})).unwrap();
    h.state_mut().run("selection.set", json!({"cell":"B3"})).unwrap();
    begin(&mut h, "=10+", false);
    h.state_mut().session.execute("sheet.activate", json!({"sheet":"Other"})).unwrap();
    h.run_steps(3);
    select_chars(&mut h, 4, 4);
    let before = grid_snapshot(&h);
    let origin = context(&h);
    assert_eq!(origin.0, 0);
    assert_eq!(origin.1, CellRef::parse("B3").unwrap());
    ribbon_if(&mut h);
    assert_eq!(draft(&h), "=10+IF(");
    assert_eq!(context(&h), origin);
    assert_grid_unchanged(&h, &before);
}

#[test]
fn inserting_without_an_editor_starts_a_formula_draft_without_committing_the_cell() {
    let mut h = harness();
    let before = grid_snapshot(&h);
    assert!(h.state().editor.is_none());
    h.state_mut().run("formulas.insertFunction", json!({"name":"SUM"})).unwrap();
    h.run_steps(4);
    assert_eq!(draft(&h), "=SUM(");
    assert_eq!(context(&h).1, CellRef::parse("B3").unwrap());
    assert_grid_unchanged(&h, &before);
}
