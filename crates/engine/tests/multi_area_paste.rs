//! Pasting into discontiguous selections is one atomic edit across every destination.

use gridcraft_core::{CellRef, RangeRef, Value};
use gridcraft_engine::Session;
use serde_json::{Value as Json, json};

fn session() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s
}

fn run(s: &mut Session, command: &str, params: Json) -> Json {
    s.execute(command, params).unwrap()
}

fn set(s: &mut Session, cell: &str, input: &str) {
    run(s, "cell.set", json!({"cell": cell, "input": input}));
}

fn value(s: &Session, cell: &str) -> Value {
    s.doc().unwrap().wb.active().unwrap().value(CellRef::parse(cell).unwrap())
}

fn formula(s: &Session, cell: &str) -> String {
    s.doc().unwrap().wb.active().unwrap().cell(CellRef::parse(cell).unwrap()).unwrap().formula.as_ref().unwrap().text.clone()
}

#[test]
fn copied_cell_reaches_every_area_and_undo_redo_stays_one_step() {
    let mut s = session();
    set(&mut s, "A1", "42");
    set(&mut s, "D4", "untouched");
    run(&mut s, "home.bold", json!({"range": "A1", "on": true}));
    run(&mut s, "edit.copy", json!({"range": "A1"}));
    run(&mut s, "selection.set", json!({"range": "C3,E5,H2"}));
    let before = s.doc().unwrap().wb.clone();
    let selection = s.doc().unwrap().selection.clone();
    let undo = s.doc().unwrap().undo.len();
    let result = run(&mut s, "edit.paste", json!({}));
    for cell in ["C3", "E5", "H2"] {
        assert_eq!(value(&s, cell), Value::number(42.0), "{cell}");
        let wb = &s.doc().unwrap().wb;
        assert!(wb.styles.get(wb.active().unwrap().style_id(CellRef::parse(cell).unwrap())).font.bold);
    }
    assert_eq!(value(&s, "D4"), Value::text("untouched"));
    assert_eq!(result["pasted"], "C3,E5,H2");
    assert_eq!(s.doc().unwrap().selection, selection);
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    let pasted = s.doc().unwrap().wb.clone();
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc().unwrap().wb, before);
    assert_eq!(s.doc().unwrap().selection, selection);
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(s.doc().unwrap().wb, pasted);
    assert_eq!(s.doc().unwrap().selection, selection);
    assert!(s.clipboard.is_some());
}

#[test]
fn external_text_and_numeric_html_fill_each_selected_area() {
    let mut s = session();
    run(&mut s, "selection.set", json!({"range": "C1:C2,E4:F5", "active": "F5"}));
    let selection = s.doc().unwrap().selection.clone();
    let result = run(&mut s, "edit.paste", json!({"text": "Rp10,000.00", "html": "<table><tr><td x:num=\"10000\">Rp10,000.00</td></tr></table>"}));
    assert_eq!(result["rows"], 1);
    assert_eq!(result["cols"], 1);
    for cell in ["C1", "C2", "E4", "F4", "E5", "F5"] {
        assert_eq!(value(&s, cell), Value::number(10_000.0), "{cell}");
    }
    assert_eq!(value(&s, "D1"), Value::Empty);
    assert_eq!(s.doc().unwrap().selection, selection);
}

#[test]
fn copied_blocks_tile_and_adjust_formulas_independently_in_each_area() {
    let mut s = session();
    run(&mut s, "range.setValues", json!({"range": "A1", "values": [[3, "=A1*2"], [4, "=A2*2"]]}));
    run(&mut s, "edit.copy", json!({"range": "A1:B2"}));
    run(&mut s, "selection.set", json!({"range": "D4:G7,J10:K11", "active": "K11"}));
    let selection = s.doc().unwrap().selection.clone();
    run(&mut s, "edit.paste", json!({}));
    for (cell, expected, number) in [("E4", "D4*2", 6.0), ("G5", "F5*2", 8.0), ("E6", "D6*2", 6.0), ("K11", "J11*2", 8.0)] {
        assert_eq!(formula(&s, cell), expected);
        assert_eq!(value(&s, cell), Value::number(number));
    }
    assert_eq!(s.doc().unwrap().selection, selection);
    assert_eq!(value(&s, "H8"), Value::Empty);
}

#[test]
fn ragged_external_blocks_keep_missing_fields_at_every_destination() {
    let mut s = session();
    set(&mut s, "E5", "keep one");
    set(&mut s, "I9", "keep two");
    run(&mut s, "selection.set", json!({"range": "D4,H8"}));
    run(&mut s, "edit.paste", json!({"text": "1\t2\n3"}));
    for (cell, number) in [("D4", 1.0), ("E4", 2.0), ("D5", 3.0), ("H8", 1.0), ("I8", 2.0), ("H9", 3.0)] {
        assert_eq!(value(&s, cell), Value::number(number), "{cell}");
    }
    assert_eq!(value(&s, "E5"), Value::text("keep one"));
    assert_eq!(value(&s, "I9"), Value::text("keep two"));
    assert_eq!(s.doc().unwrap().selection.a1(), "D4:E5,H8:I9");
    assert_eq!(s.doc().unwrap().selection.active.a1(), "H8");
}

#[test]
fn paste_special_operations_transpose_and_skip_blanks_apply_to_all_areas() {
    let mut s = session();
    set(&mut s, "A1", "2");
    for cell in ["C1", "C2", "E4"] {
        set(&mut s, cell, "10");
    }
    run(&mut s, "edit.copy", json!({"range": "A1"}));
    run(&mut s, "selection.set", json!({"range": "C1:C2,E4"}));
    run(&mut s, "edit.pasteSpecial", json!({"what": "values", "operation": "multiply"}));
    for cell in ["C1", "C2", "E4"] {
        assert_eq!(value(&s, cell), Value::number(20.0));
    }
    run(&mut s, "range.setValues", json!({"range": "A1", "values": [[null], [5]]}));
    run(&mut s, "edit.copy", json!({"range": "A1:A2"}));
    run(&mut s, "selection.set", json!({"range": "C1:C2,E4:E5"}));
    run(&mut s, "edit.pasteSpecial", json!({"what": "values", "skipBlanks": true}));
    assert_eq!(value(&s, "C1"), Value::number(20.0));
    assert_eq!(value(&s, "E4"), Value::number(20.0));
    assert_eq!(value(&s, "C2"), Value::number(5.0));
    assert_eq!(value(&s, "E5"), Value::number(5.0));
    run(&mut s, "selection.set", json!({"range": "G1:J1,L3:M3"}));
    run(&mut s, "edit.pasteSpecial", json!({"what": "values", "transpose": true}));
    for cell in ["G1", "I1", "L3"] {
        assert_eq!(value(&s, cell), Value::Empty);
    }
    for cell in ["H1", "J1", "M3"] {
        assert_eq!(value(&s, cell), Value::number(5.0));
    }
    assert_eq!(s.doc().unwrap().selection.a1(), "G1:J1,L3:M3");
}

#[test]
fn explicit_destination_overrides_multi_selection_for_internal_and_external_paste() {
    for external in [false, true] {
        let mut s = session();
        set(&mut s, "A1", "7");
        run(&mut s, "edit.copy", json!({"range": "A1"}));
        run(&mut s, "selection.set", json!({"range": "C1:C3,E1:E3"}));
        let params = if external { json!({"at": "J2", "text": "9"}) } else { json!({"at": "J2"}) };
        run(&mut s, "edit.paste", params);
        assert_eq!(value(&s, "J2"), Value::number(if external { 9.0 } else { 7.0 }));
        assert_eq!(value(&s, "J3"), Value::Empty);
        assert_eq!(value(&s, "C1"), Value::Empty);
        assert_eq!(value(&s, "E1"), Value::Empty);
        assert_eq!(s.doc().unwrap().selection.a1(), "J2");
    }
}

#[test]
fn cut_to_multiple_destinations_fails_without_changing_source_selection_or_clipboard() {
    let mut s = session();
    set(&mut s, "A1", "move me");
    run(&mut s, "edit.cut", json!({"range": "A1"}));
    run(&mut s, "selection.set", json!({"range": "C1,E1"}));
    let clipboard = s.clipboard.clone().unwrap();
    let before = s.doc().unwrap().wb.clone();
    let selection = s.doc().unwrap().selection.clone();
    let undo = s.doc().unwrap().undo.len();
    assert!(s.execute("edit.paste", json!({})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
    assert_eq!(s.doc().unwrap().selection, selection);
    assert_eq!(s.doc().unwrap().undo.len(), undo);
    assert!(s.clipboard.as_ref().is_some_and(|clip| clip.cut && clip.text == clipboard.text && clip.range == clipboard.range));
    run(&mut s, "edit.paste", json!({"at": "J1"}));
    assert_eq!(value(&s, "A1"), Value::Empty);
    assert_eq!(value(&s, "J1"), Value::text("move me"));
    assert!(s.clipboard.is_none());
}

#[test]
fn a_locked_later_destination_rolls_back_all_internal_and_external_changes() {
    for external in [false, true] {
        let mut s = session();
        set(&mut s, "A1", "new");
        set(&mut s, "C1", "first");
        set(&mut s, "E1", "last");
        run(&mut s, "home.formatCells", json!({"range": "C1", "style": {"protection": {"locked": false}}}));
        run(&mut s, "review.protectSheet", json!({}));
        run(&mut s, "edit.copy", json!({"range": "A1"}));
        run(&mut s, "selection.set", json!({"range": "C1,E1"}));
        let before = s.doc().unwrap().wb.clone();
        let selection = s.doc().unwrap().selection.clone();
        let undo = s.doc().unwrap().undo.len();
        let params = if external { json!({"text": "external"}) } else { json!({}) };
        assert!(s.execute("edit.paste", params).is_err());
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().selection, selection);
        assert_eq!(s.doc().unwrap().undo.len(), undo);
        assert!(s.clipboard.is_some());
    }
}

#[test]
fn overlapping_expanded_destinations_are_rejected_before_arithmetic_or_writes() {
    let mut s = session();
    run(&mut s, "range.setValues", json!({"range": "A1", "values": [[2, 3]]}));
    run(&mut s, "range.setValues", json!({"range": "C1", "values": [[10, 10, 10]]}));
    run(&mut s, "edit.copy", json!({"range": "A1:B1"}));
    run(&mut s, "selection.set", json!({"range": "C1,D1"}));
    let before = s.doc().unwrap().wb.clone();
    let selection = s.doc().unwrap().selection.clone();
    for (command, params) in [("edit.pasteSpecial", json!({"what": "values", "operation": "multiply"})), ("edit.paste", json!({"text": "4\t5"}))] {
        assert!(s.execute(command, params).unwrap_err().to_string().contains("overlap"));
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().selection, selection);
    }
}

#[test]
fn a_later_out_of_grid_block_rejects_the_entire_multi_area_paste() {
    for external in [false, true] {
        let mut s = session();
        run(&mut s, "range.setValues", json!({"range": "A1", "values": [[1, 2]]}));
        run(&mut s, "edit.copy", json!({"range": "A1:B1"}));
        run(&mut s, "selection.set", json!({"range": "C1,XFD5"}));
        let before = s.doc().unwrap().wb.clone();
        let selection = s.doc().unwrap().selection.clone();
        let params = if external { json!({"text": "3\t4"}) } else { json!({}) };
        assert!(s.execute("edit.paste", params).is_err());
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().selection, selection);
    }
}

#[test]
fn aggregate_paste_work_is_limited_before_any_cell_is_created() {
    let mut s = session();
    set(&mut s, "A1", "2");
    run(&mut s, "edit.copy", json!({"range": "A1"}));
    run(&mut s, "selection.set", json!({"range": "C1:CD30000,DA1:GB30000"}));
    let before = s.doc().unwrap().wb.clone();
    let selection = s.doc().unwrap().selection.clone();
    for params in [json!({}), json!({"text": "external"})] {
        assert!(s.execute("edit.paste", params).unwrap_err().to_string().contains("too large"));
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().selection, selection);
    }
}

#[test]
fn merged_source_is_copied_to_every_tile_in_every_destination() {
    let mut s = session();
    set(&mut s, "A1", "merged");
    run(&mut s, "home.mergeCells", json!({"range": "A1:B1"}));
    run(&mut s, "edit.copy", json!({"range": "A1:B1"}));
    run(&mut s, "selection.set", json!({"range": "D4:G4,J6:K6"}));
    run(&mut s, "edit.paste", json!({}));
    let sheet = s.doc().unwrap().wb.active().unwrap();
    for range in ["D4:E4", "F4:G4", "J6:K6"] {
        assert!(sheet.merges.contains(&RangeRef::parse(range).unwrap()), "{range}");
    }
    for cell in ["D4", "F4", "J6"] {
        assert_eq!(value(&s, cell), Value::text("merged"));
    }
}

#[test]
fn too_many_areas_and_repeated_external_text_fail_before_allocating_cells() {
    for excessive_areas in [false, true] {
        let mut s = session();
        let range = if excessive_areas {
            (0..4097).map(|row| format!("C{}", row * 2 + 1)).collect::<Vec<_>>().join(",")
        } else {
            "C1:C20000,E1:E20000".into()
        };
        run(&mut s, "selection.set", json!({"range": range}));
        let before = s.doc().unwrap().wb.clone();
        let selection = s.doc().unwrap().selection.clone();
        let undo = s.doc().unwrap().undo.len();
        let error = s.execute("edit.paste", json!({"text": "x".repeat(2048)})).unwrap_err().to_string();
        assert!(error.contains(if excessive_areas { "too large" } else { "repeated clipboard data" }), "{error}");
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().selection, selection);
        assert_eq!(s.doc().unwrap().undo.len(), undo);
    }
}

#[test]
fn quoted_separators_are_content_rather_than_extra_tsv_columns() {
    let mut s = session();
    let field = format!("start{}end", "\t".repeat(17_000));
    let text = format!("\"{field}\"\tplain");
    run(&mut s, "selection.set", json!({"range": "C1,E3"}));
    run(&mut s, "edit.paste", json!({"text": text}));
    for cell in ["C1", "E3"] {
        assert_eq!(value(&s, cell), Value::text(field.as_str()));
    }
    for cell in ["D1", "F3"] {
        assert_eq!(value(&s, cell), Value::text("plain"));
    }
    assert_eq!(s.doc().unwrap().selection.a1(), "C1:D1,E3:F3");
}
