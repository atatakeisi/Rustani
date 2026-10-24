//! Bounded, context-aware recording. Only supported Excel object members are emitted;
//! unsupported operations stay visible as comments and in the stop result.

use gridcraft_core::{CellRef, RangeRef, Value};
use serde_json::{Value as Json, json};

use crate::{DocState, EngineError, Result, Session, vba::language::Program};

const MAX_STEPS: usize = 10_000;
const MAX_BYTES: usize = 512 * 1024;
const MAX_COMMAND_BYTES: usize = 4 * 1024 * 1024;
const MAX_CELLS: u64 = 10_000;

#[derive(Default)]
pub(crate) struct Recorder {
    owner: Option<u64>,
    commands: Vec<Json>,
    lines: Vec<String>,
    unsupported: Vec<String>,
    bytes: usize,
    command_bytes: usize,
    error: Option<String>,
}

fn bad(message: impl Into<String>) -> EngineError {
    super::bad("automate.recordActions", message)
}

pub(crate) fn begin(s: &mut Session) -> Result<Json> {
    let uid = s.doc()?.uid;
    s.recorder = Recorder { owner: Some(uid), ..Default::default() };
    s.recording = Some(s.journal.len());
    Ok(json!({"recording": true}))
}

pub(crate) fn capture(s: &mut Session, id: &str, params: &Json, before: Option<&DocState>) {
    if s.recording.is_none() || id.starts_with("automate.") || crate::cmd::find_command(id).is_none_or(|spec| !spec.journal) {
        return;
    }
    let Some(before) = before else { return };
    let Some(after) = s.active() else { return };
    if s.recorder.owner != Some(before.uid) || before.uid != after.uid {
        s.recorder.error = Some("Recording changed workbooks; return to the original workbook and cancel this recording.".into());
        return;
    }
    if s.recorder.error.is_some() {
        return;
    }
    let mut lines = translate(id, params, before, after).unwrap_or_default();
    let unsupported = lines.is_empty();
    if unsupported {
        // Command parameters may include secrets or arbitrary newlines: never put them in a comment.
        let safe_id: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_').take(128).collect();
        lines.push(format!("    ' Not recorded in VBA: {safe_id}"));
    }
    let command = json!({"command": id, "params": super::journal_params(params)});
    let command_bytes = serde_json::to_vec(&command).map_or(MAX_COMMAND_BYTES + 1, |v| v.len());
    let added = lines.iter().map(|line| line.len().saturating_add(1)).fold(0usize, usize::saturating_add);
    let recorder = &mut s.recorder;
    if recorder.commands.len() >= MAX_STEPS
        || recorder.bytes.saturating_add(added) > MAX_BYTES
        || recorder.command_bytes.saturating_add(command_bytes) > MAX_COMMAND_BYTES
    {
        recorder.error = Some("The macro recording exceeds its step or source size limit; cancel it and record a smaller macro.".into());
        return;
    }
    recorder.bytes += added;
    recorder.command_bytes += command_bytes;
    recorder.commands.push(command);
    recorder.lines.extend(lines);
    if unsupported && !recorder.unsupported.iter().any(|command| command == id) {
        recorder.unsupported.push(id.to_string());
    }
}

pub(crate) fn stop(s: &mut Session, params: &Json) -> Result<Json> {
    if params.get("cancel").and_then(Json::as_bool) == Some(true) {
        s.recording = None;
        s.recorder = Recorder::default();
        return Ok(json!({"recording": false, "cancelled": true}));
    }
    if s.recording.is_none() {
        return Ok(json!({"recording": false}));
    }
    if s.recorder.owner != Some(s.doc()?.uid) {
        return Err(bad("Return to the workbook where recording started before stopping the recording."));
    }
    if let Some(error) = &s.recorder.error {
        return Err(bad(error.clone()));
    }
    let modules = crate::vba::modules(s, &Json::Null)?;
    let items = modules.get("modules").and_then(Json::as_array).cloned().unwrap_or_default();
    let script_names = super::stash_map(&s.doc()?.wb, super::SCRIPTS_NAME);
    let requested = params.get("name").and_then(Json::as_str).unwrap_or("Macro1");
    if requested.is_empty() || requested.len() > 128 || requested.chars().any(char::is_control) {
        return Err(bad("Macro name must contain 1 to 128 bytes without control characters."));
    }
    let mut base: String = requested.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if !base.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        base.insert_str(0, "Macro_");
    }
    let reserved = [
        "and",
        "as",
        "byref",
        "byval",
        "call",
        "case",
        "class",
        "const",
        "dim",
        "do",
        "each",
        "else",
        "elseif",
        "empty",
        "end",
        "enum",
        "erase",
        "error",
        "exit",
        "explicit",
        "false",
        "for",
        "friend",
        "function",
        "get",
        "global",
        "goto",
        "if",
        "implements",
        "imp",
        "in",
        "is",
        "let",
        "loop",
        "lset",
        "me",
        "mod",
        "new",
        "next",
        "not",
        "nothing",
        "null",
        "on",
        "option",
        "or",
        "preserve",
        "print",
        "private",
        "property",
        "public",
        "redim",
        "rem",
        "resume",
        "return",
        "rset",
        "select",
        "set",
        "static",
        "step",
        "stop",
        "sub",
        "then",
        "to",
        "true",
        "type",
        "wend",
        "while",
        "with",
        "xor",
    ];
    if reserved.contains(&base.to_ascii_lowercase().as_str()) || Program::compile(&format!("Sub {base}()\nEnd Sub\n")).is_err() {
        base = "Macro1".into();
    }
    let mut used_names = std::collections::BTreeSet::new();
    for module in &items {
        if let Some(procedures) = module.get("procedures").and_then(Json::as_array) {
            for name in procedures.iter().filter_map(Json::as_str) {
                used_names.insert(name.to_ascii_lowercase());
            }
        }
    }
    if let Some(names) = script_names.as_object() {
        used_names.extend(names.keys().map(|name| name.to_ascii_lowercase()));
    }
    let mut procedure = base.clone();
    let mut suffix = 2usize;
    while used_names.contains(&procedure.to_ascii_lowercase()) {
        procedure = format!("{base}_{suffix}");
        suffix = suffix.checked_add(1).ok_or_else(|| bad("Too many recorded macros."))?;
    }
    let mut module = "Module1".to_string();
    let existing = items.iter().find(|item| item.get("name").and_then(Json::as_str).is_some_and(|name| name.eq_ignore_ascii_case(&module)));
    let mut previous = String::new();
    if let Some(existing) = existing {
        if existing.get("kind").and_then(Json::as_str) == Some("standard") && existing.get("compileError").is_none_or(Json::is_null) {
            module = existing.get("name").and_then(Json::as_str).unwrap_or("Module1").to_string();
            previous = existing.get("source").and_then(Json::as_str).unwrap_or_default().to_string();
        } else {
            let mut suffix = 2usize;
            while items.iter().any(|item| item.get("name").and_then(Json::as_str).is_some_and(|name| name.eq_ignore_ascii_case(&module))) {
                module = format!("Module{suffix}");
                suffix = suffix.checked_add(1).ok_or_else(|| bad("Too many VBA modules."))?;
            }
        }
    }
    let mut source = previous;
    source.push_str(&format!("\nSub {procedure}()\n"));
    for line in &s.recorder.lines {
        source.push_str(line);
        source.push('\n');
    }
    source.push_str("End Sub\n");
    if source.len() > 1024 * 1024 {
        return Err(bad("Recorded module source exceeds 1 MiB."));
    }
    Program::compile(&source).map_err(|error| bad(format!("Cannot save recorded VBA: {}", error.message)))?;
    let commands = s.recorder.commands.clone();
    let unsupported = s.recorder.unsupported.clone();
    // Both metadata writes happen in a staging document. A bad module, size limit or metadata
    // error leaves the live workbook, existing scripts and recording unchanged.
    let mut staged = Session::new();
    staged.add_document(s.doc()?.clone());
    super::new_script(&mut staged, &json!({"name": procedure, "commands": commands}))?;
    crate::vba::save_module(&mut staged, &json!({"name": module, "source": source}))?;
    s.doc_mut()?.wb = staged.doc()?.wb.clone();
    s.recording = None;
    s.recorder = Recorder::default();
    Ok(json!({"recording": false, "saved": procedure, "steps": commands.len(), "module": module, "procedure": procedure,
        "unsupported": unsupported, "vbaComplete": unsupported.is_empty()}))
}

fn quoted(text: &str) -> Option<String> {
    if text.len() > MAX_BYTES {
        return None;
    }
    let mut pieces = Vec::new();
    let mut part = String::new();
    for ch in text.chars() {
        let constant = match ch {
            '\n' => Some("vbLf"),
            '\r' => Some("vbCr"),
            '\t' => Some("vbTab"),
            _ => None,
        };
        if let Some(constant) = constant {
            pieces.push(format!("\"{}\"", part.replace('"', "\"\"")));
            part.clear();
            pieces.push(constant.to_string());
        } else if ch.is_control() {
            return None;
        } else {
            part.push(ch);
        }
    }
    pieces.push(format!("\"{}\"", part.replace('"', "\"\"")));
    Some(pieces.join(" & "))
}

fn range(sheet: &str, r: RangeRef) -> Option<String> {
    Some(format!("Worksheets({}).Range({})", quoted(sheet)?, quoted(&r.a1())?))
}

fn target_sheet<'a>(before: &'a DocState, params: &Json) -> Option<(usize, &'a str)> {
    let sheet = match params.get("sheet") {
        Some(Json::String(name)) => before.wb.sheet_index(name)?,
        Some(Json::Number(n)) => usize::try_from(n.as_u64()?).ok()?,
        _ => params
            .get("range")
            .or_else(|| params.get("cell"))
            .and_then(Json::as_str)
            .and_then(|text| gridcraft_core::split_sheet(text).0)
            .and_then(|name| before.wb.sheet_index(&name))
            .unwrap_or(before.wb.active_sheet),
    };
    Some((sheet, &before.wb.sheet(sheet)?.name))
}

fn target_ranges(before: &DocState, params: &Json) -> Option<Vec<RangeRef>> {
    if let Some(text) = params.get("range").or_else(|| params.get("cell")).and_then(Json::as_str) {
        let (_, body) = gridcraft_core::split_sheet(text);
        body.split(',').map(RangeRef::parse).collect()
    } else {
        Some(before.selection.ranges.clone())
    }
}

fn write_cells(si: usize, name: &str, ranges: &[RangeRef], after: &DocState) -> Option<Vec<String>> {
    if ranges.iter().map(RangeRef::count).sum::<u64>() > MAX_CELLS {
        return None;
    }
    let sheet = after.wb.sheet(si)?;
    let mut lines = Vec::new();
    for r in ranges {
        for at in r.iter() {
            let target = range(name, RangeRef::cell(at))?;
            if sheet.cell_pictures.contains_key(&at) {
                return None;
            }
            if let Some(formula) = sheet.cell(at).and_then(|cell| cell.formula.as_ref()) {
                if formula.array.is_some() {
                    return None;
                }
                lines.push(format!("    {target}.Formula = {}", quoted(&format!("={}", formula.text))?));
            } else {
                let value = match sheet.value(at) {
                    Value::Empty => {
                        lines.push(format!("    {target}.ClearContents"));
                        continue;
                    }
                    Value::Number(n) if n.is_finite() => gridcraft_core::number_to_text(n),
                    Value::Bool(b) => if b { "True" } else { "False" }.to_string(),
                    Value::Text(text) => quoted(&text)?,
                    _ => return None,
                };
                lines.push(format!("    {target}.Value = {value}"));
            }
            if lines.last().is_some_and(|line| line.len() > MAX_BYTES) || lines.iter().map(String::len).sum::<usize>() > MAX_BYTES {
                return None;
            }
        }
    }
    Some(lines)
}

fn translate(id: &str, params: &Json, before: &DocState, after: &DocState) -> Option<Vec<String>> {
    let (si, name) = target_sheet(before, params)?;
    if before.selected_sheets.len() > 1 && params.get("sheet").is_none() && !id.starts_with("sheet.") && !id.starts_with("selection.") {
        let mut lines = Vec::new();
        for index in &before.selected_sheets {
            let mut params = if params.is_object() { params.clone() } else { json!({}) };
            params["sheet"] = json!(index);
            lines.extend(translate(id, &params, before, after)?);
            if lines.iter().map(String::len).sum::<usize>() > MAX_BYTES {
                return None;
            }
        }
        return Some(lines);
    }
    match id {
        "selection.set" | "selection.move" | "selection.next" => {
            if after.selection.ranges.len() != 1 {
                return None;
            }
            let sheet = &after.wb.active()?.name;
            Some(vec![format!("    {}.Select", range(sheet, after.selection.current())?)])
        }
        "cell.set" => {
            let at = params
                .get("cell")
                .and_then(Json::as_str)
                .and_then(|text| CellRef::parse(gridcraft_core::split_sheet(text).1))
                .unwrap_or(before.selection.active);
            write_cells(si, name, &[RangeRef::cell(at)], after)
        }
        "range.fill" => write_cells(si, name, &target_ranges(before, params)?, after),
        "range.setValues" => {
            let rows = params.get("values")?.as_array()?;
            let width = rows.iter().map(|row| row.as_array().map_or(1, Vec::len)).max()?;
            let start = target_ranges(before, params)?.first()?.start;
            let mut ranges = Vec::new();
            if rows.len().checked_mul(width)? > MAX_CELLS as usize {
                return None;
            }
            for (ri, row) in rows.iter().enumerate() {
                for ci in 0..row.as_array().map_or(1, Vec::len) {
                    ranges.push(RangeRef::cell(start.offset(ri as i64, ci as i64)?));
                }
            }
            write_cells(si, name, &ranges, after)
        }
        "edit.clearContents" | "edit.clearAll" => {
            let method = if id == "edit.clearAll" { "Clear" } else { "ClearContents" };
            target_ranges(before, params)?
                .iter()
                .map(|r| {
                    if r.count() > MAX_CELLS {
                        return None;
                    }
                    Some(format!("    {}.{method}", range(name, *r)?))
                })
                .collect()
        }
        "home.bold" | "home.italic" | "home.fontName" | "home.fontSize" | "home.numberFormat" => {
            let ranges = target_ranges(before, params)?;
            if ranges.iter().map(RangeRef::count).sum::<u64>() > MAX_CELLS {
                return None;
            }
            let sh = after.wb.sheet(si)?;
            let mut lines = Vec::new();
            for r in ranges {
                for at in r.iter() {
                    let style = after.wb.styles.get(sh.style_id(at));
                    let (member, value) = match id {
                        "home.bold" => ("Font.Bold", if style.font.bold { "True" } else { "False" }.to_string()),
                        "home.italic" => ("Font.Italic", if style.font.italic { "True" } else { "False" }.to_string()),
                        "home.fontName" => ("Font.Name", quoted(&style.font.name)?),
                        "home.fontSize" => ("Font.Size", style.font.size.to_string()),
                        _ => ("NumberFormat", quoted(style.num_fmt.as_str())?),
                    };
                    lines.push(format!("    {}.{member} = {value}", range(name, RangeRef::cell(at))?));
                    if lines.iter().map(String::len).sum::<usize>() > MAX_BYTES {
                        return None;
                    }
                }
            }
            Some(lines)
        }
        "home.insertRows" | "home.deleteRows" | "home.insertColumns" | "home.deleteColumns" => {
            let rows = id.ends_with("Rows");
            let parameter = if rows { "rows" } else { "cols" };
            let text = params.get(parameter).or_else(|| params.get("range")).and_then(Json::as_str);
            let ranges: Vec<RangeRef> = if let Some(text) = text {
                text.split(',').map(crate::cmd::parse_lines).collect::<Option<_>>()?
            } else {
                before.selection.ranges.clone()
            };
            let mut spans: Vec<(u32, u32)> = if let Some(count) = params.get("count").and_then(Json::as_u64) {
                let r = if text.is_some() { *ranges.first()? } else { before.selection.current() };
                let start = if rows { r.start.row } else { r.start.col };
                let count = u32::try_from(count).ok()?.max(1);
                let limit = if rows { gridcraft_core::MAX_ROWS } else { gridcraft_core::MAX_COLS };
                let count = if id.contains("delete") { count.min(limit.saturating_sub(start)) } else { count };
                vec![(start, start.checked_add(count.checked_sub(1)?)?)]
            } else {
                ranges.iter().map(|r| if rows { (r.start.row, r.end.row) } else { (r.start.col, r.end.col) }).collect()
            };
            spans.sort_unstable();
            let mut merged: Vec<(u32, u32)> = Vec::new();
            for (start, end) in spans {
                if let Some(last) = merged.last_mut()
                    && start <= last.1.saturating_add(1)
                {
                    last.1 = last.1.max(end);
                } else {
                    merged.push((start, end));
                }
            }
            if merged.iter().map(|(start, end)| u64::from(end - start) + 1).sum::<u64>() > MAX_CELLS {
                return None;
            }
            let axis = if rows { "EntireRow" } else { "EntireColumn" };
            let method = if id.contains("insert") { "Insert" } else { "Delete" };
            merged
                .into_iter()
                .rev()
                .map(|(start, end)| {
                    let r = if rows { RangeRef::rows(start, end) } else { RangeRef::cols(start, end) };
                    Some(format!("    {}.{axis}.{method}", range(name, r)?))
                })
                .collect()
        }
        "sheet.activate" if params.get("mode").and_then(Json::as_str).is_some_and(|mode| !matches!(mode, "select" | "ungroup")) => None,
        "sheet.activate" | "sheet.next" | "sheet.previous" => Some(vec![format!("    Worksheets({}).Activate", quoted(&after.wb.active()?.name)?)]),
        "sheet.rename" => Some(vec![format!("    Worksheets({}).Name = {}", quoted(name)?, quoted(&after.wb.sheet(si)?.name)?)]),
        "home.insertSheet" if params.get("before").is_none_or(Json::is_null) => {
            Some(vec!["    Worksheets.Add".into(), format!("    ActiveSheet.Name = {}", quoted(&after.wb.active()?.name)?)])
        }
        _ => None,
    }
}
