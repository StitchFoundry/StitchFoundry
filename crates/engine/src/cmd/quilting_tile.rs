//! Quilting tile designer (issue #199, slice 1): `quilt.tile` makes one continuous, non-crossing
//! meander tile as stitches and writes it as a machine file (DST by default). The geometry is
//! `sf_core::quilt_tile`'s.

use serde_json::{Value, json};
use sf_core::pattern::Pattern;
use sf_core::quilt_tile;

use super::{CommandSpec, EngineError, Result, Session, always, bad, bool_or, cmd, number_or, str_param};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        query "quilt.tile",
        "Quilting Tile…",
        ["Embroidery", "Longarm Quilting"],
        None,
        "{width?: mm (152.4 = 6 in; 40 to 1000), height?: mm (152.4), seed?: whole number (1; the same seed gives the same design), format?: a machine format (dst), spirals?: bool (true: small double spirals, about 11 mm, hang off the line; false: the plain meander), center?: bool (false: the top-left start corner is the file's origin, so the needle starts on the line; true: the tile's middle is, and the machine jumps to the start), path?: file to write (left out: the file comes back as dataBase64 only)} one continuous line meandering in round curves over the tile, in at the top-left corner and out at the bottom-right, never crossing or touching itself, about 2 mm stitches (the cells stay the same size on every tile size) → {path?, dataBase64, format, bytes, stitches, seed (the first seed from the one given whose line stays clear of itself), widthMm, heightMm}",
        always,
        tile
    )]
}

fn tile(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "quilt.tile";
    let (width, height) = (number_or(p, "width", 152.4, CMD)?, number_or(p, "height", 152.4, CMD)?);
    let seed = number_or(p, "seed", 1.0, CMD)?;
    if !seed.is_finite() || !(0.0..=9.0e15).contains(&seed) {
        return Err(bad(CMD, "`seed` is a whole number from 0"));
    }
    let key = str_param(p, "format").unwrap_or("dst").trim_start_matches('.').to_ascii_lowercase();
    let format =
        sf_formats::Format::from_extension(&key).filter(|f| f.writable()).ok_or_else(|| bad(CMD, format!("`format` can't be written: {key}")))?;
    let make = if bool_or(p, "spirals", true) { quilt_tile::tile_line_spirals } else { quilt_tile::tile_line };
    let (line, used) = make(width, height, seed as u64).ok_or_else(|| {
        bad(
            CMD,
            format!("`width` and `height` are {} to {} mm, and the line must come out clear of itself", quilt_tile::MIN_SIDE, quilt_tile::MAX_SIDE),
        )
    })?;
    let mut pattern = Pattern::new();
    pattern.name = format!("Quilt tile {width:.0}x{height:.0}");
    for &pt in &line {
        pattern.stitch(pt);
    }
    pattern.ensure_end();
    pattern.fill_missing_threads();
    let name = format!("quilt-tile-{}x{}-seed{used}", width.round(), height.round());
    // the start corner is the file's origin, so the needle starts on the line: no lead-in jump or trim
    let opts = sf_formats::WriteOptions { center: bool_or(p, "center", false), ..sf_formats::WriteOptions::named(name) };
    let bytes = sf_formats::write(format, &pattern, &opts).map_err(|e| EngineError::Other(format!("{} export failed: {e}", format.name())))?;
    let mut out = json!({
        "dataBase64": vectorcraft_format::base64_encode(&bytes),
        "format": format.extension(),
        "bytes": bytes.len(),
        "stitches": pattern.stitch_count(),
        "seed": used,
        "widthMm": width,
        "heightMm": height,
    });
    if let Some(path) = str_param(p, "path").map(str::trim).filter(|x| !x.is_empty()) {
        super::fileio::write_file(path, &bytes)?;
        out["path"] = json!(path);
    }
    let _ = s;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_written_dst_reads_back_with_the_tile_size_and_short_stitches() {
        let mut s = Session::new();
        let r = s.execute("quilt.tile", &json!({"width": 152.4, "height": 203.2, "seed": 4})).unwrap();
        let bytes = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
        let pattern = sf_formats::read(sf_formats::Format::Dst, &bytes).unwrap();
        assert_eq!(pattern.stitch_count() as u64, r["stitches"].as_u64().unwrap());
        let n = |c| pattern.count(c);
        assert_eq!((pattern.color_blocks().len(), n(sf_core::pattern::Cmd::Jump), n(sf_core::pattern::Cmd::Trim)), (1, 0, 0));
        let b = pattern.bounds();
        assert!(((b.max.x - b.min.x) - 152.4).abs() < 1.0 && ((b.max.y - b.min.y) - 203.2).abs() < 1.0);
        let pts: Vec<_> = pattern.stitches.iter().filter(|st| st.cmd == sf_core::pattern::Cmd::Stitch).map(|st| st.pos()).collect();
        assert!(pts.windows(2).all(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y) <= 2.4));
        assert!(!sf_core::quilt_tile::self_touches(&pts));
    }

    #[test]
    fn bad_sizes_and_formats_are_refused() {
        let mut s = Session::new();
        assert!(s.execute("quilt.tile", &json!({"width": 5})).is_err());
        assert!(s.execute("quilt.tile", &json!({"width": "big"})).is_err());
        assert!(s.execute("quilt.tile", &json!({"format": "svg"})).is_err());
        assert!(s.execute("quilt.tile", &json!({"seed": -1})).is_err());
    }
}
