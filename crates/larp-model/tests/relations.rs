//! The reciprocity check of `[[person.relation]]`: relations between two player characters go both
//! ways.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::fmt::Write as _;

use larp_model::{Checked, Diagnostic, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";

/// Both players own this decision, so the destiny checks stay quiet.
const TALK: &str = "[decision]\ntitle = \"Rozhovor\"\nowners = [\"romeo\", \"julie\", \"tybalt\"]\n\n\
    [[decision.option]]\nlabel = \"Ano\"\n";

/// A person with a complete sheet and one relation per `(with, text)`.
fn person(name: &str, role: &str, relations: &[(&str, &str)]) -> String {
    let mut out = format!(
        "[person]\nname = \"{name}\"\nrole = \"{role}\"\ngoals = [\"Přežít\"]\n\n\
         [person.sheet]\nbackstory = \"b\"\nwhereabouts = \"w\"\n"
    );
    for (with, text) in relations {
        write!(
            out,
            "\n[[person.relation]]\nwith = \"{with}\"\ntext = \"{text}\"\n"
        )
        .unwrap();
    }
    out
}

/// [`person`] with `extra` keys (such as `dies = "s1"`) in the `[person]` table.
fn living(name: &str, role: &str, extra: &str, relations: &[(&str, &str)]) -> String {
    person(name, role, relations).replacen("goals = ", &format!("{extra}\ngoals = "), 1)
}

const SEGMENTS: &str =
    "[[segment]]\nid = \"s1\"\nname = \"Den\"\n\n[[segment]]\nid = \"s2\"\nname = \"Noc\"\n";

fn run(files: &[(&str, &str)]) -> Checked {
    let files: Vec<SourceFile> = [("game.toml", GAME), ("actions/talk.toml", TALK)]
        .into_iter()
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    load(&files)
}

fn warnings(checked: &Checked) -> Vec<&Diagnostic> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect()
}

#[test]
fn one_sided_relation_between_players_is_a_warning() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("julie", "Miluješ ji.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    let w = warnings(&checked);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].file, "people/romeo.toml");
    assert_eq!(
        w[0].message,
        "`romeo` has a relation with `julie`, but not the other way round"
    );
    assert!(checked.game.is_some(), "a warning must not stop the game");
}

#[test]
fn one_sided_relation_help_names_the_other_file_and_the_missing_with() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("julie", "Miluješ ji.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    let w = warnings(&checked);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(
        w[0].help.as_deref(),
        Some("add a [[person.relation]] with `with = \"romeo\"` to people/julie.toml")
    );
}

#[test]
fn one_sided_relation_warning_points_at_the_with_value() {
    let romeo = person("Romeo", "player", &[("julie", "Miluješ ji.")]);
    let checked = run(&[
        ("people/romeo.toml", &romeo),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    let w = warnings(&checked);
    assert_eq!(w.len(), 1, "{w:?}");
    let span = w[0].span.clone().unwrap();
    assert_eq!(&romeo[span], "\"julie\"");
}

#[test]
fn mutual_relation_between_players_gives_no_warning() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("julie", "Miluješ ji.")]),
        ),
        (
            "people/julie.toml",
            &person("Julie", "player", &[("romeo", "Je to Montek, ale…")]),
        ),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn each_one_sided_relation_gets_its_own_warning() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("julie", "Miluješ ji.")]),
        ),
        (
            "people/julie.toml",
            &person("Julie", "player", &[("tybalt", "Bratranec.")]),
        ),
        ("people/tybalt.toml", &person("Tybalt", "player", &[])),
    ]);
    let mut messages: Vec<&str> = warnings(&checked)
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    messages.sort_unstable();
    assert_eq!(
        messages,
        [
            "`julie` has a relation with `tybalt`, but not the other way round",
            "`romeo` has a relation with `julie`, but not the other way round",
        ]
    );
}

#[test]
fn one_sided_relation_from_a_player_to_an_npc_gives_no_warning() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("tybalt", "Zabil Merkucia.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn one_sided_relation_from_a_player_to_an_organizer_gives_no_warning() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("tybalt", "Vede hru.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "organizer", &[])),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn one_sided_relation_from_an_npc_to_a_player_gives_no_warning() {
    let checked = run(&[
        ("people/romeo.toml", &person("Romeo", "player", &[])),
        ("people/julie.toml", &person("Julie", "player", &[])),
        (
            "people/tybalt.toml",
            &person("Tybalt", "npc", &[("romeo", "Nenávidíš ho.")]),
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn relation_of_a_player_with_themself_is_skipped() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("romeo", "Pochybuješ o sobě.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn relation_with_an_unknown_person_is_only_the_reference_error() {
    let checked = run(&[
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("merkucio", "Přítel.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    assert!(warnings(&checked).is_empty(), "{:?}", checked.diagnostics);
    assert_eq!(
        checked.count(Severity::Error),
        1,
        "{:?}",
        checked.diagnostics
    );
}

// ---------- lives ----------

#[test]
fn one_sided_relation_between_players_who_never_meet_gives_no_warning() {
    let checked = run(&[
        ("segments.toml", SEGMENTS),
        (
            "people/romeo.toml",
            &living("Romeo", "player", "dies = \"s1\"", &[]),
        ),
        (
            "people/julie.toml",
            &living(
                "Julie",
                "player",
                "enters = \"s2\"",
                &[("romeo", "Slyšela jsi o něm.")],
            ),
        ),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    assert!(warnings(&checked).is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn one_sided_relation_between_players_alive_in_the_same_segment_is_a_warning() {
    let checked = run(&[
        ("segments.toml", SEGMENTS),
        (
            "people/romeo.toml",
            &living("Romeo", "player", "dies = \"s2\"", &[]),
        ),
        (
            "people/julie.toml",
            &living(
                "Julie",
                "player",
                "enters = \"s2\"",
                &[("romeo", "Miluješ ho.")],
            ),
        ),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    let messages: Vec<&str> = warnings(&checked)
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        messages,
        ["`julie` has a relation with `romeo`, but not the other way round"]
    );
}

#[test]
fn one_sided_relation_between_players_who_live_the_whole_game_with_segments_is_a_warning() {
    let checked = run(&[
        ("segments.toml", SEGMENTS),
        (
            "people/romeo.toml",
            &person("Romeo", "player", &[("julie", "Miluješ ji.")]),
        ),
        ("people/julie.toml", &person("Julie", "player", &[])),
        ("people/tybalt.toml", &person("Tybalt", "npc", &[])),
    ]);
    assert_eq!(warnings(&checked).len(), 1, "{:?}", checked.diagnostics);
}
