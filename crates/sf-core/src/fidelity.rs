//! Digitizer scorecard, slice 2: does the stitched result still look like the artwork?
//!
//! The stitches are drawn back onto a small colour raster (one thread width per stitch, the
//! thread on top wins) and compared with the artwork, held as a raster of the same size and
//! scale. The score is the **colour-region overlap**: the pixels where both show the same thread
//! colour, divided by the pixels where either shows any thread (intersection over union of the
//! coloured area, with a colour mismatch counting as a miss).
//!
//! Why this and not SSIM: embroidery art is a few flat thread colours, and the question is
//! "is every region there, in the right colour, and nothing sewn where the art is empty?".
//! SSIM measures local texture and luminance, which stitch texture always disturbs, and it
//! cannot say which region went missing. The overlap needs no tuning, is cheap, and reports
//! per-colour coverage and the worst tile. Stitch count never enters the score: only what
//! is covered counts, so dropping detail cannot win, and neither can piling on stitches.
//! It measures; it makes no claim about which system is better.

use crate::geom::Point;
use crate::pattern::{Cmd, Pattern};
use crate::threads::Rgb;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Most pixels a raster may hold (a bigger request is refused, not allocated).
pub const MAX_PIXELS: usize = 4_000_000;
/// Finest pixel a raster may have (mm): a thread is then 40 pixels wide.
pub const MIN_MM_PER_PX: f64 = 0.01;
/// Most pixel visits one [`Raster::render_like`] may make; past it the design is refused as
/// too large to compare at that resolution instead of running for minutes.
pub const MAX_RENDER_WORK: u64 = 200_000_000;
/// Width of a drawn thread (mm).
pub const THREAD_MM: f64 = 0.4;
/// The worst-tile search cuts the raster into this many tiles per side.
const TILES: usize = 8;

/// A raster of thread colours: `None` is bare fabric / transparent art.
#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    pub width: usize,
    pub height: usize,
    /// Side of one pixel (mm).
    pub mm_per_px: f64,
    /// Design-space position (mm) of the top-left corner.
    pub origin: Point,
    pub cells: Vec<Option<Rgb>>,
}

impl Raster {
    /// An empty raster, refused when the size is zero, not finite or over [`MAX_PIXELS`], or
    /// when a pixel is finer than [`MIN_MM_PER_PX`].
    pub fn blank(width: usize, height: usize, mm_per_px: f64, origin: Point) -> Result<Raster, String> {
        let n = width.checked_mul(height).ok_or("raster too large")?;
        if n == 0 || n > MAX_PIXELS {
            return Err(format!("raster must hold 1 to {MAX_PIXELS} pixels, not {width}x{height}"));
        }
        if !(mm_per_px.is_finite() && mm_per_px > 0.0 && origin.x.is_finite() && origin.y.is_finite()) {
            return Err("raster scale and origin must be finite".into());
        }
        if mm_per_px < MIN_MM_PER_PX {
            return Err(format!("raster pixels must be at least {MIN_MM_PER_PX} mm, not {mm_per_px}"));
        }
        Ok(Raster { width, height, mm_per_px, origin, cells: vec![None; n] })
    }

    /// The artwork as RGBA bytes (`width * height * 4`): each opaque pixel becomes the nearest
    /// colour of `palette`; pixels with alpha under 128 are bare.
    pub fn from_rgba(width: usize, height: usize, mm_per_px: f64, origin: Point, rgba: &[u8], palette: &[Rgb]) -> Result<Raster, String> {
        let mut r = Raster::blank(width, height, mm_per_px, origin)?;
        if rgba.len() != r.cells.len() * 4 {
            return Err("pixel data does not match the size".into());
        }
        for (cell, px) in r.cells.iter_mut().zip(rgba.as_chunks::<4>().0) {
            if let [red, green, blue, alpha] = *px
                && alpha >= 128
            {
                let d = |c: &Rgb| {
                    (i32::from(c.r) - i32::from(red)).pow(2) + (i32::from(c.g) - i32::from(green)).pow(2) + (i32::from(c.b) - i32::from(blue)).pow(2)
                };
                *cell = palette.iter().min_by_key(|c| d(c)).copied();
            }
        }
        Ok(r)
    }

    /// Draws `pattern` onto an empty raster of the same size and scale as `self`; refused when it
    /// would take more than [`MAX_RENDER_WORK`] pixel visits.
    pub fn render_like(&self, pattern: &Pattern) -> Result<Raster, String> {
        let mut out = Raster::blank(self.width, self.height, self.mm_per_px, self.origin)?;
        let radius = (THREAD_MM / 2.0).max(self.mm_per_px / 2.0);
        let mut work = MAX_RENDER_WORK;
        for block in pattern.color_blocks() {
            let color = pattern.thread(block.thread).color;
            let mut needle: Option<Point> = None;
            for s in pattern.stitches.get(block.range.clone()).unwrap_or_default() {
                if !(s.x.is_finite() && s.y.is_finite()) {
                    needle = None;
                    continue;
                }
                match s.cmd {
                    Cmd::Stitch => {
                        let p = s.pos();
                        out.line(needle.unwrap_or(p), p, radius, color, &mut work)?;
                        needle = Some(p);
                    }
                    Cmd::Trim | Cmd::End => needle = None,
                    Cmd::Jump | Cmd::Stop | Cmd::ColorChange => {}
                }
            }
        }
        Ok(out)
    }

    /// Discs along `a`–`b`, spaced at half a pixel or half the radius (whichever is longer), so
    /// a thread many pixels wide is not repainted thousands of times per pixel of length.
    fn line(&mut self, a: Point, b: Point, radius: f64, color: Rgb, work: &mut u64) -> Result<(), String> {
        let len = a.distance(b);
        let step = (self.mm_per_px / 2.0).max(radius / 2.0);
        let steps = ((len / step).ceil() as usize).clamp(1, 200_000);
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            self.disc(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), radius, color, work)?;
        }
        Ok(())
    }

    fn disc(&mut self, c: Point, radius: f64, color: Rgb, work: &mut u64) -> Result<(), String> {
        let px = (c.x - self.origin.x) / self.mm_per_px;
        let py = (c.y - self.origin.y) / self.mm_per_px;
        let r = radius / self.mm_per_px;
        if !(px.is_finite() && py.is_finite()) || px < -r || py < -r || px > self.width as f64 + r || py > self.height as f64 + r {
            return Ok(());
        }
        let (x0, x1) = ((px - r).floor().max(0.0) as usize, ((px + r).ceil().max(0.0) as usize).min(self.width.saturating_sub(1)));
        let (y0, y1) = ((py - r).floor().max(0.0) as usize, ((py + r).ceil().max(0.0) as usize).min(self.height.saturating_sub(1)));
        let visits = (x1.saturating_sub(x0) as u64 + 1).saturating_mul(y1.saturating_sub(y0) as u64 + 1);
        *work = work.checked_sub(visits).ok_or("design too large to compare at this resolution")?;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f64 + 0.5 - px, y as f64 + 0.5 - py);
                if dx * dx + dy * dy <= r * r
                    && let Some(cell) = self.cells.get_mut(y * self.width + x)
                {
                    *cell = Some(color);
                }
            }
        }
        Ok(())
    }
}

/// Fidelity of `pattern` to `art`: one JSON object with the overall `score` (0 to 1, 1 = every
/// coloured pixel of the art is stitched in its colour and nothing else is), the coverage of
/// each art colour, and the `worstTile` (where the two differ most, in mm).
pub fn fidelity(art: &Raster, pattern: &Pattern) -> Result<Value, String> {
    let sewn = art.render_like(pattern)?;
    let (w, h) = (art.width, art.height);
    let (mut inter, mut union) = (0usize, 0usize);
    let mut per_color: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut tiles = vec![(0usize, 0usize); TILES * TILES];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let (a, b) = (art.cells.get(i).copied().flatten(), sewn.cells.get(i).copied().flatten());
            if a.is_none() && b.is_none() {
                continue;
            }
            let same = a == b;
            union += 1;
            inter += usize::from(same);
            if let Some(c) = a {
                let e = per_color.entry(String::from(c)).or_default();
                e.0 += 1;
                e.1 += usize::from(same);
            }
            if let Some(t) = tiles.get_mut((y * TILES / h) * TILES + x * TILES / w) {
                t.0 += 1;
                t.1 += usize::from(same);
            }
        }
    }
    let ratio = |n: usize, d: usize| if d == 0 { 1.0 } else { n as f64 / d as f64 };
    let worst = tiles.iter().enumerate().filter(|(_, t)| t.0 > 0).min_by(|a, b| ratio(a.1.1, a.1.0).total_cmp(&ratio(b.1.1, b.1.0)));
    let worst_json = worst.map_or(Value::Null, |(i, t)| {
        let (tx, ty) = (i % TILES, i / TILES);
        let (tw, th) = (w as f64 * art.mm_per_px / TILES as f64, h as f64 * art.mm_per_px / TILES as f64);
        json!({ "xMm": art.origin.x + tx as f64 * tw, "yMm": art.origin.y + ty as f64 * th, "widthMm": tw, "heightMm": th, "score": ratio(t.1, t.0) })
    });
    let colors: Vec<Value> = per_color.iter().map(|(c, &(n, ok))| json!({ "color": c, "artPixels": n, "coverage": ratio(ok, n) })).collect();
    Ok(json!({
        "method": "colour-region overlap (intersection over union of the coloured area)",
        "score": ratio(inter, union),
        "pixelMm": art.mm_per_px,
        "colors": colors,
        "worstTile": worst_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threads::Thread;

    const RED: Rgb = Rgb::new(200, 0, 0);
    const BLUE: Rgb = Rgb::new(0, 0, 200);

    fn pattern(blocks: &[(Rgb, f64, f64)], pass: usize) -> Pattern {
        // each block: dense rows over a 10 x 10 mm square starting at (x0, y0)
        let mut p = Pattern::new();
        for (n, &(c, x0, y0)) in blocks.iter().enumerate() {
            p.threads.push(Thread { color: c, ..Thread::default() });
            if n > 0 {
                p.color_change();
            }
            for _ in 0..pass {
                for row in 0..=25 {
                    let y = y0 + row as f64 * 0.4;
                    let (a, b) = if row % 2 == 0 { (x0, x0 + 10.0) } else { (x0 + 10.0, x0) };
                    if row == 0 {
                        p.trim();
                    }
                    p.jump(Point::new(a, y));
                    p.stitch(Point::new(a, y));
                    p.stitch(Point::new(b, y));
                }
            }
        }
        p.ensure_end();
        p
    }

    fn art() -> Raster {
        let two = pattern(&[(RED, 1.0, 1.0), (BLUE, 14.0, 1.0)], 1);
        Raster::blank(100, 60, 0.25, Point::new(0.0, 0.0)).unwrap().render_like(&two).unwrap()
    }

    #[test]
    fn perfect_match_scores_one() {
        let a = art();
        let v = fidelity(&a, &pattern(&[(RED, 1.0, 1.0), (BLUE, 14.0, 1.0)], 1)).unwrap();
        assert_eq!(v["score"].as_f64(), Some(1.0));
        assert!(v["colors"].as_array().unwrap().iter().all(|c| c["coverage"].as_f64() == Some(1.0)));
    }

    #[test]
    fn dropped_region_scores_lower_and_is_located() {
        let a = art();
        let v = fidelity(&a, &pattern(&[(RED, 1.0, 1.0)], 1)).unwrap();
        let s = v["score"].as_f64().unwrap();
        assert!(s < 0.6 && s > 0.3, "{s}");
        let blue = v["colors"].as_array().unwrap().iter().find(|c| c["color"] == "#0000c8").unwrap();
        assert_eq!(blue["coverage"].as_f64(), Some(0.0));
        // the worst tile lies over the dropped blue square (x from 14 mm)
        let t = &v["worstTile"];
        assert!(t["xMm"].as_f64().unwrap() + t["widthMm"].as_f64().unwrap() > 14.0 && t["score"].as_f64() == Some(0.0), "{t}");
    }

    #[test]
    fn wrong_colour_and_stray_stitches_lower_the_score() {
        let a = art();
        let swapped = fidelity(&a, &pattern(&[(BLUE, 1.0, 1.0), (RED, 14.0, 1.0)], 1)).unwrap();
        assert!(swapped["score"].as_f64().unwrap() < 0.1);
        let stray = fidelity(&a, &pattern(&[(RED, 1.0, 1.0), (BLUE, 14.0, 1.0), (BLUE, 14.0, 8.0)], 1)).unwrap();
        assert!(stray["score"].as_f64().unwrap() < 1.0);
    }

    #[test]
    fn stitch_count_alone_cannot_change_the_score() {
        let a = art();
        let one = fidelity(&a, &pattern(&[(RED, 1.0, 1.0), (BLUE, 14.0, 1.0)], 1)).unwrap();
        let three = fidelity(&a, &pattern(&[(RED, 1.0, 1.0), (BLUE, 14.0, 1.0)], 3)).unwrap();
        assert_eq!(one["score"], three["score"]);
    }

    #[test]
    fn rgba_art_is_matched_to_the_nearest_palette_colour() {
        let mut px = vec![0u8; 4 * 4];
        px[0..4].copy_from_slice(&[190, 10, 10, 255]);
        px[4..8].copy_from_slice(&[10, 10, 220, 255]);
        px[8..12].copy_from_slice(&[255, 255, 255, 0]);
        let r = Raster::from_rgba(2, 2, 1.0, Point::new(0.0, 0.0), &px, &[RED, BLUE]).unwrap();
        assert_eq!(r.cells, vec![Some(RED), Some(BLUE), None, None]);
        assert!(Raster::from_rgba(2, 2, 1.0, Point::new(0.0, 0.0), &px[..8], &[RED]).is_err());
    }

    #[test]
    fn bad_sizes_and_non_finite_stitches_are_refused_or_skipped() {
        assert!(Raster::blank(0, 5, 1.0, Point::new(0.0, 0.0)).is_err());
        assert!(Raster::blank(5000, 5000, 1.0, Point::new(0.0, 0.0)).is_err());
        assert!(Raster::blank(5, 5, f64::NAN, Point::new(0.0, 0.0)).is_err());
        assert!(Raster::blank(5, 5, 0.0005, Point::new(0.0, 0.0)).is_err());
        let a = Raster::blank(10, 10, 1.0, Point::new(0.0, 0.0)).unwrap();
        let mut p = Pattern::new();
        p.stitch(Point::new(f64::NAN, 1.0));
        p.stitch(Point::new(1.0, 1.0));
        p.stitch(Point::new(1e300, 1e300));
        assert!(fidelity(&a, &p).unwrap()["score"].as_f64().is_some());
        assert_eq!(fidelity(&a, &Pattern::new()).unwrap()["score"].as_f64(), Some(1.0));
    }

    /// #274: the finest raster with a long stitch used to take seconds per stitch.
    #[test]
    fn fine_raster_and_long_stitches_finish_quickly_or_are_refused() {
        let art = Raster::blank(2000, 2000, MIN_MM_PER_PX, Point::new(0.0, 0.0)).unwrap();
        let mut p = Pattern::new();
        p.stitch(Point::new(1.0, 1.0));
        p.stitch(Point::new(19.0, 19.0));
        let start = std::time::Instant::now();
        assert!(art.render_like(&p).is_ok());
        let mut many = Pattern::new();
        for i in 0..20_000 {
            let y = f64::from(i % 200) * 0.1;
            many.stitch(Point::new(0.0, y));
            many.stitch(Point::new(20.0, y));
        }
        assert!(art.render_like(&many).is_err());
        assert!(start.elapsed() < std::time::Duration::from_secs(5), "{:?}", start.elapsed());
    }
}
