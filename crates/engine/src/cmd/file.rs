//! File menu: new, open, save, close, export, properties, options.

use serde_json::{Value as Json, json};

use super::*;
use crate::DocState;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(noundo "file.new", "New Workbook", ["File"], Some("Cmd+N"), "{sample?: \"budget\"|\"sales\"|\"grades\"}", always, new_workbook),
        cmd!(noundo "file.open", "Open…", ["File"], Some("Cmd+O"), "{path} | {name, base64}, plus {password} for a password-protected workbook and {csvAsText?: bool} to keep every csv/tsv/txt field as text, with no number or date conversion (xlsx, xlsm, xls import, xlsb and ods data import, csv, tsv, txt, json)", always, open),
        cmd!(noundo "file.save", "Save", ["File"], Some("Cmd+S"), "{path?} (xlsx by default; .csv/.tsv/.json/.html/.pdf by extension)", has_doc, save),
        cmd!(noundo "file.saveAs", "Save As…", ["File"], Some("Cmd+Shift+S"), "{path} (.xlsx, .csv, .tsv, .json, .html, or .pdf: a PDF of the active sheet)", has_doc, save_as),
        cmd!(query "file.saveBytes", "Encode Workbook", [], None, "{format?: xlsx|csv|tsv|json|html, name?: basename (255 UTF-8 bytes including extension; missing extension uses format)} → {name, base64, bytes}; no document state changes", has_doc, save_bytes),
        cmd!(noundo "file.close", "Close", ["File"], Some("Cmd+W"), "{force?: bool}", has_doc, close),
        cmd!(noundo "file.exportCsv", "Export as CSV", ["File", "Export"], None, "{path, sheet?}", has_doc, |s, p| export(s, p, "csv")),
        cmd!(noundo "file.exportHtml", "Save as Web Page", ["File", "Export"], None, "{path}", has_doc, |s, p| export(s, p, "html")),
        cmd!("file.properties", "Properties", ["File"], None, "{title?, subject?, author?, company?, keywords?, description?}", has_doc, properties),
        cmd!(
            query "file.options",
            "Options",
            ["File"],
            None,
            "{} → {calculation: {mode, iterative, maxIterations, maxChange}} for the active workbook (the interface opens File › Options; change settings with their own commands, e.g. formulas.calculationOptions, view.theme, app.language.set)",
            always,
            options
        ),
        cmd!(noundo "window.activate", "Switch Window", ["Window"], None, "{index}", has_doc, activate_window),
    ]
}

fn new_workbook(s: &mut Session, p: &Json) -> Result<Json> {
    let i = match str_param(p, "sample") {
        Some(name) => {
            let wb = crate::sample::build(name).ok_or_else(|| bad("file.new", format!("unknown sample `{name}`")))?;
            let title = crate::sample::title(name);
            s.add_document(DocState::new(wb, None, title))
        }
        None => s.new_workbook(),
    };
    Ok(json!({"index": i, "title": s.doc()?.display_title()}))
}

fn open(s: &mut Session, p: &Json) -> Result<Json> {
    let (name, bytes) = if let Some(b) = str_param(p, "base64") {
        (str_param(p, "name").unwrap_or("Book.xlsx").to_string(), crate::io::base64_decode(b).ok_or_else(|| bad("file.open", "invalid base64"))?)
    } else if let Some(path) = str_param(p, "path") {
        // Already open? Switch to it.
        if let Some(i) = s.documents().iter().position(|d| d.path.as_deref() == Some(path)) {
            s.set_active(i);
            return Ok(json!({"index": i, "alreadyOpen": true}));
        }
        (path.to_string(), crate::io::read_file(path)?)
    } else {
        s.ui_requests.push(crate::UiRequest::Dialog("open".into(), json!({})));
        return ok();
    };
    let encrypted = gridcraft_xlsx::sniff(&bytes) == gridcraft_xlsx::Format::Encrypted;
    let (mut wb, warnings) = crate::io::open_bytes_with(&name, &bytes, str_param(p, "password"), bool_param(p, "csvAsText").unwrap_or(false))?;
    wb.source_dir = str_param(p, "path").and_then(crate::io::source_dir);
    // What `CELL("filename")` reports (#1489). A workbook opened from bytes knows its name but
    // never a folder, so it reports `[name]Sheet` with none; an import reports the file it came
    // from, as Excel does, even though Save must still ask where to put the .xlsx.
    wb.source_path = match str_param(p, "path") {
        Some(path) => crate::io::source_path(path),
        None => Some(std::path::PathBuf::from(&name)),
    };
    let imported =
        matches!(crate::io::FileKind::from_path(&name), Some(crate::io::FileKind::Ods | crate::io::FileKind::Xlsb | crate::io::FileKind::Xls))
            || matches!(gridcraft_xlsx::sniff(&bytes), gridcraft_xlsx::Format::Ods | gridcraft_xlsx::Format::Xlsb | gridcraft_xlsx::Format::Xls);
    let title = if imported {
        format!("{} (imported).xlsx", std::path::Path::new(&name).file_stem().and_then(|n| n.to_str()).unwrap_or("Book"))
    } else {
        std::path::Path::new(&name).file_name().and_then(|n| n.to_str()).unwrap_or("Book").to_string()
    };
    // An import has no save target: ordinary Save must ask for a new destination. Neither does a
    // workbook decrypted with its password, so Save never replaces the encrypted original with
    // an unprotected copy.
    let path = str_param(p, "path").filter(|_| !imported && !encrypted).map(str::to_string);
    // Replace an untouched blank Book1 like Excel does.
    if s.documents().len() == 1
        && s.active().is_some_and(|d| d.path.is_none() && !d.is_dirty() && d.undo.is_empty() && d.wb.sheets.iter().all(|sh| sh.cells.is_empty()))
    {
        s.close_document(0);
    }
    let mut doc = DocState::new(wb, path, title);
    if crate::io::FileKind::from_path(&name).is_none() && gridcraft_xlsx::sniff(&bytes) == gridcraft_xlsx::Format::Xlsx {
        doc.content_kind = Some(crate::io::FileKind::Xlsx);
    }
    let i = s.add_document(doc);
    // The decrypted-workbook notice is for the info bar, not a message box.
    let shown: Vec<&str> = warnings.iter().map(String::as_str).filter(|w| *w != crate::io::DECRYPTED_NOTICE).collect();
    if !shown.is_empty() {
        let mut message = shown.iter().take(5).copied().collect::<Vec<_>>().join("\n\n");
        if shown.len() > 5 {
            message.push_str(&format!("\n\n{} additional import warnings.", shown.len() - 5));
        }
        s.ui_requests.push(crate::UiRequest::Message(message));
    }
    let d = s.doc()?;
    Ok(
        json!({"index": i, "title": d.display_title(), "sheets": d.wb.sheets.iter().map(|s| s.name.clone()).collect::<Vec<_>>(), "warnings": warnings}),
    )
}

fn save(s: &mut Session, p: &Json) -> Result<Json> {
    let path = match str_param(p, "path").map(str::to_string).or_else(|| s.active().and_then(|d| d.path.clone())) {
        Some(p) => p,
        None => {
            s.ui_requests.push(crate::UiRequest::Dialog("saveAs".into(), json!({})));
            return ok();
        }
    };
    // A PDF is a print of the sheet (Page Setup applies), not a copy of the workbook: the
    // document keeps its file and its unsaved changes.
    if std::path::Path::new(&path).extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
        return super::print::export_pdf(s, &json!({"path": path}));
    }
    let d = s.doc_mut()?;
    // The document's own file keeps the format its content had when opened; any other path
    // goes by its extension. Relative paths to linked workbooks are rewritten for the new folder.
    let kind = crate::io::FileKind::from_path(&path).or(d.content_kind.filter(|_| d.path.as_deref() == Some(path.as_str())));
    let rebased = crate::io::source_dir(&path).and_then(|f| gridcraft_xlsx::rebase_links(&d.wb, &f));
    let wb = rebased.as_ref().unwrap_or(&d.wb);
    let bytes = match kind {
        Some(kind) => crate::io::save_bytes_as(wb, &path, kind)?,
        None => crate::io::save_bytes(wb, &path)?,
    };
    crate::io::write_file(&path, &bytes)?;
    // Only a full-fidelity format clears the dirty flag and becomes the document's file.
    if matches!(kind, Some(crate::io::FileKind::Xlsx | crate::io::FileKind::Json)) {
        // The new file is the workbook's own now, so `CELL("filename")` reports it (#1489). Set
        // before the `saved` snapshot, or the workbook would read as dirty the moment it was
        // saved; and only when it changes, so an ordinary Save does not clone the workbook.
        let file = crate::io::source_path(&path);
        if d.wb.source_path != file {
            std::sync::Arc::make_mut(&mut d.wb).source_path = file;
        }
        d.saved = d.wb.clone();
        d.path = Some(path.clone());
    }
    Ok(json!({"path": path, "bytes": bytes.len()}))
}

fn save_as(s: &mut Session, p: &Json) -> Result<Json> {
    if str_param(p, "path").is_none() {
        s.ui_requests.push(crate::UiRequest::Dialog("saveAs".into(), json!({})));
        return ok();
    }
    save(s, p)
}

fn save_bytes(s: &mut Session, p: &Json) -> Result<Json> {
    let fmt = str_param(p, "format").unwrap_or("xlsx");
    let d = s.doc()?;
    let name = match p.get("name") {
        None => format!("{}.{fmt}", std::path::Path::new(&d.display_title()).file_stem().and_then(|x| x.to_str()).unwrap_or("Book")),
        Some(value) => export_name(value.as_str().ok_or_else(|| bad("file.saveBytes", "The name parameter must be a string."))?, fmt)?,
    };
    // The file name goes into a PivotChart's link to its PivotTable.
    let bytes = crate::io::save_bytes(&d.wb, &name)?;
    Ok(json!({"name": name, "base64": crate::io::base64_encode(&bytes), "bytes": bytes.len()}))
}

/// A download is a named byte payload, never a filesystem path. Validate its name before
/// encoding, and pass the same name to the writer (PivotChart links include the book name).
fn export_name(name: &str, format: &str) -> Result<String> {
    const MAX_NAME_BYTES: usize = 255;
    if name.trim().is_empty() {
        return Err(bad("file.saveBytes", "File name must not be empty."));
    }
    if matches!(name, "." | "..") || name.chars().any(|ch| matches!(ch, '/' | '\\') || ch.is_control()) {
        return Err(bad("file.saveBytes", "File name must be a basename without folders or control characters."));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(bad("file.saveBytes", "File name must be at most 255 UTF-8 bytes, including the extension."));
    }
    // Do not let an untrusted format add another path or an arbitrarily large extension. The
    // format-only path above retains its existing behavior; named exports require a real format.
    if format.len() > 16
        || !format.bytes().all(|ch| ch.is_ascii_alphanumeric())
        || crate::io::FileKind::from_path(&format!("export.{format}")).is_none()
    {
        return Err(bad("file.saveBytes", "Unknown export format."));
    }
    let result = match std::path::Path::new(name).extension().and_then(|ext| ext.to_str()) {
        None => format!("{name}.{format}"),
        Some(ext) if ext.eq_ignore_ascii_case(format) => name.to_string(),
        // XLSX and XLSM share a FileKind but have different package/VBA semantics. Require the
        // exact requested extension rather than treating those containers as interchangeable.
        Some(_) => return Err(bad("file.saveBytes", "File name extension must match the requested format.")),
    };
    if result.len() > MAX_NAME_BYTES {
        return Err(bad("file.saveBytes", "File name must be at most 255 UTF-8 bytes, including the extension."));
    }
    Ok(result)
}

fn close(s: &mut Session, p: &Json) -> Result<Json> {
    let force = bool_param(p, "force").unwrap_or(false);
    let d = s.doc()?;
    if d.is_dirty() && !force {
        s.ui_requests.push(crate::UiRequest::Dialog("saveChanges".into(), json!({"title": d.display_title()})));
        return Ok(json!({"closed": false, "dirty": true}));
    }
    let i = s.active_index();
    s.close_document(i);
    Ok(json!({"closed": true}))
}

fn export(s: &mut Session, p: &Json, ext: &str) -> Result<Json> {
    let path = str_param(p, "path").ok_or_else(|| bad("file.export", "missing `path`"))?.to_string();
    let d = s.doc()?;
    let mut wb = (*d.wb).clone();
    if p.get("sheet").is_some() {
        wb.active_sheet = target_sheet(s, p)?;
    }
    let target = if path.ends_with(&format!(".{ext}")) { path.clone() } else { format!("{path}.{ext}") };
    let bytes = crate::io::save_bytes(&wb, &target)?;
    crate::io::write_file(&target, &bytes)?;
    Ok(json!({"path": target, "bytes": bytes.len()}))
}

fn properties(s: &mut Session, p: &Json) -> Result<Json> {
    edit(s, |cx| {
        let pr = &mut cx.wb.props;
        for (k, slot) in [
            ("title", &mut pr.title),
            ("subject", &mut pr.subject),
            ("author", &mut pr.author),
            ("company", &mut pr.company),
            ("keywords", &mut pr.keywords),
            ("description", &mut pr.description),
        ] {
            if let Some(v) = str_param(p, k) {
                *slot = v.to_string();
            }
        }
        Ok(serde_json::to_value(&cx.wb.props).unwrap_or_default())
    })
}

/// File › Options: reports the workbook-level settings and asks the interface to show its Options
/// page. Nothing changes here; every setting has its own command.
fn options(s: &mut Session, _: &Json) -> Result<Json> {
    s.ui_requests.push(crate::UiRequest::Dialog("options".into(), json!({})));
    let calculation = s.active().map(|d| {
        let c = &d.wb.calc;
        let mode = match c.mode {
            gridcraft_model::CalcMode::Automatic => "automatic",
            gridcraft_model::CalcMode::AutomaticExceptTables => "automaticExceptTables",
            gridcraft_model::CalcMode::Manual => "manual",
        };
        json!({"mode": mode, "iterative": c.iterative, "maxIterations": c.max_iterations, "maxChange": c.max_change})
    });
    Ok(json!({"calculation": calculation}))
}

fn activate_window(s: &mut Session, p: &Json) -> Result<Json> {
    let i = u32_param(p, "index").ok_or_else(|| bad("window.activate", "missing `index`"))? as usize;
    if i >= s.documents().len() {
        return Err(bad("window.activate", "no such window"));
    }
    s.set_active(i);
    ok()
}

#[cfg(test)]
mod tests {
    use std::{io::Read, sync::Arc};

    use super::*;

    fn session() -> Session {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("cell.set", json!({"cell":"A1","input":"42"})).unwrap();
        s
    }

    fn assert_document_unchanged(s: &Session, before: &DocState) {
        let d = s.doc().unwrap();
        assert!(Arc::ptr_eq(&d.wb, &before.wb));
        assert!(Arc::ptr_eq(&d.saved, &before.saved));
        assert_eq!(d.path, before.path);
        assert_eq!(d.title, before.title);
        assert_eq!(d.content_kind, before.content_kind);
        assert_eq!(d.selection, before.selection);
        assert_eq!(d.revision, before.revision);
        assert_eq!(d.undo.len(), before.undo.len());
        assert_eq!(d.redo.len(), before.redo.len());
        assert_eq!(d.is_dirty(), before.is_dirty());
    }

    fn bytes(result: &Json) -> Vec<u8> {
        crate::io::base64_decode(result["base64"].as_str().unwrap()).unwrap()
    }

    #[test]
    fn named_export_uses_the_same_unicode_name_in_the_payload_and_pivot_chart_links() {
        let mut s = session();
        s.execute("range.setValues", json!({"range":"A1","values":[["Region","Sales"],["East",10],["West",20]]})).unwrap();
        s.execute("selection.set", json!({"cell":"A1"})).unwrap();
        s.execute("insert.pivotTable", json!({"rows":["Region"],"values":["Sales"]})).unwrap();
        s.execute("insert.pivotChart", json!({})).unwrap();
        let before = s.doc().unwrap().clone();
        let journal_len = s.journal.len();
        let result = s.execute("file.saveBytes", json!({"format":"xlsx","name":"2026年度売上"})).unwrap();
        assert_eq!(result["name"], "2026年度売上.xlsx");
        let bytes = bytes(&result);
        assert_eq!(result["bytes"].as_u64(), Some(bytes.len() as u64));
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())).unwrap();
        let mut chart = String::new();
        zip.by_name("xl/charts/chart1.xml").unwrap().read_to_string(&mut chart).unwrap();
        assert!(chart.contains("<c:name>[2026年度売上.xlsx]Sheet2!PivotTable1</c:name>"), "{chart}");
        let (reopened, _) = crate::io::open_bytes("2026年度売上.xlsx", &bytes).unwrap();
        let pivot = reopened.sheets.iter().flat_map(|sh| sh.charts.iter()).find_map(|chart| chart.pivot.as_deref());
        assert_eq!(pivot, Some("Sheet2!PivotTable1"));
        assert_document_unchanged(&s, &before);
        assert_eq!(s.journal.len(), journal_len, "encoding remains a pure query");
    }

    #[test]
    fn named_export_rejects_paths_controls_bad_types_and_names_over_the_completed_byte_limit() {
        let mut s = session();
        let before = s.doc().unwrap().clone();
        for name in [
            "",
            "  \u{3000}  ",
            ".",
            "..",
            "dir/book.xlsx",
            "dir\\book.xlsx",
            "/book.xlsx",
            "book\n.xlsx",
            "book\0.xlsx",
            "book\u{7f}.xlsx",
            "book\u{85}.xlsx",
        ] {
            let err = s.execute("file.saveBytes", json!({"name":name})).unwrap_err();
            assert!(matches!(err, EngineError::BadParams { .. }), "{name:?}: {err}");
            assert_document_unchanged(&s, &before);
        }
        for name in [Json::Null, json!(12), json!(true), json!(["book.xlsx"]), json!({"name":"book.xlsx"})] {
            assert!(matches!(s.execute("file.saveBytes", json!({"name":name})), Err(EngineError::BadParams { .. })));
            assert_document_unchanged(&s, &before);
        }
        for name in ["x".repeat(251), format!("{}.xlsx", "あ".repeat(84)), "x".repeat(256)] {
            let error = s.execute("file.saveBytes", json!({"name":name})).unwrap_err().to_string();
            assert!(error.contains("255 UTF-8 bytes"), "{error}");
            assert_document_unchanged(&s, &before);
        }
        let name = "x".repeat(250);
        let result = s.execute("file.saveBytes", json!({"name":name})).unwrap();
        assert_eq!(result["name"].as_str().unwrap().len(), 255, "the extension is included in the limit");
        assert_document_unchanged(&s, &before);
    }

    #[test]
    fn format_only_exports_remain_compatible_and_named_formats_must_match_exactly() {
        let mut s = session();
        s.doc_mut().unwrap().title = "different display title".into();
        s.doc_mut().unwrap().path = Some("/kept/source.xlsx".into());
        let before = s.doc().unwrap().clone();
        for format in ["xlsx", "XLSX", "csv", "tsv", "txt", "json", "html"] {
            let result = s.execute("file.saveBytes", json!({"format":format})).unwrap();
            assert_eq!(result["name"], format!("source.{format}"));
            assert_document_unchanged(&s, &before);
        }
        let default = s.execute("file.saveBytes", json!({})).unwrap();
        assert_eq!(default["name"], "source.xlsx");
        let result = s.execute("file.saveBytes", json!({"format":"xlsx","name":"Budget.XLSX"})).unwrap();
        assert_eq!(result["name"], "Budget.XLSX");
        let csv = s.execute("file.saveBytes", json!({"format":"csv","name":"売上.csv"})).unwrap();
        assert_eq!(String::from_utf8(bytes(&csv)).unwrap().trim(), "42");
        for params in [
            json!({"format":"xlsx","name":"budget.csv"}),
            json!({"format":"xlsx","name":"macros.xlsm"}),
            json!({"format":"xlsm","name":"macros.xlsx"}),
            json!({"format":"json","name":"budget.xlsx"}),
            json!({"format":"bogus","name":"budget.bogus"}),
            json!({"format":"../xlsx","name":"budget.xlsx"}),
        ] {
            assert!(matches!(s.execute("file.saveBytes", params), Err(EngineError::BadParams { .. })));
            assert_document_unchanged(&s, &before);
        }
    }

    #[test]
    fn named_macro_export_keeps_the_existing_project_without_changing_document_identity() {
        let mut wb = gridcraft_model::Workbook::new();
        // Synthetic opaque bytes test preservation, not VBA execution or Excel compatibility.
        let project = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1 synthetic VBA project".to_vec();
        wb.vba_project = Some(Arc::new(project.clone()));
        let mut s = Session::new();
        s.add_document(DocState::new(wb, None, "Original.xlsm".into()));
        let before = s.doc().unwrap().clone();
        let result = s.execute("file.saveBytes", json!({"format":"xlsm","name":"保存するマクロ.xlsm"})).unwrap();
        assert_eq!(result["name"], "保存するマクロ.xlsm");
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes(&result))).unwrap();
        let mut saved_project = Vec::new();
        zip.by_name("xl/vbaProject.bin").unwrap().read_to_end(&mut saved_project).unwrap();
        assert_eq!(saved_project, project);
        let mut content_types = String::new();
        zip.by_name("[Content_Types].xml").unwrap().read_to_string(&mut content_types).unwrap();
        assert!(content_types.contains("macroEnabled.main+xml"));
        assert_document_unchanged(&s, &before);
        let result = s.execute("file.saveBytes", json!({"format":"xlsx","name":"通常ブック.xlsx"})).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes(&result))).unwrap();
        assert!(zip.by_name("xl/vbaProject.bin").is_err(), "the selected container must govern macro retention");
        assert_document_unchanged(&s, &before);
    }
}
