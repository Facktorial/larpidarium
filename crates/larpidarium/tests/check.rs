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
