//! The staging switch (web app): opening the page with `?services=staging` makes this tab use the
//! staging account service and the staging MakerFoundry, with a banner saying so, so the release
//! tests can publish and open designs without touching production. The two addresses are compiled
//! in (exact, https only); nothing from the page's address is ever used as an address, and any other
//! value of `services` changes nothing. Production stays the default.

use crate::VectorcraftApp;

/// The staging account service and MakerFoundry (exact; https).
pub const CLOUD: &str = "https://cloud-staging-staging.up.railway.app";
pub const MAKERFOUNDRY: &str = "https://makerfoundry-test-test.up.railway.app";
/// The staging MakerFoundry's host, accepted by `?open=` only while in staging mode.
pub const MAKERFOUNDRY_HOST: &str = "makerfoundry-test-test.up.railway.app";
/// The banner's text.
pub const BANNER: &str = "STAGING services (test data)";

/// Whether the page's query string (`?a=b&services=staging`) asks for staging: the `services`
/// parameter, with exactly the value `staging` (a repeated parameter must agree).
pub fn requested(query: &str) -> bool {
    let mut values = query.trim_start_matches('?').split('&').filter_map(|kv| kv.strip_prefix("services=")).peekable();
    values.peek().is_some() && values.all(|v| v == "staging")
}

/// The banner, while in staging mode.
pub(crate) fn banner(app: &VectorcraftApp, ctx: &egui::Context) {
    if !app.staging {
        return;
    }
    egui::Area::new(egui::Id::new("staging-banner")).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 2.0)).interactable(false).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_rgb(0xb0, 0x20, 0x20)).corner_radius(4.0).inner_margin(egui::Margin::symmetric(10, 2)).show(ui, |ui| {
            ui.label(egui::RichText::new(BANNER).strong().size(12.0).color(egui::Color32::WHITE));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_engine::Session;

    #[test]
    fn only_the_exact_value_asks_for_staging() {
        assert!(requested("?services=staging") && requested("?lang=es&services=staging&view") && requested("services=staging"));
        for no in ["", "?", "?services=", "?services=production", "?services=Staging", "?services=staging2", "?services=https://evil.example", "?xservices=staging", "?services=staging&services=prod", "?services%3Dstaging"] {
            assert!(!requested(no), "{no}");
        }
    }

    #[test]
    fn staging_switches_both_addresses_and_production_is_the_default() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        assert!(!app.staging);
        assert_ne!(crate::embroidery::account::server(&app).as_deref(), Some(CLOUD));
        app.staging = true;
        assert_eq!(crate::embroidery::account::server(&app).as_deref(), Some(CLOUD));
        // a typed server (the settings rows) cannot move a staging tab elsewhere
        app.embroidery.account.server = "https://evil.example".into();
        app.embroidery.publish.server = "https://evil.example".into();
        assert_eq!(crate::embroidery::account::server(&app).as_deref(), Some(CLOUD));
        assert_eq!(crate::embroidery::publish::server(&app).unwrap(), MAKERFOUNDRY);
        app.staging = false;
        assert_eq!(crate::embroidery::publish::server(&app).unwrap(), "https://evil.example");
        assert!(CLOUD.starts_with("https://") && MAKERFOUNDRY.starts_with("https://") && MAKERFOUNDRY.ends_with(MAKERFOUNDRY_HOST));
    }

    #[test]
    fn the_banner_shows_only_in_staging() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        let draws = |app: &VectorcraftApp| {
            let ctx = egui::Context::default();
            crate::theme::install_fonts(&ctx);
            let mut out = ctx.run_ui(Default::default(), |ui| banner(app, ui.ctx()));
            out.textures_delta.clear();
            out.shapes.iter().any(|s| format!("{s:?}").contains("STAGING services"))
        };
        assert!(!draws(&app));
        app.staging = true;
        assert!(draws(&app));
    }
}
