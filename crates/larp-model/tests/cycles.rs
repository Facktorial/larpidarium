//! Hero cycles: the shared time axis (`larp_model::time`) and the checks on cycles and heroes.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_model::model::Person;
use larp_model::time::{self, Span};
use larp_model::{Checked, Game, Id, Severity, SourceFile, load};

/// Four steps, so every world segment is 4 ticks: "one" 0..4, "two" 4..8, "three" 8..12.
const GAME: &str = "[game]\ntitle = \"Koloběh\"\nlanguage = \"cs\"\n\
    steps = [\"Ty\", \"Potřeba\", \"Odchod\", \"Návrat\"]\n";
const SEGMENTS: &str = "[[segment]]\nid = \"one\"\nname = \"Noc 1\"\n\n\
    [[segment]]\nid = \"two\"\nname = \"Noc 2\"\n\n[[segment]]\nid = \"three\"\nname = \"Noc 3\"\n";
const PLAYERS: &str = "[[player]]\nid = \"p1\"\nname = \"Pat\"\n";
/// `short` covers segment "two"; `long` covers "two" and "three", so its steps are 2 ticks each.
const CYCLES: &str = "[[cycle]]\nid = \"short\"\nname = \"Dokařův cyklus\"\nhero = \"a\"\nfrom = \"two\"\n\n\
    [[cycle]]\nid = \"long\"\nname = \"Řehořův cyklus\"\nhero = \"h\"\nfrom = \"two\"\nto = \"three\"\n";

fn person(extra: &str) -> String {
    format!("[person]\nname = \"Řehoř Čížek\"\nrole = \"npc\"\n{extra}\n")
}

/// A clean base: `a` (p1) is the hero of `short` and dies at step 2, `b` (p1) follows `a`,
/// `h` is the hero of `long` and dies at step 4, played by nobody.
fn base() -> Vec<(String, String)> {
    [
        ("game.toml", GAME.to_owned()),
        ("segments.toml", SEGMENTS.to_owned()),
        ("players.toml", PLAYERS.to_owned()),
        ("cycles.toml", CYCLES.to_owned()),
        ("people/a.toml", person("played_by = \"p1\"\ndies = 2")),
        ("people/b.toml", person("played_by = \"p1\"\nafter = \"a\"")),
        ("people/h.toml", person("dies = 4")),
    ]
    .into_iter()
    .map(|(p, t)| (p.to_owned(), t))
    .collect()
}

/// Loads `base()` with `files` replacing or adding files.
fn run(files: &[(&str, &str)]) -> Checked {
    let mut all = base();
    all.retain(|(path, _)| !files.iter().any(|(p, _)| p == path));
    all.extend(
        files
            .iter()
            .map(|(p, t)| ((*p).to_owned(), (*t).to_owned())),
    );
    let files: Vec<SourceFile> = all
        .into_iter()
        .map(|(path, text)| SourceFile { path, text })
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

fn has(checked: &Checked, severity: Severity, needle: &str) -> bool {
    messages(checked, severity)
        .iter()
        .any(|m| m.contains(needle))
}

fn game(files: &[(&str, &str)]) -> Game {
    let checked = run(files);
    assert!(
        checked.diagnostics.is_empty(),
        "fixture must be clean: {:?}",
        checked.diagnostics
    );
    checked.game.unwrap()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

fn span(start: u32, end: u32) -> Span {
    Span { start, end }
}

fn cycle<'a>(g: &'a Game, c: &str) -> &'a larp_model::model::Cycle {
    time::find_cycle(g, &id(c)).unwrap()
}

fn person_of<'a>(g: &'a Game, p: &str) -> &'a Person {
    &g.people[&id(p)]
}

// ---------- time ----------

#[test]
fn base_fixture_is_clean() {
    let checked = run(&[]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn unit_is_the_number_of_steps() {
    let g = game(&[]);
    assert_eq!(time::unit(&g), 4);
    assert_eq!(time::total(&g), 12);
}

#[test]
fn unit_is_one_when_there_are_no_steps() {
    let checked = run(&[
        ("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"en\"\n"),
        ("cycles.toml", ""),
        ("people/a.toml", &person("played_by = \"p1\"")),
        ("people/b.toml", &person("")),
        ("people/h.toml", &person("")),
    ]);
    let g = checked.game.unwrap();
    assert_eq!(time::unit(&g), 1);
    assert_eq!(time::segment(&g, &id("three")), Some(span(2, 3)));
}

#[test]
fn segment_spans_follow_file_order() {
    let g = game(&[]);
    assert_eq!(time::segment(&g, &id("one")), Some(span(0, 4)));
    assert_eq!(time::segment(&g, &id("two")), Some(span(4, 8)));
    assert_eq!(time::segment(&g, &id("three")), Some(span(8, 12)));
    assert_eq!(time::segment(&g, &id("nowhere")), None);
}

#[test]
fn cycle_span_runs_from_start_of_from_to_end_of_to() {
    let g = game(&[]);
    assert_eq!(time::cycle(&g, cycle(&g, "short")), Some(span(4, 8)));
    assert_eq!(time::cycle(&g, cycle(&g, "long")), Some(span(4, 12)));
}

#[test]
fn steps_of_a_one_segment_cycle_are_one_tick_each() {
    let g = game(&[]);
    let c = cycle(&g, "short");
    assert_eq!(time::step(&g, c, 1), Some(span(4, 5)));
    assert_eq!(time::step(&g, c, 2), Some(span(5, 6)));
    assert_eq!(time::step(&g, c, 4), Some(span(7, 8)));
}

#[test]
fn steps_of_a_two_segment_cycle_split_both_segments_evenly() {
    let g = game(&[]);
    let c = cycle(&g, "long");
    assert_eq!(time::step(&g, c, 1), Some(span(4, 6)));
    assert_eq!(time::step(&g, c, 2), Some(span(6, 8)));
    assert_eq!(time::step(&g, c, 3), Some(span(8, 10)), "starts night 3");
    assert_eq!(time::step(&g, c, 4), Some(span(10, 12)));
}

#[test]
fn step_zero_and_steps_past_the_end_are_none() {
    let g = game(&[]);
    let c = cycle(&g, "long");
    assert_eq!(time::step(&g, c, 0), None);
    assert_eq!(time::step(&g, c, 5), None);
    assert_eq!(time::step(&g, c, u8::MAX), None);
}

#[test]
fn node_in_a_segment_spans_the_segment() {
    let act = "[action]\ntitle = \"Útěk\"\nsegment = \"three\"\n";
    let g = game(&[("actions/run.toml", act)]);
    assert_eq!(time::node(&g, &g.nodes[&id("run")]), Some(span(8, 12)));
}

#[test]
fn node_in_a_cycle_spans_its_step() {
    let act = "[action]\ntitle = \"Odchod\"\ncycle = \"long\"\nstep = 3\nowners = [\"h\"]\n";
    let g = game(&[("actions/go.toml", act)]);
    assert_eq!(time::node(&g, &g.nodes[&id("go")]), Some(span(8, 10)));
}

#[test]
fn node_without_segment_or_cycle_has_no_time() {
    let g = game(&[("actions/loose.toml", "[action]\ntitle = \"Volné\"\n")]);
    assert_eq!(time::node(&g, &g.nodes[&id("loose")]), None);
}

#[test]
fn life_ends_at_the_end_of_the_death_step_of_the_hero_cycle() {
    let g = game(&[("people/h.toml", &person("enters = \"one\"\ndies = 3"))]);
    // long: step 3 is ticks 8..10
    assert_eq!(
        time::life(&g, &id("h"), person_of(&g, "h")),
        Some(span(0, 10))
    );
}

#[test]
fn life_ends_at_the_end_of_the_death_segment() {
    let a = person("played_by = \"p1\"\ndies = \"two\"");
    let checked = run(&[("people/a.toml", &a)]);
    let g = checked.game.unwrap();
    assert_eq!(
        time::life(&g, &id("a"), person_of(&g, "a")),
        Some(span(0, 8))
    );
}

#[test]
fn life_starts_at_enters_and_runs_to_the_end_without_death() {
    let checked = run(&[("people/h.toml", &person("enters = \"two\""))]);
    let g = checked.game.unwrap();
    assert_eq!(
        time::life(&g, &id("h"), person_of(&g, "h")),
        Some(span(4, 12))
    );
}

#[test]
fn life_with_step_death_but_no_cycle_is_none() {
    let checked = run(&[("people/n.toml", &person("dies = 2"))]);
    let g = checked.draft.unwrap();
    assert_eq!(time::life(&g, &id("n"), person_of(&g, "n")), None);
}

#[test]
fn phase_by_step_starts_at_the_step_of_the_hero_cycle() {
    let h = person("dies = 4\n\n[[person.phase]]\nstep = 3\n\n[[person.phase]]\nstep = 2");
    let g = game(&[("people/h.toml", &h)]);
    let p = person_of(&g, "h");
    assert_eq!(time::phase(&g, &id("h"), &p.phases[0]), Some(8));
    assert_eq!(time::phase(&g, &id("h"), &p.phases[1]), Some(6));
}

#[test]
fn phase_by_segment_starts_at_the_segment() {
    let h = person("dies = 4\n\n[[person.phase]]\nsegment = \"three\"");
    let g = game(&[("people/h.toml", &h)]);
    let p = person_of(&g, "h");
    assert_eq!(time::phase(&g, &id("h"), &p.phases[0]), Some(8));
}

#[test]
fn hero_cycle_finds_the_cycle_of_the_hero_only() {
    let g = game(&[]);
    assert_eq!(
        time::hero_cycle(&g, &id("h")).map(|c| c.name.as_str()),
        Some("Řehořův cyklus")
    );
    assert!(time::hero_cycle(&g, &id("b")).is_none());
}

// ---------- validation ----------

#[test]
fn cycle_action_without_step_is_an_error() {
    let act = "[action]\ntitle = \"X\"\ncycle = \"long\"\n";
    let checked = run(&[("actions/x.toml", act)]);
    assert!(
        has(&checked, Severity::Error, "cycle action has no `step`"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn action_with_segment_and_cycle_is_an_error() {
    let act = "[action]\ntitle = \"X\"\nsegment = \"two\"\ncycle = \"long\"\nstep = 1\n";
    let checked = run(&[("actions/x.toml", act)]);
    assert!(
        has(&checked, Severity::Error, "not both"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn step_past_the_last_is_an_error() {
    let act = "[action]\ntitle = \"X\"\ncycle = \"long\"\nstep = 5\n";
    let checked = run(&[("actions/x.toml", act)]);
    assert!(
        has(
            &checked,
            Severity::Error,
            "step 5 does not exist; the game has 4 steps"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn step_zero_is_an_error() {
    let act = "[action]\ntitle = \"X\"\ncycle = \"long\"\nstep = 0\n";
    let checked = run(&[("actions/x.toml", act)]);
    assert!(
        has(&checked, Severity::Error, "step 0 does not exist"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn first_and_last_step_are_valid() {
    let first = "[action]\ntitle = \"X\"\ncycle = \"long\"\nstep = 1\nowners = [\"h\"]\n";
    let last = "[action]\ntitle = \"Y\"\ncycle = \"long\"\nstep = 4\nowners = [\"h\"]\n";
    let checked = run(&[("actions/x.toml", first), ("actions/y.toml", last)]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn cycles_without_steps_are_an_error() {
    let checked = run(&[("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n")]);
    assert!(
        has(
            &checked,
            Severity::Error,
            "the game has cycles but no `steps`"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn step_death_of_a_non_hero_is_an_error() {
    let n = person("dies = 2");
    let checked = run(&[("people/n.toml", &n)]);
    assert!(
        has(
            &checked,
            Severity::Error,
            "dies at step 2, but is not the hero of any cycle"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn step_death_past_the_last_step_is_an_error() {
    let checked = run(&[("people/h.toml", &person("dies = 9"))]);
    assert!(
        has(&checked, Severity::Error, "step 9 does not exist"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn hero_who_never_dies_is_a_warning() {
    let checked = run(&[("people/h.toml", &person(""))]);
    assert!(
        has(
            &checked,
            Severity::Warning,
            "hero of “Řehořův cyklus” does not die in it"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn hero_dying_in_a_segment_is_a_warning() {
    let checked = run(&[("people/h.toml", &person("dies = \"three\""))]);
    assert!(
        has(
            &checked,
            Severity::Warning,
            "hero of “Řehořův cyklus” does not die in it"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn hero_of_several_cycles_is_a_warning() {
    let cycles = format!(
        "{CYCLES}\n[[cycle]]\nid = \"again\"\nname = \"Znovu\"\nhero = \"h\"\nfrom = \"three\"\n"
    );
    let checked = run(&[("cycles.toml", &cycles)]);
    assert!(
        has(
            &checked,
            Severity::Warning,
            "hero of several cycles: Řehořův cyklus, Znovu"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn player_who_is_never_a_hero_is_a_warning() {
    let players = format!("{PLAYERS}\n[[player]]\nid = \"p2\"\nname = \"Šárka\"\n");
    let checked = run(&[
        ("players.toml", &players),
        ("people/h.toml", &person("played_by = \"p2\"\ndies = 4")),
        (
            "people/c.toml",
            &person("played_by = \"p2\"\nafter = \"h\""),
        ),
    ]);
    assert!(
        !has(&checked, Severity::Warning, "Šárka is never the hero"),
        "p2 plays the hero h: {:?}",
        checked.diagnostics
    );
    let checked = run(&[
        ("players.toml", &players),
        ("people/c.toml", &person("played_by = \"p2\"")),
    ]);
    assert!(
        has(
            &checked,
            Severity::Warning,
            "Šárka is never the hero of a cycle"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn player_who_is_never_a_hero_is_fine_without_cycles() {
    let checked = run(&[
        ("cycles.toml", ""),
        ("people/a.toml", &person("played_by = \"p1\"")),
        ("people/b.toml", &person("")),
        ("people/h.toml", &person("")),
    ]);
    assert!(
        !has(&checked, Severity::Warning, "never the hero"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn outside_cycle_on_a_hero_is_a_warning() {
    let checked = run(&[("people/h.toml", &person("dies = 4\noutside_cycle = true"))]);
    assert!(
        has(
            &checked,
            Severity::Warning,
            "is marked `outside_cycle` but is the hero of “Řehořův cyklus”"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn enters_together_with_after_is_an_error() {
    let b = person("played_by = \"p1\"\nafter = \"a\"\nenters = \"three\"");
    let checked = run(&[("people/b.toml", &b)]);
    assert!(
        has(
            &checked,
            Severity::Error,
            "use either `enters` or `after`, not both"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn after_someone_who_never_dies_is_a_warning() {
    let checked = run(&[
        ("people/n.toml", &person("")),
        ("people/m.toml", &person("after = \"n\"")),
    ]);
    assert!(
        has(&checked, Severity::Warning, "who never dies"),
        "{:?}",
        checked.diagnostics
    );
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn cycle_ending_before_it_starts_is_an_error() {
    let cycles = CYCLES.replace(
        "from = \"two\"\nto = \"three\"",
        "from = \"three\"\nto = \"one\"",
    );
    let checked = run(&[("cycles.toml", &cycles)]);
    assert!(
        has(
            &checked,
            Severity::Error,
            "cycle “Řehořův cyklus” ends before it starts"
        ),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn cycle_within_one_segment_with_to_equal_from_is_valid() {
    let cycles = CYCLES.replace(
        "from = \"two\"\nto = \"three\"",
        "from = \"three\"\nto = \"three\"",
    );
    let checked = run(&[("cycles.toml", &cycles)]);
    assert!(
        !has(&checked, Severity::Error, "ends before it starts"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn unknown_cycle_on_an_action_is_a_reference_error() {
    let act = "[action]\ntitle = \"X\"\ncycle = \"nope\"\nstep = 1\n";
    let checked = run(&[("actions/x.toml", act)]);
    let errors: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error && d.message.contains("unknown cycle `nope`"))
        .collect();
    assert_eq!(errors.len(), 1, "{:?}", checked.diagnostics);
    assert_eq!(errors[0].file, "actions/x.toml");
    assert!(checked.game.is_none());
}

#[test]
fn phase_step_out_of_range_is_an_error() {
    for step in [0, 9] {
        let h = person(&format!("dies = 4\n\n[[person.phase]]\nstep = {step}"));
        let checked = run(&[("people/h.toml", &h)]);
        assert!(
            has(
                &checked,
                Severity::Error,
                &format!("step {step} does not exist")
            ),
            "{:?}",
            checked.diagnostics
        );
    }
}

#[test]
fn step_without_cycle_is_an_error() {
    let act = "[action]\ntitle = \"X\"\nsegment = \"two\"\nstep = 200\n";
    let checked = run(&[("actions/x.toml", act)]);
    assert!(
        has(&checked, Severity::Error, "`step` needs `cycle`"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn spans_that_only_touch_do_not_overlap() {
    assert!(!span(0, 4).overlaps(span(4, 8)));
    assert!(!span(4, 8).overlaps(span(0, 4)));
    assert!(span(0, 5).overlaps(span(4, 8)));
    assert!(span(4, 8).overlaps(span(0, 5)));
}

#[test]
fn cycle_ending_in_the_segment_right_before_it_starts_has_no_span() {
    // "two" starts at tick 4 and "one" ends at tick 4: an empty, backwards cycle.
    let cycles = CYCLES.replace(
        "from = \"two\"\nto = \"three\"",
        "from = \"two\"\nto = \"one\"",
    );
    let checked = run(&[("cycles.toml", &cycles)]);
    let draft = checked.draft.unwrap();
    assert_eq!(time::cycle(&draft, cycle(&draft, "long")), None);
    assert_eq!(
        time::cycle(&draft, cycle(&draft, "short")),
        Some(span(4, 8))
    );
}
