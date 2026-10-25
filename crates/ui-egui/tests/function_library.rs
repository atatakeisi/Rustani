//! Function library menus expose the engine's categories and share AutoSum across tabs.

use egui::{Event, Key, Modifiers, accesskit::Role, os::OperatingSystem};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::{
    Session,
    core::{CellRef, Value},
};
use gridcraft_ui_egui::{SheetApp, i18n::Language};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness(tab: &str, language: Language) -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [[2], [4], [6]]})).unwrap();
    session.execute("selection.set", json!({"cell": "A4"})).unwrap();
    let mut app = SheetApp::new(session, Default::default());
    app.ui.ribbon_tab = tab.into();
    app.ui.language = language;
    let mut h = Harness::builder().with_os(OperatingSystem::Nix).with_size(egui::vec2(1800.0, 1000.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(5);
    h
}

fn click(h: &mut Harness, label: &str) {
    if let Some(node) = h.query_by_label(label) {
        node.click();
    } else {
        // egui appends its submenu arrow to menu_button's accessibility label.
        h.get_by_label(&format!("{label} ⏵")).click();
    }
    h.run_steps(4);
}

fn key(h: &mut Harness, key: Key) {
    h.key_press(key);
    h.run_steps(2);
}

fn letter(h: &mut Harness, key: Key) {
    h.input_mut().events.extend([
        Event::Text(key.name().to_ascii_lowercase()),
        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
        Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
    ]);
    h.run_steps(3);
}

fn assert_function_category(function: &str, category: &str) {
    assert!(gridcraft_engine::cmd::formulas::function_list().iter().any(|entry| entry["name"] == function && entry["category"] == category));
}

#[test]
fn more_functions_exposes_each_registered_category_as_a_submenu() {
    for (category, label, function) in [
        ("Statistical", "Statistical", "AVEDEV"),
        ("Engineering", "Engineering", "BIN2DEC"),
        ("Information", "Information", "CELL"),
        ("Database", "Database", "DAVERAGE"),
        ("Compatibility", "Compatibility", "BETADIST"),
        ("Web", "Web", "ENCODEURL"),
        ("AddIn", "Add-in and Automation", "EUROCONVERT"),
    ] {
        assert_function_category(function, category);
        let mut h = harness("Formulas", Language::En);
        let before = h.state().session.doc().unwrap().wb.clone();
        click(&mut h, "More\nFunctions");
        for category_label in ["Statistical", "Engineering", "Information", "Database", "Compatibility", "Web", "Add-in and Automation"] {
            h.get_by_label(&format!("{category_label} ⏵"));
        }
        assert_eq!(h.query_all_by_label("ENCODEURL").count(), 0, "function entries belong inside their submenu");
        assert_eq!(h.query_all_by_label("Cube").count(), 0, "unrequested stub categories are not invented");
        click(&mut h, label);
        click(&mut h, function);
        assert_eq!(h.state().editor.as_ref().unwrap().text, format!("={function}("));
        assert_eq!(h.state().session.doc().unwrap().wb, before, "choosing a function starts a draft without editing worksheet cells");
    }
}

#[test]
fn every_category_footer_opens_insert_function_with_its_canonical_filter() {
    for (nested, label, category) in [
        (false, "Financial", "Financial"),
        (false, "Logical", "Logical"),
        (false, "Text", "Text"),
        (false, "Date &\nTime", "DateTime"),
        (false, "Lookup &\nReference", "Lookup"),
        (false, "Math &\nTrig", "MathTrig"),
        (true, "Statistical", "Statistical"),
        (true, "Engineering", "Engineering"),
        (true, "Information", "Information"),
        (true, "Database", "Database"),
        (true, "Compatibility", "Compatibility"),
        (true, "Web", "Web"),
        (true, "Add-in and Automation", "AddIn"),
    ] {
        let mut h = harness("Formulas", Language::En);
        if nested {
            click(&mut h, "More\nFunctions");
        }
        click(&mut h, label);
        click(&mut h, "More Functions…");
        let dialog = h.state().dialog.as_ref().unwrap();
        assert_eq!(dialog.name(), "insertFunction");
        assert_eq!(dialog.values.get("category"), Some(&json!(category)), "{label}");
        assert!(
            h.state().editor.is_none(),
            "opening category {category} ({label}, nested={nested}) must not insert a formula: editor={:?}, search={:?}, active={:?}",
            h.state().editor.as_ref().map(|editor| (&editor.text, editor.cell, editor.from_formula_bar)),
            dialog.search,
            h.state().session.doc().unwrap().selection.active
        );
    }
}

#[test]
fn function_browser_combines_the_category_and_search_and_updates_after_switching_categories() {
    let mut h = harness("Formulas", Language::En);
    click(&mut h, "More\nFunctions");
    click(&mut h, "Web");
    click(&mut h, "More Functions…");
    let encode_row = format!("{:<18} {}", "ENCODEURL", "Web");
    let euro_row = format!("{:<18} {}", "EUROCONVERT", "Add-in and Automation");
    let if_row = format!("{:<18} {}", "IF", "Logical");
    h.get_by_role_and_label(Role::Button, &encode_row);
    assert_eq!(h.query_all_by_role_and_label(Role::Button, &if_row).count(), 0);
    assert_eq!(h.query_all_by_role_and_label(Role::Button, &euro_row).count(), 0);

    // The dialog initially focuses Search. A unique function outside Web makes the AND
    // condition observable without relying on which rows happen to fit in a long result list.
    h.input_mut().events.push(Event::Text("EUROCONVERT".into()));
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().unwrap().search, "EUROCONVERT");
    assert_eq!(h.query_all_by_role_and_label(Role::Button, &encode_row).count(), 0, "the search also narrows the selected category");
    assert_eq!(h.query_all_by_role_and_label(Role::Button, &euro_row).count(), 0, "matching another category does not bypass Web");

    h.get_all_by_role(Role::ComboBox).last().unwrap().click();
    h.run_steps(2);
    click(&mut h, "All");
    h.get_by_role_and_label(Role::Button, &euro_row);
    assert_eq!(h.query_all_by_role_and_label(Role::Button, &encode_row).count(), 0, "changing category retains the search filter");

    h.get_all_by_role(Role::TextInput).last().unwrap().click();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(1);
    key(&mut h, Key::Backspace);
    assert!(h.state().dialog.as_ref().unwrap().search.is_empty());
    h.get_all_by_role(Role::ComboBox).last().unwrap().click();
    h.run_steps(2);
    // Web is the last option in the combo's 200-point scrolling viewport. A node click
    // only sends pointer events; scroll it into view before clicking its updated rectangle.
    h.get_by_label("Web").scroll_to_me();
    h.run_steps(8);
    click(&mut h, "Web");
    assert_eq!(
        h.state().dialog.as_ref().unwrap().values.get("category"),
        Some(&json!("Web")),
        "category selection must apply before checking rows; search={:?}, popup_open={}",
        h.state().dialog.as_ref().unwrap().search,
        egui::Popup::is_any_open(&h.ctx),
    );
    h.get_by_role_and_label(Role::Button, &encode_row);
    assert_eq!(h.query_all_by_role_and_label(Role::Button, &euro_row).count(), 0);
}

#[test]
fn japanese_category_labels_keep_the_canonical_filter_and_function_name() {
    let mut h = harness("Formulas", Language::Ja);
    click(&mut h, "その他の関数");
    click(&mut h, "アドインと自動化");
    h.get_by_label("EUROCONVERT");
    click(&mut h, "その他の関数…");
    assert_eq!(h.state().dialog.as_ref().unwrap().values.get("category"), Some(&json!("AddIn")));
}

#[test]
fn both_tabs_open_the_same_autosum_menu_and_apply_the_chosen_function() {
    for tab in ["Home", "Formulas"] {
        for (label, function, value) in
            [("Sum", "SUM", 12.0), ("Average", "AVERAGE", 4.0), ("Count Numbers", "COUNT", 3.0), ("Max", "MAX", 6.0), ("Min", "MIN", 2.0)]
        {
            let mut h = harness(tab, Language::En);
            let before = h.state().session.doc().unwrap().wb.clone();
            click(&mut h, "AutoSum");
            assert_eq!(h.state().session.doc().unwrap().wb, before, "{tab} opens the menu before running a calculation");
            assert!(h.state().editor.is_none());
            for item in ["Sum", "Average", "Count Numbers", "Max", "Min", "More Functions…"] {
                h.get_by_role_and_label(Role::Button, item);
            }
            click(&mut h, label);
            let sh = h.state().session.doc().unwrap().wb.active().unwrap();
            let at = CellRef::parse("A4").unwrap();
            assert_eq!(sh.input_text(at), format!("={function}(A1:A3)"));
            assert_eq!(sh.value(at), Value::number(value));
        }
    }
}

#[test]
fn autosum_more_functions_opens_the_same_function_browser_on_both_tabs() {
    for tab in ["Home", "Formulas"] {
        let mut h = harness(tab, Language::En);
        click(&mut h, "AutoSum");
        click(&mut h, "More Functions…");
        assert_eq!(h.state().dialog.as_ref().unwrap().name(), "insertFunction");
    }
}

#[test]
fn keytips_open_the_categorized_library_and_autosum_group_without_typing_into_cells() {
    let mut h = harness("Home", Language::En);
    key(&mut h, Key::F10);
    for k in [Key::M, Key::Q] {
        letter(&mut h, k);
    }
    assert_eq!(h.state().ui.ribbon_tab, "Formulas");
    click(&mut h, "Engineering");
    click(&mut h, "BIN2DEC");
    assert_eq!(h.state().editor.as_ref().unwrap().text, "=BIN2DEC(");

    let mut h = harness("Home", Language::En);
    key(&mut h, Key::F10);
    for k in [Key::M, Key::U] {
        letter(&mut h, k);
    }
    h.get_by_label("Average");
    assert!(h.state().editor.is_none());
    letter(&mut h, Key::S);
    let sh = h.state().session.doc().unwrap().wb.active().unwrap();
    assert_eq!(sh.input_text(CellRef::parse("A4").unwrap()), "=SUM(A1:A3)");

    let mut h = harness("Formulas", Language::En);
    key(&mut h, Key::F10);
    for k in [Key::H, Key::U] {
        letter(&mut h, k);
    }
    click(&mut h, "Average");
    assert_eq!(h.state().session.doc().unwrap().wb.active().unwrap().input_text(CellRef::parse("A4").unwrap()), "=AVERAGE(A1:A3)");
}
