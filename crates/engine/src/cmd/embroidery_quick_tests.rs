//! Quick rotate, flip and resize: round trips, the group moving as one, re-centring, the aspect lock
//! and the stitch density.

use serde_json::json;

use super::Session;

/// A hoop of 200 × 200 and a filled 100 × 50 mm (about) rectangle with a second, smaller one.
fn design() -> Session {
    let mut s = Session::new();
    s.execute("embroidery.hoop.saveCustom", &json!({"name": "Quick", "width": 200, "height": 200})).unwrap();
    s.execute("embroidery.newDesign", &json!({"hoop": "Quick"})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 280, "height": 140})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 20, "y": 20, "width": 60, "height": 30})).unwrap();
    s.execute("embroidery.set", &json!({"type": "fill"})).unwrap();
    s
}

fn size(s: &mut Session) -> (f64, f64) {
    let r = s.execute("embroidery.quick.rotate", &json!({"degrees": 0})).unwrap();
    (r["width"].as_f64().unwrap(), r["height"].as_f64().unwrap())
}

fn close(a: (f64, f64), b: (f64, f64), tol: f64) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
}

#[test]
fn four_quarter_turns_and_two_flips_are_the_original() {
    let mut s = design();
    let first = size(&mut s);
    let r = s.execute("embroidery.quick.rotate", &json!({"degrees": -90})).unwrap();
    assert!(close((r["width"].as_f64().unwrap(), r["height"].as_f64().unwrap()), (first.1, first.0), 2.0), "{r}");
    for _ in 0..3 {
        s.execute("embroidery.quick.rotate", &json!({"degrees": -90})).unwrap();
    }
    assert!(close(size(&mut s), first, 1.0));
    s.execute("embroidery.quick.flip", &json!({"axis": "horizontal"})).unwrap();
    s.execute("embroidery.quick.flip", &json!({"axis": "horizontal"})).unwrap();
    assert!(close(size(&mut s), first, 1.0));
    // nothing stays selected: it was nothing before
    assert!(s.doc().unwrap().selection.is_empty());
}

#[test]
fn eight_eighth_turns_are_the_original() {
    let mut s = design();
    let first = size(&mut s);
    for _ in 0..8 {
        s.execute("embroidery.quick.rotate", &json!({"degrees": 45})).unwrap();
    }
    assert!(close(size(&mut s), first, 3.0), "{:?} vs {first:?}", size(&mut s));
}

#[test]
fn rotation_is_recentred_offers_a_way_out_and_undoes() {
    let mut s = design();
    s.execute("embroidery.settings", &json!({"hoopPosition": [30, -10]})).unwrap();
    let r = s.execute("embroidery.quick.rotate", &json!({"degrees": 45})).unwrap();
    assert_eq!(s.doc().unwrap().doc.embroidery.as_deref().unwrap().settings.hoop_offset, None, "back in the middle");
    assert_eq!(r["fits"], true);
    // a hoop 120 wide and 60 high holds the design one way, not the other
    s.execute("embroidery.hoop.saveCustom", &json!({"name": "Strip", "width": 130, "height": 50})).unwrap();
    s.execute("embroidery.settings", &json!({"hoop": {"name": "Strip", "width": 130, "height": 50, "shape": "rect"}})).unwrap();
    s.execute("embroidery.quick.rotate", &json!({"degrees": -45})).unwrap();
    let r = s.execute("embroidery.quick.rotate", &json!({"degrees": 90})).unwrap();
    assert_eq!(r["fits"], false, "{r}");
    assert!(r["offers"].as_array().unwrap().contains(&json!("fitToHoop")));
    assert!(s.execute("embroidery.quick.rotate", &json!({"degrees": f64::NAN})).is_err());
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(size(&mut s).0.round() > size(&mut s).1.round(), true);
}

#[test]
fn resize_locks_the_aspect_keeps_the_density_and_warns() {
    let mut s = design();
    let (w0, h0) = size(&mut s);
    let n0 = s.embroidery_plan().unwrap().stats.as_ref().unwrap().stitches as f64;
    let r = s.execute("embroidery.quick.resize", &json!({"width": w0 * 1.1})).unwrap();
    let (w, h) = (r["width"].as_f64().unwrap(), r["height"].as_f64().unwrap());
    assert!((w / h - w0 / h0).abs() < 0.05, "aspect {w} x {h}");
    assert!((w / w0 - 1.1).abs() < 0.02 && r.get("warning").is_none(), "{r}");
    assert!((r["widthIn"].as_f64().unwrap() * 25.4 - w).abs() < 1e-9);
    // stitches per area stays within 10% of what it was
    let n1 = s.embroidery_plan().unwrap().stats.as_ref().unwrap().stitches as f64;
    let (d0, d1) = (n0 / (w0 * h0), n1 / (w * h));
    assert!((d1 / d0 - 1.0).abs() < 0.1, "density {d0} -> {d1}");
    // unlocked: width alone; past 20%: a warning; out of range and nonsense: errors
    let r = s.execute("embroidery.quick.resize", &json!({"percent": 50, "lock": false, "width": w * 0.5})).unwrap();
    assert!(r["warning"].is_string(), "{r}");
    assert!(s.execute("embroidery.quick.resize", &json!({"percent": 5000})).is_err());
    assert!(s.execute("embroidery.quick.resize", &json!({"width": -3})).is_err());
    assert!(s.execute("embroidery.quick.resize", &json!({})).is_err());
}

#[test]
fn a_selection_turns_as_one_group_and_the_rest_stays() {
    let mut s = design();
    let before = s.embroidery_plan().unwrap().stats.as_ref().unwrap().bounds;
    s.execute("select.all", &json!({})).unwrap();
    s.execute("embroidery.quick.rotate", &json!({"degrees": 180})).unwrap();
    // still selected, and the bounds are the same box
    assert!(!s.doc().unwrap().selection.is_empty());
    let after = s.embroidery_plan().unwrap().stats.as_ref().unwrap().bounds;
    assert!((before.width() - after.width()).abs() < 1.0 && (before.height() - after.height()).abs() < 1.0);
}
