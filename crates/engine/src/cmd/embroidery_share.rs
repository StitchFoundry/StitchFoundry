//! The share image: the finished design on a plain background at a social-post size (1080 × 1080
//! or 1080 × 1350), PNG, with a small "Made with …" credit under it that can be switched off.
//! Offline: it only draws and writes a file (or returns the bytes); nothing is uploaded or tracked.

use serde_json::{Value, json};

use super::{CommandSpec, EngineError, Result, Session, bad, cmd, has_doc, str_param};

/// Whether the credit is drawn when `footer` isn't given (the issue: on by default).
const CREDIT_DEFAULT: bool = true;
/// The plain product URL (no tracking parameters).
const PRODUCT_URL: &str = "https://github.com/StitchFoundry/StitchFoundry";
const PRODUCT_NAME: &str = "StitchFoundry";
const WIDTH: u32 = 1080;
/// Space round the design, and under it for the credit.
const PAD: u32 = 80;
const FOOTER_BAND: u32 = 110;
const MAX_URL: usize = 120;

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "embroidery.exportShareImage",
        "Save Share Image",
        [],
        None,
        "{path?: a .png file to write (without it: the bytes), size?: square|portrait (square: 1080×1080, portrait: 1080×1350), footer?: whether to draw the small \"Made with StitchFoundry\" credit and link (true), url?: the link shown in the credit (the plain product URL), view?: trueview|flat} the finished design on a plain white background, centred, as a PNG ready to post; offline, nothing is uploaded → {path?, dataBase64?, format, bytes, width, height, footer}",
        has_doc,
        export_share_image
    )]
}

/// A 5 × 7 glyph (one bit a column, top row first) for the credit; unknown characters are blank.
fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '/' => [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        _ => [0; 7],
    }
}

/// The credit line, e.g. "Made with StitchFoundry - github.com/…" (scheme dropped).
fn credit_text(url: &str) -> String {
    let shown = url.trim().trim_start_matches("https://").trim_start_matches("http://");
    format!("Made with {PRODUCT_NAME} - {shown}")
}

/// Draw `text` centred on `cy` in grey, as large as fits in 90% of the width (1 to 4 times).
fn draw_credit(img: &mut image::RgbaImage, text: &str, cy: u32) {
    let n = text.chars().count().max(1) as u32;
    let scale = (img.width() * 9 / 10 / (n * 6)).clamp(1, 4);
    let total = n * 6 * scale - scale;
    let x0 = img.width().saturating_sub(total) / 2;
    let y0 = cy.saturating_sub(7 * scale / 2);
    for (i, c) in text.chars().enumerate() {
        for (row, bits) in glyph(c).iter().enumerate() {
            for col in 0..5u32 {
                if bits >> (4 - col) & 1 == 0 {
                    continue;
                }
                let (px, py) = (x0 + (i as u32 * 6 + col) * scale, y0 + row as u32 * scale);
                for dy in 0..scale {
                    for dx in 0..scale {
                        if px + dx < img.width() && py + dy < img.height() {
                            img.put_pixel(px + dx, py + dy, image::Rgba([102, 102, 102, 255]));
                        }
                    }
                }
            }
        }
    }
}

fn export_share_image(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "embroidery.exportShareImage";
    let height = match str_param(p, "size").unwrap_or("square") {
        "square" => 1080,
        "portrait" => 1350,
        _ => return Err(bad(CMD, "`size` is square or portrait")),
    };
    let footer = match p.get("footer") {
        None | Some(Value::Null) => CREDIT_DEFAULT,
        Some(v) => v.as_bool().ok_or_else(|| bad(CMD, "`footer` is true or false"))?,
    };
    let url = match str_param(p, "url") {
        None => PRODUCT_URL,
        Some(u) if !u.trim().is_empty() && u.chars().count() <= MAX_URL => u,
        Some(_) => return Err(bad(CMD, "`url` is 1 to 120 characters")),
    };
    let flat = match str_param(p, "view").unwrap_or("trueview") {
        "trueview" => false,
        "flat" => true,
        _ => return Err(bad(CMD, "`view` is trueview or flat")),
    };
    let plan = s.embroidery_plan().ok_or_else(|| bad(CMD, "no document"))?;
    if plan.is_empty() {
        return Err(bad(CMD, "nothing is stitched yet: give objects a stitch type first"));
    }
    // the design as large as the box between the paddings allows (never magnified past 40 px/mm)
    let bottom = if footer { FOOTER_BAND } else { PAD };
    let (box_w, box_h) = (f64::from(WIDTH - 2 * PAD), f64::from(height - PAD - bottom));
    let b = plan.pattern().bounds();
    let ppmm = (box_w / b.width().max(0.1)).min(box_h / b.height().max(0.1)).clamp(1.0, 40.0);
    let design = super::embroidery_image::render(plan.pattern(), ppmm, 0.0, [255, 255, 255, 255], flat).map_err(|e| bad(CMD, e))?;
    let mut img = image::RgbaImage::from_pixel(WIDTH, height, image::Rgba([255, 255, 255, 255]));
    let x = i64::from(PAD) + (box_w as i64 - i64::from(design.width())) / 2;
    let y = i64::from(PAD) + (box_h as i64 - i64::from(design.height())) / 2;
    image::imageops::overlay(&mut img, &design, x, y);
    if footer {
        draw_credit(&mut img, &credit_text(url), height - FOOTER_BAND / 2);
    }
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|e| EngineError::Other(format!("PNG encoding failed: {e}")))?;
    let mut r = json!({ "format": "png", "bytes": bytes.len(), "width": WIDTH, "height": height, "footer": footer });
    match str_param(p, "path").map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => {
            super::fileio::write_file(path, &bytes)?;
            r["path"] = json!(path);
        }
        None => r["dataBase64"] = json!(vectorcraft_format::base64_encode(&bytes)),
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(r: &Value) -> image::RgbaImage {
        let bytes = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
        image::load_from_memory(&bytes).unwrap().to_rgba8()
    }

    /// Pixels that aren't the plain background in the bottom band.
    fn footer_ink(img: &image::RgbaImage) -> usize {
        let top = img.height() - FOOTER_BAND;
        img.enumerate_pixels().filter(|(_, y, p)| *y >= top && p.0 != [255, 255, 255, 255]).count()
    }

    #[test]
    fn sizes_and_credit_on_or_off() {
        let mut s = Session::new();
        s.execute("embroidery.sample", &json!({"id": "heart-patch"})).unwrap();
        let sq = s.execute("embroidery.exportShareImage", &json!({})).unwrap();
        assert_eq!((sq["width"].as_u64(), sq["height"].as_u64(), sq["footer"].as_bool()), (Some(1080), Some(1080), Some(true)));
        let on = decode(&sq);
        assert_eq!(on.dimensions(), (1080, 1080));
        assert!(footer_ink(&on) > 200, "the credit is drawn by default");
        let off = decode(&s.execute("embroidery.exportShareImage", &json!({"footer": false})).unwrap());
        assert_eq!(footer_ink(&off), 0, "no credit when switched off");
        let tall = decode(&s.execute("embroidery.exportShareImage", &json!({"size": "portrait"})).unwrap());
        assert_eq!(tall.dimensions(), (1080, 1350));
        assert!(footer_ink(&tall) > 200);
        // the design itself is drawn in the middle
        assert!(on.get_pixel(540, 400).0 != [255, 255, 255, 255] || on.get_pixel(540, 540).0 != [255, 255, 255, 255]);
    }

    #[test]
    fn rejects_bad_params_and_empty_designs() {
        let mut s = Session::new();
        assert!(s.execute("embroidery.exportShareImage", &json!({})).is_err(), "no stitches");
        s.execute("embroidery.sample", &json!({"id": "heart-patch"})).unwrap();
        for bad in [json!({"size": "wide"}), json!({"footer": "yes"}), json!({"url": ""}), json!({"view": "x"})] {
            assert!(s.execute("embroidery.exportShareImage", &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn credit_names_the_plain_url() {
        assert_eq!(credit_text("https://example.org/x"), "Made with StitchFoundry - example.org/x");
    }
}
