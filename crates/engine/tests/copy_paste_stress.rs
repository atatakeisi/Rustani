//! Repeated copy/paste, resizing, undo and XLSX round-trip regression scenarios.

use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
    io,
};
use serde_json::{Value as Json, json};

fn run(s: &mut Session, id: &str, p: Json) -> Json {
    s.execute(id, p).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn value(s: &Session, cell: &str) -> Value {
    s.doc().unwrap().wb.active().unwrap().value(CellRef::parse(cell).unwrap())
}

#[test]
fn repeated_multi_area_pastes_and_format_changes_roundtrip() {
    let mut s = Session::new();
    s.new_workbook();
    run(&mut s, "range.setValues", json!({"range":"A1", "values":[[42,"=A1*2"]]}));
    run(&mut s, "home.bold", json!({"range":"A1:B1", "on":true}));
    run(&mut s, "home.fontSize", json!({"range":"A1:B1", "size":24}));
    run(&mut s, "edit.copy", json!({"range":"A1:B1"}));

    for i in 0..120 {
        let row = i + 3;
        // A formatting edit intentionally ends copy mode; start a fresh copy for each cycle.
        if s.clipboard.is_none() {
            run(&mut s, "edit.copy", json!({"range":"A1:B1"}));
        }
        let selection = format!("D{row},H{row}");
        run(&mut s, "selection.set", json!({"range":selection}));
        run(&mut s, "view.zoom", json!({"percent": if i % 2 == 0 {10} else {400}}));
        let before = s.doc().unwrap().wb.clone();
        let params = match i % 3 {
            0 => json!({}),
            1 => json!({"text":"10\t20"}),
            _ => json!({"text":"¥1,200", "html":"<table><tr><td x:num=\"1200\">¥1,200</td></tr></table>"}),
        };
        run(&mut s, "edit.paste", params);
        let pasted = s.doc().unwrap().wb.clone();
        for col in ["D", "H"] {
            let expected = match i % 3 {
                0 => 42.0,
                1 => 10.0,
                _ => 1200.0,
            };
            assert_eq!(value(&s, &format!("{col}{row}")), Value::number(expected), "iteration {i}");
        }
        if i % 10 == 0 {
            run(&mut s, "edit.undo", json!({}));
            assert_eq!(s.doc().unwrap().wb, before, "undo at {i}");
            run(&mut s, "edit.redo", json!({}));
            assert_eq!(s.doc().unwrap().wb, pasted, "redo at {i}");
        }
        run(&mut s, "selection.set", json!({"cell":format!("D{row}")}));
        run(&mut s, "home.increaseFontSize", json!({}));
        if i % 20 == 0 {
            run(&mut s, "home.autofitColumnWidth", json!({"cols":"D:I"}));
            run(&mut s, "home.autofitRowHeight", json!({"rows":format!("{row}:{row}")}));
        }
    }

    let original = &s.doc().unwrap().wb;
    let bytes = io::save_bytes(original, "stress.xlsx").unwrap();
    let (reopened, warnings) = io::open_bytes("stress.xlsx", &bytes).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let (a, b) = (original.active().unwrap(), reopened.active().unwrap());
    for row in 3..123 {
        for col in ["D", "E", "H", "I"] {
            let at = CellRef::parse(&format!("{col}{row}")).unwrap();
            assert_eq!(a.value(at), b.value(at), "value at {}", at.a1());
            let fa = a.cell(at).and_then(|c| c.formula.as_ref()).map(|f| &f.text);
            let fb = b.cell(at).and_then(|c| c.formula.as_ref()).map(|f| &f.text);
            assert_eq!(fa, fb, "formula at {}", at.a1());
            let sa = original.styles.get(a.style_id(at));
            let sb = reopened.styles.get(b.style_id(at));
            assert_eq!((sa.font.size, sa.font.bold), (sb.font.size, sb.font.bold), "font at {}", at.a1());
        }
    }
}

#[test]
fn font_and_paste_boundaries_reject_atomically() {
    let mut s = Session::new();
    s.new_workbook();
    run(&mut s, "cell.set", json!({"cell":"A1", "input":"9"}));
    for size in [0.0, -1.0, 409.5, 1e100] {
        let before = s.doc().unwrap().wb.clone();
        assert!(s.execute("home.fontSize", json!({"range":"A1", "size":size})).is_err());
        assert_eq!(s.doc().unwrap().wb, before);
    }
    run(&mut s, "home.fontSize", json!({"range":"A1", "size":409}));
    for _ in 0..120 {
        assert_eq!(run(&mut s, "home.increaseFontSize", json!({"range":"A1"}))["size"], 409.0);
    }
    run(&mut s, "range.setValues", json!({"range":"A2", "values":[[1,2]]}));
    run(&mut s, "edit.copy", json!({"range":"A2:B2"}));
    run(&mut s, "selection.set", json!({"range":"C3,XFD1048576"}));
    let before = s.doc().unwrap().wb.clone();
    for params in [json!({}), json!({"text":"1\t3"}), json!({"text":"1\t3", "html":"<table><tr><td>1</td><td>3</td></tr></table>"})] {
        assert!(s.execute("edit.paste", params).is_err());
        assert_eq!(s.doc().unwrap().wb, before);
    }
}

#[test]
fn zoom_after_copy_keeps_the_clipboard_for_repeated_paste() {
    let mut s = Session::new();
    s.new_workbook();
    run(&mut s, "cell.set", json!({"cell":"A1", "input":"123"}));
    run(&mut s, "edit.copy", json!({"range":"A1"}));
    for (id, params, target) in [
        ("view.zoom", json!({"percent":400}), "C3"),
        ("view.zoom100", json!({}), "E5"),
        ("view.zoomToSelection", json!({"viewWidth":800,"viewHeight":600}), "G7"),
    ] {
        run(&mut s, "selection.set", json!({"cell":target}));
        run(&mut s, id, params);
        assert!(s.clipboard.is_some(), "{id} cancelled copy mode");
        run(&mut s, "edit.paste", json!({}));
        assert_eq!(value(&s, target), Value::number(123.0));
    }
}

#[test]
fn merged_and_filtered_ranges_survive_repeated_pastes_and_roundtrip() {
    let mut s = Session::new();
    s.new_workbook();
    run(&mut s, "range.setValues", json!({"range":"A1", "values":[["Kind","Amount"],["keep",7],["hide",8],["keep",9]]}));
    run(&mut s, "data.filter", json!({"range":"A1:B4"}));
    run(&mut s, "data.filterBy", json!({"column":"A", "values":["keep"]}));
    run(&mut s, "cell.set", json!({"cell":"D1", "input":"22"}));
    run(&mut s, "home.mergeCells", json!({"range":"D1:E1"}));
    run(&mut s, "edit.copy", json!({"range":"D1:E1"}));
    for row in 2..22 {
        run(&mut s, "selection.set", json!({"range":format!("G{row},J{row}")}));
        run(&mut s, "edit.paste", json!({}));
        for col in ["G", "J"] {
            assert_eq!(value(&s, &format!("{col}{row}")), Value::number(22.0));
        }
    }
    let bytes = io::save_bytes(&s.doc().unwrap().wb, "filtered.xlsx").unwrap();
    let (opened, warnings) = io::open_bytes("filtered.xlsx", &bytes).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let sh = opened.active().unwrap();
    assert!(sh.autofilter.is_some());
    assert!(sh.merges.contains(&gridcraft_engine::core::RangeRef::parse("G2:H2").unwrap()));
    assert_eq!(sh.value(CellRef::parse("J21").unwrap()), Value::number(22.0));
}

#[test]
fn recording_reports_unsupported_paste_and_runs_the_supported_vba() {
    let mut s = Session::new();
    s.new_workbook();
    run(&mut s, "automate.recordActions", json!({}));
    run(&mut s, "cell.set", json!({"cell":"A1", "input":"42"}));
    run(&mut s, "home.fontSize", json!({"range":"A1", "size":24}));
    for row in 1..=100 {
        run(&mut s, "edit.copy", json!({"range":"A1"}));
        run(&mut s, "selection.set", json!({"range":format!("D{row},F{row}")}));
        run(&mut s, "view.zoom", json!({"percent":if row % 2 == 0 {400} else {10}}));
        run(&mut s, "edit.paste", json!({}));
        run(&mut s, "home.increaseFontSize", json!({"range":"A1"}));
    }
    let result = run(&mut s, "automate.recordActions", json!({"stop":true,"name":"Stress"}));
    assert_eq!(result["vbaComplete"], false, "paste is explicitly reported as unsupported");
    assert!(result["unsupported"].as_array().unwrap().iter().any(|id| id == "edit.paste"), "{result}");
    run(&mut s, "automate.startVba", json!({"module":result["module"],"procedure":result["procedure"]}));
    for _ in 0..100 {
        let status = run(&mut s, "automate.pollVba", json!({"budget":1000}));
        if status["state"] != "running" {
            assert_eq!(status["state"], "completed", "{status}");
            run(&mut s, "edit.undo", json!({}));
            run(&mut s, "edit.redo", json!({}));
            assert_eq!(value(&s, "A1"), Value::number(42.0));
            return;
        }
    }
    panic!("recorded Mini-VBA did not finish");
}
