//! Quick rotate, flip and resize (Add-on 40, phase 4): a turn or mirror of the selection (the whole
//! design when nothing is selected) as one group, put back in the middle of the hoop, and a resize
//! with the aspect ratio locked that keeps the stitch density (the stitches are made again from the
//! scaled objects, not their coordinates moved).

use serde_json::{Value, json};

use super::{CommandSpec, Result, Session, bad, cmd, has_doc, str_param};

#[cfg(test)]
#[path = "embroidery_quick_tests.rs"]
mod tests;

/// A resize past this many percent either way changes the look: warn.
const WARN_PERCENT: f64 = 20.0;
/// The resize range, in percent.
const SIZE_RANGE: std::ops::RangeInclusive<f64> = 1.0..=1000.0;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "embroidery.quick.rotate",
            "Quick Rotate",
            [],
            None,
            "{degrees: finite number (counter-clockwise; 90 left, -90 right, 45, 180…)} turn the selection, or the whole design when nothing is selected, as one group about its middle, then put it back in the middle of the hoop → {width, height, widthIn, heightIn, fits, offers: [rotateHoop|fitToHoop]}",
            has_doc,
            rotate
        ),
        cmd!(
            "embroidery.quick.rotateRight",
            "Quick Rotate 90° Right",
            [],
            Some("Cmd+Alt+Right"),
            "turn the selection, or the whole design when nothing is selected, 90° clockwise (same answer as embroidery.quick.rotate)",
            has_doc,
            rotate_right
        ),
        cmd!(
            "embroidery.quick.rotateLeft",
            "Quick Rotate 90° Left",
            [],
            Some("Cmd+Alt+Left"),
            "turn the selection, or the whole design when nothing is selected, 90° counter-clockwise (same answer as embroidery.quick.rotate)",
            has_doc,
            rotate_left
        ),
        cmd!(
            "embroidery.quick.flip",
            "Quick Flip",
            [],
            None,
            "{axis: horizontal (left to right) | vertical (top to bottom)} mirror the selection, or the whole design when nothing is selected, as one group → {width, height, widthIn, heightIn, fits, offers}",
            has_doc,
            flip
        ),
        cmd!(
            "embroidery.quick.resize",
            "Quick Resize",
            [],
            None,
            "{percent?: 1–1000, width?, height?: size in `unit`, unit?: mm|in (mm), lock?: bool (true: the aspect ratio kept, `height` ignored)} size the selection, or the whole design, with the stitches made again at the same density; past ±20% a warning comes back → {width, height, widthIn, heightIn, percent, locked, warning?, fits, offers}",
            has_doc,
            resize
        ),
    ]
}

/// Runs `f` with something selected: the selection, else everything (and nothing again after).
fn on_target<T>(s: &mut Session, f: impl FnOnce(&mut Session) -> Result<T>) -> Result<T> {
    let none = s.doc()?.selection.is_empty();
    if none {
        s.execute("select.all", &json!({}))?;
    }
    let r = f(s);
    if none {
        s.execute("select.none", &json!({}))?;
    }
    r
}

/// The size, as the answers give it, and what to offer when the design is over the hoop's field.
fn report(s: &Session, mut extra: Value) -> Value {
    let size = super::embroidery_hoops::design_size(s).unwrap_or((0.0, 0.0));
    let hoop = s.active().and_then(|d| d.doc.embroidery.as_deref()).and_then(|e| e.hoop.clone());
    let fits = hoop.as_ref().is_none_or(|h| super::embroidery_hoops::fits(h, size));
    let mut offers = vec![];
    if let (Some(h), false) = (&hoop, fits) {
        if super::embroidery_hoops::fits(&super::embroidery_hoops::turn(h), size) {
            offers.push("rotateHoop");
        }
        offers.push("fitToHoop");
    }
    if let Some(o) = extra.as_object_mut() {
        o.insert("width".into(), json!(size.0));
        o.insert("height".into(), json!(size.1));
        o.insert("widthIn".into(), json!(size.0 / 25.4));
        o.insert("heightIn".into(), json!(size.1 / 25.4));
        o.insert("fits".into(), json!(fits));
        o.insert("offers".into(), json!(offers));
    }
    extra
}

/// Puts the design back in the middle of its hoop (no hoop: nothing to do).
fn recentre(s: &mut Session) -> Result<()> {
    if s.doc()?.doc.embroidery.as_deref().is_some_and(|e| e.hoop.is_some()) {
        s.execute("embroidery.settings", &json!({ "hoopPosition": "centre" }))?;
    }
    Ok(())
}

fn rotate(s: &mut Session, p: &Value) -> Result<Value> {
    let deg =
        p.get("degrees").and_then(Value::as_f64).filter(|d| d.is_finite()).ok_or_else(|| bad("embroidery.quick.rotate", "`degrees` is a number"))?;
    on_target(s, |s| s.execute("object.rotate", &json!({ "angle": deg })).map(drop))?;
    recentre(s)?;
    Ok(report(s, json!({})))
}

fn rotate_right(s: &mut Session, _: &Value) -> Result<Value> {
    rotate(s, &json!({ "degrees": -90.0 }))
}

fn rotate_left(s: &mut Session, _: &Value) -> Result<Value> {
    rotate(s, &json!({ "degrees": 90.0 }))
}

fn flip(s: &mut Session, p: &Value) -> Result<Value> {
    // object.reflect's "vertical" axis mirrors left to right
    let axis = match str_param(p, "axis") {
        Some("horizontal") => "vertical",
        Some("vertical") => "horizontal",
        _ => return Err(bad("embroidery.quick.flip", "`axis` is horizontal or vertical")),
    };
    on_target(s, |s| s.execute("object.reflect", &json!({ "axis": axis })).map(drop))?;
    recentre(s)?;
    Ok(report(s, json!({})))
}

fn resize(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.quick.resize";
    let num = |k: &str| -> Result<Option<f64>> {
        match p.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v.as_f64().filter(|v| v.is_finite() && *v > 0.0).map(Some).ok_or_else(|| bad(CMD, format!("`{k}` is a positive number"))),
        }
    };
    let k = match str_param(p, "unit").unwrap_or("mm") {
        "mm" => 1.0,
        "in" => 25.4,
        _ => return Err(bad(CMD, "`unit` is mm or in")),
    };
    let locked = p.get("lock").and_then(Value::as_bool).unwrap_or(true);
    let (pct, w, h) = (num("percent")?, num("width")?.map(|v| v * k), num("height")?.map(|v| v * k));
    let (w0, h0) =
        super::embroidery_hoops::design_size(s).filter(|d| d.0 > 0.0 && d.1 > 0.0).ok_or_else(|| bad(CMD, "the design has no stitches yet"))?;
    // the factors asked for, in percent
    let (px, py) = match (pct, w, h) {
        (Some(p), _, _) => (p, p),
        (None, Some(w), _) if locked => (w / w0 * 100.0, w / w0 * 100.0),
        (None, None, Some(h)) if locked => (h / h0 * 100.0, h / h0 * 100.0),
        (None, w, h) if w.is_some() || h.is_some() => (w.map_or(100.0, |w| w / w0 * 100.0), h.map_or(100.0, |h| h / h0 * 100.0)),
        _ => return Err(bad(CMD, "give `percent`, `width` or `height`")),
    };
    if !SIZE_RANGE.contains(&px) || !SIZE_RANGE.contains(&py) {
        return Err(bad(CMD, "the size is 1% to 1000% of the design's"));
    }
    on_target(s, |s| {
        s.execute("object.scale", &json!({ "sx": px, "sy": py }))?;
        // stitch widths don't scale with the art: measure again and correct a few times
        for _ in 0..3 {
            let Some((w1, h1)) = super::embroidery_hoops::design_size(s).filter(|d| d.0 > 0.0 && d.1 > 0.0) else { break };
            let (wa, ha) = (w0 * px / 100.0, h0 * py / 100.0);
            let (cx, cy) = (wa / w1, ha / h1);
            if (cx - 1.0).abs() < 0.005 && (cy - 1.0).abs() < 0.005 {
                break;
            }
            let c = if locked { cx } else { cy };
            s.execute("object.scale", &json!({ "sx": cx * 100.0, "sy": c * 100.0 }))?;
        }
        Ok(())
    })?;
    recentre(s)?;
    let warn = (px - 100.0).abs() > WARN_PERCENT || (py - 100.0).abs() > WARN_PERCENT;
    let mut out = json!({ "percent": px, "locked": locked });
    if warn {
        out["warning"] =
            json!(format!("resized by {:+.0}%: past ±{WARN_PERCENT:.0}% the stitching may look different; check the result", px - 100.0));
    }
    Ok(report(s, out))
}
