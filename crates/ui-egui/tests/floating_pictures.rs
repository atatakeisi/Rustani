//! Floating pictures on the sheet, with original synthetic pictures.

use std::io::Cursor;

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape, TextureId, vec2};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{Services, SheetApp, grid::Geo};
use serde_json::json;

fn png(color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(120, 60, image::Rgba(color));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

fn harness() -> egui_kittest::Harness<'static, SheetApp> {
    harness_with_services(Default::default())
}

fn harness_with_services(services: Services) -> egui_kittest::Harness<'static, SheetApp> {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        SheetApp::new(session, services),
    );
    h.run_steps(4);
    h
}

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn logical_rect(h: &Harness) -> Rect {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    let image = sh.images.first().unwrap();
    let a = image.anchor;
    let geo = Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
    Rect::from_min_size(egui::pos2(geo.x(sh, a.cell.col) + a.dx * geo.z, geo.y(sh, a.cell.row) + a.dy * geo.z), vec2(a.width, a.height) * geo.z)
}

fn physical_pos(h: &Harness, pos: Pos2) -> Pos2 {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll).mirror_pos(pos)
}

fn button(h: &mut Harness, pos: Pos2, button: PointerButton, pressed: bool) {
    h.input_mut().events.extend([Event::PointerMoved(pos), Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE }]);
    h.run_steps(2);
}

#[test]
fn every_picture_handle_resizes_from_its_outer_half_and_keeps_the_opposite_edge_fixed() {
    for rtl in [false, true] {
        for (x, y) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            let mut h = harness();
            h.state_mut().run("view.rightToLeft", json!({"on": rtl})).unwrap();
            let result = h
                .state_mut()
                .run("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&png([24, 150, 110, 255])), "at": "D6"}))
                .unwrap();
            h.run_steps(3);
            assert_eq!(h.state().selected_chart, result["image"].as_u64().map(|id| id as u32));
            let before = logical_rect(&h);
            let center = before.center();
            // Three pixels beyond the frame prove the expanded hit area works at every edge.
            let start = egui::pos2(
                if x < 0 {
                    before.left() - 3.0
                } else if x > 0 {
                    before.right() + 3.0
                } else {
                    center.x
                },
                if y < 0 {
                    before.top() - 3.0
                } else if y > 0 {
                    before.bottom() + 3.0
                } else {
                    center.y
                },
            );
            let start = physical_pos(&h, start);
            h.input_mut().events.push(Event::PointerMoved(start));
            h.run_steps(2);
            let physical_x = if rtl { -x } else { x };
            let cursor = if x == 0 {
                egui::CursorIcon::ResizeVertical
            } else if y == 0 {
                egui::CursorIcon::ResizeHorizontal
            } else if physical_x == y {
                egui::CursorIcon::ResizeNwSe
            } else {
                egui::CursorIcon::ResizeNeSw
            };
            assert_eq!(h.output().platform_output.cursor_icon, cursor, "rtl={rtl}, handle=({x},{y})");
            let undo = h.state().session.doc().unwrap().undo.len();
            button(&mut h, start, PointerButton::Primary, true);
            let end = start + vec2(physical_x as f32 * 28.0, y as f32 * 24.0);
            h.input_mut().events.push(Event::PointerMoved(end));
            h.run_steps(3);
            assert!(
                matches!(h.state().grid.drag, gridcraft_ui_egui::grid::Drag::Object { resize: Some(_), .. }),
                "rtl={rtl}, handle=({x},{y}), drag={:?}",
                h.state().grid.drag
            );
            button(&mut h, end, PointerButton::Primary, false);
            let after = logical_rect(&h);
            let close = |a: f32, b: f32| assert!((a - b).abs() < 0.1, "{a} != {b}, rtl={rtl}, handle=({x},{y})");
            if x < 0 {
                close(after.right(), before.right());
                assert!(after.left() < before.left() - 20.0, "rtl={rtl}, handle=({x},{y}), {before:?} -> {after:?}");
            } else if x > 0 {
                close(after.left(), before.left());
                assert!(after.right() > before.right() + 20.0, "rtl={rtl}, handle=({x},{y}), {before:?} -> {after:?}");
            } else {
                close(after.left(), before.left());
                close(after.right(), before.right());
            }
            if y < 0 {
                close(after.bottom(), before.bottom());
                assert!(after.top() < before.top() - 18.0, "rtl={rtl}, handle=({x},{y}), {before:?} -> {after:?}");
            } else if y > 0 {
                close(after.top(), before.top());
                assert!(after.bottom() > before.bottom() + 18.0, "rtl={rtl}, handle=({x},{y}), {before:?} -> {after:?}");
            } else {
                close(after.top(), before.top());
                close(after.bottom(), before.bottom());
            }
            assert_eq!(h.state().session.doc().unwrap().undo.len(), undo + 1, "one drag is one undo");
            h.state_mut().run("edit.undo", json!({})).unwrap();
            h.run_steps(2);
            assert_eq!(logical_rect(&h), before);
        }
    }
}

#[test]
fn picture_context_menus_convert_both_ways_and_select_the_result() {
    let mut h = harness();
    let bytes = png([24, 150, 110, 255]);
    h.state_mut().run("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&bytes), "at": "D6"})).unwrap();
    h.run_steps(2);
    let at = physical_pos(&h, logical_rect(&h).center());
    button(&mut h, at, PointerButton::Secondary, true);
    button(&mut h, at, PointerButton::Secondary, false);
    h.get_by_label("Place in Cell").click();
    h.run_steps(3);
    let doc = h.state().session.doc().unwrap();
    assert_eq!(doc.selection.active.a1(), "D6");
    assert!(doc.wb.active().unwrap().images.is_empty());
    assert_eq!(doc.wb.active().unwrap().cell_pictures[&doc.selection.active].data, bytes);
    assert_eq!(h.state().selected_chart, None);
    let cell = {
        let app = h.state();
        let sh = doc.wb.active().unwrap();
        Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll).cell_rect(sh, doc.selection.active).center()
    };
    button(&mut h, cell, PointerButton::Secondary, true);
    button(&mut h, cell, PointerButton::Secondary, false);
    h.get_by_label("Place over Cells").click();
    h.run_steps(3);
    let doc = h.state().session.doc().unwrap();
    let sh = doc.wb.active().unwrap();
    assert!(sh.cell_pictures.is_empty());
    assert_eq!(*sh.images[0].data, bytes);
    assert_eq!(h.state().selected_chart, Some(sh.images[0].id));
}

#[test]
fn screenshot_clipboard_paste_inserts_one_selected_picture_with_or_without_a_text_event() {
    for order in [0, 1, 2] {
        let bytes = png([24, 150, 110, 255]);
        let clipboard = bytes.clone();
        let mut h = harness_with_services(Services { paste_image: Some(Box::new(move || Some(clipboard.clone()))), ..Default::default() });
        h.state_mut().run("selection.set", json!({"cell": "D6"})).unwrap();
        let undo = h.state().session.doc().unwrap().undo.len();
        let modifiers = Modifiers::COMMAND;
        let key = Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: true, repeat: false, modifiers };
        h.input_mut().events.push(Event::ModifiersChanged(modifiers));
        match order {
            0 => h.input_mut().events.push(key),
            1 => h.input_mut().events.extend([key, Event::Paste("image description".into())]),
            _ => h.input_mut().events.extend([Event::Paste("image description".into()), key]),
        }
        h.run_steps(3);
        let doc = h.state().session.doc().unwrap();
        let sh = doc.wb.active().unwrap();
        assert_eq!(sh.images.len(), 1, "event order {order}");
        assert_eq!(*sh.images[0].data, bytes);
        assert_eq!(sh.images[0].anchor.cell.a1(), "D6");
        assert!(sh.value(sh.images[0].anchor.cell).is_empty(), "the text flavour must not also change a cell");
        assert_eq!(h.state().selected_chart, Some(sh.images[0].id));
        assert_eq!(doc.undo.len(), undo + 1);
    }
}

fn picture_textures(h: &egui_kittest::Harness<'static, SheetApp>) -> Vec<TextureId> {
    h.output()
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => Some(mesh.texture_id),
            _ => None,
        })
        .collect()
}

/// Picture ids are per workbook, so two open workbooks both have a picture 1. Each must show its
/// own picture, even when the two are the same number of bytes: the texture cache used to tell
/// pictures apart by id and size alone, for the whole app.
#[test]
fn each_workbook_shows_its_own_picture() {
    let (green, red) = (png([24, 150, 110, 255]), png([200, 40, 40, 255]));
    assert_eq!(green.len(), red.len());
    let mut h = harness();
    h.state_mut().run("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&green), "at": "B2"})).unwrap();
    h.run_steps(4);
    let first = picture_textures(&h);
    assert_eq!(first.len(), 1, "the picture is painted");

    h.state_mut().run("file.new", json!({})).unwrap();
    h.state_mut().run("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&red), "at": "B2"})).unwrap();
    h.run_steps(4);
    let doc = h.state().session.active().unwrap();
    assert_eq!(doc.wb.active().unwrap().images[0].id, h.state().session.documents()[0].wb.active().unwrap().images[0].id);
    let second = picture_textures(&h);
    assert_eq!(second.len(), 1);
    assert_ne!(second, first, "another workbook's picture 1 has a texture of its own");

    // Closing the workbook and painting again leaves the first one's picture as it was.
    h.state_mut().run("file.close", json!({"force": true})).unwrap();
    h.run_steps(4);
    assert_eq!(picture_textures(&h), first);
}
