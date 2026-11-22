//! "Get updates" signup and privacy-friendly funnel events (issue 109, add-on 29).
//!
//! No network code lives here: the app sends the [`HttpRequest`]s built below with its own client.
//!
//! * [`Signup`] becomes the hub's `POST /api/subscribe`. It needs the user's explicit consent
//!   (never pre-ticked) and a plausible email address.
//! * [`Analytics`] builds PostHog capture requests. Only the [`ALLOWED_PROPS`] ever leave the app
//!   ([`sanitize`] drops everything else, so file names, design content and typed text can't),
//!   and nothing is built when the project key is unset, the visitor sends Do Not Track or Global
//!   Privacy Control, or (desktop) the user has not opted in. Anonymous events carry no person
//!   profile, no cookie and no stored id.

use serde_json::{Map, Value, json};

use crate::ai::{HttpRequest, Method, cloud};

/// The hub's address unless another is set (the same hub as Send to StitchShop).
pub use crate::ai::stitchshop::DEFAULT_HUB;
/// PostHog's US cloud, used unless `POSTHOG_HOST` says otherwise.
pub const DEFAULT_POSTHOG_HOST: &str = "https://us.i.posthog.com";
/// The only event properties that may be sent.
pub const ALLOWED_PROPS: [&str; 12] = [
    "app",
    "app_version",
    "platform",
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_content",
    "utm_term",
    "step",
    "format",
    "error_code",
    "duration_ms",
];
/// `ok` is allowed too (a boolean); kept apart so the list above reads as the plan's.
pub const OK_PROP: &str = "ok";
/// The longest string property sent (characters).
const MAX_PROP: usize = 64;
/// The longest email, name and source the form sends (characters).
const MAX_EMAIL: usize = 254;
const MAX_NAME: usize = 100;

/// Why a request could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdatesError {
    #[error("The consent box is not ticked.")]
    NoConsent,
    #[error("That doesn't look like an email address.")]
    BadEmail,
    #[error("The server address is not valid.")]
    BadServer,
}

/// What the "Get updates" form collected.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Signup {
    pub email: String,
    /// Optional.
    pub name: String,
    /// Products of interest (`stitchfoundry` unless the user changed it).
    pub interests: Vec<String>,
    /// Where the form was shown (`app`, `web`, a page name).
    pub source: String,
    /// `utm_*` values of the visit.
    pub utm: Vec<(String, String)>,
    /// The consent box, which is never pre-ticked.
    pub consent: bool,
}

fn clip(s: &str, max: usize) -> String {
    s.trim().chars().filter(|c| !c.is_control()).take(max).collect()
}

/// A plausible email: one `@`, something either side, a dot in the domain, no spaces.
pub fn valid_email(email: &str) -> bool {
    let e = email.trim();
    if e.is_empty() || e.chars().count() > MAX_EMAIL || e.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let mut parts = e.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with(['.', '-']) && !domain.ends_with(['.', '-']) && !domain.contains("..")
}

/// `POST /api/subscribe` on `hub`.
pub fn subscribe(hub: &str, signup: &Signup) -> Result<HttpRequest, UpdatesError> {
    if !signup.consent {
        return Err(UpdatesError::NoConsent);
    }
    if !valid_email(&signup.email) {
        return Err(UpdatesError::BadEmail);
    }
    let hub = cloud::clean_server(hub).ok_or(UpdatesError::BadServer)?;
    let mut interests: Vec<String> = signup.interests.iter().map(|i| clip(i, MAX_NAME)).filter(|i| !i.is_empty()).collect();
    if interests.is_empty() {
        interests.push("stitchfoundry".into());
    }
    let utm: Map<String, Value> = signup
        .utm
        .iter()
        .filter(|(k, _)| k.starts_with("utm_") && ALLOWED_PROPS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), Value::String(clip(v, MAX_PROP))))
        .collect();
    let body = json!({
        "email": signup.email.trim(),
        "name": clip(&signup.name, MAX_NAME),
        "interests": interests,
        "source": clip(&signup.source, MAX_PROP),
        "utm": utm,
        "consent": true,
    });
    Ok(HttpRequest {
        method: Method::Post,
        url: format!("{hub}/api/subscribe"),
        headers: vec![("content-type", "application/json".into())],
        body: body.to_string(),
        ..Default::default()
    })
}

/// A funnel event (names shared across the family).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    LandingViewed,
    ProjectStarted,
    FileImported,
    ImportFailed,
    DesignEdited,
    PreviewViewed,
    ExportCreated,
    ExportFailed,
    ErrorShown,
    StepViewed,
    StepCompleted,
    AiAddonViewed,
}

impl Event {
    pub fn name(self) -> &'static str {
        match self {
            Self::LandingViewed => "landing_viewed",
            Self::ProjectStarted => "project_started",
            Self::FileImported => "file_imported",
            Self::ImportFailed => "import_failed",
            Self::DesignEdited => "design_edited",
            Self::PreviewViewed => "preview_viewed",
            Self::ExportCreated => "export_created",
            Self::ExportFailed => "export_failed",
            Self::ErrorShown => "error_shown",
            Self::StepViewed => "step_viewed",
            Self::StepCompleted => "step_completed",
            Self::AiAddonViewed => "ai_addon_viewed",
        }
    }
}

/// Keeps only the allowed properties, with scalar values (strings cut to 64 characters).
pub fn sanitize(props: &Map<String, Value>) -> Map<String, Value> {
    props
        .iter()
        .filter(|(k, _)| ALLOWED_PROPS.contains(&k.as_str()) || k.as_str() == OK_PROP)
        .filter_map(|(k, v)| {
            let v = match v {
                Value::String(s) => Value::String(clip(s, MAX_PROP)),
                Value::Number(n) if n.as_f64().is_some_and(f64::is_finite) => v.clone(),
                Value::Bool(_) => v.clone(),
                _ => return None,
            };
            Some((k.clone(), v))
        })
        .collect()
}

/// What the visitor or user has said about being counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Privacy {
    /// The browser sends Do Not Track.
    pub dnt: bool,
    /// The browser sends Global Privacy Control.
    pub gpc: bool,
    /// The desktop app's setting (first-run prompt; default off). Ignored on the web.
    pub opted_in: bool,
    /// A desktop build: counting needs `opted_in`.
    pub desktop: bool,
}

impl Privacy {
    /// Whether an event may be sent at all.
    pub fn allows(self) -> bool {
        !self.dnt && !self.gpc && (!self.desktop || self.opted_in)
    }
}

/// The PostHog project, when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct Analytics {
    key: String,
    host: String,
}

impl Analytics {
    /// From the build's `POSTHOG_KEY` and `POSTHOG_HOST`. No key (unset or blank), or a host that
    /// isn't a plain address: none, which turns analytics fully off.
    pub fn from_config(key: Option<&str>, host: Option<&str>) -> Option<Self> {
        let key = key?.trim();
        if key.is_empty() || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return None;
        }
        let host = match host.map(str::trim).filter(|h| !h.is_empty()) {
            Some(h) => h,
            None => DEFAULT_POSTHOG_HOST,
        };
        let host = cloud::clean_server(host)?;
        Some(Self { key: key.to_string(), host })
    }

    /// The capture request for `event`, or none when `privacy` doesn't allow it. `session` is a
    /// random id for this run, held in memory only.
    pub fn capture(&self, event: Event, props: &Map<String, Value>, session: &str, privacy: Privacy) -> Option<HttpRequest> {
        if !privacy.allows() {
            return None;
        }
        let mut properties = sanitize(props);
        // no person profile for anonymous visitors
        properties.insert("$process_person_profile".into(), Value::Bool(false));
        let body = json!({
            "api_key": self.key,
            "event": event.name(),
            "distinct_id": clip(session, MAX_PROP),
            "properties": properties,
        });
        Some(HttpRequest {
            method: Method::Post,
            url: format!("{}/capture/", self.host),
            headers: vec![("content-type", "application/json".into())],
            body: body.to_string(),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn signup() -> Signup {
        Signup { email: "a@b.example".into(), consent: true, source: "app".into(), ..Default::default() }
    }

    #[test]
    fn subscribe_needs_consent_and_email() {
        let mut s = signup();
        s.consent = false;
        assert_eq!(subscribe(DEFAULT_HUB, &s), Err(UpdatesError::NoConsent));
        for bad in ["", "a", "a@b", "@b.c", "a@@b.c", "a b@c.d", "a@.c.d", "a@b..c", "a@b.c-"] {
            s.consent = true;
            s.email = bad.into();
            assert_eq!(subscribe(DEFAULT_HUB, &s), Err(UpdatesError::BadEmail), "{bad}");
        }
        assert_eq!(subscribe("javascript:x", &signup()), Err(UpdatesError::BadServer));
    }

    #[test]
    fn subscribe_body() {
        let mut s = signup();
        s.utm = vec![("utm_source".into(), "fb".into()), ("secret".into(), "x".into())];
        s.name = "  Ann\n".into();
        let r = subscribe(&format!("{DEFAULT_HUB}/"), &s).unwrap();
        assert_eq!(r.url, format!("{DEFAULT_HUB}/api/subscribe"));
        let b: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(b["interests"], json!(["stitchfoundry"]));
        assert_eq!(b["name"], "Ann");
        assert_eq!(b["utm"], json!({"utm_source": "fb"}));
        assert_eq!(b["consent"], true);
    }

    #[test]
    fn sanitize_keeps_only_the_allow_list() {
        let props = json!({
            "app": "stitchfoundry", "step": "export", "ok": true, "duration_ms": 12,
            "file_name": "secret.dst", "text": "typed", "design": {"a": 1}, "format": ["dst"],
            "error_code": "x".repeat(200),
        });
        let out = sanitize(props.as_object().unwrap());
        let mut keys: Vec<&str> = out.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["app", "duration_ms", "error_code", "ok", "step"]);
        assert_eq!(out["error_code"].as_str().unwrap().chars().count(), MAX_PROP);
        assert!(keys.iter().all(|k| ALLOWED_PROPS.contains(k) || *k == OK_PROP));
    }

    #[test]
    fn analytics_is_off_without_a_key() {
        assert_eq!(Analytics::from_config(None, None), None);
        assert_eq!(Analytics::from_config(Some("  "), None), None);
        assert_eq!(Analytics::from_config(Some("phc_x"), Some("ftp://x")), None);
        assert!(Analytics::from_config(Some("phc_x"), Some("")).is_some());
    }

    #[test]
    fn capture_respects_privacy_and_strips_content() {
        let a = Analytics::from_config(Some("phc_x"), None).unwrap();
        let props = json!({"step": "1", "file_name": "mum.dst"});
        let props = props.as_object().unwrap();
        let web = Privacy::default();
        let r = a.capture(Event::ExportCreated, props, "s1", web).unwrap();
        assert_eq!(r.url, "https://us.i.posthog.com/capture/");
        let b: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(b["event"], "export_created");
        assert_eq!(b["properties"]["step"], "1");
        assert!(b["properties"].get("file_name").is_none());
        assert_eq!(b["properties"]["$process_person_profile"], false);
        assert!(!r.body.contains("mum.dst"));
        for p in [Privacy { dnt: true, ..web }, Privacy { gpc: true, ..web }, Privacy { desktop: true, ..web }] {
            assert!(a.capture(Event::StepViewed, props, "s1", p).is_none());
        }
        let desk = Privacy { desktop: true, opted_in: true, ..web };
        assert!(a.capture(Event::StepViewed, props, "s1", desk).is_some());
        assert!(!Privacy { desktop: true, opted_in: true, dnt: true, gpc: false }.allows());
    }

    #[test]
    fn event_names_are_the_shared_ones() {
        assert_eq!(Event::LandingViewed.name(), "landing_viewed");
        assert_eq!(Event::AiAddonViewed.name(), "ai_addon_viewed");
        assert_eq!(Event::ImportFailed.name(), "import_failed");
    }
}
