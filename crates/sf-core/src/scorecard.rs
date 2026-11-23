//! Digitizer scorecard, slice 1: measurable properties of a stitch pattern as one JSON object.
//!
//! Works on any [`Pattern`]: our own output or a machine file read by `sf-formats` (DST, PES,
//! JEF, EXP, ...). It only measures; it says nothing about which system is better. What a
//! bare pattern cannot tell (underlay, satin width: the file keeps only needle points) comes
//! from the design's objects when they are given, and is `null` otherwise.

use crate::attr::CheckLimits;
use crate::density::LayerGrid;
use crate::model::{Design, DesignSettings, ObjectKind};
use crate::pattern::{Cmd, Pattern};
use crate::stats::pattern_stats;
use serde_json::{Value, json};

/// Machine speed the sew-time estimate assumes (stitches per minute).
pub const SEW_SPM: f64 = 800.0;
/// Stitches shorter than this (mm) count as short.
pub const SHORT_MM: f64 = 0.4;
/// Stitches longer than this (mm) count as long.
pub const LONG_MM: f64 = 7.0;
/// A satin column is in range from this width (mm) ...
pub const SATIN_MIN_MM: f64 = 1.0;
/// ... to this width (mm).
pub const SATIN_MAX_MM: f64 = 7.0;
/// Side of a density cell (mm).
const CELL: f64 = 1.0;

/// What the design's objects know that the stitches alone don't.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectFacts {
    /// Whether the object sews underlay.
    pub underlay: bool,
    /// Average column width, for satin columns.
    pub satin_width_mm: Option<f64>,
}

/// The facts of every satin, fill and appliqué object in `design`.
pub fn object_facts(design: &Design) -> Vec<ObjectFacts> {
    design
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Satin(s) => {
                let u = &s.params.underlay;
                let w = s.column.average_width();
                Some(ObjectFacts {
                    underlay: u.auto || u.center_run || u.edge_run || u.zigzag || u.double_zigzag,
                    satin_width_mm: w.is_finite().then_some(w),
                })
            }
            ObjectKind::Fill(f) => Some(ObjectFacts { underlay: f.params.underlay.edge_run || f.params.underlay.tatami, satin_width_mm: None }),
            _ => None,
        })
        .collect()
}

/// The scorecard of `pattern`: `objects` are the design's [`object_facts`] when known.
/// `limits` set what counts as a hot spot (`dense_layers`) and the machine-check counts.
pub fn score(pattern: &Pattern, objects: Option<&[ObjectFacts]>, limits: &CheckLimits) -> Value {
    let settings = DesignSettings { machine_speed: SEW_SPM, ..DesignSettings::default() };
    let st = pattern_stats(pattern, &settings);

    // Needle to needle while the thread is attached; records that aren't finite are counted and skipped.
    let (mut non_finite, mut measured, mut short, mut long, mut too_short, mut too_long) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut needle = None;
    for s in &pattern.stitches {
        if s.is_move() && !(s.x.is_finite() && s.y.is_finite()) {
            non_finite += 1;
            continue;
        }
        match s.cmd {
            Cmd::Stitch => {
                if let Some(p) = needle {
                    let len = s.pos().distance(p);
                    measured += 1;
                    short += usize::from(len < SHORT_MM);
                    long += usize::from(len > LONG_MM);
                    too_short += usize::from(len < limits.short_stitch);
                    too_long += usize::from(len > limits.long_stitch);
                }
                needle = Some(s.pos());
            }
            Cmd::Trim | Cmd::ColorChange | Cmd::End => needle = None,
            Cmd::Jump | Cmd::Stop => {}
        }
    }
    let share = |n: usize| if measured == 0 { 0.0 } else { n as f64 / measured as f64 };

    // Density per colour region: the layers of thread on 1 mm cells that carry thread.
    let mut regions = Vec::new();
    let mut dense_cells = 0usize;
    let mut block = Pattern::new();
    let mut index = 0usize;
    let flush = |block: &mut Pattern, index: &mut usize, regions: &mut Vec<Value>, dense: &mut usize| {
        if block.stitches.iter().any(|s| s.cmd == Cmd::Stitch) {
            let grid = LayerGrid::of(block, CELL);
            let used: Vec<f64> = grid.layers.iter().map(|&l| f64::from(l)).filter(|l| *l > 0.0).collect();
            let n = used.len().max(1) as f64;
            let mean = used.iter().sum::<f64>() / n;
            let spread = (used.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / n).sqrt();
            let hot = used.iter().filter(|&&l| l >= limits.dense_layers).count();
            *dense += hot;
            regions.push(json!({
                "region": *index,
                "areaMm2": used.len() as f64 * grid.cell * grid.cell,
                "meanLayers": mean,
                "spreadLayers": spread,
                "maxLayers": used.iter().copied().fold(0.0, f64::max),
                "hotCells": hot,
            }));
        }
        *index += 1;
        *block = Pattern::new();
    };
    for s in &pattern.stitches {
        if s.cmd == Cmd::ColorChange {
            flush(&mut block, &mut index, &mut regions, &mut dense_cells);
        } else {
            block.stitches.push(*s);
        }
    }
    flush(&mut block, &mut index, &mut regions, &mut dense_cells);

    let satin: Vec<f64> = objects.unwrap_or_default().iter().filter_map(|o| o.satin_width_mm).collect();
    let satin_json = if objects.is_none() {
        Value::Null
    } else if satin.is_empty() {
        json!({ "count": 0 })
    } else {
        let outside = satin.iter().filter(|&&w| !(SATIN_MIN_MM..=SATIN_MAX_MM).contains(&w)).count();
        json!({
            "count": satin.len(),
            "minMm": satin.iter().copied().fold(f64::INFINITY, f64::min),
            "maxMm": satin.iter().copied().fold(0.0, f64::max),
            "shareOutside1To7Mm": outside as f64 / satin.len() as f64,
        })
    };
    let underlay_json = objects.map_or(Value::Null, |o| {
        json!({ "objects": o.len(), "withUnderlay": o.iter().filter(|f| f.underlay).count(), "perObject": o.iter().map(|f| f.underlay).collect::<Vec<_>>() })
    });

    json!({
        "stitches": st.stitches,
        "jumps": st.jumps,
        "trims": st.trims,
        "colorChanges": st.color_changes,
        "stitchLength": {
            "measured": measured,
            "shorterThan0_4Mm": short,
            "longerThan7Mm": long,
            "shareShorterThan0_4Mm": share(short),
            "shareLongerThan7Mm": share(long),
        },
        "density": { "cellMm": CELL, "hotLayers": limits.dense_layers, "hotCells": dense_cells, "regions": regions },
        "underlay": underlay_json,
        "satinWidth": satin_json,
        "sewSeconds": st.seconds,
        "sewSecondsAtSpm": SEW_SPM,
        "machineCheck": {
            "shortStitches": too_short,
            "longStitches": too_long,
            "denseCells": dense_cells,
            "nonFiniteRecords": non_finite,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// 4 stitches: 0.2 mm, 3 mm, 8 mm; one jump, one trim, one colour change, then 2 more.
    fn tiny() -> Pattern {
        let mut pat = Pattern::new();
        pat.stitch(p(0.0, 0.0));
        pat.stitch(p(0.2, 0.0));
        pat.stitch(p(3.2, 0.0));
        pat.stitch(p(11.2, 0.0));
        pat.trim();
        pat.jump(p(20.0, 0.0));
        pat.stitch(p(20.0, 0.0));
        pat.color_change();
        pat.stitch(p(20.0, 1.0));
        pat.stitch(p(20.0, 3.0));
        pat.ensure_end();
        pat
    }

    #[test]
    fn counts_and_stitch_lengths() {
        let v = score(&tiny(), None, &CheckLimits::default());
        assert_eq!(
            (v["stitches"].as_u64(), v["jumps"].as_u64(), v["trims"].as_u64(), v["colorChanges"].as_u64()),
            (Some(7), Some(1), Some(1), Some(1))
        );
        let l = &v["stitchLength"];
        assert_eq!((l["measured"].as_u64(), l["shorterThan0_4Mm"].as_u64(), l["longerThan7Mm"].as_u64()), (Some(4), Some(1), Some(1)));
        assert_eq!(l["shareLongerThan7Mm"].as_f64(), Some(0.25));
        // 7 stitches at 800 per minute, plus the trim and the colour change
        let secs = v["sewSeconds"].as_f64().unwrap();
        assert!(secs > 7.0 / 800.0 * 60.0 && secs < 60.0, "{secs}");
        assert_eq!(v["machineCheck"]["shortStitches"].as_u64(), Some(1));
        assert_eq!(v["machineCheck"]["longStitches"].as_u64(), Some(1));
        assert!(v["underlay"].is_null() && v["satinWidth"].is_null());
    }

    #[test]
    fn density_is_split_by_colour_with_hot_spots() {
        let mut pat = Pattern::new();
        // colour 1: 40 passes over the same 4 mm line: far above any dense limit
        for i in 0..40 {
            pat.stitch(p(if i % 2 == 0 { 0.0 } else { 4.0 }, 0.0));
        }
        pat.color_change();
        pat.stitch(p(0.0, 10.0));
        pat.stitch(p(4.0, 10.0));
        let v = score(&pat, None, &CheckLimits::default());
        let r = v["density"]["regions"].as_array().unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[0]["hotCells"].as_u64().unwrap() > 0);
        assert_eq!(r[1]["hotCells"].as_u64(), Some(0));
        assert!(r[0]["meanLayers"].as_f64().unwrap() > r[1]["meanLayers"].as_f64().unwrap());
        assert_eq!(v["machineCheck"]["denseCells"], v["density"]["hotCells"]);
    }

    #[test]
    fn objects_give_underlay_and_satin_widths() {
        let facts = [
            ObjectFacts { underlay: true, satin_width_mm: Some(3.0) },
            ObjectFacts { underlay: false, satin_width_mm: Some(9.0) },
            ObjectFacts { underlay: true, satin_width_mm: None },
        ];
        let v = score(&tiny(), Some(&facts), &CheckLimits::default());
        assert_eq!(v["underlay"]["withUnderlay"].as_u64(), Some(2));
        assert_eq!(v["underlay"]["perObject"], json!([true, false, true]));
        let s = &v["satinWidth"];
        assert_eq!(
            (s["count"].as_u64(), s["minMm"].as_f64(), s["maxMm"].as_f64(), s["shareOutside1To7Mm"].as_f64()),
            (Some(2), Some(3.0), Some(9.0), Some(0.5))
        );
    }

    #[test]
    fn empty_and_non_finite_patterns_do_not_panic() {
        let v = score(&Pattern::new(), Some(&[]), &CheckLimits::default());
        assert_eq!(v["stitches"].as_u64(), Some(0));
        assert_eq!(v["density"]["regions"], json!([]));
        let mut pat = Pattern::new();
        pat.stitch(p(0.0, 0.0));
        pat.stitch(p(f64::NAN, 1.0));
        pat.stitch(p(1.0, 0.0));
        let v = score(&pat, None, &CheckLimits::default());
        assert_eq!(v["machineCheck"]["nonFiniteRecords"].as_u64(), Some(1));
        assert_eq!(v["stitchLength"]["measured"].as_u64(), Some(1));
    }
}
