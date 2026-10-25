//! Text menus use TextEdit's own event handling, preserving Unicode cursors and undo history.
use egui::{Response, Ui};
use std::sync::Weak;

use crate::SheetApp;

pub(crate) fn before_show(app: &mut SheetApp, ctx: &egui::Context, id: egui::Id) {
    app.apply_editor_clipboard(ctx, id);
    let key = id.with("text_menu.selection");
    ctx.data_mut(|data| data.remove::<egui::text::CCursorRange>(key));
    if ctx.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary))
        && let Some(range) = egui::TextEdit::load_state(ctx, id).and_then(|state| state.cursor.char_range()).filter(|range| !range.is_empty())
    {
        ctx.data_mut(|data| data.insert_temp(key, range));
    }
}

pub(crate) fn show(app: &mut SheetApp, ui: &mut Ui, response: &Response, char_count: usize) {
    show_scoped(app, ui, response, char_count, None);
}

pub(crate) fn show_scoped(app: &mut SheetApp, ui: &mut Ui, response: &Response, char_count: usize, scope: Option<Weak<()>>) {
    let id = response.id;
    // TextEdit moves its caret on *any* pointer press, including a right press. Keep the
    // selected text available to Cut/Copy when opening this menu.
    if ui.input(|input| input.pointer.interact_pos().is_some_and(|pos| response.rect.contains(pos)))
        && let Some(range) = ui.ctx().data_mut(|data| data.remove_temp::<egui::text::CCursorRange>(id.with("text_menu.selection")))
        && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id)
    {
        state.cursor.set_char_range(Some(range));
        state.store(ui.ctx(), id);
    }
    let mut event = None;
    let mut paste = false;
    let mut select_all = false;
    response.context_menu(|ui| {
        for (label, action) in [("Cut", egui::Event::Cut), ("Copy", egui::Event::Copy)] {
            if ui.button(app.ui.language.tr(label)).clicked() {
                event = Some(action);
            }
        }
        if ui.button(app.ui.language.tr("Paste")).clicked() {
            paste = true;
            ui.close();
        }
        if ui.button(app.ui.language.tr("Select All")).clicked() {
            select_all = true;
            ui.close();
        }
        if event.is_some() {
            ui.close();
        }
    });
    // Restore focus after popup layout/closing; an action inside the popup can otherwise
    // replace the request. Start the browser read before returning from the same gesture.
    if paste || select_all || event.is_some() {
        ui.ctx().memory_mut(|m| m.request_focus(id));
        if select_all && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(char_count))));
            state.store(ui.ctx(), id);
        }
        if paste {
            app.request_editor_clipboard(id, scope);
        }
        if let Some(event) = event {
            app.synthetic.push_back(vec![event]);
        }
        ui.ctx().request_repaint();
    }
    // Right-clicking a TextEdit must not redirect subsequent Paste to the sheet grid.
    if response.secondary_clicked() {
        ui.ctx().memory_mut(|m| m.request_focus(id));
    }
}
