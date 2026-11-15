//! Stitch advisor: a static, hand-written table of starting points (needle, stabilizer, hints) by
//! fabric, plus warnings for small lettering. Every answer says to test on scrap first.

use serde_json::{Value, json};

use super::{CommandSpec, Result, Session, always, bad, cmd, str_param};

const CMD: &str = "embroidery.advisor";
/// The closing note on all advice.
const NOTE: &str = "A starting point, test on scrap first.";
/// Lettering shorter than this (mm) is flagged.
const SMALL_MM: f64 = 6.0;
/// Lettering this short (mm) or less is too small for any font.
const TINY_MM: f64 = 3.0;

/// One row: (fabric, needle, stabilizer, hints).
type Row = (&'static str, &'static str, &'static str, &'static [&'static str]);

/// Starting points by kind of fabric, in the order the app lists them.
const TABLE: [Row; 8] = [
    (
        "woven",
        "75/11 sharp or universal",
        "Tear-away for a stable weave; cut-away for a loose weave or a dense design",
        &["Standard density and underlay.", "Hoop firmly without stretching the fabric."],
    ),
    (
        "knit",
        "70/10 or 75/11 ballpoint",
        "Cut-away, or no-show mesh under light colours",
        &["Use edge and zigzag underlay.", "Add pull compensation.", "Don't pull the knit tight in the hoop."],
    ),
    (
        "heavy",
        "90/14 sharp (denim, heavy fleece)",
        "Tear-away, or cut-away under a dense design",
        &["Open the density a little.", "A water-soluble topping keeps fleece nap from swallowing stitches."],
    ),
    (
        "towel",
        "80/12 sharp or ballpoint",
        "Tear-away or cut-away, with a water-soluble topping on the top",
        &["Use a knockdown stitch under the design.", "Add pull compensation.", "Avoid small lettering."],
    ),
    (
        "silk",
        "65/9 or 70/10 sharp, new",
        "No-show mesh or a light fusible backing",
        &["Lower the density.", "Keep underlay light.", "Hoop gently to avoid hoop marks."],
    ),
    (
        "leather",
        "90/14 leather or sharp",
        "Tear-away on the back; hold the leather with clips or tape rather than hooping",
        &["Lower the density: needle holes don't heal.", "Avoid stitching the same spot twice."],
    ),
    (
        "cap",
        "75/11 sharp or cap needle",
        "Firm tear-away or cut-away, with a cap frame",
        &["Sew the middle first.", "Add pull compensation.", "Keep the design inside the cap's height."],
    ),
    (
        "stretchy-thin",
        "70/10 ballpoint",
        "Fusible cut-away (iron-on) to stabilise the stretch",
        &["Fuse the backing before hooping.", "Keep density low to avoid puckers."],
    ),
];

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "embroidery.advisor",
        "Stitch Advisor",
        [],
        None,
        "{fabric?: woven|knit|heavy|towel|silk|leather|cap|stretchy-thin (the list of fabrics when not given), textHeight?: the shortest lettering in mm (a warning when it is under 6 mm)} suggested needle, stabilizer and hints for a fabric, with needle-break and puckering advice and a small-lettering warning; a starting point, test on scrap first → {fabric, needle, stabilizer, hints, warnings, note} or {fabrics, note}",
        always,
        advisor
    )]
}

/// Warnings for lettering `mm` tall.
fn small_text(mm: f64) -> Vec<String> {
    if mm >= SMALL_MM {
        return Vec::new();
    }
    let mut w = vec![format!(
        "Lettering under about 6 mm (0.25 in) tall: use a simple sans font, finer thread (such as 60 weight), or make it larger. {NOTE}"
    )];
    if mm > TINY_MM {
        w.push("3 to 5 mm only works for simple fonts with fine thread.".into());
    } else {
        w.push("Under 3 mm is too small to sew cleanly: make it larger.".into());
    }
    w
}

fn advisor(_s: &mut Session, p: &Value) -> Result<Value> {
    let height = match p.get("textHeight") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_f64().filter(|h| h.is_finite() && *h > 0.0).ok_or_else(|| bad(CMD, "`textHeight` is a number of mm above 0"))?),
    };
    let Some(fabric) = str_param(p, "fabric") else {
        return Ok(json!({ "fabrics": TABLE.iter().map(|r| r.0).collect::<Vec<_>>(), "note": NOTE }));
    };
    let &(id, needle, stabilizer, hints) =
        TABLE.iter().find(|r| r.0 == fabric).ok_or_else(|| bad(CMD, format!("no fabric `{fabric}`: {}", TABLE.map(|r| r.0).join(", "))))?;
    let mut hints: Vec<String> = hints.iter().map(|h| (*h).to_string()).collect();
    hints.push("Needle breaks: look for a dense spot, replace a worn or bent needle, and check the needle size suits the thread.".into());
    hints.push("Puckering: reduce density, add underlay or a firmer stabilizer, hoop snugly; resizing a design for a smaller hoop packs the stitches denser.".into());
    Ok(json!({
        "fabric": id, "needle": needle, "stabilizer": stabilizer, "hints": hints,
        "warnings": height.map(small_text).unwrap_or_default(), "note": NOTE,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advice_for_each_fabric_says_test_on_scrap() {
        let mut s = Session::new();
        let all = s.execute(CMD, &json!({})).unwrap();
        assert_eq!(all["fabrics"].as_array().unwrap().len(), TABLE.len());
        for r in TABLE {
            let a = s.execute(CMD, &json!({ "fabric": r.0 })).unwrap();
            assert!(!a["needle"].as_str().unwrap().is_empty());
            assert!(a["note"].as_str().unwrap().contains("test on scrap first"));
            assert!(a["hints"].as_array().unwrap().iter().any(|h| h.as_str().unwrap().contains("Puckering")));
        }
        assert!(s.execute(CMD, &json!({ "fabric": "heavy" })).unwrap()["needle"].as_str().unwrap().starts_with("90/14"));
        assert!(s.execute(CMD, &json!({ "fabric": "lace" })).is_err());
    }

    #[test]
    fn small_lettering_is_flagged() {
        let mut s = Session::new();
        let warn =
            |s: &mut Session, h: f64| s.execute(CMD, &json!({ "fabric": "woven", "textHeight": h })).unwrap()["warnings"].as_array().unwrap().len();
        assert_eq!(warn(&mut s, 8.0), 0);
        assert_eq!(warn(&mut s, 5.0), 2);
        assert_eq!(warn(&mut s, 2.0), 2);
        assert!(s.execute(CMD, &json!({ "fabric": "woven", "textHeight": -1 })).is_err());
    }
}
