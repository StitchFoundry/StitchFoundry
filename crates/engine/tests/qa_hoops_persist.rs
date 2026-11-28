//! Add-on 40 QA: hoop and profile settings persist after a reload (preferences saved, read back
//! into a fresh session). A custom hoop edited in inches or mm, the overlay settings, My Machine
//! and the last hoop picked must all come back unchanged.
// Integration tests: unwrapping and panicking on failure is fine here (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use vectorcraft_engine::{Prefs, Session};

fn reload(s: &Session) -> Session {
    let mut fresh = Session::new();
    fresh.prefs = serde_json::from_str::<Prefs>(&serde_json::to_string(&s.prefs).unwrap()).unwrap();
    fresh
}

#[test]
fn custom_hoop_overlay_and_machine_survive_a_reload() {
    let mut s = Session::new();
    s.execute(
        "embroidery.hoop.saveCustom",
        &json!({"name": "QA Rect", "width": 5, "height": 7, "unit": "in", "shape": "rect", "cornerRadius": 0.5, "frameWidth": 6, "frameHeight": 8}),
    )
    .unwrap();
    // editing the same hoop (in mm this time) replaces it, and the edit is what comes back
    s.execute(
        "embroidery.hoop.saveCustom",
        &json!({"name": "QA Rect", "width": 130, "height": 180, "shape": "rect", "cornerRadius": 12, "was": "QA Rect"}),
    )
    .unwrap();
    s.execute("embroidery.hoopOverlay", &json!({"show": false, "opacity": 0.4})).unwrap();
    let machine = sf_core::machines::HOME_MACHINES.first().unwrap().name;
    s.execute("embroidery.myMachine.set", &json!({"machine": machine})).unwrap();
    s.execute("embroidery.newDesign", &json!({"hoop": "QA Rect"})).unwrap();

    let mut fresh = reload(&s);
    assert_eq!(fresh.prefs.embroidery_custom_hoops, s.prefs.embroidery_custom_hoops);
    assert_eq!(fresh.prefs.embroidery_custom_hoops.len(), 1);
    let h = &fresh.prefs.embroidery_custom_hoops[0];
    assert!((h.width - 130.0).abs() < 1e-9 && (h.height - 180.0).abs() < 1e-9 && (h.corner_radius - 12.0).abs() < 1e-9, "{h:?}");
    assert!(!fresh.prefs.embroidery_hoop_overlay);
    assert!((fresh.prefs.embroidery_hoop_overlay_opacity - 0.4).abs() < 1e-6);
    // the reloaded session still offers the hoop and picks it for the next design
    let r = fresh.execute("embroidery.newDesign", &json!({})).unwrap();
    assert_eq!(r["hoop"], "QA Rect");
    let r = fresh.execute("embroidery.hoopOverlay", &json!({})).unwrap();
    assert_eq!(r["show"], true, "toggling from the persisted off state turns it on");
}
