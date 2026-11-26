//! Built-in thread charts of thread makers (issue 212), one module per maker.
//!
//! Each maker's chart lives in its own file (`thread_makers/<maker>.rs`) and is registered in
//! [`MAKERS`], so one maker's chart can be pulled in a small change. [`ENABLED`] is the single
//! config list of the charts that are shown: take a name out of it to hide that chart from the
//! pickers, matching and export without touching the data. Every chart's source is recorded in
//! `docs/thread-charts.md`.

use crate::threads::ThreadChart;

/// Shown wherever a chart is chosen, and in the docs.
pub const NOTICE: &str = "Thread colour data is approximate and for matching only. Not affiliated with or endorsed by the thread makers.";

/// Shown with [`NOTICE`]: screen colours are not thread colours.
pub const SEW_ADVICE: &str = "Test-sew on scrap before you sew the design.";

/// A maker's chart: its name and the function that builds it.
pub type Maker = (&'static str, fn() -> ThreadChart);

/// Every maker chart in the build (none yet: each is added with its own source row).
pub const MAKERS: &[Maker] = &[];

/// The maker charts that are shown (names from [`MAKERS`]). Remove one to hide it.
pub const ENABLED: &[&str] = &[];

/// The charts of `makers` whose name is in `enabled`, in the order of `makers`.
pub fn enabled_from(makers: &[Maker], enabled: &[&str]) -> Vec<ThreadChart> {
    makers.iter().filter(|(name, _)| enabled.contains(name)).map(|(_, build)| build()).collect()
}

/// The enabled maker charts.
pub fn charts() -> Vec<ThreadChart> {
    enabled_from(MAKERS, ENABLED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threads::{Rgb, Thread};

    fn sample() -> ThreadChart {
        ThreadChart {
            name: "Sample".into(),
            threads: vec![
                Thread::with_code(Rgb::new(255, 0, 0), "Red", "1", "Sample"),
                Thread::with_code(Rgb::new(0, 0, 255), "Blue", "2", "Sample"),
            ],
        }
    }

    #[test]
    fn hiding_a_chart_removes_it() {
        let makers: &[Maker] = &[("Sample", sample)];
        assert_eq!(enabled_from(makers, &["Sample"]).len(), 1);
        assert!(enabled_from(makers, &[]).is_empty());
    }

    #[test]
    fn every_registered_chart_is_valid() {
        for (name, build) in MAKERS {
            let c = build();
            assert_eq!(&c.name, name);
            let mut codes = std::collections::HashSet::new();
            for t in &c.threads {
                assert!(!t.code.is_empty() && !t.name.is_empty(), "{name}: entry without code or name");
                assert!(codes.insert(t.code.clone()), "{name}: duplicate code {}", t.code);
            }
        }
        for e in ENABLED {
            assert!(MAKERS.iter().any(|(n, _)| n == e), "enabled chart {e} is not registered");
        }
    }

    #[test]
    fn nearest_picks_a_neighbour() {
        let c = sample();
        assert_eq!(c.threads[c.nearest(Rgb::new(200, 30, 30))].name, "Red");
    }
}
