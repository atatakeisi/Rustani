//! Excel object adapters. Unknown members fail explicitly; no arbitrary command, URL or file
//! access is exposed to VBA. All workbook mutations go through the existing engine commands.
use super::language::{Error, Host, Value};
use crate::Session;
use gridcraft_core::{CellRef, MAX_COLS, MAX_ROWS, RangeRef, Value as CellValue};
use serde_json::{Value as Json, json};

const MAX_CELLS: u64 = 10_000;
const MAX_OPERATIONS: u64 = 1_000_000;
const MAX_OBJECTS: usize = 32_768;
const MAX_CELL_VALUE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Object {
    Application,
    Workbook,
    Worksheets,
    Worksheet(usize),
    Range(usize, RangeRef),
    Rows(usize, RangeRef),
    Columns(usize, RangeRef),
    Font(usize, RangeRef),
    Interior(usize, RangeRef),
    WorksheetFunction,
    Debug,
    ChartObjects(usize),
    EmbeddedChart(usize),
    Chart(usize),
    ChartTitle(usize),
}

struct PendingChart {
    sheet: usize,
    id: Option<u32>,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    kind: String,
    title: Option<String>,
}

pub(super) struct ExcelHost {
    pub session: Session,
    pub log: Vec<String>,
    objects: Vec<Object>,
    object_ids: std::collections::HashMap<Object, u64>,
    charts: Vec<PendingChart>,
    operations: u64,
    cell_value_bytes: u64,
}

fn err(message: impl Into<String>) -> Error {
    Error::new(0, message)
}
fn text(value: &Value) -> Result<String, Error> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(if *b { "True" } else { "False" }.into()),
        Value::Empty => Ok(String::new()),
        _ => Err(err("expected a scalar value")),
    }
}
fn number(value: &Value) -> Result<f64, Error> {
    match value {
        Value::Number(n) if n.is_finite() => Ok(*n),
        Value::Empty => Ok(0.0),
        Value::Bool(b) => Ok(if *b { -1.0 } else { 0.0 }),
        Value::Text(s) => s.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or_else(|| err("type mismatch: expected number")),
        _ => Err(err("type mismatch: expected number")),
    }
}
fn boolean(v: &Value) -> Result<bool, Error> {
    match v {
        Value::Bool(b) => Ok(*b),
        _ => Ok(number(v)? != 0.0),
    }
}
fn arg(args: &[Value], i: usize) -> Result<&Value, Error> {
    args.get(i).ok_or_else(|| err("argument not optional"))
}
fn no_args(args: &[Value], name: &str) -> Result<(), Error> {
    if args.is_empty() { Ok(()) } else { Err(err(format!("{name} does not accept arguments"))) }
}
fn index(args: &[Value], i: usize, max: u32) -> Result<u32, Error> {
    let n = number(arg(args, i)?)?;
    if n.fract() != 0.0 || n < 1.0 || n > f64::from(max) {
        return Err(err("subscript out of range"));
    }
    Ok(n as u32 - 1)
}
fn scalar_json(v: &Value, formula: bool) -> Result<Json, Error> {
    Ok(match v {
        Value::Empty => Json::Null,
        Value::Number(n) if n.is_finite() => json!(n),
        Value::Bool(b) => json!(b),
        Value::Text(s) => {
            if s.len() > 1024 * 1024 {
                return Err(err("cell text exceeds 1 MiB"));
            }
            // An apostrophe keeps numeric-looking text as text. Formula assignment is separate.
            if formula { json!(s) } else { json!(format!("'{s}")) }
        }
        _ => return Err(err("unsupported value for a worksheet cell")),
    })
}

// Validate and account for the expanded payload before cloning a scalar across a range or
// converting an array to JSON. Counting all writes, including replacements, bounds retained
// workbook values as well as the temporary command payload for the whole staged run.
fn scalar_storage_bytes(v: &Value, formula: bool) -> Result<u64, Error> {
    match v {
        Value::Empty => Ok(0),
        Value::Number(n) if n.is_finite() => Ok(std::mem::size_of::<f64>() as u64),
        Value::Bool(_) => Ok(1),
        Value::Text(s) => {
            if s.len() > 1024 * 1024 {
                return Err(err("cell text exceeds 1 MiB"));
            }
            Ok(s.len() as u64 + u64::from(!formula))
        }
        _ => Err(err("unsupported value for a worksheet cell")),
    }
}

impl ExcelHost {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            log: vec![],
            objects: vec![],
            object_ids: std::collections::HashMap::new(),
            charts: vec![],
            operations: 0,
            cell_value_bytes: 0,
        }
    }
    fn doc(&self) -> Result<&crate::DocState, Error> {
        self.session.doc().map_err(|e| err(e.to_string()))
    }
    fn object(&self, value: &Value) -> Result<Object, Error> {
        let Value::Object(id) = value else { return Err(err("object required")) };
        let id = usize::try_from(*id).map_err(|_| err("invalid object reference"))?;
        self.objects.get(id).cloned().ok_or_else(|| err("invalid object reference"))
    }
    fn wrap(&mut self, object: Object) -> Result<Value, Error> {
        if let Some(i) = self.object_ids.get(&object) {
            return Ok(Value::Object(*i));
        }
        if self.objects.len() >= MAX_OBJECTS {
            return Err(err("macro object limit exceeded"));
        }
        let id = self.objects.len() as u64;
        self.object_ids.insert(object.clone(), id);
        self.objects.push(object);
        Ok(Value::Object(id))
    }
    fn command(&mut self, id: &str, params: Json) -> Result<Json, Error> {
        self.operations = self.operations.saturating_add(1);
        if self.operations > MAX_OPERATIONS {
            return Err(err("macro operation limit exceeded"));
        }
        let result = self.session.execute(id, params).map_err(|e| err(e.to_string()));
        // The outer staged transaction supplies one history entry. Do not retain per-cell
        // history or a huge journal in the staging session.
        if let Some(d) = self.session.active_mut() {
            d.undo.clear();
            d.redo.clear();
        }
        self.session.journal.clear();
        self.session.ui_requests.clear();
        self.session.journal_sizes.clear();
        self.session.journal_bytes = 0;
        result
    }
    fn log(&mut self, message: String) -> Result<(), Error> {
        if self.log.len() >= 1000 || self.log.iter().map(String::len).sum::<usize>().saturating_add(message.len()) > 256 * 1024 {
            return Err(err("macro output log limit exceeded"));
        }
        self.log.push(message);
        Ok(())
    }
    fn sheet_index(&self, value: &Value) -> Result<usize, Error> {
        let d = self.doc()?;
        match value {
            Value::Text(name) => d.wb.sheet_index(name).ok_or_else(|| err(format!("worksheet not found: {name}"))),
            _ => {
                let i = index(std::slice::from_ref(value), 0, d.wb.sheets.len().min(u32::MAX as usize) as u32)? as usize;
                Ok(i)
            }
        }
    }
    fn range(&self, sheet: usize, args: &[Value], origin: Option<RangeRef>) -> Result<(usize, RangeRef), Error> {
        if args.len() == 2 {
            let one = match self.object(arg(args, 0)?)? {
                Object::Range(si, r) if si == sheet => r,
                _ => return Err(err("Range endpoints must belong to the same worksheet")),
            };
            let two = match self.object(arg(args, 1)?)? {
                Object::Range(si, r) if si == sheet => r,
                _ => return Err(err("Range endpoints must belong to the same worksheet")),
            };
            return Ok((sheet, RangeRef::new(one.start, two.end)));
        }
        if args.len() != 1 {
            return Err(err("Range expects an address or two cell objects"));
        }
        let address = text(arg(args, 0)?)?;
        let (qualifier, body) = gridcraft_core::split_sheet(&address);
        let si = match &qualifier {
            Some(name) => self.doc()?.wb.sheet_index(name).ok_or_else(|| err("worksheet not found"))?,
            None => sheet,
        };
        let range = RangeRef::parse(body).ok_or_else(|| err(format!("invalid range: {address}")))?;
        if let Some(origin) = origin {
            if qualifier.is_some() {
                return Err(err("qualified addresses on relative Range are not supported"));
            }
            let a = origin.start.offset(range.start.row as i64, range.start.col as i64).ok_or_else(|| err("range leaves worksheet"))?;
            let b = origin.start.offset(range.end.row as i64, range.end.col as i64).ok_or_else(|| err("range leaves worksheet"))?;
            Ok((si, RangeRef::new(a, b)))
        } else {
            Ok((si, range))
        }
    }
    fn cells(&mut self, si: usize, origin: RangeRef, args: &[Value]) -> Result<Value, Error> {
        if args.is_empty() {
            return self.wrap(Object::Range(si, origin));
        }
        let (row, col) = if args.len() == 1 {
            let n = number(arg(args, 0)?)?;
            if n < 1.0 || n.fract() != 0.0 || n > f64::from(MAX_ROWS) * f64::from(MAX_COLS) {
                return Err(err("cell index out of range"));
            }
            let n = n as u64 - 1;
            ((n / u64::from(origin.width())) as i64, (n % u64::from(origin.width())) as i64)
        } else if args.len() == 2 {
            (i64::from(index(args, 0, MAX_ROWS)?), i64::from(index(args, 1, MAX_COLS)?))
        } else {
            return Err(err("Cells expects zero, one or two indices"));
        };
        let cell = origin.start.offset(row, col).ok_or_else(|| err("cell leaves worksheet"))?;
        self.wrap(Object::Range(si, RangeRef::cell(cell)))
    }
    fn read(&self, si: usize, r: RangeRef, formula: bool) -> Result<Value, Error> {
        if r.count() > MAX_CELLS {
            return Err(err("range read exceeds 10,000 cells per operation"));
        }
        let d = self.doc()?;
        let sh = d.wb.sheet(si).ok_or_else(|| err("worksheet no longer exists"))?;
        // Cell text is shared in the workbook, but from_cell/input_text creates owned VBA
        // strings. Check the full returned shape before copying any of those strings.
        let mut bytes = 0u64;
        for c in r.iter() {
            let size = if let Some(f) = sh.cells.get(c).and_then(|cell| cell.formula.as_ref()).filter(|_| formula) {
                f.text.len().checked_add(1).ok_or_else(|| err("range read byte size overflow"))? as u64
            } else {
                match sh.value_ref(c) {
                    Some(CellValue::Text(s)) => s.len() as u64,
                    Some(CellValue::Number(_)) => std::mem::size_of::<f64>() as u64,
                    Some(CellValue::Bool(_)) => 1,
                    _ => 0,
                }
            };
            if size > 1024 * 1024 {
                return Err(err("range read cell text/formula exceeds 1 MiB"));
            }
            bytes = bytes.checked_add(size).ok_or_else(|| err("range read byte size overflow"))?;
            if bytes > MAX_CELL_VALUE_BYTES {
                return Err(err("range read exceeds the 16 MiB byte limit"));
            }
        }
        let at = |c: CellRef| -> Result<Value, Error> {
            if formula && sh.cells.get(c).is_some_and(|cell| cell.formula.is_some()) {
                return Ok(Value::Text(sh.input_text(c)));
            }
            from_cell(&sh.value(c))
        };
        if r.is_single() {
            return at(r.start);
        }
        let mut rows = Vec::new();
        for row in r.start.row..=r.end.row {
            let mut cols = Vec::new();
            for col in r.start.col..=r.end.col {
                cols.push(at(CellRef::new(row, col))?);
            }
            rows.push(cols);
        }
        Ok(Value::Array(rows))
    }
    fn write(&mut self, si: usize, r: RangeRef, value: Value, formula: bool) -> Result<(), Error> {
        if r.count() > MAX_CELLS {
            return Err(err("range write exceeds 10,000 cells per operation"));
        }
        self.operations = self.operations.saturating_add(r.count());
        if self.operations > MAX_OPERATIONS {
            return Err(err("macro changed too many cells"));
        }
        let too_large = || err("macro cell values/formulas exceed the 16 MiB total byte limit");
        let bytes = match &value {
            Value::Array(rows) => {
                if rows.len() != r.height() as usize || rows.iter().any(|row| row.len() != r.width() as usize) {
                    return Err(err("array dimensions must match destination range (other array assignment semantics are not implemented)"));
                }
                rows.iter().flatten().try_fold(0u64, |total, v| total.checked_add(scalar_storage_bytes(v, formula)?).ok_or_else(too_large))?
            }
            scalar => scalar_storage_bytes(scalar, formula)?.checked_mul(r.count()).ok_or_else(too_large)?,
        };
        let total = self.cell_value_bytes.checked_add(bytes).ok_or_else(too_large)?;
        if total > MAX_CELL_VALUE_BYTES {
            return Err(too_large());
        }
        self.cell_value_bytes = total;
        let values = match value {
            Value::Array(rows) => {
                rows.iter().map(|row| row.iter().map(|v| scalar_json(v, formula)).collect::<Result<Vec<_>, _>>()).collect::<Result<Vec<_>, _>>()?
            }
            scalar => {
                let scalar = scalar_json(&scalar, formula)?;
                vec![vec![scalar; r.width() as usize]; r.height() as usize]
            }
        };
        self.command("range.setValues", json!({"sheet":si,"range":r.a1(),"values":values}))?;
        Ok(())
    }
    fn worksheet_function(&self, name: &str, args: &[Value]) -> Result<Value, Error> {
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(err("invalid worksheet function"));
        }
        let parts = args.iter().map(|a| self.formula_arg(a)).collect::<Result<Vec<_>, _>>()?;
        let input = format!("={name}({})", parts.join(","));
        if input.len() > 1024 * 1024 {
            return Err(err("worksheet function input too large"));
        }
        let d = self.doc()?;
        from_cell(&gridcraft_calc::evaluate(&d.wb, d.wb.active_sheet, d.selection.active, &input))
    }
    fn formula_arg(&self, value: &Value) -> Result<String, Error> {
        match value {
            Value::Empty => Ok("0".into()),
            Value::Number(n) if n.is_finite() => Ok(n.to_string()),
            Value::Bool(b) => Ok(if *b { "TRUE" } else { "FALSE" }.into()),
            Value::Text(s) => Ok(format!("\"{}\"", s.replace('"', "\"\""))),
            Value::Object(_) => match self.object(value)? {
                Object::Range(si, r) => {
                    if r.count() > MAX_CELLS {
                        return Err(err("worksheet function range exceeds 10,000 cells"));
                    }
                    let d = self.doc()?;
                    let sh = d.wb.sheet(si).ok_or_else(|| err("worksheet not found"))?;
                    Ok(format!("{}!{}", gridcraft_formula::quote_sheet(&sh.name), r.a1()))
                }
                _ => Err(err("expected a Range argument")),
            },
            Value::Array(rows) => {
                if rows.iter().map(Vec::len).sum::<usize>() > MAX_CELLS as usize {
                    return Err(err("array too large"));
                }
                let rows = rows
                    .iter()
                    .map(|row| row.iter().map(|v| self.formula_arg(v)).collect::<Result<Vec<_>, _>>().map(|v| v.join(",")))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{{{}}}", rows.join(";")))
            }
            _ => Err(err("invalid worksheet function argument")),
        }
    }
    fn chart_update(&mut self, i: usize) -> Result<(), Error> {
        let c = self.charts.get(i).ok_or_else(|| err("chart not found"))?;
        if let Some(id) = c.id {
            let p = json!({"chart":id,"type":c.kind,"title":c.title,"width":c.width,"height":c.height,"dx":c.left,"dy":c.top});
            self.command("sheet.activate", json!({"sheet":c.sheet}))?;
            self.command("chart.set", p)?;
        }
        Ok(())
    }
    pub fn finish(&self) -> Result<(), Error> {
        if self.charts.iter().any(|c| c.id.is_none()) {
            return Err(err("creating an empty chart is not implemented; use Chart.SetSourceData"));
        }
        Ok(())
    }
}

impl Host for ExcelHost {
    fn get(&mut self, receiver: Option<&Value>, name: &str, args: &[Value]) -> Result<Value, Error> {
        let key = name.to_ascii_lowercase();
        let object = receiver.map(|r| self.object(r)).transpose()?;
        if object.is_none() || object == Some(Object::Application) {
            if !args.is_empty()
                && matches!(
                    key.as_str(),
                    "application"
                        | "thisworkbook"
                        | "activeworkbook"
                        | "activesheet"
                        | "worksheetfunction"
                        | "debug"
                        | "selection"
                        | "activecell"
                        | "calculate"
                        | "xlup"
                        | "xldown"
                        | "xltoleft"
                        | "xltoright"
                        | "xlcolumnclustered"
                        | "xlbarclustered"
                        | "xlline"
                        | "xlpie"
                )
            {
                return Err(err(format!("{name} does not accept arguments")));
            }
            let si = self.doc()?.wb.active_sheet;
            match key.as_str() {
                "application" => return self.wrap(Object::Application),
                "thisworkbook" | "activeworkbook" => return self.wrap(Object::Workbook),
                "activesheet" => return self.wrap(Object::Worksheet(si)),
                "worksheetfunction" => return self.wrap(Object::WorksheetFunction),
                "debug" => return self.wrap(Object::Debug),
                "worksheets" | "sheets" => {
                    return if args.is_empty() {
                        self.wrap(Object::Worksheets)
                    } else if args.len() == 1 {
                        let i = self.sheet_index(arg(args, 0)?)?;
                        self.wrap(Object::Worksheet(i))
                    } else {
                        Err(err("Worksheets requires one index"))
                    };
                }
                "range" => {
                    let (si, r) = self.range(si, args, None)?;
                    return self.wrap(Object::Range(si, r));
                }
                "cells" => return self.cells(si, RangeRef::all(), args),
                "rows" => return self.get_sheet_member(si, &key, args),
                "columns" => return self.get_sheet_member(si, &key, args),
                "selection" => return self.wrap(Object::Range(si, self.doc()?.selection.current())),
                "activecell" => return self.wrap(Object::Range(si, RangeRef::cell(self.doc()?.selection.active))),
                "rgb" => {
                    if args.len() != 3 {
                        return Err(err("RGB expects three arguments"));
                    }
                    let mut n = 0u32;
                    for (i, a) in args.iter().enumerate() {
                        let c = number(a)?;
                        if !(0.0..=255.0).contains(&c) {
                            return Err(err("RGB component out of range"));
                        }
                        n |= (c.round() as u32) << (i * 8);
                    }
                    return Ok(Value::Number(f64::from(n)));
                }
                "msgbox" => return Err(err("interactive MsgBox is not implemented; use Debug.Print for execution output")),
                "array" => return Ok(Value::Array(vec![args.to_vec()])),
                "xlup" => return Ok(Value::Number(-4162.0)),
                "xldown" => return Ok(Value::Number(-4121.0)),
                "xltoleft" => return Ok(Value::Number(-4159.0)),
                "xltoright" => return Ok(Value::Number(-4161.0)),
                "xlcolumnclustered" => return Ok(Value::Number(51.0)),
                "xlbarclustered" => return Ok(Value::Number(57.0)),
                "xlline" => return Ok(Value::Number(4.0)),
                "xlpie" => return Ok(Value::Number(5.0)),
                "calculate" => {
                    self.command("formulas.calculateNow", json!({}))?;
                    return Ok(Value::Empty);
                }
                _ => {}
            }
        }
        match object {
            Some(Object::Workbook) => match key.as_str() {
                "worksheets" | "sheets" => {
                    if args.is_empty() {
                        self.wrap(Object::Worksheets)
                    } else if args.len() == 1 {
                        let i = self.sheet_index(arg(args, 0)?)?;
                        self.wrap(Object::Worksheet(i))
                    } else {
                        Err(err("Worksheets requires one index"))
                    }
                }
                "name" => {
                    no_args(args, "Workbook.Name")?;
                    Ok(Value::Text(self.doc()?.display_title()))
                }
                _ => Err(err(format!("Workbook.{name} is not implemented"))),
            },
            Some(Object::Worksheets) => match key.as_str() {
                "count" => {
                    no_args(args, "Worksheets.Count")?;
                    Ok(Value::Number(self.doc()?.wb.sheets.len() as f64))
                }
                "" | "item" => {
                    if args.len() != 1 {
                        return Err(err("Worksheets.Item requires one index"));
                    }
                    let i = self.sheet_index(arg(args, 0)?)?;
                    self.wrap(Object::Worksheet(i))
                }
                "add" => {
                    if !args.is_empty() {
                        return Err(err("Worksheets.Add arguments are not implemented"));
                    }
                    self.command("home.insertSheet", json!({}))?;
                    self.wrap(Object::Worksheet(self.doc()?.wb.active_sheet))
                }
                _ => Err(err(format!("Worksheets.{name} is not implemented"))),
            },
            Some(Object::Worksheet(si)) => self.get_sheet_member(si, &key, args),
            Some(Object::Range(si, r)) => self.get_range_member(si, r, &key, args),
            Some(Object::Rows(si, r)) => self.get_axis_member(si, r, &key, args, true),
            Some(Object::Columns(si, r)) => self.get_axis_member(si, r, &key, args, false),
            Some(Object::WorksheetFunction) => self.worksheet_function(name, args),
            Some(Object::Debug) if key == "print" => {
                let mut parts = Vec::new();
                for v in args {
                    parts.push(text(v)?);
                }
                self.log(parts.join(" "))?;
                Ok(Value::Empty)
            }
            Some(Object::Font(si, r)) => {
                no_args(args, &format!("Font.{name}"))?;
                match key.as_str() {
                    "bold" => self.uniform_style(si, r, "Font.Bold", |st| Value::Bool(st.font.bold)),
                    "italic" => self.uniform_style(si, r, "Font.Italic", |st| Value::Bool(st.font.italic)),
                    "name" => self.uniform_style(si, r, "Font.Name", |st| Value::Text(st.font.name.clone())),
                    "size" => self.uniform_style(si, r, "Font.Size", |st| Value::Number(f64::from(st.font.size))),
                    _ => Err(err(format!("Font.{name} is not implemented"))),
                }
            }
            Some(Object::ChartObjects(si)) if key == "add" => {
                if args.len() != 4 || self.charts.len() >= 128 {
                    return Err(err("ChartObjects.Add requires four dimensions; at most 128 charts per run"));
                }
                let n = args.iter().map(number).collect::<Result<Vec<_>, _>>()?;
                if n.iter().any(|v| *v < 0.0 || *v > 100_000.0) || n.get(2) == Some(&0.0) || n.get(3) == Some(&0.0) {
                    return Err(err("invalid chart dimensions"));
                }
                let i = self.charts.len();
                self.charts.push(PendingChart {
                    sheet: si,
                    id: None,
                    left: n.first().copied().unwrap_or(0.0) * 4.0 / 3.0,
                    top: n.get(1).copied().unwrap_or(0.0) * 4.0 / 3.0,
                    width: n.get(2).copied().unwrap_or(300.0) * 4.0 / 3.0,
                    height: n.get(3).copied().unwrap_or(200.0) * 4.0 / 3.0,
                    kind: "column".into(),
                    title: None,
                });
                self.wrap(Object::EmbeddedChart(i))
            }
            Some(Object::EmbeddedChart(i)) if key == "chart" => {
                no_args(args, "ChartObject.Chart")?;
                self.wrap(Object::Chart(i))
            }
            Some(Object::Chart(i)) => match key.as_str() {
                "charttitle" => {
                    no_args(args, "Chart.ChartTitle")?;
                    self.wrap(Object::ChartTitle(i))
                }
                "hastitle" => {
                    no_args(args, "Chart.HasTitle")?;
                    Ok(Value::Bool(self.charts.get(i).ok_or_else(|| err("chart missing"))?.title.is_some()))
                }
                "charttype" => {
                    no_args(args, "Chart.ChartType")?;
                    let c = self.charts.get(i).ok_or_else(|| err("chart missing"))?;
                    let n = match c.kind.as_str() {
                        "column" => 51.0,
                        "bar" => 57.0,
                        "line" => 4.0,
                        "pie" => 5.0,
                        _ => return Err(err("this ChartType is not implemented")),
                    };
                    Ok(Value::Number(n))
                }
                "setsourcedata" => {
                    if args.len() != 1 {
                        return Err(err("SetSourceData requires one Range; PlotBy is not implemented"));
                    }
                    let Object::Range(si, r) = self.object(arg(args, 0)?)? else { return Err(err("SetSourceData requires a Range")) };
                    if r.count() > MAX_CELLS {
                        return Err(err("chart source exceeds 10,000 cells"));
                    }
                    let c = self.charts.get(i).ok_or_else(|| err("chart missing"))?;
                    let sheet = c.sheet;
                    let id = c.id;
                    if si != sheet {
                        return Err(err("chart source on another worksheet is not implemented"));
                    }
                    let sh = self.doc()?.wb.sheet(si).ok_or_else(|| err("source worksheet missing"))?;
                    let range = format!("{}!{}", gridcraft_formula::quote_sheet(&sh.name), r.a1());
                    if id.is_some() {
                        return Err(err("changing chart source a second time is not implemented"));
                    }
                    let p = json!({"sheet":sheet,"range":range,"type":c.kind,"title":c.title,"at":"A1","width":c.width,"height":c.height});
                    self.command("sheet.activate", json!({"sheet":sheet}))?;
                    let result = self.command("insert.chart", p)?;
                    let id = result
                        .get("chart")
                        .and_then(Json::as_u64)
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or_else(|| err("chart creation returned no id"))?;
                    if let Some(c) = self.charts.get_mut(i) {
                        c.id = Some(id);
                    }
                    self.chart_update(i)?;
                    Ok(Value::Empty)
                }
                _ => Err(err(format!("Chart.{name} is not implemented"))),
            },
            Some(Object::ChartTitle(i)) if key == "text" => {
                no_args(args, "ChartTitle.Text")?;
                let title = self.charts.get(i).and_then(|c| c.title.clone()).ok_or_else(|| err("chart has no title"))?;
                Ok(Value::Text(title))
            }
            _ => Err(err(format!("VBA member or function `{name}` is not implemented"))),
        }
    }
    fn set(&mut self, receiver: Option<&Value>, name: &str, args: &[Value], value: Value) -> Result<(), Error> {
        let key = name.to_ascii_lowercase();
        let object = receiver.map(|r| self.object(r)).transpose()?;
        if object.is_some() {
            no_args(args, &format!("{name} assignment"))?;
        }
        match object {
            None if key == "range" || key == "cells" => {
                let target = self.get(None, name, args)?;
                self.set(Some(&target), "value", &[], value)
            }
            Some(Object::Worksheet(si)) if key == "name" => {
                self.command("sheet.rename", json!({"sheet":si,"name":text(&value)?}))?;
                Ok(())
            }
            Some(Object::Range(si, r)) => match key.as_str() {
                "" | "value" | "value2" => self.write(si, r, value, false),
                "formula" | "formula2" => self.write(si, r, value, true),
                "numberformat" => {
                    self.check_range(r)?;
                    let code = self.style_text(r, &value)?;
                    self.command("home.numberFormat", json!({"sheet":si,"range":r.a1(),"code":code}))?;
                    Ok(())
                }
                _ => Err(err(format!("Range.{name} assignment is not implemented"))),
            },
            Some(Object::Font(si, r)) => {
                self.check_range(r)?;
                let mut p = json!({"sheet":si,"range":r.a1()});
                let command = match key.as_str() {
                    "bold" => {
                        p["on"] = json!(boolean(&value)?);
                        "home.bold"
                    }
                    "italic" => {
                        p["on"] = json!(boolean(&value)?);
                        "home.italic"
                    }
                    "name" => {
                        p["name"] = json!(self.style_text(r, &value)?);
                        "home.fontName"
                    }
                    "size" => {
                        p["size"] = json!(number(&value)?);
                        "home.fontSize"
                    }
                    "color" => {
                        p["color"] = json!(color(&value)?);
                        "home.fontColor"
                    }
                    _ => return Err(err(format!("Font.{name} assignment is not implemented"))),
                };
                self.command(command, p)?;
                Ok(())
            }
            Some(Object::Interior(si, r)) if key == "color" => {
                self.check_range(r)?;
                self.command("home.fillColor", json!({"sheet":si,"range":r.a1(),"color":color(&value)?}))?;
                Ok(())
            }
            Some(Object::Chart(i)) if key == "charttype" => {
                let kind = match number(&value)? {
                    51.0 => "column",
                    57.0 => "bar",
                    4.0 => "line",
                    5.0 => "pie",
                    _ => return Err(err("this ChartType is not implemented (an exact supported integer constant is required)")),
                };
                let c = self.charts.get_mut(i).ok_or_else(|| err("chart missing"))?;
                c.kind = kind.into();
                self.chart_update(i)
            }
            Some(Object::Chart(i)) if key == "hastitle" => {
                let on = boolean(&value)?;
                let c = self.charts.get_mut(i).ok_or_else(|| err("chart missing"))?;
                if on {
                    c.title.get_or_insert_with(String::new);
                } else {
                    c.title = None;
                }
                self.chart_update(i)
            }
            Some(Object::ChartTitle(i)) if key == "text" => {
                let title = text(&value)?;
                let c = self.charts.get_mut(i).ok_or_else(|| err("chart missing"))?;
                c.title = Some(title);
                self.chart_update(i)
            }
            _ => Err(err(format!("VBA property `{name}` assignment is not implemented"))),
        }
    }
}

impl ExcelHost {
    fn style_text(&mut self, r: RangeRef, value: &Value) -> Result<String, Error> {
        let text = text(value)?;
        if text.len() > 1024 {
            return Err(err("VBA font names and number formats are limited to 1,024 bytes"));
        }
        // Different existing styles can each retain a copy of the new text. Charge the
        // expanded range even when the style table can deduplicate some of those copies.
        let bytes = (text.len() as u64).checked_mul(r.count()).ok_or_else(|| err("style text byte size overflow"))?;
        let total = self.cell_value_bytes.checked_add(bytes).ok_or_else(|| err("style text byte size overflow"))?;
        if total > MAX_CELL_VALUE_BYTES {
            return Err(err("macro cell values and style text exceed the 16 MiB total byte limit"));
        }
        self.cell_value_bytes = total;
        Ok(text)
    }
    fn check_range(&self, r: RangeRef) -> Result<(), Error> {
        if r.count() > MAX_CELLS { Err(err("range exceeds 10,000 cells per operation")) } else { Ok(()) }
    }
    fn uniform_style(&self, si: usize, r: RangeRef, name: &str, read: impl Fn(&gridcraft_model::Style) -> Value) -> Result<Value, Error> {
        self.check_range(r)?;
        let d = self.doc()?;
        let sh = d.wb.sheet(si).ok_or_else(|| err("worksheet missing"))?;
        let first = read(d.wb.styles.get(sh.style_id(r.start)));
        for cell in r.iter() {
            if read(d.wb.styles.get(sh.style_id(cell))) != first {
                return Err(err(format!("mixed {name} values require VBA Null, which is not implemented")));
            }
        }
        Ok(first)
    }
    fn get_axis_member(&mut self, si: usize, r: RangeRef, key: &str, args: &[Value], rows: bool) -> Result<Value, Error> {
        let name = if rows { "Rows" } else { "Columns" };
        let count = if rows { r.height() } else { r.width() };
        match key {
            "count" => {
                no_args(args, &format!("{name}.Count"))?;
                Ok(Value::Number(f64::from(count)))
            }
            "insert" | "delete" => {
                no_args(args, &format!("{name}.{key}"))?;
                if u64::from(count) > MAX_CELLS {
                    return Err(err(format!("{name}.{key} exceeds 10,000 lines per operation")));
                }
                self.operations = self.operations.saturating_add(u64::from(count));
                if self.operations > MAX_OPERATIONS {
                    return Err(err("macro changed too many rows or columns"));
                }
                let (id, parameter, range) = if rows {
                    (if key == "insert" { "home.insertRows" } else { "home.deleteRows" }, "rows", RangeRef::rows(r.start.row, r.end.row))
                } else {
                    (if key == "insert" { "home.insertColumns" } else { "home.deleteColumns" }, "cols", RangeRef::cols(r.start.col, r.end.col))
                };
                let mut params = json!({"sheet": si});
                params[parameter] = json!(range.a1());
                self.command(id, params)?;
                Ok(Value::Empty)
            }
            "autofit" => {
                no_args(args, &format!("{name}.AutoFit"))?;
                if u64::from(count) > MAX_CELLS {
                    return Err(err(format!("{name}.AutoFit exceeds 10,000 lines per operation")));
                }
                self.operations = self.operations.saturating_add(u64::from(count));
                if self.operations > MAX_OPERATIONS {
                    return Err(err("macro changed too many rows or columns"));
                }
                if rows {
                    self.command("home.autofitRowHeight", json!({"sheet":si,"rows":format!("{}:{}",r.start.row+1,r.end.row+1)}))?;
                } else {
                    self.command("home.autofitColumnWidth",json!({"sheet":si,"cols":format!("{}:{}",gridcraft_core::col_to_letters(r.start.col),gridcraft_core::col_to_letters(r.end.col))}))?;
                }
                Ok(Value::Empty)
            }
            "" | "item" => {
                if args.len() != 1 {
                    return Err(err(format!("{name}.Item requires one index")));
                }
                let n = index(args, 0, count)?;
                let range = if rows {
                    let row = r.start.row + n;
                    RangeRef::new(CellRef::new(row, r.start.col), CellRef::new(row, r.end.col))
                } else {
                    let col = r.start.col + n;
                    RangeRef::new(CellRef::new(r.start.row, col), CellRef::new(r.end.row, col))
                };
                self.wrap(Object::Range(si, range))
            }
            _ => Err(err(format!("{name}.{key} is not implemented"))),
        }
    }
    fn get_sheet_member(&mut self, si: usize, key: &str, args: &[Value]) -> Result<Value, Error> {
        if !args.is_empty() && matches!(key, "name" | "usedrange" | "activate" | "select" | "parent") {
            return Err(err(format!("Worksheet.{key} does not accept arguments")));
        }
        match key {
            "parent" => self.wrap(Object::Workbook),
            "name" => Ok(Value::Text(self.doc()?.wb.sheet(si).ok_or_else(|| err("worksheet missing"))?.name.clone())),
            "range" => {
                let (target, r) = self.range(si, args, None)?;
                if target != si {
                    return Err(err("Range address names a different worksheet than its receiver"));
                }
                self.wrap(Object::Range(si, r))
            }
            "cells" => self.cells(si, RangeRef::all(), args),
            "rows" => {
                if args.len() > 1 {
                    return Err(err("Rows expects at most one index"));
                }
                let r = if args.is_empty() {
                    RangeRef::all()
                } else {
                    let i = index(args, 0, MAX_ROWS)?;
                    RangeRef::rows(i, i)
                };
                self.wrap(Object::Rows(si, r))
            }
            "columns" => {
                if args.len() > 1 {
                    return Err(err("Columns expects at most one index"));
                }
                let r = if args.is_empty() {
                    RangeRef::all()
                } else {
                    let i = index(args, 0, MAX_COLS)?;
                    RangeRef::cols(i, i)
                };
                self.wrap(Object::Columns(si, r))
            }
            "usedrange" => {
                let r = self.doc()?.wb.sheet(si).ok_or_else(|| err("worksheet missing"))?.used_range().unwrap_or_default();
                self.wrap(Object::Range(si, r))
            }
            "chartobjects" => {
                if !args.is_empty() {
                    return Err(err("existing ChartObjects indexing is not implemented"));
                }
                self.wrap(Object::ChartObjects(si))
            }
            "activate" | "select" => {
                self.command("sheet.activate", json!({"sheet":si}))?;
                Ok(Value::Empty)
            }
            _ => Err(err(format!("Worksheet.{key} is not implemented"))),
        }
    }
    fn get_range_member(&mut self, si: usize, r: RangeRef, key: &str, args: &[Value]) -> Result<Value, Error> {
        if !args.is_empty()
            && matches!(
                key,
                "value"
                    | "value2"
                    | "formula"
                    | "formula2"
                    | "row"
                    | "column"
                    | "count"
                    | "countlarge"
                    | "font"
                    | "interior"
                    | "clearcontents"
                    | "clear"
                    | "select"
                    | "activate"
                    | "numberformat"
                    | "parent"
                    | "worksheet"
                    | "entirerow"
                    | "entirecolumn"
            )
        {
            return Err(err(format!("Range.{key} arguments are not implemented")));
        }
        match key {
            "parent" | "worksheet" => self.wrap(Object::Worksheet(si)),
            "" if args.is_empty() => self.read(si, r, false),
            "value" | "value2" => self.read(si, r, false),
            "formula" | "formula2" => self.read(si, r, true),
            "" | "item" | "cells" => self.cells(si, r, args),
            "range" => {
                let (si, range) = self.range(si, args, Some(r))?;
                self.wrap(Object::Range(si, range))
            }
            "row" => Ok(Value::Number(f64::from(r.start.row) + 1.0)),
            "column" => Ok(Value::Number(f64::from(r.start.col) + 1.0)),
            "count" | "countlarge" => Ok(Value::Number(r.count() as f64)),
            "address" => {
                if !args.is_empty() {
                    return Err(err("Address arguments are not implemented"));
                }
                let a = format!("${}${}", gridcraft_core::col_to_letters(r.start.col), r.start.row + 1);
                let b = format!("${}${}", gridcraft_core::col_to_letters(r.end.col), r.end.row + 1);
                Ok(Value::Text(if r.is_single() { a } else { format!("{a}:{b}") }))
            }
            "entirerow" => self.wrap(Object::Rows(si, RangeRef::rows(r.start.row, r.end.row))),
            "entirecolumn" => self.wrap(Object::Columns(si, RangeRef::cols(r.start.col, r.end.col))),
            "font" => self.wrap(Object::Font(si, r)),
            "interior" => self.wrap(Object::Interior(si, r)),
            "rows" => {
                if args.is_empty() {
                    self.wrap(Object::Rows(si, r))
                } else if args.len() == 1 {
                    let n = index(args, 0, r.height())?;
                    let row = r.start.row + n;
                    self.wrap(Object::Range(si, RangeRef::new(CellRef::new(row, r.start.col), CellRef::new(row, r.end.col))))
                } else {
                    Err(err("Rows expects at most one index"))
                }
            }
            "columns" => {
                if args.is_empty() {
                    self.wrap(Object::Columns(si, r))
                } else if args.len() == 1 {
                    let n = index(args, 0, r.width())?;
                    let col = r.start.col + n;
                    self.wrap(Object::Range(si, RangeRef::new(CellRef::new(r.start.row, col), CellRef::new(r.end.row, col))))
                } else {
                    Err(err("Columns expects at most one index"))
                }
            }
            "offset" => {
                if args.len() > 2 {
                    return Err(err("Offset expects at most two offsets"));
                }
                let dr = args.first().map(number).transpose()?.unwrap_or(0.0);
                let dc = args.get(1).map(number).transpose()?.unwrap_or(0.0);
                if dr.fract() != 0.0 || dc.fract() != 0.0 || dr.abs() > f64::from(MAX_ROWS) || dc.abs() > f64::from(MAX_COLS) {
                    return Err(err("Offset requires integer offsets within worksheet dimensions"));
                }
                let a = r.start.offset(dr as i64, dc as i64).ok_or_else(|| err("Offset leaves worksheet"))?;
                let b = r.end.offset(dr as i64, dc as i64).ok_or_else(|| err("Offset leaves worksheet"))?;
                self.wrap(Object::Range(si, RangeRef::new(a, b)))
            }
            "resize" => {
                if args.len() > 2 {
                    return Err(err("Resize expects at most two dimensions"));
                }
                let rows = args.first().map(number).transpose()?.unwrap_or(f64::from(r.height()));
                let cols = args.get(1).map(number).transpose()?.unwrap_or(f64::from(r.width()));
                if rows < 1.0 || cols < 1.0 || rows.fract() != 0.0 || cols.fract() != 0.0 || rows > f64::from(MAX_ROWS) || cols > f64::from(MAX_COLS)
                {
                    return Err(err("Resize requires positive integer dimensions within worksheet limits"));
                }
                let b = r.start.offset(rows as i64 - 1, cols as i64 - 1).ok_or_else(|| err("Resize leaves worksheet"))?;
                self.wrap(Object::Range(si, RangeRef::new(r.start, b)))
            }
            "end" => {
                if args.len() != 1 {
                    return Err(err("End requires one direction"));
                }
                let (dir_r, dir_c) = match number(arg(args, 0)?)? {
                    -4162.0 => (-1, 0),
                    -4121.0 => (1, 0),
                    -4159.0 => (0, -1),
                    -4161.0 => (0, 1),
                    _ => return Err(err("invalid End direction (an exact supported integer constant is required)")),
                };
                let sh = self.doc()?.wb.sheet(si).ok_or_else(|| err("worksheet missing"))?;
                let cell = crate::selection::jump(sh, r.start, dir_r, dir_c);
                self.wrap(Object::Range(si, RangeRef::cell(cell)))
            }
            "clearcontents" | "clear" => {
                self.check_range(r)?;
                let id = if key == "clear" { "edit.clearAll" } else { "edit.clearContents" };
                self.command(id, json!({"sheet":si,"range":r.a1()}))?;
                Ok(Value::Empty)
            }
            "select" | "activate" => {
                self.command("selection.set", json!({"sheet":si,"range":r.a1()}))?;
                Ok(Value::Empty)
            }
            "numberformat" => self.uniform_style(si, r, "NumberFormat", |st| Value::Text(st.num_fmt.0.to_string())),
            _ => Err(err(format!("Range.{key} is not implemented"))),
        }
    }
}

fn from_cell(value: &CellValue) -> Result<Value, Error> {
    match value {
        CellValue::Empty => Ok(Value::Empty),
        CellValue::Number(n) => Ok(Value::Number(*n)),
        CellValue::Bool(b) => Ok(Value::Bool(*b)),
        CellValue::Text(s) => Ok(Value::Text(s.to_string())),
        CellValue::Error(e) => Err(err(format!("worksheet error: {e}"))),
        _ => Err(err("this worksheet value type is not implemented in VBA")),
    }
}
fn color(value: &Value) -> Result<String, Error> {
    let n = number(value)?;
    if n < 0.0 || n > 16_777_215.0 || n.fract() != 0.0 {
        return Err(err("invalid VBA RGB color"));
    }
    let n = n as u32;
    Ok(format!("#{:02X}{:02X}{:02X}", n & 255, (n >> 8) & 255, (n >> 16) & 255))
}
