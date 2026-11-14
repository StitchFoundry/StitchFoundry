//! Machine profiles: the list (Edit, Delete or Reset) and the form (name, file format, longest
//! stitch and jump, speed, trim time). Every action is an `embroidery.machine.*` command; the
//! profiles are kept with the preferences (`vectorcraft_engine::profile_sync`).

use egui::Ui;
use serde_json::{Value, json};

use crate::VectorcraftApp;

/// The form as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct MachineDraft {
    pub name: String,
    /// The file format (`dst`, `pes`…); the profile of one's own starts from its format's.
    pub format: String,
    pub max_stitch: f64,
    pub max_jump: f64,
    pub speed: f64,
    pub trim_seconds: f64,
    /// The profile being edited (its name before the edit); none for a new one.
    pub was: Option<String>,
    /// A standard profile keeps its name and format.
    pub standard: bool,
    pub open: bool,
    pub error: Option<String>,
}

impl Default for MachineDraft {
    fn default() -> Self {
        MachineDraft {
            name: String::new(),
            format: "dst".into(),
            max_stitch: 12.1,
            max_jump: 12.1,
            speed: 800.0,
            trim_seconds: 5.0,
            was: None,
            standard: false,
            open: false,
            error: None,
        }
    }
}

impl MachineDraft {
    /// The `embroidery.machine.save` params for the form.
    pub fn params(&self) -> Value {
        let mut p = json!({
            "name": self.was.as_deref().unwrap_or(self.name.trim()),
            "maxStitch": self.max_stitch,
            "maxJump": self.max_jump,
            "speed": self.speed,
            "trimSeconds": self.trim_seconds,
        });
        if !self.standard {
            p["format"] = json!(self.format);
        }
        if let Some(was) = &self.was
            && !self.standard
            && self.name.trim() != was
        {
            p["newName"] = json!(self.name.trim());
        }
        p
    }

    /// The form filled in from a profile of `embroidery.machine.list`.
    pub fn edit(m: &Value) -> Self {
        let num = |k: &str, d: f64| m[k].as_f64().unwrap_or(d);
        let name = m["name"].as_str().unwrap_or_default().to_string();
        let base = MachineDraft::default();
        MachineDraft {
            was: Some(name.clone()),
            name,
            format: m["format"].as_str().unwrap_or("dst").to_string(),
            max_stitch: num("maxStitch", base.max_stitch),
            max_jump: num("maxJump", base.max_jump),
            speed: num("speed", base.speed),
            trim_seconds: num("trimSeconds", base.trim_seconds),
            standard: m["standard"].as_bool().unwrap_or(false),
            open: true,
            error: None,
        }
    }
}

/// The formats a profile can be made for, from `embroidery.machine.list`'s standard profiles.
fn formats(list: &[Value]) -> Vec<String> {
    list.iter().filter(|m| m["standard"] == true).filter_map(|m| m["format"].as_str().map(str::to_string)).collect()
}

/// The whole editor: the profiles and the form.
pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let list = app.session.execute("embroidery.machine.list", &json!({})).map(|r| r["machines"].as_array().cloned().unwrap_or_default()).unwrap_or_default();
    for m in &list {
        let Some(name) = m["name"].as_str() else { continue };
        let standard = m["standard"] == true;
        // the standard ones as they ship need no row: only the ones changed, and one's own
        if standard && m["edited"] != true {
            continue;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} ({})", m["label"].as_str().unwrap_or(name), m["format"].as_str().unwrap_or_default().to_uppercase()));
            if ui.small_button(tl!("Edit")).clicked() {
                app.embroidery.machine_form = MachineDraft::edit(m);
            }
            let label = if standard { tl!("Reset") } else { tl!("Delete") };
            if ui.small_button(label).clicked()
                && let Err(e) = app.run("embroidery.machine.delete", json!({ "name": name }))
            {
                app.status(e);
            }
        });
    }
    if ui.button(tl!("New Machine")).clicked() {
        app.embroidery.machine_form = MachineDraft { open: true, ..Default::default() };
    }
    form(app, ui, &list);
}

fn form(app: &mut VectorcraftApp, ui: &mut Ui, list: &[Value]) {
    if !app.embroidery.machine_form.open {
        return;
    }
    let mut d = std::mem::take(&mut app.embroidery.machine_form);
    let (mut save, mut cancel) = (false, false);
    ui.separator();
    ui.add_enabled(!d.standard, egui::TextEdit::singleline(&mut d.name).desired_width(180.0).hint_text(tl!("Machine name")));
    if !d.standard {
        ui.horizontal(|ui| {
            ui.label(tl!("File format"));
            craft_ui::ComboBox::from_id_salt("machine-format").selected_text(d.format.to_uppercase()).width(90.0).show_ui(ui, |ui| {
                for f in formats(list) {
                    let label = f.to_uppercase();
                    ui.selectable_value(&mut d.format, f, label);
                }
            });
        });
    }
    for (label, v, lo, hi, unit) in [
        (tl!("Longest stitch"), &mut d.max_stitch, 1.0, 50.0, " mm"),
        (tl!("Longest jump"), &mut d.max_jump, 1.0, 50.0, " mm"),
        (tl!("Speed"), &mut d.speed, 50.0, 3000.0, " /min"),
        (tl!("Trim time"), &mut d.trim_seconds, 0.0, 60.0, " s"),
    ] {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(v).range(lo..=hi).speed(0.5).suffix(unit));
        });
    }
    if let Some(e) = &d.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    ui.horizontal(|ui| {
        if !d.name.trim().is_empty() && ui.button(tl!("Save Machine")).clicked() {
            save = true;
        }
        if ui.button(tl!("Cancel")).clicked() {
            cancel = true;
        }
    });
    if save {
        match app.run("embroidery.machine.save", d.params()) {
            Ok(_) => (d.open, d.error, d.was) = (false, None, None),
            Err(e) => d.error = Some(e),
        }
    }
    if cancel {
        (d.open, d.error, d.was) = (false, None, None);
    }
    app.embroidery.machine_form = d;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_asks_for_what_the_command_takes() {
        let new = MachineDraft { name: " Shop ".into(), format: "pes".into(), speed: 700.0, ..Default::default() };
        let p = new.params();
        assert_eq!((p["name"].as_str(), p["format"].as_str(), p["speed"].as_f64()), (Some("Shop"), Some("pes"), Some(700.0)));
        assert!(p.get("newName").is_none());
        // a rename keeps the old name as the key
        let renamed = MachineDraft { name: "Shop 2".into(), was: Some("Shop".into()), ..new };
        let p = renamed.params();
        assert_eq!((p["name"].as_str(), p["newName"].as_str()), (Some("Shop"), Some("Shop 2")));
        // a standard profile keeps its name and format
        let std = MachineDraft::edit(&json!({"name": "dst", "format": "dst", "standard": true}));
        assert!(std.params().get("format").is_none() && std.params().get("newName").is_none());
    }

    #[test]
    fn a_machine_is_made_edited_and_deleted_from_the_form() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
        let d = MachineDraft { name: "Shop".into(), format: "pes".into(), speed: 650.0, ..Default::default() };
        app.run("embroidery.machine.save", d.params()).unwrap();
        let shop = |app: &mut VectorcraftApp| {
            let l = app.run("embroidery.machine.list", json!({})).unwrap();
            l["machines"].as_array().unwrap().iter().find(|m| m["name"] == "Shop" || m["name"] == "Shop 2").cloned()
        };
        let mut e = MachineDraft::edit(&shop(&mut app).unwrap());
        assert_eq!((e.format.as_str(), e.speed), ("pes", 650.0));
        e.name = "Shop 2".into();
        e.speed = 900.0;
        app.embroidery.machine_form = e;
        let ctx = egui::Context::default();
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 800.0))), ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| show(&mut app, ui));
        out.textures_delta.clear();
        let e = app.embroidery.machine_form.clone();
        app.run("embroidery.machine.save", e.params()).unwrap();
        assert_eq!(shop(&mut app).unwrap()["speed"], 900.0);
        assert_eq!(shop(&mut app).unwrap()["name"], "Shop 2");
        app.run("embroidery.machine.delete", json!({"name": "Shop 2"})).unwrap();
        assert!(shop(&mut app).is_none());
    }
}
