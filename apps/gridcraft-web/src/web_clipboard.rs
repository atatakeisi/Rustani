//! Browser clipboard bridge: begin permission-sensitive reads in the original gesture.
use gridcraft_ui_egui::{ClipboardData, ClipboardInbox, ImagePasteInbox, ImagePasteTargetState};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;

const MAX_BYTES: usize = 64 * 1024 * 1024;

fn error(e: JsValue) -> String {
    format!("Clipboard access failed: {e:?}")
}

async fn blob_bytes(blob: web_sys::Blob) -> Result<Vec<u8>, String> {
    if blob.size() > MAX_BYTES as f64 {
        return Err("Clipboard content is larger than 64 MB.".into());
    }
    let buffer = JsFuture::from(blob.array_buffer()).await.map_err(error)?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

async fn item_blob(item: &web_sys::ClipboardItem, mime: &str) -> Result<web_sys::Blob, String> {
    JsFuture::from(item.get_type(mime)).await.map_err(error)?.dyn_into().map_err(error)
}

pub(super) fn read(inbox: ClipboardInbox, ctx: egui::Context) {
    let Some(window) = web_sys::window() else { return };
    let clipboard = window.navigator().clipboard();
    if clipboard.is_undefined() || clipboard.is_null() {
        *inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Err("Browser clipboard is unavailable in this context.".into()));
        ctx.request_repaint();
        return;
    }
    let rich = js_sys::Reflect::get(&clipboard, &"read".into()).ok().is_some_and(|v| v.is_function());
    if !rich && !js_sys::Reflect::get(&clipboard, &"readText".into()).ok().is_some_and(|v| v.is_function()) {
        *inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Err("This browser does not support clipboard reads.".into()));
        ctx.request_repaint();
        return;
    }
    let promise = if rich { clipboard.read() } else { clipboard.read_text() };
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let value = JsFuture::from(promise).await.map_err(error)?;
            if !rich {
                let text = value.as_string().unwrap_or_default();
                if text.len() > MAX_BYTES {
                    return Err("Clipboard content is larger than 64 MB.".into());
                }
                return Ok(ClipboardData::Text { text, html: None });
            }
            let items = js_sys::Array::from(&value);
            let mut text = None;
            let mut html = None;
            // A single image wins over its text/html alternative, as screenshot paste does in Excel.
            for value in items.iter().take(32) {
                let item: web_sys::ClipboardItem = value.dyn_into().map_err(error)?;
                let types = item.types();
                for mime in ["image/png", "image/jpeg"] {
                    if types.includes(&mime.into(), 0) {
                        return Ok(ClipboardData::Image(blob_bytes(item_blob(&item, mime).await?).await?));
                    }
                }
                for (mime, destination) in [("text/plain", &mut text), ("text/html", &mut html)] {
                    if types.includes(&mime.into(), 0) {
                        let bytes = blob_bytes(item_blob(&item, mime).await?).await?;
                        *destination = Some(String::from_utf8(bytes).map_err(|e| e.to_string())?);
                    }
                }
            }
            Ok(match text {
                Some(text) => ClipboardData::Text { text: text.replace("\r\n", "\n"), html },
                None => ClipboardData::Empty,
            })
        }
        .await;
        *inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(result);
        ctx.request_repaint();
    });
}

pub(super) struct Listener {
    document: web_sys::Document,
    callback: Closure<dyn FnMut(web_sys::ClipboardEvent)>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.document.remove_event_listener_with_callback_and_bool("paste", self.callback.as_ref().unchecked_ref(), true);
    }
}

pub(super) fn listen(inbox: ImagePasteInbox, target: ImagePasteTargetState, ctx: egui::Context) -> Result<Listener, JsValue> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or_else(|| JsValue::from_str("no document"))?;
    let callback = Closure::wrap(Box::new(move |event: web_sys::ClipboardEvent| {
        // When a text field or dialog owns input, eframe must still receive the event's text
        // flavour. Capture the destination before beginning any asynchronous Blob reads.
        let Some(target) = target.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone() else { return };
        let Some(data) = event.clipboard_data() else { return };
        let items = data.items();
        for i in 0..items.length().min(32) {
            let Some(item) = items.get(i) else { continue };
            if !matches!(item.type_().as_str(), "image/png" | "image/jpeg") {
                continue;
            }
            let Ok(Some(file)) = item.get_as_file() else { continue };
            event.prevent_default();
            // Capture before eframe's text-only handler to avoid pasting both image and alt text.
            event.stop_immediate_propagation();
            let inbox = inbox.clone();
            let ctx = ctx.clone();
            let target = target.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match blob_bytes(file.into()).await {
                    Ok(bytes) => {
                        let mut queue = inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                        if queue.len() < 4 {
                            queue.push((target, bytes));
                        }
                    }
                    Err(error) => log::warn!("{error}"),
                }
                ctx.request_repaint();
            });
            break;
        }
    }) as Box<dyn FnMut(web_sys::ClipboardEvent)>);
    document.add_event_listener_with_callback_and_bool("paste", callback.as_ref().unchecked_ref(), true)?;
    Ok(Listener { document, callback })
}
