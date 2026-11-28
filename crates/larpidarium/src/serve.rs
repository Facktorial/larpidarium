//! `larpidarium serve`: a local server that shows the game and lets the browser edit it.
//!
//! The server only listens on 127.0.0.1. Every page is generated from the game folder on each
//! request, so edits made in a text editor show up too: the viewer polls `/api/version` and
//! reloads when the files change. Write requests must carry the `X-Larpidarium` header, which a
//! foreign web page cannot send without a CORS preflight this server never answers, and a
//! `Host` of `localhost` or `127.0.0.1`, which stops DNS rebinding.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use larp_model::{Checked, Game, Severity, SourceFile};
use miette::{Result, miette};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::build::{ASSETS, open_in_browser, read_template};
use crate::check::read_game_dir;
use crate::edit;

/// Largest accepted request body.
const MAX_BODY: u64 = 1 << 20;

/// Runs the `serve` command until interrupted.
pub(crate) fn run(dir: &Path, port: u16, open: bool) -> Result<()> {
    if !dir.join("game.toml").is_file() {
        return Err(miette!("{} has no game.toml", dir.display()));
    }
    let server = tiny_http::Server::http(("127.0.0.1", port))
        .map_err(|e| miette!("cannot listen on 127.0.0.1:{port}: {e}"))?;
    let url = format!("http://localhost:{port}/");
    println!("✔ serving {} at {url}  (Ctrl+C to stop)", dir.display());
    println!(
        "  edits in the browser are written into the TOML files in {}",
        dir.display()
    );
    if open {
        open_in_browser(url.as_ref())?;
    }
    for mut request in server.incoming_requests() {
        let mut body = String::new();
        let read = request.as_reader().take(MAX_BODY).read_to_string(&mut body);
        let header = |name: &'static str| {
            request
                .headers()
                .iter()
                .find(|h| h.field.equiv(name))
                .map(|h| h.value.as_str().to_owned())
        };
        let foreign = |name: &'static str| header(name).is_some_and(|v| !is_local_url(&v));
        let cross_site = header("Sec-Fetch-Site").is_some_and(|v| v == "cross-site")
            || foreign("Origin")
            || foreign("Referer");
        let req = Request {
            cross_site,
            method: request.method().as_str().to_owned(),
            url: request.url().to_owned(),
            host: header("Host"),
            token: header("X-Larpidarium").is_some(),
            body,
        };
        let response = if read.is_err() {
            Response::text(400, "cannot read the request body")
        } else {
            handle(dir, &req)
        };
        if req.method == "POST" {
            println!("  {} {} → {}", req.method, req.url, response.status);
        }
        let content_type = tiny_http::Header::from_bytes("Content-Type", response.content_type);
        let no_cache = tiny_http::Header::from_bytes("Cache-Control", "no-store");
        let mut reply =
            tiny_http::Response::from_data(response.body).with_status_code(response.status);
        for h in [content_type, no_cache].into_iter().flatten() {
            reply.add_header(h);
        }
        if let Err(e) = request.respond(reply) {
            eprintln!("  cannot send a response: {e}");
        }
    }
    Ok(())
}

/// One HTTP request, reduced to what the handler needs.
#[derive(Debug, Default)]
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) host: Option<String>,
    pub(crate) token: bool,
    /// The browser says the request comes from another site (`Sec-Fetch-Site`, `Origin` or
    /// `Referer`).
    pub(crate) cross_site: bool,
    pub(crate) body: String,
}

/// One HTTP response.
#[derive(Debug)]
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) content_type: &'static str,
    pub(crate) body: Vec<u8>,
}

impl Response {
    fn new(status: u16, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
        }
    }

    fn text(status: u16, body: impl Into<String>) -> Self {
        Self::new(status, "text/plain; charset=utf-8", body.into())
    }

    fn json(status: u16, value: &serde_json::Value) -> Self {
        Self::new(status, "application/json", value.to_string())
    }

    fn refused(message: impl std::fmt::Display) -> Self {
        Self::json(
            400,
            &serde_json::json!({ "ok": false, "error": message.to_string() }),
        )
    }
}

/// Answers one request. Pure apart from reading and writing files in `dir`.
pub(crate) fn handle(dir: &Path, req: &Request) -> Response {
    let local = req.host.as_deref().is_some_and(|h| {
        let name = h.rsplit_once(':').map_or(h, |(name, _)| name);
        name == "localhost" || name == "127.0.0.1"
    });
    if !local {
        return Response::text(403, "only http://localhost is allowed");
    }
    // Other web pages could otherwise include /graph-data.js as a script and read the game.
    if req.cross_site {
        return Response::text(403, "requests from other sites are not allowed");
    }
    let path = req.url.split(['?', '#']).next().unwrap_or("/");
    match (req.method.as_str(), path) {
        ("GET", "/") => asset("index.html"),
        ("GET", "/graph-data.js") => graph_data(dir),
        ("GET", "/api/version") => version(dir),
        ("GET", p) if p.starts_with("/assets/") => asset(&p[1..]),
        ("GET", p) if p.starts_with("/players/") => sheet(dir, p),
        ("GET", p) if p.starts_with("/organizers/") => organizer_sheet(dir, p),
        ("GET", "/packing.html") => packing_page(dir, larp_render::Page::Packing),
        ("GET", "/handouts.html") => packing_page(dir, larp_render::Page::Handouts),
        ("POST", _) if !req.token => Response::text(403, "missing X-Larpidarium header"),
        ("POST", "/api/node") => write_node(dir, &req.body),
        ("POST", "/api/node/delete") => delete_node(dir, &req.body),
        ("POST", "/api/thing") => write_thing(dir, &req.body),
        ("POST", "/api/segment") => write_segment(dir, &req.body),
        ("POST", "/api/layout") => write_layout(dir, &req.body),
        ("POST", "/api/map") => write_map(dir, &req.body),
        ("POST", "/api/map-preset") => write_preset(dir, &req.body),
        _ => Response::text(404, "not found"),
    }
}

/// `http://localhost[:port]/…` or `http://127.0.0.1[:port]/…`.
fn is_local_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split(['/', ':']).next().unwrap_or("");
    host == "localhost" || host == "127.0.0.1"
}

fn asset(path: &str) -> Response {
    let Some((_, text)) = ASSETS.iter().find(|(p, _)| *p == path) else {
        return Response::text(404, "not found");
    };
    let content_type = match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        _ => "text/plain; charset=utf-8",
    };
    Response::new(200, content_type, *text)
}

fn read(dir: &Path) -> std::result::Result<(Vec<SourceFile>, Checked), Response> {
    let files = read_game_dir(dir).map_err(|e| Response::text(500, format!("{e:?}")))?;
    let checked = larp_model::load(&files);
    Ok((files, checked))
}

/// The game, or the draft when it has errors.
fn shown(checked: &Checked) -> Option<&Game> {
    checked.game.as_ref().or(checked.draft.as_ref())
}

fn graph_data(dir: &Path) -> Response {
    let (_, checked) = match read(dir) {
        Ok(r) => r,
        Err(response) => return response,
    };
    let script = if let Some(game) = shown(&checked) {
        let graph = larp_graph::build(game, &checked.diagnostics, checked.playthroughs.as_ref());
        match serde_json::to_string(&graph) {
            Ok(json) => format!("window.LARP_GRAPH = {json};\nwindow.LARP_EDIT = true;\n"),
            Err(e) => return Response::text(500, e.to_string()),
        }
    } else {
        let problems: Vec<String> = checked
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| format!("{}: {}", d.file, d.message))
            .collect();
        format!(
            "window.LARP_GRAPH = null;\nwindow.LARP_EDIT = true;\nwindow.LARP_PROBLEMS = {};\n",
            serde_json::Value::from(problems)
        )
    };
    Response::new(200, "text/javascript; charset=utf-8", script)
}

fn version(dir: &Path) -> Response {
    let (files, _) = match read(dir) {
        Ok(r) => r,
        Err(response) => return response,
    };
    let mut hasher = DefaultHasher::new();
    for f in &files {
        f.path.hash(&mut hasher);
        f.text.hash(&mut hasher);
    }
    if let Ok(Some(template)) = read_template(dir) {
        template.hash(&mut hasher);
    }
    Response::json(
        200,
        &serde_json::json!({ "version": format!("{:016x}", hasher.finish()) }),
    )
}

fn sheet(dir: &Path, path: &str) -> Response {
    let Some(raw) = path
        .strip_prefix("/players/")
        .and_then(|p| p.strip_suffix(".html"))
    else {
        return Response::text(404, "not found");
    };
    let Ok(id) = edit::id(raw) else {
        return Response::text(404, "not found");
    };
    let (_, checked) = match read(dir) {
        Ok(r) => r,
        Err(response) => return response,
    };
    // Sheets are for players: never render them from a draft with errors.
    let Some(raw) = checked.game.as_ref() else {
        return Response::text(
            409,
            "the game has errors; fix them (see Game health on the graph page) to see the sheet",
        );
    };
    // Sheets show texts with `[[links]]` replaced.
    let shown = larp_model::text::expand(raw);
    let game = &shown;
    let template = match read_template(dir) {
        Ok(t) => t,
        Err(e) => return Response::text(500, format!("{e:?}")),
    };
    let graph = larp_graph::build(raw, &checked.diagnostics, checked.playthroughs.as_ref());
    let destiny = larp_graph::destiny(game, &graph, &id);
    match larp_render::player_sheet_html(game, &id, &destiny, template.as_deref()) {
        Ok(html) => Response::new(200, "text/html; charset=utf-8", html),
        Err(larp_render::RenderError::UnknownPerson(_)) => Response::text(404, "no such person"),
        Err(e) => Response::text(500, e.to_string()),
    }
}

fn organizer_sheet(dir: &Path, path: &str) -> Response {
    let Some(id) = path
        .strip_prefix("/organizers/")
        .and_then(|p| p.strip_suffix(".html"))
        .and_then(|raw| edit::id(raw).ok())
    else {
        return Response::text(404, "not found");
    };
    let (_, checked) = match read(dir) {
        Ok(r) => r,
        Err(response) => return response,
    };
    let Some(raw) = checked.game.as_ref() else {
        return Response::text(
            409,
            "the game has errors; fix them (see Game health on the graph page) to see the sheet",
        );
    };
    // Sheets show texts with `[[links]]` replaced.
    let shown = larp_model::text::expand(raw);
    let game = &shown;
    if !larp_graph::schedule_owners(game).contains(&&id) {
        return Response::text(404, "no organizer sheet for this id");
    }
    let template = match crate::build::read_named_template(dir, crate::build::ORGANIZER_TEMPLATE) {
        Ok(t) => t,
        Err(e) => return Response::text(500, format!("{e:?}")),
    };
    let schedule = larp_graph::schedule(game, &id);
    match larp_render::organizer_sheet_html(game, &schedule, template.as_deref()) {
        Ok(html) => Response::new(200, "text/html; charset=utf-8", html),
        Err(e) => Response::text(500, e.to_string()),
    }
}

/// The packing list or the handout cards, rendered from the game as it is on disk now.
fn packing_page(dir: &Path, page: larp_render::Page) -> Response {
    let (_, checked) = match read(dir) {
        Ok(r) => r,
        Err(response) => return response,
    };
    let Some(raw) = checked.game.as_ref() else {
        return Response::text(
            409,
            "the game has errors; fix them (see Game health on the graph page) to see this page",
        );
    };
    let game = larp_model::text::expand(raw);
    let name = match page {
        larp_render::Page::Packing => "templates/packing.html.j2",
        larp_render::Page::Handouts => "templates/handouts.html.j2",
    };
    let template = match crate::build::read_named_template(dir, name) {
        Ok(t) => t,
        Err(e) => return Response::text(500, format!("{e:?}")),
    };
    let packing = larp_graph::packing(&game);
    match larp_render::packing_html(&game, &packing, page, template.as_deref()) {
        Ok(html) => Response::new(200, "text/html; charset=utf-8", html),
        Err(e) => Response::text(500, e.to_string()),
    }
}

// ---------- writing ----------

fn body<T: DeserializeOwned>(raw: &str) -> std::result::Result<T, Response> {
    serde_json::from_str(raw).map_err(|e| Response::refused(format!("bad request: {e}")))
}

fn current(path: &Path) -> std::result::Result<Option<String>, Response> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Response::text(
            500,
            format!("cannot read {}: {e}", path.display()),
        )),
    }
}

fn save(dir: &Path, path: &Path, text: &str) -> Response {
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return Response::text(500, format!("cannot create {}: {e}", parent.display()));
    }
    if let Err(e) = fs::write(path, text) {
        return Response::text(500, format!("cannot write {}: {e}", path.display()));
    }
    saved(dir)
}

/// Success response with the game's state after the change.
fn saved(dir: &Path) -> Response {
    match read(dir) {
        Ok((_, checked)) => Response::json(
            200,
            &serde_json::json!({
                "ok": true,
                "errors": checked.count(Severity::Error),
                "warnings": checked.count(Severity::Warning),
            }),
        ),
        Err(response) => response,
    }
}

/// Runs `change` on the current text of `file` and writes the result.
fn update(
    dir: &Path,
    file: &Path,
    change: impl FnOnce(Option<&str>) -> std::result::Result<String, edit::EditError>,
) -> Response {
    let before = match current(file) {
        Ok(text) => text,
        Err(response) => return response,
    };
    match change(before.as_deref()) {
        Ok(text) => save(dir, file, &text),
        Err(e) => Response::refused(e),
    }
}

fn write_node(dir: &Path, raw: &str) -> Response {
    let e: edit::NodeEdit = match body(raw) {
        Ok(e) => e,
        Err(response) => return response,
    };
    let Ok(id) = edit::id(&e.id) else {
        return Response::refused(format!("invalid id `{}`", e.id));
    };
    let file = dir.join("actions").join(format!("{id}.toml"));
    update(dir, &file, |text| edit::node(text, &e))
}

#[derive(Deserialize)]
struct Delete {
    id: String,
}

fn delete_node(dir: &Path, raw: &str) -> Response {
    let d: Delete = match body(raw) {
        Ok(d) => d,
        Err(response) => return response,
    };
    let Ok(id) = edit::id(&d.id) else {
        return Response::refused(format!("invalid id `{}`", d.id));
    };
    let file = dir.join("actions").join(format!("{id}.toml"));
    if let Err(e) = fs::remove_file(&file) {
        return Response::refused(format!("cannot delete {}: {e}", file.display()));
    }
    let layout = dir.join("layout.toml");
    match current(&layout) {
        Ok(Some(text)) => match edit::layout_without(&text, &id) {
            Ok(new) => {
                let r = save(dir, &layout, &new);
                if r.status == 200 {
                    r
                } else {
                    Response::text(
                        r.status,
                        format!("`{id}` was deleted, but layout.toml was not updated"),
                    )
                }
            }
            // The node is gone; a broken layout.toml is reported by the next check.
            Err(_) => saved(dir),
        },
        Ok(None) => saved(dir),
        Err(response) => response,
    }
}

fn write_thing(dir: &Path, raw: &str) -> Response {
    let e: edit::ThingEdit = match body(raw) {
        Ok(e) => e,
        Err(response) => return response,
    };
    update(dir, &dir.join(e.kind.file()), |text| edit::thing(text, &e))
}

fn write_segment(dir: &Path, raw: &str) -> Response {
    let e: edit::SegmentEdit = match body(raw) {
        Ok(e) => e,
        Err(response) => return response,
    };
    update(dir, &dir.join("segments.toml"), |text| {
        edit::segment(text, &e)
    })
}

#[derive(Deserialize)]
struct LayoutBody {
    text: String,
}

fn write_layout(dir: &Path, raw: &str) -> Response {
    let l: LayoutBody = match body(raw) {
        Ok(l) => l,
        Err(response) => return response,
    };
    update(dir, &dir.join("layout.toml"), |current| {
        edit::layout(current, &l.text)
    })
}

fn write_map(dir: &Path, raw: &str) -> Response {
    let m: edit::MapEdit = match body(raw) {
        Ok(m) => m,
        Err(response) => return response,
    };
    update(dir, &dir.join("map.toml"), |_| edit::map(&m))
}

fn write_preset(dir: &Path, raw: &str) -> Response {
    let p: edit::PresetEdit = match body(raw) {
        Ok(p) => p,
        Err(response) => return response,
    };
    let file = match p.file() {
        Ok(f) => f,
        Err(e) => return Response::refused(e),
    };
    if let Err(e) = fs::create_dir_all(dir.join("map-presets")) {
        return Response::text(500, format!("cannot create map-presets: {e}"));
    }
    update(dir, &dir.join(file), |_| edit::preset(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copy of the example game in a fresh temporary folder.
    fn game_dir(name: &str) -> PathBuf {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lost-abbey");
        let dir =
            std::env::temp_dir().join(format!("larpidarium-serve-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for sub in ["", "actions", "people"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
            for entry in fs::read_dir(src.join(sub)).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    fs::copy(&path, dir.join(sub).join(path.file_name().unwrap())).unwrap();
                }
            }
        }
        dir
    }

    fn get(dir: &Path, url: &str) -> Response {
        handle(
            dir,
            &Request {
                method: "GET".into(),
                url: url.into(),
                host: Some("localhost:7878".into()),
                ..Request::default()
            },
        )
    }

    fn post(dir: &Path, url: &str, body: &str) -> Response {
        handle(
            dir,
            &Request {
                method: "POST".into(),
                url: url.into(),
                host: Some("127.0.0.1:7878".into()),
                token: true,
                body: body.into(),
                ..Request::default()
            },
        )
    }

    fn text(r: &Response) -> &str {
        std::str::from_utf8(&r.body).unwrap()
    }

    #[test]
    fn serves_the_viewer_and_editable_data() {
        let dir = game_dir("viewer");
        assert_eq!(get(&dir, "/").status, 200);
        assert_eq!(get(&dir, "/assets/graph.js?v=1").status, 200);
        let data = get(&dir, "/graph-data.js");
        assert!(text(&data).contains("window.LARP_EDIT = true"));
        assert!(text(&data).contains("\"segments\""));
        assert_eq!(get(&dir, "/players/inspector-hugo.html").status, 200);
        assert_eq!(get(&dir, "/players/nobody.html").status, 404);
        assert_eq!(get(&dir, "/players/../game.toml").status, 404);
        assert_eq!(get(&dir, "/organizers/org-petra.html").status, 200);
        assert_eq!(get(&dir, "/organizers/crypt.html").status, 200);
        assert_eq!(get(&dir, "/organizers/inspector-hugo.html").status, 404);
        assert_eq!(get(&dir, "/organizers/../game.toml").status, 404);
        assert_eq!(get(&dir, "/nope").status, 404);
        let packing = get(&dir, "/packing.html");
        assert_eq!(packing.status, 200);
        assert!(text(&packing).contains("Rezavý klíč"));
        let handouts = get(&dir, "/handouts.html");
        assert_eq!(handouts.status, 200);
        // Links are expanded on the cards.
        assert!(text(&handouts).contains("Bratr Anselm"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_foreign_hosts_and_posts_without_header() {
        let dir = game_dir("security");
        let foreign = handle(
            &dir,
            &Request {
                method: "GET".into(),
                url: "/".into(),
                host: Some("evil.example:7878".into()),
                ..Request::default()
            },
        );
        assert_eq!(foreign.status, 403);
        let no_token = handle(
            &dir,
            &Request {
                method: "POST".into(),
                url: "/api/segment".into(),
                host: Some("localhost".into()),
                body: "{}".into(),
                ..Request::default()
            },
        );
        assert_eq!(no_token.status, 403);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn created_node_is_written_and_changes_the_version() {
        let dir = game_dir("create");
        let before = text(&get(&dir, "/api/version")).to_owned();
        let r = post(
            &dir,
            "/api/node",
            r#"{"create":true,"kind":"action","id":"pray","title":"Modlit se","segment":"verdict"}"#,
        );
        assert_eq!(r.status, 200, "{}", text(&r));
        assert!(text(&r).contains("\"errors\":0"));
        let file = fs::read_to_string(dir.join("actions/pray.toml")).unwrap();
        assert!(file.contains("title = \"Modlit se\""));
        assert_ne!(text(&get(&dir, "/api/version")), before);

        let again = post(
            &dir,
            "/api/node",
            r#"{"create":true,"kind":"action","id":"pray","title":"x"}"#,
        );
        assert_eq!(again.status, 400);
        assert!(text(&again).contains("already exists"));

        let deleted = post(&dir, "/api/node/delete", r#"{"id":"pray"}"#);
        assert_eq!(deleted.status, 200);
        assert!(!dir.join("actions/pray.toml").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn games_with_errors_are_still_shown() {
        let dir = game_dir("draft");
        let r = post(
            &dir,
            "/api/thing",
            r#"{"create":true,"kind":"info","id":"ending-lost","name":"Konec","ending":true}"#,
        );
        assert!(text(&r).contains("\"errors\":1"), "{}", text(&r));
        let data = get(&dir, "/graph-data.js");
        assert!(text(&data).contains("cannot be reached"));

        // A broken file is skipped and reported, the rest of the game is still shown.
        fs::write(dir.join("items.toml"), "[[item]\n").unwrap();
        let skipped = get(&dir, "/graph-data.js");
        assert!(text(&skipped).contains("window.LARP_EDIT = true"));
        assert!(text(&skipped).contains("items.toml"));

        // Without a readable game.toml there is nothing to show but the problems.
        fs::write(dir.join("game.toml"), "[game\n").unwrap();
        let broken = get(&dir, "/graph-data.js");
        assert!(text(&broken).contains("window.LARP_GRAPH = null"));
        assert!(text(&broken).contains("LARP_PROBLEMS"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn other_sites_cannot_read_the_game() {
        let dir = game_dir("cross");
        let r = handle(
            &dir,
            &Request {
                method: "GET".into(),
                url: "/graph-data.js".into(),
                host: Some("localhost:7878".into()),
                cross_site: true,
                ..Request::default()
            },
        );
        assert_eq!(r.status, 403);
        assert!(is_local_url("http://localhost:7878/"));
        assert!(is_local_url("http://127.0.0.1/x"));
        assert!(!is_local_url("http://localhost.evil.example/"));
        assert!(!is_local_url("https://evil.example/?http://localhost"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn editing_an_item_keeps_keys_the_form_sends_back() {
        let dir = game_dir("item");
        // What the viewer sends for an unchanged rusty-key: everything it got from graph-data.
        let data = text(&get(&dir, "/graph-data.js")).to_owned();
        assert!(
            data.contains("\"category\":\"key\""),
            "graph data carries the category"
        );
        let r = post(
            &dir,
            "/api/thing",
            r#"{"kind":"item","id":"rusty-key","name":"Rezavý klíč","category":"key","owners":["brother-anselm"],"starting":true,"text":"x"}"#,
        );
        assert_eq!(r.status, 200, "{}", text(&r));
        let items = fs::read_to_string(dir.join("items.toml")).unwrap();
        assert!(items.contains("category = \"key\""), "{items}");
        assert!(items.contains("owners = [\"brother-anselm\"]"), "{items}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sheets_are_not_rendered_from_drafts() {
        let dir = game_dir("sheet-draft");
        let r = post(
            &dir,
            "/api/thing",
            r#"{"create":true,"kind":"info","id":"ending-lost","name":"Konec","ending":true}"#,
        );
        assert!(text(&r).contains("\"errors\":1"));
        assert_eq!(get(&dir, "/players/inspector-hugo.html").status, 409);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bad_requests_are_refused_with_a_message() {
        let dir = game_dir("bad");
        let r = post(&dir, "/api/node", "not json");
        assert_eq!(r.status, 400);
        assert!(text(&r).contains("bad request"));
        let r = post(&dir, "/api/layout", r#"{"text":"x = "}"#);
        assert_eq!(r.status, 400);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn map_is_saved_and_checked() {
        let dir = game_dir("map");
        let ok = r#"{"cells":["=..","=.."],"walls":[[0,0,0,2]],"rooms":{"crypt":{"x":1,"y":0,"w":2,"h":2}}}"#;
        let r = post(&dir, "/api/map", ok);
        assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
        let text = fs::read_to_string(dir.join("map.toml")).unwrap();
        assert!(
            text.contains("crypt = { x = 1, y = 0, w = 2, h = 2 }"),
            "{text}"
        );
        let bad = r##"{"cells":["#x"],"rooms":{}}"##;
        assert_eq!(post(&dir, "/api/map", bad).status, 400);
        let preset = r#"{"name":"sal","cells":["="],"walls":[[0,0,1,0]]}"#;
        let r = post(&dir, "/api/map-preset", preset);
        assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
        assert!(dir.join("map-presets/sal.toml").is_file());
        assert_eq!(post(&dir, "/api/map", bad).status, 400);
        let _ = fs::remove_dir_all(&dir);
    }
}
