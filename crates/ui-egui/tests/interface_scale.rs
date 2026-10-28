//! Interface size preference (#230): a persistent zoom for the whole window, clamped.

use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{SheetApp, UiState};
use serde_json::json;

#[test]
fn floating_customization_opens_from_home_and_pinning_previews_the_home_ribbon() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.state_mut().run("ui.backstage", json!({"page":"home"})).unwrap();
    h.run_steps(3);
    h.get_by_label("Pop up over sheet (adjust while viewing live changes)").click();
    h.run_steps(3);
    assert!(h.state().backstage.is_none());
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("displayCustomization"));
    h.state_mut().ui.ribbon_tab = "View".into();
    h.state_mut().ui.ribbon_collapsed = true;
    h.run_steps(3);
    h.get_all_by_label("Pin first").last().unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().ui.ribbon_tab, "Home");
    assert!(!h.state().ui.ribbon_collapsed);
    assert_eq!(h.state().ui.pinned_ribbon_groups.first().map(String::as_str), Some("editing"));
    assert!(h.state().grid.rect.is_some());
    h.get_by_label("Close").click();
    h.run_steps(2);
    assert!(h.state().dialog.is_none());
}

#[test]
fn ribbon_preferences_are_lenient_and_restore_the_standard_view() {
    use gridcraft_ui_egui::RibbonMode;
    let mut ui: UiState = serde_json::from_value(
        json!({"dark": true, "ribbonMode": "unknown", "hiddenRibbonGroups": ["font", 4, "font", "unknown"], "pinnedRibbonGroups": false}),
    )
    .unwrap();
    assert!(ui.dark);
    assert_eq!(ui.ribbon_mode, RibbonMode::Standard);
    assert_eq!(ui.hidden_ribbon_groups, ["font"]);
    assert!(ui.is_customized_view());
    ui.ribbon_mode = RibbonMode::ContextMenuOnly;
    ui.ribbon_collapsed = true;
    ui.pinned_ribbon_groups.push("font".into());
    ui.interface_scale = 200;
    let restored: UiState = serde_json::from_str(&serde_json::to_string(&ui).unwrap()).unwrap();
    assert_eq!(restored.ribbon_mode, RibbonMode::ContextMenuOnly);
    ui.reset_view_customizations();
    assert!(!ui.is_customized_view());
    assert!(ui.dark);
}

#[test]
fn customized_ribbon_modes_render_and_return_to_standard() {
    use gridcraft_ui_egui::{HOME_GROUPS, RibbonMode};
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    for mode in [RibbonMode::Standard, RibbonMode::Compact, RibbonMode::ContextMenuOnly] {
        h.state_mut().ui.ribbon_mode = mode;
        h.state_mut().ui.hidden_ribbon_groups = HOME_GROUPS.iter().map(|(id, _)| (*id).to_owned()).collect();
        h.state_mut().ui.pinned_ribbon_groups = vec!["font".into(), "editing".into()];
        h.run_steps(2);
    }
    h.state_mut().ui.reset_view_customizations();
    h.run_steps(2);
    assert!(!h.state().ui.is_customized_view());
}

#[test]
fn interface_size_applies_persists_and_clamps() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.run_steps(2);
    assert_eq!(h.ctx.zoom_factor(), 1.0);

    h.state_mut().run("view.interfaceScale", json!({"percent": 125})).unwrap();
    h.run_steps(2);
    assert_eq!(h.ctx.zoom_factor(), 1.25);
    let saved: UiState = serde_json::from_str(&serde_json::to_string(&h.state().ui).unwrap()).unwrap();
    assert_eq!(saved.interface_scale, 125);

    // Hostile and missing values: clamped or refused, never applied raw.
    assert_eq!(h.state_mut().run("view.interfaceScale", json!({"percent": 1e12})).unwrap()["percent"], 200);
    assert_eq!(h.state_mut().run("view.interfaceScale", json!({"percent": -5})).unwrap()["percent"], 75);
    assert!(h.state_mut().run("view.interfaceScale", json!({})).is_err(), "no value is an error, not a dialog");
    assert!(h.state_mut().run("view.interfaceScale", json!({"percent": "huge"})).is_err());
    h.state_mut().run("view.interfaceScale", json!({"percent": "auto"})).unwrap();
    h.run_steps(2);
    assert_eq!(h.ctx.zoom_factor(), 1.0);

    // A malformed or out-of-range ui.json keeps the other preferences.
    let ui: UiState = serde_json::from_str(r#"{"dark": true, "interfaceScale": "big"}"#).unwrap();
    assert!(ui.dark);
    assert_eq!(ui.interface_scale, 100);
    let ui: UiState = serde_json::from_str(r#"{"interfaceScale": 900}"#).unwrap();
    assert_eq!(ui.interface_scale, 200);
}
