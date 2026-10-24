//! Moving original synthetic pictures between cell content and floating drawings is atomic.

use std::io::Cursor;

use gridcraft_core::CellRef;
use gridcraft_engine::Session;
use serde_json::{Value as Json, json};

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(120, 60, image::Rgba([24, 150, 110, 255]));
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn run(s: &mut Session, command: &str, params: Json) -> Json {
    s.execute(command, params).unwrap()
}

fn session() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s
}

#[test]
fn conversions_preserve_pixels_alt_and_cell_style_and_each_undo_once() {
    let mut s = session();
    let bytes = png();
    let cell = CellRef::parse("B2").unwrap();
    run(&mut s, "home.bold", json!({"range": "B2", "on": true}));
    let id =
        run(&mut s, "insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&bytes), "at": "B2", "alt": "Green"}))["image"].clone();
    let original = s.doc().unwrap().wb.clone();
    let undo = s.doc().unwrap().undo.len();
    let result = run(&mut s, "picture.placeInCell", json!({"image": id}));
    assert_eq!(result, json!({"cell": "B2", "placement": "cell"}));
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert!(sh.images.is_empty());
    let picture = sh.cell_pictures.get(&cell).unwrap();
    assert_eq!(picture.data, bytes);
    assert_eq!(picture.alt, "Green");
    assert_eq!(picture.mime, "image/png");
    assert!(s.doc().unwrap().wb.styles.get(sh.style_id(cell)).font.bold);
    assert_eq!(s.doc().unwrap().undo.len(), undo + 1);
    let in_cell = s.doc().unwrap().wb.clone();
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc().unwrap().wb, original);
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(s.doc().unwrap().wb, in_cell);

    let result = run(&mut s, "picture.placeOverCells", json!({"cell": "B2"}));
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert!(sh.cell_pictures.is_empty());
    assert!(sh.value(cell).is_empty());
    assert!(s.doc().unwrap().wb.styles.get(sh.style_id(cell)).font.bold);
    assert_eq!(sh.images.len(), 1);
    assert_eq!(json!(sh.images[0].id), result["image"]);
    assert_eq!(*sh.images[0].data, bytes);
    assert_eq!(sh.images[0].alt, "Green");
    assert_eq!(sh.images[0].anchor.cell, cell);
    assert_eq!((sh.images[0].anchor.width, sh.images[0].anchor.height), (120.0, 60.0));
    assert_eq!(s.doc().unwrap().undo.len(), undo + 2);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc().unwrap().wb, in_cell);
}

#[test]
fn conversions_resolve_merged_cell_anchors() {
    let mut s = session();
    run(&mut s, "home.mergeCenter", json!({"range": "B2:D4"}));
    let id = run(&mut s, "insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&png()), "at": "C3"}))["image"].clone();
    assert_eq!(run(&mut s, "picture.placeInCell", json!({"image": id}))["cell"], "B2");
    run(&mut s, "picture.placeOverCells", json!({"cell": "D4", "width": 240, "height": 120}));
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert!(sh.cell_pictures.is_empty());
    assert_eq!(sh.images[0].anchor.cell.a1(), "B2");
    assert_eq!((sh.images[0].anchor.width, sh.images[0].anchor.height), (240.0, 120.0));
}

#[test]
fn protected_or_invalid_conversions_leave_both_placements_and_undo_intact() {
    let mut s = session();
    let bytes = gridcraft_engine::io::base64_encode(&png());
    let id = run(&mut s, "insert.picture", json!({"base64": bytes, "at": "B2"}))["image"].clone();
    run(&mut s, "insert.picture", json!({"base64": bytes, "at": "D4", "placement": "cell"}));
    run(&mut s, "review.protectSheet", json!({}));
    for (command, params) in [("picture.placeInCell", json!({"image": id})), ("picture.placeOverCells", json!({"cell": "D4"}))] {
        let before = s.doc().unwrap().wb.clone();
        let undo = s.doc().unwrap().undo.len();
        assert!(s.execute(command, params).unwrap_err().to_string().contains("protected"));
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().undo.len(), undo);
    }
    run(&mut s, "review.unprotectSheet", json!({}));
    for (command, params) in [
        ("picture.placeInCell", json!({"image": 999})),
        ("picture.placeOverCells", json!({"cell": "D4", "width": -3})),
        ("picture.placeOverCells", json!({"cell": "A1"})),
    ] {
        let before = s.doc().unwrap().wb.clone();
        let undo = s.doc().unwrap().undo.len();
        assert!(s.execute(command, params).is_err());
        assert_eq!(s.doc().unwrap().wb, before);
        assert_eq!(s.doc().unwrap().undo.len(), undo);
    }
}

#[test]
fn undecodable_floating_picture_is_not_lost_on_conversion() {
    let mut s = session();
    let id = run(&mut s, "insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(b"invalid pixels")}))["image"].clone();
    let before = s.doc().unwrap().wb.clone();
    assert!(s.execute("picture.placeInCell", json!({"image": id})).is_err());
    assert_eq!(s.doc().unwrap().wb, before);
}
