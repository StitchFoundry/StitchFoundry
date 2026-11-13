//! Hoops of one's own: create, put on a machine, keep, export and import, and fit.

use serde_json::{Value, json};
use sf_core::hoops::{Hoop, HoopShape};
use sf_core::{BBox, Point};

use super::Session;

const DIAMOND: &str = "Husqvarna Viking Designer Diamond deLuxe";

fn names(s: &mut Session) -> Vec<String> {
    let r = s.execute("embroidery.myMachine", &json!({})).unwrap();
    r["hoops"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn create_attach_persist_and_reload() {
    let mut s = Session::new();
    s.execute(
        "embroidery.hoop.saveCustom",
        &json!({"name": "Round 6in", "width": 6, "unit": "in", "shape": "round", "frameWidth": 7.5, "frameHeight": 7.5}),
    )
    .unwrap();
    let h = s.prefs.embroidery_custom_hoops[0].clone();
    assert!((h.width - 152.4).abs() < 1e-9 && h.height == h.width && h.shape == HoopShape::Oval);
    s.execute("embroidery.myMachine.set", &json!({"machine": DIAMOND})).unwrap();
    s.execute("embroidery.hoop.attach", &json!({"name": "round 6in", "machine": DIAMOND})).unwrap();
    assert!(names(&mut s).contains(&"Round 6in".to_string()));
    // attached to another machine only: not on this machine's list
    let other = sf_core::machines::HOME_MACHINES.iter().find(|m| m.name != DIAMOND).unwrap().name;
    s.execute("embroidery.hoop.attach", &json!({"name": "Round 6in", "machine": other})).unwrap();
    s.execute("embroidery.hoop.attach", &json!({"name": "Round 6in", "machine": DIAMOND, "attach": false})).unwrap();
    assert!(!names(&mut s).contains(&"Round 6in".to_string()));
    // saved in the preferences file and read back
    let text = serde_json::to_string(&s.prefs).unwrap();
    let back: crate::prefs::Prefs = serde_json::from_str(&text).unwrap();
    assert_eq!(back.embroidery_custom_hoops, s.prefs.embroidery_custom_hoops);
    assert_eq!(back.embroidery_custom_hoops[0].machines, vec![other.to_string()]);
    // edit (rename) keeps its machines; delete removes it
    s.execute("embroidery.hoop.saveCustom", &json!({"name": "Round 7in", "width": 7, "unit": "in", "shape": "round", "was": "Round 6in"})).unwrap();
    assert_eq!(s.prefs.embroidery_custom_hoops.len(), 1);
    assert_eq!(s.prefs.embroidery_custom_hoops[0].name, "Round 7in");
    assert_eq!(s.prefs.embroidery_custom_hoops[0].machines, vec![other.to_string()]);
    s.execute("embroidery.hoop.deleteCustom", &json!({"name": "Round 7in"})).unwrap();
    assert!(s.prefs.embroidery_custom_hoops.is_empty());
}

#[test]
fn validation() {
    let mut s = Session::new();
    let cases = [
        json!({"name": "A", "width": 0, "height": 100}),
        json!({"name": "A", "width": -5, "height": 100}),
        json!({"name": "A", "width": 100, "height": 5000}),
        json!({"name": "A", "width": 100}),
        json!({"name": "A", "width": 100, "height": 120, "shape": "square"}),
        json!({"name": "A", "width": 100, "height": 100, "cornerRadius": 60}),
        json!({"name": "A", "width": 100, "height": 100, "shape": "oval", "cornerRadius": 5}),
        json!({"name": "A", "width": 100, "height": 100, "frameWidth": 90, "frameHeight": 120}),
        json!({"name": "A", "width": 100, "height": 100, "unit": "cm"}),
        json!({"name": "A", "width": 100, "height": 100, "shape": "star"}),
        json!({"name": "A", "width": 100, "height": 100, "was": "Nobody"}),
        json!({"name": "A\nB", "width": 100, "height": 100}),
    ];
    for p in cases {
        assert!(s.execute("embroidery.hoop.saveCustom", &p).is_err(), "{p}");
    }
    assert!(s.prefs.embroidery_custom_hoops.is_empty());
}

#[test]
fn export_then_import_is_identical() {
    let mut s = Session::new();
    let p = json!({"name": "Shop frame", "width": 175, "height": 115, "cornerRadius": 12, "frameWidth": 200, "frameHeight": 140});
    s.execute("embroidery.hoop.saveCustom", &p).unwrap();
    s.execute("embroidery.hoop.attach", &json!({"name": "Shop frame", "machine": DIAMOND})).unwrap();
    let before = s.prefs.embroidery_custom_hoops[0].clone();
    let text = s.execute("embroidery.hoop.export", &json!({"name": "Shop frame"})).unwrap()["text"].as_str().unwrap().to_string();
    s.execute("embroidery.hoop.deleteCustom", &json!({"name": "Shop frame"})).unwrap();
    s.execute("embroidery.hoop.import", &json!({"text": text})).unwrap();
    let after = s.prefs.embroidery_custom_hoops[0].clone();
    // everything but this PC's machines
    assert_eq!(after, Hoop { machines: Vec::new(), ..before });
    // through a file on disk
    let path = std::env::temp_dir().join(format!("sf-hoop-{}.json", std::process::id())).to_string_lossy().to_string();
    s.execute("embroidery.hoop.export", &json!({"name": "Shop frame", "path": path})).unwrap();
    s.execute("embroidery.hoop.deleteCustom", &json!({"name": "Shop frame"})).unwrap();
    s.execute("embroidery.hoop.import", &json!({"path": path})).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(s.prefs.embroidery_custom_hoops[0], after);
    // junk and out-of-range files are refused
    let junk = [
        "",
        "[]",
        "{}",
        r#"{"format":"stitchfoundry-hoop","hoop":{"name":"X","width":-1,"height":50}}"#,
        r#"{"format":"stitchfoundry-hoop","hoop":3}"#,
    ];
    for t in junk {
        assert!(s.execute("embroidery.hoop.import", &json!({"text": t})).is_err(), "{t}");
    }
}

#[test]
fn fit_check_uses_the_custom_hoop() {
    let mut s = Session::new();
    s.execute("embroidery.hoop.saveCustom", &json!({"name": "Tall", "width": 100, "height": 200, "cornerRadius": 30})).unwrap();
    s.execute("embroidery.newDesign", &json!({"hoop": "tall"})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 50, "height": 150})).unwrap();
    s.execute("embroidery.set", &json!({"type": "fill"})).unwrap();
    let r = s.execute("embroidery.hoops", &json!({})).unwrap();
    let tall: &Value = r["hoops"].as_array().unwrap().iter().find(|h| h["name"] == "Tall").unwrap();
    assert_eq!(tall["fits"], true);
    assert_eq!(r["hoop"], "Tall");
    // rounded corners are cut off and the outline follows them
    let h = s.prefs.embroidery_custom_hoops[0].clone();
    let b = |w: f64, ht: f64| BBox::new(Point::new(-w / 2.0, -ht / 2.0), Point::new(w / 2.0, ht / 2.0));
    assert!(h.fits(&b(80.0, 150.0)) && !h.fits(&b(100.0, 150.0)) && !h.fits(&b(100.0, 220.0)));
    assert!(!h.contains(Point::new(49.0, 99.0)) && h.contains(Point::new(49.0, 60.0)));
    assert!(h.outline().iter().all(|p| h.contains(Point::new(p.x * 0.999, p.y * 0.999))));
    // turned a quarter, it is 200 × 100
    let r = s.execute("embroidery.hoop.rotate", &json!({})).unwrap();
    assert_eq!((r["width"].as_f64(), r["height"].as_f64()), (Some(200.0), Some(100.0)));
}
