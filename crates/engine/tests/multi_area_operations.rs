//! Public command workflows: discontiguous edits keep their scope and one Undo boundary.
use gridcraft_engine::{
    Session,
    core::{CellRef, RangeRef, Value},
};
use serde_json::{Value as Json, json};

fn book() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s
}
fn run(s: &mut Session, id: &str, p: Json) -> Json {
    s.execute(id, p).unwrap()
}
fn set(s: &mut Session, at: &str, text: &str) {
    run(s, "cell.set", json!({"cell":at,"input":text}));
}
fn value(s: &Session, at: &str) -> Value {
    s.doc().unwrap().wb.active().unwrap().value(CellRef::parse(at).unwrap())
}

#[test]
fn edits_end_copy_mode_but_repeated_copy_pastes_keep_the_snapshot() {
    for (id, params) in [
        ("cell.set", json!({"cell":"C2","input":"new"})),
        ("range.fill", json!({"range":"C2:C3","input":"new"})),
        ("edit.clearContents", json!({"range":"B1"})),
        ("home.bold", json!({"range":"B1","on":true})),
        ("edit.fillDown", json!({"range":"B1:B2"})),
    ] {
        let mut s = book();
        set(&mut s, "A1", "source");
        set(&mut s, "B1", "old");
        run(&mut s, "edit.copy", json!({"range":"A1"}));
        run(&mut s, id, params);
        assert!(s.clipboard.is_none(), "{id}");
    }
    let mut s = book();
    set(&mut s, "A1", "42");
    run(&mut s, "edit.copy", json!({"range":"A1"}));
    run(&mut s, "edit.paste", json!({"at":"C1"}));
    run(&mut s, "edit.pasteSpecial", json!({"at":"E1","what":"values"}));
    assert_eq!(value(&s, "C1"), Value::number(42.0));
    assert_eq!(value(&s, "E1"), Value::number(42.0));
    assert!(s.clipboard.is_some());
    run(&mut s, "review.protectSheet", json!({}));
    run(&mut s, "edit.copy", json!({"range":"A1"}));
    assert!(s.execute("cell.set", json!({"cell":"D1","value":3})).is_err());
    assert!(s.clipboard.is_some());
}

#[test]
fn command_error_restores_a_partial_workbook_and_selection_without_ending_copy_mode() {
    gridcraft_engine::register_test_command("test.partialEditThenError", |s, _| {
        let d = s.doc_mut()?;
        let wb = std::sync::Arc::make_mut(&mut d.wb);
        wb.sheet_mut(0)
            .ok_or(gridcraft_engine::EngineError::NoDocument)?
            .set_cell(CellRef::default(), gridcraft_engine::model::Cell::value(Value::text("partial")));
        d.selection = gridcraft_engine::Selection::at(CellRef::new(8, 8));
        d.selected_sheets.clear();
        d.sheet_selection_anchor = 99;
        Err(gridcraft_engine::EngineError::Other("late validation failed".into()))
    });
    let mut s = book();
    set(&mut s, "A1", "original");
    run(&mut s, "edit.copy", json!({"range":"A1"}));
    run(&mut s, "selection.set", json!({"range":"C2,D3"}));
    let before = s.doc().unwrap().wb.clone();
    let selection = s.doc().unwrap().selection.clone();
    let undo = s.doc().unwrap().undo.len();
    assert!(s.execute("test.partialEditThenError", json!({})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
    assert_eq!(s.doc().unwrap().selection, selection);
    assert_eq!(s.doc().unwrap().selected_sheets, vec![0]);
    assert_eq!(s.doc().unwrap().sheet_selection_anchor, 0);
    assert_eq!(s.doc().unwrap().undo.len(), undo);
    assert!(s.clipboard.is_some());
}

#[test]
fn every_direction_fills_each_area_with_one_undo_and_preserves_selection() {
    for (id, selection, seeds, targets) in [
        ("edit.fillDown", "A1:A3,C1:C3", ["A1", "C1"], ["A3", "C3"]),
        ("edit.fillUp", "A1:A3,C1:C3", ["A3", "C3"], ["A1", "C1"]),
        ("edit.fillRight", "A1:C1,A3:C3", ["A1", "A3"], ["C1", "C3"]),
        ("edit.fillLeft", "A1:C1,A3:C3", ["C1", "C3"], ["A1", "A3"]),
    ] {
        let mut s = book();
        set(&mut s, seeds[0], "11");
        set(&mut s, seeds[1], "23");
        run(&mut s, "selection.set", json!({"range":selection}));
        let before = s.doc().unwrap().wb.clone();
        let selection = s.doc().unwrap().selection.clone();
        let undo = s.doc().unwrap().undo.len();
        run(&mut s, id, json!({}));
        assert_eq!(value(&s, targets[0]), Value::number(11.0), "{id}");
        assert_eq!(value(&s, targets[1]), Value::number(23.0), "{id}");
        assert_eq!(s.doc().unwrap().selection, selection);
        assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.doc().unwrap().wb, before);
        run(&mut s, "edit.redo", json!({}));
        assert_eq!(value(&s, targets[1]), Value::number(23.0));
    }
}

#[test]
fn later_protected_fill_destination_rolls_back_every_area() {
    let mut s = book();
    set(&mut s, "A1", "2");
    set(&mut s, "C1", "5");
    run(&mut s, "home.lockCell", json!({"range":"A1:A3","on":false}));
    run(&mut s, "review.protectSheet", json!({}));
    run(&mut s, "selection.set", json!({"range":"A1:A3,C1:C3"}));
    let before = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    assert!(s.execute("edit.fillDown", json!({})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
    assert_eq!(s.doc().unwrap().undo.len(), undo);
}

#[test]
fn singleton_fill_reads_its_original_preceding_source_even_after_another_area_wrote_it() {
    let mut s = book();
    set(&mut s, "A2", "2");
    set(&mut s, "A3", "9");
    run(&mut s, "selection.set", json!({"range":"A2:A3,A4"}));
    run(&mut s, "edit.fillDown", json!({}));
    assert_eq!(value(&s, "A3"), Value::number(2.0));
    assert_eq!(value(&s, "A4"), Value::number(9.0));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(value(&s, "A3"), Value::number(9.0));
    assert_eq!(value(&s, "A4"), Value::Empty);
}

#[test]
fn row_and_column_expansion_retains_every_area_and_the_active_cell() {
    let mut s = book();
    for (id, expected) in
        [("selection.row", vec![RangeRef::rows(1, 2), RangeRef::rows(5, 6)]), ("selection.column", vec![RangeRef::cols(1, 2), RangeRef::cols(4, 5)])]
    {
        run(&mut s, "selection.set", json!({"range":"B2:C3,E6:F7","active":"F7"}));
        run(&mut s, id, json!({}));
        assert_eq!(s.doc().unwrap().selection.ranges, expected);
        assert_eq!(s.doc().unwrap().selection.active, CellRef::parse("F7").unwrap());
    }
}

#[test]
fn row_insert_uses_original_coordinates_and_merges_duplicate_row_targets() {
    let mut s = book();
    run(&mut s, "range.setValues", json!({"range":"A1","values":[[1],[2],[3],[4],[5],[6],[7],[8]]}));
    run(&mut s, "selection.set", json!({"range":"B2,D2,B5"}));
    let before = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "home.insertRows", json!({}));
    for (at, n) in [("A1", 1), ("A3", 2), ("A5", 4), ("A7", 5), ("A10", 8)] {
        assert_eq!(value(&s, at), Value::number(n as f64));
    }
    assert_eq!(value(&s, "A2"), Value::Empty);
    assert_eq!(value(&s, "A6"), Value::Empty);
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc().unwrap().wb, before);
}

#[test]
fn column_insert_and_delete_use_all_original_areas() {
    let mut s = book();
    run(&mut s, "range.setValues", json!({"range":"A1","values":[[1,2,3,4,5,6,7,8]]}));
    run(&mut s, "selection.set", json!({"range":"B2,E4"}));
    run(&mut s, "home.insertColumns", json!({}));
    assert_eq!(value(&s, "C1"), Value::number(2.0));
    assert_eq!(value(&s, "G1"), Value::number(5.0));
    assert_eq!(value(&s, "J1"), Value::number(8.0));
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "home.deleteColumns", json!({}));
    assert_eq!(value(&s, "B1"), Value::number(3.0));
    assert_eq!(value(&s, "D1"), Value::number(6.0));
}

#[test]
fn cell_insert_delete_reverse_the_shift_axis_without_coordinate_drift() {
    for horizontal in [false, true] {
        let mut s = book();
        let (values, ranges, insert, delete, check1, check2, checklast) = if horizontal {
            (json!([[1, 2, 3, 4, 5, 6, 7, 8]]), "B1,E1", "right", "left", "C1", "G1", "J1")
        } else {
            (json!([[1], [2], [3], [4], [5], [6], [7], [8]]), "A2,A5", "down", "up", "A3", "A7", "A10")
        };
        run(&mut s, "range.setValues", json!({"range":"A1","values":values}));
        run(&mut s, "selection.set", json!({"range":ranges}));
        let before = s.doc().unwrap().wb.clone();
        let undo = s.doc().unwrap().undo.len();
        run(&mut s, "home.insertCells", json!({"shift":insert}));
        assert_eq!(value(&s, check1), Value::number(2.0));
        assert_eq!(value(&s, check2), Value::number(5.0));
        assert_eq!(value(&s, checklast), Value::number(8.0));
        assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.doc().unwrap().wb, before);
        run(&mut s, "home.deleteCells", json!({"shift":delete}));
        assert_eq!(value(&s, if horizontal { "B1" } else { "A2" }), Value::number(3.0));
        assert_eq!(value(&s, if horizontal { "D1" } else { "A4" }), Value::number(6.0));
    }
}

#[test]
fn cumulative_line_overflow_and_later_cell_overflow_leave_the_workbook_untouched() {
    let mut s = book();
    set(&mut s, "A1048575", "keep");
    run(&mut s, "selection.set", json!({"range":"A1,A3"}));
    let before = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    assert!(s.execute("home.insertRows", json!({})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
    assert_eq!(s.doc().unwrap().undo.len(), undo);
    set(&mut s, "C1048576", "edge");
    run(&mut s, "selection.set", json!({"range":"A2,C1048576"}));
    let before = s.doc().unwrap().wb.clone();
    assert!(s.execute("home.insertCells", json!({"shift":"down"})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
}

#[test]
fn replace_all_respects_the_union_but_single_cell_selection_searches_the_sheet() {
    let mut s = book();
    for at in ["A1", "A2", "B1", "C1", "C2", "D8"] {
        set(&mut s, at, "old");
    }
    run(&mut s, "selection.set", json!({"range":"A1:A2,C1:C2"}));
    let undo = s.doc().unwrap().undo.len();
    assert_eq!(run(&mut s, "edit.replace", json!({"what":"old","with":"new","all":true}))["replaced"], 4);
    assert_eq!(value(&s, "B1"), Value::text("old"));
    assert_eq!(value(&s, "D8"), Value::text("old"));
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "selection.set", json!({"cell":"A1"}));
    assert_eq!(run(&mut s, "edit.replace", json!({"what":"old","with":"new","all":true}))["replaced"], 6);
}

#[test]
fn replace_advances_through_the_selected_areas_without_collapsing_them() {
    let mut s = book();
    for at in ["A1", "A2", "B1", "C1", "C2"] {
        set(&mut s, at, "old");
    }
    run(&mut s, "selection.set", json!({"range":"C1:C2,A1:A2","active":"A1"}));
    run(&mut s, "edit.replace", json!({"what":"old","with":"new","all":false}));
    assert_eq!(value(&s, "A1"), Value::text("new"));
    assert_eq!(s.doc().unwrap().selection.active, CellRef::parse("C1").unwrap());
    assert_eq!(s.doc().unwrap().selection.ranges.len(), 2);
    assert_eq!(value(&s, "B1"), Value::text("old"));
}
