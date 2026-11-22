//! Quilting tile designer (issue #199, slice 1): one continuous, non-crossing meander line that
//! fills a tile, in at the top-left corner and out at the bottom-right. A random Hamiltonian path
//! on a grid (snake path, then "backbite" moves at its far end), jittered, rounded with corner
//! cutting and resampled to a stitch length. Millimetres, y down. Seeded: a seed reproduces a design.

use crate::geom::Point;

/// Grid spacing (mm): the size of one meander cell, the same on every tile size.
pub const SPACING: f64 = 32.0;
/// Stitch length (mm); no stitch is longer than [`MAX_STITCH`].
pub const STITCH: f64 = 2.0;
pub const MAX_STITCH: f64 = 2.2;
/// Smallest and largest tile side (mm).
pub const MIN_SIDE: f64 = 40.0;
pub const MAX_SIDE: f64 = 1000.0;
/// Two parts of the line closer than this (mm) count as touching.
pub const CLEARANCE: f64 = 0.5;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A number in `0..n` (`n` > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    /// A number in -1..1.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }
}

/// A Hamiltonian path over `cols` x `rows` grid cells from (0,0) to (cols-1, rows-1). `rows` is made
/// odd by the caller so the snake path already ends there; backbite moves then shuffle it and the
/// end is brought back to that corner.
pub fn grid_path(cols: usize, rows: usize, seed: u64) -> Vec<(usize, usize)> {
    let mut path: Vec<(usize, usize)> = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        if r % 2 == 0 {
            path.extend((0..cols).map(|c| (c, r)));
        } else {
            path.extend((0..cols).rev().map(|c| (c, r)));
        }
    }
    let snake = path.clone();
    let target = (cols.saturating_sub(1), rows.saturating_sub(1));
    let n = path.len();
    if n < 4 || snake.last() != Some(&target) {
        return snake;
    }
    let mut rng = Rng(seed);
    for step in 0..(60 * n + 200) {
        if step >= 20 * n && path.last() == Some(&target) {
            return path;
        }
        let Some(&(c, r)) = path.last() else { break };
        let mut near = Vec::with_capacity(4);
        if c > 0 {
            near.push((c - 1, r));
        }
        if c + 1 < cols {
            near.push((c + 1, r));
        }
        if r > 0 {
            near.push((c, r - 1));
        }
        if r + 1 < rows {
            near.push((c, r + 1));
        }
        let pick = near[rng.below(near.len())];
        if let Some(k) = path.iter().position(|&q| q == pick) {
            path[k + 1..].reverse();
        }
    }
    snake
}

/// Corner cutting, `rounds` times, the two ends kept in place.
fn chaikin(pts: &[Point], rounds: usize) -> Vec<Point> {
    let mut cur = pts.to_vec();
    for _ in 0..rounds {
        let (Some(&first), Some(&last)) = (cur.first(), cur.last()) else { break };
        let mut out = vec![first];
        for w in cur.windows(2) {
            let (a, b) = (w[0], w[1]);
            out.push(Point::new(0.75 * a.x + 0.25 * b.x, 0.75 * a.y + 0.25 * b.y));
            out.push(Point::new(0.25 * a.x + 0.75 * b.x, 0.25 * a.y + 0.75 * b.y));
        }
        out.push(last);
        cur = out;
    }
    cur
}

/// The line as points `step` mm apart along it, the first and last points kept.
fn resample(pts: &[Point], step: f64) -> Vec<Point> {
    let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else { return Vec::new() };
    let mut out = vec![first];
    let mut at = first;
    let mut need = step;
    for &p in pts.iter().skip(1) {
        loop {
            let (dx, dy) = (p.x - at.x, p.y - at.y);
            let d = dx.hypot(dy);
            if d < need {
                need -= d;
                at = p;
                break;
            }
            let t = need / d;
            at = Point::new(at.x + dx * t, at.y + dy * t);
            out.push(at);
            need = step;
        }
    }
    if out.last() != Some(&last) {
        out.push(last);
    }
    out
}

fn seg_dist(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// The distance between segments `ab` and `cd` (0 when they cross).
fn segs_dist(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if cross(a, b, c) * cross(a, b, d) < 0.0 && cross(c, d, a) * cross(c, d, b) < 0.0 {
        return 0.0;
    }
    seg_dist(a, c, d).min(seg_dist(b, c, d)).min(seg_dist(c, a, b)).min(seg_dist(d, a, b))
}

/// Whether the line crosses or touches itself: two segments that aren't neighbours closer than
/// [`CLEARANCE`].
pub fn self_touches(line: &[Point]) -> bool {
    let n = line.len().saturating_sub(1);
    for i in 0..n {
        for j in i + 2..n {
            if let (Some(&a), Some(&b), Some(&c), Some(&d)) = (line.get(i), line.get(i + 1), line.get(j), line.get(j + 1)) {
                if segs_dist(a, b, c, d) < CLEARANCE {
                    return true;
                }
            }
        }
    }
    false
}

/// A tile's line for one seed (it may touch itself: see [`tile_line`]).
fn one_try(width: f64, height: f64, seed: u64) -> Vec<Point> {
    let cols = ((width / SPACING).round() as usize + 1).clamp(2, 400);
    let mut rows = ((height / SPACING).round() as usize + 1).clamp(3, 400);
    if rows % 2 == 0 {
        rows += 1;
    }
    let (sx, sy) = (width / (cols - 1) as f64, height / (rows - 1) as f64);
    let path = grid_path(cols, rows, seed);
    let mut rng = Rng(seed ^ 0xA5A5_5A5A);
    let last = path.len().saturating_sub(1);
    let pts: Vec<Point> = path
        .iter()
        .enumerate()
        .map(|(i, &(c, r))| {
            let (x, y) = (c as f64 * sx, r as f64 * sy);
            if i == 0 || i == last {
                return Point::new(x, y);
            }
            let (jx, jy) = (rng.unit() * 0.2 * sx, rng.unit() * 0.2 * sy);
            Point::new((x + jx).clamp(0.0, width), (y + jy).clamp(0.0, height))
        })
        .collect();
    resample(&chaikin(&pts, 4), STITCH)
}

/// The tile's stitch points (mm) and the seed that made them: the first seed from `seed` on whose
/// line doesn't cross or touch itself. `None` when the size is out of range or no seed works.
pub fn tile_line(width: f64, height: f64, seed: u64) -> Option<(Vec<Point>, u64)> {
    let ok = |v: f64| v.is_finite() && (MIN_SIDE..=MAX_SIDE).contains(&v);
    if !ok(width) || !ok(height) {
        return None;
    }
    (0..200u64).map(|k| seed.wrapping_add(k)).find_map(|s| {
        let line = one_try(width, height, s);
        (!self_touches(&line)).then_some((line, s))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_path_visits_every_node_once_corner_to_corner() {
        for (c, r) in [(2, 3), (4, 5), (7, 9)] {
            let p = grid_path(c, r, 7);
            assert_eq!(p.len(), c * r);
            let mut s = p.clone();
            s.sort();
            s.dedup();
            assert_eq!(s.len(), c * r);
            assert_eq!((p[0], p[p.len() - 1]), ((0, 0), (c - 1, r - 1)));
            assert!(p.windows(2).all(|w| w[0].0.abs_diff(w[1].0) + w[0].1.abs_diff(w[1].1) == 1));
        }
        assert_eq!(grid_path(5, 7, 1), grid_path(5, 7, 1));
        assert_ne!(grid_path(5, 7, 1), grid_path(5, 7, 2));
    }

    #[test]
    fn tiles_are_one_continuous_untouched_line_corner_to_corner() {
        // 4x4, 6x6, 8x8, 8x14 inches
        for (w, h) in [(101.6, 101.6), (152.4, 152.4), (203.2, 203.2), (203.2, 355.6)] {
            for seed in [1, 2, 3] {
                let (line, _) = tile_line(w, h, seed).expect("a tile");
                assert!(!self_touches(&line), "{w}x{h} seed {seed}");
                let (a, z) = (line[0], line[line.len() - 1]);
                assert!(a.x.hypot(a.y) < 1e-9 && (z.x - w).hypot(z.y - h) < 1e-9);
                assert!(line.windows(2).all(|s| (s[1].x - s[0].x).hypot(s[1].y - s[0].y) <= MAX_STITCH));
                assert!(line.iter().all(|p| p.x >= -1e-9 && p.y >= -1e-9 && p.x <= w + 1e-9 && p.y <= h + 1e-9));
            }
        }
        assert_eq!(tile_line(152.4, 152.4, 5), tile_line(152.4, 152.4, 5));
        assert!(tile_line(f64::NAN, 100.0, 1).is_none() && tile_line(10.0, 100.0, 1).is_none());
    }

    #[test]
    fn the_touch_check_sees_a_crossing() {
        let x = [Point::new(0.0, 0.0), Point::new(10.0, 10.0), Point::new(10.0, 0.0), Point::new(0.0, 10.0)];
        assert!(self_touches(&x));
    }
}
