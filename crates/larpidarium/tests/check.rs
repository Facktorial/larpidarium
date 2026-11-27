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
    assert!(stdout.contains("11 warning(s)"), "{stdout}");
    for expected in [
        "unknown file",
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
    // org-petra and the three rooms (gate, crypt, library).
    assert_eq!(all.matches(r"\orgpart{Vytížení}").count(), 4);
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
