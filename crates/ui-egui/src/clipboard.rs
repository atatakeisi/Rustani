//! Async clipboard requests retain their intended editor and reject stale destinations.
use std::sync::{Arc, Mutex};

use egui::{Context, Id};
use gridcraft_engine::Selection;
use serde_json::json;

use crate::SheetApp;

pub enum ClipboardData {
    Text { text: String, html: Option<String> },
    Image(Vec<u8>),
    Empty,
}

pub type ClipboardInbox = Arc<Mutex<Option<Result<ClipboardData, String>>>>;

/// The worksheet destination captured during a browser's original paste gesture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImagePasteTarget {
    pub uid: u64,
    pub sheet: usize,
    pub selection: Selection,
}

pub type ImagePasteInbox = Arc<Mutex<Vec<(ImagePasteTarget, Vec<u8>)>>>;
/// The browser shell publishes this after UI and host commands have finished each frame.
pub type ImagePasteTargetState = Arc<Mutex<Option<ImagePasteTarget>>>;

pub(crate) struct Pending {
    inbox: ClipboardInbox,
    uid: u64,
    sheet: usize,
    selection: Selection,
    editor: Option<Id>,
}

impl SheetApp {
    /// `None` leaves the browser's ordinary text paste handler in control. Text fields in
    /// side panes, the name box and object editors count as editors too.
    pub fn image_paste_target(&self, ctx: &Context) -> Option<ImagePasteTarget> {
        if self.editor.is_some()
            || self.text_box_editor.is_some()
            || self.dialog.is_some()
            || self.message.is_some()
            || self.backstage.is_some()
            || self.name_box.is_some()
            || self.grid.hf_edit.is_some()
            || ctx.text_edit_focused()
        {
            return None;
        }
        self.session.active().map(|doc| ImagePasteTarget { uid: doc.uid, sheet: doc.wb.active_sheet, selection: doc.selection.clone() })
    }

    pub(crate) fn request_clipboard(&mut self, editor: Option<Id>) {
        let Some(read) = &self.services.paste_async else { return };
        let Some(d) = self.session.active() else { return };
        let inbox = ClipboardInbox::default();
        let pending = Pending { inbox: inbox.clone(), uid: d.uid, sheet: d.wb.active_sheet, selection: d.selection.clone(), editor };
        // Invoke before returning from the gesture; browsers can otherwise revoke activation.
        read(inbox);
        self.clipboard_pending = Some(pending);
    }

    pub fn try_paste_picture(&mut self) -> bool {
        if self.editor.is_some() || self.dialog.is_some() {
            return false;
        }
        let Some(bytes) = self.services.paste_image.as_mut().and_then(|read| read()) else { return false };
        self.paste_picture(bytes);
        true
    }

    pub(crate) fn paste_picture(&mut self, bytes: Vec<u8>) {
        if bytes.len() > 64 * 1024 * 1024 {
            self.alert("Clipboard image is larger than 64 MB.".into());
            return;
        }
        self.run_or_alert("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&bytes), "placement": "overCells"}));
    }

    pub(crate) fn poll_clipboard(&mut self, ctx: &Context) {
        let arrived = self.clipboard_pending.as_ref().and_then(|p| p.inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take());
        if let Some(result) = arrived
            && let Some(p) = self.clipboard_pending.take()
        {
            let current = self.session.active().is_some_and(|d| d.uid == p.uid && d.wb.active_sheet == p.sheet && d.selection == p.selection);
            let focused = p.editor.map_or_else(|| self.image_paste_target(ctx).is_some(), |id| ctx.memory(|m| m.has_focus(id)));
            if current && focused {
                match (p.editor, result) {
                    (Some(id), Ok(ClipboardData::Text { text, .. })) => {
                        ctx.memory_mut(|m| m.request_focus(id));
                        self.synthetic.push_back(vec![egui::Event::Paste(text)]);
                    }
                    (None, Ok(ClipboardData::Text { text, html })) if !text.is_empty() => {
                        if Self::same_clipboard_text(Some(&text), self.clipboard_text_at_copy.as_deref()) && self.session.clipboard.is_some() {
                            self.run_or_alert("edit.paste", json!({}));
                        } else if let Some(html) = html {
                            self.run_or_alert("edit.paste", json!({"text":text, "html":html}));
                        } else {
                            self.paste_external(text);
                        }
                    }
                    (None, Ok(ClipboardData::Image(bytes))) if self.image_paste_target(ctx).is_some() => self.paste_picture(bytes),
                    (None, Ok(ClipboardData::Empty) | Err(_)) if self.session.clipboard.is_some() => self.run_or_alert("edit.paste", json!({})),
                    (_, Err(error)) => self.alert(error),
                    _ => {}
                }
                ctx.request_repaint();
            }
        }
        let images = self
            .services
            .clipboard_images
            .as_ref()
            .map(|inbox| std::mem::take(&mut *inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner)))
            .unwrap_or_default();
        for (target, bytes) in images {
            if self.image_paste_target(ctx).as_ref() == Some(&target) {
                self.paste_picture(bytes);
            }
        }
    }
}
