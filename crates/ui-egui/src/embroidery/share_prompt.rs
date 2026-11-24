//! "Share it on MakerFoundry?" (add-on 37): after the first export to a machine embroidery file
//! in a project, a small prompt with Share free / Sell it / Not now. It never blocks or delays the
//! export (it is raised after the file is written), is shown at most once per project, and can be
//! turned off for good (its checkbox, or Settings › MakerFoundry › Offer to share after export).
//!
//! Share free and Sell it both go to the existing Publish to MakerFoundry path: Foundry Account
//! sign-in only happens then, the draft opens in the browser, and the price, licence and Seller
//! Terms are set there, never here. "I made this / I own the rights" must be ticked first.

use std::collections::HashSet;

use egui::RichText;
use serde_json::json;

use super::{account, publish};
use crate::VectorcraftApp;
use crate::theme::Tokens;

/// What the maker picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Free,
    Sell,
    NotNow,
}

/// The prompt between frames.
#[derive(Default)]
pub struct ShareState {
    pub open: bool,
    /// Projects already asked this session (their path, else title).
    asked: HashSet<String>,
    /// "I made this / I own the rights".
    pub own_rights: bool,
    /// "Don't ask again".
    pub dont_ask: bool,
    /// The last choice made, for the control channel and tests.
    pub last_choice: Option<Choice>,
}

/// The project's key: where it is saved, else its title.
fn project_key(app: &VectorcraftApp) -> Option<String> {
    let d = app.session.active()?;
    Some(d.path.clone().unwrap_or_else(|| d.doc.title.clone()))
}

/// Called after an export to a machine file was written: raises the prompt when it is due.
/// → whether it was raised.
pub fn after_export(app: &mut VectorcraftApp) -> bool {
    if !app.session.prefs.offer_share_after_export {
        return false;
    }
    let Some(key) = project_key(app) else { return false };
    let st = &mut app.embroidery.share;
    if st.open || !st.asked.insert(key) {
        return false;
    }
    st.own_rights = false;
    st.dont_ask = false;
    st.open = true;
    true
}

/// Applies a choice: closes the prompt, keeps "don't ask again", and for Share/Sell starts
/// publishing (signed in) or opens the Foundry Account sign-in (not). → an error to show.
pub fn choose(app: &mut VectorcraftApp, choice: Choice) -> Result<(), String> {
    let st = &mut app.embroidery.share;
    if matches!(choice, Choice::Free | Choice::Sell) && !st.own_rights {
        return Err(crate::i18n::msg("Tick “I made this / I own the rights” first.").to_string());
    }
    st.open = false;
    st.last_choice = Some(choice);
    let dont_ask = st.dont_ask;
    if dont_ask {
        // (a bad write of the setting only means the prompt may come back for another project)
        let _ = app.session.execute("prefs.set", &json!({ "key": "offerShareAfterExport", "value": false }));
    }
    if choice == Choice::NotNow {
        return Ok(());
    }
    let signed = !app.embroidery.account.token.is_empty();
    publish::open(app, &json!({ "send": signed }))?;
    if !signed {
        let _ = account::open(app, &json!({ "create": false })); // (only fails without a connection)
    }
    Ok(())
}

/// The prompt, while open: a small window that does not block the canvas.
pub fn window(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.embroidery.share.open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    let mut picked = None;
    let mut error = None;
    egui::Window::new(tl!("Share it on MakerFoundry?"))
        .id(egui::Id::unique("share-makerfoundry"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -40.0))
        .show(ctx, |ui| {
            ui.set_width(340.0);
            let st = &mut app.embroidery.share;
            ui.label(
                RichText::new(tl!(
                    "Your file is saved. Put this design on MakerFoundry: the draft opens in your browser, where you set the price and licence."
                ))
                .size(12.0)
                .color(t.text_dim),
            );
            ui.checkbox(&mut st.own_rights, tl!("I made this / I own the rights"));
            ui.checkbox(&mut st.dont_ask, tl!("Don't ask again"));
            ui.horizontal(|ui| {
                if ui.button(tl!("Share free")).clicked() {
                    picked = Some(Choice::Free);
                }
                if ui.button(tl!("Sell it")).clicked() {
                    picked = Some(Choice::Sell);
                }
                if ui.button(tl!("Not now")).clicked() {
                    picked = Some(Choice::NotNow);
                }
            });
        });
    if let Some(c) = picked {
        error = choose(app, c).err();
    } else if !open {
        let _ = choose(app, Choice::NotNow); // (the window's close box)
    }
    if let Some(e) = error {
        app.status(e);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use super::*;

    fn app() -> VectorcraftApp {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
        app
    }

    #[test]
    fn asked_once_per_project() {
        let mut app = app();
        assert!(after_export(&mut app));
        choose(&mut app, Choice::NotNow).unwrap();
        assert!(!app.embroidery.share.open);
        assert!(!after_export(&mut app), "the same project is not asked twice");
        app.run("file.new", json!({"width": 100, "height": 100})).unwrap();
        // a new project (its title may match; the key is the path, else the title)
        app.session.active_mut().unwrap().path = Some("/tmp/other.vectorcraft".into());
        assert!(after_export(&mut app));
    }

    #[test]
    fn dont_ask_again_turns_the_setting_off() {
        let mut app = app();
        assert!(after_export(&mut app));
        app.embroidery.share.dont_ask = true;
        choose(&mut app, Choice::NotNow).unwrap();
        assert!(!app.session.prefs.offer_share_after_export);
        app.session.active_mut().unwrap().path = Some("/tmp/another.vectorcraft".into());
        assert!(!after_export(&mut app));
    }

    #[test]
    fn the_setting_off_never_asks() {
        let mut app = app();
        app.session.execute("prefs.set", &json!({"key": "offerShareAfterExport", "value": false})).unwrap();
        assert!(!after_export(&mut app));
        assert!(!app.embroidery.share.open);
    }

    #[test]
    fn sharing_needs_the_rights_box_and_not_now_needs_nothing() {
        let mut app = app();
        assert!(after_export(&mut app));
        assert!(choose(&mut app, Choice::Free).is_err());
        assert!(app.embroidery.share.open, "still open after the refusal");
        choose(&mut app, Choice::NotNow).unwrap();
        assert_eq!(app.embroidery.share.last_choice, Some(Choice::NotNow));
    }

    #[test]
    fn without_an_account_sharing_opens_sign_in_and_sends_nothing() {
        let mut app = app();
        assert!(after_export(&mut app));
        app.embroidery.share.own_rights = true;
        choose(&mut app, Choice::Sell).unwrap();
        assert_eq!(app.embroidery.share.last_choice, Some(Choice::Sell));
        assert!(app.embroidery.publish.sent.is_none());
        assert!(app.embroidery.publish.error.is_none());
    }
}
