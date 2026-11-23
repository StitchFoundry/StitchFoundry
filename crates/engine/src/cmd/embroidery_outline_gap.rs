//! Check Design's gap-in-outline test: a fill whose outline (a run or satin sewn round it) sits
//! away from its edge, so the fabric shows between them.

use sf_core::geom::{Contour, Point, Shape, polyline, shape::FLATTEN_TOLERANCE};
use sf_core::model::{Design, ObjectKind};

/// A gap narrower than this (mm) is hidden by the outline's own thread width.
pub const GAP_MIN: f64 = 0.4;
/// An outline farther than this (mm) from the fill isn't taken to be that fill's outline.
pub const MATCH_MAX: f64 = 2.5;
/// Share of the outline that must lie outside the fill to count as a gap (not an overlap).
const OUTSIDE_SHARE: f64 = 0.8;

/// One gap: the fill and outline object indexes, the gap (mm: how far the outline's nearest point
/// sits from the fill's edge) and a point on the outline.
pub struct OutlineGap {
    pub fill: usize,
    pub outline: usize,
    pub gap: f64,
    pub at: Point,
}

/// Points along a polyline: its vertices and the middle of each segment.
fn samples(pts: &[Point], closed: bool) -> Vec<Point> {
    let mut out = pts.to_vec();
    let n = pts.len();
    let last = if closed { n } else { n.saturating_sub(1) };
    for i in 0..last {
        if let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n.max(1))) {
            out.push(a.midpoint(*b));
        }
    }
    out
}

/// Distance from `p` to the edge of `shape` (outer outline and holes), and whether it's outside.
fn edge_distance(shape: &Shape, p: Point) -> (f64, bool) {
    let mut d = polyline::distance_to_ring(&shape.outer.polyline(FLATTEN_TOLERANCE), p);
    for h in &shape.holes {
        d = d.min(polyline::distance_to_ring(&h.polyline(FLATTEN_TOLERANCE), p));
    }
    (d, !shape.contains(p))
}

/// The gap of `pts` from the edge of `shape`, when they trace it from outside: the nearest
/// approach, so an outline 1 mm out reads 1.0 whatever its corners do.
fn gap_of(shape: &Shape, pts: &[Point]) -> Option<(f64, Point)> {
    if pts.is_empty() {
        return None;
    }
    let (mut outside, mut near) = (0usize, (f64::INFINITY, pts[0]));
    for p in pts {
        let (d, out) = edge_distance(shape, *p);
        if !d.is_finite() || d > MATCH_MAX {
            return None;
        }
        if out {
            outside += 1;
            if d < near.0 {
                near = (d, *p);
            }
        }
    }
    let share = outside as f64 / pts.len() as f64;
    (outside > 0 && share >= OUTSIDE_SHARE && near.0 >= GAP_MIN).then_some(near)
}

fn contour_samples(c: &Contour) -> Vec<Point> {
    samples(&c.polyline(FLATTEN_TOLERANCE), c.closed)
}

/// Every fill whose outline sits `GAP_MIN` mm or more outside its edge.
pub fn outline_gaps(design: &Design) -> Vec<OutlineGap> {
    let mut found = Vec::new();
    for (i, fill) in design.objects.iter().enumerate() {
        let ObjectKind::Fill(f) = &fill.kind else { continue };
        for (j, other) in design.objects.iter().enumerate() {
            if j == i {
                continue;
            }
            let best = match &other.kind {
                ObjectKind::Run(r) if r.path.closed => gap_of(&f.shape, &contour_samples(&r.path)),
                // a satin border centred on the edge has a rail inside the fill: only a column
                // lying wholly outside leaves a gap, as wide as its nearer rail is far
                ObjectKind::Satin(s) => {
                    let (a, b) = (gap_of(&f.shape, &contour_samples(&s.column.rail_a())), gap_of(&f.shape, &contour_samples(&s.column.rail_b())));
                    match (a, b) {
                        (Some(x), Some(y)) => Some(if x.0 <= y.0 { x } else { y }),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some((gap, at)) = best {
                found.push(OutlineGap { fill: i, outline: j, gap, at });
            }
        }
    }
    found
}
