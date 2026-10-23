//! VBA development foundation. Execution stages edits in a private workbook snapshot and
//! publishes them only after success. This is not a claim of full Excel/VBA compatibility.
mod host;
pub mod language;
#[cfg(test)]
mod tests;

use crate::{EngineError, Result, Session};
use gridcraft_model::{DefinedName, Workbook};
use language::{Error, Execution, Program};
use serde::de::{self, MapAccess, Visitor};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;
use std::sync::Arc;

const SOURCES: &str = "_gc_vba_sources_v1";
const MAX_SOURCE: usize = 1024 * 1024;
const MAX_TOTAL_SOURCE: usize = 8 * 1024 * 1024;
const MAX_STEPS: u64 = 1_000_000;

#[derive(Default)]
pub(crate) struct State {
    run: Option<Box<Run>>,
    result: Json,
}

struct Run {
    execution: Execution,
    host: host::ExcelHost,
    uid: u64,
    revision: u64,
    module: String,
    procedure: String,
}

fn bad(message: impl Into<String>) -> EngineError {
    crate::cmd::bad("VBA", message)
}

struct SourceMap(BTreeMap<String, String>);

impl<'de> serde::Deserialize<'de> for SourceMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct SourceVisitor;
        impl<'de> Visitor<'de> for SourceVisitor {
            type Value = SourceMap;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a map of unique VBA module names to source strings")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                let mut bytes = 0usize;
                while let Some((name, source)) = map.next_entry::<String, String>()? {
                    bytes = bytes.saturating_add(source.len());
                    if values.len() >= 128 || source.len() > MAX_SOURCE || bytes > MAX_TOTAL_SOURCE {
                        return Err(de::Error::custom("VBA source metadata exceeds its module or size limits"));
                    }
                    if values.insert(name, source).is_some() {
                        return Err(de::Error::custom("duplicate VBA module name"));
                    }
                }
                Ok(SourceMap(values))
            }
        }
        deserializer.deserialize_map(SourceVisitor)
    }
}

fn sources(wb: &Workbook) -> Result<BTreeMap<String, String>> {
    let mut stored_names = wb.names.iter().filter(|n| n.name == SOURCES);
    let Some(stored) = stored_names.next() else { return Ok(BTreeMap::new()) };
    if stored_names.next().is_some() {
        return Err(bad("duplicate VBA source metadata entries"));
    }
    if stored.formula.len() > MAX_TOTAL_SOURCE * 6 + 65_536 {
        return Err(bad("VBA source metadata exceeds its encoded size limit"));
    }
    let raw = stored
        .formula
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| bad("invalid VBA source metadata string"))?
        .replace("\"\"", "\"");
    let values = serde_json::from_str::<SourceMap>(&raw).map_err(|e| bad(format!("invalid VBA source metadata JSON: {e}")))?.0;
    if values.len() > 128 || values.values().any(|s| s.len() > MAX_SOURCE) || values.values().map(String::len).sum::<usize>() > MAX_TOTAL_SOURCE {
        return Err(bad("VBA source metadata exceeds module count or source size limits"));
    }
    let mut names = std::collections::BTreeSet::new();
    for name in values.keys() {
        let mut chars = name.chars();
        if name.len() > 128
            || !chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
            || !chars.all(|c| c.is_alphanumeric() || c == '_')
            || !names.insert(name.to_ascii_lowercase())
        {
            return Err(bad("VBA source metadata contains an invalid or duplicate module name"));
        }
    }
    Ok(values)
}

pub(crate) fn has_modules(wb: &Workbook) -> bool {
    wb.names.iter().any(|n| n.name == SOURCES)
}

fn describe(name: &str, source: &str, origin: &str, kind: &str) -> Json {
    let mut value = json!({"name":name,"source":source,"origin":origin,"kind":kind,"procedures":[]});
    match Program::compile(source) {
        Ok(program) => value["procedures"] = json!(program.procedures()),
        Err(e) => value["compileError"] = json!({"line":e.line,"message":e.message}),
    }
    value
}

pub(crate) fn modules(s: &mut Session, _: &Json) -> Result<Json> {
    let d = s.doc()?;
    let own = sources(&d.wb)?;
    let mut list: Vec<Json> = own.iter().map(|(name, source)| describe(name, source, "gridcraft", "standard")).collect();
    let mut import_error = None;
    if let Some(project) = &d.wb.vba_project {
        match gridcraft_xlsx::vba::read_modules(project) {
            Ok(imported) => {
                for module in imported {
                    if !own.keys().any(|n| n.eq_ignore_ascii_case(&module.name)) {
                        list.push(describe(&module.name, &module.source, "project", &module.kind));
                    }
                }
            }
            Err(error) => import_error = Some(error),
        }
    }
    Ok(json!({"modules":list,"trusted":d.scripts_trusted,"preservedVba":d.wb.vba_project.is_some(),"importError":import_error}))
}

pub(crate) fn save_module(s: &mut Session, p: &Json) -> Result<Json> {
    let name = p.get("name").and_then(Json::as_str).ok_or_else(|| bad("missing module name"))?;
    let source = p.get("source").and_then(Json::as_str).ok_or_else(|| bad("missing source"))?;
    let mut chars = name.chars();
    if name.len() > 128 || !chars.next().is_some_and(|c| c.is_alphabetic() || c == '_') || !chars.all(|c| c.is_alphanumeric() || c == '_') {
        return Err(bad("module name must be an identifier of at most 128 bytes"));
    }
    if source.len() > MAX_SOURCE {
        return Err(bad("module source exceeds 1 MiB"));
    }
    let mut own = sources(&s.doc()?.wb)?;
    let key = own.keys().find(|n| n.eq_ignore_ascii_case(name)).cloned().unwrap_or_else(|| name.to_owned());
    own.insert(key.clone(), source.to_owned());
    if own.len() > 128 || own.values().map(String::len).sum::<usize>() > MAX_TOTAL_SOURCE {
        return Err(bad("too many modules or total source exceeds 8 MiB"));
    }
    let text = serde_json::to_string(&own).map_err(|e| bad(e.to_string()))?;
    let wb = Arc::make_mut(&mut s.doc_mut()?.wb);
    wb.names.retain(|n| n.name != SOURCES);
    wb.names.push(DefinedName {
        name: SOURCES.into(),
        scope: None,
        formula: format!("\"{}\"", text.replace('"', "\"\"")),
        comment: "GridCraft VBA source; does not rewrite vbaProject.bin".into(),
        hidden: true,
    });
    Ok(describe(&key, source, "gridcraft", "standard"))
}

fn idle() -> Json {
    json!({"state":"idle","module":"","procedure":"","line":0,"steps":0,"log":[]})
}

fn report(run: &Run, state: &str, error: Option<&Error>) -> Json {
    let mut result = json!({"state":state,"module":run.module,"procedure":run.procedure,
        "line":run.execution.line(),"steps":run.execution.steps(),"log":run.host.log});
    if let Some(e) = error {
        result["error"] = json!({"line":e.line,"message":e.message});
    }
    result
}

pub(crate) fn status(s: &mut Session, _: &Json) -> Result<Json> {
    Ok(s.vba.run.as_ref().map_or_else(|| if s.vba.result.is_null() { idle() } else { s.vba.result.clone() }, |r| report(r, "running", None)))
}

pub(crate) fn start(s: &mut Session, p: &Json) -> Result<Json> {
    if s.vba.run.is_some() {
        return Err(bad("a macro is already running; stop it first"));
    }
    if !s.doc()?.scripts_trusted {
        return Err(bad("this workbook's macros are disabled; use Trust Scripts explicitly before running them"));
    }
    let procedure = p.get("procedure").and_then(Json::as_str).filter(|n| !n.is_empty()).ok_or_else(|| bad("select a Sub procedure"))?.to_owned();
    let module = p.get("module").and_then(Json::as_str).unwrap_or("(editor)").to_owned();
    let source = if let Some(source) = p.get("source").and_then(Json::as_str) {
        source.to_owned()
    } else {
        let all = modules(s, &Json::Null)?;
        all.get("modules")
            .and_then(Json::as_array)
            .and_then(|modules| modules.iter().find(|m| m["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(&module))))
            .and_then(|m| m["source"].as_str())
            .ok_or_else(|| bad("module not found"))?
            .to_owned()
    };
    let execution = match Program::compile(&source).and_then(|p| p.start(&procedure)) {
        Ok(execution) => execution,
        Err(error) => {
            s.vba.result = json!({"state":"failed","module":module,"procedure":procedure,"line":error.line,"steps":0,"log":[],"error":{"line":error.line,"message":error.message}});
            return Ok(s.vba.result.clone());
        }
    };
    let doc = s.doc()?;
    let mut stage = Session::new();
    let mut staged = doc.clone();
    staged.undo.clear();
    staged.redo.clear();
    stage.add_document(staged);
    stage.prefs = s.prefs.clone();
    stage.text_width = s.text_width.clone();
    let run = Run { execution, host: host::ExcelHost::new(stage), uid: doc.uid, revision: doc.revision, module, procedure };
    s.vba.result = report(&run, "running", None);
    s.vba.run = Some(Box::new(run));
    Ok(s.vba.result.clone())
}

pub(crate) fn stop(s: &mut Session, _: &Json) -> Result<Json> {
    if let Some(run) = s.vba.run.take() {
        s.vba.result = report(&run, "stopped", None);
    }
    status(s, &Json::Null)
}

pub(crate) fn poll(s: &mut Session, p: &Json) -> Result<Json> {
    let Some(mut run) = s.vba.run.take() else { return status(s, p) };
    let budget = p.get("budget").and_then(Json::as_u64).unwrap_or(500).clamp(1, 1000).min(MAX_STEPS.saturating_sub(run.execution.steps())) as usize;
    let valid = s.active().is_some_and(|d| d.uid == run.uid && d.revision == run.revision && d.scripts_trusted);
    // New blank workbooks start trusted. Every execution, including drafts, requires that
    // trust; revoking it, switching or editing the workbook aborts staged work.
    let outcome = if !valid {
        Err(Error::new(run.execution.line(), "workbook changed, closed, switched, or macro trust was revoked; pending changes discarded"))
    } else if run.execution.steps() >= MAX_STEPS {
        Err(Error::new(run.execution.line(), "macro exceeded the execution limit of 1,000,000 instructions"))
    } else {
        let _sandbox = crate::io::ScriptSandbox::enter();
        run.execution.step(&mut run.host, budget)
    };
    match outcome {
        Ok(false) => {
            s.vba.result = report(&run, "running", None);
            s.vba.run = Some(run);
        }
        Ok(true) => {
            if let Err(error) = run.host.finish() {
                s.vba.result = report(&run, "failed", Some(&error));
            } else {
                let staged = run.host.session.doc()?;
                let d = s.doc_mut()?;
                if !Arc::ptr_eq(&d.wb, &staged.wb) {
                    crate::push_history(&mut d.undo, format!("VBA: {}", run.procedure), d.wb.clone(), &staged.wb, d.selection.clone());
                    d.wb = staged.wb.clone();
                    d.calc = staged.calc.clone();
                    d.selection = staged.selection.clone();
                    d.redo.clear();
                    d.revision = d.revision.saturating_add(1);
                }
                s.vba.result = report(&run, "completed", None);
            }
        }
        Err(error) => s.vba.result = report(&run, "failed", Some(&error)),
    }
    Ok(s.vba.result.clone())
}
