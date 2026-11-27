//! Characters' lives: `played_by`, `enters`, `dies`, phases and `players.toml`.

use larp_model::{Checked, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"en\"\n";
const SEGMENTS: &str = "[[segment]]\nid = \"one\"\nname = \"One\"\ngroup = \"Cycle\"\n\n\
    [[segment]]\nid = \"two\"\nname = \"Two\"\n\n[[segment]]\nid = \"three\"\nname = \"Three\"\n";
const PLAYERS: &str = "[[player]]\nid = \"p1\"\nname = \"Pat\"\n";

fn person(extra: &str) -> String {
    format!("[person]\nname = \"X\"\nrole = \"npc\"\n{extra}\n")
}

fn run(files: &[(&str, &str)]) -> Checked {
    let mut all = vec![
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        ("players.toml", PLAYERS),
    ];
    all.retain(|(path, _)| !files.iter().any(|(p, _)| p == path));
    all.extend_from_slice(files);
    let files: Vec<SourceFile> = all
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    load(&files)
}

fn messages(checked: &Checked, severity: Severity) -> Vec<String> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn consecutive_lives_of_one_player_are_clean() {
    let a = person("played_by = \"p1\"\ndies = \"one\"");
    let b = person(
        "played_by = \"p1\"\nenters = \"two\"\n\n[[person.phase]]\nsegment = \"three\"\n[person.phase.sheet]\nwhereabouts = \"later\"",
    );
    let checked = run(&[("people/a.toml", &a), ("people/b.toml", &b)]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let game = checked.game.unwrap();
    assert_eq!(game.players.len(), 1);
    assert_eq!(game.segments[0].group.as_deref(), Some("Cycle"));
    assert_eq!(
        game.people.values().map(|p| p.phases.len()).sum::<usize>(),
        1
    );
}

#[test]
fn overlapping_lives_are_reported_once() {
    let a = person("played_by = \"p1\"");
    let b = person("played_by = \"p1\"\nenters = \"two\"");
    let checked = run(&[("people/a.toml", &a), ("people/b.toml", &b)]);
    let warnings = messages(&checked, Severity::Warning);
    let overlaps: Vec<_> = warnings
        .iter()
        .filter(|m| m.contains("at the same time"))
        .collect();
    assert_eq!(overlaps.len(), 1, "{warnings:?}");
    assert!(overlaps[0].contains("from “Two”"), "{warnings:?}");
}

#[test]
fn idle_player_lists_the_segments() {
    let a = person("played_by = \"p1\"\ndies = \"one\"");
    let checked = run(&[("people/a.toml", &a)]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m == "Pat has no character in: Two, Three"),
        "{warnings:?}"
    );
}

#[test]
fn player_without_any_character_is_idle_everywhere() {
    let checked = run(&[]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m == "Pat has no character in: One, Two, Three"),
        "{warnings:?}"
    );
}

#[test]
fn dying_before_entering_is_an_error() {
    let a = person("played_by = \"p1\"\nenters = \"three\"\ndies = \"one\"");
    let checked = run(&[("people/a.toml", &a)]);
    let errors = messages(&checked, Severity::Error);
    assert!(
        errors.iter().any(|m| m.contains("before entering")),
        "{errors:?}"
    );
}

#[test]
fn acting_outside_life_is_a_warning() {
    let a = person("played_by = \"p1\"\nenters = \"two\"");
    let filler = person("played_by = \"p1\"\ndies = \"one\"");
    let act = "[action]\ntitle = \"Early\"\nsegment = \"one\"\nowners = [\"a\"]\n";
    let late = "[action]\ntitle = \"Late\"\nsegment = \"three\"\nowners = [\"f\"]\n";
    let checked = run(&[
        ("people/a.toml", &a),
        ("people/f.toml", &filler),
        ("actions/early.toml", act),
        ("actions/late.toml", late),
    ]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("acts in “One”, before entering")),
        "{warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("acts in “Three”, after dying")),
        "{warnings:?}"
    );
}

#[test]
fn phase_outside_life_is_a_warning() {
    let a = person("played_by = \"p1\"\ndies = \"two\"\n\n[[person.phase]]\nsegment = \"three\"");
    let b = person("played_by = \"p1\"\nenters = \"three\"");
    let checked = run(&[("people/a.toml", &a), ("people/b.toml", &b)]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("phase “Three” is outside")),
        "{warnings:?}"
    );
}

#[test]
fn unknown_player_and_segment_are_errors() {
    let a = person("played_by = \"nobody\"\nenters = \"nowhere\"");
    let checked = run(&[("people/a.toml", &a)]);
    let errors = messages(&checked, Severity::Error);
    assert!(errors.len() >= 2, "{errors:?}");
}

#[test]
fn lives_need_segments() {
    let files: Vec<SourceFile> = [
        ("game.toml", GAME),
        (
            "people/a.toml",
            "[person]\nname = \"A\"\nrole = \"npc\"\ndies = \"x\"\n",
        ),
    ]
    .iter()
    .map(|(p, t)| SourceFile {
        path: (*p).into(),
        text: (*t).into(),
    })
    .collect();
    let checked = load(&files);
    let errors = messages(&checked, Severity::Error);
    assert!(errors.iter().any(|m| m.contains("`x`")), "{errors:?}");
}

#[test]
fn every_overlapping_pair_is_reported() {
    let a = person("played_by = \"p1\"");
    let b = person("played_by = \"p1\"\nenters = \"two\"");
    let c = person("played_by = \"p1\"\nenters = \"three\"");
    let checked = run(&[
        ("people/a.toml", &a),
        ("people/b.toml", &b),
        ("people/c.toml", &c),
    ]);
    let warnings = messages(&checked, Severity::Warning);
    let overlaps = warnings
        .iter()
        .filter(|m| m.contains("at the same time"))
        .count();
    assert_eq!(overlaps, 3, "{warnings:?}");
}

#[test]
fn duplicate_phase_segment_is_a_warning() {
    let a = person(
        "played_by = \"p1\"\n\n[[person.phase]]\nsegment = \"two\"\n\n[[person.phase]]\nsegment = \"two\"",
    );
    let checked = run(&[("people/a.toml", &a)]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("two phases start at “Two”")),
        "{warnings:?}"
    );
}

#[test]
fn lives_are_checked_even_when_other_errors_exist() {
    let a = person("played_by = \"p1\"\nenters = \"three\"\ndies = \"one\"");
    let bad = "[action]\ntitle = \"Bad\"\nowners = [\"nobody\"]\n";
    let checked = run(&[("people/a.toml", &a), ("actions/bad.toml", bad)]);
    let errors = messages(&checked, Severity::Error);
    assert!(
        errors.iter().any(|m| m.contains("before entering")),
        "{errors:?}"
    );
}

#[test]
fn after_starts_the_life_at_the_predecessors_death() {
    let cycles = "[[cycle]]\nid = \"c1\"\nname = \"C\"\nhero = \"a\"\nfrom = \"two\"\n";
    let game =
        "[game]\ntitle = \"T\"\nlanguage = \"en\"\nsteps = [\"s1\", \"s2\", \"s3\", \"s4\"]\n";
    let a = person("played_by = \"p1\"\ndies = 2");
    let b = person("played_by = \"p1\"\nafter = \"a\"");
    let checked = run(&[
        ("game.toml", game),
        ("cycles.toml", cycles),
        ("people/a.toml", &a),
        ("people/b.toml", &b),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let g = checked.game.unwrap();
    let id = |s: &str| g.people.keys().find(|k| k.as_str() == s).unwrap().clone();
    let life = |s: &str| larp_model::time::life(&g, &id(s), &g.people[&id(s)]).unwrap();
    // 4 ticks per segment; cycle c1 is segment "two" (ticks 4..8), step 2 ends at tick 6.
    assert_eq!((life("a").start, life("a").end), (0, 6));
    assert_eq!((life("b").start, life("b").end), (6, 12));
}

#[test]
fn after_loop_does_not_hang() {
    let a = person("played_by = \"p1\"\nafter = \"b\"\ndies = \"one\"");
    let b = person("played_by = \"p1\"\nafter = \"a\"\ndies = \"two\"");
    let checked = run(&[("people/a.toml", &a), ("people/b.toml", &b)]);
    assert!(checked.game.is_some());
}
