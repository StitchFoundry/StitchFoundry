//! Decorative frames for monogram templates (vine, wreaths, scrolls, rope, western, varsity,
//! wedding, baby, holly, pumpkin and a diamond with corner leaves). All drawn here in code, from
//! plain geometry: no traced or borrowed art. A frame is a list of parts around the letters' box.

use std::f64::consts::{PI, TAU};

use vectorcraft_geom::Rect;

use super::Point;

/// How a part is sewn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum How {
    /// A satin line `width` mm wide along a path (or across a closed shape).
    Satin(f64),
    /// A triple run along the path.
    Run,
    /// A filled closed shape.
    Fill,
}

/// One piece of a frame: its points, whether it closes, how it is sewn, and which of the
/// template's colours it takes (0 the main one, 1 the accent, 2 the third).
#[derive(Clone, Debug)]
pub struct Part {
    pub name: &'static str,
    pub pts: Vec<Point>,
    pub closed: bool,
    pub how: How,
    pub color: usize,
}

/// The frames there are (besides none).
pub const FRAMES: [&str; 11] =
    ["vine", "floralWreath", "scroll", "cornerLeaves", "rope", "western", "varsity", "weddingHearts", "baby", "holly", "pumpkin"];

fn pt(c: Point, r: f64, a: f64) -> Point {
    Point::new(c.x + r * a.cos(), c.y + r * a.sin())
}

fn circle(c: Point, r: f64, n: u32) -> Vec<Point> {
    (0..n).map(|i| pt(c, r, f64::from(i) / f64::from(n) * TAU)).collect()
}

/// A pointed leaf from `base` along `angle`, `len` long and `wid` wide.
fn leaf(base: Point, angle: f64, len: f64, wid: f64) -> Vec<Point> {
    let (ux, uy) = (angle.cos(), angle.sin());
    let side = |s: f64, i: u32| {
        let u = f64::from(i) / 10.0;
        let w = s * wid / 2.0 * (PI * u).sin();
        Point::new(base.x + ux * len * u - uy * w, base.y + uy * len * u + ux * w)
    };
    (0..=10).map(|i| side(1.0, i)).chain((1..10).rev().map(|i| side(-1.0, i))).collect()
}

fn star(c: Point, r: f64, points: u32) -> Vec<Point> {
    (0..points * 2).map(|i| pt(c, if i % 2 == 0 { r } else { r * 0.45 }, f64::from(i) / f64::from(points * 2) * TAU - PI / 2.0)).collect()
}

fn heart(c: Point, r: f64) -> Vec<Point> {
    let k = r / 16.0;
    (0..60)
        .map(|i| {
            let t = f64::from(i) / 60.0 * TAU;
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            Point::new(c.x + 16.0 * t.sin().powi(3) * k, c.y - y * k)
        })
        .collect()
}

/// Five rounded petals round a middle.
fn flower(c: Point, r: f64) -> Vec<Point> {
    (0..90).map(|i| pt(c, r * (0.55 + 0.45 * (2.5 * f64::from(i) / 90.0 * TAU).cos().abs()), f64::from(i) / 90.0 * TAU)).collect()
}

fn spiral(c: Point, r: f64, side: f64) -> Vec<Point> {
    (0..=80)
        .map(|i| {
            let t = f64::from(i) / 80.0 * 3.5 * PI;
            let rr = r * (1.0 - t / (3.5 * PI) * 0.9);
            Point::new(c.x + side * rr * t.cos(), c.y + rr * t.sin())
        })
        .collect()
}

fn rect(c: Point, hw: f64, hh: f64) -> Vec<Point> {
    vec![Point::new(c.x - hw, c.y - hh), Point::new(c.x + hw, c.y - hh), Point::new(c.x + hw, c.y + hh), Point::new(c.x - hw, c.y + hh)]
}

fn part(name: &'static str, pts: Vec<Point>, closed: bool, how: How, color: usize) -> Part {
    Part { name, pts, closed, how, color }
}

/// The frame `kind` round the letters' box `b` (points), `margin` clear of it, scaled with it.
/// An unknown kind has no parts.
pub fn frame_parts(kind: &str, b: Rect, margin: f64) -> Vec<Part> {
    let c = b.center();
    let (hw, hh) = (b.width() / 2.0 + margin, b.height() / 2.0 + margin);
    let r = hw.hypot(hh);
    // the width of the lines and the size of the small art grow with the frame, within limits
    let unit = (r / 12.0).clamp(2.0, 14.0);
    let mut v = Vec::new();
    match kind {
        "vine" => {
            v.push(part("Vine", circle(c, r, 96), true, How::Run, 0));
            for i in 0..14 {
                let a = f64::from(i) / 14.0 * TAU;
                let out = if i % 2 == 0 { 0.5 } else { -0.5 };
                v.push(part(
                    "Leaf",
                    leaf(pt(c, r, a), a + PI / 2.0 + out + if i % 2 == 0 { 0.0 } else { PI }, unit * 2.6, unit * 1.2),
                    true,
                    How::Fill,
                    1,
                ));
            }
        }
        "floralWreath" => {
            v.push(part("Stems", circle(c, r, 96), true, How::Run, 1));
            for i in 0..8 {
                let a = f64::from(i) / 8.0 * TAU;
                v.push(part("Flower", flower(pt(c, r, a), unit * 1.9), true, How::Fill, 0));
                let b2 = a + PI / 8.0;
                v.push(part("Leaf", leaf(pt(c, r, b2), b2, unit * 3.0, unit * 1.2), true, How::Fill, 1));
                v.push(part("Leaf", leaf(pt(c, r, b2), b2 + PI, unit * 3.0, unit * 1.2), true, How::Fill, 1));
            }
        }
        "scroll" => {
            for (sy, name) in [(-1.0, "Top scroll"), (1.0, "Bottom scroll")] {
                let y = c.y + sy * (hh + unit);
                v.push(part(name, vec![Point::new(c.x - hw, y), Point::new(c.x + hw, y)], false, How::Satin(unit * 0.5), 0));
                v.push(part("Scroll", spiral(Point::new(c.x - hw - unit * 0.5, y + sy * unit * 1.6), unit * 1.8, -1.0), false, How::Run, 1));
                v.push(part("Scroll", spiral(Point::new(c.x + hw + unit * 0.5, y + sy * unit * 1.6), unit * 1.8, 1.0), false, How::Run, 1));
            }
        }
        "cornerLeaves" => {
            let d = hw + hh;
            let corners = [Point::new(c.x, c.y - d), Point::new(c.x + d, c.y), Point::new(c.x, c.y + d), Point::new(c.x - d, c.y)];
            v.push(part("Diamond", corners.to_vec(), true, How::Satin(unit * 0.5), 0));
            for (i, p) in corners.iter().enumerate() {
                let out = (i as f64) * PI / 2.0 - PI / 2.0;
                for s in [-0.6, 0.6] {
                    v.push(part("Leaf", leaf(*p, out + s, unit * 3.4, unit * 1.4), true, How::Fill, 1));
                }
            }
        }
        "rope" => {
            for (phase, name) in [(0.0, "Rope strand"), (PI, "Rope strand")] {
                let pts = (0..240)
                    .map(|i| {
                        let a = f64::from(i) / 240.0 * TAU;
                        pt(c, r + unit * 0.35 * (28.0 * a + phase).sin(), a)
                    })
                    .collect();
                v.push(part(name, pts, true, How::Satin(unit * 0.3), 0));
            }
        }
        "western" => {
            let q = hw.max(hh) + unit;
            v.push(part("Frame", rect(c, q, q), true, How::Satin(unit * 0.5), 0));
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                v.push(part("Star", star(Point::new(c.x + dx * q, c.y + dy * q), unit * 1.8, 5), true, How::Fill, 1));
            }
            v.push(part("Star", star(Point::new(c.x, c.y - q - unit * 1.2), unit * 1.5, 5), true, How::Fill, 1));
        }
        "varsity" => {
            let (w, h) = (hw * 1.15 + unit, hh + unit);
            v.push(part("Patch", rect(c, w, h), true, How::Satin(unit * 0.6), 0));
            v.push(part("Inner line", rect(c, w - unit * 1.1, h - unit * 1.1), true, How::Run, 1));
            for s in [-1.0, 1.0] {
                v.push(part("Star", star(Point::new(c.x + s * (w - unit * 2.2), c.y + h - unit * 2.2), unit * 0.9, 5), true, How::Fill, 1));
            }
        }
        "weddingHearts" => {
            let k = hw.max(hh) * 1.9 / 16.0;
            let pts: Vec<Point> = heart(Point::new(c.x, c.y), 16.0 * k);
            v.push(part("Heart frame", pts, true, How::Satin(unit * 0.45), 0));
            for (i, dx) in [-2.6, 0.0, 2.6].iter().enumerate() {
                let rr = if i == 1 { unit * 1.4 } else { unit * 1.0 };
                v.push(part("Small heart", heart(Point::new(c.x + dx * unit, c.y - hh * 1.35 - unit * 1.8), rr), true, How::Fill, 1));
            }
        }
        "baby" => {
            v.push(part("Ring", circle(c, r, 96), true, How::Run, 0));
            for i in 0..12 {
                v.push(part("Dot", circle(pt(c, r, f64::from(i) / 12.0 * TAU), unit * 0.7, 16), true, How::Fill, 1));
            }
            for (k, dx) in [-1.8, 0.0, 1.8].iter().enumerate() {
                v.push(part(
                    "Star",
                    star(Point::new(c.x + dx * unit * 1.4, c.y - r - unit * (2.2 - 0.3 * (k % 2) as f64)), unit * 0.9, 5),
                    true,
                    How::Fill,
                    2,
                ));
            }
        }
        "holly" => {
            for i in 0..5 {
                let a = f64::from(i) / 5.0 * TAU - PI / 2.0;
                let p = pt(c, r, a);
                for s in [-0.55, 0.55] {
                    v.push(part("Holly leaf", leaf(p, a + PI / 2.0 + s * 1.3, unit * 3.4, unit * 1.9), true, How::Fill, 1));
                    v.push(part("Holly leaf", leaf(p, a - PI / 2.0 + s * 1.3, unit * 3.4, unit * 1.9), true, How::Fill, 1));
                }
                for (j, off) in [(0.0, 0.0), (0.5, 0.0), (-0.25, 0.9)].iter().enumerate() {
                    let _ = j;
                    v.push(part(
                        "Berry",
                        circle(Point::new(p.x + off.0 * unit * 1.4, p.y + off.1 * unit * 1.4), unit * 0.65, 14),
                        true,
                        How::Fill,
                        0,
                    ));
                }
            }
        }
        "pumpkin" => {
            let (a, h) = (hw * 1.25 + unit, hh * 1.1 + unit);
            let pts = (0..120)
                .map(|i| {
                    let t = f64::from(i) / 120.0 * TAU;
                    let lobe = 1.0 + 0.07 * (8.0 * t).cos();
                    Point::new(c.x + a * t.cos() * lobe, c.y + h * t.sin() * lobe)
                })
                .collect();
            v.push(part("Pumpkin", pts, true, How::Satin(unit * 0.6), 0));
            v.push(part("Stem", rect(Point::new(c.x, c.y - h - unit * 1.2), unit * 0.7, unit * 1.2), true, How::Fill, 2));
            v.push(part("Leaf", leaf(Point::new(c.x + unit * 0.8, c.y - h - unit * 0.6), -PI / 6.0, unit * 3.4, unit * 1.6), true, How::Fill, 1));
        }
        _ => {}
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_frame_has_finite_parts_that_scale_with_the_letters() {
        for kind in FRAMES {
            for size in [20.0, 80.0, 400.0] {
                let b = Rect::new(-size / 2.0, -size / 2.0, size / 2.0, size / 2.0);
                let parts = frame_parts(kind, b, 8.0);
                assert!(parts.len() >= 2, "{kind}");
                for p in &parts {
                    assert!(p.pts.len() >= 2 && p.pts.iter().all(|q| q.x.is_finite() && q.y.is_finite()), "{kind} {}", p.name);
                }
            }
        }
        assert!(frame_parts("nope", Rect::new(0.0, 0.0, 10.0, 10.0), 3.0).is_empty());
    }
}
