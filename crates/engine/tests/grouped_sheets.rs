//! Group selection and edits are public command workflows with atomic history/protection.
use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
    model::Visibility,
};
use serde_json::{Value as Json, json};

fn run(s: &mut Session, id: &str, p: Json) -> Json {
    s.execute(id, p).unwrap()
}
fn book() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    for name in ["Two", "Three", "Four"] {
        run(&mut s, "home.insertSheet", json!({"name":name}));
    }
    run(&mut s, "sheet.activate", json!({"sheet":0}));
    s
}
fn group(s: &mut Session) {
    run(s, "sheet.activate", json!({"sheet":2,"mode":"extend"}));
}
fn value(s: &Session, sheet: usize, at: &str) -> Value {
    s.doc().unwrap().wb.sheet(sheet).unwrap().value(CellRef::parse(at).unwrap())
}
fn names(s: &Session) -> Vec<String> {
    s.doc().unwrap().wb.sheets.iter().map(|sh| sh.name.clone()).collect()
}
fn bold(s: &Session, sheet: usize) -> bool {
    let wb = &s.doc().unwrap().wb;
    wb.styles.get(wb.sheet(sheet).unwrap().style_id(CellRef::default())).font.bold
}

#[test]
fn activation_modes_preserve_a_visible_nonempty_group_and_fixed_shift_anchor() {
    let mut s = book();
    run(&mut s, "sheet.hide", json!({"sheet":1}));
    let saved = s.doc().unwrap().wb.clone();
    s.doc_mut().unwrap().saved = saved;
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "sheet.activate", json!({"sheet":3,"mode":"extend"}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 2, 3]);
    assert_eq!(s.doc().unwrap().sheet_selection_anchor, 0);
    run(&mut s, "sheet.activate", json!({"sheet":2,"mode":"extend"}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 2]);
    run(&mut s, "sheet.activate", json!({"sheet":2,"mode":"toggle"}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0]);
    assert_eq!(s.doc().unwrap().wb.active_sheet, 0);
    run(&mut s, "sheet.activate", json!({"sheet":0,"mode":"toggle"}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0]);
    run(&mut s, "sheet.activate", json!({"mode":"all"}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 2, 3]);
    run(&mut s, "sheet.activate", json!({"mode":"ungroup"}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0]);
    assert_eq!(s.doc().unwrap().undo.len(), undo);
    assert!(!s.doc().unwrap().is_dirty());
    let before = s.doc().unwrap().selected_sheets.clone();
    assert!(s.execute("sheet.activate", json!({"sheet":1,"mode":"all"})).is_err());
    assert_eq!(s.doc().unwrap().selected_sheets, before);
}

#[test]
fn group_entry_and_mixed_format_toggle_apply_one_active_state_with_one_undo() {
    let mut s = book();
    run(&mut s, "home.bold", json!({"sheet":0,"range":"A1","on":true}));
    run(&mut s, "selection.set", json!({"range":"A1:A2,C1:C2"}));
    group(&mut s);
    // Active sheet 2 starts unbold. Both a bold and an unbold peer must become bold.
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "home.bold", json!({}));
    for sheet in 0..3 {
        assert!(bold(&s, sheet));
    }
    assert!(!bold(&s, 3));
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    assert!(bold(&s, 0));
    assert!(!bold(&s, 1));
    assert!(!bold(&s, 2));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 1, 2]);
    run(&mut s, "range.fill", json!({"range":"A1:A2,C1:C2","input":"=ROW()+COLUMN()"}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "C2"), Value::number(5.0));
    }
    assert_eq!(value(&s, 3, "C2"), Value::Empty);
    run(&mut s, "edit.undo", json!({}));
    for sheet in 0..4 {
        assert_eq!(value(&s, sheet, "C2"), Value::Empty);
    }
    run(&mut s, "edit.redo", json!({}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "C2"), Value::number(5.0));
    }
}

#[test]
fn implicit_cell_entry_fans_out_while_explicit_sheet_stays_single() {
    let mut s = book();
    group(&mut s);
    run(&mut s, "cell.set", json!({"cell":"B2","input":"8"}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "B2"), Value::number(8.0));
    }
    assert_eq!(value(&s, 3, "B2"), Value::Empty);
    run(&mut s, "cell.set", json!({"sheet":1,"cell":"C3","input":"single"}));
    assert_eq!(value(&s, 1, "C3"), Value::text("single"));
    assert_eq!(value(&s, 0, "C3"), Value::Empty);
    assert_eq!(value(&s, 2, "C3"), Value::Empty);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 1, 2]);
}

#[test]
fn later_protected_group_member_rolls_back_cells_format_clipboard_and_history() {
    let mut s = book();
    run(&mut s, "sheet.activate", json!({"sheet":2}));
    run(&mut s, "review.protectSheet", json!({}));
    run(&mut s, "sheet.activate", json!({"sheet":0}));
    group(&mut s);
    run(&mut s, "edit.copy", json!({"range":"D4"}));
    let before = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    for (id, p) in [("cell.set", json!({"cell":"A1","input":"5"})), ("home.bold", json!({"range":"A1","on":true}))] {
        assert!(s.execute(id, p).is_err(), "{id}");
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().undo.len(), undo);
        assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 1, 2]);
        assert!(s.clipboard.is_some());
    }
}

#[test]
fn copy_and_external_paste_reach_grouped_sheets_with_one_undo_and_cut_is_refused() {
    let mut s = book();
    run(&mut s, "cell.set", json!({"cell":"A1","input":"12"}));
    run(&mut s, "edit.copy", json!({"range":"A1"}));
    group(&mut s);
    run(&mut s, "selection.set", json!({"range":"C1:C2,E1:E2"}));
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "edit.paste", json!({}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "E2"), Value::number(12.0));
    }
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    assert!(s.clipboard.is_some());
    run(&mut s, "edit.undo", json!({}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "E2"), Value::Empty);
    }
    run(&mut s, "edit.paste", json!({"text":"21"}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "E2"), Value::number(21.0));
    }
    run(&mut s, "edit.cut", json!({"range":"A1"}));
    let before = s.doc().unwrap().wb.clone();
    assert!(s.execute("edit.paste", json!({"at":"G1"})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
    assert!(s.clipboard.as_ref().unwrap().cut);
}

#[test]
fn cumulative_group_work_is_rejected_before_any_member_is_edited() {
    let mut s = book();
    group(&mut s);
    // Every member is within range.fill's individual 2M limit, but the group exceeds 5M.
    run(&mut s, "selection.set", json!({"range":"A1:B1000000"}));
    let before = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    assert!(s.execute("range.fill", json!({"input":"x"})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
    assert_eq!(s.doc().unwrap().undo.len(), undo);
}

#[test]
fn tab_color_hide_delete_and_undo_apply_to_the_selected_group() {
    let mut s = book();
    group(&mut s);
    run(&mut s, "sheet.tabColor", json!({"sheet":1,"color":"#FF0000"}));
    for sheet in 0..3 {
        assert!(s.doc().unwrap().wb.sheet(sheet).unwrap().tab_color.is_some());
    }
    assert!(s.doc().unwrap().wb.sheet(3).unwrap().tab_color.is_none());
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "sheet.hide", json!({"sheet":1}));
    for sheet in 0..3 {
        assert_eq!(s.doc().unwrap().wb.sheet(sheet).unwrap().visibility, Visibility::Hidden);
    }
    assert_eq!(s.doc().unwrap().selected_sheets, vec![3]);
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 1, 2]);
    run(&mut s, "home.deleteSheet", json!({"sheet":1}));
    assert_eq!(names(&s), vec!["Four"]);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0]);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), vec!["Sheet1", "Two", "Three", "Four"]);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 1, 2]);
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(names(&s), vec!["Four"]);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0]);
}

#[test]
fn group_move_and_copy_preserve_source_order_scope_and_one_history_step() {
    let mut s = book();
    run(&mut s, "cell.set", json!({"sheet":0,"cell":"A1","input":"1"}));
    run(&mut s, "cell.set", json!({"sheet":2,"cell":"A1","input":"3"}));
    run(&mut s, "sheet.activate", json!({"sheet":2,"mode":"toggle"}));
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "sheet.move", json!({"to":2}));
    assert_eq!(names(&s), vec!["Two", "Four", "Sheet1", "Three"]);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![2, 3]);
    assert_eq!(s.doc().unwrap().wb.active_sheet, 3);
    assert_eq!(value(&s, 2, "A1"), Value::number(1.0));
    assert_eq!(value(&s, 3, "A1"), Value::number(3.0));
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), vec!["Sheet1", "Two", "Three", "Four"]);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 2]);
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "sheet.move", json!({"to":4,"copy":true}));
    assert_eq!(names(&s), vec!["Sheet1", "Two", "Three", "Four", "Sheet1 (2)", "Three (2)"]);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![4, 5]);
    assert_eq!(value(&s, 4, "A1"), Value::number(1.0));
    assert_eq!(value(&s, 5, "A1"), Value::number(3.0));
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0, 2]);
}

#[test]
fn last_visible_group_cannot_be_hidden_or_deleted_and_invalid_mode_is_inert() {
    let mut s = book();
    run(&mut s, "sheet.activate", json!({"mode":"all"}));
    let before = s.doc().unwrap().wb.clone();
    let selected = s.doc().unwrap().selected_sheets.clone();
    for (id, p) in [("sheet.hide", json!({})), ("home.deleteSheet", json!({})), ("sheet.activate", json!({"sheet":2,"mode":"bad"}))] {
        assert!(s.execute(id, p).is_err());
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().selected_sheets, selected);
    }
}

#[test]
fn existing_three_dimensional_formula_parser_recalculates_the_selected_span() {
    let mut s = book();
    for (sheet, n) in [(0, 2), (1, 3), (2, 5)] {
        run(&mut s, "cell.set", json!({"sheet":sheet,"cell":"A1","value":n}));
    }
    group(&mut s);
    run(&mut s, "cell.set", json!({"sheet":3,"cell":"B1","input":"=SUM(Sheet1:Three!A1)"}));
    assert_eq!(value(&s, 3, "B1"), Value::number(10.0));
}

#[test]
fn grouped_autofill_series_and_autosum_reach_each_sheet_with_one_undo() {
    let mut s = book();
    for sheet in 0..3 {
        let n = sheet + 1;
        run(&mut s, "range.setValues", json!({"sheet":sheet,"range":"A1","values":[[n],[n*2]]}));
    }
    group(&mut s);
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "edit.autoFill", json!({"source":"A1:A2","target":"A1:A4"}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "A4"), Value::number(((sheet + 1) * 4) as f64));
    }
    assert_eq!(value(&s, 3, "A4"), Value::Empty);
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "A4"), Value::Empty);
    }
    run(&mut s, "selection.set", json!({"range":"A1:A3"}));
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "formulas.autoSum", json!({}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "A3"), Value::number(((sheet + 1) * 3) as f64));
    }
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "cell.set", json!({"cell":"C1","value":5}));
    run(&mut s, "selection.set", json!({"range":"C1:C3"}));
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "edit.fillSeries", json!({"type":"linear","step":2}));
    for sheet in 0..3 {
        assert_eq!(value(&s, sheet, "C3"), Value::number(9.0));
    }
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
}

#[test]
fn grouped_autosum_empty_cell_requests_one_editor_without_creating_history() {
    let mut s = book();
    group(&mut s);
    let undo = s.doc().unwrap().undo.len();
    let before = s.doc().unwrap().wb.clone();
    run(&mut s, "formulas.autoSum", json!({}));
    assert_eq!(s.doc().unwrap().undo.len(), undo);
    assert_eq!(s.doc().unwrap().wb, before);
    let requests = s.take_ui_requests();
    assert_eq!(requests.len(), 1);
    assert!(matches!(&requests[0],gridcraft_engine::UiRequest::EditCell(Some(text)) if text=="=SUM()"));
}
