# Excel VBA compatibility

> **Last reviewed:** 2026-10-23 · **Last updated:** 2026-10-23 · **Change:** major (document the local VBA foundation and the full compatibility requirement) · **Target:** Microsoft Excel (Microsoft 365), including Windows VBA features

**Full compatibility with macros authored in the commercial version of Excel is the final
requirement. It has not been achieved.** The current local implementation provides source
import, an editor and a bounded interpreter for a small subset of VBA and the Excel object
model. These are implementation stages toward that requirement, not a reduction of its scope.

This inventory is based on source inspection on 2026-10-23. The local validation below covers
CI and selected Chrome editor workflows, not an Excel-authored macro corpus or full compatibility.
Readiness percentages in the existing parity documents have not been raised. Their historical
hour estimates do not include the newly required full VBA compatibility; that work is **not yet
estimated**.

## Implemented foundation

| Area | Present in the local source | Boundary |
|---|---|---|
| Original project | `Workbook::vba_project` retains the original `vbaProject.bin`; macro-enabled `.xlsm`/`.xltm` exports write its bytes back | Preservation of these bytes does not establish complete package, signature or runtime compatibility |
| Source import | `crates/xlsx/src/vba.rs` reads CFB streams, MS-OVBA compressed source and the declared code page; reports module names, source and kinds | Does not execute the compiled performance cache, resolve referenced libraries, remove protection or create/edit a VBA project |
| Language | `crates/engine/src/vba/language.rs` compiles source into cooperative instructions and reports line numbers | Explicit subset, not the complete VBA compiler/runtime |
| Excel adapter | `crates/engine/src/vba/host.rs` maps supported object operations to existing workbook commands | Unsupported members return errors; GridCraft's calculation and chart behaviour are not an Excel conformance oracle |
| Session | `crates/engine/src/vba/mod.rs` stages macro edits and exposes status, output, stop and commit | One running macro per session; changes publish only after successful completion |
| Editor | `crates/ui-egui/src/vba_editor.rs`, reached through Macros → VBA Editor | Module source, procedure choice, save, run, stop, line/error/output display and a user-inserted example; no full VBE debugger or form designer |

The editor can show imported standard, document, class and form source. A module's reported
kind is import metadata, not evidence that class, UserForm or event execution is implemented.

## Language and object model currently represented

- Parameterless `Sub` procedures, optional `Public`/`Private`, `Option Explicit`, selected
  exported module attributes, comments, line continuation, local scalar `Dim`, assignment,
  `Set`, same-module parameterless procedure calls, arithmetic/comparison/Boolean operators,
  string concatenation, `If`/`ElseIf`/`Else`, `For`/`Next`/`Step`, `Do`/`Loop`,
  `While`/`Wend`, `With` and supported `Exit` statements.
- Selected scalar types and built-ins: Variant, object references, String, Boolean, Double,
  Single, Integer, Long and Byte; `CStr`, `CDbl`, `CInt`, `CLng`, `CBool`, `Len`, `UCase`,
  `LCase`, `Trim`, `Abs`, `Int`, `Fix`, `Sqr`, and selected `vb*` string constants.
  These names do not imply complete VBA coercion, locale or numeric semantics.
- `Application`, `ThisWorkbook`/`ActiveWorkbook`, `ActiveSheet`, `Worksheets`/`Sheets`
  indexing/count and argumentless `Add`, worksheet name/activation, `UsedRange`, `Selection`
  and `ActiveCell`. Only the current workbook is exposed.
- `Range`, `Cells`, row/column access and counts, `Offset`, `Resize`, `End`, `Address`,
  `Value`/`Value2`, `Formula`/`Formula2`, `NumberFormat`, `ClearContents`, `Clear`, selection,
  row/column `AutoFit`, selected font properties, fill colour and `RGB`. Argument variants,
  mixed formats, default properties and array assignments are only partially represented.
- `WorksheetFunction` calls use GridCraft's formula evaluator; `Calculate` and `Debug.Print`
  are available. `Debug.Print` writes to the editor output, rather than an external console.
- A limited new-chart path: `ChartObjects.Add`, `Chart.SetSourceData` from the same worksheet,
  column/bar/line/pie `ChartType`, `HasTitle` and `ChartTitle.Text`. Existing chart indexing,
  repeated source changes, other chart types and the complete chart object model are missing.

The compiler processes a whole module. An unsupported declaration or statement elsewhere in
that module can prevent a selected procedure from compiling. A module with a compile error
can still be saved for further editing. Unsupported code must be diagnosed rather than
silently skipped.

## Known missing compatibility

The remaining requirement includes the complete language and standard library; parameterized
procedures, `Function`, `ByRef`/`ByVal`, optional/named arguments, module/global state,
arrays and `ReDim`, `For Each`, `Select Case`, `On Error`/`Err`/`Resume`, full type/coercion
and date/currency semantics, class/property modules, events, references and cross-module calls.
The current subset is not enough to run arbitrary Excel VBA unchanged.

The Excel object model still lacks most properties, methods and collections, multi-workbook
workflows, workbook/worksheet events, macro UDF integration, UserForms and their controls,
interactive `MsgBox`/`InputBox`, and the VBE debugger/designer. Add-in lifecycle support for
`.xlam`/`.xla`, COM/ActiveX automation, `CreateObject`, external DLL/Windows API calls,
file/process/network operations and trusted-location/signature policy are not implemented.
Excel 4.0 macro sheets are a separate unimplemented execution format.

Full compatibility also requires writing edited source back into an Excel VBA project and
verifying the resulting workbook in Excel. Existing preservation and source extraction do
not satisfy that requirement. Windows-dependent macros will require an explicit execution
architecture for the native dependencies; the current browser interpreter supplies none.
The choice and its compatibility tests remain design work, not a promise that a browser-only
runtime can reproduce every Windows feature.

## Execution, trust and current safety limits

Execution checks the workbook's session trust for both stored and explicitly supplied source.
A new blank workbook starts trusted. A workbook loaded with a VBA project, GridCraft source
metadata or recorded scripts starts untrusted and needs the explicit **Trust Scripts** action
before any VBA runs. Trust initialization checks content, not the filename extension; a loaded
workbook with none of these can start trusted. Opening a file or inspecting/importing source
never runs VBA automatically;
Excel auto-open and worksheet event dispatch are not implemented. Trust does not enable any
unimplemented host capability.

A run edits a private workbook snapshot. Successful completion publishes workbook changes as
one Undo step; a runtime error, stop or limit failure discards them. Changing, closing or switching
the active workbook, or revoking trust during a run, invalidates its pending commit. Output and
error status remain available. These transaction semantics intentionally differ from Excel's
usual handling of macro edits and are a current compatibility gap.

There is no arbitrary engine-command, file, URL, process or COM access through the VBA host.
`Shell`, `CreateObject` and unknown members are rejected. The cooperative VM permits the UI to
poll and stop between execution slices; it is not a hard wall-clock limit on each individual
formula, calculation or host operation.

| Limit in the current code | Value |
|---|---|
| VBA project source import | 32 MiB project; 1 MiB decompressed directory; 128 modules; 1 MiB decoded source per module; 8 MiB total source |
| Edited source storage | 128 modules; 1 MiB per module; 8 MiB total; module names at most 128 bytes |
| Compiler | 65,536 tokens and 16,384 source statements per module; compiled-instruction and nesting/call-depth bounds; 1,024 declared variables per procedure |
| Session execution | 1,000,000 VM instructions; `pollVba` budget of 1–1,000 instructions per call |
| Workbook adapter | 10,000 cells per bounded range operation; 1,000,000 operation/cell-change budget; 16 MiB cumulative written cell values/formulas and expanded style text; 1 MiB per read cell and 16 MiB per range read; 1,024 bytes per font name/number format; 10,000 AutoFit rows/columns per operation; 32,768 object references; 128 new charts per run |
| Output | 1,000 entries and 256 KiB combined text |

Additional VM bounds cover strings, arrays and retained values. Exact limits are defined in
the source; they are current safeguards, not Excel-compatible limits or a measured performance
guarantee. Unknown encodings and malformed/compressed source are errors. Character conversion
uses `encoding_rs` mappings, including Windows-31J for code page 932, and is not a claim of
identical behaviour for all Windows code pages.

## Source save and Excel handoff

**Save module stores GridCraft's copy of source in workbook metadata. It does not rewrite
`vbaProject.bin`.** The metadata is JSON in the hidden workbook-level defined name
`_gc_vba_sources_v1`. A saved module with the same name takes precedence over imported source
inside GridCraft. The original project remains unchanged.

Saving a workbook as `.xlsm`/`.xltm` retains the original project bytes if one was loaded.
Saving as `.xlsx`/`.xltx` omits that binary project. GridCraft source metadata can survive its
own XLSX writer/reader, but it is not an Excel executable VBA project. Inserting a new module
into a blank workbook and saving it as `.xlsm` does not compile or generate `vbaProject.bin`.
Excel will still execute the original binary project's code, if present, rather than the
edited GridCraft metadata. Large source metadata and resulting files still need validation
in Excel; successful internal round trips cannot establish Excel handoff compatibility.

## Commands and verification work

The UI, CLI and MCP share these engine commands:

| Command | Purpose |
|---|---|
| `automate.vbaModules` | List stored/imported modules, procedure/compile diagnostics, trust, preservation and import error status |
| `automate.saveVbaModule` | Store a module's source in GridCraft metadata |
| `automate.startVba` | Compile a module or supplied source and begin the selected procedure |
| `automate.pollVba` | Advance a bounded slice and publish only a successful completed run |
| `automate.stopVba` | Stop the run and discard pending workbook edits |
| `automate.vbaStatus` | Inspect idle/running/completed/stopped/failed state, line, steps, log and error |
| `automate.trustScripts` | Set the current workbook's session trust explicitly |

Regression tests cover language semantics, bounded source import, workbook adapter
operations, atomic completion/error/stop, Undo/Redo, trust and editor interactions
(`crates/engine/src/vba/{language,tests}.rs`, `crates/xlsx/src/vba.rs`,
`crates/ui-egui/tests/vba_editor.rs`). Remaining evidence includes browser workbook file
save/reopen, Excel opening our output, and a pinned clean-room corpus comparing results,
side effects and errors against Excel. Internal source round trips do not prove that Excel
will accept the metadata or execute edited source.

### Local validation — 2026-10-23

- `cargo xtask ci` passed all eight gates: fmt, workspace clippy with warnings denied,
  workspace tests, assets, layers, Wasm check, parity and keytips. The run recorded 2,206
  passing tests and one pre-existing timing-sensitive ignored test. The Wasm check retains
  the existing `backstage.rs` dead-code warning; it is not a clean Wasm-clippy result.
- Trunk 0.22.0 built the Web app into a fresh output directory. Chrome loaded the resulting
  Wasm on `127.0.0.1:8771`.
- In a new blank workbook, a pasted `BuildReport` procedure used a loop, Japanese text,
  cell values, a SUM formula, bold, number format and AutoFit to produce a total of 1,500.
  Save module, closing/reopening the editor and executing the stored module succeeded.
- An unsupported `Shell` statement produced a compile diagnostic at line 3. A separate
  unknown Range member failed at runtime after a staged write; the original A1 stayed blank.
  Stop VBA stopped a `Do`/`Loop` run and discarded its pending A1 write.
- These checks used paste and UI actions. Real OS Japanese IME, Safari, browser workbook
  file save/reopen and execution of Excel-authored macro projects remain unverified.

Next development should establish those baselines, expand language and object-model coverage
against reproducible Excel observations, implement project writing and form/event support,
and decide how native Windows dependencies execute. Every supported stage must retain explicit
diagnostics and the final full compatibility requirement. Completion criteria and effort for
the whole requirement remain to be measured.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-23 | major | Created from the local source: full VBA compatibility required but unmet; source preservation/import, partial interpreter/editor, trust, limits, metadata-only source save; recorded passing CI and selected Chrome workflows while retaining Excel verification gaps |
