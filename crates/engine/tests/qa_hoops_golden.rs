//! Add-on 40 QA matrix, slice 2: golden files for every export format we write, and the check
//! that the hoop overlay never blocks the design. Regenerate the goldens (after checking the
//! change is intended) with `UPDATE_GOLDEN=1 cargo test -p vectorcraft-engine --test qa_hoops_golden`.
// Integration tests: unwrapping and panicking on failure is fine here (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use vectorcraft_doc::geom::Point;
use vectorcraft_doc::hit::{HitOptions, hit_test};
use vectorcraft_embroidery::PT_PER_MM;
use vectorcraft_engine::Session;

const HOOP: &str = "Brother 100 × 100 (4×4)";
const FORMATS: [&str; 5] = ["dst", "pes", "jef", "exp", "vp3"];

/// A 30 mm filled square, centred, on the 100 × 100 mm hoop.
fn design() -> Session {
    let mut s = Session::new();
    let side = 30.0 * PT_PER_MM;
    s.execute("embroidery.newDesign", &json!({ "hoop": HOOP })).unwrap();
    s.execute("shape.rectangle", &json!({ "x": 0, "y": 0, "width": side, "height": side })).unwrap();
    s.execute("select.all", &json!({})).unwrap();
    s.execute("embroidery.set", &json!({ "type": "fill" })).unwrap();
    s.execute("select.none", &json!({})).unwrap();
    s
}

fn export(s: &mut Session, fmt: &str) -> Vec<u8> {
    let r = s.execute("embroidery.export", &json!({ "format": fmt })).unwrap();
    let mut bytes = r["dataBase64"].as_str().and_then(vectorcraft_format::base64_decode).unwrap();
    // JEF holds its creation time (YYYYMMDDHHMMSS) at bytes 8..22: blank it so the file is comparable.
    if fmt == "jef"
        && let Some(stamp) = bytes.get_mut(8..22)
    {
        stamp.fill(b'0');
    }
    bytes
}

#[test]
fn exports_match_the_golden_files() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/qa_hoops");
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut s = design();
    for fmt in FORMATS {
        let bytes = export(&mut s, fmt);
        assert_eq!(bytes, export(&mut s, fmt), "{fmt}: two exports of one design differ");
        let path = dir.join(format!("square30.{fmt}"));
        if update {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &bytes).unwrap();
        }
        let golden = std::fs::read(&path).unwrap_or_else(|_| panic!("missing golden {path:?}; run with UPDATE_GOLDEN=1"));
        assert!(
            golden == bytes,
            "{fmt}: export differs from {path:?} ({} vs {} bytes); if intended, rerun with UPDATE_GOLDEN=1",
            bytes.len(),
            golden.len()
        );
    }
}

#[test]
fn every_golden_reads_back_inside_the_field() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/qa_hoops");
    for fmt in FORMATS {
        let bytes = std::fs::read(dir.join(format!("square30.{fmt}"))).unwrap();
        let (_, p) = sf_formats::read_any(&bytes, Some(fmt)).unwrap_or_else(|e| panic!("{fmt}: {e}"));
        let b = p.bounds();
        assert!(p.stitch_count() > 0 && b.width() < 100.0 && b.height() < 100.0, "{fmt}: {} x {} mm", b.width(), b.height());
        assert!((b.width() - 30.0).abs() < 2.0, "{fmt}: width {} mm", b.width());
    }
}

/// The overlay is drawn by the canvas, not part of the document: with it on or off a click on the
/// design hits the same object, selecting works, and toggling it leaves the document untouched.
#[test]
fn the_overlay_never_blocks_the_design() {
    let mut s = design();
    let mid = Point::new(15.0 * PT_PER_MM, 15.0 * PT_PER_MM);
    let hit = |s: &Session| hit_test(&s.doc().unwrap().doc, mid, HitOptions::default()).map(|h| h.leaf);
    let before = serde_json::to_string(&s.execute("embroidery.export", &json!({ "format": "dst" })).unwrap()).unwrap();
    let mut seen = Vec::new();
    for show in [true, false, true] {
        s.execute("embroidery.hoopOverlay", &json!({ "show": show })).unwrap();
        assert_eq!(s.execute("embroidery.hoopOverlay", &json!({ "show": show })).unwrap()["show"], show);
        let id = hit(&s).expect("the design is hit with the overlay set");
        s.execute("select.set", &json!({ "ids": [id.0] })).unwrap();
        assert_eq!(s.doc().unwrap().selection.objects, vec![id]);
        s.execute("select.none", &json!({})).unwrap();
        seen.push(id);
    }
    assert!(seen.windows(2).all(|w| w[0] == w[1]), "overlay on/off changed what a click hits");
    let after = serde_json::to_string(&s.execute("embroidery.export", &json!({ "format": "dst" })).unwrap()).unwrap();
    assert_eq!(before, after, "toggling the overlay changed the design");
}
