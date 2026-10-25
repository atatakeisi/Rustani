# Operational parity verification

> **Last updated:** 2026-10-25 · Items 17–20 implemented; final integration CI, refreshed Web build and Chrome checks passed.

Source: `ai-team/reports/gemini-excel-parity-20261024-080500.md` (read in full).

Command registration is a breadth inventory, not proof of Excel compatibility. `cargo xtask parity`
reports **registered IDs**. This document tracks observable behavior and regression evidence;
the tests must pass independently in `cargo xtask ci`. Browser permission and OS IME behavior
also require live browser checks. No numerical Excel compatibility claim is made.

Implementation was integrated on 2026-10-24. That integration's final `cargo xtask ci` passed all eight steps:
2,321 tests passed, none failed, and one existing test was ignored (158 suites).
Log: `../outputs/GridCraft-ExcelParity-CI-Complete-20261024T1800.log`.
The inventory below also includes the follow-up items 17–20. Their targeted UI regressions
and final integration CI passed; dated results are recorded below, separately from Web checks.
The owner subsequently reported completion of Antigravity/Gemini's code and test review with
no issues found. The review integration is identified as `EXCEL.1`; the remaining browser,
language and executable-project work is collected in `PROJECT_STATUS.md` §14.

| Report item | Required behavior | Verification scope |
|---|---|---|
| Root A | Registration, UI placement and operational evidence reported separately | parity renderer tests and this inventory |
| Root B / 1 | Successful workbook edits end copy mode; failed commands and queries preserve it; copied paste repeats | `engine/tests/multi_area_operations.rs` (including partial-error restoration) |
| Root C / 6 | All structural areas, merged line spans, reverse order, cumulative overflow, one Undo | `engine/tests/multi_area_operations.rs` |
| Root D / 11 | User-gesture async Web clipboard, internal fallback, stale destination guard; follow-up exception handling described below | `ui-egui/tests/async_clipboard.rs`; dated item 17 WASM/browser evidence below |
| Root E / 3 | macOS Control+Enter and Command+Enter fill every selected area | `ui-egui/tests/ime_input.rs` |
| 0 | Macros and Record/Stop in each of quick access/View/Automate/status; menu command opens dialog, headless remains query | `ui-egui/tests/macro_controls.rs`; `async_clipboard.rs` |
| 0b | Red circle/square recording controls; pending edit commits before stopping; WMR; context-aware valid VBA appended to Module1, with a new standard ModuleN when Module1 cannot safely accept it | `engine/tests/macro_recorder.rs`; `macro_controls.rs`; generated keytip tests |
| 2 | Fill in four directions over every area, atomic history | `engine/tests/multi_area_operations.rs` |
| 4 | Backspace enters empty active-cell edit; Delete clears full selection | `ui-egui/tests/selection.rs` |
| 5 | Additive click removes selected cells without overlapping rectangles | `ui-egui/tests/selection.rs` |
| 7 | Row/column selection expands every area | `engine/tests/multi_area_operations.rs` |
| 8 | Replace All respects selected areas; Replace advances | `engine/tests/multi_area_operations.rs` |
| 9 | AutoSum each area; Define Name absolute union | `engine/tests/multi_area_formulas.rs` |
| 10 | Double-click Format Painter remains armed across applications | `ui-egui/tests/format_painter.rs` (actual double-click timing) |
| 12 | Formula/cell editor focus and IME target on opening frame | `ui-egui/tests/ime_input.rs` |
| 13 | Text Cut/Copy/Paste/Select All menus; destination-scoped async editor Paste; recorded script deletion | `ui-egui/tests/formula_text_menu.rs`, `vba_text_menu.rs`, `macro_controls.rs`; follow-up verification below |
| 14 | Japanese static and parameterized formula errors | `i18n::tests::japanese_formula_errors_translate_structure_without_losing_parameters` |
| 15 | Picture placement conversion, selection, all eight handles, clipboard images | `engine/tests/picture_placement.rs`; `ui-egui/tests/floating_pictures.rs`, `clipboard_images.rs`, `async_clipboard.rs`; WASM |
| 16 | Sheet modifier selection, grouped atomic operations, 3D point references | `engine/tests/grouped_sheets.rs`; `ui-egui/tests/grouped_sheet_tabs.rs`, `formula_sheet_spans.rs`; editor unit tests |
| 17 (follow-up) | Stable VBA source ID, scoped async Paste, browser exception handling and isolated draft Undo | `ui-egui/tests/vba_text_menu.rs`; 2026-10-25 evidence below; Safari live revalidation remains incomplete |
| 18 | More Functions category submenus and category-prefiltered Insert Function dialog | `function_library.rs`: seven tests cover all 13 category footers, seven More Functions submenus, Japanese labels and live category/search filtering |
| 19 | The same complete AutoSum menu on Home and Formulas | `function_library.rs` and `keytips.rs`: both tabs, five functions and evaluated results, More Functions route and all 12 HU/MU choices |
| 20 | Insert a selected function at the formula caret/selection and preserve TextEdit Undo | `function_insertion.rs`: nine tests cover both editors, Unicode selection replacement, one-step Undo/Redo, dialog confirmation, captured locale and origin cell |

The source report permits unsupported recorded commands to remain explicit VBA comments. The
stop result must expose these commands, and generated source must compile; a compilable comment
does not assert that the corresponding operation was replayed. Preserve original imported VBA
module kinds and existing user procedures when saving a recording.

Recorded row/column insertion and deletion use executable `EntireRow`/`EntireColumn` VBA
statements and have replay tests. Unsupported operations are returned in `unsupported` with
`vbaComplete: false`. Recorded source is stored in GridCraft metadata; writing an executable
Excel `vbaProject.bin` remains outside the current implementation.

The 2026-10-24 final Trunk build was `dist/web-excel-parity-complete-20261024-v2`, served at
`http://127.0.0.1:8784/?sample=sales&lang=ja`. Build log:
`../outputs/GridCraft-ExcelParity-Web-Complete-20261024-v2.log`.
Chrome live checks covered Control+Enter, multi-area paste and one-step Undo, PNG paste and
selection, northwest picture resizing, picture-to-cell conversion, grouped edits with an
unchanged non-grouped sheet, and quick-access recording through status-bar stop to generated
Module1 VBA (including committing the pending cell edit). Browser error logs were empty.
The final controls and generated source are pictured in
`../outputs/GridCraft-ExcelParity-Final-Controls-20261024.jpg` and
`../outputs/GridCraft-ExcelParity-Final-VBA-20261024.jpg`.
Safari, real OS IME candidate conversion and Excel-authored VBA round trips remain unverified.

## 2026-10-25 editor clipboard follow-up

A Safari failure was reported when choosing Paste from the VBA editor's right-click menu.
The actual Safari panic stack has not been inspected, so the changes below do not establish
the cause of that failure or successful revalidation on Safari.

- Context-menu actions restore editor focus after popup layout/closure. Async Paste replies
  are excluded from the global synthetic-event queue and delivered only by the destination
  TextEdit's `text_menu::before_show` hook, preventing a modal reply from reaching the grid.
- The VBA source uses the stable widget ID `gridcraft.vba_source`. Each source lifetime owns
  an `Arc<()>`; pending requests hold a `Weak<()>`, so closing/reopening the editor or replacing
  its module/source invalidates earlier replies even when the widget ID is reused. A valid
  scoped request may restore absent focus; a different focused field causes it to be discarded.
- Web clipboard `read`, `readText`, `getType` and Blob `arrayBuffer` calls now use
  Result-returning `Reflect::get` and `Function::apply` paths. This catches synchronous
  JavaScript exceptions that the previous web-sys bindings did not catch, alongside existing
  Promise rejection handling, while beginning permission-sensitive reads in the user gesture.

The final targeted run passed eight VBA text-menu tests, one formula text-menu test and six
async-clipboard tests. Coverage includes Unicode replacement and TextEdit Undo, unchanged
workbook data/selection/history, close/reopen, focus loss/restoration, another focused field,
permission errors and a reply polled before a draft is replaced. A further regression reproduced
Undo restoring an old draft's source before the fix; resetting TextEdit state only when the
draft/source changes now isolates Undo histories while preserving Paste/Undo within a draft.

The item 17 final `cargo xtask ci` passed all eight steps: **2,328 passed, zero failed, one existing
ignored test** (158 suites). Log: `../outputs/GridCraft-Safari-VbaPaste-Final-CI-20261025.log`.
Its Trunk build was `dist/web-safari-vba-paste-20261025-v2`, served on `127.0.0.1:8784` at that verification.
Build log: `../outputs/GridCraft-Safari-VbaPaste-Final-Web-20261025.log`.
Chrome verified the final build's actual right-click Paste into the modal VBA source, Japanese
text and one-step Undo, with no browser error logs. Screenshot:
`../outputs/GridCraft-Safari-VbaPaste-Chrome-Final-20261025.jpg`.
Safari live revalidation was attempted but could not complete because native screen capture
failed. The 2,321-test result above belongs to the prior 2026-10-24 integration.

## 2026-10-25 function-library follow-up (items 18–20)

The appended report findings are implemented. The report's appended text has been
preserved without edits by Codex. The completed 2,328-test run above belongs to item 17 and
does not validate these new changes. Item 17's eight VBA regressions, stable ID/Weak scope,
editor-only Paste delivery, browser exception handling and draft Undo isolation are retained;
Safari live revalidation remains incomplete.

| Item | Implemented behavior | Regression evidence |
|---|---|---|
| 18 | More Functions exposes Statistical, Engineering, Information, Database, Compatibility, Web and Add-in/Automation submenus. Each category includes More Functions… opening Insert Function with that category selected | `function_library.rs`: all seven submenus, all 13 footers, category/search intersection and switching, Japanese labels; menu clicks do not start a draft behind the popup |
| 19 | Home and Formulas delegate to one AutoSum menu with Sum, Average, Count Numbers, Max, Min and More Functions… | `function_library.rs` / `keytips.rs`: five choices and their calculated values on both tabs, More Functions, HU/MU routing and Undo |
| 20 | Ribbon category and Insert Function choices insert `NAME(` at the existing formula's caret or replace selected text, retaining its surrounding expression and TextEdit Undo | `function_insertion.rs`: nine regressions; middle/end caret, Unicode replacement, Undo/Redo and selection restoration, dialog focus/Enter, captured locale and cross-sheet origin |

The final targeted run passed **438 tests** across eight suites, including the eight VBA Paste
regressions. Log: `../outputs/GridCraft-Functions-Regression-20261025.log`. The new selection
and dialog tests first reproduced selection loss and Enter being consumed by the source editor;
retaining the focused editor's selection and reserving dialog keys fixed both. Category tests also
reproduced a menu click starting an empty formula-bar draft; layer-aware hit testing fixed it.
The final integration `cargo xtask ci` passed all eight steps: **2,349 passed, zero failed,
one existing ignored test** (160 suites). Log:
`../outputs/GridCraft-Items17-20-CI-Complete-20261025.log`. This result covers the integrated
items 17–20, including regenerated AutoSum keytips and Japanese category-label regressions.
Trunk 0.22.0 built `dist/web-functions-20261025`, now served on `127.0.0.1:8784`.
WASM: `gridcraft-web-be236058a5e66552_bg.wasm`; build record:
`../outputs/GridCraft-Items17-20-Web-Build-20261025.log`. The old build directories were retained.
Chrome verified the seven categories, a Web-filtered function dialog and all six AutoSum menu
choices. It also verified inserting IF in the middle of `=10+20` to produce `=10+IF(20`,
one Undo restoring the original draft and Redo restoring the insertion. Browser error logs
were empty. Screenshot: `../outputs/GridCraft-Items17-20-Chrome-20261025.jpg`.
Safari live revalidation remains incomplete.

## Revision history

| Date | Change |
|---|---|
| 2026-10-25 | Complete items 18–20 and retain item 17's guards; 438 targeted passes and final eight-step CI passed with 2,349 tests across 160 suites. Refreshed Web deployed on 8784 and Chrome checked categories and insertion/Undo/Redo; preserve item 17's historical 2,328-test evidence and unresolved Safari live verification |
| 2026-10-25 | Record item 17's stable editor ID, scoped async Paste, caught browser exceptions, isolated draft Undo and eight VBA regressions; full CI passed 2,328 tests |
