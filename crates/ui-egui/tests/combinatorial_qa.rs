//! Ten distinct end-to-end combinations. No soak loops: each state transition is exercised once.
//! Two/three egui frames settle layout, focus and accessibility; they do not replay an operation.
use egui::{Event, Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;
use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
    io::{base64_decode, base64_encode},
};
use gridcraft_ui_egui::{RibbonMode, SheetApp, grid, i18n::Language};
use serde_json::{Value as Json, json};
use std::sync::Arc;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut app = SheetApp::new(session, Default::default());
    app.ui.language = Language::Ja;
    Harness::builder().with_size(egui::vec2(1500.0, 1100.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    )
}

fn run(h: &mut Harness, id: &str, p: Json) -> Json {
    h.state_mut().run(id, p).unwrap_or_else(|e| panic!("{id}: {e}"))
}
fn value(h: &Harness, sheet: usize, address: &str) -> Value {
    h.state().session.active().unwrap().wb.sheet(sheet).unwrap().value(CellRef::parse(address).unwrap())
}
fn japanese(text: &str) -> bool {
    text.chars().any(|c| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}'))
}
fn assert_alert(h: &mut Harness) {
    let (_, raw) = h.state().message.clone().expect("operation must be refused with a message");
    let expected = Language::Ja.tr_error(&raw);
    assert!(japanese(&expected), "{raw} -> {expected}");
    h.run_steps(2);
    assert!(h.query_by_label(&expected).is_some(), "translated text actually reaches the popup: {expected}");
    if raw != expected {
        assert!(h.query_by_label(&raw).is_none());
    }
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(h.state().message.is_none());
}
fn refuse(h: &mut Harness, id: &str, p: Json) {
    let before = h.state().session.active().unwrap().wb.clone();
    h.state_mut().run_or_alert(id, p);
    assert_alert(h);
    assert!(Arc::ptr_eq(&before, &h.state().session.active().unwrap().wb), "failed edit must be atomic: {id}");
}

#[test]
fn combination_01_cross_sheet_sort_filter_undo_redo() {
    let mut h = harness();
    run(&mut h, "range.setValues", json!({"range":"A1:B5", "values":[["項目","売上"],["a",30],["b",10],["c",40],["d",20]]}));
    run(&mut h, "home.insertSheet", json!({"name":"Sheet2"}));
    run(&mut h, "cell.set", json!({"cell":"A1","input":"=SUM(Sheet1!B2:B5)"}));
    run(&mut h, "cell.set", json!({"cell":"A2","input":"=VLOOKUP(\"a\",Sheet1!A2:B5,2,FALSE)"}));
    run(&mut h, "sheet.activate", json!({"sheet":"Sheet1"}));
    run(&mut h, "selection.set", json!({"range":"A1:B5"}));
    // Install before sorting so the two undo steps are exactly sort and filter criteria.
    run(&mut h, "data.filter", json!({"range":"A1:B5"}));
    run(&mut h, "data.sort", json!({"range":"A1:B5","header":true,"keys":[{"column":"B","order":"desc"}]}));
    run(&mut h, "data.filterBy", json!({"column":"B","values":[40,30]}));
    assert_eq!(value(&h, 0, "B2"), Value::number(40.0));
    assert_eq!(value(&h, 1, "A1"), Value::number(100.0));
    assert_eq!(value(&h, 1, "A2"), Value::number(30.0));
    assert!(h.state().session.active().unwrap().wb.sheet(0).unwrap().is_row_hidden(3));
    run(&mut h, "edit.undo", json!({}));
    run(&mut h, "edit.undo", json!({}));
    assert_eq!(value(&h, 0, "B2"), Value::number(30.0));
    assert!(!h.state().session.active().unwrap().wb.sheet(0).unwrap().is_row_hidden(3));
    run(&mut h, "edit.redo", json!({}));
    run(&mut h, "edit.redo", json!({}));
    assert_eq!(value(&h, 0, "B2"), Value::number(40.0));
    assert_eq!(value(&h, 1, "A2"), Value::number(30.0));
    assert!(h.state().session.active().unwrap().wb.sheet(0).unwrap().is_row_hidden(3));
}

#[test]
fn combination_02_copy_cut_skip_font_transpose_zoom() {
    let mut h = harness();
    run(&mut h, "range.setValues", json!({"range":"A1:B2","values":[[12,34],[56,78]]}));
    run(&mut h, "home.fontSize", json!({"range":"A1:B2","size":72}));
    run(&mut h, "home.increaseFontSize", json!({"range":"A1:B2"}));
    run(&mut h, "edit.copy", json!({"range":"A1:B2"}));
    run(&mut h, "selection.set", json!({"cell":"H20"}));
    run(&mut h, "selection.set", json!({"cell":"D4"}));
    run(&mut h, "edit.paste", json!({}));
    let doc = h.state().session.active().unwrap();
    assert_eq!(doc.wb.styles.get(doc.wb.active().unwrap().style_id(CellRef::parse("D4").unwrap())).font.size, 96.0);
    run(&mut h, "selection.set", json!({"cell":"H4"}));
    run(&mut h, "edit.pasteSpecial", json!({"what":"values","transpose":true}));
    assert_eq!(value(&h, 0, "I4"), Value::number(56.0));
    assert_eq!(value(&h, 0, "H5"), Value::number(34.0));
    run(&mut h, "edit.cut", json!({"range":"D4:E5"}));
    run(&mut h, "edit.paste", json!({"at":"L8"}));
    assert_eq!(value(&h, 0, "D4"), Value::Empty);
    assert_eq!(value(&h, 0, "L8"), Value::number(12.0));
    for percent in [50, 200, 100] {
        run(&mut h, "view.zoom", json!({"percent":percent}));
        h.run_steps(2);
        assert!(h.state().message.is_none());
    }
}

#[test]
fn combination_03_validation_japanese_alert_then_valid_entry() {
    let mut h = harness();
    run(&mut h, "data.validation", json!({"range":"A1","type":"whole","operator":"between","formula1":"1","formula2":"100"}));
    h.state_mut().begin_edit(Some("999".into()), false);
    assert!(!h.state_mut().commit_edit(0, 0, false, false));
    assert_alert(&mut h);
    assert_eq!(value(&h, 0, "A1"), Value::Empty);
    h.state_mut().editor.as_mut().unwrap().text = "50".into();
    assert!(h.state_mut().commit_edit(0, 0, false, false));
    assert_eq!(value(&h, 0, "A1"), Value::number(50.0));
}

#[test]
fn combination_04_protection_refusals_and_recovery() {
    let mut h = harness();
    run(&mut h, "review.protectSheet", json!({}));
    refuse(&mut h, "cell.set", json!({"cell":"A1","input":"42"}));
    refuse(&mut h, "home.insertRows", json!({"rows":"1:1"}));
    refuse(&mut h, "home.deleteColumns", json!({"cols":"B:B"}));
    refuse(&mut h, "home.bold", json!({"range":"A1"}));
    run(&mut h, "review.protectWorkbook", json!({"on":true}));
    refuse(&mut h, "home.insertSheet", json!({"name":"Blocked"}));
    run(&mut h, "review.unprotectSheet", json!({}));
    run(&mut h, "review.protectWorkbook", json!({"on":false}));
    run(&mut h, "cell.set", json!({"cell":"A1","input":"42"}));
    assert_eq!(value(&h, 0, "A1"), Value::number(42.0));
}

#[test]
fn combination_05_merge_spill_and_partial_cut_are_atomic() {
    let mut h = harness();
    run(&mut h, "home.mergeCells", json!({"range":"B2:C3"}));
    run(&mut h, "cell.set", json!({"cell":"E2","input":"=SEQUENCE(3)"}));
    run(&mut h, "selection.set", json!({"cell":"E3"}));
    let before = h.state().session.active().unwrap().wb.clone();
    h.state_mut().begin_edit(Some("99".into()), false);
    if h.state().editor.is_some() {
        h.state_mut().commit_edit(0, 0, false, false);
    }
    assert_alert(&mut h);
    assert!(Arc::ptr_eq(&before, &h.state().session.active().unwrap().wb));
    assert_eq!(value(&h, 0, "E3"), Value::number(2.0));
    run(&mut h, "range.setValues", json!({"range":"G1:H1","values":[[7,8]]}));
    run(&mut h, "edit.cut", json!({"range":"G1:H1"}));
    refuse(&mut h, "edit.paste", json!({"at":"C3"}));
    assert_eq!(value(&h, 0, "G1"), Value::number(7.0));
    assert_eq!(h.state().session.active().unwrap().wb.active().unwrap().merges.len(), 1);
    run(&mut h, "edit.paste", json!({"at":"J10"}));
    assert_eq!(value(&h, 0, "G1"), Value::Empty);
    assert_eq!(value(&h, 0, "J10"), Value::number(7.0));
    run(&mut h, "edit.cut", json!({"range":"B2:C3"}));
    run(&mut h, "edit.paste", json!({"at":"M10"}));
    assert!(h.state().session.active().unwrap().wb.active().unwrap().merges.iter().any(|r| r.a1() == "M10:N11"));
    run(&mut h, "edit.cut", json!({"range":"M10:N11"}));
    run(&mut h, "edit.paste", json!({"at":"N11"}));
    assert!(h.state().session.active().unwrap().wb.active().unwrap().merges.iter().any(|r| r.a1() == "N11:O12"));
}

#[test]
fn combination_06_live_view_context_action_and_restore() {
    let mut h = harness();
    h.state_mut().open_dialog("displayCustomization", json!({}));
    h.run_steps(3);
    for (label, mode) in [("Light", "light"), ("Dark", "dark"), ("Auto (match device Light/Dark)", "system")] {
        h.get_by_label(Language::Ja.tr(label)).click();
        h.run_steps(2);
        assert_eq!(h.state().ui.theme_mode(), mode);
    }
    for (label, mode) in [
        ("Standard ribbon", RibbonMode::Standard),
        ("Compact ribbon", RibbonMode::Compact),
        ("Hide ribbon (use right-click menu)", RibbonMode::ContextMenuOnly),
    ] {
        h.get_by_label(Language::Ja.tr(label)).click();
        h.run_steps(2);
        assert_eq!(h.state().ui.ribbon_mode, mode);
    }
    // Checkbox order follows HOME_GROUPS: number is fourth, editing is seventh.
    h.get_all_by_label(Language::Ja.tr("Pin first")).nth(6).unwrap().click();
    h.run_steps(2);
    h.get_all_by_label(Language::Ja.tr("Pin first")).nth(3).unwrap().click();
    h.run_steps(2);
    assert_eq!(h.state().ui.pinned_ribbon_groups, ["editing", "number"]);
    h.get_by_label(Language::Ja.tr("Styles")).click();
    h.run_steps(2);
    assert!(h.state().ui.hidden_ribbon_groups.contains(&"styles".into()));
    run(&mut h, "view.interfaceScale", json!({"percent":125}));
    h.run_steps(3);
    assert_eq!(h.ctx.zoom_factor(), 1.25);
    h.get_by_label(Language::Ja.tr("Close")).click();
    h.run_steps(2);
    let app = h.state();
    let sheet = app.session.active().unwrap().wb.active().unwrap();
    let pos = grid::Geo::new(sheet, app.grid.rect.unwrap(), app.view().scroll).cell_rect(sheet, CellRef::parse("C5").unwrap()).center();
    h.input_mut().events.extend([
        Event::PointerMoved(pos),
        Event::PointerButton { pos, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE },
    ]);
    h.run_steps(3);
    assert!(h.state().grid.context_menu.is_some());
    h.get_by_label(Language::Ja.tr("Copy")).click();
    h.run_steps(2);
    assert!(h.state().session.clipboard.is_some());
    h.get_by_label(Language::Ja.tr("Restore Normal View")).click();
    h.run_steps(2);
    assert!(!h.state().ui.is_customized_view());
    h.run_steps(1); // egui applies the restored scale at the next frame boundary.
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    assert_eq!(h.state().ui.theme_mode(), "system", "restoring layout preserves the user's theme preference");
}

#[test]
fn combination_07_japanese_custom_dialogs_and_notifications() {
    let mut h = harness();
    run(&mut h, "range.setValues", json!({"range":"A1:B3","values":[["項目","値"],["一",10],["二",20]]}));
    run(&mut h, "selection.set", json!({"range":"A1:B3"}));
    let chart = run(&mut h, "insert.chart", json!({"type":"column","range":"A1:B3"}))["chart"].clone();
    for (dialog, params, labels) in [
        ("find", json!({}), vec!["Find what:", "Match case", "Find All", "Find Next"]),
        ("find", json!({"replace":true}), vec!["Replace with:", "Replace All"]),
        ("goTo", json!({}), vec!["Reference:", "Special…"]),
        ("sort", json!({}), vec!["My list has headers", "Sort by", "Cell Values", "A to Z", "+ Add Level"]),
        ("nameManager", json!({}), vec!["New…", "Edit…", "Name", "Value", "Refers To", "Scope", "No names defined"]),
        ("recommendedCharts", json!({}), vec!["Recommended Charts", "All Charts"]),
        ("recommendedCharts", json!({"tab":"All Charts"}), vec!["Clustered Column", "Stacked Column"]),
        ("selectData", json!({"chart":chart}), vec!["Chart data range:", "Legend Entries (Series)", "Horizontal (Category) Axis Labels:"]),
        ("manageRules", json!({}), vec!["There are no conditional formatting rules on this sheet."]),
        ("goalSeek", json!({}), vec!["Set cell:", "To value:", "By changing cell:"]),
        ("analysisToolPak", json!({}), vec!["Analysis Tools", "Descriptive Statistics"]),
        ("analysisToolPak", json!({"tool":"descriptive"}), vec!["Input Range:", "Summary statistics", "Output options", "New Worksheet Ply:"]),
        ("solver", json!({}), vec!["Set Objective:", "To:", "Max", "Min", "Make Unconstrained Variables Non-Negative", "Solver|Add", "Solve"]),
        (
            "solverResults",
            json!({"message":"Solver found an optimal solution. All constraints are satisfied.","changed":true,"values":{"A1":1}}),
            vec!["Keep Solver Solution", "Restore Original Values", "Reports:", "Answer", "Limits"],
        ),
        ("spelling", json!({}), vec!["The spelling check is complete for the entire sheet."]),
        ("saveChanges", json!({}), vec!["Don't Save", "Cancel", "Save"]),
        ("saveCopy", json!({"format":"xlsx"}), vec!["Cancel"]),
        (
            "insertPicture",
            json!({"placement":"cell"}),
            vec!["Picture file path (PNG or JPEG):", "Description (optional):", "Fits inside the selected cell and moves with its content."],
        ),
        ("displayCustomization", json!({}), vec!["Display theme", "Standard ribbon", "Compact ribbon", "Restore Normal View"]),
    ] {
        h.state_mut().dialog = None;
        h.state_mut().open_dialog(dialog, params);
        h.run_steps(3);
        assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some(dialog));
        let title = &h.state().dialog.as_ref().unwrap().title;
        assert_ne!(Language::Ja.tr(title), title, "untranslated dialog heading: {dialog}");
        assert!(h.query_all_by_label(Language::Ja.tr(title)).next().is_some(), "missing localized heading: {dialog}");
        for label in labels {
            let translated = Language::Ja.tr(label);
            assert_ne!(label, translated, "missing dictionary entry: {label}");
            assert!(
                h.query_all_by(|n| n.label().is_some_and(|s| s.trim() == translated) || n.value().is_some_and(|s| s.trim() == translated))
                    .next()
                    .is_some(),
                "{dialog}: missing translated control {translated}"
            );
            assert!(
                h.query_all_by_label(label).next().is_none() && h.query_all_by_value(label).next().is_none(),
                "{dialog}: English remains: {label}"
            );
        }
        assert!(h.state().message.is_none(), "{dialog}: unexpected error {:?}", h.state().message);
    }
    h.state_mut().dialog = None;
    h.state_mut().open_dialog("find", json!({"replace":true}));
    h.run_steps(3);
    h.state_mut().dialog.as_mut().unwrap().values.insert("what".into(), json!("一"));
    h.state_mut().dialog.as_mut().unwrap().values.insert("with".into(), json!("三"));
    h.get_by_label(Language::Ja.tr("Replace All")).click();
    h.run_steps(2);
    assert_eq!(value(&h, 0, "A2"), Value::from("三"));
    assert!(h.state().toast.as_ref().unwrap().0.contains("1 件"));
    assert!(h.query_by_label_contains("1 件を置換").is_some());
    h.state_mut().dialog.as_mut().unwrap().values.insert("what".into(), json!("三"));
    h.get_by_label(Language::Ja.tr("Find All")).click();
    h.run_steps(2);
    assert!(h.query_by_label("1 個のセルが見つかりました").is_some());
    h.state_mut().dialog = None;
    h.state_mut().open_dialog("spelling", json!({}));
    h.state_mut().dialog.as_mut().unwrap().result = Some(json!({"issues":[{"cell":"A2","word":"helllo","suggestions":["hello"]}]}));
    h.state_mut().dialog.as_mut().unwrap().values.insert("pick".into(), json!("hello"));
    h.run_steps(3);
    for label in ["Suggestions:", "Ignore Once", "Add to Dictionary", "Change", "Change All"] {
        assert!(h.query_by_label(Language::Ja.tr(label)).is_some(), "{label}");
        assert!(h.query_by_label(label).is_none());
    }
    assert!(h.query_by_label("辞書にない単語 (A2):").is_some());
    assert!(h.query_by_label("helllo").is_some(), "user words stay literal");
    h.state_mut().dialog = None;
    h.state_mut().services.pick_picture_async = Some(Box::new(|_| {}));
    h.state_mut().open_dialog("insertPicture", json!({}));
    h.run_steps(3);
    assert!(h.query_by_label(Language::Ja.tr("No picture selected")).is_some());
    assert!(h.query_by_label(Language::Ja.tr("Browse…")).is_some());
}

#[test]
fn combination_08_mini_vba_presets_combined_macro_and_errors() {
    for (label, check_cell, expected) in [
        ("Insert summary example", "E1", Value::from("Summary")),
        ("Format table example", "B2", Value::number(120.0)),
        ("Highlight rows example", "D2", Value::number(1.0)),
    ] {
        let mut h = harness();
        run(&mut h, "cell.set", json!({"cell":"B2","input":"120"}));
        h.state_mut().open_dialog("vbaEditor", json!({}));
        h.run_steps(3);
        h.get_by_label(Language::Ja.tr(label)).click();
        h.run_steps(2);
        h.get_by_label(Language::Ja.tr("Run VBA")).click();
        h.run_steps(3);
        let status = run(&mut h, "automate.vbaStatus", json!({}));
        assert_eq!(status["state"], "completed", "{label}: {status}");
        assert_eq!(value(&h, 0, check_cell), expected);
        if label == "Format table example" {
            let doc = h.state().session.active().unwrap();
            assert!(doc.wb.styles.get(doc.wb.active().unwrap().style_id(CellRef::parse("A1").unwrap())).font.bold);
            assert!(status["log"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("[MsgBox]")));
        }
    }
    let mut h = harness();
    let source = r#"Sub Main()
    Dim c As Range
    Worksheets.Add
    Range("A1").Value = 10
    Range("B1").Value = 20
    Range("A2").Value = 30
    Range("B2").Value = 40
    Range("D1:E2").Value = Range("A1:B2").Value
    For Each c In Range("D1:E2")
        c.Font.Bold = True
    Next c
    Range("D1:E2").Borders.LineStyle = xlContinuous
    MsgBox "完了"
End Sub"#;
    run(&mut h, "automate.startVba", json!({"source":source,"procedure":"Main"}));
    let status = run(&mut h, "automate.pollVba", json!({"budget":10000}));
    assert_eq!(status["state"], "completed", "{status}");
    assert_eq!(h.state().session.active().unwrap().wb.sheets.len(), 2);
    assert_eq!(value(&h, 1, "E2"), Value::number(40.0));
    assert_eq!(status["log"], json!(["[MsgBox] 完了"]));
    for source in ["Sub Main()\nRange(\"A1\").Value =\nEnd Sub", "Sub Main()\nRange(\"A1\").Value = 99\nRange(\"B1\").Value = 1 / 0\nEnd Sub"] {
        let before = h.state().session.active().unwrap().wb.clone();
        let result = h.state_mut().run("automate.startVba", json!({"source":source,"procedure":"Main"}));
        let message = match result {
            Err(e) => e,
            Ok(_) => {
                let failed = run(&mut h, "automate.pollVba", json!({"budget":10000}));
                assert_eq!(failed["state"], "failed", "{failed}");
                failed["error"]["message"].as_str().unwrap().to_owned()
            }
        };
        h.state_mut().message = Some(("VBA".into(), message));
        assert_alert(&mut h);
        assert!(Arc::ptr_eq(&before, &h.state().session.active().unwrap().wb));
    }
}

#[test]
fn combination_09_roundtrip_and_japanese_macro_notice() {
    let mut h = harness();
    run(&mut h, "range.setValues", json!({"range":"A1:B3","values":[["項目","金額"],["一",10],["二",20]]}));
    run(&mut h, "cell.set", json!({"cell":"B4","input":"=SUM(B2:B3)"}));
    run(&mut h, "home.bold", json!({"range":"A1:B1"}));
    run(&mut h, "insert.chart", json!({"range":"A1:B3","type":"column"}));
    let source = "Sub Main()\nRange(\"A1\").Value = 999\nEnd Sub";
    run(&mut h, "automate.saveVbaModule", json!({"name":"Module1","source":source}));
    let bytes = run(&mut h, "file.saveBytes", json!({"format":"xlsx","name":"検証.xlsx"}));
    assert!(!base64_decode(bytes["base64"].as_str().unwrap()).unwrap().is_empty());
    run(&mut h, "file.open", json!({"name":"検証.xlsx","base64":bytes["base64"]}));
    assert_eq!(value(&h, 0, "B4"), Value::number(30.0));
    let doc = h.state().session.active().unwrap();
    assert_eq!(doc.wb.active().unwrap().charts.len(), 1);
    assert!(doc.wb.styles.get(doc.wb.active().unwrap().style_id(CellRef::parse("A1").unwrap())).font.bold);
    assert_eq!(run(&mut h, "automate.vbaModules", json!({}))["modules"][0]["source"], source);
    // Synthetic opaque project bytes exercise transport/notice, not Excel executable VBA generation.
    let project = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1 synthetic VBA project".to_vec();
    Arc::make_mut(&mut h.state_mut().session.active_mut().unwrap().wb).vba_project = Some(Arc::new(project.clone()));
    let bytes = run(&mut h, "file.saveBytes", json!({"format":"xlsm","name":"検証.xlsm"}));
    let result = run(&mut h, "file.open", json!({"name":"検証.xlsm","base64":bytes["base64"]}));
    assert_eq!(h.state().session.active().unwrap().wb.vba_project.as_deref(), Some(&project));
    h.state_mut().show_open_warnings(&result);
    h.run_steps(2);
    assert!(h.query_by_label_contains("元のプロジェクトが保持").is_some());
    assert!(h.query_by_label("詳細を確認").is_some());
    assert!(h.query_by_label("What this means").is_none());
    // The distinct dropped-project warning must never promise preservation.
    h.state_mut().show_open_warnings(&json!({"warnings":["VBA macros dropped"]}));
    h.run_steps(2);
    assert!(h.query_by_label_contains("元のファイルを保管").is_some());
    assert!(h.query_by_label_contains("元のプロジェクトが保持").is_none());
    h.state_mut().show_open_warnings(&json!({"warnings":[]}));
    assert!(h.state().notice.is_none());
}

#[test]
fn combination_10_no_english_error_popups_in_japanese_mode() {
    let mut h = harness();
    // Known modules and platform failures, plus an unknown future error. Identifiers remain literal.
    let errors = [
        "no workbook open",
        "No workbook is open.",
        "invalid base64",
        "Incorrect password.",
        "nothing to undo",
        "nothing to redo",
        "the clipboard is empty",
        "Nothing to paste.",
        "paste destinations overlap",
        "a paste destination extends past the sheet edge",
        "selection too large",
        "selection too large to format cell by cell",
        "missing `size`",
        "missing or invalid `color`",
        "no such sheet",
        "no sheet named `売上`",
        "invalid cell reference `ZZ0` for `cell`",
        "invalid range `??`",
        "invalid range",
        "The cell or chart you're trying to change is on a protected sheet.",
        "Formatting cells isn't allowed on this protected sheet (Review › Unprotect Sheet).",
        "The range \"担当者\" needs its password before it can be changed (Review › Allow Edit Ranges).",
        "The password you supplied is not correct.",
        "The workbook is protected and cannot be changed.",
        "A workbook must contain at least one visible worksheet.",
        "A table can't overlap another table.",
        "The name that you entered is not valid.",
        "The name entered already exists. Enter a unique name.",
        "To do this, all the merged cells need to be the same size.",
        "You can't change part of an array.",
        "This value doesn't match the data validation restrictions defined for this cell.",
        "Select a chart first.",
        "column outside the filter range",
        "missing `what`",
        "missing `reference`",
        "There's a problem with this formula: unexpected end of formula",
        "SUM has too few arguments. Use SUM(number1, [number2])",
        "Select an available printer in File > Print before submitting.",
        "The print command did not produce a PDF.",
        "Couldn't decode the generated PDF.",
        "A print job is already being submitted.",
        "Printing is unavailable here; export a PDF instead.",
        "Exporting needs a file picker, which this window doesn't have.",
        "Solver could not find a solution that satisfies all constraints.",
        "The objective can grow without limit: the problem is unbounded.",
        "goal seek did not converge",
        "Variable not defined: example",
        "Procedure not found: Main",
        "division by zero",
        "subscript out of range",
        "macro operation limit exceeded",
        "VBA member or function `Foo` is not implemented",
        "invalid parameters for `cell.set`: no such sheet",
        "internal error in `cell.set` (the workbook was kept as it was): unexpected future failure",
        "command `edit.undo` is not available right now: nothing to undo",
        "Unknown future error with ASCII only",
        "Unknown error with fullwidth punctuation！",
    ];
    assert!(errors.len() >= 40);
    for error in errors {
        let translated = Language::Ja.tr_error(error);
        assert!(japanese(&translated), "{error} -> {translated}");
        assert!(!translated.contains("invalid parameters for") && !translated.contains("workbook was kept"));
        h.state_mut().message = Some(("Rustani".into(), error.into()));
        h.run_steps(2);
        assert!(h.query_by_label(&translated).is_some(), "actual popup: {error}");
        assert!(h.query_by_label(error).is_none(), "raw error leaked: {error}");
        h.state_mut().message = None;
    }
    assert_eq!(Language::Ja.tr_error("no sheet named `売上`"), "シート `売上` が見つかりません。");
    assert_eq!(Language::En.tr_error("Unknown future error"), "Unknown future error");
    assert_eq!(Language::Ja.tr_error("入力内容を確認してください。"), "入力内容を確認してください。");
    // Saving and user content are never passed through the UI translation safety net.
    let text = "User text | Keep ASCII";
    run(&mut h, "cell.set", json!({"cell":"A1","input":text}));
    assert_eq!(value(&h, 0, "A1"), Value::from(text));
    assert_eq!(base64_decode(&base64_encode(text.as_bytes())).unwrap(), text.as_bytes());
}
