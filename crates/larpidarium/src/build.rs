//! `larpidarium build`: write the static site.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

use larp_model::model::Role;
use miette::{IntoDiagnostic, Result, WrapErr, miette};

use crate::check::load_game;

/// Viewer files, compiled into the binary so `build` works from any folder.
pub(crate) const ASSETS: &[(&str, &str)] = &[
    ("index.html", include_str!("../../../web/index.html")),
    ("assets/graph.js", include_str!("../../../web/graph.js")),
    ("assets/style.css", include_str!("../../../web/style.css")),
    (
        "assets/LICENSE-taraxaplan",
        include_str!("../../../web/LICENSE-taraxaplan"),
    ),
];

/// Runs the `build` command: validate `dir`, write the site into `out`, optionally open it.
pub(crate) fn run(dir: &Path, out: &Path, open: bool) -> Result<()> {
    let loaded = load_game(dir)?;
    let graph = larp_graph::build(
        &loaded.game,
        &loaded.diagnostics,
        loaded.playthroughs.as_ref(),
    );
    let json = serde_json::to_string_pretty(&graph).into_diagnostic()?;

    for (path, contents) in ASSETS {
        write(&out.join(path), contents)?;
    }
    write(&out.join("graph.json"), &json)?;
    // A script file instead of fetch(), so the page also works when opened as file://.
    write(
        &out.join("graph-data.js"),
        &format!("window.LARP_GRAPH = {json};\n"),
    )?;

    // Sheets show texts with `[[links]]` replaced.
    let shown = larp_model::text::expand(&loaded.game);
    let template = read_template(dir)?;
    let mut sheets = 0;
    for (id, person) in &shown.people {
        if person.role != Role::Player {
            continue;
        }
        let destiny = larp_graph::destiny(&shown, &graph, id);
        let html = larp_render::player_sheet_html(&shown, id, &destiny, template.as_deref())
            .map_err(|e| miette!("{e}"))
            .wrap_err_with(|| format!("cannot render the sheet of `{id}`"))?;
        write(&out.join("players").join(format!("{id}.html")), &html)?;
        sheets += 1;
    }

    let org_template = read_named_template(dir, ORGANIZER_TEMPLATE)?;
    let mut schedules = 0;
    for id in larp_graph::schedule_owners(&shown) {
        let schedule = larp_graph::schedule(&shown, id);
        let html = larp_render::organizer_sheet_html(&shown, &schedule, org_template.as_deref())
            .map_err(|e| miette!("{e}"))
            .wrap_err_with(|| format!("cannot render the organizer sheet of `{id}`"))?;
        write(&out.join("organizers").join(format!("{id}.html")), &html)?;
        schedules += 1;
    }

    let index = out.join("index.html");
    println!(
        "✔ wrote {}, {sheets} character sheet(s) in {} and {schedules} organizer sheet(s) in {}",
        index.display(),
        out.join("players").display(),
        out.join("organizers").display()
    );
    if open {
        let path = index.canonicalize().into_diagnostic()?;
        open_in_browser(path.as_os_str())?;
    }
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot create {}", parent.display()))?;
    }
    fs::write(path, contents)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot write {}", path.display()))
}

/// Where a game keeps its own organizer sheet template.
pub(crate) const ORGANIZER_TEMPLATE: &str = "templates/organizer.html.j2";

/// The game's own sheet template, `templates/player.html.j2`, if it has one.
pub(crate) fn read_template(dir: &Path) -> Result<Option<String>> {
    read_named_template(dir, "templates/player.html.j2")
}

/// `<game>/<name>`, if the game has that template.
pub(crate) fn read_named_template(dir: &Path, name: &str) -> Result<Option<String>> {
    let custom = dir.join(name);
    if !custom.is_file() {
        return Ok(None);
    }
    fs::read_to_string(&custom)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot read {}", custom.display()))
        .map(Some)
}

/// Opens a file path or URL in the default browser.
pub(crate) fn open_in_browser(target: &OsStr) -> Result<()> {
    let mut cmd = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        Command::new("xdg-open")
    };
    cmd.arg(target)
        .spawn()
        .into_diagnostic()
        .wrap_err("cannot open a browser; open the file above manually")?;
    Ok(())
}
