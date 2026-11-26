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
        if r.is_multiple_of(2) {
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
    match out.last().copied() {
        // a last stitch shorter than a tenth of a step (still within the stitch limit) replaces the one before it instead of making a sliver
        Some(prev) if out.len() > 1 && (prev.x - last.x).hypot(prev.y - last.y) < step * 0.1 => {
            if let Some(l) = out.last_mut() {
                *l = last;
            }
        }
        Some(prev) if prev != last => out.push(last),
        _ => {}
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

/// Segment `i` of a line (`line[i]` to `line[i + 1]`).
fn seg(line: &[Point], i: usize) -> Option<(Point, Point)> {
    Some((*line.get(i)?, *line.get(i + 1)?))
}

/// A uniform grid over a line's segments, so a distance check only looks at nearby segments
/// (issue #257: comparing every segment with every other took seconds on a 1 m tile).
struct SegGrid {
    x0: f64,
    y0: f64,
    cell: f64,
    cols: usize,
    rows: usize,
    bins: Vec<Vec<usize>>,
}

/// Cells per side at most, so a line spread over a huge or broken area can't allocate without bound
/// (the cells just get bigger).
const GRID_MAX_SIDE: usize = 1024;

impl SegGrid {
    /// Bins every segment of `line` by its bounding box, with cells of about `cell` mm.
    fn new(line: &[Point], cell: f64) -> Self {
        let finite: Vec<Point> = line.iter().copied().filter(|p| p.x.is_finite() && p.y.is_finite()).collect();
        let (x0, y0, x1, y1) =
            finite.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(a, b, c, d), p| (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)));
        let (x0, y0, x1, y1) = if finite.is_empty() { (0.0, 0.0, 0.0, 0.0) } else { (x0, y0, x1, y1) };
        let span = (x1 - x0).max(y1 - y0).max(0.0);
        let cell = cell.max(span / GRID_MAX_SIDE as f64).max(1e-6);
        let cols = ((((x1 - x0) / cell).floor() as usize).saturating_add(1)).min(GRID_MAX_SIDE + 1);
        let rows = ((((y1 - y0) / cell).floor() as usize).saturating_add(1)).min(GRID_MAX_SIDE + 1);
        let mut g = SegGrid { x0, y0, cell, cols, rows, bins: vec![Vec::new(); cols * rows] };
        for i in 0..line.len().saturating_sub(1) {
            if let Some((a, b)) = seg(line, i) {
                let (c0, c1, r0, r1) = g.range(a, b, 0.0);
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        if let Some(bin) = g.bins.get_mut(r * cols + c) {
                            bin.push(i);
                        }
                    }
                }
            }
        }
        g
    }

    /// The cells (`c0..=c1`, `r0..=r1`) under segment `ab`'s bounding box grown by `pad`. A point
    /// that isn't finite lands in the first or last cell (`as` saturates; NaN gives 0).
    fn range(&self, a: Point, b: Point, pad: f64) -> (usize, usize, usize, usize) {
        let cell_of = |v: f64, o: f64, n: usize| (((v - o) / self.cell).floor().max(0.0) as usize).min(n.saturating_sub(1));
        let (lx, hx) = (a.x.min(b.x) - pad, a.x.max(b.x) + pad);
        let (ly, hy) = (a.y.min(b.y) - pad, a.y.max(b.y) + pad);
        (cell_of(lx, self.x0, self.cols), cell_of(hx, self.x0, self.cols), cell_of(ly, self.y0, self.rows), cell_of(hy, self.y0, self.rows))
    }

    /// Whether any segment `j` binned near `ab` (within `pad`) has `hit(j)` true.
    fn any_near(&self, a: Point, b: Point, pad: f64, mut hit: impl FnMut(usize) -> bool) -> bool {
        let (c0, c1, r0, r1) = self.range(a, b, pad);
        (r0..=r1).any(|r| (c0..=c1).any(|c| self.bins.get(r * self.cols + c).is_some_and(|bin| bin.iter().any(|&j| hit(j)))))
    }
}

/// Whether the line crosses or touches itself: two segments that aren't neighbours closer than
/// [`CLEARANCE`].
pub fn self_touches(line: &[Point]) -> bool {
    if line.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return true;
    }
    let grid = SegGrid::new(line, 4.0);
    (0..line.len().saturating_sub(1)).any(|i| {
        let Some((a, b)) = seg(line, i) else { return false };
        grid.any_near(a, b, CLEARANCE, |j| j > i + 1 && seg(line, j).is_some_and(|(c, d)| segs_dist(a, b, c, d) < CLEARANCE))
    })
}

/// A tile's line for one seed (it may touch itself: see [`tile_line`]).
fn one_try(width: f64, height: f64, seed: u64) -> Vec<Point> {
    let cols = ((width / SPACING).round() as usize + 1).clamp(2, 400);
    let mut rows = ((height / SPACING).round() as usize + 1).clamp(3, 400);
    if rows.is_multiple_of(2) {
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

/// Spiral arm pitch parameter: arm `r = A * theta`, so the two interleaved arms are `PI * A` apart.
const SPIRAL_A: f64 = 0.7;
/// Where the inward arm starts and the outward arm ends (radians); the outward arm is a turn and a
/// bit longer, ending short of the straight-up angle so the line can reach the inward arm from above.
const SPIRAL_IN: f64 = 5.0;
const SPIRAL_OUT: f64 = 6.95;
/// Least distance (mm) from a spiral to the rest of the line.
pub const SPIRAL_CLEARANCE: f64 = 1.7;
/// Stitches kept free of spirals between two of them, and at each end of the line.
const SPIRAL_GAP: usize = 24;

/// A double spiral hung off the line at `m` (tangent `u`, side `v`): in along one arm, out along the
/// interleaved arm, then a short hook back onto the line at `b`. Starts at `a`, ends at `b`.
fn double_spiral(a: Point, b: Point, m: Point, u: Point, v: Point) -> Vec<Point> {
    let c = Point::new(m.x + v.x * (SPIRAL_A * SPIRAL_OUT + 3.0), m.y + v.y * (SPIRAL_A * SPIRAL_OUT + 3.0));
    // local x along the tangent, local y towards the line
    let at = |lx: f64, ly: f64| Point::new(c.x + u.x * lx - v.x * ly, c.y + u.y * lx - v.y * ly);
    let mut pts = vec![a];
    let n = 300;
    // the inward arm is the outward arm turned half a turn, walked from its outer end to the middle
    for i in (0..=n).rev() {
        let th = SPIRAL_IN * i as f64 / n as f64;
        let rr = SPIRAL_A * th;
        pts.push(at(-rr * th.cos(), -rr * th.sin()));
    }
    for i in 1..=n {
        let th = SPIRAL_OUT * i as f64 / n as f64;
        let rr = SPIRAL_A * th;
        pts.push(at(rr * th.cos(), rr * th.sin()));
    }
    pts.push(b);
    resample(&pts, STITCH)
}

/// Hangs small double spirals (about 11 mm) off near-straight stretches of the line, each at least
/// [`SPIRAL_CLEARANCE`] from the rest of the line and inside the tile. Stretches where a spiral
/// would touch anything are skipped.
pub fn add_spirals(line: &[Point], width: f64, height: f64) -> Vec<Point> {
    let mut cur = line.to_vec();
    let mut grid = SegGrid::new(&cur, 4.0);
    let mut k = SPIRAL_GAP;
    while k + SPIRAL_GAP < cur.len() {
        let placed = (|| {
            let (&a, &m, &b) = (cur.get(k - 3)?, cur.get(k)?, cur.get(k + 3)?);
            let (&p, &q) = (cur.get(k - 8)?, cur.get(k + 8)?);
            let len = (q.x - p.x).hypot(q.y - p.y);
            if len < 14.0 || [a, m, b].iter().any(|&x| seg_dist(x, p, q) > 0.8) {
                return None;
            }
            let u = Point::new((q.x - p.x) / len, (q.y - p.y) / len);
            for side in [1.0, -1.0] {
                let v = Point::new(-u.y * side, u.x * side);
                let piece = double_spiral(a, b, m, u, v);
                let inside = piece.iter().all(|t| t.x >= 1.0 && t.y >= 1.0 && t.x <= width - 1.0 && t.y <= height - 1.0);
                // the stitches before `k - 3` and after `k + 3` are the rest of the line: segments
                // up to `k - 5` and from `k + 4` on
                let rest = |j: usize| j + 4 < k || j >= k + 4;
                let clear = piece.windows(2).skip(2).all(|s| {
                    !grid.any_near(s[0], s[1], SPIRAL_CLEARANCE, |j| {
                        rest(j) && seg(&cur, j).is_some_and(|(c, d)| segs_dist(s[0], s[1], c, d) < SPIRAL_CLEARANCE)
                    })
                });
                // the piece with the stitch before and after it: a hairpin where the line joins the
                // spiral is caught here (`clear` skips the piece's first segments)
                let joined: Vec<Point> = cur.get(k - 4).into_iter().chain(piece.iter()).chain(cur.get(k + 4)).copied().collect();
                if inside && !self_touches(&joined) && clear {
                    return Some(piece);
                }
            }
            None
        })();
        match placed {
            Some(piece) => {
                let (head, tail) = (cur.get(..k - 3).unwrap_or_default().to_vec(), cur.get(k + 4..).unwrap_or_default().to_vec());
                // the old stitches k-3..=k+3 are replaced; `piece` runs from a to b, so drop the doubles
                let mut next = head;
                let at = next.len();
                next.extend(piece);
                next.extend(tail);
                k = at + SPIRAL_GAP;
                cur = next;
                grid = SegGrid::new(&cur, 4.0);
            }
            None => k += 2,
        }
    }
    cur
}

/// Like [`tile_line`], with double spirals hung on the line. The seed's line is searched until the
/// whole result, spirals included, doesn't touch itself.
pub fn tile_line_spirals(width: f64, height: f64, seed: u64) -> Option<(Vec<Point>, u64)> {
    let (line, used) = tile_line(width, height, seed)?;
    let with = add_spirals(&line, width, height);
    if !self_touches(&with) { Some((with, used)) } else { Some((line, used)) }
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

    fn touches_brute(line: &[Point]) -> bool {
        let n = line.len().saturating_sub(1);
        (0..n).any(|i| (i + 2..n).any(|j| segs_dist(line[i], line[i + 1], line[j], line[j + 1]) < CLEARANCE))
    }

    #[test]
    fn the_grid_touch_check_agrees_with_comparing_every_pair() {
        let mut rng = Rng(3);
        for round in 0..300 {
            // random walks with steps up to 6 mm, some long jumps, in a small box so they often touch
            let mut at = Point::new(0.0, 0.0);
            let line: Vec<Point> = (0..40)
                .map(|i| {
                    let step = if i % 13 == 7 { 30.0 } else { 3.0 };
                    at = Point::new((at.x + rng.unit() * step).clamp(-20.0, 20.0), (at.y + rng.unit() * step).clamp(-20.0, 20.0));
                    at
                })
                .collect();
            assert_eq!(self_touches(&line), touches_brute(&line), "round {round}");
        }
        for seed in 1..=3 {
            let (line, _) = tile_line(152.4, 152.4, seed).expect("a tile");
            assert!(!touches_brute(&line));
            let mut bent = line.clone();
            let k = bent.len() / 2;
            bent[k] = bent[k + 40];
            assert!(self_touches(&bent) && touches_brute(&bent));
        }
        assert!(self_touches(&[Point::new(0.0, 0.0), Point::new(f64::NAN, 1.0), Point::new(5.0, 5.0)]));
        assert!(!self_touches(&[]) && !self_touches(&[Point::new(1.0, 1.0)]));
    }

    #[test]
    fn the_largest_tile_is_quick() {
        // issue #257: a 1000 x 1000 mm tile with spirals took about 12 s (release build) when every
        // segment was compared with every other; with the grid it is well under a second there
        let t = std::time::Instant::now();
        let (line, _) = tile_line_spirals(MAX_SIDE, MAX_SIDE, 1).expect("a tile");
        let limit = if cfg!(debug_assertions) { 30.0 } else { 3.0 };
        assert!(t.elapsed().as_secs_f64() < limit, "{:?}", t.elapsed());
        assert!(line.len() > 10_000 && !touches_brute(&line[..2000]));
    }

    #[test]
    fn the_touch_check_sees_a_crossing() {
        let x = [Point::new(0.0, 0.0), Point::new(10.0, 10.0), Point::new(10.0, 0.0), Point::new(0.0, 10.0)];
        assert!(self_touches(&x));
    }

    #[test]
    fn spirals_are_small_clear_and_keep_the_line_one_untouched_piece() {
        for (w, h) in [(152.4, 152.4), (203.2, 355.6)] {
            let (plain, _) = tile_line(w, h, 1).expect("a tile");
            let (line, _) = tile_line_spirals(w, h, 1).expect("a tile");
            assert!(line.len() > plain.len() + 100, "{w}x{h}: spirals were added");
            assert!(!self_touches(&line));
            let (a, z) = (line[0], line[line.len() - 1]);
            assert!(a.x.hypot(a.y) < 1e-9 && (z.x - w).hypot(z.y - h) < 1e-9);
            assert!(line.windows(2).all(|s| (s[1].x - s[0].x).hypot(s[1].y - s[0].y) <= MAX_STITCH));
            assert!(line.iter().all(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= w && p.y <= h));
        }
    }

    #[test]
    fn spirals_never_make_the_line_touch_itself_where_they_join_it() {
        // a hairpin where the line ran into a spiral used to make the whole result touch, so the
        // tile came back without spirals (about one seed in five at 6 x 6 in)
        for seed in 1..=30 {
            let (plain, _) = tile_line(152.4, 152.4, seed).expect("a tile");
            assert!(!self_touches(&add_spirals(&plain, 152.4, 152.4)), "seed {seed}");
        }
    }

    #[test]
    fn one_spiral_is_about_eleven_mm_across() {
        let (m, u, v) = (Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0));
        let piece = double_spiral(Point::new(-6.0, 0.0), Point::new(6.0, 0.0), m, u, v);
        let (lo, hi) = piece.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
        assert!(hi - lo > 9.0 && hi - lo < 16.0, "{}", hi - lo);
        assert!(!self_touches(&piece));
    }
}
