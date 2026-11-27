//! `larpidarium check`: read a game folder, validate it, print the problems.

use std::fs;
use std::path::Path;

use larp_model::model::Node;
use larp_model::{Checked, Diagnostic, Game, Playthroughs, Severity, SourceFile};
use miette::{
    IntoDiagnostic, LabeledSpan, MietteDiagnostic, NamedSource, Report, Result, WrapErr, miette,
};

/// Runs the `check` command on `dir`.
pub(crate) fn run(dir: &Path) -> Result<()> {
    load_game(dir).map(|_| ())
}

/// A valid game with everything `check` found out about it.
pub(crate) struct Loaded {
    /// The game.
    pub(crate) game: Game,
    /// Warnings (a loaded game has no errors).
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// Simulation result.
    pub(crate) playthroughs: Option<Playthroughs>,
}

/// Reads and validates `dir`, prints all diagnostics and a summary, returns the game if valid.
pub(crate) fn load_game(dir: &Path) -> Result<Loaded> {
    let files = read_game_dir(dir)?;
    let checked = larp_model::load(&files);
    for diagnostic in &checked.diagnostics {
        eprintln!("{:?}", to_report(diagnostic, &files));
    }
    let errors = checked.count(Severity::Error);
    let warnings = checked.count(Severity::Warning);
    let summary = checked.game.as_ref().map(|g| summary(g, &checked));
    match (checked.game, summary) {
        (Some(game), Some(summary)) => {
            println!("✔ {summary}");
            if let Some(p) = &checked.playthroughs {
                print_playthroughs(&game, p);
            }
            Ok(Loaded {
                game,
                diagnostics: checked.diagnostics,
                playthroughs: checked.playthroughs,
            })
        }
        _ => Err(miette!(
            "{} has {errors} error(s) and {warnings} warning(s)",
            dir.display()
        )),
    }
}

/// Reads every `.toml` file in `dir`, `dir/actions` and `dir/people`, sorted by path.
pub(crate) fn read_game_dir(dir: &Path) -> Result<Vec<SourceFile>> {
    if !dir.is_dir() {
        return Err(miette!("{} is not a folder", dir.display()));
    }
    let mut files = Vec::new();
    for sub in ["", "actions", "people", "map-presets"] {
        let folder = dir.join(sub);
        if !folder.is_dir() {
            continue;
        }
        let entries = fs::read_dir(&folder)
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot list {}", folder.display()))?;
        for entry in entries {
            let path = entry.into_diagnostic()?.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "toml") {
                let text = fs::read_to_string(&path)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("cannot read {}", path.display()))?;
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let rel = if sub.is_empty() {
                    name.into_owned()
                } else {
                    format!("{sub}/{name}")
                };
                files.push(SourceFile { path: rel, text });
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn to_report(d: &Diagnostic, files: &[SourceFile]) -> Report {
    let severity = match d.severity {
        Severity::Error => miette::Severity::Error,
        Severity::Warning => miette::Severity::Warning,
    };
    let message = if d.span.is_some() {
        d.message.clone()
    } else {
        format!("{}: {}", d.file, d.message)
    };
    let mut diag = MietteDiagnostic::new(message).with_severity(severity);
    if let Some(help) = &d.help {
        diag = diag.with_help(help);
    }
    if let Some(span) = &d.span {
        diag = diag.with_label(LabeledSpan::at(span.clone(), "here"));
    }
    let report = Report::new(diag);
    match files.iter().find(|f| f.path == d.file) {
        Some(f) if d.span.is_some() => {
            report.with_source_code(NamedSource::new(&f.path, f.text.clone()).with_language("toml"))
        }
        _ => report,
    }
}

fn print_playthroughs(game: &Game, p: &Playthroughs) {
    let name = |id: &larp_model::Id| {
        game.items
            .iter()
            .chain(&game.info)
            .find(|t| t.id.get_ref() == id)
            .map_or_else(|| id.to_string(), |t| t.name.clone())
    };
    let limit = if p.truncated { " (stopped early)" } else { "" };
    println!(
        "  playthroughs: {} possible outcomes{limit}, {} reachable ending(s)",
        p.end_states,
        p.endings.len()
    );
    for (id, path) in &p.endings {
        let route = if path.is_empty() {
            "no decision needed".to_owned()
        } else {
            path.join("; ")
        };
        println!("  • {}: {route}", name(id));
    }
}

fn summary(game: &Game, checked: &Checked) -> String {
    let decisions = game
        .nodes
        .values()
        .filter(|n| matches!(n, Node::Decision(_)))
        .count();
    format!(
        "{}: {} actions ({decisions} decisions), {} people, {} rooms, {} items, {} info · {} warning(s)",
        game.meta.title,
        game.nodes.len(),
        game.people.len(),
        game.rooms.len(),
        game.items.len(),
        game.info.len(),
        checked.count(Severity::Warning),
    )
}
