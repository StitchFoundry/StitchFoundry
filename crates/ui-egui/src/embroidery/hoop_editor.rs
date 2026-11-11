//! Hoops of one's own: the list (Edit, Delete, Export), the form (name, shape, size in mm or
//! inches, corner radius, outer frame), Add to My Machine and Import Hoop. Used by the Embroidery
//! panel and by Make a Design's hoop step. Every action is an `embroidery.hoop.*` command.

use egui::Ui;
use serde_json::{Value, json};

use crate::VectorcraftApp;

/// File type of an exported hoop.
const HOOP_EXT: &str = "json";

/// The form as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct HoopDraft {
    pub name: String,
    /// 0 rectangle, 1 oval, 2 square, 3 round.
    pub shape: usize,
    pub inches: bool,
    pub width: f64,
    pub height: f64,
    pub corner_radius: f64,
    pub frame_width: f64,
    pub frame_height: f64,
    /// The hoop being edited (its name before the edit).
    pub was: Option<String>,
    /// Which machine of the drop-down "Add to My Machine" uses.
    pub machine: usize,
    pub open: bool,
    pub error: Option<String>,
}

impl Default for HoopDraft {
    fn default() -> Self {
        HoopDraft {
            name: String::new(),
            shape: 0,
            inches: false,
            width: 150.0,
            height: 150.0,
            corner_radius: 0.0,
            frame_width: 0.0,
            frame_height: 0.0,
            was: None,
            machine: 0,
            open: false,
            error: None,
        }
    }
}

const SHAPES: [&str; 4] = ["rect", "oval", "square", "round"];

impl HoopDraft {
    /// The `embroidery.hoop.saveCustom` params for the form.
    pub fn params(&self) -> Value {
        let shape = SHAPES.get(self.shape).copied().unwrap_or("rect");
        let mut p = json!({
            "name": self.name.trim(),
            "width": self.width,
            "unit": if self.inches { "in" } else { "mm" },
            "shape": shape,
        });
        if matches!(shape, "rect" | "oval") {
            p["height"] = json!(self.height);
        }
        if shape == "rect" && self.corner_radius > 0.0 {
            p["cornerRadius"] = json!(self.corner_radius);
        }
        if self.frame_width > 0.0 && self.frame_height > 0.0 {
            p["frameWidth"] = json!(self.frame_width);
            p["frameHeight"] = json!(self.frame_height);
        }
        if let Some(was) = &self.was {
            p["was"] = json!(was);
        }
        p
    }

    /// The form filled in from a hoop of one's own (`embroidery.myMachine`'s `own`, millimetres).
    pub fn edit(h: &Value) -> Self {
        let num = |k: &str| h[k].as_f64().unwrap_or(0.0);
        let shape = match h["shape"].as_str() {
            Some("oval") => 1,
            _ => 0,
        };
        let frame = h["frame"].as_array().map(|f| (f.first().and_then(Value::as_f64), f.get(1).and_then(Value::as_f64)));
        let (fw, fh) = match frame {
            Some((Some(a), Some(b))) => (a, b),
            _ => (0.0, 0.0),
        };
        let name = h["name"].as_str().unwrap_or_default().to_string();
        HoopDraft {
            was: Some(name.clone()),
            name,
            shape,
            width: num("width"),
            height: num("height"),
            corner_radius: num("cornerRadius"),
            frame_width: fw,
            frame_height: fh,
            open: true,
            ..Default::default()
        }
    }

    /// Switch the unit, converting the typed sizes.
    fn set_inches(&mut self, inches: bool) {
        if inches == self.inches {
            return;
        }
        let k = if inches { 1.0 / 25.4 } else { 25.4 };
        for v in [&mut self.width, &mut self.height, &mut self.corner_radius, &mut self.frame_width, &mut self.frame_height] {
            *v *= k;
        }
        self.inches = inches;
    }
}

/// The hoops of one's own and the machines to put them on, from `embroidery.myMachine`.
fn lists(app: &mut VectorcraftApp) -> (Vec<Value>, Vec<String>) {
    let mine = app.session.execute("embroidery.myMachine", &json!({})).unwrap_or_default();
    let own = mine["own"].as_array().cloned().unwrap_or_default();
    let machines = mine["machines"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(str::to_string)).collect()).unwrap_or_default();
    (own, machines)
}

/// Export Hoop…: the hoop `name` written to a file the user picks.
pub fn export(app: &mut VectorcraftApp, name: &str) {
    let stem: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    let pick =
        crate::FilePick { name: format!("{stem}.{HOOP_EXT}"), filters: vec![crate::FilePick::filter("Hoop", &[HOOP_EXT])], ..Default::default() };
    if let Some(path) = crate::picks::save(app, &pick) {
        match app.run("embroidery.hoop.export", json!({ "name": name, "path": path })) {
            Ok(_) => app.status(format!("{}: {name}", tl!("Hoop saved"))),
            Err(e) => app.status(e),
        }
    }
}

/// Import Hoop…: a hoop file the user picks, added to one's own hoops.
pub fn import(app: &mut VectorcraftApp) {
    let pick = crate::FilePick { filters: vec![crate::FilePick::filter("Hoop", &[HOOP_EXT])], ..Default::default() };
    if let Some(path) = crate::picks::open(app, &pick) {
        match app.run("embroidery.hoop.import", json!({ "path": path })) {
            Ok(r) => app.status(format!("{}: {}", tl!("Hoop imported"), r["name"].as_str().unwrap_or_default())),
            Err(e) => app.status(e),
        }
    }
}

/// The whole editor: a list of one's own hoops and the form. `select` is true where saving a hoop
/// should also make it the open design's hoop (the Embroidery panel).
pub fn show(app: &mut VectorcraftApp, ui: &mut Ui, select: bool) {
    let (own, machines) = lists(app);
    if own.is_empty() {
        ui.weak(tl!("No hoops of your own yet."));
    }
    for h in &own {
        let Some(name) = h["name"].as_str() else { continue };
        let attached: Vec<&str> = h["machines"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            let size = format!("{} × {} mm", h["width"].as_f64().unwrap_or(0.0).round(), h["height"].as_f64().unwrap_or(0.0).round());
            ui.label(format!("{name} ({size})"));
            if ui.small_button(tl!("Edit")).clicked() {
                app.embroidery.hoop_form = HoopDraft::edit(h);
            }
            if ui.small_button(tl!("Delete")).clicked()
                && let Err(e) = app.run("embroidery.hoop.deleteCustom", json!({ "name": name }))
            {
                app.status(e);
            }
            if ui.small_button(tl!("Export…")).clicked() {
                export(app, name);
            }
        });
        if !attached.is_empty() {
            ui.weak(format!("{}: {}", tl!("On machines"), attached.join(", ")));
        }
        if !machines.is_empty() {
            let mut d = std::mem::take(&mut app.embroidery.hoop_form);
            ui.horizontal(|ui| {
                let cur = machines.get(d.machine).cloned().unwrap_or_default();
                craft_ui::ComboBox::from_id_salt(("hoop-machine", name)).selected_text(cur).width(150.0).show_ui(ui, |ui| {
                    for (i, m) in machines.iter().enumerate() {
                        ui.selectable_value(&mut d.machine, i, m);
                    }
                });
                let target = machines.get(d.machine).cloned();
                if let Some(m) = target {
                    let on = attached.contains(&m.as_str());
                    let label = if on { tl!("Remove from My Machine") } else { tl!("Add to My Machine") };
                    if ui.small_button(label).clicked()
                        && let Err(e) = app.run("embroidery.hoop.attach", json!({ "name": name, "machine": m, "attach": !on }))
                    {
                        app.status(e);
                    }
                }
            });
            app.embroidery.hoop_form = d;
        }
    }
    ui.horizontal(|ui| {
        if ui.button(tl!("New Hoop")).clicked() {
            app.embroidery.hoop_form = HoopDraft { open: true, ..Default::default() };
        }
        if ui.button(tl!("Import Hoop…")).clicked() {
            import(app);
        }
    });
    form(app, ui, select);
}

fn form(app: &mut VectorcraftApp, ui: &mut Ui, select: bool) {
    if !app.embroidery.hoop_form.open {
        return;
    }
    let mut d = std::mem::take(&mut app.embroidery.hoop_form);
    let mut save = false;
    let mut cancel = false;
    ui.separator();
    ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(180.0).hint_text(tl!("Hoop name")));
    ui.horizontal(|ui| {
        for (i, label) in [tl!("Rectangle"), tl!("Oval"), tl!("Square"), tl!("Round")].into_iter().enumerate() {
            ui.selectable_value(&mut d.shape, i, label);
        }
    });
    let mut inches = d.inches;
    ui.horizontal(|ui| {
        ui.selectable_value(&mut inches, false, "mm");
        ui.selectable_value(&mut inches, true, "in");
    });
    d.set_inches(inches);
    let (unit, hi) = if d.inches { (" in", 80.0) } else { (" mm", 2000.0) };
    let two = d.shape < 2;
    ui.horizontal(|ui| {
        ui.label(if two { tl!("Sewing field") } else { tl!("Size") });
        ui.add(egui::DragValue::new(&mut d.width).range(0.0..=hi).speed(1.0).suffix(unit));
        if two {
            ui.label("×");
            ui.add(egui::DragValue::new(&mut d.height).range(0.0..=hi).speed(1.0).suffix(unit));
        }
    });
    if d.shape == 0 {
        ui.horizontal(|ui| {
            ui.label(tl!("Corner radius"));
            ui.add(egui::DragValue::new(&mut d.corner_radius).range(0.0..=hi).speed(0.5).suffix(unit));
        });
    }
    ui.horizontal(|ui| {
        ui.label(tl!("Outer frame (0: unknown)"));
        ui.add(egui::DragValue::new(&mut d.frame_width).range(0.0..=hi * 1.5).speed(1.0).suffix(unit));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut d.frame_height).range(0.0..=hi * 1.5).speed(1.0).suffix(unit));
    });
    if let Some(e) = &d.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    ui.horizontal(|ui| {
        if !d.name.trim().is_empty() && ui.button(tl!("Save Hoop")).clicked() {
            save = true;
        }
        if ui.button(tl!("Cancel")).clicked() {
            cancel = true;
        }
    });
    if save {
        let name = d.name.trim().to_string();
        match app.run("embroidery.hoop.saveCustom", d.params()) {
            Ok(_) => {
                if select {
                    app.run("embroidery.settings", json!({ "hoop": name })).ok();
                }
                d.open = false;
                d.error = None;
                d.was = None;
            }
            Err(e) => d.error = Some(e),
        }
    }
    if cancel {
        d.open = false;
        d.error = None;
        d.was = None;
    }
    app.embroidery.hoop_form = d;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_asks_for_what_the_command_takes() {
        let d = HoopDraft { name: " Hat ".into(), shape: 0, width: 200.0, height: 100.0, corner_radius: 5.0, ..Default::default() };
        let p = d.params();
        assert_eq!(p["name"], "Hat");
        assert_eq!(p["unit"], "mm");
        assert_eq!(p["cornerRadius"], 5.0);
        let round = HoopDraft { name: "R".into(), shape: 3, width: 150.0, ..Default::default() };
        assert!(round.params().get("height").is_none());
    }

    #[test]
    fn a_hoop_is_made_attached_exported_and_imported_from_the_form() {
        let dir = std::env::temp_dir().join(format!("sf-hoop-ui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Hat.json").to_string_lossy().to_string();
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
        let d = HoopDraft { name: "Hat".into(), width: 8.0, height: 4.0, inches: true, corner_radius: 0.5, ..Default::default() };
        app.run("embroidery.hoop.saveCustom", d.params()).unwrap();
        let mine = app.run("embroidery.myMachine", json!({})).unwrap();
        let machine = mine["machines"][0]["name"].as_str().unwrap().to_string();
        app.run("embroidery.hoop.attach", json!({"name": "Hat", "machine": machine})).unwrap();
        // the editor draws the list with Edit, Delete and Export, and the form when open
        app.embroidery.hoop_form = HoopDraft::edit(&app.run("embroidery.myMachine", json!({})).unwrap()["own"][0]);
        assert!((app.embroidery.hoop_form.corner_radius - 12.7).abs() < 1e-6, "Edit keeps the corner radius");
        let ctx = egui::Context::default();
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 800.0))), ..Default::default() };
        let _ = ctx.run_ui(raw, |ui| show(&mut app, ui, false));
        // Export, delete, Import
        let picked = path.clone();
        app.services.pick_save = Some(Box::new(move |_: &crate::FilePick| Some(picked.clone())));
        export(&mut app, "Hat");
        assert!(std::path::Path::new(&path).exists());
        app.run("embroidery.hoop.deleteCustom", json!({"name": "Hat"})).unwrap();
        let picked = path.clone();
        app.services.pick_open = Some(Box::new(move |_: &crate::FilePick| Some(picked.clone())));
        import(&mut app);
        let own = app.run("embroidery.myMachine", json!({})).unwrap()["own"].clone();
        assert_eq!(own[0]["name"], "Hat");
        assert!((own[0]["width"].as_f64().unwrap() - 203.2).abs() < 1e-6);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn changing_the_unit_converts_the_sizes() {
        let mut d = HoopDraft { width: 254.0, ..Default::default() };
        d.set_inches(true);
        assert!((d.width - 10.0).abs() < 1e-9 && d.inches);
        d.set_inches(false);
        assert!((d.width - 254.0).abs() < 1e-9);
    }

    #[test]
    fn editing_fills_the_form_from_a_hoop() {
        let d = HoopDraft::edit(&json!({"name": "Mine", "width": 120, "height": 80, "shape": "oval"}));
        assert_eq!((d.was.as_deref(), d.shape, d.width, d.height, d.open), (Some("Mine"), 1, 120.0, 80.0, true));
    }
}
