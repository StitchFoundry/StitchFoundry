//! The scorecard measures patterns read from machine files the same way as our own output.
// Integration tests: unwrapping and panicking on failure is fine here, unlike in shipped code (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sf_core::attr::CheckLimits;
use sf_core::{Pattern, Point};
use sf_formats::{Format, WriteOptions, read_any, write};

#[test]
fn files_of_the_common_formats_score_like_the_pattern_they_hold() {
    let mut pat = Pattern::new();
    for i in 0..20 {
        pat.stitch(Point::new(f64::from(i) * 0.5, 0.0));
    }
    pat.trim();
    pat.jump(Point::new(30.0, 5.0));
    for i in 0..10 {
        pat.stitch(Point::new(30.0 + f64::from(i) * 3.0, 5.0));
    }
    pat.color_change();
    pat.jump(Point::new(30.0, 10.0));
    pat.stitch(Point::new(30.0, 10.0));
    pat.stitch(Point::new(35.0, 10.0));
    pat.ensure_end();
    let limits = CheckLimits::default();
    for format in [Format::Dst, Format::Exp, Format::Jef, Format::Pes] {
        let bytes = write(format, &pat, &WriteOptions::default()).unwrap();
        let (_, back) = read_any(&bytes, Some(format.extension())).unwrap();
        let v = sf_core::scorecard::score(&back, None, &limits);
        // writers may add a few stitches (the move from the origin, split long moves)
        let n = v["stitches"].as_u64().unwrap();
        assert!((32..=38).contains(&n), "{format:?}: {n}");
        assert_eq!(v["colorChanges"].as_u64(), Some(1), "{format:?}");
        assert_eq!(v["stitchLength"]["longerThan7Mm"].as_u64(), Some(0), "{format:?}");
        assert_eq!(v["density"]["regions"].as_array().map(Vec::len), Some(2), "{format:?}");
        assert!(v["sewSeconds"].as_f64().unwrap() > 0.0);
    }
}
