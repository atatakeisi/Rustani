//! VBA editing and cooperative execution, shared by the UI, CLI and MCP.
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "automate.vbaModules", "VBA Modules", [], None, "{} → {modules, trusted, preservedVba, importError?}", has_doc, crate::vba::modules),
        cmd!(
            "automate.saveVbaModule",
            "Save VBA Module",
            [],
            None,
            "{name, source} → {name, procedures, compileError?}; stores source in GridCraft workbook metadata",
            has_doc,
            crate::vba::save_module
        ),
        cmd!(query "automate.startVba", "Run VBA", [], None, "{module | source, procedure} → status; all execution requires workbook session trust", has_doc, crate::vba::start),
        cmd!(query "automate.pollVba", "Continue VBA", [], None, "{budget?: 1..1000} → status; commits a successful macro as one Undo step", always, crate::vba::poll),
        cmd!(query "automate.stopVba", "Stop VBA", [], None, "{} → status; discards all pending workbook changes", always, crate::vba::stop),
        cmd!(query "automate.vbaStatus", "VBA Status", [], None, "{} → {state, module, procedure, line, steps, log, error?}", always, crate::vba::status),
    ]
}
