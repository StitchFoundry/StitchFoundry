//! Clipart sets (Holidays, Sports, Pets & animals, Nature, Kids, Words & frames) and the first
//! twenty motifs of the growing library. Every motif is drawn here, in code, for StitchFoundry:
//! original art, no traced or scraped drawings (see ASSETS.md).

use std::f64::consts::PI;

use super::embroidery_clipart::{circle, ellipse, poly, svg};

/// The sets, in the order the picker shows them.
pub const SETS: [&str; 6] = ["Holidays", "Sports", "Pets & animals", "Nature", "Kids", "Words & frames"];

/// The set of a motif drawn before the sets existed.
pub fn set_of_original(name: &str) -> &'static str {
    match name {
        "Heart" | "Star" | "Sparkle" | "Snowflake" => "Holidays",
        "Shield" => "Sports",
        "Paw" => "Pets & animals",
        "Leaf" | "Flower" | "Sun" | "Moon" | "Lightning" => "Nature",
        "Crown" | "Music note" => "Kids",
        _ => "Words & frames",
    }
}

/// A star of `n` points centred on (`cx`, `cy`).
fn star_at(cx: f64, cy: f64, n: u32, outer: f64, inner: f64) -> String {
    let pts: Vec<(f64, f64)> = (0..2 * n)
        .map(|k| {
            let r = if k % 2 == 0 { outer } else { inner };
            let a = -PI / 2.0 + PI * f64::from(k) / f64::from(n);
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    poly(&pts)
}

/// A `w` × `h` rectangle centred on (`cx`, `cy`), turned `deg`.
fn bar(cx: f64, cy: f64, w: f64, h: f64, deg: f64) -> String {
    let a = deg.to_radians();
    let p = |x: f64, y: f64| (cx + x * a.cos() - y * a.sin(), cy + x * a.sin() + y * a.cos());
    poly(&[p(-w / 2.0, -h / 2.0), p(w / 2.0, -h / 2.0), p(w / 2.0, h / 2.0), p(-w / 2.0, h / 2.0)])
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

/// The new motifs: name, set, SVG (100 × 100).
pub fn motifs() -> Vec<(&'static str, &'static str, String)> {
    let axis = (-30.0f64).to_radians();
    let laces: String =
        [-9.0, 0.0, 9.0].iter().map(|d| bar(50.0 + d * axis.cos(), 50.0 + d * axis.sin(), 3.0, 14.0, 60.0)).collect::<Vec<_>>().join(" ");
    vec![
        // Holidays
        (
            "Pumpkin",
            "Holidays",
            svg(&[
                ("#d9731a", format!("{} {}", ellipse(28.0, 60.0, 20.0, 30.0, 0.0), ellipse(72.0, 60.0, 20.0, 30.0, 0.0))),
                ("#f28c28", ellipse(50.0, 60.0, 24.0, 32.0, 0.0)),
                ("#6b4a1f", poly(&[(45.0, 10.0), (55.0, 10.0), (57.0, 28.0), (43.0, 28.0)])),
            ]),
        ),
        (
            "Christmas tree",
            "Holidays",
            svg(&[
                ("#2d6a4f", format!("{} {} {}", poly(&[(50.0, 8.0), (72.0, 38.0), (28.0, 38.0)]), poly(&[(50.0, 24.0), (80.0, 62.0), (20.0, 62.0)]), poly(&[(50.0, 44.0), (90.0, 86.0), (10.0, 86.0)]))),
                ("#6b4a1f", rect(44.0, 86.0, 56.0, 97.0)),
                ("#f4d35e", star_at(50.0, 9.0, 5, 9.0, 4.0)),
                ("#d62828", format!("{} {} {}", circle(42.0, 50.0, 4.0), circle(60.0, 66.0, 4.0), circle(36.0, 76.0, 4.0))),
            ]),
        ),
        (
            "Easter egg",
            "Holidays",
            svg(&[
                ("#7ec8e3", "M 50 6 C 78 6 90 50 90 66 C 90 86 72 96 50 96 C 28 96 10 86 10 66 C 10 50 22 6 50 6 Z".into()),
                ("#f4a6c0", poly(&[(14.0, 58.0), (26.0, 50.0), (38.0, 58.0), (50.0, 50.0), (62.0, 58.0), (74.0, 50.0), (86.0, 58.0), (87.0, 70.0), (74.0, 62.0), (62.0, 70.0), (50.0, 62.0), (38.0, 70.0), (26.0, 62.0), (13.0, 70.0)])),
            ]),
        ),
        (
            "Ghost",
            "Holidays",
            svg(&[
                ("#e8e8f0", "M 20 92 L 20 44 C 20 20 34 8 50 8 C 66 8 80 20 80 44 L 80 92 L 68 82 L 56 92 L 44 82 L 32 92 Z".into()),
                ("#1a1a1a", format!("{} {} {}", circle(38.0, 42.0, 6.0), circle(62.0, 42.0, 6.0), ellipse(50.0, 62.0, 6.0, 8.0, 0.0))),
            ]),
        ),
        (
            "Gift box",
            "Holidays",
            svg(&[
                ("#d62828", rect(14.0, 42.0, 86.0, 94.0)),
                ("#b01e1e", rect(10.0, 30.0, 90.0, 44.0)),
                ("#f4d35e", rect(45.0, 30.0, 55.0, 94.0)),
                ("#f4d35e", format!("{} {}", ellipse(36.0, 20.0, 15.0, 9.0, -25.0), ellipse(64.0, 20.0, 15.0, 9.0, 25.0))),
            ]),
        ),
        ("Firework", "Holidays", svg(&[("#e63946", star_at(50.0, 50.0, 12, 47.0, 14.0)), ("#f4d35e", circle(50.0, 50.0, 10.0))])),
        // Sports
        ("Football", "Sports", svg(&[("#8b4513", ellipse(50.0, 50.0, 46.0, 28.0, -30.0)), ("#f2f2f2", format!("{} {}", bar(50.0, 50.0, 40.0, 4.0, -30.0), laces))])),
        (
            "Baseball",
            "Sports",
            svg(&[
                ("#9aa5b1", circle(50.0, 50.0, 46.0)),
                ("#f5f5f5", circle(50.0, 50.0, 42.0)),
                ("#d62828", "M 24 14 C 38 30 38 70 24 86 L 20 82 C 32 68 32 32 20 18 Z M 76 14 C 62 30 62 70 76 86 L 80 82 C 68 68 68 32 80 18 Z".into()),
            ]),
        ),
        (
            "Trophy",
            "Sports",
            svg(&[
                ("#e9c46a", "M 28 8 L 72 8 L 70 40 C 68 56 58 62 50 62 C 42 62 32 56 30 40 Z M 28 14 C 6 14 6 46 32 48 L 31 40 C 18 38 18 24 29 24 Z M 72 14 C 94 14 94 46 68 48 L 69 40 C 82 38 82 24 71 24 Z".into()),
                ("#e9c46a", rect(45.0, 60.0, 55.0, 78.0)),
                ("#b8860b", poly(&[(28.0, 94.0), (34.0, 76.0), (66.0, 76.0), (72.0, 94.0)])),
            ]),
        ),
        // Pets & animals
        (
            "Fish",
            "Pets & animals",
            svg(&[
                ("#2d7fb8", poly(&[(70.0, 50.0), (94.0, 26.0), (94.0, 74.0)])),
                ("#4aa3df", ellipse(44.0, 50.0, 34.0, 22.0, 0.0)),
                ("#2d7fb8", poly(&[(36.0, 30.0), (56.0, 28.0), (50.0, 40.0)])),
                ("#1a1a1a", circle(26.0, 45.0, 4.0)),
            ]),
        ),
        (
            "Bone",
            "Pets & animals",
            svg(&[(
                "#e8d9b5",
                format!("{} {} {} {} {}", rect(24.0, 40.0, 76.0, 60.0), circle(22.0, 38.0, 11.0), circle(22.0, 62.0, 11.0), circle(78.0, 38.0, 11.0), circle(78.0, 62.0, 11.0)),
            )]),
        ),
        (
            "Cat face",
            "Pets & animals",
            svg(&[
                ("#f4a259", format!("{} {} {}", ellipse(50.0, 60.0, 36.0, 30.0, 0.0), poly(&[(16.0, 46.0), (20.0, 8.0), (46.0, 30.0)]), poly(&[(84.0, 46.0), (80.0, 8.0), (54.0, 30.0)]))),
                ("#f7c6d0", format!("{} {}", poly(&[(24.0, 36.0), (26.0, 20.0), (38.0, 30.0)]), poly(&[(76.0, 36.0), (74.0, 20.0), (62.0, 30.0)]))),
                ("#1a1a1a", format!("{} {}", circle(36.0, 56.0, 5.0), circle(64.0, 56.0, 5.0))),
                ("#d62828", poly(&[(44.0, 66.0), (56.0, 66.0), (50.0, 73.0)])),
            ]),
        ),
        // Nature
        (
            "Butterfly",
            "Nature",
            svg(&[
                ("#e76f51", format!("{} {}", ellipse(30.0, 34.0, 24.0, 18.0, -30.0), ellipse(70.0, 34.0, 24.0, 18.0, 30.0))),
                ("#f4a261", format!("{} {}", ellipse(35.0, 68.0, 17.0, 13.0, 25.0), ellipse(65.0, 68.0, 17.0, 13.0, -25.0))),
                ("#3d2b1f", ellipse(50.0, 50.0, 5.0, 30.0, 0.0)),
            ]),
        ),
        (
            "Pine tree",
            "Nature",
            svg(&[
                ("#2a7a4b", format!("{} {} {}", poly(&[(50.0, 4.0), (70.0, 34.0), (30.0, 34.0)]), poly(&[(50.0, 22.0), (78.0, 58.0), (22.0, 58.0)]), poly(&[(50.0, 44.0), (86.0, 84.0), (14.0, 84.0)]))),
                ("#6b4a1f", rect(44.0, 84.0, 56.0, 97.0)),
            ]),
        ),
        (
            "Raindrop",
            "Nature",
            svg(&[
                ("#4aa3df", "M 50 6 C 50 6 18 46 18 64 C 18 82 32 94 50 94 C 68 94 82 82 82 64 C 82 46 50 6 50 6 Z".into()),
                ("#bfe6ff", ellipse(38.0, 66.0, 5.0, 11.0, 20.0)),
            ]),
        ),
        // Kids
        (
            "Balloon",
            "Kids",
            svg(&[
                ("#e63946", ellipse(50.0, 38.0, 28.0, 34.0, 0.0)),
                ("#e63946", poly(&[(45.0, 72.0), (55.0, 72.0), (50.0, 82.0)])),
                ("#555555", poly(&[(48.0, 82.0), (52.0, 82.0), (56.0, 97.0), (52.0, 97.0)])),
            ]),
        ),
        (
            "Rocket",
            "Kids",
            svg(&[
                ("#d62828", format!("{} {}", poly(&[(36.0, 54.0), (16.0, 80.0), (36.0, 72.0)]), poly(&[(64.0, 54.0), (84.0, 80.0), (64.0, 72.0)]))),
                ("#f4a261", poly(&[(40.0, 74.0), (60.0, 74.0), (50.0, 97.0)])),
                ("#d9dde3", "M 50 3 C 70 20 72 50 66 74 L 34 74 C 28 50 30 20 50 3 Z".into()),
                ("#4aa3df", circle(50.0, 36.0, 8.0)),
            ]),
        ),
        (
            "Ice cream",
            "Kids",
            svg(&[
                ("#d9a066", poly(&[(28.0, 52.0), (72.0, 52.0), (50.0, 97.0)])),
                ("#f7c6d0", format!("{} {}", circle(50.0, 38.0, 24.0), rect(26.0, 38.0, 74.0, 54.0))),
                ("#d62828", circle(50.0, 11.0, 6.0)),
            ]),
        ),
        // Words & frames
        (
            "Banner",
            "Words & frames",
            svg(&[
                ("#9e1b1b", format!("{} {}", poly(&[(2.0, 42.0), (26.0, 42.0), (26.0, 78.0), (2.0, 78.0), (12.0, 60.0)]), poly(&[(98.0, 42.0), (74.0, 42.0), (74.0, 78.0), (98.0, 78.0), (88.0, 60.0)]))),
                ("#d62828", rect(18.0, 28.0, 82.0, 68.0)),
            ]),
        ),
        (
            "Round frame",
            "Words & frames",
            svg(&[
                ("#457b9d", "M 6 50 A 44 44 0 1 0 94 50 A 44 44 0 1 0 6 50 Z M 22 50 A 28 28 0 1 1 78 50 A 28 28 0 1 1 22 50 Z".into()),
                ("#f4d35e", format!("{} {}", star_at(50.0, 8.0, 5, 6.0, 2.6), star_at(50.0, 92.0, 5, 6.0, 2.6))),
            ]),
        ),
    ]
}
