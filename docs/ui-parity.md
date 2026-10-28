# UI parity

> **Last reviewed:** 2026-10-28 · **Last updated:** 2026-10-28 · **Change:** minor (live floating display customization) · **Target:** Microsoft Excel (Microsoft 365), Excel for Mac 16.113.4

Interaction, controls and feel, against Excel. Part of [target-app-parity.md](target-app-parity.md).
The application display name is now **Rustani** (item 22); internal crate and file names retain `gridcraft`. The Account/About page credits the upstream GridCraft project in plain text. Build and Pages publication status are recorded in [PROJECT_STATUS.md §19](../PROJECT_STATUS.md).
The earlier **UI/UX fidelity estimate of ~55% and 50–80 h of work** has not been reassessed.
Ribbon command coverage is measured separately in [parity-checklist.md](parity-checklist.md)
(275 / 291 registered IDs). Registration does not establish UI availability or operational
compatibility. Current behavior and regression evidence are in
[operational-parity.md](operational-parity.md) and [PROJECT_STATUS.md §13](../PROJECT_STATUS.md).
Safari and real OS IME behavior still require live verification.

Sources: `crates/ui-egui/src` (ribbon, grid, editor, formula bar, dialogs, panes),
`crates/engine/src/cmd` (shortcuts in command specs), the owner's black-box notes
(`plan/excel/observed-ui.md`, local only) and user issues. Dated issue references below are
context, not evidence that every original platform-specific report has been retested.

## Live display customization (2026-10-28)

A draggable, non-modal customization window opens over the worksheet from the title bar, View tab, cell context menu, and the Options page. Theme, scale, ribbon mode and Home group choices apply immediately. Changing group visibility or pinning switches to Home and expands the ribbon; if it was stored in the context menu, the normal ribbon is restored so the change is visible. Existing limitations on compact mode outside Home remain. A local offscreen screenshot and automated UI regressions cover the live preview; actual Safari and Chrome interaction requires separate verification.

## Window and chrome

| Element | Excel | Rustani | Status / remaining scope |
|---|---|---|---|
| Title bar, Quick Access Toolbar, AutoSave toggle | yes | yes | Earlier macOS window-control alignment (#170) and Dock icon-size (#169) reports have not been rechecked in this audit |
| Ribbon: tabs, groups, split buttons, galleries, contextual tabs | yes | yes; contextual Table Design, Chart Design and Header & Footer tabs; colour pickers include Recent Colors and More Colors… (#928) | Dedicated contextual tabs for pictures, shapes, sparklines and slicers are missing; no contextual Format or PivotTable tab is currently added by `contextual_tabs` |
| Ribbon collapse, simplified ribbon, customize ribbon/QAT | yes | partial | Home groups can be hidden or pinned; compact Home and context-menu modes have a restore action. Other tabs retain their full controls in compact mode; full QAT customization is missing |
| Key tips (Alt / F10 then letters) | yes, every ribbon control | 367-sequence table across the ribbon tabs, with control badges, prefix waiting and dropdown editing; left-Alt-held sequences work | Some bindings are pending their commands; complete QAT/Backstage coverage is missing |
| Formula bar with Name Box, expand, function hints | yes | yes | Dynamic-array spill children show the source formula muted and read-only using current spill ownership; expansion exists, but Ctrl+Shift+U is unbound |
| Sheet tabs: rename, reorder, colour, hide, scroll | yes | yes | Ctrl-drag (Cmd or Option on macOS) copies (#971); active tabs are revealed on navigation/resize; manual wheel/trackpad browsing is preserved; New sheet (+) follows the last tab within the 60% strip cap. Navigation skips Hidden/VeryHidden sheets and retains the selection if no visible destination exists. Earlier macOS rename-focus report (#145) remains unverified |
| Sheet groups | yes | yes | Shift and Cmd/Ctrl tab selection, group highlighting, [Group] title, Select All Sheets and Ungroup; supported edits apply atomically to the selected sheets with Undo. Ctrl+Shift+Page Up/Down is not bound to grouping |
| Macros and Record/Stop | yes | Macros and recording controls in View, Automate and status bar; Stop is shown in the title bar while recording | Stopping commits a pending cell edit, appends valid VBA to a safe standard module and opens the VBA editor. Alt+F8 opens Macros; WMR starts/stops recording. Recording/replay and unsupported-command limits are documented in [operational-parity.md](operational-parity.md) and [vba-compatibility.md](vba-compatibility.md) |
| Status bar: statistics, view buttons, zoom, macro controls | yes | yes | Customizable statistics remain partial; Macros and Record/Stop are available at the left |
| Task panes (Format Chart, Comments, Watch, Selection, PivotTable fields) | yes | yes; the PivotTable source-field scrollbar sits at the right edge | Element-level Format pane for shapes and pictures is missing |
| Backstage (File): Info, New, Open recent, Save, Print, Export, Options | yes | yes; includes blank/sample workbooks, file properties/protection, page preview and existing application settings | Account shows Rustani, version, license and About; Share and a template gallery beyond the samples remain missing. Options applies immediately without OK/Cancel and exposes only the app's settings, not the full Excel Options dialog |
| Dark mode, system appearance | yes | yes | Follows the system on first run; Options display customization and View theme control (#194, #239) |
| Page Layout view (paper view with rulers and header editing) | yes | yes; per-sheet view saved in XLSX, print pagination, rulers and in-place header/footer editing | Ruler margins can be dragged (#792); header/footer pictures and Different First Page / Odd & Even remain missing |
| Multiple windows per workbook | yes | partial | `view.newWindow` is a linked document copy, not simultaneous views of one shared workbook; see [hardware-parity.md](hardware-parity.md) |

## Grid interaction

AutoFit column width includes dropdown space only for filter header cells (tables and ordinary
AutoFilters), then takes the maximum against body cells. Windows Excel 16.0 build 20326,
observed using synthetic workbooks via its public automation API on 2026-10-11, reserves
12.5 points at 100-200% zoom and 25 points at 50%, independently of row height. GridCraft
uses that allowance in its grid units; its existing approximate font measurement remains.
Header text aligns and clips before the button. Command and header-double-click paths share
this logic, with one undo step.

| Behaviour | Current status / remaining scope |
|---|---|
| Click/drag/Shift/Ctrl selection, multi-area selection | Pointer regressions cover both drag directions and release endpoints. Cmd-click adds/removes areas on Mac; Ctrl-click does so on Windows/Linux, subtracting selected cells without overlapping rectangles. Mac Ctrl-click opens the context menu. Synthetic coverage does not establish every browser gesture |
| Paste into multiple selected areas | Internal copy/Paste Special and external TSV/HTML target every area in one undoable edit; output areas remain selected. Cut moves reject multiple destinations; overlapping output blocks and invalid footprints are rejected atomically. Bounds and verification: PROJECT_STATUS.md §12 |
| Web clipboard paste | Ribbon/menu paste requests the system clipboard asynchronously from the user gesture, with an internal-copy fallback and a stale-destination guard; PNG paste inserts and selects a floating picture. Clipboard method/property access now handles synchronous JavaScript exceptions as well as rejected Promises. Permissions, browser formats and the reported Safari VBA Paste failure still require live verification |
| Multi-area edits | Fill, supported row/column insert/delete, row/column selection, Replace All, AutoSum and defined names handle all selected areas; supported grouped-sheet edits are atomic. See [operational-parity.md](operational-parity.md) for each tested contract |
| Cursor while dragging cells, headers, freeze handles and objects | Stable for the active drag (#936); Draw tools keep their crosshair; header arrows and a custom cell cursor remain unimplemented |
| Selection outline animation | 0.12 s deceleration for keyboard selection, fill and fill handle; pointer selection draws immediately. View ▸ Animations turns it off. Timing is estimated against Excel; system reduced-motion preference is not followed |
| Fill handle: drag, double-click fill down, right-drag menu | Ctrl+drag swaps copy/series; month patterns retain the workbook's 1900/1904 date system. Right-drag offers Copy Cells, Fill Series, Fill Formatting Only, Fill Without Formatting and Series…; date-specific menu choices, Linear/Growth Trend and Flash Fill are missing |
| Drag-move and drag-copy cells (border drag, Ctrl/Option, Shift to insert) | Implemented (M3) |
| In-cell editing, Enter/Tab movement, Alt/Option+Enter line breaks | Implemented; Mac Control+Enter and Command+Enter fill all selected areas. Backspace starts an empty active-cell edit; Delete clears the full selection. IME boundary keys and protected edit entry have synthetic regressions; real OS IME remains unverified |
| Point mode, reference colouring, F4 | Cell/header/drag/arrow references; Cmd/Ctrl-click adds locale-separated arguments, Shift-click extends from an anchor. Shift-clicking sheet tabs supports 3D references; plain tab selection clears the explicit span |
| Number-format colors (`[Green]`, `[Color1]`–`[Color56]`) | Rendered in grid and PDF from shared RGB mappings (#1452) |
| AutoComplete, function autocomplete and argument tips | Implemented; helpers clip to the worksheet and hide when the editor scrolls out (#200) |
| Header resize and double-click AutoFit | Implemented (#42, #88) |
| Data › Advanced Filter | In-place/copy-to, list/optional criteria and unique records; Alt+A,Q opens the dialog (#1622). Copy-to clears only the result footprint and rejects overflow (#1031) |
| Outline symbols for grouped rows / columns | Brackets, − / + and level buttons run hide/show/levels commands, including frozen panes and RTL (#373). Summary position is fixed below/right; no Show outline symbols option or Rows / Columns choice dialog for cell-block grouping. Ctrl+8 is unbound |
| Reorder selected whole columns by dragging headers | Implemented |
| Freeze panes, split panes | Freeze implemented; full split-pane interaction remains to verify |
| Scrolled Table headers in column headings | With the active cell in a Table, names and filter buttons replace letters when its header scrolls away (#1423); the rest of the heading selects the column. Plain AutoFilter ranges retain letters |
| Copy as HTML table | Implemented (#36) |
| Cell context menu | Includes clipboard, insert/delete, clear, filter/sort, comments/notes, formatting, drop-down list, names and links; Insert Copied Cells and whole-row/column Insert Cut Cells are available. Cell-block Insert Cut Cells, paste-option icons, Quick Analysis, Get Data from Table, Show Changes and Smart Lookup remain missing |
| Move and resize charts, pictures and shapes | All eight resize handles, with opposite-edge anchoring and one Undo; LTR/RTL regressions. Floating/in-cell pictures can be converted through context menus; picture insertion offers over-cells before in-cell placement |
| Row/column, tab and object context menus | Headers/tabs and picture placement controls exist; full Excel object menu coverage remains partial |
| Editor text context menus | Formula/cell/VBA editors offer Cut, Copy, Paste and Select All; reopening preserves selected Unicode text and TextEdit Undo. Async Paste is delivered only before the destination TextEdit is shown, with focus and lifetime guards; replacing a VBA draft resets its Undo history. Item 17 has eight VBA regressions and the 2,328-test CI result recorded in operational-parity.md. Safari panic causation and live revalidation remain unconfirmed |
| Format Painter | Single application and double-click sticky behavior are covered by real double-click event regressions; sticky mode survives repeated applications |
| Quick Analysis (Ctrl+Q) | Missing |
| Smooth scrolling, scrollbars | Vertical and horizontal scrollbars implemented; horizontal bar supports dragging and page clicks. The tab/scrollbar split cannot be dragged |
| Text overflow, wrap, rotation, shrink to fit | Implemented |

## Keyboard

| Area | Excel | GridCraft |
|---|---|---|
| Shortcuts bound to commands | 200+ documented shortcuts per platform | Command, grid and editor bindings are audited below; implemented features and unbound shortcuts are listed separately |
| Ctrl+arrow, Ctrl+Shift+arrow, Ctrl+Home/End, Ctrl+Shift+Home/End, Page Up/Down, Alt+Page | yes | implemented |
| Ctrl+1, Ctrl+; / Ctrl+Shift+;, Ctrl+D/R/E, Ctrl+T/L/K, Ctrl+Shift+L and format keys | yes | implemented, with platform-specific mappings below |
| Function keys | yes | Editing F4, F2/F5/F7/F9, Shift+F2/F3/F10/F11 and F12 exist; F4 Repeat, F8 extend mode, F3 Paste Name and F11 chart sheet remain missing |
| Mac variants | yes | Cmd/Control/Option mappings in `keymap.rs`; remaining exceptions below |
| Sequential Alt access keys | every ribbon tab | 367-sequence table across tabs; pending entries are not implemented commands, and complete QAT/Backstage coverage remains missing |
| Requests | — | #110 (shortcuts), #33 (sequential Alt); initial support #43 is historical, not the present coverage limit |

### Shortcut audit (#110)

The original comparison used Microsoft's published Windows and Mac shortcut tables on
2026-10-11; current GridCraft statuses were checked against local source on 2026-10-24.
Alt key-tip sequences are tracked separately (#33). Bindings live in `grid.rs` (navigation and
editing), `ribbon::shortcut` (Ctrl/Cmd letters), `keymap.rs` and `formula_bar.rs` (editing).
“Ctrl” means Cmd on Mac unless the Mac column says otherwise. Format/border combinations
match physical keys (e.g. Ctrl+Shift+4 regardless of the character printed on that layout).

| Shortcut (Windows) | Mac | Action | Current status |
|---|---|---|---|
| Ctrl+C / X / V, Ctrl+Z / Y, Ctrl+A, Ctrl+B / I / U | same with Cmd | clipboard, undo/redo, select all, bold/italic/underline | present |
| Ctrl+S, Ctrl+Shift+S, Ctrl+O, Ctrl+N, Ctrl+W | Cmd | save, save as, open, new, close | present |
| Ctrl+F, Ctrl+H, Ctrl+G, Ctrl+K, Ctrl+T | Cmd (Ctrl+H) | find, replace, go to, hyperlink, table | present |
| Ctrl+D / R / E, Ctrl+1, Ctrl+9 / 0, Ctrl+Shift+9 / 0 | Cmd (Ctrl+E exception below) | fill down/right, Flash Fill, Format Cells, hide/unhide | present |
| Ctrl+- / Ctrl+Shift+= | Cmd | Delete / Insert cells dialog | present |
| Ctrl+; / Ctrl+` | Control+; for date | current date, show formulas | present |
| Arrows, Shift / Ctrl / Ctrl+Shift+arrows, Home, Ctrl+Home / End, Page Up / Down, Ctrl+Page Up / Down | Cmd+arrows; Option+Left / Right for sheet navigation | navigation and selection extension | present; Ctrl+Shift+Page Up/Down grouping is a separate missing binding |
| Tab, Shift+Tab, Enter, Shift+Enter, Ctrl+Space, Shift+Space, Delete, Esc | Control+Space for columns; otherwise same | entry and selection | present; Delete clears selection, Backspace edits active cell |
| F2, F9, F10 (key tips), F4 / Alt+Enter / Ctrl+Enter / Ctrl+Shift+Enter while editing | F4, Option+Enter; both Control+Enter and Command+Enter fill | edit, calculate, references, new line, fill, array | present |
| Ctrl+Shift+~ ! @ # $ % ^ | Control+Shift | General, Number, Time, Date, Currency, Percent, Scientific | present |
| Ctrl+Shift+& / Ctrl+Shift+_ | Cmd+Option+0 / Cmd+Option+-; Cmd+Option+arrows for one side | outline border / remove borders | present |
| Ctrl+2 / 3 / 4 / 5 | — | bold / italic / underline / strikethrough | present |
| Ctrl+6 | Control+6 | hide/show charts, pictures, shapes and slicers; saved as `workbookPr showObjects` | present |
| Ctrl+Shift+: | Cmd+; | current time | present |
| Ctrl+' / Ctrl+Shift+" | Control | formula / value from cell above | present |
| Ctrl+L | Cmd+L Align Left, Cmd+E Center, Control+E Flash Fill | create table | present |
| Ctrl+Shift+L | Cmd+Shift+F / Control+Shift+L | filter | present |
| Ctrl+Shift+F / Ctrl+Shift+G | — | Format Cells Font tab / Workbook Statistics | present |
| — | Cmd+Shift+H / R / K / J, Control+L / U | replace, sort, row/column outline group/ungroup, Define Name, edit cell | present |
| Alt+Shift+Right / Left | Option+Right / Left navigates sheets | row/column outline group / ungroup | present; distinct from worksheet tab groups |
| Ctrl+Shift+* (Ctrl+Shift+8), Ctrl+Shift+Space | Cmd+Shift+Space | current region, select all | present |
| Ctrl+Shift+Home / End, Shift+Home | same | extend to A1 / last used cell / row start | present |
| Ctrl+P | Cmd+P | print | present |
| Ctrl+Shift+> / < | Cmd+Shift+> / < | font size | present |
| Ctrl+Alt+V | Cmd+Control+V / Cmd+Option+V | Paste Special | present |
| Ctrl+Alt+= / Ctrl+Alt+- | — | zoom in / out (10% steps) | present |
| Alt+=, Alt+Down, Alt+Page Down / Up | Option+Down, Option+Page Down / Up | AutoSum, dropdown, one screen right / left | present |
| Alt+Down on filter header | Option+Down | existing AutoFilter popup; validation has priority | present; broader ribbon dropdown/column value-picker work is separate (#441) |
| Alt+F1, Alt+Shift+F1, Shift+F11 | Shift+F11 | embedded chart, new sheet | present |
| Shift+F2, Ctrl+Shift+F2 | Shift+F2, Cmd+Shift+F2 | note, threaded comment | present |
| Shift+F3, Ctrl+F3, Ctrl+Shift+F3 | Shift+F3, Cmd+F3 (Define Name), Cmd+Shift+F3 | Insert Function, Name Manager, Create from Selection | present |
| F5, Shift+F5, F7, Shift+F9, Ctrl+Alt+F9, F12, Ctrl+F4 | corresponding Mac mappings | Go To, Find, Spelling, calculate sheet/all, Save As, close | present |
| Shift+F10, Alt+Shift+F10, Ctrl+F1 | Shift+F10, Cmd+Option+R for ribbon | context menu, Error Checking, ribbon collapse | present |
| Alt+F8 | same key event | Macros dialog | present; View/Automate/status controls also exist; the title bar shows Stop while recording |
| Ctrl+Shift+A after function name | Control+Shift+A | insert argument names and closing parenthesis; add `(` if absent | present, one editor Undo |
| Ctrl+8 | same | toggle outline symbols | shortcut missing; row/column outline UI exists |
| Ctrl+Shift+U | same | expand/collapse formula bar | shortcut missing; formula-bar expansion control exists |
| Ctrl+A after function name | same | Function Arguments | shortcut missing; Insert Function dialog exists |
| Ctrl+Shift+Page Up / Down | platform-specific worksheet-selection keys not bound | group sheets | shortcut missing; modifier-click tab grouping and grouped edits exist |
| Alt+F11 | — | VBA editor | shortcut missing; editor, macro dialog and recorded-script deletion exist |
| F1, F3, F4 Repeat, F6 / Shift+F6, F8 / Shift+F8, F11, End mode | corresponding Mac keys | Help, Paste Name, Repeat, pane cycling, extend/add mode, chart sheet | missing features/bindings; editing F4 is implemented |
| Ctrl+Q, Ctrl+Alt+P, Ctrl+Alt+5, Ctrl+Shift+F1, Ctrl+F5–F10, Alt+F12 | platform equivalents not bound | Quick Analysis, tooltip toggle, object cycling, full screen, window keys, Power Query | missing; no Quick Analysis or Power Query editor |
| — | Cmd+=, Cmd+T while editing, Shift+F7, Cmd+Shift+L | calculate, reference anchoring, thesaurus, Modify Cell Style | missing Mac extras; Cmd+= is egui zoom, F4 anchors references |

On Windows, AltGr arrives as Ctrl+Alt: a Ctrl+Alt key that also types a character is treated as
text, so AltGr symbols (e.g. `~` on a German layout) and user-defined Ctrl+Alt+letter hotkeys that type
a character (e.g. Ctrl+Alt+A for `ä`) never trigger a shortcut, Ctrl+letter ones like Select All included.

## Dialogs

Format Cells, Insert Function/Formula Builder, Find and Replace, Go To and Go To Special, Sort,
Name Manager, Data Validation, Paste Special, Conditional Formatting manager, Page Setup, Goal
Seek, Scenario Manager, Data Analysis, Forecast Sheet, Solver, Protect Sheet and Allow Edit
Ranges exist. Macros and the VBA editor are available; recorded JSON scripts can be deleted.
Options is a Backstage page with immediate application of existing settings, rather than the
full Excel Options dialog. Power Query, Insert Object, Equation editor, full Workbook Links
management and Accessibility checker remain missing. Dialogs open centred and move by their
title bar, retaining their position while open (#1250).

### Web Save As follow-up (item 21)

Verified by UI/engine regressions and the final CI: when no native `pick_save` service exists and a download
service is available, Save As opens a filename dialog instead of immediately downloading under
the current name. It suggests the workbook name, initially focuses/selects the filename, fixes
the format to `.xlsx`, and supports Save/Enter and Cancel/Escape with an IME boundary guard.
Empty/invalid names retain the dialog; valid names gain `.xlsx` if absent. The chosen name is
passed to both workbook encoding and download dispatch, then becomes the document title.
The existing native save picker is preserved.

`ui.saveAsDownload({name, document?})` exposes the UI operation; a captured document UID guards
against saving a different workbook. Dispatch does not acknowledge a completed browser save:
standalone dirty/saved state and local path remain unchanged, and existing host synchronization
is retained. The targeted run passed 35 tests; final CI passed all eight gates with 2,363
tests, zero failures and one existing ignored test (161 suites). `dist/web-save-as-20261025`
serves port 8784. Chrome checked Japanese name input, empty-name errors and Esc cancellation
with zero error logs; download payloads were checked with mocks. Browser OS-write completion,
Safari and real OS IME remain unverified. Log: `../outputs/GridCraft-Item21-CI-20261025.log`.

### Function-library follow-up (items 18–20)

The appended Gemini report findings below are implemented. The targeted integration run
passed 438 tests, including the existing item 17 regressions. Final `cargo xtask ci` passed
all eight steps: **2,349 tests passed, zero failed, one existing ignored test** (160 suites).
Log: `../outputs/GridCraft-Items17-20-CI-Complete-20261025.log`.

| Report item | Required UI behavior | Current stage |
|---|---|---|
| 18: More Functions categories | Submenus for Statistical, Engineering, Information, Database, Compatibility, Web and Add-in/Automation; each category's More Functions… opens Insert Function with that category preselected | Seven `function_library` tests: seven submenus, 13 category footers, category/search filtering and switching, Japanese labels |
| 19: Formulas AutoSum | Home and Formulas use one shared menu: Sum, Average, Count Numbers, Max, Min and More Functions… | Both-tab menu clicks, calculated results for five functions, More Functions routes and all HU/MU variants pass |
| 20: Function insertion during editing | Ribbon category and Insert Function selections insert `NAME(` at the caret or replace the selected text, retaining the surrounding formula; one TextEdit Undo restores the previous expression | Nine `function_insertion` tests pass: both editors, Unicode selection, Undo/Redo and selection restoration, dialog, locale and cross-sheet origin |

Menu hit testing also respects popup layers, so a category click cannot start editing the
formula bar behind it. The refreshed Trunk build is served on 8784. Chrome verified the seven
categories, a Web-filtered dialog, six AutoSum choices and insertion/Undo/Redo in a live formula.
Safari live revalidation remains incomplete.
Item 17's stable VBA ID, scoped Paste, exception handling and draft Undo isolation remain in
place. See [operational-parity.md](operational-parity.md) for dated verification evidence.

## Accessibility

AccessKit is enabled, and numerous controls, tabs, inputs and dialogs expose labels/states;
macro-control placement and text-menu actions have UI regressions. A complete accessible
worksheet tree with cell addresses/values and screen-reader sheet navigation is still missing,
as are a high-contrast theme and Accessibility checker. No complete accessibility conformance
is claimed; the older 12–18 h estimate has not been reassessed.

## Feel

Pixel comparison against Excel screenshots hasn't been run since M1; fonts depend on the system
(no Calibri; Carlito would match metrics, #29, #4, #168 for weights); icons are GridCraft's own
by rule and will never match Office's, by design.

## Remaining work and historical estimate

The earlier 50–80 h estimate included work now implemented, including key tips, Page Layout,
Options and clipboard controls; it is retained as historical context, not a current work budget.
Remaining areas include the missing shortcuts/features above, complete contextual panes and
menus, ribbon/QAT customization, accessibility, platform gesture/IME checks and a fresh visual
comparison. A new effort estimate requires reviewing those remaining items individually.

## Revision history

These entries describe changes on their stated dates. Older limitations may be superseded by
the current inventory above.

| Date | Change | Summary |
|---|---|---|
| 2026-10-25 | minor | Implement item 21 Web Save As filename dialog, fixed XLSX format, focus/selection, keyboard/IME guards and custom-name encoding/download through ui.saveAsDownload. Preserve native picker and standalone dirty/saved/path semantics without claiming browser save completion. 35 targeted passes and final eight-step CI passed (2,363 tests); Web refreshed and Chrome dialog checked. Browser OS-write completion and Safari remain unverified; prior item 17–20 results remain dated evidence |
| 2026-10-25 | minor | Complete items 18–20: categorized More Functions menus/category-prefiltered dialogs, a shared Home/Formulas AutoSum menu with distinct keytips and caret/selection-aware function insertion with TextEdit Undo. Final eight-step CI passed 2,349 tests across 160 suites; refreshed Web deployed on 8784 and Chrome categories/insertion/Undo/Redo verified. Preserve item 17's eight VBA regressions, historical 2,328-test CI evidence and unresolved Safari live verification |
| 2026-10-25 | minor | Harden editor context-menu Paste: restore focus outside popup layout, route async replies only to their destination TextEdit, use a stable VBA widget ID with a separate lifetime scope, and discard stale or redirected replies. Catch synchronous browser clipboard property/method exceptions through Result-returning calls. Initial targeted VBA/formula/async regressions pass; no actual Safari panic stack or successful Safari revalidation is claimed. Full post-change CI evidence is tracked separately in operational-parity.md |
| 2026-10-24 | minor | Consolidate duplicate current-state rows; distinguish implemented macros, VBA editor, sheet groups, outline controls and formula-bar expansion from unbound shortcuts; document all eight object resize handles and asynchronous Web paste. Preserve historical estimates and dated change records |
| 2026-10-24 | major | Operational audit: macro entry points/recorded VBA, multi-area fill/structural edits/selection/replace/AutoSum/names, Japanese formula diagnostics, text context menus, async clipboard, 8-handle pictures and placement conversions, grouped sheets and 3D formula pointing. Behavior evidence: operational-parity.md and PROJECT_STATUS.md §13; no new compatibility percentage inferred |
| 2026-10-24 | minor | Multi-area paste across internal copy/Paste Special and external TSV/HTML; retain all destination areas and one Undo; preflight cut/overlap/size limits. Mac Ctrl-primary click routes to context menus while Cmd additive selection stays. Regression evidence: PROJECT_STATUS.md §12; scores unchanged |
| 2026-10-24 | minor | IME boundary keys handled before cell actions and TextEdit; direct input preserves focus filtering; protected-cell edit startup uses engine rules including Allow Edit Ranges. Synthetic regression evidence in PROJECT_STATUS.md §11; OS IME and Safari/Chrome comparison unverified; scores unchanged |
| 2026-10-21 | minor | Background error checking, first part (#1994): a constant whose text would be a number if typed (signs, thousands, currency, percent, exponent, parenthesised negative) gets Excel's green corner triangle; the active cell then shows a warning button whose menu names the problem and offers Convert to Number for the selection (`formulas.convertToNumber`). Dates, times and fractions kept as text and formula results are not flagged, as in Excel |
| 2026-10-21 | minor | Paste Special laid out as Excel's: Paste and Operation options each in two columns inside a frame, Skip blanks and Transpose side by side, Paste Link as a button that pastes references at once; labels translated (the generic form didn't translate them) and the window no longer runs past the screen |
| 2026-10-21 | minor | Form dialogs (conditional-format quick rules, New Name, Remove Duplicates, Text to Columns and every other `Dialog::form`) fit their content: OK and Cancel sit in a row under the last field instead of a right-to-left layout that took the screen's whole height and left the buttons floating halfway down |
| 2026-10-20 | minor | Esc and Enter close a message over the cell editor, such as a data validation alert, and the entry stays open to fix, with the keyboard back in it. Before, the editor took them (the first Esc cancelled the edit behind the alert and only the second closed it), and after the message typing went nowhere until the cell was clicked |
| 2026-10-19 | minor | Ctrl+Enter (`range.fill`) gives each cell what typing the input there gives: a General cell gets the automatic number format (`5%`, `$5`, `3/4/2026`, `=TODAY()`) even when it already held a value, and a cell with its own format, a formatted row or column, or Text keeps it. Cells that held a value used to keep General (`5%` showed 0.05), and empty cells took the active cell's format |
| 2026-10-19 | trivial | While a mouse button is held (a click or a drag selecting cells) the selection outline is drawn on its cells at once instead of gliding after the pointer, which read as lag; keyboard moves still glide |
| 2026-10-18 | minor | Keytip letters typed with left Alt held continue the sequence (Alt+H, O, A / Alt+H, O, I ran nothing before: Alt+letter cancelled keytips); other Alt combinations, AltGr and macOS Option are unchanged (#1846) |
| 2026-10-17 | minor | A Ctrl+Alt+letter key that types a character (a user-defined Windows hotkey such as Ctrl+Alt+A for `ä`) types it instead of running the Ctrl+letter shortcut, e.g. Select All (#1530) |
| 2026-10-16 | minor | Right-drag on the fill handle opens the fill menu on release: Copy Cells, Fill Series, Fill Formatting Only, Fill Without Formatting, Series…; Esc cancels the drag or closes the menu without filling |
| 2026-10-16 | minor | With the active cell in a Table whose header row is scrolled out of view, the column headings show the header names and their filter buttons in place of the letters (#1423) |
| 2026-10-16 | minor | Ctrl+6 hides every object (charts, pictures, shapes, slicers) and shows them again; hidden objects are not drawn, take no clicks and drop their selection, and the setting is saved with the workbook (`workbookPr showObjects`) |
| 2026-10-16 | minor | Ctrl+Shift+A (Control+Shift+A on a Mac) after a function name, or after its `(`, inserts the argument names and the closing parenthesis, as in Excel; one Undo in the editor takes them back |
| 2026-10-16 | minor | Advanced Filter ribbon dialog, optional criteria for unique-only extraction (#1622); safe copy-to output (#1031) |
| 2026-10-16 | minor | Remove Duplicates shows an editable Range field initialized from the selection and passes it to the engine on OK (#1388). Live range-picking outside the dialog remains unsupported |
| 2026-10-16 | minor | Web printing registers one-shot `afterprint` cleanup before opening the dialog; missing content windows and setup, listener or print failures also release the hidden iframe and PDF blob URL (#1054) |
| 2026-10-16 | minor | The shared UI clock delegates to the core Unix-millisecond clock, which reads Date.now() on wasm, instead of returning zero there (#1026). Native clock contract tested; browser timing remains unverified |
| 2026-10-16 | trivial | Web: preferences (theme, interface size, language, editing options…) are kept in the browser's `localStorage` between visits, as the desktop app keeps `ui.json` and `prefs.json`; before, a reload brought back light mode and the defaults (#1562) |
| 2026-10-15 | minor | Number-format named colors use their RGB values; indexed `[Color1]`–`[Color56]` resolve through the shared legacy palette in grid and PDF (#1452) |
| 2026-10-14 | minor | Dialogs (Find and Replace, Go To, Format Cells, Sort and the rest) move by their title bar instead of staying pinned to the centre (#1250) |
| 2026-10-14 | minor | Spill-child formula bar uses the anchor formula with disabled theme text and blocks formula-bar edit entry; selection and recalculation restore ordinary behavior |
| 2026-10-14 | minor | PivotTable source-field list fills the pane width so its scrollbar stays at the list's right edge |
| 2026-10-13 | minor | Dragging a sheet tab with Ctrl held (Cmd or Option on macOS) drops a copy of the sheet where the tab is released, with the copy cursor (#971) |
| 2026-10-13 | trivial | More Colors… and Recent Colors (#928) |
| 2026-10-13 | minor | Grid drags keep their own cursor over selection borders, fill handles and outside the grid; hover resumes on release (#936) |
| 2026-10-13 | minor | AutoFill month patterns retain the workbook's date system, preventing shifted dates in 1904 workbooks when filling forward or backward |
| 2026-10-13 | minor | Sheet tab bar: the tab strip shrinks to its tabs, up to 60% of the bar, so New sheet (+) sits right after the last tab and the horizontal scrollbar takes the rest (part of #943) |
| 2026-10-13 | minor | Horizontal scrollbar in the sheet tab bar: drag, track click pages, frozen columns excluded from the page (#840) |
| 2026-10-13 | minor | Formula-entry Command/Ctrl-click adds arguments and Shift-click extends references; ordinary replacement and dragging retain their behavior. Readiness estimates unchanged. |
| 2026-10-12 | minor | Next/previous sheet navigation no longer activates hidden worksheets at either workbook boundary; command tests cover Hidden, VeryHidden and selection retention |
| 2026-10-12 | minor | The File view's recent list tells when each workbook was last changed, read off the UI thread |
| 2026-10-12 | minor | Ctrl+wheel zoom steps along a ladder of percentages about 20% apart that never skips 100%, gliding to each step with animations on, so a notch out undoes a notch in (it drifted: 100 → 122 → 101); Excel steps 15 points, see decisions.md |
| 2026-10-12 | minor | Page Layout view: left/right/top/bottom margins are dragged on the rulers (#792) |
| 2026-10-12 | minor | Alt+Down opens the active filter header's existing popup, with validation priority and mirrored anchoring on right-to-left sheets (part of #441) |
| 2026-10-12 | minor | Paste from the ribbon (button and options menu), the cell and header context menus and the macOS menu bar's Edit menu pastes what is on the system clipboard, as Cmd/Ctrl+V does, instead of only GridCraft's own copy (the web build, which can't read the clipboard on request, still pastes GridCraft's copy). Copy and Cut by keytip (H,C / H,X) publish to the system clipboard like the ribbon buttons. After a control-channel or MCP copy, which leaves the system clipboard alone, Paste pastes that copy until other text is copied. Once a cut has been pasted, Paste (ribbon, menus or Cmd/Ctrl+V) doesn't paste the cut's clipboard text again over the moved cells (Excel pastes a cut once), and the Paste keytip (H,V,P) pastes the system clipboard like the button. Regression test for the Paste button click freeze in 0.5.0 (#273, fixed by #272); the Paste button gets an accessible "Paste" name for screen readers |
| 2026-10-11 | minor | AutoFit accounts for table and ordinary filter dropdowns, including low zoom; header text excludes the button area |
| 2026-10-11 | minor | Outline band for grouped rows and columns, Show / Hide Detail on the Data tab, numpad navigation keys in shortcuts (#373) |
| 2026-10-11 | minor | Shift+Space / Ctrl+Space no longer start an edit; Control+Space selects the column on Mac (#338) |
| 2026-10-11 | minor | Selected whole columns can be reordered by dragging their headers to an insertion point |
| 2026-10-11 | minor | Page Layout view with rulers and in-place headers and footers (#329) |
| 2026-10-11 | minor | Forecast Sheet dialog (#322) |
| 2026-10-11 | minor | Delete and Backspace now remove the selected chart, picture or shape (#213) |
| 2026-10-10 | minor | Keep formula helpers inside the worksheet when scrolling (#200) |
| 2026-10-11 | trivial | Theme: title-bar light/dark switch, File › Display theme, system appearance on first run (#194, #239) |
| 2026-10-11 | minor | Keyboard shortcut audit against Microsoft's Windows and Mac lists; missing shortcuts for existing features added (#110) |
| 2026-10-11 | minor | File tab Backstage view and File › Options (#174) |
| 2026-10-11 | minor | Selection outline glide and the Animations setting |
| 2026-10-11 | major | Sequential Alt keytips: full-tab table (364 sequences), badges on every control, dropdowns open for typing/arrow editing, colour-palette arrows, real-Excel corrections (issues #33, #49) |
| 2026-10-10 | minor | Chart resize bug (#190) |
| 2026-10-10 | major | Created: chrome, grid, keyboard, dialogs, accessibility; key tips, Page Layout view and accessibility tree flagged |
