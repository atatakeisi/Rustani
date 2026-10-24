//! Text menus use TextEdit's own event handling, preserving Unicode cursors and undo history.
use egui::{Response, Ui};

use crate::SheetApp;

pub(crate) fn before_show(ctx: &egui::Context, id: egui::Id) {
    let key = id.with("text_menu.selection");
    ctx.data_mut(|data| data.remove::<egui::text::CCursorRange>(key));
    if ctx.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary))
        && let Some(range) = egui::TextEdit::load_state(ctx, id).and_then(|state| state.cursor.char_range()).filter(|range| !range.is_empty())
    {
        ctx.data_mut(|data| data.insert_temp(key, range));
    }
}

pub(crate) fn show(app: &mut SheetApp, ui: &mut Ui, response: &Response, char_count: usize) {
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
    response.context_menu(|ui| {
        let mut event = None;
        for (label, action) in [("Cut", egui::Event::Cut), ("Copy", egui::Event::Copy)] {
            if ui.button(app.ui.language.tr(label)).clicked() {
                event = Some(action);
            }
        }
        if ui.button(app.ui.language.tr("Paste")).clicked() {
            ui.ctx().memory_mut(|m| m.request_focus(id));
            if app.services.paste_async.is_some() {
                app.request_clipboard(Some(id));
            } else if let Some(text) = app.services.paste_text.as_mut().and_then(|read| read()) {
                event = Some(egui::Event::Paste(text));
            }
            ui.close();
        }
        if ui.button(app.ui.language.tr("Select All")).clicked() {
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(char_count))));
                state.store(ui.ctx(), id);
            }
            ui.ctx().memory_mut(|m| m.request_focus(id));
            ui.close();
        }
        if let Some(event) = event {
            ui.ctx().memory_mut(|m| m.request_focus(id));
            app.synthetic.push_back(vec![event]);
            ui.ctx().request_repaint();
            ui.close();
        }
    });
    // Right-clicking a TextEdit must not redirect subsequent Paste to the sheet grid.
    if response.secondary_clicked() {
        ui.ctx().memory_mut(|m| m.request_focus(id));
    }
}
