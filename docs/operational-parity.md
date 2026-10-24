# Operational parity verification

Source: `ai-team/reports/gemini-excel-parity-20261024-080500.md` (read in full).

Command registration is a breadth inventory, not proof of Excel compatibility. `cargo xtask parity`
reports **registered IDs**. This document tracks observable behavior and regression evidence;
the tests must pass independently in `cargo xtask ci`. Browser permission and OS IME behavior
also require live browser checks. No numerical Excel compatibility claim is made.

Implementation is integrated (2026-10-24). The final `cargo xtask ci` passed all eight steps:
2,321 tests passed, none failed, and one existing test was ignored (158 suites).
Log: `../outputs/GridCraft-ExcelParity-CI-Complete-20261024T1800.log`.
The following contracts are covered by the regression suite.
The owner subsequently reported completion of Antigravity/Gemini's code and test review with
no issues found. The review integration is identified as `EXCEL.1`; the remaining browser,
language and executable-project work is collected in `PROJECT_STATUS.md` §14.

| Report item | Required behavior | Verification scope |
|---|---|---|
| Root A | Registration, UI placement and operational evidence reported separately | parity renderer tests and this inventory |
| Root B / 1 | Successful workbook edits end copy mode; failed commands and queries preserve it; copied paste repeats | `engine/tests/multi_area_operations.rs` (including partial-error restoration) |
| Root C / 6 | All structural areas, merged line spans, reverse order, cumulative overflow, one Undo | `engine/tests/multi_area_operations.rs` |
| Root D / 11 | User-gesture async Web clipboard, internal fallback, stale destination guard | `ui-egui/tests/async_clipboard.rs`, WASM compilation, browser |
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
| 13 | Text Cut/Copy/Paste/Select All menus; recorded script deletion | `ui-egui/tests/formula_text_menu.rs`, `vba_text_menu.rs`, `macro_controls.rs` |
| 14 | Japanese static and parameterized formula errors | `i18n::tests::japanese_formula_errors_translate_structure_without_losing_parameters` |
| 15 | Picture placement conversion, selection, all eight handles, clipboard images | `engine/tests/picture_placement.rs`; `ui-egui/tests/floating_pictures.rs`, `clipboard_images.rs`, `async_clipboard.rs`; WASM |
| 16 | Sheet modifier selection, grouped atomic operations, 3D point references | `engine/tests/grouped_sheets.rs`; `ui-egui/tests/grouped_sheet_tabs.rs`, `formula_sheet_spans.rs`; editor unit tests |

The source report permits unsupported recorded commands to remain explicit VBA comments. The
stop result must expose these commands, and generated source must compile; a compilable comment
does not assert that the corresponding operation was replayed. Preserve original imported VBA
module kinds and existing user procedures when saving a recording.

Recorded row/column insertion and deletion use executable `EntireRow`/`EntireColumn` VBA
statements and have replay tests. Unsupported operations are returned in `unsupported` with
`vbaComplete: false`. Recorded source is stored in GridCraft metadata; writing an executable
Excel `vbaProject.bin` remains outside the current implementation.

The final Trunk build is `dist/web-excel-parity-complete-20261024-v2`, served at
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
