//! Learn: short in-app video lessons. A **Learn** button on Home and in the Help menu opens a list
//! of lessons (title, length, thumbnail) read from one small JSON file on stitchfoundry.app, so
//! lessons can be added without a release. Each lesson links to a YouTube video and to the matching
//! page of the built-in guide, and may have a **Try it** that opens a sample design. Until the first
//! videos exist (and offline) the list shows the planned lessons as "coming soon".
//!
//! The file (`LESSONS_URL`): `{"lessons": [{"title", "minutes"?, "thumbnail"?, "video"?, "docs"?,
//! "try"?}]}`. `video` and `thumbnail` are https links on YouTube (or stitchfoundry.app), `docs` is
//! a guide topic id (`help.topics`), `try` is `sample:<id>` (`embroidery.samples`). Anything else is
//! dropped: a lesson file never makes the app open an arbitrary address or run a command.

use std::sync::mpsc::{Receiver, channel};

use serde_json::{Value, json};

use crate::VectorcraftApp;

/// Where the lessons are listed (one small file) and where the videos are (the channel).
pub const LESSONS_URL: &str = "https://stitchfoundry.app/learn/lessons.json";
pub const CHANNEL_URL: &str = "https://www.youtube.com/@MakerFoundryApp";
/// The most of the file read, and the most lessons kept.
const MAX_BYTES: u64 = 200_000;
const MAX_LESSONS: usize = 200;
const MAX_TITLE: usize = 120;

/// One lesson.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lesson {
    pub title: String,
    pub minutes: Option<u32>,
    pub thumbnail: Option<String>,
    /// The YouTube video; none yet: "coming soon".
    pub video: Option<String>,
    /// A topic of the built-in guide.
    pub docs: Option<String>,
    /// A sample design to open (`embroidery.sample {id}`).
    pub try_sample: Option<String>,
}

/// The links a lesson file may carry: https on YouTube or the app's own site.
pub fn safe_link(url: &str) -> bool {
    ["https://www.youtube.com/", "https://youtube.com/", "https://youtu.be/", "https://i.ytimg.com/", "https://stitchfoundry.app/"]
        .iter()
        .any(|p| url.starts_with(p) && !url.contains(char::is_whitespace))
        && url.len() <= 300
}

/// The lessons planned before any video exists.
pub fn seed() -> Vec<Lesson> {
    let l = |title: &str, docs: &str, sample: Option<&str>| Lesson {
        title: title.into(),
        minutes: None,
        thumbnail: None,
        video: None,
        docs: Some(docs.into()),
        try_sample: sample.map(str::to_string),
    };
    vec![
        l("Your first design in Simple mode", "quick-start", Some("heart-patch")),
        l("Lettering a name", "lettering", None),
        l("Photo to stitches", "photo-stitch", None),
        l("Appliqué", "appliqué-covers", Some("applique-star")),
        l("Saving to your machine", "machine-files", None),
    ]
}

/// Reads a lesson file; bad entries are dropped, a bad file is an error.
pub fn parse(body: &[u8]) -> Result<Vec<Lesson>, String> {
    let v: Value = serde_json::from_slice(body).map_err(|e| format!("the lesson list isn't valid: {e}"))?;
    let items = v["lessons"].as_array().ok_or("the lesson list has no `lessons`")?;
    let link = |x: &Value, k: &str| x[k].as_str().filter(|u| safe_link(u)).map(str::to_string);
    Ok(items
        .iter()
        .filter_map(|x| {
            let title: String = x["title"].as_str()?.trim().chars().take(MAX_TITLE).collect();
            if title.is_empty() {
                return None;
            }
            Some(Lesson {
                title,
                minutes: x["minutes"].as_u64().filter(|m| (1..=240).contains(m)).map(|m| m as u32),
                thumbnail: link(x, "thumbnail"),
                video: link(x, "video"),
                docs: x["docs"]
                    .as_str()
                    .filter(|d| !d.is_empty() && d.len() <= 80 && d.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
                    .map(str::to_string),
                try_sample: x["try"]
                    .as_str()
                    .and_then(|t| t.strip_prefix("sample:"))
                    .filter(|s| !s.is_empty() && s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
                    .map(str::to_string),
            })
        })
        .take(MAX_LESSONS)
        .collect())
}

/// The Learn window and its list.
#[derive(Default)]
pub struct LearnState {
    pub open: bool,
    /// The lessons from the site; none: the planned ones show.
    pub lessons: Option<Vec<Lesson>>,
    /// Why the list is the planned one (offline, bad file), for a short note.
    pub note: Option<String>,
    rx: Option<Receiver<Result<Vec<Lesson>, String>>>,
    asked: bool,
}

/// Starts reading the list in the background (desktop; the web has no `http_get`).
fn fetch(app: &mut VectorcraftApp) {
    app.learn.asked = true;
    let Some(get) = app.services.http_get.clone() else {
        app.learn.note = Some("Offline: showing the planned lessons.".into());
        return;
    };
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new().name("learn-list".into()).spawn(move || {
        let r = vectorcraft_engine::guard::catch_panic(|| match get(LESSONS_URL, MAX_BYTES) {
            Ok((200, body)) => parse(&body),
            Ok((status, _)) => Err(format!("The site answered {status}.")),
            Err(e) => Err(format!("Can't reach the site: {e}")),
        })
        .unwrap_or_else(|msg| Err(format!("internal error: {msg}")));
        let _ = tx.send(r);
    });
    if spawned.is_ok() {
        app.learn.rx = Some(rx);
    }
}

/// `ui.learn`: opens the list (and asks the site for it, once a run).
pub fn open(app: &mut VectorcraftApp) -> Result<Value, String> {
    app.learn.open = true;
    if !app.learn.asked {
        fetch(app);
    }
    Ok(json!({ "open": true }))
}

/// `ui.learn.lessons`: the lessons shown now → {lessons: [{title, minutes, video, docs, try}], comingSoon}.
pub fn current(app: &mut VectorcraftApp) -> Value {
    let lessons = shown(app);
    let soon = lessons.iter().all(|l| l.video.is_none());
    json!({
        "lessons": lessons.iter().map(|l| json!({ "title": l.title, "minutes": l.minutes, "thumbnail": l.thumbnail, "video": l.video, "docs": l.docs, "try": l.try_sample.as_ref().map(|s| format!("sample:{s}")) })).collect::<Vec<_>>(),
        "comingSoon": soon,
        "note": app.learn.note,
    })
}

fn shown(app: &VectorcraftApp) -> Vec<Lesson> {
    match &app.learn.lessons {
        Some(l) if !l.is_empty() => l.clone(),
        _ => seed(),
    }
}

/// `ui.learn.watch {index}`, `.docs`, `.try`: what a lesson's buttons do (also for tests).
pub fn act(app: &mut VectorcraftApp, what: &str, p: &Value) -> Result<Value, String> {
    let i = p["index"].as_u64().ok_or("missing `index`")? as usize;
    let lesson = shown(app).get(i).cloned().ok_or("no such lesson")?;
    match what {
        "watch" => {
            let url = lesson.video.ok_or("this lesson's video is coming soon")?;
            app.open_url(&url);
            Ok(json!({ "opened": url }))
        }
        "docs" => {
            let topic = lesson.docs.ok_or("this lesson has no guide page")?;
            app.run("help.show", json!({ "topic": topic }))
        }
        _ => {
            let id = lesson.try_sample.ok_or("this lesson has nothing to try")?;
            app.run("embroidery.sample", json!({ "id": id }))
        }
    }
}

/// Each frame: the list's answer when it comes.
pub fn poll(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let Some(rx) = &app.learn.rx else { return };
    let Ok(r) = rx.try_recv() else {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        return;
    };
    app.learn.rx = None;
    match r {
        Ok(l) => (app.learn.lessons, app.learn.note) = (Some(l), None),
        Err(e) => app.learn.note = Some(format!("{e} Showing the planned lessons.")),
    }
}

/// The window, while open.
pub(crate) fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    poll(app, ctx);
    if !app.learn.open {
        return;
    }
    let mut open = true;
    let data = current(app);
    egui::Window::new(tl!("Learn"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(260.0, 220.0))
        .show(ctx, |ui| {
            let t = crate::theme::Tokens::get(ui.ctx());
            ui.set_width(520.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            if data["comingSoon"] == true {
                ui.label(egui::RichText::new(tl!("Lessons coming soon")).font(crate::theme::semibold(16.0)).color(t.text));
                ui.label(
                    egui::RichText::new(tl!("The first short videos are on their way. Until then, each lesson opens its page in the guide."))
                        .size(13.0)
                        .color(t.text_dim),
                );
            }
            if let Some(n) = data["note"].as_str() {
                ui.label(egui::RichText::new(n).size(12.0).color(t.text_dim));
            }
            let body = (ui.ctx().content_rect().height() - 220.0).max(160.0);
            egui::ScrollArea::vertical().max_height(body).auto_shrink([false, true]).show(ui, |ui| {
                for (i, l) in data["lessons"].as_array().map(Vec::as_slice).unwrap_or_default().iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(l["title"].as_str().unwrap_or_default()).font(crate::theme::semibold(14.0)).color(t.text));
                        if let Some(m) = l["minutes"].as_u64() {
                            ui.label(egui::RichText::new(format!("{m} min")).color(t.text_dim));
                        }
                        let p = json!({ "index": i });
                        if l["video"].is_string() {
                            if crate::widgets::primary_button(ui, tl!("Watch")).clicked() {
                                act(app, "watch", &p).ok();
                            }
                        } else {
                            ui.label(egui::RichText::new(tl!("Video coming soon")).color(t.text_dim));
                        }
                        if l["docs"].is_string() && ui.link(tl!("Read the guide page")).clicked() {
                            act(app, "docs", &p).ok();
                        }
                        if l["try"].is_string() && ui.button(tl!("Try it")).clicked() {
                            if let Err(e) = act(app, "try", &p) {
                                app.status(e);
                            }
                        }
                    });
                }
            });
            ui.horizontal(|ui| {
                if ui.link(tl!("More videos on YouTube")).clicked() {
                    app.open_url(CHANNEL_URL);
                }
            });
        });
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.learn.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_engine::Session;

    fn app_with(body: Option<Vec<u8>>) -> VectorcraftApp {
        let services = crate::Services {
            http_get: body.map(|b| {
                std::sync::Arc::new(move |url: &str, _: u64| {
                    assert_eq!(url, LESSONS_URL);
                    Ok((200u16, b.clone()))
                }) as _
            }),
            ..Default::default()
        };
        VectorcraftApp::new(Session::new(), services)
    }

    fn wait(app: &mut VectorcraftApp) {
        let ctx = egui::Context::default();
        for _ in 0..200 {
            poll(app, &ctx);
            if app.learn.rx.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn the_loader_keeps_good_lessons_and_drops_unsafe_links_and_bad_entries() {
        let body = json!({ "lessons": [
            { "title": "Names", "minutes": 4, "video": "https://www.youtube.com/watch?v=abc", "thumbnail": "https://i.ytimg.com/vi/abc/mq.jpg", "docs": "lettering", "try": "sample:heart-patch" },
            { "title": "Sneaky", "video": "https://evil.example/x", "thumbnail": "javascript:alert(1)", "docs": "../etc", "try": "command:file.quit" },
            { "title": "  " }, { "video": "https://youtu.be/x" }, 7
        ]});
        let l = parse(&serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(
            (l[0].minutes, l[0].video.as_deref(), l[0].try_sample.as_deref()),
            (Some(4), Some("https://www.youtube.com/watch?v=abc"), Some("heart-patch"))
        );
        assert_eq!((l[1].video.clone(), l[1].thumbnail.clone(), l[1].docs.clone(), l[1].try_sample.clone()), (None, None, None, None));
        assert!(parse(b"nonsense").is_err() && parse(b"{}").is_err());
        assert!(parse(br#"{"lessons": []}"#).unwrap().is_empty());
        assert!(
            !safe_link("http://www.youtube.com/x") && !safe_link("https://www.youtube.com.evil.com/x") && !safe_link("https://www.youtube.com/a b")
        );
    }

    #[test]
    fn the_seed_lessons_point_at_real_guide_pages_and_samples() {
        let mut s = Session::new();
        let seeds = seed();
        assert_eq!(seeds.len(), 5);
        for l in &seeds {
            let topic = l.docs.as_deref().unwrap();
            assert!(s.execute("help.topic", &json!({ "id": topic })).is_ok(), "{}: {topic}", l.title);
            if let Some(id) = &l.try_sample {
                assert!(
                    s.execute("embroidery.samples", &json!({})).unwrap()["samples"].as_array().unwrap().iter().any(|x| x["id"] == id.as_str()),
                    "{id}"
                );
            }
        }
    }

    #[test]
    fn online_the_site_list_shows_and_offline_the_planned_lessons_show() {
        let body = serde_json::to_vec(&json!({ "lessons": [{ "title": "Names", "video": "https://youtu.be/abc" }] })).unwrap();
        let mut app = app_with(Some(body));
        app.run("ui.learn", json!({})).unwrap();
        wait(&mut app);
        let now = current(&mut app);
        assert_eq!((now["lessons"].as_array().unwrap().len(), now["comingSoon"].clone()), (1, json!(false)));
        let r = app.run("ui.learn.watch", json!({ "index": 0 })).unwrap();
        assert_eq!(r["opened"], "https://youtu.be/abc");
        assert!(app.run("ui.learn.watch", json!({ "index": 5 })).is_err());
        // an empty list and a list that can't be read both show the planned lessons, "coming soon"
        let mut app = app_with(Some(br#"{"lessons": []}"#.to_vec()));
        app.run("ui.learn", json!({})).unwrap();
        wait(&mut app);
        let now = current(&mut app);
        assert_eq!((now["lessons"].as_array().unwrap().len(), now["comingSoon"].clone()), (5, json!(true)));
        assert!(app.run("ui.learn.watch", json!({ "index": 0 })).is_err(), "no video yet");
        let mut off = app_with(None);
        off.run("ui.learn", json!({})).unwrap();
        let now = current(&mut off);
        assert_eq!(now["lessons"].as_array().unwrap().len(), 5);
        assert!(now["note"].is_string());
        let mut bad = app_with(Some(b"not json".to_vec()));
        bad.run("ui.learn", json!({})).unwrap();
        wait(&mut bad);
        assert!(current(&mut bad)["note"].as_str().unwrap().contains("Showing the planned lessons"));
        // Try it opens the sample, the guide link opens the guide
        let n = off.run("ui.learn.try", json!({ "index": 0 })).unwrap();
        assert_eq!(n["id"], "heart-patch");
        off.run("ui.learn.docs", json!({ "index": 1 })).unwrap();
    }
}
