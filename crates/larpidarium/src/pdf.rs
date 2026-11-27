//! `larpidarium pdf`: player sheets as LaTeX, compiled to PDF with tectonic.

use std::fs;
use std::path::Path;

use larp_model::model::Role;
use miette::{IntoDiagnostic, Result, WrapErr, miette};

use crate::build::read_named_template;
use crate::check::load_game;

/// File name of the document with every sheet; ids cannot start with `_`, so no player clashes.
const ALL: &str = "_all-players.tex";
/// The same for organizer sheets.
const ALL_ORGANIZERS: &str = "_all-organizers.tex";
/// Where a game keeps its own player LaTeX template.
const TEMPLATE: &str = "templates/player.tex.j2";
/// Where a game keeps its own organizer LaTeX template.
const ORGANIZER_TEMPLATE: &str = "templates/organizer.tex.j2";

/// Runs the `pdf` command: one `.tex` (and `.pdf`) per player plus `_all-players` with every sheet,
/// and the same for organizer sheets in `organizers/` and `_all-organizers`.
pub(crate) fn run(dir: &Path, out: &Path, tex_only: bool) -> Result<()> {
    let loaded = load_game(dir)?;
    let game = &loaded.game;
    let graph = larp_graph::build(game, &loaded.diagnostics, loaded.playthroughs.as_ref());
    let template = read_named_template(dir, TEMPLATE)?;
    let template_name = template.as_ref().map_or_else(
        || "the built-in LaTeX template".to_owned(),
        |_| dir.join(TEMPLATE).display().to_string(),
    );

    let players: Vec<_> = game
        .people
        .iter()
        .filter(|(_, p)| p.role == Role::Player)
        .map(|(id, _)| (id, larp_graph::destiny(game, &graph, id)))
        .collect();
    fs::create_dir_all(out)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot create {}", out.display()))?;

    let mut files = Vec::new();
    for (id, destiny) in &players {
        let tex = render(game, &[(id, destiny)], template.as_deref(), &template_name)?;
        files.push(write(&out.join(format!("{id}.tex")), &tex)?);
    }
    if !players.is_empty() {
        let all: Vec<_> = players.iter().map(|(id, d)| (*id, d)).collect();
        let tex = render(game, &all, template.as_deref(), &template_name)?;
        files.push(write(&out.join(ALL), &tex)?);
    }
    files.extend(organizers(dir, game, out)?);
    if files.is_empty() {
        return Err(miette!(
            "the game has no people or rooms, so there are no sheets to print"
        ));
    }

    if tex_only {
        println!("✔ wrote {} LaTeX file(s) in {}", files.len(), out.display());
        return Ok(());
    }
    if !larp_pdf::available() {
        return Err(miette!(
            help = larp_pdf::INSTALL_HINT,
            "`tectonic` was not found; the .tex files are in {} (use --tex-only to skip PDFs)",
            out.display()
        ));
    }
    println!(
        "  compiling {} file(s) with tectonic (the first run downloads LaTeX packages)…",
        files.len()
    );
    let mut failed = Vec::new();
    for file in &files {
        match larp_pdf::compile(file) {
            Ok(done) if !done.missing_glyphs.is_empty() => println!(
                "  ⚠ {}: no glyph for {} (left out of the PDF)",
                done.pdf.display(),
                done.missing_glyphs.join(", ")
            ),
            Ok(_) => {}
            Err(e) => failed.push(e.to_string()),
        }
    }
    if !failed.is_empty() {
        return Err(miette!(
            "{} of {} file(s) failed to compile:\n\n{}",
            failed.len(),
            files.len(),
            failed.join("\n\n")
        ));
    }
    println!("✔ wrote {} PDF(s) in {}", files.len(), out.display());
    Ok(())
}

/// `organizers/<id>.tex` for every organizer, NPC and room, and `_all-organizers.tex`.
fn organizers(dir: &Path, game: &larp_model::Game, out: &Path) -> Result<Vec<std::path::PathBuf>> {
    let template = read_named_template(dir, ORGANIZER_TEMPLATE)?;
    let name = template.as_ref().map_or_else(
        || "the built-in organizer LaTeX template".to_owned(),
        |_| dir.join(ORGANIZER_TEMPLATE).display().to_string(),
    );
    let schedules: Vec<_> = larp_graph::schedule_owners(game)
        .into_iter()
        .map(|id| larp_graph::schedule(game, id))
        .collect();
    if schedules.is_empty() {
        return Ok(Vec::new());
    }
    let folder = out.join("organizers");
    fs::create_dir_all(&folder)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot create {}", folder.display()))?;
    let render = |list: &[&larp_graph::Schedule]| {
        larp_render::organizer_sheets_tex(game, list, template.as_deref())
            .map_err(|e| miette!("{e}"))
            .wrap_err_with(|| format!("cannot render the organizer sheets with {name}"))
    };
    let mut files = Vec::new();
    for s in &schedules {
        files.push(write(
            &folder.join(format!("{}.tex", s.id)),
            &render(&[s])?,
        )?);
    }
    let all: Vec<_> = schedules.iter().collect();
    files.push(write(&out.join(ALL_ORGANIZERS), &render(&all)?)?);
    Ok(files)
}

fn render(
    game: &larp_model::Game,
    people: &[(&larp_model::Id, &larp_graph::Destiny)],
    template: Option<&str>,
    template_name: &str,
) -> Result<String> {
    larp_render::player_sheets_tex(game, people, template)
        .map_err(|e| miette!("{e}"))
        .wrap_err_with(|| format!("cannot render the LaTeX sheets with {template_name}"))
}

fn write(path: &Path, text: &str) -> Result<std::path::PathBuf> {
    fs::write(path, text)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot write {}", path.display()))?;
    Ok(path.to_owned())
}
