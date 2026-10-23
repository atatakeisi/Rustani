//! IME composition must start a cell edit and leave the workbook untouched until commit.

use egui::{Context, Event, ImeEvent, Key, Modifiers, RawInput, Rect, pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_engine::core::{CellRef, Value};
use gridcraft_ui_egui::SheetApp;

fn frame_output(ctx: &Context, app: &mut SheetApp, events: Vec<Event>) -> egui::FullOutput {
    let mut output =
        ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))), events, ..Default::default() }, |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
        });
    output.textures_delta.clear();
    output
}

fn frame(ctx: &Context, app: &mut SheetApp, events: Vec<Event>) {
    let _ = frame_output(ctx, app, events);
}

fn blank() -> (Context, SheetApp) {
    let ctx = Context::default();
    let mut session = Session::new();
    session.new_workbook();
    let mut app = SheetApp::new(session, Default::default());
    for _ in 0..4 {
        frame(&ctx, &mut app, vec![]);
    }
    (ctx, app)
}

fn key(key: Key) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }
}

fn preedit(text: &str) -> Event {
    Event::Ime(ImeEvent::Preedit { text: text.into(), active_range_chars: None })
}

fn value(app: &SheetApp) -> Value {
    app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0))
}

fn assert_pending(app: &SheetApp, text: &str) {
    assert_eq!(app.editor.as_ref().map(|editor| editor.text.as_str()), Some(text));
    assert_eq!(value(app), Value::Empty);
    assert_eq!(app.session.active().unwrap().selection.active, CellRef::new(0, 0));
}

fn commit(ctx: &Context, app: &mut SheetApp) {
    frame(ctx, app, vec![Event::Ime(ImeEvent::Commit("日本".into()))]);
    assert_pending(app, "日本");
    frame(ctx, app, vec![key(Key::Enter)]);
    assert!(app.editor.is_none());
    assert_eq!(value(app), Value::text("日本"));
    assert_eq!(app.session.active().unwrap().selection.active, CellRef::new(1, 0));
}

#[test]
fn composition_inside_an_open_editor_waits_for_a_separate_enter() {
    let (ctx, mut app) = blank();
    frame(&ctx, &mut app, vec![key(Key::F2)]);
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    assert_pending(&app, "にほん");
    commit(&ctx, &mut app);
}

#[test]
fn composition_from_the_selected_cell_starts_editing() {
    let (ctx, mut app) = blank();
    let ready = frame_output(&ctx, &mut app, vec![]);
    assert!(ready.platform_output.ime.is_some(), "the browser must allow IME before the first character");
    let first = frame_output(&ctx, &mut app, vec![preedit("にほん")]);
    let ime = first.platform_output.ime.expect("the editor supplies the IME caret rectangle");
    assert!(!ime.should_interrupt_composition, "opening the editor must preserve the current browser composition");
    assert_pending(&app, "にほん");
    assert!(app.editor.as_ref().unwrap().enter_mode, "direct typing keeps entry-mode arrow navigation");
    commit(&ctx, &mut app);
}

#[test]
fn an_arrow_after_direct_composition_commits_and_moves_like_plain_typing() {
    let (ctx, mut app) = blank();
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    frame(&ctx, &mut app, vec![Event::Ime(ImeEvent::Commit("日本".into()))]);
    assert_pending(&app, "日本");
    assert!(app.editor.as_ref().unwrap().enter_mode);
    frame(&ctx, &mut app, vec![key(Key::ArrowDown)]);
    assert!(app.editor.is_none());
    assert_eq!(value(&app), Value::text("日本"));
    assert_eq!(app.session.active().unwrap().selection.active, CellRef::new(1, 0));
}

#[test]
fn a_commit_without_preedit_starts_editing_once() {
    let (ctx, mut app) = blank();
    commit(&ctx, &mut app);
}

#[test]
fn cancelling_composition_preserves_the_original_cell() {
    let (ctx, mut app) = blank();
    app.session.execute("cell.set", serde_json::json!({"cell": "A1", "input": "original"})).unwrap();
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    assert_eq!(app.editor.as_ref().unwrap().text, "にほん");
    frame(&ctx, &mut app, vec![preedit("")]);
    assert_eq!(app.editor.as_ref().unwrap().text, "");
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert!(app.editor.is_none());
    assert_eq!(value(&app), Value::text("original"));
}

#[test]
fn direct_composition_replaces_existing_text_without_duplicate_preedit() {
    let (ctx, mut app) = blank();
    app.session.execute("cell.set", serde_json::json!({"cell": "A1", "input": "original"})).unwrap();
    frame(&ctx, &mut app, vec![preedit("に"), preedit("にほん")]);
    assert_eq!(app.editor.as_ref().unwrap().text, "にほん");
    frame(&ctx, &mut app, vec![Event::Ime(ImeEvent::Commit("日本".into()))]);
    assert_eq!(app.editor.as_ref().unwrap().text, "日本");
    assert_eq!(value(&app), Value::text("original"));
    frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert_eq!(value(&app), Value::text("日本"));
}

#[test]
fn empty_composition_notifications_do_not_start_an_edit() {
    let (ctx, mut app) = blank();
    for event in [preedit(""), Event::Ime(ImeEvent::Commit("".into())), Event::Ime(ImeEvent::Commit("\n".into()))] {
        frame(&ctx, &mut app, vec![event]);
        assert!(app.editor.is_none());
        assert_eq!(value(&app), Value::Empty);
    }
}

#[test]
fn consecutive_cells_do_not_reuse_the_previous_composition_selection() {
    let (ctx, mut app) = blank();
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    commit(&ctx, &mut app);
    frame(&ctx, &mut app, vec![preedit("とうきょう"), Event::Ime(ImeEvent::Commit("東京".into()))]);
    assert_eq!(app.editor.as_ref().unwrap().text, "東京");
    frame(&ctx, &mut app, vec![key(Key::Enter)]);
    let doc = app.session.active().unwrap();
    assert_eq!(doc.wb.active().unwrap().value(CellRef::new(0, 0)), Value::text("日本"));
    assert_eq!(doc.wb.active().unwrap().value(CellRef::new(1, 0)), Value::text("東京"));
    assert_eq!(doc.selection.active, CellRef::new(2, 0));
}

#[test]
fn escape_during_preedit_allows_a_fresh_composition() {
    let (ctx, mut app) = blank();
    frame(&ctx, &mut app, vec![preedit("とりけし")]);
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert!(app.editor.is_none());
    assert_eq!(value(&app), Value::Empty);
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    commit(&ctx, &mut app);
}

#[test]
fn text_events_batched_before_a_repaint_are_not_lost_or_reordered() {
    for text in ["日本語テスト", "fast typing"] {
        let (ctx, mut app) = blank();
        frame(&ctx, &mut app, text.chars().map(|c| Event::Text(c.to_string())).collect());
        assert_pending(&app, text);
        frame(&ctx, &mut app, vec![Event::Text("!".into())]);
        assert_pending(&app, &format!("{text}!"));
        frame(&ctx, &mut app, vec![key(Key::Enter)]);
        assert_eq!(value(&app), Value::text(format!("{text}!")));
    }
}
