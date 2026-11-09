//! Start a Design: begin in the hoop, not on a page. Three plain steps (your machine, the hoop,
//! the fabric) then Create. The machine is asked once (it is remembered as My Machine) and the
//! step is skipped afterwards; the hoops shown are the ones that machine has; the fabric can be
//! skipped. Create runs `embroidery.newDesign` (and `embroidery.applyFabric`); a blank page of
//! any size stays one click away (File › New…).

use egui::{RichText, Ui, vec2};
use serde_json::{Value, json};

use crate::VectorcraftApp;
use crate::theme::{self, Tokens};

/// The machine chosen for "I don't know / just exploring": a general profile (plain DST output).
pub const GENERIC_MACHINE: &str = "Another machine (DST)";

/// The steps, in order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step {
    #[default]
    Machine,
    Hoop,
    Fabric,
}

/// What the window has been told so far.
#[derive(Clone, Debug, Default)]
pub struct StartDesignState {
    pub open: bool,
    pub step: Step,
    /// The hoop chosen (kept after a design is made: the next one starts on the same hoop).
    pub hoop: Option<String>,
    pub error: Option<String>,
}

fn mine(app: &mut VectorcraftApp) -> Value {
    app.session.execute("embroidery.myMachine", &json!({})).unwrap_or_default()
}

/// The step to open on: the hoop when a machine is remembered, else the machine.
pub fn first_step(mine: &Value) -> Step {
    if mine["machine"].as_str().is_some() { Step::Hoop } else { Step::Machine }
}

/// `ui.startDesign`: opens the window.
pub fn open(app: &mut VectorcraftApp) -> Result<Value, String> {
    let step = first_step(&mine(app));
    let st = &mut app.ui.start_design;
    (st.open, st.step, st.error) = (true, step, None);
    Ok(json!({ "open": true, "step": step_id(step) }))
}

fn step_id(s: Step) -> &'static str {
    match s {
        Step::Machine => "machine",
        Step::Hoop => "hoop",
        Step::Fabric => "fabric",
    }
}

/// `ui.startDesign.create {hoop, fabric?}`: the design in the hoop, with the fabric when one is
/// named; the window closes.
pub fn create(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let hoop = p.get("hoop").and_then(Value::as_str).ok_or("choose a hoop first")?.to_string();
    let r = app.run("embroidery.newDesign", json!({ "hoop": hoop }))?;
    if let Some(name) = p.get("fabric").and_then(Value::as_str).filter(|n| !n.is_empty()) {
        app.run("embroidery.applyFabric", json!({ "name": name }))?;
    }
    let st = &mut app.ui.start_design;
    st.open = false;
    st.hoop = Some(hoop);
    st.error = None;
    Ok(r)
}

/// The window, while open.
pub(crate) fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.ui.start_design.open {
        return;
    }
    let mut open = true;
    let mut done = false;
    egui::Window::new(tl!("Start a design"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(300.0, 260.0))
        .show(ctx, |ui| {
            let t = Tokens::get(ui.ctx());
            ui.set_width(600.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            let body = (ui.ctx().content_rect().height() - 170.0).max(160.0);
            egui::ScrollArea::vertical().max_height(body).auto_shrink([false, true]).show(ui, |ui| match app.ui.start_design.step {
                Step::Machine => machine_step(app, ui, &t),
                Step::Hoop => hoop_step(app, ui, &t),
                Step::Fabric => done = fabric_step(app, ui, &t),
            });
            if let Some(e) = &app.ui.start_design.error {
                ui.label(RichText::new(e.as_str()).size(12.0).color(t.error));
            }
        });
    if !open || done || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ui.start_design.open = false;
    }
}

/// Step 1: which machine (asked once; "just exploring" takes a general profile).
fn machine_step(app: &mut VectorcraftApp, ui: &mut Ui, t: &Tokens) {
    ui.label(RichText::new(tl!("Step 1 of 3: your machine")).font(theme::semibold(13.0)).color(t.accent));
    ui.label(RichText::new(tl!("Which embroidery machine do you have?")).font(theme::semibold(18.0)).color(t.text));
    ui.label(RichText::new(tl!("We ask once and remember it. It sets the hoops you can choose and the file your machine reads.")).size(13.0).color(t.text_dim));
    let m = mine(app);
    let mut chosen = None;
    for machine in m["machines"].as_array().map(Vec::as_slice).unwrap_or_default() {
        let Some(name) = machine["name"].as_str() else { continue };
        if ui.selectable_label(false, RichText::new(crate::dialogs::my_machine::machine_name(name)).size(15.0)).clicked() {
            chosen = Some(name.to_string());
        }
    }
    ui.add_space(4.0);
    if ui.button(tl!("I don't know / just exploring")).clicked() {
        chosen = Some(GENERIC_MACHINE.to_string());
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new(tl!("Measure in")).size(14.0));
        let units = app.session.prefs.units_general.clone();
        for (id, label) in [("millimeters", tl!("Millimeters")), ("inches", tl!("Inches"))] {
            if ui.selectable_label(units == id, RichText::new(label).size(14.0)).clicked() {
                app.session.execute("prefs.set", &json!({ "key": "unitsGeneral", "value": id })).ok();
            }
        }
    });
    ui.label(RichText::new(tl!("Where designs are saved (a folder or your USB stick) can be set later in My Machine.")).size(12.0).color(t.text_dim));
    if let Some(name) = chosen {
        match app.run("embroidery.myMachine.set", json!({ "machine": name })) {
            Ok(_) => (app.ui.start_design.step, app.ui.start_design.error) = (Step::Hoop, None),
            Err(e) => app.ui.start_design.error = Some(e),
        }
    }
}

/// Step 2: the hoops that machine has, each drawn to scale.
fn hoop_step(app: &mut VectorcraftApp, ui: &mut Ui, t: &Tokens) {
    let m = mine(app);
    ui.label(RichText::new(tl!("Step 2 of 3: your hoop")).font(theme::semibold(13.0)).color(t.accent));
    ui.label(RichText::new(tl!("Which hoop are you sewing in?")).font(theme::semibold(18.0)).color(t.text));
    ui.horizontal_wrapped(|ui| {
        if let Some(name) = m["machine"].as_str() {
            ui.label(RichText::new(format!("{} {}", tl!("My machine:"), crate::dialogs::my_machine::machine_name(name))).color(t.text_dim));
            if ui.link(tl!("Change machine")).clicked() {
                app.ui.start_design.step = Step::Machine;
            }
        }
    });
    let hoops = crate::canvas::home::home_hoops(&m);
    // the last hoop used, when this machine has it
    let current = app.ui.start_design.hoop.clone().filter(|h| hoops.iter().any(|(_, name, ..)| name == h));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
        for (label, hoop, w, h) in &hoops {
            let r = crate::canvas::home::hoop_card(ui, t, label, (*w, *h));
            if current.as_deref() == Some(hoop.as_str()) {
                ui.painter().rect_stroke(r.rect, 6.0, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
            }
            if r.clicked() {
                app.ui.start_design.hoop = Some(hoop.clone());
                app.ui.start_design.step = Step::Fabric;
            }
        }
    });
}

/// Step 3: the fabric, optional. Returns true when the design was made.
fn fabric_step(app: &mut VectorcraftApp, ui: &mut Ui, t: &Tokens) -> bool {
    ui.label(RichText::new(tl!("Step 3 of 3: your fabric (you can skip this)")).font(theme::semibold(13.0)).color(t.accent));
    let Some(hoop) = app.ui.start_design.hoop.clone() else {
        app.ui.start_design.step = Step::Hoop;
        return false;
    };
    if ui.link(tl!("Back to hoops")).clicked() {
        app.ui.start_design.step = Step::Hoop;
    }
    let Some(choice) = crate::guide::fabric_choice(app, ui, t, None) else { return false };
    match app.run("ui.startDesign.create", json!({ "hoop": hoop, "fabric": choice })) {
        Ok(_) => true,
        Err(e) => {
            app.ui.start_design.error = Some(e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_engine::Session;

    fn app() -> VectorcraftApp {
        VectorcraftApp::new(Session::new(), Default::default())
    }

    fn frame(app: &mut VectorcraftApp) {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, Default::default());
        let _ = ctx.run_ui(Default::default(), |ui| show(app, ui.ctx()));
    }

    #[test]
    fn the_first_run_asks_for_the_machine_and_then_it_is_skipped() {
        let mut app = app();
        let r = app.run("ui.startDesign", json!({})).unwrap();
        assert_eq!(r["step"], "machine");
        frame(&mut app);
        app.run("embroidery.myMachine.set", json!({ "machine": "Brother or Baby Lock" })).unwrap();
        let r = app.run("ui.startDesign", json!({})).unwrap();
        assert_eq!(r["step"], "hoop", "a remembered machine skips step 1");
        assert_eq!(app.ui.start_design.step, Step::Hoop);
        frame(&mut app);
    }

    #[test]
    fn just_exploring_takes_a_general_profile() {
        let mut app = app();
        app.run("embroidery.myMachine.set", json!({ "machine": GENERIC_MACHINE })).unwrap();
        let m = mine(&mut app);
        assert_eq!(m["machine"], GENERIC_MACHINE);
        assert!(!m["hoops"].as_array().unwrap().is_empty());
    }

    #[test]
    fn the_hoop_list_is_the_machines_hoops() {
        let mut app = app();
        app.run("embroidery.myMachine.set", json!({ "machine": "Husqvarna Viking (other models)" })).unwrap();
        let m = mine(&mut app);
        let listed: Vec<String> = crate::canvas::home::home_hoops(&m).into_iter().map(|h| h.1).collect();
        let machine_hoops: Vec<&str> = m["hoops"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap()).collect();
        assert_eq!(listed, machine_hoops);
        assert!(listed.iter().all(|h| h.starts_with("Husqvarna")), "{listed:?}");
    }

    #[test]
    fn create_makes_a_design_in_the_hoop_with_the_fabric() {
        let mut app = app();
        app.run("ui.startDesign", json!({})).unwrap();
        let r = app.run("ui.startDesign.create", json!({ "hoop": "Brother 100 × 100 (4×4)", "fabric": "Woven (twill, poplin)" })).unwrap();
        assert_eq!((r["width"].as_f64(), r["height"].as_f64()), (Some(100.0), Some(100.0)));
        assert!(!app.ui.start_design.open);
        assert_eq!(app.ui.start_design.hoop.as_deref(), Some("Brother 100 × 100 (4×4)"));
        let info = app.session.execute("embroidery.info", &json!({})).unwrap();
        assert_eq!(info["fabric"], "Woven (twill, poplin)");
        assert_eq!(info["hoop"], "Brother 100 × 100 (4×4)");
        // the fabric is optional, and an unknown hoop is an error, not a crash
        app.run("ui.startDesign.create", json!({ "hoop": "Brother 130 × 180 (5×7)" })).unwrap();
        assert!(app.run("ui.startDesign.create", json!({ "hoop": "No such hoop" })).is_err());
        assert!(app.run("ui.startDesign.create", json!({})).is_err());
    }

    #[test]
    fn blank_canvas_and_new_presets_still_work() {
        let mut app = app();
        app.run("file.newDialog", json!({})).unwrap();
        assert_eq!(app.ui.dialog.as_ref().map(|d| d.kind.as_str()), Some("newDocument"));
        app.run("file.newWithoutDialog", json!({})).unwrap();
        assert_eq!(app.session.documents().len(), 1);
    }
}
