//! Quick Transform: big rotate and flip buttons, the aspect lock and a size in mm and inches, for
//! the selection or the whole design (Add-on 40, phase 4). Every button is an
//! `embroidery.quick.*` command; the answers' `offers` become buttons for the hoop commands.

use egui::Ui;
use serde_json::{Value, json};

use crate::VectorcraftApp;

/// The buttons: label and the turn in degrees, counter-clockwise (so 90 right is -90).
const TURNS: [(&str, f64); 5] = [("↺ 90°", 90.0), ("↻ 90°", -90.0), ("↺ 45°", 45.0), ("↻ 45°", -45.0), ("180°", 180.0)];

/// What the last quick action answered (the size, the warning, the offers), kept between frames.
#[derive(Clone, Default)]
struct Last(Value);

/// The aspect lock; locked until the user unlocks it.
#[derive(Clone, Copy)]
struct Unlocked(bool);

/// The size line: mm and inches.
pub fn size_text(r: &Value) -> Option<String> {
    let (w, h) = (r["width"].as_f64()?, r["height"].as_f64()?);
    let (wi, hi) = (r["widthIn"].as_f64().unwrap_or(w / 25.4), r["heightIn"].as_f64().unwrap_or(h / 25.4));
    Some(format!("{w:.1} × {h:.1} mm  ({wi:.2} × {hi:.2} in)"))
}

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let id = egui::Id::unique("emb-quick-last");
    let mut last: Last = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let mut unlocked: Unlocked = ui.data(|d| d.get_temp(id.with("lock"))).unwrap_or(Unlocked(false));
    let mut call: Option<(&str, Value)> = None;
    ui.label(tl!("Transform"));
    ui.horizontal_wrapped(|ui| {
        for (label, deg) in TURNS {
            if ui.add(egui::Button::new(label).min_size(egui::vec2(52.0, 30.0))).on_hover_text(tl!("Rotate")).clicked() {
                call = Some(("embroidery.quick.rotate", json!({ "degrees": deg })));
            }
        }
        for (label, axis) in [(tl!("Flip Horizontal"), "horizontal"), (tl!("Flip Vertical"), "vertical")] {
            if ui.add(egui::Button::new(label).min_size(egui::vec2(52.0, 30.0))).clicked() {
                call = Some(("embroidery.quick.flip", json!({ "axis": axis })));
            }
        }
    });
    ui.horizontal(|ui| {
        let (icon, tip) = if unlocked.0 { ("🔓", tl!("Lock")) } else { ("🔒", tl!("Unlock")) };
        if ui.button(icon).on_hover_text(format!("{}: {tip}", tl!("Aspect Ratio"))).clicked() {
            unlocked.0 = !unlocked.0;
        }
        if let Some(t) = size_text(&last.0) {
            ui.label(t);
        }
    });
    if let (Some(mut w), Some(mut h)) = (last.0["width"].as_f64(), last.0["height"].as_f64()) {
        ui.horizontal(|ui| {
            ui.label(tl!("Size"));
            let a = ui.add(egui::DragValue::new(&mut w).range(1.0..=2000.0).speed(0.5).suffix(" mm"));
            ui.label("×");
            let b = ui.add(egui::DragValue::new(&mut h).range(1.0..=2000.0).speed(0.5).suffix(" mm"));
            let lock = !unlocked.0;
            if a.drag_stopped() || (a.changed() && !a.dragged()) {
                call = Some(("embroidery.quick.resize", json!({ "width": w, "unit": "mm", "lock": lock })));
            } else if b.drag_stopped() || (b.changed() && !b.dragged()) {
                call = Some(("embroidery.quick.resize", json!({ "height": h, "unit": "mm", "lock": lock })));
            }
        });
    }
    if let Some(w) = last.0["warning"].as_str() {
        ui.colored_label(ui.visuals().warn_fg_color, w);
    }
    if last.0["fits"] == json!(false) {
        ui.horizontal_wrapped(|ui| {
            let offers = last.0["offers"].as_array().cloned().unwrap_or_default();
            if offers.iter().any(|o| o == "rotateHoop") && ui.button(tl!("Rotate Hoop")).clicked() {
                call = Some(("embroidery.hoop.rotate", json!({})));
            }
            if offers.iter().any(|o| o == "fitToHoop") && ui.button(tl!("Fit")).clicked() {
                call = Some(("embroidery.hoop.fit", json!({})));
            }
        });
    }
    ui.data_mut(|d| d.insert_temp(id.with("lock"), unlocked));
    if let Some((cmd, params)) = call {
        match app.run(cmd, params) {
            Ok(r) => last = Last(r),
            Err(e) => app.status(e),
        }
    }
    ui.data_mut(|d| d.insert_temp(id, last));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_shows_in_mm_and_inches() {
        let t = size_text(&json!({"width": 25.4, "height": 50.8, "widthIn": 1.0, "heightIn": 2.0})).unwrap();
        assert_eq!(t, "25.4 × 50.8 mm  (1.00 × 2.00 in)");
        assert!(size_text(&json!({})).is_none());
    }

    #[test]
    fn the_panel_draws_and_the_turns_are_quarter_and_eighth_steps() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
        let ctx = egui::Context::default();
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 800.0))), ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| show(&mut app, ui));
        out.textures_delta.clear();
        let sum: f64 = TURNS.iter().map(|t| t.1).sum();
        assert_eq!(sum, 180.0);
    }
}
