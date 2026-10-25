//! Web Save As names a download without inventing a local path or claiming it was written.

use serde_json::{Value as Json, json};

use crate::SheetApp;

pub(crate) fn suggested_name(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        return "Book1.xlsx".into();
    }
    if gridcraft_engine::io::FileKind::from_path(title).is_some() {
        return std::path::Path::new(title).with_extension("xlsx").to_string_lossy().into_owned();
    }
    workbook_name(title)
}

fn workbook_name(name: &str) -> String {
    if name.get(name.len().saturating_sub(5)..).is_some_and(|ext| ext.eq_ignore_ascii_case(".xlsx")) {
        name.to_string()
    } else {
        format!("{name}.xlsx")
    }
}

pub(crate) fn download(app: &mut SheetApp, params: &Json) -> Result<Json, String> {
    let doc = app.session.active().ok_or("No workbook is open.")?;
    if params.get("document").is_some_and(|value| value.as_u64() != Some(doc.uid)) {
        return Err("Return to the original workbook to save it, or cancel and open Save As again.".into());
    }
    let name = params.get("name").and_then(Json::as_str).unwrap_or_default().trim();
    if name.is_empty() {
        return Err("Enter a file name.".into());
    }
    if app.services.download.is_none() {
        return Err("File downloads are unavailable in this window.".into());
    }
    let name = workbook_name(name);
    let result = app.run("file.saveBytes", json!({"format": "xlsx", "name": name}))?;
    let bytes = result.get("base64").and_then(Json::as_str).and_then(gridcraft_engine::io::base64_decode).ok_or("Couldn't decode the workbook.")?;
    let download = app.services.download.as_ref().ok_or("File downloads are unavailable in this window.")?;
    download(&name, &bytes);
    if let Some(doc) = app.session.active_mut() {
        doc.title.clone_from(&name);
    }
    // Browser pickers may still be pending or cancelled. Only the host's existing acknowledgement
    // can mark a document saved; a standalone download keeps its dirty flag and has no local path.
    Ok(json!({"name": name, "bytes": bytes.len(), "download": true}))
}
