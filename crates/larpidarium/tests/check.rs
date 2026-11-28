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

// ---------- packing list and handouts ----------

/// A fresh, empty temp folder for one test.
fn fresh(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("larpidarium-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

/// A copy of `lost-abbey` with one more item and one more info carrying a prop, a location and
/// a handout with characters LaTeX must escape and a `[[link]]`. New ids only, so this works
/// whatever props and handouts the example itself has. Returns (base folder, game folder).
fn abbey_with_handouts(name: &str) -> (PathBuf, PathBuf) {
    let base = fresh(name);
    let game = base.join("game");
    copy_dir(&example("lost-abbey"), &game);
    let append = |file: &str, extra: &str| {
        let path = game.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text + extra).unwrap();
    };
    append(
        "items.toml",
        "\n[[item]]\nid = \"test-candle\"\nname = \"Zkušební svíce\"\nowners = [\"novice-marta\"]\n\
         starting = true\nprop = \"vosková svíce 50 % & knot\"\nquantity = 4\nlocation = \"library\"\n",
    );
    append(
        "info.toml",
        "\n[[info]]\nid = \"test-note\"\nname = \"Zkušební vzkaz\"\nowners = [\"novice-marta\"]\n\
         starting = true\nhandout = \"Pro [[brother-anselm]]: cena 5 $ & 10 % — přijď_hned.\"\n",
    );
    (base, game)
}

/// A game with one player and nothing to pack; `items` is the content of `items.toml`, if any.
fn tiny_game(base: &std::path::Path, items: Option<&str>) -> PathBuf {
    let game = base.join("game");
    std::fs::create_dir_all(game.join("people")).unwrap();
    std::fs::write(
        game.join("game.toml"),
        "[game]\ntitle = \"Malá hra\"\nlanguage = \"cs\"\n",
    )
    .unwrap();
    std::fs::write(
        game.join("people/ann.toml"),
        "[person]\nname = \"Anna Řeháková\"\nrole = \"player\"\n",
    )
    .unwrap();
    if let Some(items) = items {
        std::fs::write(game.join("items.toml"), items).unwrap();
    }
    game
}

fn run_ok(args: &[&std::ffi::OsStr]) {
    let out = Command::new(env!("CARGO_BIN_EXE_larpidarium"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn build(game: &std::path::Path, out: &std::path::Path) {
    run_ok(&[
        "build".as_ref(),
        game.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
    ]);
}

fn pdf_tex_only(game: &std::path::Path, out: &std::path::Path) {
    run_ok(&[
        "pdf".as_ref(),
        game.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
        "--tex-only".as_ref(),
    ]);
}

#[test]
fn build_writes_the_packing_list_and_handouts() {
    let (base, game) = abbey_with_handouts("build-packing");
    let site = base.join("site");
    build(&game, &site);
    let packing = std::fs::read_to_string(site.join("packing.html")).unwrap();
    assert!(packing.contains("Balicí seznam"), "{packing}");
    assert!(packing.contains("Zkušební svíce"), "{packing}");
    assert!(
        packing.contains("vosková svíce 50 % &amp; knot"),
        "{packing}"
    );
    assert!(packing.contains("4×"), "{packing}");
    assert!(packing.contains("Knihovna"), "the room is named: {packing}");
    let handouts = std::fs::read_to_string(site.join("handouts.html")).unwrap();
    assert!(
        handouts.contains("Pro Bratr Anselm: cena 5 $ &amp; 10 % — přijď_hned."),
        "the link is expanded and the text escaped: {handouts}"
    );
    assert!(!handouts.contains("[[brother-anselm]]"), "{handouts}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn build_writes_both_pages_even_for_a_game_without_items() {
    let base = fresh("build-no-packing");
    let game = tiny_game(&base, None);
    let site = base.join("site");
    build(&game, &site);
    let packing = std::fs::read_to_string(site.join("packing.html")).unwrap();
    assert!(
        packing.contains("Hra nemá žádné předměty ani kartičky."),
        "{packing}"
    );
    let handouts = std::fs::read_to_string(site.join("handouts.html")).unwrap();
    assert!(
        handouts.contains("Žádná informace ani předmět nemá"),
        "{handouts}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pdf_tex_only_writes_the_packing_list_and_handouts() {
    let (base, game) = abbey_with_handouts("pdf-packing");
    let out = base.join("pdf");
    pdf_tex_only(&game, &out);
    let packing = std::fs::read_to_string(out.join("_packing.tex")).unwrap();
    assert!(packing.contains(r"\begin{document}"), "{packing}");
    assert!(
        packing.contains(r"vosková svíce 50 \% \& knot"),
        "{packing}"
    );
    let handouts = std::fs::read_to_string(out.join("_handouts.tex")).unwrap();
    assert!(
        handouts.contains(r"Pro Bratr Anselm: cena 5 \$ \& 10 \% — přijď\_hned."),
        "{handouts}"
    );
    assert!(!out.join("packing.tex").exists() && !out.join("handouts.tex").exists());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pdf_tex_only_writes_no_packing_list_or_handouts_without_items() {
    let base = fresh("pdf-no-packing");
    let game = tiny_game(&base, None);
    let out = base.join("pdf");
    pdf_tex_only(&game, &out);
    assert!(out.join("ann.tex").is_file(), "the player sheet is written");
    assert!(!out.join("_packing.tex").exists());
    assert!(!out.join("_handouts.tex").exists());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pdf_tex_only_writes_a_packing_list_but_no_handouts_without_a_handout() {
    let base = fresh("pdf-props-only");
    let items = "[[item]]\nid = \"key\"\nname = \"Klíč\"\nowners = [\"ann\"]\nstarting = true\n\
                 prop = \"mosazný klíč\"\nending = true\n";
    let game = tiny_game(&base, Some(items));
    let out = base.join("pdf");
    pdf_tex_only(&game, &out);
    let packing = std::fs::read_to_string(out.join("_packing.tex")).unwrap();
    assert!(packing.contains("mosazný klíč"), "{packing}");
    assert!(!out.join("_handouts.tex").exists());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn packing_and_handout_templates_in_the_game_folder_are_used() {
    let (base, game) = abbey_with_handouts("packing-tpl");
    std::fs::create_dir_all(game.join("templates")).unwrap();
    for (file, text) in [
        (
            "packing.html.j2",
            "PACK-HTML {% for h in p.handouts %}{% if h[0] == \"Zkušební vzkaz\" %}{{ h[0] }}{% endif %}{% endfor %}",
        ),
        ("handouts.html.j2", "CARDS-HTML {{ game_title }}"),
        ("packing.tex.j2", "PACK-TEX {{ game_title }}"),
        ("handouts.tex.j2", "CARDS-TEX {{ game_title }}"),
    ] {
        std::fs::write(game.join("templates").join(file), text).unwrap();
    }
    let site = base.join("site");
    build(&game, &site);
    let read = |p: PathBuf| std::fs::read_to_string(p).unwrap();
    assert_eq!(read(site.join("packing.html")), "PACK-HTML Zkušební vzkaz");
    assert_eq!(
        read(site.join("handouts.html")),
        "CARDS-HTML Ztracené opatství"
    );
    let out = base.join("pdf");
    pdf_tex_only(&game, &out);
    assert_eq!(read(out.join("_packing.tex")), "PACK-TEX Ztracené opatství");
    assert_eq!(
        read(out.join("_handouts.tex")),
        "CARDS-TEX Ztracené opatství"
    );
    let _ = std::fs::remove_dir_all(&base);
}

// ---------- character profiles ----------

#[test]
fn build_writes_a_profile_for_every_person_including_organizers_and_npcs() {
    let base = fresh("profiles");
    let game = base.join("game");
    copy_dir(&example("lost-abbey"), &game);
    std::fs::write(
        game.join("people/test-npc.toml"),
        "[person]\nname = \"Zkušební převor\"\nrole = \"npc\"\n",
    )
    .unwrap();
    let site = base.join("site");
    build(&game, &site);
    let mut people = 0;
    for entry in std::fs::read_dir(game.join("people")).unwrap() {
        let path = entry.unwrap().path();
        let id = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let profile = site.join("profiles").join(format!("{id}.html"));
        assert!(profile.is_file(), "missing profile of {id}");
        people += 1;
    }
    assert_eq!(people, 5, "four people in lost-abbey and the test NPC");
    let org = std::fs::read_to_string(site.join("profiles/org-petra.html")).unwrap();
    assert!(org.contains("Petra (organizátorka)"), "{org}");
    let npc = std::fs::read_to_string(site.join("profiles/test-npc.html")).unwrap();
    assert!(npc.contains("Zkušební převor"), "{npc}");
    assert!(npc.contains("cizí postava"), "{npc}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn anselm_profile_shows_his_faction_and_what_marta_thinks_of_him() {
    let base = fresh("profile-anselm");
    let site = base.join("site");
    build(&example("lost-abbey"), &site);
    let html = std::fs::read_to_string(site.join("profiles/brother-anselm.html")).unwrap();
    assert!(html.contains("Bratr Anselm"), "{html}");
    assert!(html.contains("Tvá frakce: Konvent"), "{html}");
    assert!(
        html.contains("Pro organizátory: obsahuje celý příběh"),
        "{html}"
    );
    assert!(html.contains("Co si o postavě myslí ostatní"), "{html}");
    let seen_by = html.split("Co si o postavě myslí ostatní").nth(1).unwrap();
    assert!(
        seen_by
            .contains("<dt>Novicka Marta</dt><dd>Pokladník. Od opatova zmizení je nervózní.</dd>"),
        "{html}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn profile_template_in_the_game_folder_is_used() {
    let base = fresh("profile-tpl");
    let game = base.join("game");
    copy_dir(&example("lost-abbey"), &game);
    std::fs::create_dir_all(game.join("templates")).unwrap();
    std::fs::write(
        game.join("templates/profile.html.j2"),
        "PROFILE {{ p.name }} {{ p.factions | join(\",\") }}",
    )
    .unwrap();
    let site = base.join("site");
    build(&game, &site);
    let html = std::fs::read_to_string(site.join("profiles/brother-anselm.html")).unwrap();
    assert_eq!(html, "PROFILE Bratr Anselm Konvent");
    let _ = std::fs::remove_dir_all(&base);
}

// ---------- counters ----------

#[test]
fn build_shows_the_suspicion_counter_on_the_crypt_sheet() {
    let base = fresh("build-counters");
    let site = base.join("site");
    build(&example("lost-abbey"), &site);
    let crypt = std::fs::read_to_string(site.join("organizers/crypt.html")).unwrap();
    assert!(
        crypt.contains("<b>Počitadla:</b> Hugovo podezření +3</p>"),
        "{crypt}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pdf_tex_only_shows_the_suspicion_counter_on_the_crypt_sheet() {
    let base = fresh("pdf-counters");
    let out = base.join("pdf");
    pdf_tex_only(&example("lost-abbey"), &out);
    let crypt = std::fs::read_to_string(out.join("organizers/crypt.tex")).unwrap();
    assert!(
        crypt.contains(r"\flowline{Počitadla}{Hugovo podezření +3}"),
        "{crypt}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn build_writes_counters_and_their_uses_into_the_graph_data() {
    let base = fresh("build-counters-json");
    let site = base.join("site");
    build(&example("lost-abbey"), &site);
    let text = std::fs::read_to_string(site.join("graph.json")).unwrap();
    let data: serde_json::Value = serde_json::from_str(&text).unwrap();
    let counters = data["counters"].as_array().unwrap();
    assert_eq!(counters.len(), 1, "{counters:?}");
    assert_eq!(counters[0]["id"], "suspicion");
    assert_eq!(counters[0]["name"], "Hugovo podezření");
    assert_eq!(
        (
            &counters[0]["start"],
            &counters[0]["min"],
            &counters[0]["max"]
        ),
        (&0.into(), &0.into(), &10.into())
    );
    let node = |id: &str| {
        data["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .unwrap_or_else(|| panic!("no node {id}"))
            .clone()
    };
    assert_eq!(
        node("open-crypt")["change"],
        serde_json::json!([["suspicion", 3]])
    );
    let accuse = &node("hugo-verdict")["options"][0];
    assert_eq!(accuse["label"], "Obvinit Anselma");
    assert_eq!(
        accuse["require"],
        serde_json::json!([{ "counter": "suspicion", "at_least": 6 }]),
        "a missing bound is left out, not null"
    );
    let _ = std::fs::remove_dir_all(&base);
}
