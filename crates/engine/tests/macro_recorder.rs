//! Recorder output compiles, preserves editing context, and publishes both stores atomically.
use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
};
use serde_json::{Value as Json, json};

fn workbook() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s
}

fn run(s: &mut Session, command: &str, params: Json) -> Json {
    s.execute(command, params).unwrap()
}

fn source(s: &mut Session, module: &str) -> String {
    run(s, "automate.vbaModules", json!({}))["modules"].as_array().unwrap().iter().find(|item| item["name"] == module).unwrap()["source"]
        .as_str()
        .unwrap()
        .to_string()
}

fn execute_macro(s: &mut Session, result: &Json) {
    run(s, "automate.startVba", json!({"module":result["module"],"procedure":result["procedure"]}));
    for _ in 0..100 {
        let status = run(s, "automate.pollVba", json!({"budget":1000}));
        if status["state"] != "running" {
            assert_eq!(status["state"], "completed", "{status}");
            return;
        }
    }
    panic!("recorded macro did not finish");
}

#[test]
fn recorded_values_formulas_and_toggled_formats_replay_in_the_original_sheet_context() {
    let mut s = workbook();
    run(&mut s, "sheet.rename", json!({"name":"売上 \"2026\""}));
    run(&mut s, "selection.set", json!({"cell":"B2"}));
    run(&mut s, "automate.recordActions", json!({}));
    run(&mut s, "cell.set", json!({"input":"'001\n\"日本語\""}));
    run(&mut s, "cell.set", json!({"cell":"C2","input":"=2+3"}));
    run(&mut s, "home.bold", json!({"range":"B2:C2"}));
    let result = run(&mut s, "automate.recordActions", json!({"stop":true,"name":"Revenue"}));
    assert_eq!(result["module"], "Module1");
    assert_eq!(result["steps"], 3);
    assert_eq!(result["vbaComplete"], true);
    let code = source(&mut s, "Module1");
    assert!(code.contains("vbLf") && code.contains("\"\"日本語\"\""), "{code}");
    let saved = run(&mut s, "file.saveBytes", json!({"format":"xlsx"}));
    assert!(saved["base64"].as_str().is_some());
    run(&mut s, "edit.clearContents", json!({"range":"B2:C2"}));
    run(&mut s, "home.bold", json!({"range":"B2:C2","on":false}));
    run(&mut s, "home.insertSheet", json!({"name":"Other"}));
    execute_macro(&mut s, &result);
    let d = s.doc().unwrap();
    let sh = d.wb.sheet(0).unwrap();
    assert_eq!(sh.value(CellRef::parse("B2").unwrap()), Value::text("001\n\"日本語\""));
    assert_eq!(sh.value(CellRef::parse("C2").unwrap()), Value::number(5.0));
    assert!(d.wb.styles.get(sh.style_id(CellRef::parse("B2").unwrap())).font.bold);
    assert!(d.wb.sheet(1).unwrap().cell(CellRef::parse("B2").unwrap()).is_none());
}

#[test]
fn repeated_recordings_append_unique_procedures_and_keep_unsupported_operations_explicit() {
    let mut s = workbook();
    for _ in 0..2 {
        run(&mut s, "automate.recordActions", json!({}));
        run(&mut s, "home.wrapText", json!({"range":"A1"}));
        let result = run(&mut s, "automate.recordActions", json!({"stop":true,"name":"Macro1"}));
        assert_eq!(result["vbaComplete"], false);
        assert_eq!(result["unsupported"], json!(["home.wrapText"]));
    }
    let modules = run(&mut s, "automate.vbaModules", json!({}));
    let module = &modules["modules"][0];
    assert_eq!(module["procedures"], json!(["Macro1", "Macro1_2"]));
    assert!(module.get("compileError").is_none_or(Json::is_null));
    assert!(module["source"].as_str().unwrap().contains("' Not recorded in VBA: home.wrapText"));
}

#[test]
fn stopping_is_one_undo_step_and_a_metadata_failure_keeps_the_recording() {
    let mut s = workbook();
    run(&mut s, "automate.recordActions", json!({}));
    run(&mut s, "cell.set", json!({"cell":"A1","input":"kept"}));
    let before = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    run(&mut s, "automate.recordActions", json!({"stop":true}));
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(*s.doc().unwrap().wb, *before);
    std::sync::Arc::make_mut(&mut s.active_mut().unwrap().wb).names.push(gridcraft_engine::model::DefinedName {
        name: "_gc_vba_sources_v1".into(),
        scope: None,
        formula: "invalid metadata".into(),
        comment: String::new(),
        hidden: true,
    });
    run(&mut s, "automate.recordActions", json!({}));
    run(&mut s, "cell.set", json!({"cell":"B1","input":"also kept"}));
    let before = s.doc().unwrap().wb.clone();
    let selection = s.doc().unwrap().selection.clone();
    assert!(s.execute("automate.recordActions", json!({"stop":true})).is_err());
    assert!(s.recording.is_some());
    assert_eq!(*s.doc().unwrap().wb, *before);
    assert_eq!(s.doc().unwrap().selection, selection);
    run(&mut s, "automate.recordActions", json!({"cancel":true}));
    assert!(s.recording.is_none());
}

#[test]
fn failed_commands_queries_and_nested_commands_do_not_duplicate_recorded_steps() {
    let mut s = workbook();
    run(&mut s, "automate.recordActions", json!({}));
    assert!(s.execute("cell.set", json!({"cell":"bad","input":"x"})).is_err());
    run(&mut s, "cell.get", json!({"cell":"A1"}));
    run(&mut s, "cell.set", json!({"cell":"A1","input":"7"}));
    run(&mut s, "table.removeDuplicates", json!({"range":"A1:A2","header":false}));
    let result = run(&mut s, "automate.recordActions", json!({"stop":true}));
    assert_eq!(result["steps"], 2);
    assert_eq!(s.journal.iter().filter(|(id, _)| id == "cell.set").count(), 1);
    assert_eq!(s.journal.iter().filter(|(id, _)| id == "table.removeDuplicates").count(), 1);
    assert_eq!(s.journal.iter().filter(|(id, _)| id == "data.removeDuplicates").count(), 0, "nested implementation calls are not recorded twice");
}

#[test]
fn recorded_row_and_column_edits_replay_every_span_and_update_formulas_once() {
    fn seed(s: &mut Session) {
        run(s, "range.setValues", json!({"range":"A1","values":[[1,2,3,4],[5,6,7,8],[9,10,11,12],[13,14,15,16],[17,18,19,20],[21,22,23,24]]}));
        run(s, "cell.set", json!({"cell":"F1","input":"=SUM(A1:D6)"}));
    }
    let mut recorded = workbook();
    seed(&mut recorded);
    run(&mut recorded, "automate.recordActions", json!({}));
    run(&mut recorded, "home.insertRows", json!({"rows":"2:2,5:5"}));
    run(&mut recorded, "home.deleteRows", json!({"rows":"3:3"}));
    run(&mut recorded, "home.insertColumns", json!({"cols":"B:B,D:D"}));
    run(&mut recorded, "home.deleteColumns", json!({"cols":"C:C","count":1}));
    let result = run(&mut recorded, "automate.recordActions", json!({"stop":true,"name":"Restructure"}));
    assert_eq!(result["vbaComplete"], true, "{result}");
    let code = source(&mut recorded, result["module"].as_str().unwrap());
    assert!(code.contains("EntireRow.Insert") && code.contains("EntireColumn.Delete"), "{code}");
    let mut replayed = workbook();
    seed(&mut replayed);
    run(&mut replayed, "automate.saveVbaModule", json!({"name":result["module"],"source":code}));
    let undo = replayed.doc().unwrap().undo.len();
    execute_macro(&mut replayed, &result);
    assert_eq!(replayed.doc().unwrap().undo.len(), undo + 1, "the whole macro is one undo step");
    assert_eq!(replayed.doc().unwrap().wb.active().unwrap().cells, recorded.doc().unwrap().wb.active().unwrap().cells);
}

#[test]
fn reserved_macro_names_are_replaced_by_a_valid_unique_procedure_name() {
    let mut s = workbook();
    run(&mut s, "automate.recordActions", json!({}));
    let result = run(&mut s, "automate.recordActions", json!({"stop":true,"name":"If"}));
    assert_eq!(result["procedure"], "Macro1");
    execute_macro(&mut s, &result);
}
