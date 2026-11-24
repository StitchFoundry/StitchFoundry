//! Timelapse clip maths (issue 218, slice 1): how long the clip is, how many frames it has, how
//! many needle points each frame adds, and where the design sits on a vertical (9:16), square or
//! as-is canvas. Pure arithmetic with no dependencies, so the same file can be copied to the
//! sibling app. Nothing here draws or encodes; nothing calls it yet.

/// Default clip length in seconds.
pub const DEFAULT_SECONDS: f64 = 15.0;
/// Default frames a second.
pub const DEFAULT_FPS: u32 = 24;
const MAX_SECONDS: f64 = 60.0;
const MAX_FPS: u32 = 60;

/// The clip's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aspect {
    /// The design's own proportions.
    AsIs,
    /// 9:16, 1080 × 1920 (Reels, Shorts).
    Vertical,
    /// 1:1, 1080 × 1080.
    Square,
}

/// Frame count and stepping for a clip.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// Frames in the clip (sewing frames plus the closing hold).
    pub frames: usize,
    /// Frames that show sewing in progress.
    pub sewing_frames: usize,
    /// Needle points added between one sewing frame and the next (at least 1).
    pub points_per_frame: usize,
}

/// The plan for `points` needle points in `seconds` at `fps`, the finished design held for the
/// last `hold` seconds. `None` when nothing is stitched or a number is out of range or not finite.
pub fn plan(points: usize, seconds: f64, hold: f64, fps: u32) -> Option<Plan> {
    let ok = |v: f64, hi: f64| v.is_finite() && (0.0..=hi).contains(&v);
    if points == 0 || fps == 0 || fps > MAX_FPS || !ok(seconds, MAX_SECONDS) || !ok(hold, seconds) {
        return None;
    }
    let total = (seconds * f64::from(fps)).round() as usize;
    let hold_frames = (hold * f64::from(fps)).round() as usize;
    let sewing_frames = total.checked_sub(hold_frames)?.max(1);
    // every sewing frame adds the same share of the points, the last frame reaching the end
    let points_per_frame = points.div_ceil(sewing_frames).max(1);
    Some(Plan { frames: sewing_frames + hold_frames, sewing_frames, points_per_frame })
}

/// Needle points drawn by sewing frame `i` (0-based), capped at `points`.
pub fn points_at(plan: &Plan, i: usize, points: usize) -> usize {
    if i + 1 >= plan.sewing_frames { points } else { (i + 1).saturating_mul(plan.points_per_frame).min(points) }
}

/// Clip length in seconds for `frames` at `fps` (0 when `fps` is 0).
pub fn length_seconds(frames: usize, fps: u32) -> f64 {
    if fps == 0 { 0.0 } else { frames as f64 / f64::from(fps) }
}

/// Where the design sits on the canvas (pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Canvas width and height.
    pub canvas: (u32, u32),
    /// The design's drawn width and height.
    pub design: (u32, u32),
    /// The design's top-left corner.
    pub origin: (u32, u32),
}

/// The canvas in pixels, and the design's drawn size and top-left corner on it (centred, scaled to
/// fit inside `margin` pixels, never enlarged past `max_scale` pixels a mm). `None` for a design
/// with no size or bad numbers. `AsIs` makes the canvas the design plus the margin.
pub fn layout(aspect: Aspect, width_mm: f64, height_mm: f64, margin: u32, max_scale: f64) -> Option<Layout> {
    if ![width_mm, height_mm, max_scale].iter().all(|v| v.is_finite() && *v > 0.0) {
        return None;
    }
    let (cw, ch) = match aspect {
        Aspect::Vertical => (1080, 1920),
        Aspect::Square => (1080, 1080),
        Aspect::AsIs => {
            let k = max_scale.min(1080.0 / width_mm.max(height_mm));
            let side = |mm: f64| (mm * k).ceil() as u32 + 2 * margin;
            (side(width_mm), side(height_mm))
        }
    };
    let (aw, ah) = (cw.checked_sub(2 * margin)?, ch.checked_sub(2 * margin)?);
    if aw == 0 || ah == 0 {
        return None;
    }
    let k = (f64::from(aw) / width_mm).min(f64::from(ah) / height_mm).min(max_scale);
    let (dw, dh) = (((width_mm * k).floor() as u32).clamp(1, aw), ((height_mm * k).floor() as u32).clamp(1, ah));
    Some(Layout { canvas: (cw, ch), design: (dw, dh), origin: ((cw - dw) / 2, (ch - dh) / 2) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifteen_seconds_is_the_default_length() {
        let p = plan(9000, DEFAULT_SECONDS, 2.0, DEFAULT_FPS).unwrap();
        assert_eq!(p.frames, 360);
        assert_eq!(p.sewing_frames, 312);
        assert!((length_seconds(p.frames, DEFAULT_FPS) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn frames_step_evenly_and_end_on_the_whole_design() {
        let points = 1000;
        let p = plan(points, 10.0, 1.0, 10).unwrap();
        assert_eq!((p.frames, p.sewing_frames, p.points_per_frame), (100, 90, 12));
        let counts: Vec<_> = (0..p.sewing_frames).map(|i| points_at(&p, i, points)).collect();
        assert!(counts.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(counts[0], 12);
        assert_eq!(*counts.last().unwrap(), points);
    }

    #[test]
    fn a_tiny_design_still_steps_by_one_point() {
        let p = plan(3, 15.0, 0.0, 24).unwrap();
        assert_eq!(p.points_per_frame, 1);
        assert_eq!(points_at(&p, 0, 3), 1);
        assert_eq!(points_at(&p, p.sewing_frames - 1, 3), 3);
    }

    #[test]
    fn bad_numbers_give_no_plan() {
        assert!(plan(0, 15.0, 2.0, 24).is_none());
        assert!(plan(10, f64::NAN, 2.0, 24).is_none());
        assert!(plan(10, 15.0, 20.0, 24).is_none());
        assert!(plan(10, 15.0, 2.0, 0).is_none());
        assert!(plan(10, 120.0, 2.0, 24).is_none());
        assert!(plan(10, 15.0, -1.0, 24).is_none());
    }

    #[test]
    fn vertical_and_square_canvases_centre_the_design() {
        let l = layout(Aspect::Vertical, 100.0, 50.0, 60, 20.0).unwrap();
        assert_eq!((l.canvas, l.design, l.origin), ((1080, 1920), (960, 480), (60, 720)));
        let l = layout(Aspect::Square, 50.0, 100.0, 60, 20.0).unwrap();
        assert_eq!((l.canvas, l.design, l.origin), ((1080, 1080), (480, 960), (300, 60)));
        // a small design is not blown up past max_scale
        assert_eq!(layout(Aspect::Square, 10.0, 10.0, 60, 20.0).unwrap().design, (200, 200));
    }

    #[test]
    fn as_is_keeps_the_designs_proportions() {
        let l = layout(Aspect::AsIs, 80.0, 40.0, 10, 8.0).unwrap();
        assert_eq!((l.design, l.canvas, l.origin), ((640, 320), (660, 340), (10, 10)));
    }

    #[test]
    fn bad_layouts_give_none() {
        assert!(layout(Aspect::Square, 0.0, 10.0, 0, 5.0).is_none());
        assert!(layout(Aspect::Square, f64::INFINITY, 10.0, 0, 5.0).is_none());
        assert!(layout(Aspect::Square, 10.0, 10.0, 600, 5.0).is_none());
    }
}
