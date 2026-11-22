//! My Machine (E4): the embroidery machine someone sews on, chosen once. New designs take its
//! everyday hoop and its machine settings, and Save for My Machine (`embroidery.send`) writes its
//! file format to its folder or USB stick.

use serde_json::{Value, json};
use sf_core::machines::{HOME_MACHINES, HomeMachine, home_machine};

use super::{CommandSpec, Result, Session, always, bad, cmd};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "embroidery.myMachine",
            "My Machine",
            [],
            None,
            "{} the machine chosen as one's own, the machines to choose from and the folder or USB stick designs were last saved to → {machine: name | null, format (the one saved by default), nativeFormat, reads: [format] (native first), summary: '<machine>: saves PES' | null, hoops: [{name, width, height}] (mm, the everyday one first, then one's own), own: [{name, width, height, shape}] (the hoops of one's own, embroidery.hoop.saveCustom), folder: path | null, machines: [{name, format, reads}]}",
            always,
            get
        ),
        cmd!(
            "embroidery.myMachine.set",
            "Set My Machine",
            [],
            None,
            "{machine: a name from embroidery.myMachine's machines | null (none)} choose one's machine: new designs take its first hoop and its settings, and Save for My Machine writes its format → as embroidery.myMachine",
            always,
            set
        ),
        cmd!(
            "embroidery.myMachine.setFormat",
            "Set My Machine's File Format",
            [],
            None,
            "{format: one of embroidery.myMachine's reads | null (back to the recommended, native one)} choose the file format saved by default for the chosen machine (Save, Export, Save for My Machine) → as embroidery.myMachine",
            always,
            set_format
        ),
        cmd!(
            "embroidery.myMachine.setOrientation",
            "Set Hoop Orientation",
            [],
            None,
            "{rotation: 0|90|180|270 (degrees clockwise: how the arrow of the Hoop orientation test (embroidery.testSew) pointed when sewn), mirror?: bool (default false)} save how the hoop sits in the chosen machine: exports are turned by it → as embroidery.myMachine",
            always,
            set_orientation
        ),
    ]
}

/// The machine chosen as one's own, if any.
pub(super) fn mine(s: &Session) -> Option<&'static HomeMachine> {
    Some(s.prefs.my_machine.as_str()).filter(|n| !n.is_empty()).and_then(home_machine)
}

/// How the hoop sits in the chosen machine: (degrees clockwise, mirrored); (0, false) when unset.
pub(super) fn orientation(s: &Session) -> (u16, bool) {
    mine(s).and_then(|m| s.prefs.my_machine_orientation.get(m.name)).copied().unwrap_or((0, false))
}

/// `pattern` mirrored left to right, then turned `deg` clockwise (a multiple of 90) about its middle.
pub(super) fn orient(pattern: &sf_core::pattern::Pattern, deg: u16, mirror: bool) -> sf_core::pattern::Pattern {
    let mut q = pattern.clone();
    let b = q.bounds();
    let (cx, cy) = ((b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0);
    for st in &mut q.stitches {
        let (mut x, mut y) = (st.x - cx, st.y - cy);
        if mirror {
            x = -x;
        }
        // y points down, so (x, y) → (-y, x) is a quarter turn clockwise
        for _ in 0..(deg / 90) % 4 {
            (x, y) = (-y, x);
        }
        (st.x, st.y) = (cx + x, cy + y);
    }
    q
}

/// The format saved by default: the one chosen for the machine, else its native one.
pub(super) fn save_format(s: &Session) -> Option<String> {
    let m = mine(s)?;
    let chosen = s.prefs.my_machine_formats.get(m.name).filter(|f| m.reads.contains(&f.as_str()));
    Some(chosen.map_or(m.format, String::as_str).to_string())
}

fn get(s: &mut Session, _: &Value) -> Result<Value> {
    let m = mine(s);
    let mut hoops: Vec<Value> = m
        .map(|m| m.hoops)
        .unwrap_or_default()
        .iter()
        .filter_map(|name| super::embroidery_hoops::find_hoop(s, name))
        .map(|h| json!({ "name": h.name, "width": h.width, "height": h.height }))
        .collect();
    // hoops of one's own, after the machine's (when a machine is chosen)
    let own: Vec<Value> = s
        .prefs
        .embroidery_custom_hoops
        .iter()
        .map(|h| {
            let shape = if h.is_elliptical() { "oval" } else { "rect" };
            json!({ "name": h.name, "width": h.width, "height": h.height, "shape": shape, "cornerRadius": h.corner_radius, "frame": h.frame, "machines": h.machines })
        })
        .collect();
    if let Some(m) = m {
        // the ones of one's own put on this machine, and the ones put on none (on every machine)
        let on = |h: &&sf_core::hoops::Hoop| h.machines.is_empty() || h.machines.iter().any(|n| n == m.name);
        hoops.extend(s.prefs.embroidery_custom_hoops.iter().filter(on).map(|h| json!({ "name": h.name, "width": h.width, "height": h.height })));
    }
    Ok(json!({
        "machine": m.map(|m| m.name),
        "format": save_format(s),
        "nativeFormat": m.map(|m| m.format),
        "reads": m.map(|m| m.reads),
        "summary": m.zip(save_format(s)).map(|(m, f)| format!("{}: saves {}", m.name, f.to_ascii_uppercase())),
        "hoops": hoops,
        "own": own,
        "orientation": { "rotation": orientation(s).0, "mirror": orientation(s).1 },
        "folder": s.prefs.embroidery_send_dirs.first(),
        "machines": HOME_MACHINES.iter().map(|m| json!({ "name": m.name, "format": m.format, "reads": m.reads })).collect::<Vec<_>>(),
    }))
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.myMachine.set";
    match p.get("machine") {
        None => return Err(bad(CMD, "missing `machine`: a name from embroidery.myMachine, or null")),
        Some(Value::Null) => s.prefs.my_machine.clear(),
        Some(v) => {
            let name = v.as_str().ok_or_else(|| bad(CMD, "`machine` is a name or null"))?;
            let m = home_machine(name).ok_or_else(|| bad(CMD, format!("unknown machine `{name}`: see embroidery.myMachine")))?;
            s.prefs.my_machine = m.name.to_string();
        }
    }
    get(s, &json!({}))
}

fn set_format(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.myMachine.setFormat";
    let m = mine(s).ok_or_else(|| bad(CMD, "choose a machine first: embroidery.myMachine.set"))?;
    match p.get("format") {
        None => return Err(bad(CMD, "missing `format`: one of embroidery.myMachine's reads, or null")),
        Some(Value::Null) => {
            s.prefs.my_machine_formats.remove(m.name);
        }
        Some(v) => {
            let f = v.as_str().ok_or_else(|| bad(CMD, "`format` is a format or null"))?.trim().trim_start_matches('.').to_ascii_lowercase();
            if !m.reads.contains(&f.as_str()) {
                return Err(bad(CMD, format!("{} doesn't read `{f}`: one of {}", m.name, m.reads.join(", "))));
            }
            if f == m.format {
                s.prefs.my_machine_formats.remove(m.name);
            } else {
                s.prefs.my_machine_formats.insert(m.name.to_string(), f);
            }
        }
    }
    get(s, &json!({}))
}

fn set_orientation(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.myMachine.setOrientation";
    let m = mine(s).ok_or_else(|| bad(CMD, "choose a machine first: embroidery.myMachine.set"))?;
    let rotation = p
        .get("rotation")
        .and_then(Value::as_u64)
        .filter(|r| [0, 90, 180, 270].contains(r))
        .ok_or_else(|| bad(CMD, "`rotation` is 0, 90, 180 or 270"))?;
    let mirror = p.get("mirror").and_then(Value::as_bool).unwrap_or(false);
    if rotation == 0 && !mirror {
        s.prefs.my_machine_orientation.remove(m.name);
    } else {
        s.prefs.my_machine_orientation.insert(m.name.to_string(), (rotation as u16, mirror));
    }
    get(s, &json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The format chosen per machine is the default for export; reset puts the native one back.
    #[test]
    fn the_machine_format_can_be_chosen_and_reset() {
        let mut s = Session::new();
        assert!(s.execute("embroidery.myMachine.setFormat", &json!({"format": "dst"})).is_err());
        let r = s.execute("embroidery.myMachine.set", &json!({"machine": "Brother or Baby Lock"})).unwrap();
        assert_eq!((r["format"].as_str(), r["summary"].as_str()), (Some("pes"), Some("Brother or Baby Lock: saves PES")));
        assert_eq!(r["reads"], json!(["pes", "dst"]));
        assert!(s.execute("embroidery.myMachine.setFormat", &json!({"format": "jef"})).is_err());
        let r = s.execute("embroidery.myMachine.setFormat", &json!({"format": ".DST"})).unwrap();
        assert_eq!((r["format"].as_str(), r["nativeFormat"].as_str()), (Some("dst"), Some("pes")));
        s.execute("embroidery.newDesign", &json!({})).unwrap();
        s.execute("shape.rectangle", &json!({"x": 100, "y": 100, "width": 100, "height": 60})).unwrap();
        s.execute("embroidery.set", &json!({"type": "fill"})).unwrap();
        assert_eq!(s.execute("embroidery.export", &json!({})).unwrap()["format"], "dst");
        // kept per machine
        s.execute("embroidery.myMachine.set", &json!({"machine": "Bernina"})).unwrap();
        assert_eq!(s.execute("embroidery.myMachine", &json!({})).unwrap()["format"], "exp");
        s.execute("embroidery.myMachine.set", &json!({"machine": "Brother or Baby Lock"})).unwrap();
        assert_eq!(s.execute("embroidery.myMachine", &json!({})).unwrap()["format"], "dst");
        let r = s.execute("embroidery.myMachine.setFormat", &json!({"format": null})).unwrap();
        assert_eq!(r["format"], "pes");
    }

    /// E4: the machine chosen once sets new designs' hoop and machine, and the file format saved.
    #[test]
    fn my_machine_sets_the_hoop_the_settings_and_the_format() {
        let mut s = Session::new();
        let none = s.execute("embroidery.myMachine", &json!({})).unwrap();
        assert!(none["machine"].is_null() && none["machines"].as_array().unwrap().len() >= 5);
        // no hoop and no machine: the 5×7 most machines take
        assert_eq!(s.execute("embroidery.newDesign", &json!({})).unwrap()["hoop"], "Brother 130 × 180 (5×7)");
        let r = s.execute("embroidery.myMachine.set", &json!({"machine": "Husqvarna Viking Designer Diamond deLuxe"})).unwrap();
        assert_eq!(r["format"], "vp3");
        // the Crown hoop it comes with first
        assert_eq!(r["hoops"][0]["name"], "Husqvarna 260 × 200");
        // a hoop of one's own joins the machine's
        s.execute("embroidery.hoop.saveCustom", &json!({"name": "My quilting frame", "width": 300, "height": 250})).unwrap();
        let r = s.execute("embroidery.myMachine", &json!({})).unwrap();
        let names: Vec<&str> = r["hoops"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
        assert_eq!(names.last(), Some(&"My quilting frame"), "{names:?}");
        assert_eq!(r["own"][0]["width"], 300.0);
        s.execute("embroidery.hoop.deleteCustom", &json!({"name": "My quilting frame"})).unwrap();
        // a new design without a hoop: the machine's everyday hoop, its settings
        let d = s.execute("embroidery.newDesign", &json!({})).unwrap();
        assert_eq!(d["hoop"], "Husqvarna 260 × 200");
        s.execute("shape.rectangle", &json!({"x": 100, "y": 100, "width": 100, "height": 60})).unwrap();
        s.execute("embroidery.set", &json!({"type": "fill"})).unwrap();
        let out = s.execute("embroidery.export", &json!({})).unwrap();
        assert_eq!(out["format"], "vp3", "the machine's format");
        assert!(s.execute("embroidery.myMachine.set", &json!({"machine": "Sewing Wonder 3000"})).is_err());
        s.execute("embroidery.myMachine.set", &json!({"machine": null})).unwrap();
        assert!(s.execute("embroidery.myMachine", &json!({})).unwrap()["machine"].is_null());
    }

    /// The saved orientation turns every export; each offset rotates every stitch (DST, PES, JEF, EXP).
    #[test]
    fn exports_are_turned_by_the_saved_orientation() {
        use sf_formats::Format;
        let mut s = Session::new();
        s.execute("embroidery.myMachine.set", &json!({"machine": "Brother or Baby Lock"})).unwrap();
        assert!(s.execute("embroidery.myMachine.setOrientation", &json!({"rotation": 45})).is_err());
        s.execute("embroidery.testSew", &json!({"test": "orientation"})).unwrap();
        let read = |s: &mut Session, f: &str| {
            let r = s.execute("embroidery.export", &json!({"format": f})).unwrap();
            let bytes = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
            sf_formats::read(Format::from_extension(f).unwrap(), &bytes).unwrap()
        };
        for f in ["dst", "pes", "jef", "exp"] {
            let base = read(&mut s, f);
            for (rot, mirror) in [(0u16, false), (90, false), (180, false), (270, false), (0, true), (90, true)] {
                s.execute("embroidery.myMachine.setOrientation", &json!({"rotation": rot, "mirror": mirror})).unwrap();
                let got = read(&mut s, f);
                let want = orient(&base, rot, mirror);
                assert_eq!(got.stitches.len(), want.stitches.len(), "{f} {rot} {mirror}");
                let (bb, gb) = (base.bounds(), got.bounds());
                // a quarter turn swaps the width and the height (the formats keep 0.1 mm)
                let swap = rot % 180 == 90;
                let (bw, bh) = (bb.max.x - bb.min.x, bb.max.y - bb.min.y);
                let (gw, gh) = (gb.max.x - gb.min.x, gb.max.y - gb.min.y);
                let (ew, eh) = if swap { (bh, bw) } else { (bw, bh) };
                assert!((gw - ew).abs() < 3.0 && (gh - eh).abs() < 3.0, "{f} {rot} {mirror}: {gw}x{gh} vs {ew}x{eh}");
                // every stitch lands where the turn puts it (relative to the design's middle; 0.1 mm formats)
                let mid = |b: sf_core::geom::BBox| ((b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0);
                let (wm, gm) = (mid(want.bounds()), mid(gb));
                for (i, (g, w)) in got.stitches.iter().zip(&want.stitches).enumerate() {
                    let (dx, dy) = ((g.x - gm.0) - (w.x - wm.0), (g.y - gm.1) - (w.y - wm.1));
                    assert!(dx.abs() < 6.0 && dy.abs() < 6.0, "{f} {rot} {mirror} stitch {i}: off by {dx},{dy}");
                }
            }
            s.execute("embroidery.myMachine.setOrientation", &json!({"rotation": 0})).unwrap();
        }
        assert_eq!(s.execute("embroidery.myMachine", &json!({})).unwrap()["orientation"]["rotation"], 0);
    }
}
