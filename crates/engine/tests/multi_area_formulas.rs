use gridcraft_engine::{Session, core::CellRef};
use serde_json::json;

fn value(s: &Session, a: &str) -> String {
    s.active().unwrap().wb.active().unwrap().value(CellRef::parse(a).unwrap()).to_text().unwrap()
}

#[test]
fn autosum_each_area_is_one_history_step_and_undo_restores_all() {
    let mut s = Session::new();
    s.new_workbook();
    for (a, n) in [("A1", 2), ("A2", 3), ("C1", 7), ("C2", 11)] {
        s.run("cell.set", json!({"cell":a,"value":n})).unwrap();
    }
    s.run("selection.set", json!({"range":"A1:A3,C1:C3"})).unwrap();
    s.run("formulas.autoSum", json!({})).unwrap();
    assert_eq!(value(&s, "A3"), "5");
    assert_eq!(value(&s, "C3"), "18");
    s.run("edit.undo", json!({})).unwrap();
    assert_eq!(value(&s, "A3"), "");
    assert_eq!(value(&s, "C3"), "");
    assert_eq!(value(&s, "C2"), "11");
}

#[test]
fn autosum_single_cell_areas_guess_independently_and_later_error_is_atomic() {
    let mut s = Session::new();
    s.new_workbook();
    s.run("cell.set", json!({"cell":"A1","value":9})).unwrap();
    s.run("cell.set", json!({"cell":"C1","value":4})).unwrap();
    s.run("selection.set", json!({"range":"A2,C2"})).unwrap();
    s.run("formulas.autoSum", json!({})).unwrap();
    assert_eq!(value(&s, "A2"), "9");
    assert_eq!(value(&s, "C2"), "4");
    s.run("selection.set", json!({"range":"A3,C1048575:C1048576"})).unwrap();
    s.run("cell.set", json!({"cell":"C1048576","value":3})).unwrap();
    assert!(s.run("formulas.autoSum", json!({})).is_err());
    assert_eq!(value(&s, "A3"), "");
}

#[test]
fn default_name_contains_absolute_union_and_evaluates_every_area() {
    let mut s = Session::new();
    s.new_workbook();
    for (a, n) in [("A1", 2), ("C1", 5), ("C2", 7)] {
        s.run("cell.set", json!({"cell":a,"value":n})).unwrap();
    }
    s.run("selection.set", json!({"range":"A1,C1:C2"})).unwrap();
    s.run("formulas.defineName", json!({"name":"Areas"})).unwrap();
    assert_eq!(s.active().unwrap().wb.names[0].formula, "Sheet1!$A$1,Sheet1!$C$1:$C$2");
    s.run("cell.set", json!({"cell":"E1","input":"=SUM(Areas)"})).unwrap();
    assert_eq!(value(&s, "E1"), "14");
}
