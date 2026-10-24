//! File kinds that preserve the whole workbook when it is handed to an embedding host.

use gridcraft_engine::io::FileKind;

pub(crate) fn is_workbook(name: &str) -> bool {
    matches!(FileKind::from_path(name), Some(FileKind::Xlsx))
}

#[cfg(test)]
mod tests {
    use super::is_workbook;

    #[test]
    fn recognizes_full_workbook_and_template_filenames() {
        for name in ["book.xlsx", "macros.xlsm", "template.xltx", "macros.xltm", "MACROS.XLSM", "Budget.2026.XLTM"] {
            assert!(is_workbook(name), "{name} saves the whole workbook");
        }
    }

    #[test]
    fn exports_import_only_formats_and_unknown_files_are_not_full_workbook_saves() {
        for name in ["book.csv", "book.pdf", "book.json", "book.xls", "book.xlsb", "book.ods", "book.xlsx.pdf", "book", "book."] {
            assert!(!is_workbook(name), "{name} must leave unsaved workbook changes dirty");
        }
    }
}
