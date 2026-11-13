//! The embroidery font picker, the one place every font is chosen (the Lettering panel, Add Words
//! and the Lettering Kiosk): a dropdown whose entries are drawn in their own fonts, and under it a
//! live preview of the user's actual text in the chosen font (lines, justification, sizes and
//! spacing as they will sew). The drawings come from `embroidery.font.preview`, which lays the
//! text out as the lettering command does, so what the picker shows is what is placed.

use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, vec2};
use serde_json::{Value, json};
use vectorcraft_embroidery::sf_core::attr::LetterAlign;

use crate::VectorcraftApp;
use crate::theme::Tokens;

/// The most drawings kept; past it the cache starts over.
const MAX_CACHED: usize = 400;
/// The most letters of the user's text drawn in a dropdown entry.
const ENTRY_LETTERS: usize = 14;
const ENTRY_W: f32 = 270.0;
const ENTRY_H: f32 = 30.0;
const LIVE_H: f32 = 84.0;

/// Text drawn in a font: polylines in mm and their bounds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    pub lines: Vec<Vec<[f32; 2]>>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// What the live preview draws: the user's text and how it is set.
#[derive(Clone, Copy, Debug)]
pub struct Sample<'a> {
    pub text: &'a str,
    pub align: LetterAlign,
    pub leading: f64,
    pub size: f64,
    pub line_sizes: &'a [f64],
}

impl Sample<'_> {
    fn align_name(&self) -> &'static str {
        match self.align {
            LetterAlign::Left => "left",
            LetterAlign::Centre => "centre",
            LetterAlign::Right => "right",
        }
    }
}

fn parse(v: &Value) -> Option<Preview> {
    let f = |k: &str| v[k].as_f64().filter(|n| n.is_finite()).map(|n| n as f32);
    let lines = v["lines"]
        .as_array()?
        .iter()
        .map(|l| {
            l.as_array()
                .map(|pts| {
                    pts.iter()
                        .filter_map(|p| Some([p.get(0)?.as_f64()? as f32, p.get(1)?.as_f64()? as f32]))
                        .filter(|p| p[0].is_finite() && p[1].is_finite())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();
    Some(Preview { lines, x: f("x")?, y: f("y")?, w: f("width")?, h: f("height")? })
}

/// `font`'s drawing of the text `params` say (cached; None if the font can't draw it).
pub(crate) fn preview(app: &mut VectorcraftApp, font: &str, mut params: Value) -> Option<Preview> {
    params["font"] = json!(font);
    let key = params.to_string();
    if let Some(p) = app.embroidery.font_previews.get(&key) {
        return p.clone();
    }
    if app.embroidery.font_previews.len() >= MAX_CACHED {
        app.embroidery.font_previews.clear();
    }
    // (a font that can't draw this text stays blank: remembered, so it isn't asked every frame)
    let drawn = app.run("embroidery.font.preview", params).ok().and_then(|v| parse(&v));
    app.embroidery.font_previews.insert(key, drawn.clone());
    drawn
}

/// `p` drawn into `rect`, as large as fits and centred.
pub(crate) fn paint(ui: &Ui, rect: Rect, p: &Preview, color: Color32) {
    if p.w <= 0.0 && p.h <= 0.0 {
        return;
    }
    let k = (rect.width() / p.w.max(0.001)).min(rect.height() / p.h.max(0.001));
    if !k.is_finite() || k <= 0.0 {
        return;
    }
    let origin = rect.center() - vec2((p.x + p.w / 2.0) * k, (p.y + p.h / 2.0) * k);
    let stroke = Stroke::new(1.0, color);
    for line in &p.lines {
        let pts: Vec<Pos2> = line.iter().map(|q| origin + vec2(q[0] * k, q[1] * k)).collect();
        ui.painter().add(Shape::line(pts, stroke));
    }
}

/// The text a dropdown entry is drawn with: the user's first line, else the font's name.
fn entry_text(sample: &Sample, font: &str) -> String {
    let first = sample.text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    if first.is_empty() { font.to_string() } else { first.chars().take(ENTRY_LETTERS).collect() }
}

/// The font picker: the dropdown (`label` beside it) with every font drawn in itself, and the
/// user's text in the chosen font under it. → The font chosen, as an index into `names`.
pub(crate) fn font_picker(
    app: &mut VectorcraftApp,
    ui: &mut Ui,
    id: &str,
    label: &str,
    names: &[String],
    current: usize,
    sample: &Sample,
) -> Option<usize> {
    let t = Tokens::get(ui.ctx());
    let shown = names.get(current).cloned().unwrap_or_default();
    let mut chosen = None;
    crate::widgets::label_row(ui, label, 112.0, |ui| {
        chosen = crate::widgets::combo(ui, ("font-picker", id), &shown, 150.0, |ui| {
            let mut pick = None;
            egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    let (rect, r) = ui.allocate_exact_size(vec2(ENTRY_W, ENTRY_H), Sense::click());
                    if i == current {
                        ui.painter().rect_filled(rect, 3.0, t.hover);
                    } else if r.hovered() {
                        ui.painter().rect_filled(rect, 3.0, t.hover.gamma_multiply(0.6));
                    }
                    ui.painter().text(
                        rect.left_center() + vec2(6.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        name,
                        egui::FontId::proportional(11.5),
                        t.text_dim,
                    );
                    // the entry drawn in its own font
                    let one = json!({ "text": entry_text(sample, name), "size": 10, "align": "centre" });
                    if let Some(p) = preview(app, name, one) {
                        let art =
                            Rect::from_min_max(Pos2::new(rect.left() + 118.0, rect.top() + 3.0), Pos2::new(rect.right() - 6.0, rect.bottom() - 3.0));
                        paint(ui, art, &p, t.text);
                    }
                    if r.on_hover_text(name).clicked() {
                        pick = Some(i);
                    }
                }
            });
            pick
        })
        .filter(|i| *i != current);
    });
    // the user's text in the font chosen now (or the one just picked)
    if let Some(font) = names.get(chosen.unwrap_or(current)) {
        let text = if sample.text.trim().is_empty() { font.as_str() } else { sample.text };
        let mut params =
            json!({ "text": text, "size": sample.size.clamp(1.0, 500.0), "leading": sample.leading.clamp(0.2, 5.0), "align": sample.align_name() });
        if !sample.line_sizes.is_empty() {
            params["lineSizes"] = json!(sample.line_sizes);
        }
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().max(60.0), LIVE_H), Sense::hover());
        ui.painter().rect(rect, 3.0, t.input, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        if let Some(p) = preview(app, font, params) {
            paint(ui, rect.shrink(6.0), &p, t.text);
        }
    }
    chosen
}
