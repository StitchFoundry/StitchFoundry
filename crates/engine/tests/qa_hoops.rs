//! Add-on 40 QA matrix, slice 1: every hoop in the library × (rotate 90/180/270/45, flip H/V, scale
//! ±10/±20%), checking that the design stays inside the sewing field and that the DST and PES files
//! written for it read back. `cargo xtask qa-hoops` runs it (the full matrix, and the report in
//! `docs/qa/addon-40-report.md`); `cargo xtask ci` runs the fast subset (3 hoops, the default here; `QA_HOOPS_FULL=1` is the full matrix).
// Integration tests: unwrapping and panicking on failure is fine here (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use sf_core::hoops::{Hoop, standard_hoops};
use vectorcraft_engine::Session;

/// A transform of the matrix: the command and its params.
fn transforms() -> Vec<(String, &'static str, Value)> {
    let mut v = Vec::new();
    for d in [90, 180, 270, 45] {
        v.push((format!("rotate {d}"), "embroidery.quick.rotate", json!({ "degrees": d })));
    }
    for a in ["horizontal", "vertical"] {
        v.push((format!("flip {}", &a[..1].to_uppercase()), "embroidery.quick.flip", json!({ "axis": a })));
    }
    for p in [90, 110, 80, 120] {
        v.push((format!("scale {p}%"), "embroidery.quick.resize", json!({ "percent": p })));
    }
    v
}

/// One matrix cell: the hoop with a square design half its smaller side, transformed and exported.
/// → the failures (empty when it passes) for the field check, then each format's round trip.
fn cell(hoop: &Hoop, cmd: &str, params: &Value) -> Result<Vec<(String, String)>, String> {
    let mut s = Session::new();
    let side = hoop.width.min(hoop.height) * 0.5;
    s.execute("embroidery.newDesign", &json!({ "hoop": hoop.name })).map_err(|e| format!("newDesign: {e}"))?;
    s.execute("shape.rectangle", &json!({ "x": 0, "y": 0, "width": side, "height": side })).map_err(|e| format!("rectangle: {e}"))?;
    s.execute("select.all", &json!({})).map_err(|e| e.to_string())?;
    s.execute("embroidery.set", &json!({ "type": "fill" })).map_err(|e| format!("set: {e}"))?;
    s.execute("select.none", &json!({})).map_err(|e| e.to_string())?;
    s.execute(cmd, params).map_err(|e| format!("{cmd}: {e}"))?;
    let mut out = Vec::new();
    let mut sizes = Vec::new();
    for fmt in ["dst", "pes"] {
        let r = s.execute("embroidery.export", &json!({ "format": fmt })).map_err(|e| format!("export {fmt}: {e}"))?;
        let bytes = r["dataBase64"].as_str().and_then(vectorcraft_format::base64_decode).ok_or("export gave no data")?;
        match sf_formats::read_any(&bytes, Some(fmt)) {
            Err(e) => out.push((format!("{fmt} round trip"), format!("read back failed: {e}"))),
            Ok((_, p)) if p.stitch_count() == 0 => out.push((format!("{fmt} round trip"), "read back with no stitches".into())),
            Ok((_, p)) => {
                let b = p.bounds();
                sizes.push((fmt, b.width(), b.height()));
                // the file is centred, so the field check is on a box about the hoop's middle
                let c = sf_core::Point::new((b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0);
                let centred =
                    sf_core::BBox::new(sf_core::Point::new(b.min.x - c.x, b.min.y - c.y), sf_core::Point::new(b.max.x - c.x, b.max.y - c.y));
                if !hoop.fits(&centred) {
                    out.push((
                        format!("{fmt} inside field"),
                        format!("{:.1} x {:.1} mm in a {} x {} mm field", b.width(), b.height(), hoop.width, hoop.height),
                    ));
                }
            }
        }
    }
    // both formats hold the same design to within a millimetre or so
    if let [(_, w0, h0), (_, w1, h1)] = sizes[..]
        && ((w0 - w1).abs() > 2.0 || (h0 - h1).abs() > 2.0)
    {
        out.push(("dst/pes agree".into(), format!("{w0:.1} x {h0:.1} vs {w1:.1} x {h1:.1} mm")));
    }
    Ok(out)
}

#[test]
fn hoop_matrix() {
    let fast = std::env::var_os("QA_HOOPS_FULL").is_none();
    let mut hoops = standard_hoops();
    if fast {
        let n = hoops.len();
        hoops = [0, n / 2, n.saturating_sub(1)].iter().filter_map(|&i| hoops.get(i).cloned()).collect();
    }
    let (mut pass, mut failures) = (0usize, Vec::<String>::new());
    // per check: (passed, failed)
    let mut by_check: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    for h in &hoops {
        for (label, cmd, params) in transforms() {
            let checks = ["dst inside field", "pes inside field", "dst round trip", "pes round trip", "dst/pes agree"];
            let found = match cell(h, cmd, &params) {
                Ok(f) => f,
                Err(e) => vec![("setup".to_string(), e)],
            };
            for c in checks {
                let bad = found.iter().any(|(k, _)| k == c || k == "setup");
                let e = by_check.entry(c.to_string()).or_default();
                if bad { e.1 += 1 } else { e.0 += 1 }
            }
            if found.is_empty() {
                pass += 1;
            }
            for (k, why) in found {
                failures.push(format!("- `{}` ({} x {} mm), {label}, {k}: {why}. Repro: `embroidery.newDesign {{hoop: \"{}\"}}`, a square of {:.1} mm filled, `{cmd} {params}`, `embroidery.export`.", h.name, h.width, h.height, h.name, h.width.min(h.height) * 0.5));
            }
        }
    }
    let total = hoops.len() * transforms().len();
    let verdict = if failures.is_empty() { "PASS" } else { "FAIL" };
    if let Ok(path) = std::env::var("QA_REPORT") {
        let mut r = format!("# Add-on 40 QA report\n\n**Result: {verdict}** ({pass} of {total} matrix cells clean; slice 1 of the release gate)\n\n");
        r.push_str("Generated by `cargo xtask qa-hoops`; do not edit by hand.\n\n");
        r.push_str(&format!("## Hoop matrix: {} hoops × {} transforms\n\nEach cell: a filled square half the hoop's smaller side, transformed by a quick command, exported as DST and PES and read back.\n\n| Check | Pass | Fail |\n|---|---|---|\n", hoops.len(), transforms().len()));
        for (k, (p, f)) in &by_check {
            r.push_str(&format!("| {k} | {p} | {f} |\n"));
        }
        r.push_str("\n### Failures\n\n");
        if failures.is_empty() {
            r.push_str("None.\n");
        }
        for f in &failures {
            r.push_str(f);
            r.push('\n');
        }
        r.push_str("\n### Not yet covered\n\nOther export formats, golden files, overlay checks, persistence, resize-on-switch checks, visual regression and the list of unverified hoop sizes (slices 2 and 3).\n");
        std::fs::write(path, r).unwrap();
    }
    assert!(failures.is_empty(), "{} of {total} cells failed:\n{}", failures.len(), failures.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}
