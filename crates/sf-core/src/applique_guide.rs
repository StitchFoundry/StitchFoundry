//! Appliqué guidance: the sewing steps (placement line, tack-down, cover) in order, and checks
//! that the cut fabric is big enough for the tack-down to catch it and that the cover hides the
//! tack-down. Wording is plain sewing advice; the numbers come from the piece's own settings.

use crate::geom::{Polygon, poly};
use crate::model::{AppliqueParams, CoverKind, TackdownKind};

/// How much attention a note needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// A step, in sewing order.
    Step,
    /// Something likely to sew badly.
    Warning,
}

/// One line of guidance.
#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
    pub level: Level,
    pub text: String,
}

fn step(text: impl Into<String>) -> Advice {
    Advice { level: Level::Step, text: text.into() }
}

fn warn(text: impl Into<String>) -> Advice {
    Advice { level: Level::Warning, text: text.into() }
}

/// Guidance for an appliqué whose cut shape is `base` (the outline the placement line sews).
pub fn advise(base: &[Polygon], p: &AppliqueParams) -> Vec<Advice> {
    let mut out = Vec::new();
    let wide = matches!(p.tackdown, TackdownKind::Zigzag | TackdownKind::Blanket);
    // how far in from the cut edge the tack-down stitches reach
    let half = if wide { p.tackdown_width.max(0.0) / 2.0 } else { 0.0 };
    let inset = p.tackdown_inset.max(0.0);
    let reach = inset + half;

    if p.placement {
        out.push(step("1. Placement line: sew it, then lay the fabric over it, covering the whole line."));
    } else {
        out.push(warn("No placement line: there is nothing to line the fabric up with. Turn Placement on."));
    }
    if p.tackdown == TackdownKind::None {
        out.push(warn("No tack-down: the fabric is not held while the cover sews and can shift. Turn a tack-down on."));
    } else {
        out.push(step("2. Tack-down: sew it, then trim the fabric close to the stitching."));
        if half > inset + 1e-6 {
            out.push(warn(format!(
                "The tack-down is {:.1} mm wide but sits only {:.1} mm inside the edge, so its outer stitches fall off the fabric. \
                 Move it in to at least {:.1} mm or make it narrower.",
                p.tackdown_width, inset, half
            )));
        }
        if !p.trim_stop {
            out.push(warn("No stop after the tack-down: the machine goes straight on to the cover, so excess fabric stays on."));
        }
    }
    if p.cover == CoverKind::None {
        out.push(warn("No cover stitch: the fabric's raw edge stays open and may fray."));
    } else {
        out.push(step("3. Cover: sews over the trimmed edge."));
        if p.tackdown != TackdownKind::None && reach > p.cover_width / 2.0 + 1e-6 {
            out.push(warn(format!(
                "The tack-down reaches {:.1} mm in from the edge but the cover only reaches {:.1} mm, so the tack-down shows. \
                 Make the cover wider or the tack-down closer to the edge.",
                reach,
                p.cover_width / 2.0
            )));
        }
    }
    // the cut shape must be wide enough that the tack-down has fabric to catch on every side
    if p.tackdown != TackdownKind::None && !base.is_empty() && poly::offset(base, -(reach + 1.0)).is_empty() {
        out.push(warn(format!(
            "The piece is too small for the tack-down to catch the fabric: it needs to be more than {:.1} mm across. \
             Make the piece bigger or the tack-down narrower.",
            2.0 * (reach + 1.0)
        )));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Point, Shape};

    fn base(w: f64) -> Vec<Polygon> {
        vec![Shape::ellipse(Point::new(0.0, 0.0), w / 2.0, w / 2.0).polygon(0.05)]
    }

    fn warnings(a: &[Advice]) -> Vec<&str> {
        a.iter().filter(|a| a.level == Level::Warning).map(|a| a.text.as_str()).collect()
    }

    #[test]
    fn default_settings_on_a_normal_piece_give_the_three_steps_and_no_warnings() {
        let a = advise(&base(40.0), &AppliqueParams::default());
        assert!(warnings(&a).is_empty(), "{a:?}");
        assert_eq!(a.iter().filter(|a| a.level == Level::Step).count(), 3);
    }

    #[test]
    fn a_piece_too_small_for_the_tackdown_is_flagged() {
        let a = advise(&base(2.0), &AppliqueParams::default());
        assert!(warnings(&a).iter().any(|w| w.contains("too small")), "{a:?}");
    }

    #[test]
    fn a_tackdown_wider_than_its_inset_or_showing_past_the_cover_is_flagged() {
        let spill = AppliqueParams { tackdown_width: 3.0, tackdown_inset: 0.4, ..AppliqueParams::default() };
        assert!(warnings(&advise(&base(40.0), &spill)).iter().any(|w| w.contains("fall off the fabric")));
        let shows = AppliqueParams { cover_width: 1.0, ..AppliqueParams::default() };
        assert!(warnings(&advise(&base(40.0), &shows)).iter().any(|w| w.contains("tack-down shows")));
    }

    #[test]
    fn missing_steps_are_flagged() {
        let p = AppliqueParams { placement: false, tackdown: TackdownKind::None, cover: CoverKind::None, ..AppliqueParams::default() };
        assert_eq!(warnings(&advise(&base(40.0), &p)).len(), 3);
    }
}
