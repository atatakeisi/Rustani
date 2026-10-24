//! Exercise the real command registry, staged workbook, language and Excel host together.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{ChartKind, DefinedName, Workbook};
use serde_json::{Value as Json, json};

use crate::{DocState, Session};

fn session() -> Session {
    let mut session = Session::new();
    session.new_workbook();
    session
}

fn cell(session: &Session, sheet: usize, address: &str) -> Value {
    session.doc().unwrap().wb.sheet(sheet).unwrap().value(CellRef::parse(address).unwrap())
}

fn start(session: &mut Session, source: &str) -> Json {
    session.execute("automate.startVba", json!({"module":"Example","procedure":"Main","source":source})).unwrap()
}

fn finish_with_budget(session: &mut Session, budget: usize) -> Json {
    for _ in 0..2000 {
        let status = session.execute("automate.pollVba", json!({"budget":budget})).unwrap();
        if status["state"] != "running" {
            return status;
        }
    }
    panic!("macro remained running after the bounded test polling limit");
}

fn run(session: &mut Session, source: &str) -> Json {
    let status = start(session, source);
    if status["state"] != "running" { status } else { finish_with_budget(session, 1000) }
}

fn completed(session: &mut Session, source: &str) -> Json {
    let result = run(session, source);
    assert_eq!(result["state"], "completed", "{result}");
    result
}

#[test]
fn loops_cells_and_worksheet_functions_share_calculated_workbook_state() {
    let mut session = session();
    completed(
        &mut session,
        r#"
Option Explicit
Sub Main()
    Dim i As Long
    Dim total As Double
    For i = 1 To 5
        Cells(i, 1).Value = i * 2
        total = total + Cells(i, 1).Value
    Next i
    Range("B1").Value = total
    Range("B2").Value = WorksheetFunction.Sum(Range("A1:A5"))
    Range("B3").Value = Application.WorksheetFunction.Sum(1, 2, 3)
End Sub
"#,
    );
    for row in 1..=5 {
        assert_eq!(cell(&session, 0, &format!("A{row}")), Value::number(f64::from(row * 2)));
    }
    assert_eq!(cell(&session, 0, "B1"), Value::number(30.0));
    assert_eq!(cell(&session, 0, "B2"), Value::number(30.0));
    assert_eq!(cell(&session, 0, "B3"), Value::number(6.0));
}

#[test]
fn formulas_text_values_font_and_number_formats_change_real_cells() {
    let mut session = session();
    completed(
        &mut session,
        r#"
Sub Main()
    Range("A1").Value = 12.5
    Range("A2").Value = "0012"
    Range("A3").Formula = "=A1*2"
    Range("A1:A3").Font.Bold = True
    Range("A1:A3").NumberFormat = "0.00"
    Range("B1").Value = Range("A3").Value
    Range("B2").Value = Range("A3").Formula
End Sub
"#,
    );
    assert_eq!(cell(&session, 0, "A1"), Value::number(12.5));
    assert_eq!(cell(&session, 0, "A2"), Value::text("0012"));
    assert_eq!(cell(&session, 0, "A3"), Value::number(25.0));
    assert_eq!(cell(&session, 0, "B1"), Value::number(25.0));
    assert_eq!(cell(&session, 0, "B2"), Value::text("=A1*2"));
    let document = session.doc().unwrap();
    let sheet = document.wb.sheet(0).unwrap();
    for address in ["A1", "A2", "A3"] {
        let style = document.wb.styles.get(sheet.style_id(CellRef::parse(address).unwrap()));
        assert!(style.font.bold, "{address}");
        assert_eq!(style.num_fmt.0.as_ref(), "0.00");
    }
}

#[test]
fn worksheets_add_rename_and_qualified_access_use_correct_sheet() {
    let mut session = session();
    completed(
        &mut session,
        r#"
Sub Main()
    Dim output As Object
    Worksheets(1).Name = "Input"
    Worksheets("Input").Range("A1").Value = 7
    Set output = Worksheets.Add
    output.Name = "Output"
    output.Cells(2, 3).Value = ThisWorkbook.Worksheets("Input").Cells(1, 1).Value * 6
    Worksheets("Input").Range("B1").Value = Worksheets.Count
End Sub
"#,
    );
    let workbook = &session.doc().unwrap().wb;
    let input = workbook.sheet_index("Input").unwrap();
    let output = workbook.sheet_index("Output").unwrap();
    assert_eq!(workbook.sheets.len(), 2);
    assert_eq!(cell(&session, input, "A1"), Value::number(7.0));
    assert_eq!(cell(&session, input, "B1"), Value::number(2.0));
    assert_eq!(cell(&session, output, "C2"), Value::number(42.0));
}

#[test]
fn chartobjects_add_source_type_and_title_create_a_real_chart() {
    let mut session = session();
    completed(
        &mut session,
        r#"
Sub Main()
    Dim output As Object
    Range("A1").Value = "Item"
    Range("B1").Value = "Sales"
    Range("A2").Value = "One"
    Range("B2").Value = 10
    Range("A3").Value = "Two"
    Range("B3").Value = 20
    Set output = ActiveSheet.ChartObjects.Add(10, 20, 300, 200)
    output.Chart.ChartType = xlColumnClustered
    output.Chart.HasTitle = True
    output.Chart.ChartTitle.Text = "Synthetic sales"
    output.Chart.SetSourceData Range("A1:B3")
End Sub
"#,
    );
    let sheet = session.doc().unwrap().wb.sheet(0).unwrap();
    assert_eq!(sheet.charts.len(), 1);
    let chart = &sheet.charts[0];
    assert_eq!(chart.kind, ChartKind::ColumnClustered);
    assert_eq!(chart.title.as_deref(), Some("Synthetic sales"));
    assert_eq!(chart.series.len(), 1);
    assert!(chart.source.as_deref().unwrap().contains("A1:B3"));
}

#[test]
fn staging_hides_intermediate_edits_and_publishes_one_undo_redo_step() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    let history = session.doc().unwrap().undo.len();
    let status = start(&mut session, "Sub Main()\nRange(\"A1\").Value = 1\nRange(\"B1\").Value = 2\nEnd Sub");
    assert_eq!(status["state"], "running");
    let status = session.execute("automate.pollVba", json!({"budget":1})).unwrap();
    assert_eq!(status["state"], "running");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert_eq!(cell(&session, 0, "A1"), Value::Empty);
    assert_eq!(finish_with_budget(&mut session, 1)["state"], "completed");
    assert_eq!(session.doc().unwrap().undo.len(), history + 1);
    assert_eq!(cell(&session, 0, "A1"), Value::number(1.0));
    assert_eq!(cell(&session, 0, "B1"), Value::number(2.0));
    session.execute("edit.undo", json!({})).unwrap();
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert_eq!(cell(&session, 0, "B1"), Value::Empty);
    session.execute("edit.redo", json!({})).unwrap();
    assert_eq!(cell(&session, 0, "A1"), Value::number(1.0));
    assert_eq!(cell(&session, 0, "B1"), Value::number(2.0));
}

#[test]
fn compile_failure_reports_a_line_without_modifying_workbook() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    let result = start(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nIf True Then\nEnd Sub");
    assert_eq!(result["state"], "failed");
    assert!(result["error"]["line"].as_u64().unwrap() > 0);
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert!(session.doc().unwrap().undo.is_empty());
}

#[test]
fn runtime_failure_discards_earlier_writes_and_new_worksheets() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    let result = run(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nWorksheets.Add\nRange(\"B1\").Value = 1 / 0\nEnd Sub");
    assert_eq!(result["state"], "failed", "{result}");
    assert!(result["error"]["line"].as_u64().unwrap() > 0);
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert_eq!(session.doc().unwrap().wb.sheets.len(), 1);
    assert!(session.doc().unwrap().undo.is_empty());
}

#[test]
fn stopping_discards_staged_writes_and_prevents_later_poll_commit() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    start(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nDo\nLoop\nEnd Sub");
    assert_eq!(session.execute("automate.pollVba", json!({"budget":10})).unwrap()["state"], "running");
    assert_eq!(session.execute("automate.stopVba", json!({})).unwrap()["state"], "stopped");
    assert_eq!(session.execute("automate.pollVba", json!({})).unwrap()["state"], "stopped");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert!(session.doc().unwrap().undo.is_empty());
}

#[test]
fn editing_real_workbook_during_run_aborts_without_replacing_user_edits() {
    let mut session = session();
    start(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nDo\nLoop\nEnd Sub");
    session.execute("automate.pollVba", json!({"budget":10})).unwrap();
    session.execute("cell.set", json!({"cell":"B2","input":"42"})).unwrap();
    let edited = session.doc().unwrap().wb.clone();
    let result = session.execute("automate.pollVba", json!({"budget":1000})).unwrap();
    assert_eq!(result["state"], "failed", "{result}");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &edited));
    assert_eq!(cell(&session, 0, "A1"), Value::Empty);
    assert_eq!(cell(&session, 0, "B2"), Value::number(42.0));
}

#[test]
fn switching_or_closing_the_workbook_discards_its_running_macro() {
    for close in [false, true] {
        let mut session = session();
        let first = session.active_index();
        start(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nDo\nLoop\nEnd Sub");
        session.execute("automate.pollVba", json!({"budget":10})).unwrap();
        let original = session.doc().unwrap().wb.clone();
        session.new_workbook();
        if close {
            session.close_document(first);
        }
        let second = session.doc().unwrap().wb.clone();
        let result = session.execute("automate.pollVba", json!({})).unwrap();
        assert_eq!(result["state"], "failed", "{result}");
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &second));
        if !close {
            session.set_active(first);
            assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &original));
            assert_eq!(cell(&session, 0, "A1"), Value::Empty);
        }
    }
}

#[test]
fn trust_revocation_aborts_pending_edits_and_blocks_fresh_execution() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    start(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nDo\nLoop\nEnd Sub");
    session.execute("automate.pollVba", json!({"budget":10})).unwrap();
    session.execute("automate.trustScripts", json!({"trusted":false})).unwrap();
    assert_eq!(session.execute("automate.pollVba", json!({})).unwrap()["state"], "failed");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert!(session.execute("automate.startVba", json!({"procedure":"Main","source":"Sub Main()\nEnd Sub"})).is_err());
}

#[test]
fn one_million_instruction_limit_is_exact_and_rolls_back_changes() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    start(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nDo\nLoop\nEnd Sub");
    let result = finish_with_budget(&mut session, 777);
    assert_eq!(result["state"], "failed", "{result}");
    assert!(result["error"]["message"].as_str().unwrap().contains("1,000,000"));
    assert!(result["steps"].as_u64().unwrap() <= 1_000_000, "{result}");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert!(session.doc().unwrap().undo.is_empty());
}

#[test]
fn saved_sources_survive_xlsx_roundtrip_and_require_new_session_trust() {
    let mut session = session();
    let source = "Sub Main()\nRange(\"A1\").Value = \"日本語\"\nEnd Sub";
    let saved = session.execute("automate.saveVbaModule", json!({"name":"Japanese","source":source})).unwrap();
    assert_eq!(saved["procedures"], json!(["Main"]));
    assert_eq!(session.execute("automate.vbaModules", json!({})).unwrap()["modules"].as_array().unwrap().len(), 1);
    let bytes = gridcraft_xlsx::write_xlsx(&session.doc().unwrap().wb).unwrap();
    let (workbook, _) = gridcraft_xlsx::read_xlsx(&bytes).unwrap();
    let mut reopened = Session::new();
    reopened.add_document(DocState::new(workbook, None, "Reopened".into()));
    let modules = reopened.execute("automate.vbaModules", json!({})).unwrap();
    assert_eq!(modules["trusted"], false);
    assert_eq!(modules["modules"][0]["source"], source);
    assert!(reopened.execute("automate.startVba", json!({"module":"Japanese","procedure":"Main"})).is_err());
    reopened.execute("automate.trustScripts", json!({"trusted":true})).unwrap();
    assert_eq!(reopened.execute("automate.startVba", json!({"module":"Japanese","procedure":"Main"})).unwrap()["state"], "running");
    assert_eq!(finish_with_budget(&mut reopened, 1000)["state"], "completed");
    assert_eq!(cell(&reopened, 0, "A1"), Value::text("日本語"));
}

/// Original synthetic MS-OVBA records inside the shared test CFB writer; no external workbook.
/// Project order deliberately differs from lexical order, and each module kind is represented.
fn imported_project() -> Vec<u8> {
    fn record(bytes: &mut Vec<u8>, id: u16, value: &[u8]) {
        bytes.extend(id.to_le_bytes());
        bytes.extend((value.len() as u32).to_le_bytes());
        bytes.extend(value);
    }
    fn wide(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }
    fn literal(bytes: &[u8]) -> Vec<u8> {
        assert!(!bytes.is_empty() && bytes.len() <= 3600);
        let mut encoded = Vec::new();
        for group in bytes.chunks(8) {
            encoded.push(0);
            encoded.extend(group);
        }
        let mut result = vec![1];
        result.extend((0xB000u16 | (encoded.len() as u16 - 1)).to_le_bytes());
        result.extend(encoded);
        result
    }
    let modules = [("Zeta", true), ("ThisWorkbook", false), ("Worker", false), ("EntryForm", false), ("Opaque", false)];
    let mut dir = Vec::new();
    for (id, value) in [(1, 1u32), (2, 0x409), (0x14, 0x409)] {
        record(&mut dir, id, &value.to_le_bytes());
    }
    record(&mut dir, 3, &1252u16.to_le_bytes());
    record(&mut dir, 4, b"SyntheticProject");
    for id in [5, 0x40, 6, 0x3D] {
        record(&mut dir, id, &[]);
    }
    record(&mut dir, 7, &0u32.to_le_bytes());
    record(&mut dir, 8, &0u32.to_le_bytes());
    record(&mut dir, 9, &1u32.to_le_bytes());
    dir.extend(0u16.to_le_bytes()); // PROJECTVERSION MinorVersion is outside its reserved size.
    record(&mut dir, 0xF, &(modules.len() as u16).to_le_bytes());
    record(&mut dir, 0x13, &0xFFFFu16.to_le_bytes());
    for (name, standard) in modules {
        record(&mut dir, 0x19, name.as_bytes());
        record(&mut dir, 0x47, &wide(name));
        record(&mut dir, 0x1A, name.as_bytes());
        record(&mut dir, 0x32, &wide(name));
        record(&mut dir, 0x1C, &[]);
        record(&mut dir, 0x48, &[]);
        record(&mut dir, 0x31, &0u32.to_le_bytes());
        record(&mut dir, 0x1E, &0u32.to_le_bytes());
        record(&mut dir, 0x2C, &0xFFFFu16.to_le_bytes());
        record(&mut dir, if standard { 0x21 } else { 0x22 }, &[]);
        record(&mut dir, 0x2B, &[]);
    }
    record(&mut dir, 0x10, &[]);
    let dir = literal(&dir);
    let source = literal(b"Sub Main()\r\nEnd Sub\r\n");
    let manifest = b"Module=Zeta\r\nDocument=ThisWorkbook/&H00000000\r\nClass=Worker\r\nBaseClass=EntryForm\r\n";
    let mut streams = vec![("VBA", &[][..]), ("dir", dir.as_slice())];
    streams.extend(modules.iter().map(|(name, _)| (*name, source.as_slice())));
    streams.push(("PROJECT", manifest.as_slice()));
    let mut file = gridcraft_xlsx::testing::compound_file(&streams);
    let directory = (u32::from_le_bytes(file[48..52].try_into().unwrap()) as usize + 1) * 512;
    file[directory + 128 + 66] = 1; // VBA storage, rather than a stream.
    file[directory + 128 + 72..directory + 128 + 76].copy_from_slice(&(modules.len() as u32 + 3).to_le_bytes()); // PROJECT under root.
    file[directory + 128 + 76..directory + 128 + 80].copy_from_slice(&2u32.to_le_bytes()); // dir child.
    let last_child = directory + (modules.len() + 2) * 128;
    file[last_child + 72..last_child + 76].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    file
}

#[test]
fn edited_imported_modules_keep_kind_canonical_name_order_and_original_project_after_reload() {
    let project = Arc::new(imported_project());
    let mut workbook = Workbook::new();
    workbook.vba_project = Some(Arc::clone(&project));
    let mut session = Session::new();
    session.add_document(DocState::new(workbook, None, "Imported modules".into()));
    let expected = [("Zeta", "standard"), ("ThisWorkbook", "document"), ("Worker", "class"), ("EntryForm", "form"), ("Opaque", "object")];
    let original = session.execute("automate.vbaModules", json!({})).unwrap();
    assert!(original["importError"].is_null(), "{original}");
    for (module, (name, kind)) in original["modules"].as_array().unwrap().iter().zip(expected) {
        assert_eq!(module["name"], name);
        assert_eq!(module["kind"], kind);
        assert_eq!(module["origin"], "project");
    }
    assert_eq!(original["modules"].as_array().unwrap().len(), expected.len());
    let edited = "Sub Edited()\nEnd Sub";
    for (name, kind) in expected {
        let result = session.execute("automate.saveVbaModule", json!({"name":name.to_ascii_lowercase(),"source":edited})).unwrap();
        assert_eq!(result["name"], name, "saving must return the imported spelling");
        assert_eq!(result["kind"], kind);
        assert_eq!(result["source"], edited);
        assert_eq!(result["origin"], "gridcraft");
        assert!(Arc::ptr_eq(session.doc().unwrap().wb.vba_project.as_ref().unwrap(), &project));
    }
    let created = session.execute("automate.saveVbaModule", json!({"name":"Aardvark","source":edited})).unwrap();
    assert_eq!(created["kind"], "standard");
    let listed = session.execute("automate.vbaModules", json!({})).unwrap();
    assert_eq!(listed["modules"].as_array().unwrap().len(), expected.len() + 1);
    for (module, (name, kind)) in listed["modules"].as_array().unwrap().iter().zip(expected) {
        assert_eq!(module["name"], name, "editing must retain project order");
        assert_eq!(module["kind"], kind);
        assert_eq!(module["source"], edited);
        assert_eq!(module["origin"], "gridcraft");
    }
    assert_eq!(listed["modules"][expected.len()]["name"], "Aardvark", "new standalone modules follow the imported project");
    let saved_sources = super::sources(&session.doc().unwrap().wb).unwrap();
    assert_eq!(saved_sources.len(), expected.len() + 1);
    for (name, _) in expected {
        assert_eq!(saved_sources.get(name).map(String::as_str), Some(edited));
    }
    let bytes = gridcraft_xlsx::write_xlsx(&session.doc().unwrap().wb).unwrap();
    let (workbook, _) = gridcraft_xlsx::read_xlsx(&bytes).unwrap();
    assert_eq!(workbook.vba_project.as_deref(), Some(project.as_ref()), "saving only overlays source metadata, preserving vbaProject.bin bytes");
    let mut reopened = Session::new();
    reopened.add_document(DocState::new(workbook, None, "Reopened modules".into()));
    let reloaded = reopened.execute("automate.vbaModules", json!({})).unwrap();
    assert_eq!(reloaded["modules"], listed["modules"]);
    assert_eq!(reloaded["trusted"], false);
    assert_eq!(reloaded["preservedVba"], true);
    assert!(reloaded["importError"].is_null());
}

#[test]
fn legacy_source_aliases_use_imported_identity_and_are_canonicalized_when_saved() {
    let project = Arc::new(imported_project());
    let mut workbook = Workbook::new();
    workbook.vba_project = Some(Arc::clone(&project));
    workbook.names.push(DefinedName {
        name: super::SOURCES.into(),
        scope: None,
        formula: metadata_formula(&json!({"thisworkbook":"Sub Legacy()\nEnd Sub"}).to_string()),
        comment: String::new(),
        hidden: true,
    });
    let mut session = Session::new();
    session.add_document(DocState::new(workbook, None, "Legacy module alias".into()));
    let listed = session.execute("automate.vbaModules", json!({})).unwrap();
    assert_eq!(listed["modules"][1]["name"], "ThisWorkbook");
    assert_eq!(listed["modules"][1]["kind"], "document");
    assert_eq!(listed["modules"][1]["source"], "Sub Legacy()\nEnd Sub");
    assert_eq!(listed["modules"][1]["origin"], "gridcraft");
    let source = "Sub Current()\nEnd Sub";
    let saved = session.execute("automate.saveVbaModule", json!({"name":"THISWORKBOOK","source":source})).unwrap();
    assert_eq!(saved["name"], "ThisWorkbook");
    assert_eq!(saved["kind"], "document");
    assert_eq!(super::sources(&session.doc().unwrap().wb).unwrap(), std::collections::BTreeMap::from([("ThisWorkbook".into(), source.into())]));
    assert!(Arc::ptr_eq(session.doc().unwrap().wb.vba_project.as_ref().unwrap(), &project));
}

#[test]
fn unreadable_original_project_keeps_saved_sources_available_and_preserves_its_bytes() {
    let project = Arc::new(b"synthetic invalid project".to_vec());
    let mut workbook = Workbook::new();
    workbook.vba_project = Some(Arc::clone(&project));
    let mut session = Session::new();
    session.add_document(DocState::new(workbook, None, "Unreadable project".into()));
    let source = "Sub Main()\nEnd Sub";
    let saved = session.execute("automate.saveVbaModule", json!({"name":"Saved","source":source})).unwrap();
    assert_eq!(saved["name"], "Saved");
    assert_eq!(saved["kind"], "standard");
    let listed = session.execute("automate.vbaModules", json!({})).unwrap();
    assert!(listed["importError"].is_string());
    assert_eq!(listed["modules"], json!([saved]));
    assert!(Arc::ptr_eq(session.doc().unwrap().wb.vba_project.as_ref().unwrap(), &project));
}

#[test]
fn shell_createobject_and_unknown_members_fail_without_committing_prior_edits() {
    for unsupported in [
        "Shell \"echo should-not-run\"",
        "CreateObject(\"WScript.Shell\")",
        "Range(\"A1\").UnsupportedMember = 1",
        "Application.Run \"anything\"",
        "MsgBox \"unsupported prompt\"",
    ] {
        let mut session = session();
        let before = session.doc().unwrap().wb.clone();
        let source = format!("Sub Main()\nRange(\"A1\").Value = 99\n{unsupported}\nEnd Sub");
        let result = run(&mut session, &source);
        assert_eq!(result["state"], "failed", "{unsupported}: {result}");
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
        assert!(session.ui_requests.is_empty());
    }
}

#[test]
fn large_value_format_and_function_ranges_fail_without_partial_commit() {
    for action in [
        "Range(\"A1:A10001\").Value = 1",
        "Range(\"A1:A10001\").Font.Bold = True",
        "Range(\"A1:A10001\").NumberFormat = \"0.00\"",
        "Range(\"B1\").Value = WorksheetFunction.Sum(Range(\"A1:A10001\"))",
    ] {
        let mut session = session();
        let before = session.doc().unwrap().wb.clone();
        let result = run(&mut session, &format!("Sub Main()\nRange(\"C1\").Value = 99\n{action}\nEnd Sub"));
        assert_eq!(result["state"], "failed", "{action}: {result}");
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    }
}

fn assert_runtime_failure_is_atomic(action: &str, expected: &str) {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    let source = format!("Sub Main()\nRange(\"Z1\").Value = 99\n{action}\nEnd Sub");
    let result = run(&mut session, &source);
    assert_eq!(result["state"], "failed", "{action}: {result}");
    assert!(result["error"]["message"].as_str().unwrap().contains(expected), "{action}: {result}");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before), "{action}");
    assert!(session.doc().unwrap().undo.is_empty(), "{action}");
}

#[test]
fn scalar_range_expansion_is_bounded_before_allocating_its_command_payload() {
    // A small source can produce a 512 KiB scalar, whose 10,000-cell expansion used to
    // allocate roughly 5 GiB inside one cooperative VM instruction.
    assert_runtime_failure_is_atomic(
        r#"Dim s As String
Dim i As Long
s = "x"
For i = 1 To 19
    s = s & s
Next i
Range("A1:J1000").Value = s"#,
        "16 MiB total byte limit",
    );
}

#[test]
fn style_strings_cannot_amplify_long_text_across_distinct_cell_styles() {
    for property in ["Font.Name", "NumberFormat"] {
        assert_runtime_failure_is_atomic(
            &format!("Dim s As String\nDim i As Long\ns = \"x\"\nFor i = 1 To 19\ns = s & s\nNext i\nRange(\"A1:A10000\").{property} = s"),
            "1,024 bytes",
        );
        assert_runtime_failure_is_atomic(
            &format!(
                "Dim s As String\nDim i As Long\ns = \"x\"\nFor i = 1 To 9\ns = s & s\nNext i\nFor i = 1 To 4\nRange(\"A1:A10000\").{property} = s\nNext i"
            ),
            "16 MiB total byte limit",
        );
    }
}

#[test]
fn cumulative_cell_value_bytes_are_bounded_and_discard_earlier_writes() {
    assert_runtime_failure_is_atomic(
        r#"Dim s As String
Dim i As Long
s = "x"
For i = 1 To 16
    s = s & s
Next i
Range("A1:A200").Value = s
Range("B1:B200").Value = s"#,
        "16 MiB total byte limit",
    );
}

#[test]
fn array_values_and_formula_payloads_are_checked_before_command_conversion() {
    use super::language::{Host, Value as VbaValue};

    for formula in [false, true] {
        let mut host = super::host::ExcelHost::new(session());
        let before = host.session.doc().unwrap().wb.clone();
        let target = host.get(None, "Range", &[VbaValue::Text("A1:P1".into())]).unwrap();
        // Sixteen maximum-size values fit the payload budget exactly as formulas; text
        // values also need the apostrophe prefix. The extra scalar already written below
        // makes both cases exceed the cumulative budget before any JSON values are built.
        let prior = host.get(None, "Range", &[VbaValue::Text("Z1".into())]).unwrap();
        host.set(Some(&prior), "Value", &[], VbaValue::Number(99.0)).unwrap();
        let staged = host.session.doc().unwrap().wb.clone();
        let values = VbaValue::Array(vec![vec![VbaValue::Text("x".repeat(1024 * 1024)); 16]]);
        let error = host.set(Some(&target), if formula { "Formula" } else { "Value" }, &[], values).unwrap_err();
        assert!(error.message.contains("16 MiB total byte limit"), "{error}");
        assert!(Arc::ptr_eq(&host.session.doc().unwrap().wb, &staged));
        assert!(!Arc::ptr_eq(&host.session.doc().unwrap().wb, &before));
        assert_eq!(cell(&host.session, 0, "A1"), Value::Empty);
        assert_eq!(cell(&host.session, 0, "P1"), Value::Empty);
        assert_eq!(cell(&host.session, 0, "Z1"), Value::number(99.0));
    }
}

#[test]
fn range_read_checks_shared_workbook_text_and_formulas_before_cloning() {
    for formula in [false, true] {
        let mut session = session();
        let value = if formula {
            gridcraft_model::Cell::formula(gridcraft_model::Formula { text: "x".repeat(1024 * 1024 - 1), expr: None, array: None, dynamic: false })
        } else {
            gridcraft_model::Cell::value(Value::text("x".repeat(1024 * 1024)))
        };
        let sheet = Arc::make_mut(&mut session.doc_mut().unwrap().wb).sheet_mut(0).unwrap();
        for col in 0..17 {
            sheet.cells.set(CellRef::new(0, col), value.clone());
        }
        let before = session.doc().unwrap().wb.clone();
        let property = if formula { "Formula" } else { "Value" };
        let result = run(&mut session, &format!("Sub Main()\nDim result\nresult = Range(\"A1:Q1\").{property}\nEnd Sub"));
        assert_eq!(result["state"], "failed", "{result}");
        assert!(result["error"]["message"].as_str().unwrap().contains("range read exceeds the 16 MiB byte limit"), "{result}");
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
        assert!(session.doc().unwrap().undo.is_empty());
    }
}

#[test]
fn oversized_autofit_axes_and_style_reads_cannot_bypass_range_limits() {
    for action in [
        "Rows.AutoFit",
        "Columns.AutoFit",
        "Range(\"A1:A10001\").Rows.AutoFit",
        "Range(\"A1:XFD1\").Columns.AutoFit",
        "Debug.Print Range(\"A1:A10001\").Font.Bold",
        "Debug.Print Range(\"A1:A10001\").NumberFormat",
    ] {
        assert_runtime_failure_is_atomic(action, "10,000");
    }
}

#[test]
fn unsupported_getter_method_and_setter_arguments_are_never_ignored() {
    for action in [
        "Debug.Print ThisWorkbook.Name(1)",
        "Debug.Print Worksheets.Count(1)",
        "Debug.Print Rows.Count(1)",
        "Debug.Print Columns.Count(1)",
        "Debug.Print Range(\"A1\").Font.Bold(1)",
        "Debug.Print Range(\"A1\").Font.Italic(1)",
        "Debug.Print Range(\"A1\").Font.Name(1)",
        "Debug.Print Range(\"A1\").Font.Size(1)",
        "Range(\"A1\").Value(1) = 2",
        "Range(\"A1\").Font.Bold(1) = True",
        "ActiveSheet.Name(1) = \"Output\"",
        "Rows(1).AutoFit(1)",
        "Columns(1).AutoFit(1)",
        "Dim chart As Object\nSet chart = ActiveSheet.ChartObjects.Add(0, 0, 300, 200)\nchart.Chart(1).ChartType = xlColumnClustered",
        "Dim chart As Object\nSet chart = ActiveSheet.ChartObjects.Add(0, 0, 300, 200)\nDebug.Print chart.Chart.ChartTitle(1).Text",
        "Dim chart As Object\nSet chart = ActiveSheet.ChartObjects.Add(0, 0, 300, 200)\nchart.Chart.ChartType(1) = xlColumnClustered",
    ] {
        assert_runtime_failure_is_atomic(action, "does not accept arguments");
    }
    for (action, expected) in [
        ("Rows(1, 2).AutoFit", "at most one"),
        ("Columns(1, 2).AutoFit", "at most one"),
        ("Range(\"A1\").Rows.Item(1, 2).ClearContents", "one index"),
        ("Range(\"A1\").Columns.Item(1, 2).ClearContents", "one index"),
        ("Range(\"A1\").Offset(0, 0, 1).Value = 1", "at most two"),
        ("Range(\"A1\").Resize(1, 1, 1).Value = 1", "at most two"),
        ("Range(\"A1\").End(xlUp, 1).Value = 1", "one direction"),
    ] {
        assert_runtime_failure_is_atomic(action, expected);
    }
}

#[test]
fn fractional_or_unknown_enum_constants_fail_without_committing_staged_edits() {
    for direction in ["-4162.5", "-4121.5", "-4159.5", "-4161.5", "-4163", "0"] {
        assert_runtime_failure_is_atomic(&format!("Range(\"A3\").End({direction}).Value = 1"), "invalid End direction");
    }
    for kind in ["51.5", "57.5", "4.5", "5.5", "52", "0"] {
        assert_runtime_failure_is_atomic(
            &format!("Dim chart As Object\nSet chart = ActiveSheet.ChartObjects.Add(0, 0, 300, 200)\nchart.Chart.ChartType = {kind}"),
            "ChartType is not implemented",
        );
    }
}

#[test]
fn mixed_font_and_number_format_getters_report_unsupported_null_explicitly() {
    for (first, second, property) in [
        ("True", "False", "Font.Bold"),
        ("True", "False", "Font.Italic"),
        ("\"GridCraft Synthetic One\"", "\"GridCraft Synthetic Two\"", "Font.Name"),
        ("10", "12", "Font.Size"),
        ("\"0.00\"", "\"0%\"", "NumberFormat"),
    ] {
        let action = format!("Range(\"A1\").{property} = {first}\nRange(\"A2\").{property} = {second}\nDebug.Print Range(\"A1:A2\").{property}");
        assert_runtime_failure_is_atomic(&action, "require VBA Null");
    }
}

#[test]
fn bounded_autofit_uniform_style_reads_and_range_transformations_remain_usable() {
    let mut session = session();
    completed(
        &mut session,
        r#"
Sub Main()
    Range("A1:A3").Value = 7
    Range("A1:A3").Font.Bold = True
    Range("A1:A3").Font.Italic = True
    Range("A1:A3").Font.Name = "GridCraft Synthetic"
    Range("A1:A3").Font.Size = 12
    Range("A1:A3").NumberFormat = "0.00"
    Range("B1").Value = Range("A1:A3").Font.Bold
    Range("B2").Value = Range("A1:A3").Font.Italic
    Range("B3").Value = Range("A1:A3").Font.Name
    Range("B4").Value = Range("A1:A3").Font.Size
    Range("B5").Value = Range("A1:A3").NumberFormat
    Range("A1").Font.Bold = False
    Range("B6").Value = Range("A1:A3").Font.Italic
    Rows(1).AutoFit
    Columns(1).AutoFit
    Range("A1:A3").Rows.AutoFit
    Range("A1:B3").Columns.AutoFit
    Range("C1").Value = Range("A3").End(xlUp).Address
    Range("C2").Value = Range("A1").Offset(1, 1).Address
    Range("C3").Value = Range("A1").Resize(2, 2).Address
    Range("C4").Value = Range("A1").Offset.Address
    Range("C5").Value = Range("A1").Resize.Address
End Sub
"#,
    );
    assert_eq!(cell(&session, 0, "B1"), Value::Bool(true));
    assert_eq!(cell(&session, 0, "B2"), Value::Bool(true));
    assert_eq!(cell(&session, 0, "B3"), Value::text("GridCraft Synthetic"));
    assert_eq!(cell(&session, 0, "B4"), Value::number(12.0));
    assert_eq!(cell(&session, 0, "B5"), Value::text("0.00"));
    assert_eq!(cell(&session, 0, "B6"), Value::Bool(true));
    for (address, expected) in [("C1", "$A$1"), ("C2", "$B$2"), ("C3", "$A$1:$B$2"), ("C4", "$A$1"), ("C5", "$A$1")] {
        assert_eq!(cell(&session, 0, address), Value::text(expected));
    }
}

#[test]
fn worksheet_range_cannot_silently_redirect_to_a_different_sheet() {
    assert_runtime_failure_is_atomic(
        "Worksheets(1).Name = \"Input\"\nWorksheets.Add\nActiveSheet.Name = \"Output\"\nWorksheets(\"Input\").Range(\"Output!A1\").Value = 7",
        "different worksheet",
    );
    let mut session = session();
    completed(&mut session, "Sub Main()\nActiveSheet.Name = \"Input\"\nWorksheets(\"Input\").Range(\"Input!A1\").Value = 7\nEnd Sub");
    assert_eq!(cell(&session, 0, "A1"), Value::number(7.0));
}

#[test]
fn module_names_are_case_insensitive_and_source_count_size_bounds_are_atomic() {
    let mut session = session();
    let source = "Sub Main()\nEnd Sub";
    session.execute("automate.saveVbaModule", json!({"name":"Example","source":source})).unwrap();
    session.execute("automate.saveVbaModule", json!({"name":"example","source":"Sub Other()\nEnd Sub"})).unwrap();
    let modules = session.execute("automate.vbaModules", json!({})).unwrap();
    assert_eq!(modules["modules"].as_array().unwrap().len(), 1);
    assert_eq!(modules["modules"][0]["procedures"], json!(["Other"]));
    for index in 1..128 {
        session.execute("automate.saveVbaModule", json!({"name":format!("Module{index}"),"source":source})).unwrap();
    }
    let before = session.doc().unwrap().wb.clone();
    for params in [
        json!({"name":"Module129","source":source}),
        json!({"name":"Example","source":"x".repeat(1024*1024+1)}),
        json!({"name":"not a module","source":source}),
    ] {
        assert!(session.execute("automate.saveVbaModule", params).is_err());
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    }
}

#[test]
fn log_only_macros_create_no_undo_and_report_output_without_dialogs() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    let result = completed(&mut session, "Sub Main()\nDebug.Print \"Hello\"\nDebug.Print \"日本語\"\nEnd Sub");
    assert_eq!(result["log"], json!(["Hello", "日本語"]));
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert!(session.doc().unwrap().undo.is_empty());
    assert!(session.ui_requests.is_empty());
}

#[test]
fn missing_procedure_parallel_start_and_incomplete_charts_are_explicit_failures() {
    let mut session = session();
    let before = session.doc().unwrap().wb.clone();
    let result = session.execute("automate.startVba", json!({"procedure":"Missing","source":"Sub Main()\nEnd Sub"})).unwrap();
    assert_eq!(result["state"], "failed");
    start(&mut session, "Sub Main()\nDo\nLoop\nEnd Sub");
    assert!(session.execute("automate.startVba", json!({"procedure":"Main","source":"Sub Main()\nEnd Sub"})).is_err());
    session.execute("automate.stopVba", json!({})).unwrap();
    let result = run(&mut session, "Sub Main()\nRange(\"A1\").Value = 99\nActiveSheet.ChartObjects.Add(0, 0, 300, 200)\nEnd Sub");
    assert_eq!(result["state"], "failed", "{result}");
    assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
    assert!(session.doc().unwrap().undo.is_empty());
}

fn metadata_formula(raw: &str) -> String {
    format!("\"{}\"", raw.replace('"', "\"\""))
}

fn malformed_metadata_is_atomic(formula: String) {
    let mut workbook = Workbook::new();
    workbook.names.push(DefinedName { name: super::SOURCES.into(), scope: None, formula, comment: String::new(), hidden: true });
    let mut session = Session::new();
    session.add_document(DocState::new(workbook, None, "Malformed metadata".into()));
    assert!(!session.doc().unwrap().scripts_trusted);
    session.execute("automate.trustScripts", json!({"trusted":true})).unwrap();
    let before = session.doc().unwrap().wb.clone();
    for (command, params) in [
        ("automate.vbaModules", json!({})),
        ("automate.startVba", json!({"module":"Example","procedure":"Main"})),
        ("automate.saveVbaModule", json!({"name":"Example","source":"Sub Main()\nEnd Sub"})),
    ] {
        assert!(session.execute(command, params).is_err(), "{command} accepted malformed source metadata");
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
        assert!(session.doc().unwrap().undo.is_empty());
        assert_eq!(cell(&session, 0, "A1"), Value::Empty);
    }
    assert_eq!(session.execute("automate.vbaStatus", json!({})).unwrap()["state"], "idle");
}

#[test]
fn malformed_loaded_source_metadata_is_an_explicit_atomic_error() {
    for formula in [
        "not a quoted string".to_string(),
        metadata_formula("{invalid json}"),
        metadata_formula("[]"),
        metadata_formula(r#"{"Example":42}"#),
        metadata_formula(r#"{"Example":"","Example":"Sub Main()\nEnd Sub"}"#),
        metadata_formula(r#"{"Example":"","example":""}"#),
        metadata_formula(r#"{"not an identifier":""}"#),
        metadata_formula(r#"{"":""}"#),
    ] {
        malformed_metadata_is_atomic(formula);
    }
}

#[test]
fn duplicate_workbook_source_metadata_entries_are_rejected_atomically() {
    let mut workbook = Workbook::new();
    for source in [r#"{"Example":"Sub Main()\nEnd Sub"}"#, r#"{"Other":"Sub Other()\nEnd Sub"}"#] {
        workbook.names.push(DefinedName {
            name: super::SOURCES.into(),
            scope: None,
            formula: metadata_formula(source),
            comment: String::new(),
            hidden: true,
        });
    }
    let mut session = Session::new();
    session.add_document(DocState::new(workbook, None, "Duplicate metadata".into()));
    session.execute("automate.trustScripts", json!({"trusted":true})).unwrap();
    let before = session.doc().unwrap().wb.clone();
    for (command, params) in [
        ("automate.vbaModules", json!({})),
        ("automate.startVba", json!({"module":"Example","procedure":"Main"})),
        ("automate.saveVbaModule", json!({"name":"Example","source":"Sub Main()\nEnd Sub"})),
    ] {
        let result = session.execute(command, params).unwrap_err();
        assert!(result.to_string().contains("duplicate VBA source metadata"), "{command}: {result}");
        assert!(Arc::ptr_eq(&session.doc().unwrap().wb, &before));
        assert!(session.doc().unwrap().undo.is_empty());
    }
}

#[test]
fn loaded_source_module_count_individual_total_and_encoded_size_are_bounded() {
    let many = (0..129).map(|index| (format!("Module{index}"), String::new())).collect::<std::collections::BTreeMap<_, _>>();
    malformed_metadata_is_atomic(metadata_formula(&serde_json::to_string(&many).unwrap()));
    malformed_metadata_is_atomic(metadata_formula(&json!({"Example":"x".repeat(super::MAX_SOURCE+1)}).to_string()));
    let total = (0..9).map(|index| (format!("Module{index}"), "x".repeat(super::MAX_SOURCE))).collect::<std::collections::BTreeMap<_, _>>();
    malformed_metadata_is_atomic(metadata_formula(&serde_json::to_string(&total).unwrap()));
    // Reject the encoded input before JSON parsing, even when it begins with a quote.
    let mut encoded = String::with_capacity(super::MAX_TOTAL_SOURCE * 6 + 65_537);
    encoded.push('"');
    encoded.extend(std::iter::repeat_n('x', super::MAX_TOTAL_SOURCE * 6 + 65_536));
    malformed_metadata_is_atomic(encoded);
}
