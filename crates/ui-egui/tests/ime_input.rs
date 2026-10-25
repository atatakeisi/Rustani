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

#[test]
fn mac_control_enter_and_command_enter_commit_all_selected_cells() {
    for modifiers in [Modifiers::CTRL, Modifiers { mac_cmd: true, command: true, ..Modifiers::NONE }] {
        let (ctx, mut app) = blank();
        ctx.set_os(egui::os::OperatingSystem::Mac);
        app.run("selection.set", serde_json::json!({"range":"A1:A2,C3"})).unwrap();
        frame(&ctx, &mut app, vec![Event::Text("日本".into())]);
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers }]);
        assert!(app.editor.is_none(), "{modifiers:?}");
        let sh = app.session.active().unwrap().wb.active().unwrap();
        for at in [CellRef::new(0, 0), CellRef::new(1, 0), CellRef::new(2, 2)] {
            assert_eq!(sh.value(at), Value::text("日本"), "{modifiers:?}: {at:?}");
        }
        app.run("edit.undo", serde_json::json!({})).unwrap();
        assert_eq!(value(&app), Value::Empty);
    }
}

#[test]
fn f2_opening_frame_retains_an_ime_target() {
    let (ctx, mut app) = blank();
    let output = frame_output(&ctx, &mut app, vec![key(Key::F2)]);
    assert!(output.platform_output.ime.is_some());
    let output = frame_output(&ctx, &mut app, vec![]);
    assert!(output.platform_output.ime.is_some());
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new("gridcraft.cell_editor"))));
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

fn boundary_events(ime: Event, boundary: Key, key_first: bool) -> Vec<Event> {
    if key_first { vec![key(boundary), ime] } else { vec![ime, key(boundary)] }
}

#[test]
fn direct_composition_boundary_keys_do_not_finish_or_cancel_the_cell() {
    for boundary in [Key::Enter, Key::Tab, Key::Escape] {
        for key_first in [false, true] {
            for commit_first in [false, true] {
                let (ctx, mut app) = blank();
                let ime = if commit_first { Event::Ime(ImeEvent::Commit("日本".into())) } else { preedit("にほん") };
                frame(&ctx, &mut app, boundary_events(ime, boundary, key_first));
                assert_pending(&app, if commit_first { "日本" } else { "にほん" });
                if !commit_first {
                    frame(&ctx, &mut app, vec![Event::Ime(ImeEvent::Commit("日本".into()))]);
                }
                assert_pending(&app, "日本");
                frame(&ctx, &mut app, vec![key(Key::Enter)]);
                assert!(app.editor.is_none());
                assert_eq!(value(&app), Value::text("日本"));
                assert_eq!(app.session.active().unwrap().selection.active, CellRef::new(1, 0));
            }
        }
    }
}

#[test]
fn active_cell_and_formula_bar_compositions_keep_boundary_keys_inside_the_ime() {
    for formula_bar in [false, true] {
        for boundary in [Key::Enter, Key::Tab, Key::Escape] {
            for key_first in [false, true] {
                let (ctx, mut app) = blank();
                app.begin_edit(None, formula_bar);
                frame(&ctx, &mut app, vec![]);
                frame(&ctx, &mut app, vec![]);
                frame(&ctx, &mut app, vec![preedit("にほん")]);
                frame(&ctx, &mut app, boundary_events(Event::Ime(ImeEvent::Commit("日本".into())), boundary, key_first));
                assert_pending(&app, "日本");
                assert_eq!(app.editor.as_ref().unwrap().from_formula_bar, formula_bar);
                frame(&ctx, &mut app, vec![key(Key::Enter)]);
                assert!(app.editor.is_none());
                assert_eq!(value(&app), Value::text("日本"));
                assert_eq!(app.session.active().unwrap().selection.active, CellRef::new(1, 0));
            }
        }
    }
}

#[test]
fn boundary_keys_in_the_frame_after_direct_ime_start_keep_editor_focus() {
    for boundary in [Key::Enter, Key::Tab, Key::Escape] {
        for key_first in [false, true] {
            let (ctx, mut app) = blank();
            frame(&ctx, &mut app, vec![preedit("にほん")]);
            let output = frame_output(&ctx, &mut app, boundary_events(Event::Ime(ImeEvent::Commit("日本".into())), boundary, key_first));
            assert_pending(&app, "日本");
            assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("gridcraft.cell_editor"))));
            assert!(!output.platform_output.ime.expect("the editor must still own the IME").should_interrupt_composition);
            frame(&ctx, &mut app, vec![key(Key::Enter)]);
            assert!(app.editor.is_none());
            assert_eq!(value(&app), Value::text("日本"));
        }
    }
}

#[test]
fn cancelling_the_ime_candidate_does_not_cancel_the_cell_in_the_same_frame() {
    for key_first in [false, true] {
        let (ctx, mut app) = blank();
        app.session.execute("cell.set", serde_json::json!({"cell": "A1", "input": "original"})).unwrap();
        frame(&ctx, &mut app, vec![preedit("とりけし")]);
        frame(&ctx, &mut app, boundary_events(preedit(""), Key::Escape, key_first));
        assert_eq!(app.editor.as_ref().unwrap().text, "");
        assert_eq!(value(&app), Value::text("original"));
        frame(&ctx, &mut app, vec![key(Key::Escape)]);
        assert!(app.editor.is_none());
        assert_eq!(value(&app), Value::text("original"));
    }
}

#[test]
fn modified_enter_does_not_apply_a_cell_action_when_batched_with_ime_commit() {
    for modifiers in [Modifiers::SHIFT, Modifiers::CTRL, Modifiers::COMMAND, Modifiers::ALT, Modifiers::CTRL | Modifiers::SHIFT] {
        let (ctx, mut app) = blank();
        frame(&ctx, &mut app, vec![preedit("にほん")]);
        let enter = Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers };
        frame(&ctx, &mut app, vec![Event::Ime(ImeEvent::Commit("日本".into())), enter]);
        assert_pending(&app, "日本");
        assert!(app.message.is_none());
    }
}

#[test]
fn locked_cells_reject_direct_ime_and_plain_text_before_an_editor_opens() {
    for event in [preedit("にほん"), Event::Ime(ImeEvent::Commit("日本".into())), Event::Text("x".into()), key(Key::F2)] {
        let (ctx, mut app) = blank();
        app.session.execute("cell.set", serde_json::json!({"cell": "A1", "input": "original"})).unwrap();
        app.session.execute("review.protectSheet", serde_json::json!({})).unwrap();
        let before = app.session.active().unwrap().wb.clone();
        frame(&ctx, &mut app, vec![event]);
        assert!(app.editor.is_none(), "a locked cell never owns an inline editor");
        assert!(app.message.as_ref().is_some_and(|(_, message)| message.contains("protected sheet")));
        assert_eq!(app.session.active().unwrap().wb, before);
        assert_eq!(value(&app), Value::text("original"));
        assert_eq!(app.session.active().unwrap().selection.active, CellRef::new(0, 0));
    }
}

#[test]
fn a_refused_direct_composition_tells_the_browser_to_interrupt_it() {
    let (ctx, mut app) = blank();
    app.session.execute("review.protectSheet", serde_json::json!({})).unwrap();
    let output = frame_output(&ctx, &mut app, vec![preedit("にほん")]);
    assert!(app.editor.is_none());
    assert!(output.platform_output.ime.expect("the grid supplies its IME target").should_interrupt_composition);
}

#[test]
fn protected_sheets_still_allow_ime_in_unlocked_cells_and_allowed_ranges() {
    for allow_range in [false, true] {
        let (ctx, mut app) = blank();
        if allow_range {
            app.session.execute("review.allowEditRanges", serde_json::json!({"action": "add", "title": "Input", "range": "A1"})).unwrap();
        } else {
            app.session.execute("home.formatCells", serde_json::json!({"range": "A1", "style": {"protection": {"locked": false}}})).unwrap();
        }
        app.session.execute("review.protectSheet", serde_json::json!({})).unwrap();
        frame(&ctx, &mut app, vec![preedit("にほん")]);
        assert_pending(&app, "にほん");
        commit(&ctx, &mut app);
        assert!(app.message.is_none());
    }
}

#[test]
fn the_ime_boundary_guard_does_not_take_keys_from_another_focused_widget() {
    let (ctx, mut app) = blank();
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("another_text_field")));
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
            events: boundary_events(Event::Ime(ImeEvent::Commit("日本".into())), Key::Tab, false),
            ..Default::default()
        },
        |ui| assert!(ui.input(|input| input.key_pressed(Key::Tab)), "an unrelated field owns its keyboard input"),
    );
    output.textures_delta.clear();
}

#[test]
fn opening_a_message_suspends_the_editors_ime_boundary_guard() {
    let (ctx, mut app) = blank();
    frame(&ctx, &mut app, vec![preedit("にほん")]);
    app.message = Some(("Rustani".into(), "A modal message owns Enter.".into()));
    frame(&ctx, &mut app, vec![]);
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
            events: boundary_events(preedit(""), Key::Enter, false),
            ..Default::default()
        },
        |ui| assert!(ui.input(|input| input.key_pressed(Key::Enter)), "the modal must still receive Enter"),
    );
    output.textures_delta.clear();
}
