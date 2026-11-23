//! Named underlay presets for fills and satins: one pick sets the underlay flags, with a
//! one-line note on when to use it. Pure data and setters; the panel offers them, and the
//! values still go through `embroidery.setParams`.

use crate::model::{FillUnderlay, SatinUnderlay};

/// A preset for the underlay under a fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillUnderlayPreset {
    /// No underlay.
    None,
    /// A run just inside the outline.
    EdgeRun,
    /// Sparse tatami across the shape, at 90 degrees to the fill.
    Tatami90,
    /// An edge run, then the tatami over it.
    EdgeRunTatami,
}

impl FillUnderlayPreset {
    pub const ALL: [FillUnderlayPreset; 4] =
        [FillUnderlayPreset::None, FillUnderlayPreset::EdgeRun, FillUnderlayPreset::Tatami90, FillUnderlayPreset::EdgeRunTatami];

    pub fn id(self) -> &'static str {
        match self {
            FillUnderlayPreset::None => "none",
            FillUnderlayPreset::EdgeRun => "edge-run",
            FillUnderlayPreset::Tatami90 => "tatami-90",
            FillUnderlayPreset::EdgeRunTatami => "edge-run-tatami",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FillUnderlayPreset::None => "No underlay",
            FillUnderlayPreset::EdgeRun => "Edge run",
            FillUnderlayPreset::Tatami90 => "Tatami at 90 degrees",
            FillUnderlayPreset::EdgeRunTatami => "Edge run and tatami",
        }
    }

    /// When to use it.
    pub fn description(self) -> &'static str {
        match self {
            FillUnderlayPreset::None => "Dense or small fills on firm fabric, and fills sewn over other stitching.",
            FillUnderlayPreset::EdgeRun => "Small or thin fills on stable fabric: it sharpens the edge but adds little support.",
            FillUnderlayPreset::Tatami90 => "Medium fills: the rows cross the fill's rows and hold the fabric flat.",
            FillUnderlayPreset::EdgeRunTatami => "Large fills, knits and anything that stretches: the most support and the crispest edge.",
        }
    }

    /// The preset these flags match, if any (the angle offset must be 90 for a tatami preset).
    pub fn of(u: &FillUnderlay) -> Option<FillUnderlayPreset> {
        Self::ALL.into_iter().find(|p| {
            let (edge, tatami) = p.flags();
            u.edge_run == edge && u.tatami == tatami && (!tatami || (u.angle_offset - 90.0).abs() < 1e-6)
        })
    }

    fn flags(self) -> (bool, bool) {
        match self {
            FillUnderlayPreset::None => (false, false),
            FillUnderlayPreset::EdgeRun => (true, false),
            FillUnderlayPreset::Tatami90 => (false, true),
            FillUnderlayPreset::EdgeRunTatami => (true, true),
        }
    }

    /// Sets `u` to this preset; spacing, inset and stitch length stay as they are.
    pub fn apply(self, u: &mut FillUnderlay) {
        let (edge, tatami) = self.flags();
        u.edge_run = edge;
        u.tatami = tatami;
        if tatami {
            u.angle_offset = 90.0;
        }
    }
}

/// A preset for the underlay under a satin column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SatinUnderlayPreset {
    /// Chosen from each column's width.
    Auto,
    /// One run down the middle.
    CenterRun,
    /// A run along each edge.
    EdgeRun,
    /// A zigzag across the column.
    Zigzag,
    /// An edge run, then a zigzag over it.
    EdgeRunZigzag,
}

impl SatinUnderlayPreset {
    pub const ALL: [SatinUnderlayPreset; 5] = [
        SatinUnderlayPreset::Auto,
        SatinUnderlayPreset::CenterRun,
        SatinUnderlayPreset::EdgeRun,
        SatinUnderlayPreset::Zigzag,
        SatinUnderlayPreset::EdgeRunZigzag,
    ];

    pub fn id(self) -> &'static str {
        match self {
            SatinUnderlayPreset::Auto => "auto",
            SatinUnderlayPreset::CenterRun => "center-run",
            SatinUnderlayPreset::EdgeRun => "edge-run",
            SatinUnderlayPreset::Zigzag => "zigzag",
            SatinUnderlayPreset::EdgeRunZigzag => "edge-run-zigzag",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SatinUnderlayPreset::Auto => "Automatic",
            SatinUnderlayPreset::CenterRun => "Centre run",
            SatinUnderlayPreset::EdgeRun => "Edge run",
            SatinUnderlayPreset::Zigzag => "Zigzag",
            SatinUnderlayPreset::EdgeRunZigzag => "Edge run and zigzag",
        }
    }

    /// When to use it.
    pub fn description(self) -> &'static str {
        match self {
            SatinUnderlayPreset::Auto => "Lets each column's width choose, so a mixed design needs no thought.",
            SatinUnderlayPreset::CenterRun => "Narrow columns, up to about 3 mm: one run is enough to anchor them.",
            SatinUnderlayPreset::EdgeRun => "Medium columns: the edges stay crisp and the stitches sit up.",
            SatinUnderlayPreset::Zigzag => "Wide columns, about 4 mm and up: it stops long stitches sinking into the fabric.",
            SatinUnderlayPreset::EdgeRunZigzag => "Wide columns on knits, pile and towels: the most support.",
        }
    }

    /// The preset these flags match, if any.
    pub fn of(u: &SatinUnderlay) -> Option<SatinUnderlayPreset> {
        if u.auto {
            return Some(SatinUnderlayPreset::Auto);
        }
        Self::ALL.into_iter().skip(1).find(|p| p.flags() == Some((u.center_run, u.edge_run, u.zigzag)))
    }

    fn flags(self) -> Option<(bool, bool, bool)> {
        match self {
            SatinUnderlayPreset::Auto => None,
            SatinUnderlayPreset::CenterRun => Some((true, false, false)),
            SatinUnderlayPreset::EdgeRun => Some((false, true, false)),
            SatinUnderlayPreset::Zigzag => Some((false, false, true)),
            SatinUnderlayPreset::EdgeRunZigzag => Some((false, true, true)),
        }
    }

    /// Sets `u` to this preset; inset and spacings stay as they are.
    pub fn apply(self, u: &mut SatinUnderlay) {
        match self.flags() {
            None => u.auto = true,
            Some((center, edge, zigzag)) => {
                u.auto = false;
                u.center_run = center;
                u.edge_run = edge;
                u.zigzag = zigzag;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_presets_round_trip() {
        for p in FillUnderlayPreset::ALL {
            let mut u = FillUnderlay { angle_offset: 30.0, ..FillUnderlay::default() };
            p.apply(&mut u);
            assert_eq!(FillUnderlayPreset::of(&u), Some(p), "{}", p.id());
        }
    }

    #[test]
    fn fill_tatami_is_at_ninety_degrees_and_keeps_spacing() {
        let mut u = FillUnderlay { angle_offset: 0.0, spacing: 4.0, ..FillUnderlay::default() };
        FillUnderlayPreset::Tatami90.apply(&mut u);
        assert!(u.tatami && !u.edge_run);
        assert_eq!(u.angle_offset, 90.0);
        assert_eq!(u.spacing, 4.0);
    }

    #[test]
    fn a_tatami_at_another_angle_matches_no_preset() {
        let u = FillUnderlay { tatami: true, edge_run: false, angle_offset: 45.0, ..FillUnderlay::default() };
        assert_eq!(FillUnderlayPreset::of(&u), None);
    }

    #[test]
    fn satin_presets_round_trip() {
        for p in SatinUnderlayPreset::ALL {
            let mut u = SatinUnderlay::default();
            p.apply(&mut u);
            assert_eq!(SatinUnderlayPreset::of(&u), Some(p), "{}", p.id());
        }
    }

    #[test]
    fn satin_zigzag_turns_automatic_off_and_keeps_spacing() {
        let mut u = SatinUnderlay { zigzag_spacing: 3.0, ..SatinUnderlay::default() };
        SatinUnderlayPreset::Zigzag.apply(&mut u);
        assert!(!u.auto && u.zigzag && !u.center_run && !u.edge_run);
        assert_eq!(u.zigzag_spacing, 3.0);
    }

    #[test]
    fn every_preset_has_a_label_and_a_one_line_description() {
        let mut ids = std::collections::HashSet::new();
        for (id, label, d) in FillUnderlayPreset::ALL
            .iter()
            .map(|p| (p.id(), p.label(), p.description()))
            .chain(SatinUnderlayPreset::ALL.iter().map(|p| (p.id(), p.label(), p.description())))
        {
            assert!(!label.is_empty() && !d.is_empty() && !d.contains('\n'), "{id}");
            ids.insert(id);
        }
        assert!(ids.len() >= 7);
    }

    #[test]
    fn sewing_advice_is_stated_plainly() {
        // the guidance names what the preset is for, not a product
        for p in SatinUnderlayPreset::ALL {
            assert!(p.description().ends_with('.'), "{}", p.id());
        }
        assert!(SatinUnderlayPreset::Zigzag.description().contains("Wide"));
        assert!(SatinUnderlayPreset::CenterRun.description().contains("Narrow"));
    }
}
