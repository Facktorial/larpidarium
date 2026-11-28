//! End-to-end tests: run the real `larpidarium` binary.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::process::Command;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn example_game_passes_check() {
    let out = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("check")
        .arg(example("lost-abbey"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr:\n{stderr}");
    assert!(stdout.contains("Ztracené opatství"), "stdout:\n{stdout}");
    assert!(stdout.contains("0 warning(s)"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("3 reachable ending(s)"),
        "stdout:\n{stdout}"
    );
}

#[test]
fn missing_folder_fails() {
    let out = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .args(["check", "no/such/folder"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn build_writes_the_site() {
    let out = std::env::temp_dir().join(format!("larpidarium-build-test-{}", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("build")
        .arg(example("lost-abbey"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    for file in [
        "index.html",
        "graph.json",
        "graph-data.js",
        "assets/graph.js",
        "assets/style.css",
    ] {
        assert!(out.join(file).is_file(), "missing {file}");
    }
    let data = std::fs::read_to_string(out.join("graph-data.js")).unwrap();
    assert!(data.starts_with("window.LARP_GRAPH = {"));
    assert!(data.contains("Ztracené opatství"));
    std::fs::remove_dir_all(&out).unwrap();
}

fn check(name: &str) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("check")
        .arg(example(name))
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn flawed_example_reports_every_planted_issue_but_passes() {
    let (ok, stdout, stderr) = check("flawed-abbey");
    assert!(ok, "warnings must not fail the check:\n{stderr}");
    assert!(stdout.contains("13 warning(s)"), "{stdout}");
    for expected in [
        "unknown file",
        "has a relation with `inspector-hugo`, but not the other",
        "`silver-chalice` is never needed",
        "`ghost-chain` is never needed",
        "no decision of their own",
        "no `whereabouts`",
        "no goals",
        "gets stuck",
        "“Padělat klíč” can never happen",
        "“Otisknout klíč do vosku” can never happen",
        "can never be chosen",
    ] {
        assert!(
            stderr.contains(expected),
            "missing {expected:?} in:\n{stderr}"
        );
    }
}

#[test]
fn broken_example_fails_on_the_unreachable_ending() {
    let (ok, _, stderr) = check("broken-abbey");
    assert!(!ok);
    assert!(
        stderr.contains("ending `ending-pilgrimage` cannot be reached"),
        "{stderr}"
    );
    assert!(stderr.contains("1 error(s)"), "{stderr}");
}

#[test]
fn build_writes_a_sheet_per_player() {
    let out = std::env::temp_dir().join(format!("larpidarium-sheets-test-{}", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("build")
        .arg(example("lost-abbey"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    let sheet = std::fs::read_to_string(out.join("players/inspector-hugo.html")).unwrap();
    assert!(sheet.contains("Inspektor Hugo"));
    assert!(sheet.contains("Verdikt inspektora"));
    assert!(
        !out.join("players/org-petra.html").exists(),
        "organizers get no player sheet"
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn evzentura_example_builds_and_lists_open_gaps() {
    // A transcript of unfinished design notes: warnings are the to-do list, but no errors,
    // so the GitHub Pages build keeps working.
    let (ok, stdout, stderr) = check("evzentura");
    assert!(ok, "{stderr}");
    assert!(stdout.contains("2 reachable ending(s)"), "{stdout}");
    assert!(stderr.contains("is never needed"), "{stderr}");
}

#[test]
fn kolobeh_example_has_cycles_lives_and_three_endings() {
    let (ok, stdout, stderr) = check("kolobeh");
    assert!(ok, "{stderr}");
    assert!(stdout.contains("15 people"), "{stdout}");
    assert!(stdout.contains("0 warning(s)"), "{stdout}\n{stderr}");
    assert!(stdout.contains("3 reachable ending(s)"), "{stdout}");
}

#[test]
fn pdf_tex_only_writes_a_document_per_player_and_one_with_all() {
    let out_dir = std::env::temp_dir().join(format!("larpidarium-pdf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    for game in ["lost-abbey", "kolobeh"] {
        let out = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
            .arg("pdf")
            .arg(example(game))
            .arg("--out")
            .arg(out_dir.join(game))
            .arg("--tex-only")
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{game}: {stderr}");
    }
    let abbey = out_dir.join("lost-abbey");
    for id in ["brother-anselm", "inspector-hugo", "novice-marta"] {
        let tex = std::fs::read_to_string(abbey.join(format!("{id}.tex"))).unwrap();
        assert!(tex.contains(r"\begin{document}"), "{id}");
    }
    assert!(
        !abbey.join("org-petra.tex").exists(),
        "organizers get no player sheet"
    );
    let all = std::fs::read_to_string(abbey.join("_all-players.tex")).unwrap();
    assert_eq!(all.matches(r"\sheetpart{I}").count(), 3);
    let kolobeh = std::fs::read_to_string(out_dir.join("kolobeh/_all-players.tex")).unwrap();
    assert_eq!(kolobeh.matches(r"\sheetpart{I}").count(), 15);
    assert!(
        kolobeh.contains(r"\sheetpart{III}"),
        "cycle heroes have phases"
    );
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn pdf_tex_only_writes_organizer_sheets_and_one_with_all() {
    let out = std::env::temp_dir().join(format!("larpidarium-pdf-org-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let run = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("pdf")
        .arg(example("lost-abbey"))
        .arg("--out")
        .arg(&out)
        .arg("--tex-only")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let petra = std::fs::read_to_string(out.join("organizers/org-petra.tex")).unwrap();
    assert!(petra.contains(r"\begin{document}"));
    assert!(petra.contains(r"\orgpart{Vytížení}"));
    let crypt = std::fs::read_to_string(out.join("organizers/crypt.tex")).unwrap();
    assert!(crypt.contains("Krypta"));
    assert!(
        !out.join("organizers/inspector-hugo.tex").exists(),
        "players get no organizer sheet"
    );
    let all = std::fs::read_to_string(out.join("_all-organizers.tex")).unwrap();
    assert_eq!(all.matches(r"\documentclass").count(), 1);
    // org-petra, the three rooms (gate, crypt, library) and the faction (bratri).
    assert_eq!(all.matches(r"\orgpart{Vytížení}").count(), 5);
    assert!(all.contains("Konvent"), "the faction has an organizer part");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_an_organizer_sheet_for_organizers() {
    let out = std::env::temp_dir().join(format!("larpidarium-org-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("build")
        .arg(example("lost-abbey"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    let sheet = std::fs::read_to_string(out.join("organizers/org-petra.html")).unwrap();
    assert!(sheet.contains("Petra (organizátorka)"), "{sheet}");
    assert!(sheet.contains("Vytížení"));
    assert!(out.join("organizers/crypt.html").is_file());
    assert!(
        !out.join("organizers/inspector-hugo.html").exists(),
        "players get no organizer sheet"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn check_prints_the_load_of_organizers_and_rooms() {
    let (ok, stdout, stderr) = check("lost-abbey");
    assert!(ok, "{stderr}");
    let line = stdout
        .lines()
        .find(|l| l.starts_with("  load:"))
        .unwrap_or_else(|| panic!("no load line in:\n{stdout}"));
    assert!(line.contains("Krypta 2"), "{line}");
}

/// Copies a directory tree (the example game) so a test can change it.
fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn organizer_templates_in_the_game_folder_are_used() {
    let base = std::env::temp_dir().join(format!("larpidarium-org-tpl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let game = base.join("game");
    copy_dir(&example("lost-abbey"), &game);
    std::fs::create_dir_all(game.join("templates")).unwrap();
    std::fs::write(
        game.join("templates/organizer.html.j2"),
        "CUSTOM-HTML {{ s.name }}",
    )
    .unwrap();
    std::fs::write(
        game.join("templates/organizer.tex.j2"),
        "CUSTOM-TEX {% for x in sheets %}{{ x.schedule.name }};{% endfor %}",
    )
    .unwrap();
    let bin = env!("CARGO_BIN_EXE_larpidarium");
    let built = Command::new(bin)
        .args(["build".as_ref(), game.as_os_str(), "--out".as_ref()])
        .arg(base.join("site"))
        .status()
        .unwrap();
    assert!(built.success());
    let html = std::fs::read_to_string(base.join("site/organizers/crypt.html")).unwrap();
    assert_eq!(html, "CUSTOM-HTML Krypta");
    let pdf = Command::new(bin)
        .args([
            "pdf".as_ref(),
            game.as_os_str(),
            "--tex-only".as_ref(),
            "--out".as_ref(),
        ])
        .arg(base.join("pdf"))
        .status()
        .unwrap();
    assert!(pdf.success());
    let tex = std::fs::read_to_string(base.join("pdf/organizers/crypt.tex")).unwrap();
    assert_eq!(tex, "CUSTOM-TEX Krypta;");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn check_prints_the_load_but_build_does_not() {
    let out = std::env::temp_dir().join(format!("larpidarium-load-{}", std::process::id()));
    let run = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("build")
        .arg(example("lost-abbey"))
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(run.status.success());
    assert!(!String::from_utf8_lossy(&run.stdout).contains("  load:"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn check_prints_writing_progress() {
    let (ok, stdout, stderr) = check("lost-abbey");
    assert!(ok, "{stderr}");
    let line = stdout
        .lines()
        .find(|l| l.starts_with("  progress:"))
        .unwrap_or_else(|| panic!("no progress line in:\n{stdout}"));
    assert!(
        line.contains("1 in review") && line.contains("Petra 1/3"),
        "{line}"
    );
}

/// A copy of `lost-abbey` in a fresh temp folder, with a `blocks.toml` and a `[[datum]]` link in
/// the brief of `marta-letter`. Returns (base folder, game folder).
fn abbey_with_blocks(name: &str, link: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("larpidarium-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let game = base.join("game");
    copy_dir(&example("lost-abbey"), &game);
    // Added to the example's own blocks.
    let blocks = std::fs::read_to_string(game.join("blocks.toml")).unwrap_or_default();
    std::fs::write(
        game.join("blocks.toml"),
        blocks
            + "\n[[block]]\nid = \"datum\"\ntext = \"třináctého března\"\n\n\
         [[block]]\nid = \"zvesti-z-cel\"\ntitle = \"Zvěsti z cel\"\n\
         text = \"Šeptá se, že [[brother-anselm]] ví víc, než říká.\"\nshow = [\"novice-marta\"]\n",
    )
    .unwrap();
    let letter = game.join("actions/marta-letter.toml");
    let text = std::fs::read_to_string(&letter).unwrap();
    let old = "brief = \"Víš, co je v dopise.";
    assert!(text.contains(old), "the example changed: {text}");
    let text = text.replace(
        old,
        &format!("brief = \"Dopis je z {link}. Víš, co je v dopise."),
    );
    std::fs::write(&letter, text).unwrap();
    (base, game)
}

fn run_check(game: &std::path::Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("check")
        .arg(game)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn check_passes_a_game_with_blocks_and_links_without_warnings() {
    let (base, game) = abbey_with_blocks("blocks-check", "[[datum]]");
    let (ok, stdout, stderr) = run_check(&game);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("0 warning(s)"), "{stdout}\n{stderr}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn check_fails_on_an_unknown_link_with_a_suggestion() {
    let (base, game) = abbey_with_blocks("blocks-unknown", "[[datun]]");
    let (ok, _, stderr) = run_check(&game);
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("unknown link [[datun]]"), "{stderr}");
    assert!(stderr.contains("did you mean [[datum]]?"), "{stderr}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn build_expands_links_on_sheets_and_keeps_them_for_the_editor() {
    let (base, game) = abbey_with_blocks("blocks-build", "[[datum]]");
    let site = base.join("site");
    let run = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("build")
        .arg(&game)
        .arg("--out")
        .arg(&site)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let sheet = std::fs::read_to_string(site.join("players/novice-marta.html")).unwrap();
    assert!(sheet.contains("Dopis je z třináctého března."), "{sheet}");
    assert!(!sheet.contains("[["), "{sheet}");
    assert!(sheet.contains("<h3>Zvěsti z cel</h3>"), "{sheet}");
    assert!(
        sheet.contains("Šeptá se, že Bratr Anselm ví víc"),
        "{sheet}"
    );
    let hugo = std::fs::read_to_string(site.join("players/inspector-hugo.html")).unwrap();
    assert!(
        !hugo.contains("Zvěsti z cel"),
        "only `show` people get the block"
    );

    let data = std::fs::read_to_string(site.join("graph-data.js")).unwrap();
    // Only briefs with links keep their text as written: the changed one and the example's own.
    assert_eq!(data.matches("\"raw_brief\"").count(), 2, "{data}");
    assert!(
        data.contains("\"raw_brief\": \"Dopis je z [[datum]]. Víš, co je v dopise."),
        "{data}"
    );
    assert!(data.contains("\"blocks\""), "{data}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pdf_expands_links_on_tex_sheets() {
    let (base, game) = abbey_with_blocks("blocks-pdf", "[[datum]]");
    let out = base.join("pdf");
    let run = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("pdf")
        .arg(&game)
        .arg("--out")
        .arg(&out)
        .arg("--tex-only")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let tex = std::fs::read_to_string(out.join("novice-marta.tex")).unwrap();
    assert!(tex.contains("Dopis je z třináctého března."), "{tex}");
    assert!(tex.contains(r"\subhead{Zvěsti z cel}"), "{tex}");
    assert!(!tex.contains("[[datum]]"), "{tex}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn build_writes_faction_sections_and_a_faction_organizer_sheet() {
    let out = std::env::temp_dir().join(format!("larpidarium-faction-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .arg("build")
        .arg(example("lost-abbey"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    let marta = std::fs::read_to_string(out.join("players/novice-marta.html")).unwrap();
    assert!(marta.contains("Tvá frakce"), "{marta}");
    assert!(marta.contains("Konvent"), "{marta}");
    assert!(
        marta.contains("Bratr Anselm"),
        "the other member is named: {marta}"
    );
    assert!(
        marta.contains("Zvěsti z kláštera"),
        "the block shown to the faction is on the sheet: {marta}"
    );
    let hugo = std::fs::read_to_string(out.join("players/inspector-hugo.html")).unwrap();
    assert!(!hugo.contains("Tvá frakce"), "Hugo is not in the Konvent");
    assert!(!hugo.contains("Zvěsti z kláštera"));
    let bratri = std::fs::read_to_string(out.join("organizers/bratri.html")).unwrap();
    assert!(bratri.contains("Konvent"), "{bratri}");
    assert!(bratri.contains("frakce"), "{bratri}");
    let _ = std::fs::remove_dir_all(&out);
}
