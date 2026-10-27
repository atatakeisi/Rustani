# Roadmap detail

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** policy (sandboxed Mini-VBA; full Excel VBA is outside scope) · **Target:** Rustani

Forward-looking plan. The one-page summary is [`ROADMAP.md`](../ROADMAP.md); the ranked work
list is [`gaps.md`](gaps.md); numbers and method are in
[`target-app-parity.md`](target-app-parity.md). The agent-hour estimates, readiness percentages
and release/issue references below are inherited assessments, not a fresh project-wide audit or
delivery forecast. Rustani targets a bounded sandboxed Mini-VBA, not full Excel VBA compatibility; historical estimates have not been recalculated.

## Current focus

The operational fixes in [operational-parity.md](operational-parity.md) are integrated:
multi-area and grouped-sheet edits, Web clipboard handling, macros/recording, Japanese formula
diagnostics, picture placement and eight resize handles. Full CI and selected Chrome live
checks are recorded in [PROJECT_STATUS.md §13](../PROJECT_STATUS.md). Chart resizing, KeyTips
across multiple tabs, localization infrastructure and pictures in cells are implemented.
Registration counts and this evidence do not establish complete Excel compatibility.

The next work is:

1. **Browser and input validation:** Safari in a real browser, real OS Japanese IME candidate
   conversion, browser clipboard permissions/file-save flows, WebGPU/WebGL comparison, and
   Windows/Linux platform checks. Selected Chrome checks and synthetic IME tests do not replace them.
2. **Sandboxed Mini-VBA:** maintain the existing limited interpreter, trust checks, bounded execution and source metadata. Verify supported workbook and browser workflows; full Excel VBA, external I/O and executable-project writing are outside the Rustani target. See [vba-compatibility.md](vba-compatibility.md).
3. **XLSX fidelity and performance:** real-file corpus and Excel output verification (gaps #1,
   #3), large workbooks and multi-threaded recalc (gap #2). Recheck the source and current PR
   queue before scheduling inherited issues; the old 2026-10-10 queue is not a current backlog.

## Alpha gate

The inherited table records the earlier macOS desktop core-workflow assessment, including save
and reopen. It has not been rerun as a complete product gate for the 2026-10-24 changes. Current
CI/Chrome scope and explicit remaining validation are linked above.

| Core workflow | Works end to end? | Evidence | Hours to pass |
|---|---|---|---|
| Build a model: enter data and formulas, recalc, save XLSX, reopen | yes | Dynamic arrays, LET/LAMBDA, names, tables and XLSX round-trip tests. Previously listed 15-digit equality and stale-spill reports are recorded as fixed in gaps §6; an Excel oracle corpus remains | 0 (historical gate result) |
| Open a colleague's XLSX, edit, save, hand it back | partial, not blocking | Typical workbooks open and save with cells, styles, CF, validation, tables, names and pivots; unmodelled parts are dropped with a warning (gap #3); Excel-side repair prompts unverified (gap #1); ~100 MB files fail (#175) | 0 for alpha (gaps #1–#3 are beta gates) |
| Format and present: number formats, styles, conditional formatting, print or PDF | yes | Full number-format language, 47 cell styles, all CF kinds, Page Break Preview, PDF export and printing. PDF can't carry non-Latin text (gap #5) | 0 |
| Analyze: sort, filter, tables, PivotTable | yes | Multi-level sort, AutoFilter, tables with totals, PivotTables with field list, layouts, grouping, refresh and XLSX round-trip. Calculated fields done; slicers missing (gap #8) | 0 |
| Chart the data | partial, not blocking | Column, bar, line, pie, area, scatter, combo, histogram, box, waterfall, funnel, treemap, sunburst and stock charts are created, edited and survive save and reopen; trendlines are drawn; no axis options (gap #4) | 0 for alpha |
| Import and export CSV | yes | Encoding and delimiter detection on read, CSV/TSV write; exports raw rather than displayed values (gap #19) | 0 |

The earlier assessment classified these workflows as alpha, with advanced fidelity and scale
work deferred to beta. The old ~50% readiness estimate has not been remeasured; it is not a
score for the current browser or VBA implementation.

## Beta gates

The inherited beta definition was ~75% ready for real work with reliable XLSX interchange.
The table retains the old effort estimates for reference; completed subfeatures have been
removed from the remaining-work labels, so the historical hours must be re-estimated before use.

| Gate | Gap | Historical hours |
|---|---|---|
| XLSX proven on a real-world corpus; Excel opens our output without repair | #1 | 15–25 |
| Unmodelled XLSX parts preserved on save | #3 | 10–15 |
| 100 MB workbooks open; 1M rows usable; multi-threaded recalc; 100k-edit recalc < 100 ms | #2 | 25–40 |
| Charts: advanced options survive round trip; axis options, error bars and per-element formatting | #4 | 25–35 |
| Text in every script renders and exports | #5 | 12–20 |
| Calculation correctness verified against an Excel oracle corpus; known fixes retained | #6 | 10–15 |
| Broader `.xls` fidelity and password-encrypted output | #7 | 28–42 |
| PivotTables: slicers, calculated items, remaining PivotChart UI and real Excel verification | #8 | 20–30 |
| Broader keyboard/KeyTip/shortcut fidelity and real-platform checks | #9 | 12–20 |
| Field crashes and freezes closed; autosave/recovery soak | #10 | 8–12 |
| **Total** | | **~165–255 (≈160–240 after overlap with the PR queue)** |

The former multi-agent wall-clock forecast is not a current estimate. Real Excel and platform/
hardware validation remain necessary; historical totals have not been recalculated for the Mini-VBA policy.

## Milestones

| # | Milestone | Status |
|---|---|---|
| M0 | Foundation: core, number formats, formula language, functions, model, calc, XLSX/CSV, engine | done |
| M1 | Excel look: chrome, ribbon, formula bar, grid, tabs, status bar, control channel | done (polish continues) |
| M2 | Formatting completeness | done |
| M3 | Editing power (AutoComplete, spelling, drag-move/copy) | done |
| M4 | Data (advanced filter, consolidate, what-if) | partial; Solver/ToolPak implementations exist with remaining parity gaps; Power Query remains |
| M5 | Formulas tab (trace arrows, watch window, evaluate, error checking) | done |
| M6 | Conditional formatting manager parity | done |
| M7 | Charts parity | partial; resizing, Format pane and trendlines implemented; axis/error-bar/per-element options and interchange fidelity remain |
| M8 | Insert & Review | partial; ink, icons, comments pane and pictures in cells implemented; remaining objects/fidelity in gaps §17 |
| M9 | Page layout & print | Page Layout view and ruler margin editing implemented; print/PDF polish remains |
| M10 | PivotTables, Solver, Analysis ToolPak | implementations present; slicers, calculated items, remaining tools/reports and Excel verification remain |
| M11 | Automation (recorded scripts and sandboxed Mini-VBA) | bounded interpreter/editor and source import present; preserve trust and safety limits; full Excel VBA and executable-project writing are outside scope |
| M12 | Performance, accessibility, i18n | partial; i18n catalogs and switching implemented, wider language/locale and script support remain |
| M13 | Release & polish | historical releases exist; current release/signing status not audited here |
| M14 | **Beta gates** (table above) | remaining gates require current evidence and re-estimation; no fresh readiness score |
| M15 | Localization infrastructure and the twelve languages | partial; multiple catalogs, Japanese defaults/editor/errors and language switching implemented; full coverage/locale behavior and native review remain |
| M16 | Get & Transform, data model | not started |

## After beta (ranked)

| Work | Gap | Historical hours |
|---|---|---|
| Complete localization catalogs, locale behavior, script support and language review | #11 | 100–165 |
| Get & Transform and the data model | #12 | 60–100 |
| Remaining Solver/Analysis ToolPak/Forecast parity and validation | #13 | 22–35 |
| Remaining functions, oracle suite | #15 | 18–30 |
| Collaboration and review | #16 | 35–55 |
| Insert objects | #17 | 25–40 |
| Accessibility | #18 | 12–18 |
| Page Layout view, print polish | #14 | 12–20 |
| Mobile layout | #23 | 30–60 |
| AI features with local models | #22 | 30–50 |
| Plugin / custom-function API (separate from Mini-VBA) | #21 | 25–40 |

Full Excel VBA compatibility is not a Rustani requirement. Historical estimates and readiness scores have not been recalculated for the sandboxed Mini-VBA policy.

## Measuring

- `cargo xtask parity` regenerates [`parity-checklist.md`](parity-checklist.md) (ribbon
  catalog vs registered commands); `cargo xtask ci` checks it is current. This is registration
  breadth, not an Excel compatibility percentage. Operational evidence is tracked separately in
  [operational-parity.md](operational-parity.md).
- `cargo test --workspace`: engine, formula, function, XLSX and UI behaviour.
- `cargo run --release -p gridcraft-ui-egui --example snapshot -- --sample sales out.png`: look at
  the UI offscreen.
- Function coverage: the script method in [function-parity.md](function-parity.md). A
  `cargo xtask functions` that diffs the registry against a checked-in Excel name list would make
  it a CI number (1–2 h).
- No `cargo xtask scorecard` or `bench` yet.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-27 | policy | Adopt sandboxed Mini-VBA as Rustani scope; retain historical scores and earlier entries |
| 2026-10-24 | minor | Reconcile implemented operational/UI/localization/picture features; prioritize Safari/real-OS IME and parameterized VBA/executable-project/Excel round-trip work; label inherited gate scores, estimates and PR/release references as historical |
| 2026-10-23 | minor | Add the active full VBA compatibility track, partial local foundation and metadata-save gap; historical hours exclude the unestimated VBA requirement |
| 2026-10-11 | minor | PivotTable calculated fields landed |
| 2026-10-10 | minor | Current focus and M15 updated for the 10-10 merges |
| 2026-10-10 | minor | Added the alpha gate table (six core workflows; passes, stays alpha) |
| 2026-10-10 | major | Created: current focus, beta gates (~160–240 h), milestones moved from ROADMAP.md, post-beta ranking |
