//! Counters: `counters.toml`, `change` and `require` on actions and options, `Bound::allows`,
//! the checks in `validate/counters.rs` and how the playthrough simulation follows counters.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_model::model::{Bound, Node};
use larp_model::{Checked, Diagnostic, Game, Id, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";

/// One counter `suspicion` ("Podezření") with default range 0..=10.
const SUSPICION: &str = "[[counter]]\nid = \"suspicion\"\nname = \"Podezření\"\n";

/// An action that only changes `suspicion`, so the counter never warns about being unchanged.
const RAISE: &str = "[action]\ntitle = \"Zvednout\"\nchange = { suspicion = 1 }\n";

const KEY: &str = "[[item]]\nid = \"key\"\nname = \"Klíč\"\nstarting = true\n";

fn ending(id: &str) -> String {
    format!("[[info]]\nid = \"{id}\"\nname = \"{id}\"\nending = true\n")
}

fn info(id: &str) -> String {
    format!("[[info]]\nid = \"{id}\"\nname = \"{id}\"\n")
}

/// Loads `game.toml` plus `files`.
fn run(files: &[(&str, &str)]) -> Checked {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    load(&files)
}

fn game(files: &[(&str, &str)]) -> Game {
    let checked = run(files);
    let diagnostics = format!("{:?}", checked.diagnostics);
    checked
        .game
        .unwrap_or_else(|| panic!("game did not load: {diagnostics}"))
}

fn messages(checked: &Checked, severity: Severity) -> Vec<&str> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message.as_str())
        .collect()
}

/// The only diagnostic with exactly this message.
fn diag<'a>(checked: &'a Checked, message: &str) -> &'a Diagnostic {
    let found: Vec<&Diagnostic> = checked
        .diagnostics
        .iter()
        .filter(|d| d.message == message)
        .collect();
    assert_eq!(found.len(), 1, "{message:?} in {:#?}", checked.diagnostics);
    found[0]
}

/// The source text a diagnostic points at.
fn pointed<'a>(files: &[(&str, &'a str)], d: &Diagnostic) -> &'a str {
    let text = files
        .iter()
        .find(|(p, _)| *p == d.file)
        .unwrap_or_else(|| panic!("no file {}", d.file))
        .1;
    &text[d.span.clone().unwrap_or_else(|| panic!("no span: {d:?}"))]
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

fn action<'a>(g: &'a Game, node: &str) -> &'a larp_model::model::Action {
    match &g.nodes[&id(node)] {
        Node::Action(a) => a,
        Node::Decision(_) => panic!("{node} is a decision"),
    }
}

/// Changes as (counter, amount), in key order.
fn changes(c: &larp_model::model::Changes) -> Vec<(&str, i32)> {
    c.iter()
        .map(|(k, v)| (k.get_ref().as_str(), *v.get_ref()))
        .collect()
}

/// Requirements as (counter, at least, at most), in key order.
fn requires(r: &larp_model::model::Requires) -> Vec<(&str, Option<i32>, Option<i32>)> {
    r.iter()
        .map(|(k, b)| (k.get_ref().as_str(), b.at_least, b.at_most))
        .collect()
}

// ---------- loading ----------

#[test]
fn loads_a_counter_with_every_key() {
    let g = game(&[
        (
            "counters.toml",
            "[[counter]]\nid = \"suspicion\"\nname = \"Hugovo podezření\"\nstart = 2\nmin = -3\nmax = 5\ntext = \"Jak moc Hugo tuší, že Anselm lže.\"\n",
        ),
        ("actions/raise.toml", RAISE),
    ]);
    assert_eq!(g.counters.len(), 1);
    let c = &g.counters[0];
    assert_eq!(c.id.get_ref().as_str(), "suspicion");
    assert_eq!(c.name, "Hugovo podezření");
    assert_eq!((c.start(), c.min(), c.max()), (2, -3, 5));
    assert_eq!(c.text, "Jak moc Hugo tuší, že Anselm lže.");
}

#[test]
fn counter_defaults_to_start_0_min_0_max_10() {
    let g = game(&[("counters.toml", SUSPICION), ("actions/raise.toml", RAISE)]);
    let c = &g.counters[0];
    assert!(c.start.is_none() && c.min.is_none() && c.max.is_none());
    assert_eq!((c.start(), c.min(), c.max()), (0, 0, 10));
    assert!(c.text.is_empty(), "{:?}", c.text);
}

#[test]
fn loads_counters_in_file_order() {
    let g = game(&[
        (
            "counters.toml",
            "[[counter]]\nid = \"z-money\"\nname = \"Peníze\"\n\n[[counter]]\nid = \"a-hour\"\nname = \"Hodina\"\n",
        ),
        (
            "actions/raise.toml",
            "[action]\ntitle = \"x\"\nchange = { z-money = 1, a-hour = 1 }\n",
        ),
    ]);
    let ids: Vec<&str> = g.counters.iter().map(|c| c.id.get_ref().as_str()).collect();
    assert_eq!(ids, ["z-money", "a-hour"]);
}

#[test]
fn game_without_counters_toml_has_no_counters() {
    let g = game(&[]);
    assert!(g.counters.is_empty(), "{:?}", g.counters);
}

#[test]
fn empty_counters_toml_is_valid() {
    let checked = run(&[("counters.toml", "")]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let counters = checked.game.unwrap().counters;
    assert!(counters.is_empty(), "{counters:?}");
}

#[test]
fn unknown_key_in_a_counter_is_an_error() {
    let checked = run(&[("counters.toml", &format!("{SUSPICION}maximum = 5\n"))]);
    assert!(checked.game.is_none());
    let errors = messages(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("maximum"), "{errors:?}");
}

#[test]
fn counter_without_a_name_is_an_error() {
    let checked = run(&[("counters.toml", "[[counter]]\nid = \"suspicion\"\n")]);
    assert!(checked.game.is_none());
    assert!(messages(&checked, Severity::Error)[0].contains("name"));
}

#[test]
fn counter_with_an_invalid_id_is_an_error() {
    let checked = run(&[(
        "counters.toml",
        "[[counter]]\nid = \"Podezření\"\nname = \"x\"\n",
    )]);
    assert!(checked.game.is_none());
    assert_eq!(messages(&checked, Severity::Error).len(), 1);
}

#[test]
fn non_integer_counter_start_is_an_error() {
    let checked = run(&[("counters.toml", &format!("{SUSPICION}start = 1.5\n"))]);
    assert!(checked.game.is_none());
    assert_eq!(messages(&checked, Severity::Error).len(), 1);
}

#[test]
fn loads_change_and_require_on_an_action() {
    let g = game(&[
        (
            "counters.toml",
            "[[counter]]\nid = \"suspicion\"\nname = \"Podezření\"\n\n[[counter]]\nid = \"money\"\nname = \"Peníze\"\nmin = -10\n",
        ),
        (
            "actions/act.toml",
            "[action]\ntitle = \"Úplatek\"\nchange = { suspicion = 2, money = -3 }\nrequire = { suspicion = { at_least = 1 }, money = { at_least = 1, at_most = 4 } }\n",
        ),
    ]);
    let a = action(&g, "act");
    assert_eq!(changes(&a.change), [("money", -3), ("suspicion", 2)]);
    assert_eq!(
        requires(&a.require),
        [("money", Some(1), Some(4)), ("suspicion", Some(1), None)]
    );
}

#[test]
fn action_without_change_or_require_has_empty_maps() {
    let g = game(&[("actions/act.toml", "[action]\ntitle = \"x\"\n")]);
    let a = action(&g, "act");
    assert!(a.change.is_empty(), "{:?}", a.change);
    assert!(a.require.is_empty(), "{:?}", a.require);
}

#[test]
fn loads_change_and_require_on_a_decision_option() {
    let g = game(&[
        ("counters.toml", SUSPICION),
        (
            "actions/verdict.toml",
            "[decision]\ntitle = \"Verdikt\"\n\n[[decision.option]]\nlabel = \"Obvinit\"\nrequire = { suspicion = { at_most = 7 } }\n\n[[decision.option]]\nlabel = \"Odejít\"\nchange = { suspicion = -2 }\n",
        ),
    ]);
    let Node::Decision(d) = &g.nodes[&id("verdict")] else {
        panic!("not a decision");
    };
    assert_eq!(
        requires(&d.options[0].require),
        [("suspicion", None, Some(7))]
    );
    assert!(d.options[0].change.is_empty(), "{:?}", d.options[0].change);
    assert_eq!(changes(&d.options[1].change), [("suspicion", -2)]);
    assert!(
        d.options[1].require.is_empty(),
        "{:?}",
        d.options[1].require
    );
}

#[test]
fn change_key_span_points_at_the_counter_id() {
    let text = "[action]\ntitle = \"x\"\nchange = { suspicion = 2 }\n";
    let g = game(&[("counters.toml", SUSPICION), ("actions/act.toml", text)]);
    let (key, value) = action(&g, "act").change.iter().next().unwrap();
    assert_eq!(&text[key.span()], "suspicion");
    assert_eq!(&text[value.span()], "2");
}

#[test]
fn unknown_key_in_a_requirement_is_an_error() {
    let checked = run(&[
        ("counters.toml", SUSPICION),
        (
            "actions/act.toml",
            "[action]\ntitle = \"x\"\nrequire = { suspicion = { atleast = 3 } }\n",
        ),
    ]);
    assert!(checked.game.is_none());
    let errors = messages(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("atleast"), "{errors:?}");
}

#[test]
fn non_integer_change_is_an_error() {
    for value in ["1.5", "\"2\"", "true", "{ at_least = 1 }"] {
        let checked = run(&[
            ("counters.toml", SUSPICION),
            (
                "actions/act.toml",
                &format!("[action]\ntitle = \"x\"\nchange = {{ suspicion = {value} }}\n"),
            ),
        ]);
        assert!(checked.game.is_none(), "{value} was accepted");
        assert_eq!(messages(&checked, Severity::Error).len(), 1, "{value}");
    }
}

#[test]
fn non_integer_requirement_bound_is_an_error() {
    for bound in ["at_least = 2.5", "at_most = \"3\"", "at_least = true"] {
        let checked = run(&[
            ("counters.toml", SUSPICION),
            (
                "actions/act.toml",
                &format!("[action]\ntitle = \"x\"\nrequire = {{ suspicion = {{ {bound} }} }}\n"),
            ),
        ]);
        assert!(checked.game.is_none(), "{bound} was accepted");
        assert_eq!(messages(&checked, Severity::Error).len(), 1, "{bound}");
    }
}

#[test]
fn requirement_that_is_a_plain_number_is_an_error() {
    let checked = run(&[
        ("counters.toml", SUSPICION),
        (
            "actions/act.toml",
            "[action]\ntitle = \"x\"\nrequire = { suspicion = 3 }\n",
        ),
    ]);
    assert!(checked.game.is_none());
}

#[test]
fn change_too_large_for_i32_is_an_error() {
    let checked = run(&[
        ("counters.toml", SUSPICION),
        (
            "actions/act.toml",
            "[action]\ntitle = \"x\"\nchange = { suspicion = 3000000000 }\n",
        ),
    ]);
    assert!(checked.game.is_none());
}

// ---------- Bound::allows ----------

fn bound(at_least: Option<i32>, at_most: Option<i32>) -> Bound {
    Bound { at_least, at_most }
}

#[test]
fn bound_at_least_is_inclusive() {
    let b = bound(Some(3), None);
    assert!(!b.allows(2));
    assert!(b.allows(3));
    assert!(b.allows(i32::MAX));
}

#[test]
fn bound_at_most_is_inclusive() {
    let b = bound(None, Some(1));
    assert!(b.allows(i32::MIN));
    assert!(b.allows(1));
    assert!(!b.allows(2));
}

#[test]
fn bound_with_both_allows_only_the_closed_range() {
    let b = bound(Some(1), Some(4));
    assert!(!b.allows(0));
    assert!(b.allows(1));
    assert!(b.allows(4));
    assert!(!b.allows(5));
}

#[test]
fn bound_with_equal_ends_allows_exactly_one_value() {
    let b = bound(Some(-2), Some(-2));
    assert!(!b.allows(-3));
    assert!(b.allows(-2));
    assert!(!b.allows(-1));
}

#[test]
fn bound_without_ends_allows_everything() {
    let b = bound(None, None);
    assert!(b.allows(i32::MIN) && b.allows(0) && b.allows(i32::MAX));
}

#[test]
fn bound_with_crossed_ends_allows_nothing() {
    let b = bound(Some(4), Some(2));
    assert!((-1..=6).all(|v| !b.allows(v)));
}

// ---------- validation: references ----------

#[test]
fn unknown_counter_in_change_is_an_error_with_a_suggestion() {
    let act = "[action]\ntitle = \"x\"\nchange = { suspicon = 1 }\n";
    let files = [("counters.toml", SUSPICION), ("actions/act.toml", act)];
    let checked = run(&files);
    let d = diag(
        &checked,
        "unknown `change` key `suspicon`, expected a counter",
    );
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "actions/act.toml");
    assert_eq!(pointed(&files, d), "suspicon");
    assert_eq!(d.help.as_deref(), Some("did you mean `suspicion`?"));
}

#[test]
fn unknown_counter_in_require_is_an_error() {
    let act = "[action]\ntitle = \"x\"\nrequire = { suspicon = { at_least = 1 } }\n";
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", act),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "unknown `require` key `suspicon`, expected a counter",
    );
    assert_eq!(d.file, "actions/act.toml");
    assert_eq!(pointed(&files, d), "suspicon");
    assert_eq!(d.help.as_deref(), Some("did you mean `suspicion`?"));
}

#[test]
fn unknown_counter_in_an_option_is_an_error() {
    let dec = "[decision]\ntitle = \"D\"\n\n[[decision.option]]\nlabel = \"A\"\nchange = { suspicion = 1 }\n\n[[decision.option]]\nlabel = \"B\"\nrequire = { suspicon = { at_most = 2 } }\n";
    let files = [("counters.toml", SUSPICION), ("actions/dec.toml", dec)];
    let checked = run(&files);
    let d = diag(
        &checked,
        "unknown `require` key `suspicon`, expected a counter",
    );
    assert_eq!(d.file, "actions/dec.toml");
    assert_eq!(pointed(&files, d), "suspicon");
}

#[test]
fn unknown_counter_far_from_any_counter_has_no_suggestion() {
    let act = "[action]\ntitle = \"x\"\nchange = { dragon = 1 }\n";
    let files = [("counters.toml", SUSPICION), ("actions/act.toml", act)];
    let checked = run(&files);
    let d = diag(
        &checked,
        "unknown `change` key `dragon`, expected a counter",
    );
    assert!(d.help.is_none(), "{d:?}");
}

#[test]
fn suggestion_only_names_counters_not_other_kinds() {
    // `suspicio` is close to the item `suspicio-n`, but that is not a counter.
    let act = "[action]\ntitle = \"x\"\nchange = { suspicio = 1 }\n";
    let files = [
        (
            "items.toml",
            "[[item]]\nid = \"suspicio-n\"\nname = \"x\"\n",
        ),
        ("actions/act.toml", act),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "unknown `change` key `suspicio`, expected a counter",
    );
    assert!(d.help.is_none(), "{d:?}");
}

#[test]
fn counter_used_as_a_need_is_an_error() {
    let act = "[action]\ntitle = \"x\"\nneeds = [\"suspicion\"]\nchange = { suspicion = 1 }\n";
    let files = [("counters.toml", SUSPICION), ("actions/act.toml", act)];
    let checked = run(&files);
    let d = diag(
        &checked,
        "`suspicion` is a counter, but a need must be an item or an info",
    );
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "actions/act.toml");
    assert_eq!(pointed(&files, d), "\"suspicion\"");
}

#[test]
fn counter_used_as_a_gift_is_an_error() {
    let act = "[action]\ntitle = \"x\"\ngives = [\"suspicion\"]\nchange = { suspicion = 1 }\n";
    let files = [("counters.toml", SUSPICION), ("actions/act.toml", act)];
    let checked = run(&files);
    diag(
        &checked,
        "`suspicion` is a counter, but a gift must be an item or an info",
    );
}

#[test]
fn item_used_as_a_counter_is_an_error() {
    let act = "[action]\ntitle = \"x\"\nchange = { key = 1 }\n";
    let files = [("items.toml", KEY), ("actions/act.toml", act)];
    let checked = run(&files);
    let d = diag(
        &checked,
        "`key` is an item, but a `change` key must be a counter",
    );
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "actions/act.toml");
    assert_eq!(pointed(&files, d), "key");
}

#[test]
fn action_used_as_a_required_counter_is_an_error() {
    let act = "[action]\ntitle = \"x\"\nrequire = { other = { at_least = 1 } }\n";
    let files = [
        ("actions/other.toml", "[action]\ntitle = \"y\"\n"),
        ("actions/act.toml", act),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "`other` is an action, but a `require` key must be a counter",
    );
    assert_eq!(pointed(&files, d), "other");
}

#[test]
fn counter_id_shared_with_an_item_is_an_error_at_the_counter() {
    let counters = "[[counter]]\nid = \"key\"\nname = \"Klíčovost\"\n";
    let files = [
        ("items.toml", KEY),
        ("counters.toml", counters),
        (
            "actions/act.toml",
            "[action]\ntitle = \"x\"\nchange = { key = 1 }\n",
        ),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "id `key` is already used by an item in items.toml",
    );
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "counters.toml");
    assert_eq!(pointed(&files, d), "\"key\"");
    assert!(d.help.as_deref().unwrap().contains("counters"), "{d:?}");
}

#[test]
fn counter_id_shared_with_an_action_is_an_error() {
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/suspicion.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "id `suspicion` is already used by an action in actions/suspicion.toml",
    );
    assert_eq!(d.file, "counters.toml");
    assert_eq!(pointed(&files, d), "\"suspicion\"");
}

#[test]
fn block_id_shared_with_a_counter_is_an_error_at_the_block() {
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        (
            "blocks.toml",
            "[[block]]\nid = \"suspicion\"\ntext = \"Podezření roste.\"\n",
        ),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "id `suspicion` is already used by a counter in counters.toml",
    );
    assert_eq!(d.file, "blocks.toml");
    assert_eq!(pointed(&files, d), "\"suspicion\"");
}

#[test]
fn two_counters_with_one_id_are_an_error_at_the_second() {
    let counters = "[[counter]]\nid = \"suspicion\"\nname = \"A\"\n\n[[counter]]\nid = \"suspicion\"\nname = \"B\"\n";
    let files = [("counters.toml", counters), ("actions/raise.toml", RAISE)];
    let checked = run(&files);
    let d = diag(
        &checked,
        "id `suspicion` is already used by a counter in counters.toml",
    );
    let second = counters.rfind("\"suspicion\"").unwrap();
    assert_eq!(d.span.clone().unwrap().start, second);
}

// ---------- validation: ranges ----------

#[test]
fn min_above_max_is_an_error_at_max() {
    let counters = format!("{SUSPICION}min = 5\nmax = 3\n");
    let files = [
        ("counters.toml", counters.as_str()),
        ("actions/raise.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(&checked, "counter `suspicion` has min 5 above max 3");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "counters.toml");
    assert_eq!(pointed(&files, d), "3");
    assert_eq!(d.help.as_deref(), Some("min defaults to 0 and max to 10"));
}

#[test]
fn min_above_default_max_is_an_error_at_min() {
    let counters = format!("{SUSPICION}min = 11\nstart = 11\n");
    let files = [
        ("counters.toml", counters.as_str()),
        ("actions/raise.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(&checked, "counter `suspicion` has min 11 above max 10");
    assert_eq!(pointed(&files, d), "11");
    assert_eq!(d.span.clone().unwrap().start, counters.find("11").unwrap());
}

#[test]
fn max_below_default_min_is_an_error() {
    let counters = format!("{SUSPICION}max = -1\n");
    let files = [
        ("counters.toml", counters.as_str()),
        ("actions/raise.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(&checked, "counter `suspicion` has min 0 above max -1");
    assert_eq!(pointed(&files, d), "-1");
}

#[test]
fn min_equal_to_max_is_valid() {
    let counters = format!("{SUSPICION}min = 3\nmax = 3\nstart = 3\n");
    let checked = run(&[("counters.toml", &counters), ("actions/raise.toml", RAISE)]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn start_above_max_is_an_error_at_start() {
    let counters = format!("{SUSPICION}start = 11\n");
    let files = [
        ("counters.toml", counters.as_str()),
        ("actions/raise.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "counter `suspicion` starts at 11, outside its range 0 to 10",
    );
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(pointed(&files, d), "11");
    assert_eq!(
        d.help.as_deref(),
        Some("start defaults to 0, min to 0 and max to 10")
    );
}

#[test]
fn start_below_min_is_an_error() {
    let counters = format!("{SUSPICION}min = -5\nmax = 5\nstart = -6\n");
    let files = [
        ("counters.toml", counters.as_str()),
        ("actions/raise.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "counter `suspicion` starts at -6, outside its range -5 to 5",
    );
    assert_eq!(pointed(&files, d), "-6");
}

#[test]
fn default_start_below_min_is_an_error_at_the_id() {
    let counters = format!("{SUSPICION}min = 2\n");
    let files = [
        ("counters.toml", counters.as_str()),
        ("actions/raise.toml", RAISE),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "counter `suspicion` starts at 0, outside its range 2 to 10",
    );
    assert_eq!(pointed(&files, d), "\"suspicion\"");
}

#[test]
fn start_at_either_end_of_the_range_is_valid() {
    for start in ["-2", "4"] {
        let counters = format!("{SUSPICION}min = -2\nmax = 4\nstart = {start}\n");
        let checked = run(&[("counters.toml", &counters), ("actions/raise.toml", RAISE)]);
        assert!(
            checked.diagnostics.is_empty(),
            "start {start}: {:?}",
            checked.diagnostics
        );
    }
}

#[test]
fn min_above_max_is_reported_once_without_a_start_error() {
    let counters = format!("{SUSPICION}min = 5\nmax = 3\nstart = 9\n");
    let checked = run(&[("counters.toml", &counters), ("actions/raise.toml", RAISE)]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["counter `suspicion` has min 5 above max 3"]
    );
}

// ---------- validation: requirements ----------

fn requiring(bound: &str) -> String {
    format!("[action]\ntitle = \"x\"\nrequire = {{ suspicion = {bound} }}\n")
}

#[test]
fn requirement_without_bounds_is_an_error() {
    let act = requiring("{}");
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", act.as_str()),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "requirement on `suspicion` has no `at_least` or `at_most`",
    );
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "actions/act.toml");
    assert_eq!(pointed(&files, d), "suspicion");
    assert_eq!(
        d.help.as_deref(),
        Some("write it as `{ at_least = 3 }`, `{ at_most = 1 }` or both")
    );
}

#[test]
fn requirement_with_at_least_above_at_most_is_an_error() {
    let act = requiring("{ at_least = 4, at_most = 2 }");
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", act.as_str()),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "requirement on `suspicion` asks for at least 4 and at most 2",
    );
    assert_eq!(pointed(&files, d), "suspicion");
    assert_eq!(
        d.help.as_deref(),
        Some("no value can be both; swap them or drop one")
    );
}

#[test]
fn requirement_with_equal_bounds_is_valid() {
    let act = requiring("{ at_least = 2, at_most = 2 }");
    let checked = run(&[
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", &act),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

const NEVER_MET: &str =
    "requirement on `suspicion` can never be met: the counter stays between 0 and 10";

#[test]
fn requirement_above_max_is_an_error() {
    let act = requiring("{ at_least = 11 }");
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", act.as_str()),
    ];
    let checked = run(&files);
    let d = diag(&checked, NEVER_MET);
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(pointed(&files, d), "suspicion");
    assert_eq!(
        d.help.as_deref(),
        Some("change the requirement, or the counter's min or max in counters.toml")
    );
}

#[test]
fn requirement_below_min_is_an_error() {
    let act = requiring("{ at_most = -1 }");
    let checked = run(&[
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", &act),
    ]);
    diag(&checked, NEVER_MET);
}

#[test]
fn requirement_range_overlapping_the_counter_range_is_valid() {
    for bound in [
        "{ at_least = 10 }",
        "{ at_most = 0 }",
        "{ at_least = -5 }",
        "{ at_most = 50 }",
        "{ at_least = 9, at_most = 20 }",
    ] {
        let act = requiring(bound);
        let checked = run(&[
            ("counters.toml", SUSPICION),
            ("actions/raise.toml", RAISE),
            ("actions/act.toml", &act),
        ]);
        assert!(
            messages(&checked, Severity::Error).is_empty(),
            "{bound}: {:?}",
            checked.diagnostics
        );
    }
}

#[test]
fn requirement_outside_a_custom_range_names_that_range() {
    let counters = format!("{SUSPICION}min = -3\nmax = 5\n");
    let act = requiring("{ at_least = 6 }");
    let checked = run(&[
        ("counters.toml", &counters),
        ("actions/raise.toml", RAISE),
        ("actions/act.toml", &act),
    ]);
    diag(
        &checked,
        "requirement on `suspicion` can never be met: the counter stays between -3 and 5",
    );
}

#[test]
fn requirement_on_an_option_is_checked_too() {
    let dec = "[decision]\ntitle = \"D\"\n\n[[decision.option]]\nlabel = \"A\"\nrequire = { suspicion = { at_least = 3, at_most = 1 } }\n";
    let files = [
        ("counters.toml", SUSPICION),
        ("actions/raise.toml", RAISE),
        ("actions/dec.toml", dec),
    ];
    let checked = run(&files);
    let d = diag(
        &checked,
        "requirement on `suspicion` asks for at least 3 and at most 1",
    );
    assert_eq!(d.file, "actions/dec.toml");
    assert_eq!(pointed(&files, d), "suspicion");
}

// ---------- validation: unused counters ----------

#[test]
fn counter_that_is_never_changed_is_a_warning() {
    let files = [("counters.toml", SUSPICION)];
    let checked = run(&files);
    assert!(checked.game.is_some());
    let d = diag(&checked, "counter `suspicion` is never changed");
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.file, "counters.toml");
    assert_eq!(pointed(&files, d), "\"suspicion\"");
    assert_eq!(
        d.help.as_deref(),
        Some("add `change = { suspicion = 1 }` to an action or option, or remove the counter")
    );
}

#[test]
fn counter_that_is_only_required_is_still_never_changed() {
    let checked = run(&[
        ("counters.toml", SUSPICION),
        ("actions/act.toml", &requiring("{ at_most = 3 }")),
    ]);
    assert!(
        messages(&checked, Severity::Warning).contains(&"counter `suspicion` is never changed"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn counter_changed_by_an_action_gets_no_warning() {
    let checked = run(&[("counters.toml", SUSPICION), ("actions/raise.toml", RAISE)]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn counter_changed_only_by_an_option_gets_no_warning() {
    let dec = "[decision]\ntitle = \"D\"\n\n[[decision.option]]\nlabel = \"A\"\nchange = { suspicion = -1 }\n";
    let checked = run(&[("counters.toml", SUSPICION), ("actions/dec.toml", dec)]);
    assert!(
        !messages(&checked, Severity::Warning).contains(&"counter `suspicion` is never changed"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn only_the_unchanged_counter_warns() {
    let counters = format!("{SUSPICION}\n[[counter]]\nid = \"penize\"\nname = \"Peníze\"\n");
    let checked = run(&[("counters.toml", &counters), ("actions/raise.toml", RAISE)]);
    assert_eq!(
        messages(&checked, Severity::Warning),
        ["counter `penize` is never changed"]
    );
}

// ---------- simulation ----------

/// A counter `c` with the given extra keys.
fn counter(extra: &str) -> String {
    format!("[[counter]]\nid = \"c\"\nname = \"Počet\"\n{extra}")
}

fn act(title: &str, rest: &str) -> String {
    format!("[action]\ntitle = \"{title}\"\n{rest}")
}

fn reached(checked: &Checked) -> Vec<&str> {
    checked
        .playthroughs
        .as_ref()
        .unwrap_or_else(|| panic!("not simulated: {:?}", checked.diagnostics))
        .endings
        .keys()
        .map(Id::as_str)
        .collect()
}

fn route<'a>(checked: &'a Checked, end: &str) -> &'a [String] {
    &checked.playthroughs.as_ref().unwrap().endings[&id(end)]
}

/// `n` independent actions that each add 1 to `c`, and `solve`, which needs `c` at least 3.
fn gated(n: usize) -> Checked {
    let adders: Vec<(String, String)> = (0..n)
        .map(|i| {
            (
                format!("actions/add-{i}.toml"),
                act(&format!("Stopa {i}"), "change = { c = 1 }\n"),
            )
        })
        .collect();
    let counters = counter("");
    let win = ending("win");
    let solve = act(
        "Vyřešit",
        "require = { c = { at_least = 3 } }\ngives = [\"win\"]\n",
    );
    let mut files: Vec<(&str, &str)> = vec![
        ("counters.toml", &counters),
        ("info.toml", &win),
        ("actions/solve.toml", &solve),
    ];
    files.extend(adders.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    run(&files)
}

#[test]
fn requirement_is_met_once_enough_changes_happened() {
    let checked = gated(3);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["win"]);
}

#[test]
fn requirement_gates_an_action_until_enough_changes_happened() {
    let checked = gated(2);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["ending `win` cannot be reached in any playthrough"]
    );
    assert!(
        messages(&checked, Severity::Warning)
            .contains(&"“Vyřešit” can never happen in any playthrough"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn never_happening_action_help_mentions_counters() {
    let checked = gated(2);
    let d = diag(&checked, "“Vyřešit” can never happen in any playthrough");
    assert!(d.help.as_deref().unwrap().contains("counter"), "{d:?}");
}

/// `alarm` adds `amount` and gives `noise`; `sneak` needs `noise` and `c` at most 1, so it is
/// ready only after the alarm, whatever the order of anything else.
fn sneak_after_alarm(amount: i32) -> Checked {
    let counters = counter("");
    let infos = format!("{}\n{}", info("noise"), ending("win"));
    let alarm = act(
        "Poplach",
        &format!("change = {{ c = {amount} }}\ngives = [\"noise\"]\n"),
    );
    let sneak = act(
        "Proplížit se",
        "needs = [\"noise\"]\nrequire = { c = { at_most = 1 } }\ngives = [\"win\"]\n",
    );
    run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        ("actions/alarm.toml", &alarm),
        ("actions/sneak.toml", &sneak),
    ])
}

#[test]
fn at_most_closes_an_action_once_the_counter_rose() {
    let checked = sneak_after_alarm(2);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["ending `win` cannot be reached in any playthrough"]
    );
    assert!(
        messages(&checked, Severity::Warning)
            .contains(&"“Proplížit se” can never happen in any playthrough"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn at_most_allows_an_action_while_the_counter_is_low_enough() {
    let checked = sneak_after_alarm(1);
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}

#[test]
fn action_that_leads_to_a_competitor_is_ordered_both_ways() {
    // `find` and `alarm` do not compete, but `find` makes `sneak` ready, and `sneak` competes
    // with `alarm`. Finding first lets the players sneak before the alarm.
    let counters = counter("");
    let infos = format!("{}\n{}", info("clue"), ending("win"));
    let alarm = act("Poplach", "change = { c = 2 }\n");
    let find = act("Najít", "gives = [\"clue\"]\n");
    let sneak = act(
        "Proplížit se",
        "needs = [\"clue\"]\nrequire = { c = { at_most = 1 } }\ngives = [\"win\"]\n",
    );
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        ("actions/alarm.toml", &alarm),
        ("actions/find.toml", &find),
        ("actions/sneak.toml", &sneak),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["win"]);
    assert_eq!(route(&checked, "win"), ["Najít", "Proplížit se"]);
}

#[test]
fn counter_change_that_opens_a_competitor_is_ordered_both_ways() {
    // `raise` makes `wait` ready, and `wait` competes with `grab` for the token. So `raise` and
    // `grab` are a choice of order, although they share nothing directly.
    let token = "[[item]]\nid = \"token\"\nname = \"Žeton\"\nstarting = true\n";
    let infos = format!("{}\n{}", ending("end-wait"), ending("end-grab"));
    let checked = run(&[
        ("counters.toml", &counter("")),
        ("items.toml", token),
        ("info.toml", &infos),
        ("actions/raise.toml", &act("Zvýšit", "change = { c = 1 }\n")),
        (
            "actions/wait.toml",
            &act(
                "Počkat",
                "require = { c = { at_least = 1 } }\nuses = [\"token\"]\ngives = [\"end-wait\"]\n",
            ),
        ),
        (
            "actions/grab.toml",
            &act("Chňapnout", "uses = [\"token\"]\ngives = [\"end-grab\"]\n"),
        ),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["end-grab", "end-wait"]);
}

#[test]
fn counter_chain_through_another_action_is_ordered_both_ways() {
    // A raises c; C needs c ≥ 1 and raises d; B and F share the token and need d ≤ 0 / d ≥ 1.
    // A does not compete with B directly, but A leads to C, which competes with B.
    let counters =
        "[[counter]]\nid = \"c\"\nname = \"C\"\n\n[[counter]]\nid = \"d\"\nname = \"D\"\n";
    let infos = format!("{}\n{}", ending("end-b"), ending("end-f"));
    let token = "[[item]]\nid = \"token\"\nname = \"Žeton\"\nstarting = true\n";
    let checked = run(&[
        ("counters.toml", counters),
        ("items.toml", token),
        ("info.toml", &infos),
        ("actions/step-a.toml", &act("A", "change = { c = 1 }\n")),
        (
            "actions/step-b.toml",
            &act(
                "B",
                "uses = [\"token\"]\nrequire = { d = { at_most = 0 } }\ngives = [\"end-b\"]\n",
            ),
        ),
        (
            "actions/step-c.toml",
            &act(
                "C",
                "require = { c = { at_least = 1 } }\nchange = { d = 1 }\n",
            ),
        ),
        (
            "actions/step-f.toml",
            &act(
                "F",
                "uses = [\"token\"]\nrequire = { d = { at_least = 1 } }\ngives = [\"end-f\"]\n",
            ),
        ),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["end-b", "end-f"]);
    assert_eq!(route(&checked, "end-f"), ["A", "C"]);
}

/// Two actions change `c` by `up` and `down`; afterwards a decision (not an action, so it
/// adds no conflicts of its own) checks `c`: "Nízko" needs at most `low`, "Vysoko" at least
/// `high`.
fn changes_then_decision(extra: &str, up: i32, down: i32, low: i32, high: i32) -> Checked {
    let counters = counter(extra);
    let infos = format!("{}\n{}", ending("low"), ending("high"));
    let dec = format!(
        "[decision]\ntitle = \"Stav\"\n\n[[decision.option]]\nlabel = \"Nízko\"\nrequire = {{ c = {{ at_most = {low} }} }}\ngives = [\"low\"]\n\n[[decision.option]]\nlabel = \"Vysoko\"\nrequire = {{ c = {{ at_least = {high} }} }}\ngives = [\"high\"]\n"
    );
    run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        (
            "actions/up.toml",
            &act("Nahoru", &format!("change = {{ c = {up} }}\n")),
        ),
        (
            "actions/down.toml",
            &act("Dolů", &format!("change = {{ c = {down} }}\n")),
        ),
        ("actions/state.toml", &dec),
    ])
}

#[test]
fn changes_that_reach_but_do_not_pass_the_bounds_do_not_branch() {
    // From 3: +7 reaches max 10 and −3 reaches min 0, but neither passes it, so both orders
    // end at 7. The only choice is the decision.
    let checked = changes_then_decision("start = 3\n", 7, -3, 1, 5);
    assert_eq!(reached(&checked), ["high"], "{:?}", checked.diagnostics);
    assert_eq!(route(&checked, "high"), ["Stav → Vysoko"]);
    assert_eq!(checked.playthroughs.as_ref().unwrap().end_states, 1);
}

#[test]
fn changes_to_a_counter_that_can_clamp_at_both_ends_branch() {
    // From 4 with max 6: +5 then −5 is 1, −5 then +5 is 5. Each order gives one ending.
    let checked = changes_then_decision("start = 4\nmax = 6\n", 5, -5, 1, 5);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["high", "low"]);
    assert_eq!(route(&checked, "low"), ["Nahoru", "Stav → Nízko"]);
    assert_eq!(route(&checked, "high"), ["Dolů", "Stav → Vysoko"]);
}

#[test]
fn changes_to_a_counter_that_can_clamp_only_at_max_branch() {
    // From 4 with max 6: +5 then −1 is 5, −1 then +5 is 6; nothing goes below 0.
    let checked = changes_then_decision("start = 4\nmax = 6\n", 5, -1, 5, 6);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["high", "low"]);
}

#[test]
fn changes_to_a_counter_that_can_clamp_only_at_min_branch() {
    // From 2: −5 then +1 is 1, +1 then −5 is 0; nothing goes above 10.
    let checked = changes_then_decision("start = 2\n", 1, -5, 0, 1);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["high", "low"]);
}

#[test]
fn unrelated_action_is_not_ordered_against_a_competing_pair() {
    // `alarm` and `sneak` compete; `tidy` touches nothing they use and just happens.
    let counters = counter("");
    let infos = format!("{}\n{}", info("tidy"), ending("win"));
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        (
            "actions/alarm.toml",
            &act("Poplach", "change = { c = 2 }\n"),
        ),
        (
            "actions/sneak.toml",
            &act(
                "Proplížit se",
                "require = { c = { at_most = 1 } }\ngives = [\"win\"]\n",
            ),
        ),
        ("actions/z-tidy.toml", &act("Úklid", "gives = [\"tidy\"]\n")),
    ]);
    assert_eq!(route(&checked, "win"), ["Proplížit se"]);
    let stuck: Vec<&str> = messages(&checked, Severity::Warning)
        .into_iter()
        .filter(|w| w.contains("gets stuck"))
        .collect();
    assert_eq!(
        stuck,
        ["a playthrough gets stuck without reaching any ending, after: Poplach"]
    );
}

#[test]
fn changes_to_a_counter_nobody_requires_do_not_branch_even_when_they_clamp() {
    // +5 and −5 on 4..=6 would clamp differently by order, but nothing reads the counter.
    let counters = counter("start = 4\nmax = 6\n");
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &ending("win")),
        ("actions/up.toml", &act("Nahoru", "change = { c = 5 }\n")),
        (
            "actions/down.toml",
            &act("Dolů", "change = { c = -5 }\ngives = [\"win\"]\n"),
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    assert_eq!(checked.playthroughs.as_ref().unwrap().end_states, 1);
    assert_eq!(route(&checked, "win"), Vec::<String>::new());
}

#[test]
fn many_changes_to_an_unrequired_counter_do_not_explode_the_states() {
    let counters = "[[counter]]\nid = \"hour\"\nname = \"Hodina\"\nmax = 100\n";
    let actions: Vec<(String, String)> = (0..14)
        .map(|i| {
            (
                format!("actions/tick-{i:02}.toml"),
                act(&format!("Tik {i}"), "change = { hour = 1 }\n"),
            )
        })
        .collect();
    let mut files: Vec<(&str, &str)> = vec![("counters.toml", counters)];
    files.extend(actions.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let checked = run(&files);
    let p = checked.playthroughs.as_ref().unwrap();
    assert!(!p.truncated, "{:?}", checked.diagnostics);
    assert_eq!(p.end_states, 1);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

/// `first` adds `a`, then `second` adds `b`, then `check` gives `win` if the counter matches
/// `bound`. Each needs the one before, so nothing competes and the order is fixed.
fn chain(extra: &str, a: i32, b: i32, bound: &str) -> Checked {
    let counters = counter(extra);
    let infos = format!(
        "{}\n{}\n{}",
        info("first-done"),
        info("second-done"),
        ending("win")
    );
    let first = act(
        "První",
        &format!("change = {{ c = {a} }}\ngives = [\"first-done\"]\n"),
    );
    let second = act(
        "Druhá",
        &format!("needs = [\"first-done\"]\nchange = {{ c = {b} }}\ngives = [\"second-done\"]\n"),
    );
    let check = act(
        "Kontrola",
        &format!("needs = [\"second-done\"]\nrequire = {{ c = {bound} }}\ngives = [\"win\"]\n"),
    );
    run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        ("actions/a-first.toml", &first),
        ("actions/b-second.toml", &second),
        ("actions/c-check.toml", &check),
    ])
}

#[test]
fn change_stops_at_max() {
    // 4 + 5 stops at 6, and 6 − 5 = 1; without clamping it would be back at 4.
    let checked = chain("start = 4\nmax = 6\n", 5, -5, "{ at_most = 1 }");
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}

#[test]
fn change_without_clamping_would_not_reach_the_unclamped_sum() {
    // The other side of `change_stops_at_max`: 4 is never reached again after the clamp.
    let checked = chain("start = 4\nmax = 6\n", 5, -5, "{ at_least = 4 }");
    assert_eq!(
        messages(&checked, Severity::Error),
        ["ending `win` cannot be reached in any playthrough"]
    );
}

#[test]
fn change_stops_at_min() {
    // 2 − 5 stops at 0, and 0 + 3 = 3; without clamping it would be 0.
    let checked = chain("start = 2\n", -5, 3, "{ at_least = 3 }");
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}

#[test]
fn change_stops_at_a_negative_min() {
    // −1 − 100 stops at −3, and −3 + 4 = 1.
    let checked = chain(
        "start = -1\nmin = -3\n",
        -100,
        4,
        "{ at_least = 1, at_most = 1 }",
    );
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}

#[test]
fn huge_changes_do_not_overflow() {
    // 10 + i32::MAX stops at 10, and 10 + i32::MIN stops at 0.
    let checked = chain("start = 10\n", i32::MAX, i32::MIN, "{ at_most = 0 }");
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}

#[test]
fn actions_changing_the_same_counter_are_tried_in_both_orders() {
    // From 4 with max 6: up then down gives 1, down then up gives 5 (min 0 stops −1 at 0).
    // `low` needs 1 and `high` needs 5, so each ending is reachable only in one order.
    let counters = counter("start = 4\nmax = 6\n");
    let infos = format!(
        "{}\n{}\n{}\n{}",
        info("up-done"),
        info("down-done"),
        ending("low"),
        ending("high")
    );
    let up = act("Nahoru", "change = { c = 5 }\ngives = [\"up-done\"]\n");
    let down = act("Dolů", "change = { c = -5 }\ngives = [\"down-done\"]\n");
    let after = |bound: &str, end: &str| {
        act(
            end,
            &format!(
                "needs = [\"up-done\", \"down-done\"]\nrequire = {{ c = {bound} }}\ngives = [\"{end}\"]\n"
            ),
        )
    };
    let low = after("{ at_most = 1 }", "low");
    let high = after("{ at_least = 5 }", "high");
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        ("actions/up.toml", &up),
        ("actions/down.toml", &down),
        ("actions/to-low.toml", &low),
        ("actions/to-high.toml", &high),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["high", "low"]);
    assert_eq!(route(&checked, "low"), ["Nahoru"]);
    assert_eq!(route(&checked, "high"), ["Dolů"]);
    assert_eq!(checked.playthroughs.as_ref().unwrap().end_states, 2);
}

#[test]
fn action_changing_a_counter_another_requires_is_tried_in_both_orders() {
    // Both are ready at the start; sneaking first wins, the alarm first closes it.
    let counters = counter("");
    let alarm = act("Poplach", "change = { c = 2 }\n");
    let sneak = act(
        "Proplížit se",
        "require = { c = { at_most = 1 } }\ngives = [\"win\"]\n",
    );
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &ending("win")),
        ("actions/alarm.toml", &alarm),
        ("actions/sneak.toml", &sneak),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(reached(&checked), ["win"]);
    assert_eq!(route(&checked, "win"), ["Proplížit se"]);
    // The alarm-first order is a real playthrough too, and it gets stuck without an ending.
    assert_eq!(checked.playthroughs.as_ref().unwrap().end_states, 2);
    assert!(
        messages(&checked, Severity::Warning)
            .iter()
            .any(|w| w.contains("gets stuck") && w.contains("Poplach")),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn actions_on_different_counters_do_not_compete() {
    let counters =
        "[[counter]]\nid = \"a\"\nname = \"A\"\n\n[[counter]]\nid = \"b\"\nname = \"B\"\n";
    let checked = run(&[
        ("counters.toml", counters),
        ("info.toml", &ending("win")),
        ("actions/one.toml", &act("Jedna", "change = { a = 1 }\n")),
        (
            "actions/two.toml",
            &act(
                "Dva",
                "change = { b = 1 }\nrequire = { b = { at_most = 0 } }\ngives = [\"win\"]\n",
            ),
        ),
    ]);
    assert_eq!(reached(&checked), ["win"]);
    assert_eq!(route(&checked, "win"), Vec::<String>::new());
    assert_eq!(checked.playthroughs.as_ref().unwrap().end_states, 1);
}

#[test]
fn requirement_that_is_met_by_start_is_ready_at_once() {
    let counters = counter("start = 7\n");
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &ending("win")),
        ("actions/raise.toml", &act("x", "change = { c = 0 }\n")),
        (
            "actions/go.toml",
            &act(
                "Jít",
                "require = { c = { at_least = 7 } }\ngives = [\"win\"]\n",
            ),
        ),
    ]);
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}

/// A decision `verdict` whose option "Obvinit" needs `c` at least 3, after an action that adds
/// `amount`.
fn verdict(amount: i32) -> Checked {
    let counters = counter("");
    let infos = format!("{}\n{}", ending("accused"), ending("left"));
    let investigate = act("Vyšetřovat", &format!("change = {{ c = {amount} }}\n"));
    let dec = "[decision]\ntitle = \"Verdikt\"\n\n[[decision.option]]\nlabel = \"Obvinit\"\nrequire = { c = { at_least = 3 } }\ngives = [\"accused\"]\n\n[[decision.option]]\nlabel = \"Odejít\"\ngives = [\"left\"]\n";
    run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        ("actions/investigate.toml", &investigate),
        ("actions/verdict.toml", dec),
    ])
}

#[test]
fn option_requiring_a_counter_can_be_chosen_once_it_is_met() {
    let checked = verdict(3);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    assert_eq!(reached(&checked), ["accused", "left"]);
    assert_eq!(route(&checked, "accused"), ["Verdikt → Obvinit"]);
}

#[test]
fn option_requiring_an_unmet_counter_can_never_be_chosen() {
    let checked = verdict(2);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["ending `accused` cannot be reached in any playthrough"]
    );
    let d = diag(&checked, "option “Verdikt → Obvinit” can never be chosen");
    assert_eq!(d.severity, Severity::Warning);
    assert!(d.help.as_deref().unwrap().contains("counter"), "{d:?}");
}

#[test]
fn option_change_opens_an_action_that_requires_the_counter() {
    let counters = counter("");
    let dec = "[decision]\ntitle = \"Výslech\"\n\n[[decision.option]]\nlabel = \"Přiznat se\"\nchange = { c = 3 }\n\n[[decision.option]]\nlabel = \"Zapírat\"\n";
    let arrest = act(
        "Zatknout",
        "require = { c = { at_least = 3 } }\ngives = [\"win\"]\n",
    );
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &ending("win")),
        ("actions/talk.toml", dec),
        ("actions/arrest.toml", &arrest),
    ]);
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
    assert_eq!(route(&checked, "win"), ["Výslech → Přiznat se"]);
}

#[test]
fn option_change_is_clamped_too() {
    // The option adds 50 to a counter with max 10, and then an action subtracts 9: 1 is left.
    let counters = counter("");
    let infos = format!("{}\n{}", info("spoken"), ending("win"));
    let dec = "[decision]\ntitle = \"Řeč\"\n\n[[decision.option]]\nlabel = \"Křičet\"\nchange = { c = 50 }\ngives = [\"spoken\"]\n";
    let calm = act("Uklidnit", "needs = [\"spoken\"]\nchange = { c = -9 }\n");
    let check = act(
        "Kontrola",
        "needs = [\"spoken\"]\nrequire = { c = { at_least = 1, at_most = 1 } }\ngives = [\"win\"]\n",
    );
    let checked = run(&[
        ("counters.toml", &counters),
        ("info.toml", &infos),
        ("actions/a-talk.toml", dec),
        ("actions/b-calm.toml", &calm),
        ("actions/c-check.toml", &check),
    ]);
    assert_eq!(reached(&checked), ["win"], "{:?}", checked.diagnostics);
}
