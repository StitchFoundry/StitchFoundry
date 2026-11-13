//! Hoops (HO-01, HO-03, SYS-05): the smallest hoop that holds the design, chosen for it (turned
//! when that fits better), the hoop turned a quarter, and hoops of one's own kept by name beside
//! the standard ones.

use serde_json::{Value, json};
use sf_core::hoops::{Hoop, HoopShape};

use super::{CommandSpec, Result, Session, always, bad, bool_or, cmd, has_doc, str_param};

#[cfg(test)]
#[path = "embroidery_hoops_tests.rs"]
mod tests;

/// Hoops of one's own kept, at most.
const MAX_CUSTOM: usize = 100;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "embroidery.hoops",
            "Hoops",
            [],
            None,
            "{} the standard hoops and one's own, each with whether the design fits it (either way round) → {hoops: [{name, width, height, shape, custom, fits}], hoop, rotated}",
            always,
            list
        ),
        cmd!(
            "embroidery.hoop.auto",
            "Choose Smallest Hoop",
            ["Embroidery"],
            None,
            "{brand?: text the hoop's name has (Brother, Janome, Magnetic…), turn?: bool (true: a hoop turned a quarter counts too), special?: bool (false: cap, sleeve and jacket-back frames left out)} the smallest hoop (by its field) that holds the design, chosen for it → {hoop, width, height, rotated}",
            has_doc,
            auto
        ),
        cmd!(
            "embroidery.hoop.rotate",
            "Rotate Hoop",
            ["Embroidery"],
            None,
            "{} turn the design's hoop a quarter (its width and height swapped) → {hoop, width, height, rotated}",
            has_doc,
            rotate
        ),
        cmd!(
            "embroidery.hoop.saveCustom",
            "Save Custom Hoop",
            [],
            None,
            "{name, width, height?: 20–2000 mm (a round or square hoop needs just width), unit?: mm|in (mm), shape?: rect|oval|round|square (rect), cornerRadius?: rectangle corners, frameWidth?, frameHeight?: the outer frame, was?: the name of the hoop being renamed} a hoop of one's own (a new one, or the one of that name edited), listed with the standard ones (a standard hoop's name can't be taken); 100 at most → {name}",
            always,
            save_custom
        ),
        cmd!(
            "embroidery.hoop.attach",
            "Add Hoop to Machine",
            [],
            None,
            "{name, machine: a name from embroidery.myMachine's machines, attach?: bool (true; false takes it off)} put a hoop of one's own in that machine's hoop list; a hoop can be on several machines, and one on none is on every machine's list → {name, machines}",
            always,
            attach
        ),
        cmd!(
            "embroidery.hoop.export",
            "Export Hoop",
            [],
            None,
            "{name, path?: file to write} a hoop of one's own as a small JSON file to keep or share → {name, path | text}",
            always,
            export
        ),
        cmd!(
            "embroidery.hoop.import",
            "Import Hoop",
            [],
            None,
            "{path | text} a hoop file made by Export Hoop, added to one's own hoops (the one of that name replaced) → {name}",
            always,
            import
        ),
        cmd!("embroidery.hoop.deleteCustom", "Delete Custom Hoop", [], None, "{name} forget a hoop of one's own → {name}", always, delete_custom),
    ]
}

/// The standard hoops, then one's own.
pub(crate) fn all_hoops(s: &Session) -> Vec<Hoop> {
    let mut v = sf_core::hoops::standard_hoops();
    v.extend(s.prefs.embroidery_custom_hoops.iter().cloned());
    v
}

/// The hoop named `name` (any case), standard or one's own.
pub(crate) fn find_hoop(s: &Session, name: &str) -> Option<Hoop> {
    all_hoops(s).into_iter().find(|h| h.name.eq_ignore_ascii_case(name.trim()))
}

/// Whether the design's hoop is a named hoop turned a quarter.
fn turned(s: &Session, hoop: &Hoop) -> bool {
    find_hoop(s, &hoop.name).is_some_and(|h| (h.width - hoop.height).abs() < 1e-6 && (h.height - hoop.width).abs() < 1e-6 && h.width != h.height)
}

/// The design's size (mm) as it sews, if anything is stitched.
fn design_size(s: &Session) -> Option<(f64, f64)> {
    let plan = s.embroidery_plan()?;
    let st = plan.stats.as_ref()?;
    Some((st.width_mm, st.height_mm))
}

fn fits(h: &Hoop, (w, ht): (f64, f64)) -> bool {
    let b = |w: f64, h: f64| sf_core::BBox::new(sf_core::Point::new(-w / 2.0, -h / 2.0), sf_core::Point::new(w / 2.0, h / 2.0));
    h.fits(&b(w, ht))
}

fn turn(h: &Hoop) -> Hoop {
    Hoop {
        width: h.height,
        height: h.width,
        positions: h.positions.iter().map(|[x, y]| [*y, *x]).collect(),
        frame: h.frame.map(|[w, ht]| [ht, w]),
        ..h.clone()
    }
}

fn set_hoop(s: &mut Session, h: &Hoop) -> Result<()> {
    // the hoop as it is (turned, one's own), set whole
    let shape = if h.shape == HoopShape::Oval { "oval" } else { "rect" };
    let hoop = json!({ "name": h.name, "width": h.width, "height": h.height, "shape": shape, "cornerRadius": h.corner_radius, "frame": h.frame });
    s.execute("embroidery.settings", &json!({ "hoop": hoop }))?;
    Ok(())
}

fn answer(s: &Session, h: &Hoop) -> Value {
    json!({ "hoop": h.name, "width": h.width, "height": h.height, "rotated": turned(s, h) })
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let size = design_size(s);
    let custom = &s.prefs.embroidery_custom_hoops;
    let hoops: Vec<Value> = all_hoops(s)
        .iter()
        .map(|h| {
            json!({
                "name": h.name,
                "width": h.width,
                "height": h.height,
                "shape": if h.shape == HoopShape::Oval { "oval" } else { "rect" },
                "custom": custom.iter().any(|c| c.name == h.name),
                "cornerRadius": h.corner_radius,
                "frame": h.frame,
                "machines": h.machines,
                "fits": size.is_some_and(|d| fits(h, d) || fits(&turn(h), d)),
            })
        })
        .collect();
    let cur = s.active().and_then(|d| d.doc.embroidery.as_deref()).and_then(|e| e.hoop.clone());
    Ok(json!({ "hoops": hoops, "hoop": cur.as_ref().map(|h| h.name.clone()), "rotated": cur.as_ref().is_some_and(|h| turned(s, h)) }))
}

fn auto(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.auto";
    let size = design_size(s).ok_or_else(|| bad(CMD, "nothing is stitched yet: give objects a stitch type first"))?;
    let brand = str_param(p, "brand").map(|b| b.trim().to_lowercase()).filter(|b| !b.is_empty());
    let may_turn = bool_or(p, "turn", true);
    let special = bool_or(p, "special", false);
    let is_special = |h: &Hoop| {
        let n = h.name.to_lowercase();
        ["cap", "sleeve", "jacket"].iter().any(|w| n.contains(w))
    };
    let mut best: Option<Hoop> = None;
    for h in all_hoops(s) {
        if brand.as_ref().is_some_and(|b| !h.name.to_lowercase().contains(b.as_str())) || (!special && is_special(&h)) {
            continue;
        }
        let way = if fits(&h, size) {
            h
        } else if may_turn && fits(&turn(&h), size) {
            turn(&h)
        } else {
            continue;
        };
        if best.as_ref().is_none_or(|b| way.width * way.height < b.width * b.height - 1e-9) {
            best = Some(way);
        }
    }
    let h = best.ok_or_else(|| bad(CMD, format!("no hoop holds a design of {:.0} × {:.0} mm: try multi-hooping", size.0, size.1)))?;
    set_hoop(s, &h)?;
    Ok(answer(s, &h))
}

fn rotate(s: &mut Session, _: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.rotate";
    let cur = s.doc()?.doc.embroidery.as_deref().and_then(|e| e.hoop.clone()).ok_or_else(|| bad(CMD, "the design has no hoop yet"))?;
    let h = turn(&cur);
    set_hoop(s, &h)?;
    Ok(answer(s, &h))
}

/// The hoop `p` describes, checked: name, shape (rect, oval, round, square), sewing field,
/// optional corner radius and outer frame, in mm or inches (`unit`).
fn hoop_from(p: &Value, cmd: &str) -> Result<Hoop> {
    let name: String =
        str_param(p, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad(cmd, "missing `name`"))?.chars().take(80).collect();
    if name.chars().any(char::is_control) {
        return Err(bad(cmd, "the name can't hold control characters"));
    }
    if sf_core::hoops::standard_hoops().iter().any(|h| h.name.eq_ignore_ascii_case(&name)) {
        return Err(bad(cmd, format!("`{name}` is a standard hoop: give yours another name")));
    }
    let k = match str_param(p, "unit").unwrap_or("mm") {
        "mm" => 1.0,
        "in" => 25.4,
        _ => return Err(bad(cmd, "`unit` is mm or in")),
    };
    // a number in the hoop's unit, in mm; None when absent
    let num = |key: &str| -> Result<Option<f64>> {
        match p.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v.as_f64().filter(|v| v.is_finite()).map(|v| Some(v * k)).ok_or_else(|| bad(cmd, format!("`{key}` is a number"))),
        }
    };
    let shape_name = str_param(p, "shape").unwrap_or("rect");
    let (shape, same) = match shape_name {
        "rect" => (HoopShape::Rect, false),
        "oval" => (HoopShape::Oval, false),
        "square" => (HoopShape::Rect, true),
        "round" => (HoopShape::Oval, true),
        _ => return Err(bad(cmd, "`shape` is rect, oval, round or square")),
    };
    let (w, h) = match (num("width")?, num("height")?) {
        (Some(w), None) if same => (w, w),
        (Some(w), Some(h)) => (w, h),
        _ => return Err(bad(cmd, "give `width` and `height` (a round or square hoop needs just `width`)")),
    };
    if !(20.0..=2000.0).contains(&w) || !(20.0..=2000.0).contains(&h) {
        return Err(bad(cmd, "the sewing field is 20 to 2000 mm (0.8 to 78.7 in) each way"));
    }
    if same && (w - h).abs() > 1e-6 {
        return Err(bad(cmd, format!("a {shape_name} hoop has the same width and height")));
    }
    let mut hoop = Hoop::new(&name, w, h, shape);
    if let Some(r) = num("cornerRadius")?.filter(|r| *r != 0.0) {
        if shape != HoopShape::Rect || !(0.0..=w.min(h) / 2.0).contains(&r) {
            return Err(bad(cmd, "`cornerRadius` is for a rectangle or square, at most half its shorter side"));
        }
        hoop.corner_radius = r;
    }
    if let (fw, fh) = (num("frameWidth")?, num("frameHeight")?)
        && (fw.is_some() || fh.is_some())
    {
        let (Some(fw), Some(fh)) = (fw, fh) else { return Err(bad(cmd, "give both `frameWidth` and `frameHeight`")) };
        if fw < w || fh < h || fw > 3000.0 || fh > 3000.0 {
            return Err(bad(cmd, "the outer frame holds the sewing field and is 3000 mm at most"));
        }
        hoop.frame = Some([fw, fh]);
    }
    Ok(hoop)
}

/// Keeps `hoop` among one's own: replaces the one of that name (keeping its machines), else adds it.
fn keep(s: &mut Session, mut hoop: Hoop, cmd: &str) -> Result<()> {
    let own = &mut s.prefs.embroidery_custom_hoops;
    let full = own.len() >= MAX_CUSTOM;
    match own.iter_mut().find(|o| o.name.eq_ignore_ascii_case(&hoop.name)) {
        Some(o) => {
            if hoop.machines.is_empty() {
                hoop.machines = std::mem::take(&mut o.machines);
            }
            *o = hoop;
        }
        None if full => return Err(bad(cmd, format!("{MAX_CUSTOM} hoops of your own at most: delete one first"))),
        None => own.push(hoop),
    }
    Ok(())
}

fn save_custom(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.saveCustom";
    let hoop = hoop_from(p, CMD)?;
    let name = hoop.name.clone();
    // rename: `was` names the hoop being edited
    let was = str_param(p, "was").map(str::trim).filter(|w| !w.is_empty() && !w.eq_ignore_ascii_case(&name));
    if let Some(was) = was
        && !s.prefs.embroidery_custom_hoops.iter().any(|h| h.name.eq_ignore_ascii_case(was))
    {
        return Err(bad(CMD, format!("no hoop of yours is named `{was}`")));
    }
    let mut hoop = hoop;
    if let Some(was) = was
        && let Some(i) = s.prefs.embroidery_custom_hoops.iter().position(|h| h.name.eq_ignore_ascii_case(was))
    {
        hoop.machines = s.prefs.embroidery_custom_hoops.remove(i).machines;
    }
    keep(s, hoop, CMD)?;
    Ok(json!({ "name": name }))
}

fn attach(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.attach";
    let name = str_param(p, "name").map(str::trim).ok_or_else(|| bad(CMD, "missing `name`"))?;
    let machine = str_param(p, "machine").ok_or_else(|| bad(CMD, "missing `machine`: a name from embroidery.myMachine"))?;
    let machine = sf_core::machines::home_machine(machine).ok_or_else(|| bad(CMD, format!("unknown machine `{machine}`")))?.name.to_string();
    let on = bool_or(p, "attach", true);
    let h = s
        .prefs
        .embroidery_custom_hoops
        .iter_mut()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| bad(CMD, format!("no hoop of yours is named `{name}`")))?;
    h.machines.retain(|m| *m != machine);
    if on {
        h.machines.push(machine);
    }
    Ok(json!({ "name": h.name, "machines": h.machines }))
}

/// Export file marker.
const FILE_KIND: &str = "stitchfoundry-hoop";
const MAX_FILE: usize = 64 * 1024;

fn export(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.export";
    let name = str_param(p, "name").map(str::trim).ok_or_else(|| bad(CMD, "missing `name`"))?;
    let mut h = s
        .prefs
        .embroidery_custom_hoops
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .cloned()
        .ok_or_else(|| bad(CMD, format!("no hoop of yours is named `{name}`")))?;
    // machines are this PC's: the file carries the hoop only
    h.machines.clear();
    let text = serde_json::to_string_pretty(&json!({ "format": FILE_KIND, "version": 1, "hoop": h })).map_err(|e| bad(CMD, e.to_string()))?;
    match str_param(p, "path").map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => {
            super::fileio::write_file(path, text.as_bytes())?;
            Ok(json!({ "name": h.name, "path": path }))
        }
        None => Ok(json!({ "name": h.name, "text": text })),
    }
}

fn import(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.import";
    let text = match (str_param(p, "text"), str_param(p, "path")) {
        (Some(t), _) => t.to_string(),
        (None, Some(path)) => String::from_utf8_lossy(&super::fileio::read_file(path)?).into_owned(),
        (None, None) => return Err(bad(CMD, "give `path` or `text`")),
    };
    if text.len() > MAX_FILE {
        return Err(bad(CMD, "a hoop file is under 64 KB"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| bad(CMD, format!("not a hoop file: {e}")))?;
    if v.get("format").and_then(Value::as_str) != Some(FILE_KIND) {
        return Err(bad(CMD, "not a StitchFoundry hoop file"));
    }
    let h = v.get("hoop").ok_or_else(|| bad(CMD, "the file holds no hoop"))?;
    let mut hoop: Hoop = serde_json::from_value(h.clone()).map_err(|e| bad(CMD, format!("the hoop is damaged: {e}")))?;
    // the same checks as making one by hand (an oval or round field is told by its shape)
    let shape = if hoop.shape == HoopShape::Oval { "oval" } else { "rect" };
    let mut q = json!({ "name": hoop.name, "width": hoop.width, "height": hoop.height, "shape": shape });
    if hoop.corner_radius != 0.0 {
        q["cornerRadius"] = json!(hoop.corner_radius);
    }
    if let Some([fw, fh]) = hoop.frame {
        q["frameWidth"] = json!(fw);
        q["frameHeight"] = json!(fh);
    }
    let checked = hoop_from(&q, CMD)?;
    hoop = Hoop { machines: Vec::new(), positions: Vec::new(), ..checked };
    let name = hoop.name.clone();
    keep(s, hoop, CMD)?;
    Ok(json!({ "name": name }))
}

fn delete_custom(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.hoop.deleteCustom";
    let name = str_param(p, "name").map(str::trim).ok_or_else(|| bad(CMD, "missing `name`"))?.to_string();
    let own = &mut s.prefs.embroidery_custom_hoops;
    let before = own.len();
    own.retain(|h| !h.name.eq_ignore_ascii_case(&name));
    if own.len() == before {
        return Err(bad(CMD, format!("no hoop of yours is named `{name}`")));
    }
    Ok(json!({ "name": name }))
}
