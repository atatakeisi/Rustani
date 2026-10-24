# Gaps

> **Last reviewed:** 2026-10-24 (operational-parity and remaining-work reconciliation) · **Last updated:** 2026-10-24 · **Change:** minor (remove obsolete implementation gaps; retain browser and VBA verification requirements) · **Target:** Microsoft Excel (Microsoft 365)

Ranked shortfalls against Excel. Check the current source and PR status before starting work;
the inherited issue references, numerical estimates and readiness assessments below have not
been comprehensively remeasured. Agent-hour estimates are historical Opus 5.5 estimates, not a
current delivery forecast. Area detail lives in the linked parity docs; the summary is in
[target-app-parity.md](target-app-parity.md).

Kinds: **F** feature · **U** UI/UX · **FF** file format · **H** hardware · **L** localization ·
**P** performance · **S** stability.

## Current validation remainder (2026-10-24)

The multi-area editing, grouped-sheet, clipboard, macro-control and picture workflows in
[operational-parity.md](operational-parity.md) are implemented and covered by CI plus selected
Chrome live checks ([PROJECT_STATUS.md §13](../PROJECT_STATUS.md)). They are no longer absent
features. Remaining validation includes **Safari on a real browser**, **real OS Japanese IME
candidate conversion**, browser permission/file-save paths, WebGPU/WebGL comparisons and
Windows/Linux hardware checks. Synthetic input and WASM compilation do not establish those
results. **Full Excel VBA compatibility remains unmet**; see §21 for language and project-writing
work rather than interpreting the operational fixes as completion of that requirement.

## Beta blockers

The inherited [alpha gate](roadmap.md#alpha-gate) records earlier core-workflow results; it is
not a new whole-product readiness assessment.


### 1. XLSX output not proven in Excel or on real files (FF)
- **Missing:** a run over a real-world XLSX corpus, and confirmation that Excel opens what we save
  with no repair prompt (PivotTables, charts, CF, validation, dynamic arrays, tables).
- **Evidence:** round-trip tests cover our own files only (`crates/xlsx/src/tests/`); PivotTable
  round-trip noted as "unverified in real Excel" since 10-07; #64 (dynamic-array flag) shows the
  class of bug.
- **Impact:** the main format; a repair prompt or silent loss destroys trust.
- **Estimate:** 15–25 h, plus owner time in Excel. [file-format-parity.md](file-format-parity.md)

### 2. Large workbooks don't open; recalc is single-threaded (P, H)
- **Missing:** streaming read for ~100 MB XLSX; multi-threaded recalc; 100k-dependent edit under
  100 ms (historically ~0.4 s), with fresh scale benchmarks still needed. Current paste/fill/group
  commands have explicit bounds; this is not an assertion that those guards remain unimplemented.
- **Historical evidence to recheck:** #175; PR #152 (whole-sheet Fill/Remove Duplicates
  out of memory), PR #151 (current region past 10,000 rows).
- **Impact:** finance and data users routinely have big workbooks.
- **Estimate:** 25–40 h. [target-app-parity.md](target-app-parity.md#performance), [hardware-parity.md](hardware-parity.md)

### 3. Some unmodelled XLSX content is dropped on save (FF)
- **Missing:** slicers and their caches, timelines, the data model, query tables attached to
  tables, unmodelled `extLst` entries, chart details. Pass-through (#330) keeps custom XML,
  external links, connections, sheet query tables, printer settings, form and ActiveX controls,
  OLE objects and sheet backgrounds; never checked in Excel (repair prompts unknown).
- **Evidence:** `crates/xlsx/src/passthrough.rs` (what is kept, and why the rest isn't).
- **Impact:** open-edit-save still simplifies workbooks with slicers, timelines or Power Pivot.
- **Estimate:** 6–10 h. [file-format-parity.md](file-format-parity.md)

### 4. Charts lose options on round trip; shallow chart model (F, FF)
- **Missing:** axis scaling, label options, error bars, per-point formatting.
- **Present:** chart movement and resizing are implemented (`ui-egui/src/chartview.rs` and
  `grid.rs`); the former “cannot resize” entry is obsolete. Smooth lines, Switch Row/Column,
  trendlines and multi-level category axes have read/render/write paths. The chart parity
  inventory records the previously merged chart-kind work; those PRs are not listed as open here.
- **Evidence for remaining loss:** the partial chart model and XLSX chart writer preserve only
  modelled options. Real Excel round-trip fidelity still needs verification.
- **Impact:** charts are in most business workbooks.
- **Estimate:** 25–35 h for beta (40–60 h full). [chart-parity.md](chart-parity.md)

### 5. Script coverage, shaping and non-Latin export remain incomplete (L)
- **Present:** native system-font fallback and an optional external `craft-fonts` BIZ UDPGothic
  Web fallback. Japanese UI rendering has selected Chrome evidence; font sources are not
  committed to GridCraft. This does not guarantee every Japanese character or full Chinese/Korean coverage.
- **Missing:** complex shaping (Arabic, Devanagari, Thai), bidi, non-Latin PDF, and real-browser/
  real-OS IME confirmation. The PDF writer's standard-font path does not provide CJK export.
- **Impact:** blocks most of the world's users even with an English UI.
- **Estimate:** 12–20 h. [localization-parity.md](localization-parity.md)

### 6. Calculation correctness bugs (F)
- **Status:** the issues previously tracked here have landed: 15-significant-digit
  equality (#57, so `=0.1+0.2=0.3` is TRUE and MATCH and COUNTIF agree), spill updates after
  structural edits and cleared blockers (#133, #139; PRs #148, #157), 3-D span deletion (#140),
  and qualified names after a sheet rename (#134). Iterative calculation for circular references
  landed too. This dated fix list does not replace a complete calculation oracle suite.
- **Impact:** wrong numbers are worse than missing features, so this entry is kept until the
  oracle suite of §15 can say the category is clear rather than only these five reports being so.
- **Estimate:** — [function-parity.md](function-parity.md#formula-language)

### 7. Legacy `.xls` only partly read; encrypted workbooks can't be saved with a password (FF)
- **Evidence:** #121: Excel 97-2003 (BIFF8) `.xls` files now open (values, formulas, number
  formats, merges, sizes, names); fonts, fills, borders, charts, pictures, comments, conditional
  formats, validation and BIFF5 files are not read, and `.xls` is never written. Password-encrypted
  `.xlsx` workbooks (Agile and Standard encryption) open with their password since #11, but are
  saved without it: GridCraft can't write an encrypted file yet. Encrypted `.xls` (FILEPASS) and
  RC4-encrypted files are still refused with a message.
- **Impact:** common in finance, government and anything older than 2007; encrypted files are
  routine in business.
- **Estimate:** 14–21 h (`.xls` styles and objects 10–15, encryption 4–6). [file-format-parity.md](file-format-parity.md)

### 8. PivotTables: slicers, calculated items and Excel verification remain (F)
- **Remaining:** slicers/timelines, calculated items and real Excel validation. PivotCharts,
  calculated fields and values on rows are implemented; PivotChart field buttons remain absent
  ([chart-parity.md](chart-parity.md)). Command registration alone does not prove runtime fidelity.
- **Impact:** pivots are the analyst's main tool.
- **Estimate:** 20–30 h for beta (25–40 h full).

### 9. Keyboard-first users: key tips, shortcuts, context menus (U)
- **Present:** the generated KeyTips table spans File/Backstage and multiple ribbon tabs, not
  just Home. Multi-area selections, sheet modifier selection, text context menus, Mac Control+
  Enter and macro KeyTips have operational regression tests. Sheet navigation skips hidden tabs.
- **Remaining:** broader shortcut/KeyTip fidelity and real-platform keyboard/IME checks. The old
  shortcut count and Home-only claim are not a current coverage measurement. See
  [operational-parity.md](operational-parity.md) for the verified scope.
- **Estimate:** 12–20 h. [ui-parity.md](ui-parity.md)

### 10. Crashes and freezes reported from the field (S)
- **Evidence:** #120 (Intel UHD crash on Windows), #117 (error on close, macOS), #166 (freeze opening
  from Finder; PRs #164, #167), #76 (flashing on Windows 11), #130 (caught panic in table styles).
- **Estimate:** 8–12 h plus an autosave/recovery soak.

## After beta

### 11. Localization and locale behavior remain partial (L)
- Language catalogs, language switching and Japanese UI defaults are implemented. Japanese
  menus, dialogs, VBA editing and diagnostic headings are translated; the latest operational
  changes also translate static and parameterized formula errors. English remains in parts of
  the UI/diagnostics, and full locale-aware function names, separators and number/date behavior
  remain separate work. Whole-UI and per-language coverage have not been remeasured here.
- **Historical estimate:** 100–165 h including #5, not refreshed after these changes.
  [localization-parity.md](localization-parity.md)

### 12. Get & Transform, external data, data model (F)
- No Power Query editor, connections, refresh, From Web/database/folder sources, relationships or
  data model (`data.fromWeb`, `data.relationships` missing). **Estimate:** 60–100 h.

### 13. Analysis tools: remaining Solver/ToolPak/Forecast parity (F)
- Solver (`data.solver`, #327) has Simplex LP with integer/binary branch and bound, a local
  nonlinear method (augmented Lagrangian, not GRG), the Evolutionary method, and Answer and
  Limits reports. Report refusal leaves the solution, saved model and history unchanged; a
  successful solve and its reports share one Undo step (#1050). AllDifferent constraints,
  the Solver Options dialog, the Sensitivity and Population reports and Load/Save Model remain.
  The Analysis ToolPak
  (`data.analysisToolPak`) has a partial tool set. Forecast Sheet (`data.forecastSheet`), Forecast
  Start (hindcast) and the FORECAST.ETS family are implemented. Historical tool counts and
  **12–20 h** estimate have not been remeasured here.

### 14. Page Layout view and print polish (U, F)
- Page Layout view landed (#329): paper view with rulers and in-place header/footer editing.
  Margins are dragged on the rulers (#792). Remaining: pictures in headers/footers, Different First Page / Odd & Even pages. Desktop now submits directly through craft-print with printer choice, copies, duplex and grayscale; browser printing stays available. Print ranges and driver tray/options UI remain. **Estimate:** 5–10 h.

### 15. Remaining functions and an oracle suite (F)
- Remaining/stubbed function coverage needs a fresh inventory; the contradictory inherited
  counts are not a current measurement. An Excel-computed oracle corpus is still needed.
  **Historical estimate:** 18–30 h, not refreshed. [function-parity.md](function-parity.md)

### 16. Collaboration and review (F)
- Co-authoring and Show Changes remain absent; cloud storage was requested (#86). Comment
  navigation, Show Comments/Notes and notes-to-comments conversion are implemented.
  **Historical estimate:** 35–55 h (co-authoring scope needs the owner).

### 17. Insert objects (F)
- Ink strokes now survive XLSX save/reopen as editable polylines. Inserted icons still lose their
  identity/artwork on save (#551); native Office InkML import is not implemented.
- SmartArt, WordArt, equations, signature line, objects, 3-D models, screenshot; 8 shape kinds vs
  ~170 were the inherited inventory; current shape counts were not remeasured.
- **Present:** pictures in cells, floating/cell placement conversion, clipboard PNG insertion,
  selection and all eight floating-picture resize handles. CI and selected Chrome evidence are
  recorded in [operational-parity.md](operational-parity.md); this does not prove full Excel
  picture/package fidelity. **Historical estimate:** 25–40 h, not refreshed.

### 18. Accessibility (U)
- AccessKit on, but no accessible grid/ribbon tree, no high contrast, no checker. **Estimate:** 12–18 h.
  [ui-parity.md](ui-parity.md#accessibility)

### 19. XLSB/ODS beyond values; minor formats (FF)
- XLSB and ODS import values only, no write; no `.prn`, `.dif`, `.slk`, XML Spreadsheet 2003; CSV
  writes raw instead of displayed values; templates saved with the workbook content type.
  **Estimate:** 15–23 h.

### 20. Multiple windows; pen pressure (H)
- One window (#162, PR #171); no pressure or tilt in ink. **Estimate:** 10–14 h.
  [hardware-parity.md](hardware-parity.md)

### 21. Automation and add-ins (F)
- **Required:** full compatibility with Excel-authored VBA, including the language, object
  model, events, UserForms and native Windows dependencies. This requirement is not met.
- **Present locally:** original VBA project bytes retained for `.xlsm`/`.xltm`; bounded source
  extraction; partial cooperative VM and Excel object adapter; source editor, run/stop, output
  and line diagnostics. Loaded VBA/script content needs explicit trust; edits are staged and commit
  only after success. Macro controls and recording generate valid supported VBA into a standard
  module, with unsupported operations explicitly reported. Source edits save in GridCraft metadata.
- **Missing:** parameterized `Sub`, `Function`, `ByRef`/`ByVal` and broader language/library/
  object-model coverage; **generating or rewriting an executable Excel `vbaProject.bin`**;
  events/forms, native execution architecture, macro UDFs and add-in lifecycle.
- **Verification remaining:** Excel-authored macro corpus tests and **open/edit/save/reopen in
  real Excel**, including edited executable projects. Retaining original project bytes and
  passing local CI/Chrome editor tests do not establish this round-trip compatibility.
  Office Scripts compatibility and a plugin/custom-function API also remain absent.
- **Estimate:** full VBA compatibility **not yet estimated**. The historical 25–40 h covers
  the plugin/custom-function API only and must not be read as an estimate for complete VBA.
  Existing readiness scores are unchanged. [vba-compatibility.md](vba-compatibility.md)

### 22. AI features (F)
- No in-app assistant, natural-language formulas or Analyze Data; agent control via MCP is
  already strong. **Estimate:** 30–50 h; model choice needs the owner.

### 23. Mobile (platform)
- No iOS, iPadOS or Android layout. **Estimate:** 30–60 h.

### 24. Fonts and look (U)
- Font weights beyond regular/bold (#168), system font list (#4), Calibri metrics (#29), macOS
  chrome (#169, #170), Linux title bar (#54), icon consistency across apps (#173). **Estimate:** 7–12 h.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-24 | minor | Reconcile operational implementation with CI/Chrome evidence; remove obsolete resize/Home-only KeyTips/no-localization/no-cell-picture claims and contradictory inherited entries; keep historical estimates separate from Safari/real-OS IME and full VBA/project-writing verification gaps |
| 2026-10-23 | minor | §21: replace obsolete VBA exclusion/drop claim with the full requirement, local partial implementation and unestimated remaining scope; readiness unchanged |
| 2026-10-20 | minor | §11: the Japanese catalog covers the dialogs, messages and menus — 943 keys (~43%), matching the pt-BR coverage; a test keeps the two catalogs in step |
| 2026-10-17 | minor | §6: the five reports it tracked are closed and their PRs merged (#57, #133, #139, #140, #134; PRs #148, #155, #156, #157); the entry stays open for §15's oracle suite |
| 2026-10-15 | minor | §13: Solver prepares reports before committing the solution, model and report sheets together; report refusal preserves the workbook and Undo/Redo (#1050) |
| 2026-10-13 | trivial | §4: multi-level category axes landed |
| 2026-10-12 | minor | §9: next/previous sheet commands retain the current visible sheet and selection when there is no visible destination |
| 2026-10-12 | minor | rows auto-fit oversized fonts on open and whole-row/column formatting (#833) |
| 2026-10-12 | minor | Page Layout view: dragging margins on the rulers (#792) removed from the §14 remainder |
| 2026-10-12 | minor | §13: Forecast Sheet Forecast Start (hindcast) landed (#785) |
| 2026-10-12 | minor | §17: ink geometry survives XLSX saves; the icon portion of #551 remains |
| 2026-10-12 | minor | §8: values on rows (Σ Values in Rows, `dataOnRows`) laid out, moved and round-tripped |
| 2026-10-12 | minor | Desktop File > Print now discovers printers and submits PDF jobs through pinned craft-print with printer choice, copies, duplex and grayscale; browser printing and PDF export remain available. Remaining print parity includes page ranges and driver-specific tray/options UI; physical printer fidelity needs hardware validation. |
| 2026-10-11 | minor | §6: iterative calculation for circular references landed (Maximum Iterations, Maximum Change) |
| 2026-10-11 | minor | §15: trim-reference operators (`A1:.E10`, `A1.:E10`, `A1.:.E10`) parse and evaluate |
| 2026-10-11 | minor | §4: trendlines (six types, forecast, equation and R²) read, written and drawn |
| 2026-10-11 | minor | §7: password-encrypted workbooks open with their password; saving with one remains (#11) |
| 2026-10-11 | minor | §16: comment navigation, Show Comments, Show All Notes and Convert to Comments landed (#281) |
| 2026-10-11 | minor | §7: `.xls` (BIFF8) import of values, formulas and number formats (#121) |
| 2026-10-11 | minor | §3: XLSX pass-through keeps custom XML, external links, connections, query tables, printer settings, controls and OLE objects (#330, #324) |
| 2026-10-11 | minor | §13: Forecast Sheet landed (#322) |
| 2026-10-11 | minor | §15: ODDFPRICE, ODDFYIELD, ODDLPRICE, ODDLYIELD added (18 functions missing) |
| 2026-10-11 | minor | §15: PERCENTOF and EUROCONVERT added (20 functions missing) |
| 2026-10-11 | minor | §15: BAHTTEXT added (21 functions missing) |
| 2026-10-11 | minor | §15: TRIMRANGE added (21 functions missing; trim-reference operators still missing) |
| 2026-10-11 | minor | §16: Allow Edit Ranges landed (#307) |
| 2026-10-11 | minor | §8: calculated fields added |
| 2026-10-11 | minor | §8: PivotCharts added |
| 2026-10-11 | minor | §13, §15: FORECAST.ETS family landed (#314) |
| 2026-10-10 | minor | Updated for the 10-10 merges (#17, #31, #38, #62, #82, #104, #116) and the new chart resize bug (#190) |
| 2026-10-10 | minor | Noted that no gap blocks the alpha gate |
| 2026-10-10 | major | Created: 24 ranked gaps with evidence, from the full re-measure and the 60 open issues |
