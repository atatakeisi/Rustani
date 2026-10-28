//! Interface translations. Command ids and document text stay stable; only interface chrome is
//! translated. Untranslated labels fall back to English, so coverage can grow incrementally.
//!
//! The interface language is a preference ([`crate::UiState::language`]): this edition's app
//! entry points prefer Japanese, and the View tab switches it by hand. Headless/test defaults
//! remain English. Web builds require the optional Japanese font before selecting Japanese.
//!
//! [`crate::theme`] appends system CJK faces on native platforms and the optional external
//! craft-fonts Japanese fallback in Web builds; font source files are not stored in this repo.

use serde::{Deserialize, Serialize};

mod cs;
mod de;
mod el;
mod es;
mod es_functions;
mod fr;
mod it;
mod ja;
mod ko;
mod nl;
mod pl;
mod pt;
mod ro;
mod ru;
mod sr;
mod tr;
mod uk;
mod vi;
mod zh;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
    El,
    Zh,
    Ja,
    Ko,
    Ru,
    #[serde(rename = "pt")]
    PtBr,
    De,
    Cs,
    Fr,
    Sr,
    Tr,
    Pl,
    It,
    Uk,
    Vi,
    Nl,
    Ro,
}

impl Language {
    pub const ALL: [Self; 19] = [
        Self::En,
        Self::Es,
        Self::El,
        Self::Zh,
        Self::Ja,
        Self::Ko,
        Self::Ru,
        Self::PtBr,
        Self::De,
        Self::Cs,
        Self::Fr,
        Self::Sr,
        Self::Pl,
        Self::It,
        Self::Uk,
        Self::Vi,
        Self::Nl,
        Self::Ro,
        Self::Tr,
    ];

    /// Every language in the order the switcher lists them: English first (the source language),
    /// then the rest by their own name with accents folded, so Čeština sits under C. Names in
    /// other scripts follow the Latin ones in code point order: Cyrillic, then the CJK languages.
    pub fn for_picker() -> Vec<Self> {
        let mut rest: Vec<Self> = Self::ALL.into_iter().filter(|l| *l != Self::En).collect();
        rest.sort_by_cached_key(|l| name_sort_key(l.name()));
        std::iter::once(Self::En).chain(rest).collect()
    }

    /// The language's own name, shown in the switcher.
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
            Self::El => "Ελληνικά",
            Self::Zh => "简体中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::Ru => "Русский",
            Self::PtBr => "Português (Brasil)",
            Self::De => "Deutsch",
            Self::Cs => "Čeština",
            Self::Fr => "Français",
            Self::Sr => "Srpski",
            Self::Tr => "Türkçe",
            Self::Pl => "Polski",
            Self::It => "Italiano",
            Self::Uk => "Українська",
            Self::Vi => "Tiếng Việt",
            Self::Nl => "Nederlands",
            Self::Ro => "Română",
        }
    }

    /// The language's code, as saved in `ui.json` and taken by `app.language.set`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::El => "el",
            Self::Zh => "zh",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Ru => "ru",
            Self::PtBr => "pt",
            Self::De => "de",
            Self::Cs => "cs",
            Self::Fr => "fr",
            Self::Sr => "sr",
            Self::Tr => "tr",
            Self::Pl => "pl",
            Self::It => "it",
            Self::Uk => "uk",
            Self::Vi => "vi",
            Self::Nl => "nl",
            Self::Ro => "ro",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        Self::from_tag(code)
    }

    /// Every code `app.language.set` takes, quoted and joined (`"en"|"es"|…`), for command docs
    /// and errors.
    pub fn code_list(separator: &str) -> String {
        Self::ALL.iter().map(|l| format!("\"{}\"", l.code())).collect::<Vec<_>>().join(separator)
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim().split(['-', '_']).next()?.to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "es" => Some(Self::Es),
            "el" => Some(Self::El),
            "zh" => Some(Self::Zh),
            "ja" => Some(Self::Ja),
            "ko" => Some(Self::Ko),
            "ru" => Some(Self::Ru),
            "pt" => Some(Self::PtBr),
            "de" => Some(Self::De),
            "cs" => Some(Self::Cs),
            "fr" => Some(Self::Fr),
            "sr" => Some(Self::Sr),
            "tr" => Some(Self::Tr),
            "pl" => Some(Self::Pl),
            "it" => Some(Self::It),
            "uk" => Some(Self::Uk),
            "vi" => Some(Self::Vi),
            "nl" => Some(Self::Nl),
            "ro" => Some(Self::Ro),
            _ => None,
        }
    }

    /// The system's preferred interface language, best effort; English when it is unknown or
    /// unsupported. Used as the first-run default while nothing is saved.
    pub fn system() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            sys_locale::get_locale().and_then(|tag| Self::from_tag(&tag)).unwrap_or(Self::En)
        }
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window().and_then(|window| window.navigator().language()).and_then(|tag| Self::from_tag(&tag)).unwrap_or(Self::En)
        }
    }

    pub fn formula_locale(self) -> gridcraft_engine::formula::FormulaLocale {
        if self == Self::Es { gridcraft_engine::formula::FormulaLocale::Es } else { gridcraft_engine::formula::FormulaLocale::En }
    }

    /// Translates an interface label; untranslated labels fall back to English.
    ///
    /// A label that needs two translations (Home's text "Orientation" vs Page Layout's print
    /// "Orientation") is keyed `context|label`: English shows only the part after the `|`.
    pub fn tr(self, text: &str) -> &str {
        let translated = match self {
            Self::En => None,
            Self::Es => es::translate(text),
            Self::El => el::translate(text),
            Self::Ja => ja::translate(text),
            Self::Zh => zh::translate(text),
            Self::Ko => ko::translate(text),
            Self::Ru => ru::translate(text),
            Self::PtBr => pt::translate(text),
            Self::De => de::translate(text),
            Self::Cs => cs::translate(text),
            Self::Fr => fr::translate(text),
            Self::Sr => sr::translate(text),
            Self::Tr => tr::translate(text),
            Self::Pl => pl::translate(text),
            Self::It => it::translate(text),
            Self::Uk => uk::translate(text),
            Self::Vi => vi::translate(text),
            Self::Nl => nl::translate(text),
            Self::Ro => ro::translate(text),
        };
        translated.unwrap_or_else(|| text.rsplit_once('|').map_or(text, |(_, label)| label))
    }

    /// Localizes user-facing errors, retaining identifiers and logging unknown Japanese keys.
    /// Command/API results remain untouched; apply this only at the presentation boundary.
    pub fn tr_error(self, message: &str) -> String {
        self.tr_error_nested(message, 0)
    }

    fn tr_error_nested(self, message: &str, depth: u8) -> String {
        let message = crate::clean_error(message);
        if self == Self::Ja {
            if depth >= 4 {
                return japanese_unknown_error(&message);
            }
            if let Some((_, detail)) = message.strip_prefix("internal error in `").and_then(|s| s.split_once("` (the workbook was kept as it was): "))
            {
                log::warn!("Internal command error: {detail}");
                return "内部エラーのため操作を中止しました。ブックは操作前の状態に保たれています。".into();
            }
            if let Some((_, translated)) = ja::ERRORS.iter().find(|(source, _)| *source == message) {
                return (*translated).to_owned();
            }
            if let Some(what) = message.strip_suffix(" isn't allowed on this protected sheet (Review › Unprotect Sheet).") {
                let operation = self.tr(what);
                let operation = if has_japanese(operation) { operation } else { "選択した操作" };
                return format!("保護されたシート上では{operation}は許可されていません（「校閲」＞「シート保護の解除」）。");
            }
            if let Some(name) = message
                .strip_prefix("The range \"")
                .and_then(|s| s.strip_suffix("\" needs its password before it can be changed (Review › Allow Edit Ranges)."))
            {
                return format!("範囲 \"{name}\" を変更するにはパスワードが必要です（「校閲」＞「範囲の編集を許可」）。");
            }
            if let Some(name) = message.strip_prefix("no sheet named `").and_then(|s| s.strip_suffix('`')) {
                return format!("シート `{name}` が見つかりません。");
            }
            if let Some((reference, _)) = message.strip_prefix("invalid cell reference `").and_then(|s| s.split_once("` for `")) {
                return format!("セル参照 `{reference}` が正しくありません。");
            }
            if let Some(range) = message.strip_prefix("invalid range `").and_then(|s| s.strip_suffix('`')) {
                return format!("セル範囲 `{range}` の指定が正しくありません。");
            }
            if message.starts_with("command `") && message.contains("` is not available right now:") {
                return "現在の選択範囲やブックの状態では、この操作を実行できません。".into();
            }
            for (prefix, suffix, label) in [
                ("VBA member or function `", "` is not implemented", "VBA のメンバーまたは関数"),
                ("VBA property `", "` assignment is not implemented", "VBA プロパティへの代入"),
            ] {
                if let Some(name) = message.strip_prefix(prefix).and_then(|s| s.strip_suffix(suffix)) {
                    return format!("{label} `{name}` は箱庭型ミニVBAでは未対応です");
                }
            }
            if let Some(name) = message.strip_prefix("mixed ").and_then(|s| s.strip_suffix(" values require VBA Null, which is not implemented")) {
                return format!("{name} の値が混在しています。必要な VBA の Null は未対応です");
            }
            for (prefix, translation) in [
                ("Variable not defined: ", "変数が定義されていません: "),
                ("Procedure not found: ", "プロシージャが見つかりません: "),
                ("Unsupported declaration type: ", "未対応の変数型です: "),
                ("Unsupported character: ", "使用できない文字: "),
                ("worksheet not found: ", "シートが見つかりません: "),
                ("invalid range: ", "範囲が正しくありません: "),
                ("Unsupported member ", "未対応のメンバー: "),
            ] {
                if let Some(detail) = message.strip_prefix(prefix) {
                    return format!("{translation}{detail}");
                }
            }
            if let Some(detail) = message.strip_prefix("worksheet error: ") {
                return format!("ワークシート エラー: {}", self.tr_error_nested(detail.trim_start_matches("worksheet error: "), depth + 1));
            }
            for (suffix, translation) in [
                (" arguments are not implemented", "の引数は箱庭型ミニVBAでは未対応です"),
                (" does not accept arguments", "に引数は指定できません"),
                (" requires one argument", "には引数が 1 つ必要です"),
                (" assignment is not implemented", "への代入は箱庭型ミニVBAでは未対応です"),
                (" is not implemented", "は箱庭型ミニVBAでは未対応です"),
            ] {
                if let Some(name) = message.strip_suffix(suffix) {
                    return format!("{name} {translation}");
                }
            }
            if let Some(expected) = message.strip_prefix("Expected '").and_then(|s| s.strip_suffix('\'')) {
                return format!("`{expected}` が必要です");
            }
            if let Some(reason) = message.strip_prefix("There's a problem with this formula: ") {
                return format!("この数式には問題があります: {}", japanese_formula_reason(reason));
            }
            for (pattern, description) in [(" has too few arguments. Use ", "少なすぎます"), (" has too many arguments. Use ", "多すぎます")]
            {
                if let Some((name, signature)) = message.split_once(pattern) {
                    return format!("この関数に対して、入力されている引数が{description} ({name})。使用方法: {signature}");
                }
            }
        }

        let translated = self.tr(&message);
        let fallback = message.rsplit_once('|').map_or(message.as_str(), |(_, label)| label);

        let result = if translated == fallback { message } else { translated.to_owned() };
        if self == Self::Ja && !has_japanese(&result) && result.chars().any(|c| c.is_ascii_alphabetic()) {
            return japanese_unknown_error(&result);
        }
        result
    }

    /// A worksheet function's description in this language, looked up by the English text of the
    /// function catalog; `None` when untranslated, and the help shows the English.
    pub fn function_description(self, english: &str) -> Option<&'static str> {
        lookup(self.function_help()?.0, english)
    }

    /// A function argument name in this language, given as the catalog spells it without
    /// brackets or index (`number`, `logical_test`).
    pub fn function_argument(self, english: &str) -> Option<&'static str> {
        lookup(self.function_help()?.1, english)
    }

    /// The function help tables (descriptions, argument names) of the languages that have them.
    fn function_help(self) -> Option<(HelpTable, HelpTable)> {
        match self {
            Self::Es => Some((es_functions::DESCRIPTIONS, es_functions::ARGUMENTS)),
            _ => None,
        }
    }
}

fn has_japanese(text: &str) -> bool {
    text.chars().any(|c| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}' | '\u{ff66}'..='\u{ff9f}'))
}

fn japanese_unknown_error(message: &str) -> String {
    log::warn!("Untranslated Japanese notification: {message}");
    "この操作を実行できませんでした。入力内容や選択範囲を確認してください。".into()
}

fn japanese_formula_reason(reason: &str) -> String {
    if let Some(translated) = ja::translate(reason) {
        return translated.into();
    }
    let token = |s: &str| match s {
        "end of formula" => "数式の終わり".into(),
        "a reference after the sheet name" => "シート名に続くセル参照".into(),
        "a number in array" => "配列内の数値".into(),
        "`,` or `)`" => "`,` または `)`".into(),
        "`,` or `;` or `}`" | "`,`, `;` or `}`" => "`,`、`;` または `}`".into(),
        other => ja::translate(other).unwrap_or(other).to_owned(),
    };
    if let Some(rest) = reason.strip_prefix("expected ")
        && let Some((expected, found)) = rest.split_once(", found ")
    {
        return format!("{} が必要ですが、{} が見つかりました", token(expected), token(found));
    }
    if let Some(rest) = reason.strip_prefix("unexpected character ") {
        return format!("使用できない文字 {rest} があります");
    }
    if let Some(rest) = reason.strip_prefix("unexpected ") {
        return format!("予期しない {} です", token(rest));
    }
    if let Some(rest) = reason.strip_prefix("formula is longer than ").and_then(|s| s.strip_suffix(" characters")) {
        return format!("数式は {rest} 文字以内にしてください");
    }
    for (prefix, label) in [("bad number ", "無効な数値"), ("number too large ", "数値が大きすぎます")] {
        if let Some(value) = reason.strip_prefix(prefix) {
            return format!("{label}: {value}");
        }
    }
    if let Some(rest) = reason.strip_prefix("array constants may only contain constants, found ") {
        return format!("配列定数に指定できるのは定数だけです: {}", token(rest));
    }
    log::warn!("Untranslated Japanese formula error: {reason}");
    "数式の構文を確認してください".into()
}

/// English text and its translation, sorted by the English.
type HelpTable = &'static [(&'static str, &'static str)];

/// The translation of `english` in a help table (binary search, so the table must stay sorted).
fn lookup(table: HelpTable, english: &str) -> Option<&'static str> {
    let i = table.binary_search_by(|(key, _)| (*key).cmp(english)).ok()?;
    table.get(i).map(|(_, translated)| *translated)
}

/// Deserializes a saved language leniently: an unknown or malformed value (a language removed in a
/// later version, a hand-edited `ui.json`) reads as English instead of failing the whole `UiState`,
/// which would reset every other preference with it.
pub fn lenient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Language, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Code(String),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Raw::deserialize(d)? {
        Raw::Code(code) => Language::parse(&code).unwrap_or_default(),
        Raw::Other(_) => Language::default(),
    })
}

/// The language a saved `ui.json` asks for, if it names a supported one. The desktop app uses it
/// to tell "saved English" from "nothing usable saved" (first run, an older file, an unknown
/// value), which keeps the app's initial language instead.
pub fn saved_language(ui_json: &serde_json::Value) -> Option<Language> {
    ui_json.get("language").and_then(serde_json::Value::as_str).and_then(Language::parse)
}

/// egui context-data key holding the language of the frame being drawn. Widgets that only get a
/// `Ui` (not the app) read it to translate their labels; [`set_current`] writes it once per frame.
fn lang_key() -> egui::Id {
    egui::Id::new("gridcraft.language")
}

/// The language the current frame is drawing in. `En` when unset.
pub fn current(ctx: &egui::Context) -> Language {
    ctx.data(|d| d.get_temp(lang_key())).unwrap_or_default()
}

/// Sets the language for the current frame; call once per frame before anything is drawn.
pub fn set_current(ctx: &egui::Context, language: Language) {
    ctx.data_mut(|d| d.insert_temp(lang_key(), language));
}

/// `name` lowercased with the Latin accents that occur in language names folded away, so a
/// byte-wise sort reads alphabetically. Other scripts are left as they are.
fn name_sort_key(name: &str) -> String {
    name.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à'..='å' | 'ā' | 'ă' | 'ą' => 'a',
            'ç' | 'ć' | 'č' => 'c',
            'è'..='ë' | 'ē' | 'ě' | 'ę' | 'ế' | 'ệ' => 'e',
            'ì'..='ï' | 'ī' => 'i',
            'ñ' | 'ń' | 'ň' => 'n',
            'ò'..='ö' | 'ø' | 'ő' => 'o',
            'ś' | 'š' => 's',
            'ù'..='ü' | 'ū' | 'ů' | 'ű' => 'u',
            'ý' | 'ÿ' => 'y',
            'ź' | 'ż' | 'ž' => 'z',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_error_wrappers_preserve_identifiers_and_recovery_advice() {
        assert_eq!(Language::Ja.tr_error("invalid parameters for `cell.set`: no such sheet"), "指定されたシートが見つかりません。");
        assert_eq!(
            Language::Ja.tr_error("internal error in `edit.paste` (the workbook was kept as it was): failed invariant"),
            "内部エラーのため操作を中止しました。ブックは操作前の状態に保たれています。"
        );
        assert_eq!(Language::Ja.tr_error("invalid cell reference `ZZ0` for `cell`"), "セル参照 `ZZ0` が正しくありません。");
        assert_eq!(Language::Ja.tr_error("invalid range `A0:B1`"), "セル範囲 `A0:B1` の指定が正しくありません。");
        assert_eq!(
            Language::Ja.tr_error("The range \"売上|East\" needs its password before it can be changed (Review › Allow Edit Ranges)."),
            "範囲 \"売上|East\" を変更するにはパスワードが必要です（「校閲」＞「範囲の編集を許可」）。"
        );
        assert_eq!(crate::clean_error("User text `key`: keep this"), "User text `key`: keep this");
        assert_eq!(Language::En.tr_error("Printer Office: permission denied"), "Printer Office: permission denied");
        // Successful notifications must retain their meaning, never turn into generic failures.
        assert_eq!(
            Language::Ja.tr_error("Solver found an optimal solution. All constraints are satisfied."),
            "最適解が見つかりました。すべての制約を満たしています。"
        );
    }

    #[test]
    fn japanese_fallback_is_bounded_and_fullwidth_punctuation_is_not_translation() {
        let fallback = "この操作を実行できませんでした。入力内容や選択範囲を確認してください。";
        assert_eq!(Language::Ja.tr_error("Future error！"), fallback);
        assert_eq!(Language::Ja.tr_error("Future error|detail"), fallback);
        let nested = "worksheet error: command `nested`: ".repeat(32) + "Future error";
        let result = Language::Ja.tr_error(&nested);
        assert!(result.ends_with(fallback));
        assert!(result.len() < 400, "nested wrappers must not cause unbounded recursion or output");
        assert_eq!(Language::Ja.tr_error("worksheet error: no workbook open"), "ワークシート エラー: ブックが開かれていません。");
        assert_eq!(
            Language::Ja.tr_error("There's a problem with this formula: future parser reason"),
            "この数式には問題があります: 数式の構文を確認してください"
        );
    }

    #[test]
    fn japanese_formula_errors_translate_structure_without_losing_parameters() {
        for (source, expected) in [
            ("There's a problem with this formula: unexpected end of formula", "この数式には問題があります: 数式が途中で終わっています"),
            (
                "There's a problem with this formula: expected `,` or `)`, found end of formula",
                "この数式には問題があります: `,` または `)` が必要ですが、数式の終わり が見つかりました",
            ),
            (
                "SUM has too few arguments. Use SUM(number1, [number2])",
                "この関数に対して、入力されている引数が少なすぎます (SUM)。使用方法: SUM(number1, [number2])",
            ),
            ("IF has too many arguments. Use IF(test, a, b)", "この関数に対して、入力されている引数が多すぎます (IF)。使用方法: IF(test, a, b)"),
            ("There's a problem with this formula: unterminated text", "この数式には問題があります: 文字列が閉じられていません"),
            ("VBA member or function `Foo` is not implemented", "VBA のメンバーまたは関数 `Foo` は箱庭型ミニVBAでは未対応です"),
            ("Variable not defined: example", "変数が定義されていません: example"),
            ("worksheet not found: 売上", "シートが見つかりません: 売上"),
        ] {
            assert_eq!(Language::Ja.tr_error(source), expected);
            assert_eq!(Language::En.tr_error(source), source);
        }
        let keys: std::collections::HashSet<_> = ja::CORE.iter().chain(ja::DIALOGS).chain(ja::VBA).map(|(key, _)| key).collect();
        let mut extra = std::collections::HashSet::new();
        for (key, translated) in ja::ERRORS {
            assert!(!keys.contains(key), "duplicate key {key}");
            assert!(extra.insert(key));
            assert_eq!(Language::Ja.tr_error(key), *translated);
        }
    }

    /// Help tables are sorted, unique, and keyed by text the function catalog still has.
    #[test]
    fn function_help_is_sorted_and_keyed_by_current_catalog_text() {
        // Lookups binary-search the tables. A key that no longer matches the catalog (an English
        // description reworded, an argument renamed) would quietly show English again.
        let catalog = gridcraft_engine::cmd::formulas::function_list();
        let descriptions: std::collections::HashSet<&str> = catalog.iter().filter_map(|f| f["description"].as_str()).collect();
        let arguments: std::collections::HashSet<&str> =
            catalog.iter().filter_map(|f| f["signature"].as_str()).flat_map(crate::formula_locale::argument_names).collect();
        for language in Language::ALL {
            let Some((description_table, argument_table)) = language.function_help() else { continue };
            for (table, catalog_text) in [(description_table, &descriptions), (argument_table, &arguments)] {
                assert!(table.windows(2).all(|pair| pair[0].0 < pair[1].0), "{language:?} function help is not sorted and unique");
                for (english, translated) in table {
                    assert!(catalog_text.contains(english), "{language:?} translates {english:?}, which the function catalog doesn't use");
                    assert!(!translated.is_empty(), "{language:?}: empty translation of {english:?}");
                }
            }
        }
    }

    #[test]
    fn translations_are_unique_and_preserve_unknown_text() {
        for (i, (en, ja)) in ja::CORE.iter().enumerate() {
            assert!(!ja.is_empty());
            assert!(ja::CORE.iter().take(i).all(|(other, _)| en != other));
            assert_eq!(Language::En.tr(en), en.rsplit_once('|').map_or(*en, |(_, label)| label));
        }
        for (english, _) in ja::CORE {
            assert!(zh::translate(english).is_some_and(|text| !text.is_empty()), "missing Simplified Chinese: {english}");
            assert!(ko::translate(english).is_some_and(|text| !text.is_empty()), "missing Korean: {english}");
            assert!(ru::translate(english).is_some_and(|text| !text.is_empty()), "missing Russian: {english}");
            assert!(pt::translate(english).is_some_and(|text| !text.is_empty()), "missing Portuguese: {english}");
            assert!(es::translate(english).is_some_and(|text| !text.is_empty()), "missing Spanish: {english}");
            assert!(el::translate(english).is_some_and(|text| !text.is_empty()), "missing Greek: {english}");
            assert!(de::translate(english).is_some_and(|text| !text.is_empty()), "missing German: {english}");
            assert!(cs::translate(english).is_some_and(|text| !text.is_empty()), "missing Czech: {english}");
            assert!(fr::translate(english).is_some_and(|text| !text.is_empty()), "missing French: {english}");
            assert!(sr::translate(english).is_some_and(|text| !text.is_empty()), "missing Serbian: {english}");
            assert!(tr::translate(english).is_some_and(|text| !text.is_empty()), "missing Turkish: {english}");
            assert!(pl::translate(english).is_some_and(|text| !text.is_empty()), "missing Polish: {english}");
            assert!(it::translate(english).is_some_and(|text| !text.is_empty()), "missing Italian: {english}");
            assert!(uk::translate(english).is_some_and(|text| !text.is_empty()), "missing Ukrainian: {english}");
            assert!(vi::translate(english).is_some_and(|text| !text.is_empty()), "missing Vietnamese: {english}");
            assert!(nl::translate(english).is_some_and(|text| !text.is_empty()), "missing Dutch: {english}");
            assert!(ro::translate(english).is_some_and(|text| !text.is_empty()), "missing Romanian: {english}");
        }
        assert_eq!(Language::Ja.tr("Data"), "データ");
        assert_eq!(Language::Zh.tr("Data"), "数据");
        assert_eq!(Language::Ko.tr("Data"), "데이터");
        assert_eq!(Language::Ru.tr("Data"), "Данные");
        assert_eq!(Language::PtBr.tr("Data"), "Dados");
        assert_eq!(Language::PtBr.tr_error("Select a chart first."), "Selecione um gráfico primeiro.");
        assert_eq!(Language::PtBr.tr_error("Sheet|Unknown failure"), "Sheet|Unknown failure");
        assert_eq!(Language::PtBr.tr_error("command `chart`: Select a chart first."), "Selecione um gráfico primeiro.");
        assert_eq!(Language::PtBr.tr_error("Printer Office: permission denied"), "Printer Office: permission denied");
        assert_eq!(Language::Es.tr("Data"), "Datos");
        assert_eq!(Language::El.tr("Data"), "Δεδομένα");
        assert_eq!(Language::El.tr("Page Layout|Orientation"), "Προσανατολισμός σελίδας");
        assert_eq!(Language::El.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::El.tr("Page {page} of {pages}"), "Σελίδα {page} από {pages}");
        assert_eq!(Language::Es.tr("Freeze Panes"), "Inmovilizar paneles");
        assert_eq!(Language::Es.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::De.tr("Data"), "Daten");
        assert_eq!(Language::De.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Fr.tr("Data"), "Données");
        assert_eq!(Language::Fr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Tr.tr("Data"), "Veri");
        assert_eq!(Language::Tr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::PtBr.tr("Date"), "Data");
        assert_eq!(Language::PtBr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ja.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Zh.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ko.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ru.tr("Sheet1!A1"), "Sheet1!A1");
        // Czech "Data" is spelled as in English, so probe it with a label that differs.
        assert_eq!(Language::Cs.tr("Data"), "Data");
        assert_eq!(Language::Cs.tr("Formulas"), "Vzorce");
        assert_eq!(Language::Cs.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Tr.tr("Formulas"), "Formüller");
        // A `context|label` key shows only its label in English and splits the translation.
        assert_eq!(Language::En.tr("Page Layout|Orientation"), "Orientation");
        assert_eq!(Language::Ja.tr("Page Layout|Orientation"), "印刷の向き");
        assert_eq!(Language::Ja.tr("Orientation"), "方向");
        assert_eq!(Language::Cs.tr("Page Layout|Orientation"), "Orientace");
        assert_eq!(Language::Cs.tr("Orientation"), "Orientace textu");
        assert_eq!(Language::Sr.tr("Data"), "Podaci");
        assert_eq!(Language::Sr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Sr.tr("Page Layout|Orientation"), "Orijentacija");
        assert_eq!(Language::Sr.tr("Orientation"), "Orijentacija");
        assert_eq!(Language::Tr.tr("Page Layout|Orientation"), "Sayfa Yönü");
        assert_eq!(Language::Tr.tr("Orientation"), "Yönlendirme");
        assert_eq!(Language::Pl.tr("Data"), "Dane");
        assert_eq!(Language::Pl.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Pl.tr("Page Layout|Orientation"), "Orientacja");
        assert_eq!(Language::It.tr("Data"), "Dati");
        assert_eq!(Language::It.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::It.tr("Freeze Panes"), "Blocca riquadri");
        assert_eq!(Language::It.tr("Page Layout|Orientation"), "Orientamento");
        assert_eq!(Language::It.tr("Orientation"), "Orientamento");
        assert_eq!(Language::Uk.tr("Data"), "Дані");
        assert_eq!(Language::Uk.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Vi.tr("Data"), "Dữ liệu");
        assert_eq!(Language::Vi.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Vi.tr("Page Layout|Orientation"), "Hướng giấy");
        assert_eq!(Language::Vi.tr("Orientation"), "Hướng chữ");
        assert_eq!(Language::Nl.tr("Data"), "Gegevens");
        assert_eq!(Language::Nl.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Nl.tr("Page Layout|Orientation"), "Paginastand");
        assert_eq!(Language::Nl.tr("Orientation"), "Richting");
        assert_eq!(Language::Ro.tr("Data"), "Date");
        assert_eq!(Language::Ro.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ro.tr("Page Layout|Orientation"), "Orientare");
        assert_eq!(Language::Ro.tr("Orientation"), "Orientare text");
        assert_eq!(Language::parse("xx"), None);
    }

    /// A catalog whose translations slip onto their neighbours' keys still covers every key, so
    /// check the shape of each translation against its English: placeholders survive, and a
    /// one-line label (a menu item, a tooltip) doesn't turn into a two-line ribbon label. French
    /// once shipped with over a hundred translations shifted this way ("Close" read "Société").
    #[test]
    fn translations_keep_the_shape_of_their_english() {
        let placeholders =
            |s: &str| s.split('{').skip(1).filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_string())).collect::<Vec<_>>();
        for language in Language::ALL {
            for (english, _) in ja::CORE {
                let label = english.rsplit_once('|').map_or(*english, |(_, label)| label);
                let text = language.tr(english);
                assert_eq!(placeholders(text), placeholders(label), "{language:?}: {english:?} => {text:?}");
                // Big ribbon buttons whose English fits on one line may still need two.
                if !label.contains('\n') && !["AutoSum", "PivotTable"].contains(english) {
                    assert!(!text.contains('\n'), "{language:?}: one-line {english:?} => {text:?}");
                }
            }
        }
        for (english, french) in [
            ("Close", "Fermer"),
            ("File", "Fichier"),
            ("Save", "Enregistrer"),
            ("Animations", "Animations"),
            ("Company", "Société"),
            ("Show Detail", "Afficher le détail"),
            ("Solver", "Solveur"),
        ] {
            assert_eq!(Language::Fr.tr(english), french);
        }
    }

    #[test]
    fn german_cell_and_header_context_menus_are_fully_translated() {
        for label in [
            "Cut",
            "Copy",
            "Paste",
            "Paste Special…",
            "Insert…",
            "Delete…",
            "Clear Contents",
            "Filter by Selected Cell's Value",
            "Sort A to Z",
            "Sort Z to A",
            "New Comment",
            "New Note",
            "Format Cells…",
            "Pick From Drop-down List…",
            "Define Name…",
            "Insert Copied Cells",
            "Insert Copied Cells…",
            "Insert Cut Cells",
            "Insert",
            "Delete",
            "Row Height…",
            "AutoFit Row Height",
            "Column Width…",
            "AutoFit Column Width",
            "Hide",
            "Unhide",
            "Group",
        ] {
            assert_ne!(Language::De.tr(label), label, "German context menu label missing: {label}");
        }
    }

    #[test]
    fn sheet_tab_menu_and_autosave_labels_are_translated() {
        let labels = [
            "Insert Sheet",
            "Delete",
            "Rename",
            "Move or Copy…",
            "Duplicate",
            "Tab Color",
            "No Color",
            "Hide",
            "Unhide…",
            "Protect Sheet…",
            "Unprotect Sheet…",
            "Select All Sheets",
            "AutoSave",
            "AutoSave: save after every change (workbooks saved to a file)",
            "Previous sheet",
            "Next sheet",
            "All sheets",
            "New sheet (⇧F11)",
        ];
        for language in [Language::Es, Language::De, Language::Fr, Language::Ko, Language::Ru, Language::Zh, Language::Sr] {
            for label in labels {
                assert_ne!(language.tr(label), label, "{} lacks a translation of {label:?}", language.code());
            }
        }
        assert_eq!(Language::Es.tr("AutoSave"), "Autoguardado");
    }

    #[test]
    fn translations_never_show_the_context_prefix() {
        // `tr` returns a found translation as it is, so a catalog that keeps `context|` in its
        // value shows it on screen (File › Options read "Options|Général" in French).
        for language in Language::ALL {
            for (english, _) in ja::CORE {
                let shown = language.tr(english);
                assert!(!shown.contains('|'), "{} shows a context prefix for {english}: {shown}", language.code());
            }
        }
        assert_eq!(Language::Fr.tr("Options|General"), "Général");
        assert_eq!(Language::Fr.tr("Options|Save"), "Enregistrer");
        assert_eq!(Language::Fr.tr("Sheet Options|View"), "Affichage");
        assert_eq!(Language::Sr.tr("Options|General"), "Opšte");
        assert_eq!(Language::Sr.tr("Options|Save"), "Sačuvaj");
        assert_eq!(Language::Sr.tr("Sheet Options|View"), "Prikaz");
        assert_eq!(Language::Nl.tr("Options|General"), "Algemeen");
        assert_eq!(Language::Nl.tr("Options|Save"), "Opslaan");
        assert_eq!(Language::Nl.tr("Sheet Options|View"), "Weergeven");
        assert_eq!(Language::Ro.tr("Options|General"), "General");
        assert_eq!(Language::Ro.tr("Options|Save"), "Salvare");
        assert_eq!(Language::Ro.tr("Sheet Options|View"), "Vizualizare");
    }

    #[test]
    fn unknown_saved_language_keeps_the_other_preferences() {
        let ui: crate::UiState = serde_json::from_str(r#"{"dark": true, "language": "xx"}"#).unwrap();
        assert!(ui.dark);
        assert_eq!(ui.language, Language::En);
        let ui: crate::UiState = serde_json::from_str(r#"{"dark": true, "language": 7}"#).unwrap();
        assert!(ui.dark);
        let ui: crate::UiState = serde_json::from_str(r#"{"language": "ja"}"#).unwrap();
        assert_eq!(ui.language, Language::Ja);
        assert_eq!(crate::UiState::default().language, Language::En, "the default never reads the host locale");
        assert_eq!(saved_language(&serde_json::json!({"language": "ja"})), Some(Language::Ja));
        assert_eq!(saved_language(&serde_json::json!({"language": "xx"})), None);
        assert_eq!(saved_language(&serde_json::json!({})), None);
    }

    #[test]
    fn tags_reduce_to_a_supported_language() {
        assert_eq!(Language::from_tag("ja-JP"), Some(Language::Ja));
        assert_eq!(Language::from_tag("en_US"), Some(Language::En));
        assert_eq!(Language::from_tag("es_MX"), Some(Language::Es));
        assert_eq!(Language::from_tag("el-GR"), Some(Language::El));
        assert_eq!(Language::from_tag("el_GR.UTF-8"), Some(Language::El));
        assert_eq!(Language::parse("el"), Some(Language::El));
        assert_eq!(Language::from_tag("zh-Hans-CN"), Some(Language::Zh));
        assert_eq!(Language::from_tag("ko-KR"), Some(Language::Ko));
        assert_eq!(Language::from_tag("RU_ru"), Some(Language::Ru));
        assert_eq!(Language::from_tag("de-DE"), Some(Language::De));
        assert_eq!(Language::from_tag("de_AT"), Some(Language::De));
        assert_eq!(Language::from_tag("de-CH"), Some(Language::De));
        assert_eq!(Language::from_tag("pt-BR"), Some(Language::PtBr));
        assert_eq!(Language::parse("pt"), Some(Language::PtBr));
        assert_eq!(Language::parse("pt-br"), Some(Language::PtBr));
        assert_eq!(Language::from_tag("cs-CZ"), Some(Language::Cs));
        assert_eq!(Language::from_tag("cs_CZ.UTF-8"), Some(Language::Cs));
        assert_eq!(Language::parse("cs"), Some(Language::Cs));
        assert_eq!(Language::from_tag("fr-FR"), Some(Language::Fr));
        assert_eq!(Language::from_tag("fr_CA"), Some(Language::Fr));
        assert_eq!(Language::parse("fr"), Some(Language::Fr));
        assert_eq!(Language::from_tag("sr-Latn-RS"), Some(Language::Sr));
        assert_eq!(Language::from_tag("sr-RS"), Some(Language::Sr));
        assert_eq!(Language::parse("sr"), Some(Language::Sr));
        assert_eq!(Language::from_tag("tr-TR"), Some(Language::Tr));
        assert_eq!(Language::from_tag("tr_TR.UTF-8"), Some(Language::Tr));
        assert_eq!(Language::parse("tr"), Some(Language::Tr));
        assert_eq!(Language::from_tag("pl-PL"), Some(Language::Pl));
        assert_eq!(Language::from_tag("pl_PL.UTF-8"), Some(Language::Pl));
        assert_eq!(Language::parse("pl"), Some(Language::Pl));
        assert_eq!(Language::from_tag("it-IT"), Some(Language::It));
        assert_eq!(Language::from_tag("it_CH.UTF-8"), Some(Language::It));
        assert_eq!(Language::parse("it"), Some(Language::It));
        assert_eq!(Language::from_tag("uk-XX"), Some(Language::Uk));
        assert_eq!(Language::parse("uk"), Some(Language::Uk));
        assert_eq!(Language::from_tag("vi-VN"), Some(Language::Vi));
        assert_eq!(Language::parse("vi"), Some(Language::Vi));
        assert_eq!(Language::from_tag("nl-NL"), Some(Language::Nl));
        assert_eq!(Language::from_tag("nl_BE"), Some(Language::Nl));
        assert_eq!(Language::parse("nl"), Some(Language::Nl));
        assert_eq!(Language::from_tag("ro-RO"), Some(Language::Ro));
        assert_eq!(Language::from_tag("ro_MD.UTF-8"), Some(Language::Ro));
        assert_eq!(Language::parse("ro"), Some(Language::Ro));
    }

    #[test]
    fn an_unset_language_preference_is_english_until_the_app_applies_the_system_one() {
        // Deserializing never reads the host locale (tests stay deterministic); the desktop app
        // applies `Language::system()` itself when `ui.json` names no usable language.
        let restored: crate::UiState = serde_json::from_str("{}").unwrap();
        assert_eq!(restored.language, Language::En);
        assert_eq!(saved_language(&serde_json::json!({"language": "zh"})), Some(Language::Zh));
    }

    #[test]
    fn language_commands_switch_and_persist() {
        let mut app = crate::SheetApp::new(gridcraft_engine::Session::default(), Default::default());
        for language in Language::ALL {
            app.run("app.language.set", serde_json::json!({"language": language.code()})).unwrap();
            assert_eq!(app.ui.language, language);
            let restored: crate::UiState = serde_json::from_str(&serde_json::to_string(&app.ui).unwrap()).unwrap();
            assert_eq!(restored.language, language);
        }
        app.run("app.language.set", serde_json::json!({"code": "ja"})).unwrap();
        assert_eq!(app.ui.language, Language::Ja, "`code` is an alias for `language`");
        assert!(app.run("app.language.set", serde_json::json!({})).is_err(), "no language is an error, not a dialog");
        let unknown = app.run("app.language.set", serde_json::json!({"language": "xx"})).unwrap_err().to_string();
        for l in Language::ALL {
            assert!(unknown.contains(&format!("\"{}\"", l.code())), "the error lists {:?}: {unknown}", l.code());
        }
        app.run("app.language.english", serde_json::json!({})).unwrap();
        assert_eq!(app.ui.language, Language::En);
        for l in Language::ALL {
            assert_eq!(Language::parse(l.code()), Some(l));
        }
    }

    #[test]
    fn picker_lists_english_first_then_names_alphabetically() {
        let order = Language::for_picker();
        assert_eq!(order.first(), Some(&Language::En));
        assert_eq!(order.len(), Language::ALL.len());
        let keys: Vec<String> = order[1..].iter().map(|l| name_sort_key(l.name())).collect();
        assert!(keys.windows(2).all(|w| w[0] <= w[1]), "{keys:?}");
        assert_eq!(name_sort_key("Čeština"), "cestina");
    }
}
