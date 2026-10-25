//! Browser clipboard bridge: begin permission-sensitive reads in the original gesture.
use gridcraft_ui_egui::{ClipboardData, ClipboardInbox, ImagePasteInbox, ImagePasteTargetState};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;

const MAX_BYTES: usize = 64 * 1024 * 1024;

fn error(e: JsValue) -> String {
    format!("Clipboard access failed: {e:?}")
}

fn property(object: &JsValue, name: &str) -> Result<JsValue, String> {
    js_sys::Reflect::get(object, &name.into()).map_err(error)
}

// These web-sys clipboard bindings do not have `catch`: a synchronous JavaScript exception
// would escape before JsFuture could observe a rejected Promise. Reflect and Function::apply
// have catch-enabled bindings. Keep the original receiver and invoke before yielding the
// user's gesture, then handle both synchronous exceptions and asynchronous rejections.
fn call_promise(object: &JsValue, method: &str, argument: Option<&JsValue>) -> Result<js_sys::Promise, String> {
    let function: js_sys::Function =
        property(object, method)?.dyn_into().map_err(|_| format!("This browser does not support clipboard {method}."))?;
    let arguments = js_sys::Array::new();
    if let Some(argument) = argument {
        arguments.push(argument);
    }
    function.apply(object, &arguments).map_err(error)?.dyn_into().map_err(|_| format!("Clipboard {method} did not return a Promise."))
}

fn begin_read() -> Result<(js_sys::Promise, bool), String> {
    let window = web_sys::window().ok_or("Browser window is unavailable.")?;
    let navigator = property(window.as_ref(), "navigator")?;
    let clipboard = property(&navigator, "clipboard")?;
    if clipboard.is_null() || clipboard.is_undefined() {
        return Err("Browser clipboard is unavailable in this context.".into());
    }
    let rich = property(&clipboard, "read")?.is_function();
    let promise = call_promise(&clipboard, if rich { "read" } else { "readText" }, None)?;
    Ok((promise, rich))
}

async fn blob_bytes(blob: web_sys::Blob) -> Result<Vec<u8>, String> {
    let size = property(blob.as_ref(), "size")?
        .as_f64()
        .filter(|size| size.is_finite() && *size >= 0.0)
        .ok_or("Clipboard returned an invalid Blob size.")?;
    if size > MAX_BYTES as f64 {
        return Err("Clipboard content is larger than 64 MB.".into());
    }
    let promise = call_promise(blob.as_ref(), "arrayBuffer", None)?;
    let buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.map_err(error)?.dyn_into().map_err(error)?;
    let length = property(buffer.as_ref(), "byteLength")?
        .as_f64()
        .filter(|length| length.is_finite() && *length >= 0.0)
        .ok_or("Clipboard returned an invalid ArrayBuffer length.")?;
    if length > MAX_BYTES as f64 {
        return Err("Clipboard content is larger than 64 MB.".into());
    }
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

async fn item_blob(item: &JsValue, mime: &str) -> Result<web_sys::Blob, String> {
    let promise = call_promise(item, "getType", Some(&mime.into()))?;
    JsFuture::from(promise).await.map_err(error)?.dyn_into().map_err(error)
}

pub(super) fn read(inbox: ClipboardInbox, ctx: egui::Context) {
    let (promise, rich) = match begin_read() {
        Ok(read) => read,
        Err(error) => {
            *inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Err(error));
            ctx.request_repaint();
            return;
        }
    };
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let value = JsFuture::from(promise).await.map_err(error)?;
            if !rich {
                let text = value.as_string().ok_or("Clipboard readText did not return text.")?;
                if text.len() > MAX_BYTES {
                    return Err("Clipboard content is larger than 64 MB.".into());
                }
                return Ok(ClipboardData::Text { text, html: None });
            }
            let items: js_sys::Array = value.dyn_into().map_err(error)?;
            let mut text = None;
            let mut html = None;
            // A single image wins over its text/html alternative, as screenshot paste does in Excel.
            for value in items.iter().take(32) {
                let item = value;
                let types: js_sys::Array = property(&item, "types")?.dyn_into().map_err(error)?;
                let has_type = |mime: &str| types.iter().take(32).any(|value| value.as_string().as_deref() == Some(mime));
                for mime in ["image/png", "image/jpeg"] {
                    if has_type(mime) {
                        return Ok(ClipboardData::Image(blob_bytes(item_blob(&item, mime).await?).await?));
                    }
                }
                for (mime, destination) in [("text/plain", &mut text), ("text/html", &mut html)] {
                    if has_type(mime) {
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
