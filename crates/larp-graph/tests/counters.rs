//! Counters in the graph data (`NodeView`, `OptionView`, `GraphData.counters`), in words
//! (`counter_texts`) and on organizer sheets (`ScheduleStep.counters`, `ScheduleOption.counters`).

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_graph::{CounterCheck, CounterView, GraphData, NodeView, Schedule, ScheduleStep, build};
use larp_model::model::Node;
use larp_model::{Game, Id, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Opatství\"\nlanguage = \"cs\"\n";

/// `suspicion` ("Podezření", 0..=10) and `money` ("Peníze", start 5, −10..=20, with a note).
const COUNTERS: &str = "[[counter]]\nid = \"suspicion\"\nname = \"Podezření\"\n\n\
    [[counter]]\nid = \"money\"\nname = \"Peníze\"\nstart = 5\nmin = -10\nmax = 20\n\
    text = \"Kolik má gang v truhle.\"\n";

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

/// Petra runs `bribe` (an action that requires and changes both counters) and `verdict`
/// (a decision with a requiring option, a changing option and a plain one).
fn abbey() -> Game {
    game(&[
        ("counters.toml", COUNTERS),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "actions/bribe.toml",
            "[action]\ntitle = \"Úplatek\"\nowners = [\"petra\"]\n\
             change = { suspicion = 2, money = -1 }\n\
             require = { suspicion = { at_most = 1 }, money = { at_least = 1, at_most = 4 } }\n",
        ),
        (
            "actions/verdict.toml",
            "[decision]\ntitle = \"Verdikt\"\nowners = [\"petra\"]\n\n\
             [[decision.option]]\nlabel = \"Obvinit\"\nrequire = { suspicion = { at_least = 3 } }\n\n\
             [[decision.option]]\nlabel = \"Uplatit\"\nchange = { money = -3 }\n\n\
             [[decision.option]]\nlabel = \"Odejít\"\n",
        ),
    ])
}

fn graph(g: &Game) -> GraphData {
    build(g, &[], None)
}

fn node<'a>(data: &'a GraphData, node: &str) -> &'a NodeView {
    data.nodes.iter().find(|n| n.id == node).unwrap()
}

fn check(counter: &str, at_least: Option<i32>, at_most: Option<i32>) -> CounterCheck {
    CounterCheck {
        counter: counter.to_owned(),
        at_least,
        at_most,
    }
}

fn pair(counter: &str, amount: i32) -> (String, i32) {
    (counter.to_owned(), amount)
}

fn texts(g: &Game, node: &str) -> Vec<String> {
    match &g.nodes[&id(node)] {
        Node::Action(a) => larp_graph::counter_texts(g, &a.change, &a.require),
        Node::Decision(_) => panic!("{node} is a decision"),
    }
}

fn option_texts(g: &Game, node: &str, option: usize) -> Vec<String> {
    match &g.nodes[&id(node)] {
        Node::Decision(d) => {
            let o = &d.options[option];
            larp_graph::counter_texts(g, &o.change, &o.require)
        }
        Node::Action(_) => panic!("{node} is an action"),
    }
}

/// An action `act` with the given `change`/`require` lines, and the counters.
fn one_action(lines: &str) -> Game {
    game(&[
        ("counters.toml", COUNTERS),
        (
            "actions/act.toml",
            &format!("[action]\ntitle = \"x\"\n{lines}"),
        ),
    ])
}

fn sheet(g: &Game, owner: &str) -> Schedule {
    larp_graph::schedule(g, &id(owner))
}

fn step<'a>(s: &'a Schedule, node: &str) -> &'a ScheduleStep {
    s.blocks
        .iter()
        .flat_map(|b| &b.steps)
        .find(|st| st.id == node)
        .unwrap()
}

// ---------- NodeView and OptionView ----------

#[test]
fn action_node_lists_changes_by_counter_id() {
    let data = graph(&abbey());
    assert_eq!(
        node(&data, "bribe").change,
        [pair("money", -1), pair("suspicion", 2)]
    );
}

#[test]
fn action_node_lists_requirements_with_both_bounds() {
    let data = graph(&abbey());
    assert_eq!(
        node(&data, "bribe").require,
        [
            check("money", Some(1), Some(4)),
            check("suspicion", None, Some(1))
        ]
    );
}

#[test]
fn decision_node_has_no_counters_of_its_own() {
    let data = graph(&abbey());
    let v = node(&data, "verdict");
    assert!(v.change.is_empty(), "{:?}", v.change);
    assert!(v.require.is_empty(), "{:?}", v.require);
}

#[test]
fn action_without_counters_has_empty_lists() {
    let data = graph(&game(&[("actions/act.toml", "[action]\ntitle = \"x\"\n")]));
    let v = node(&data, "act");
    assert!(v.change.is_empty(), "{:?}", v.change);
    assert!(v.require.is_empty(), "{:?}", v.require);
}

#[test]
fn option_views_carry_their_own_changes_and_requirements() {
    let data = graph(&abbey());
    let options = &node(&data, "verdict").options;
    assert_eq!(options[0].require, [check("suspicion", Some(3), None)]);
    assert!(options[0].change.is_empty(), "{:?}", options[0].change);
    assert_eq!(options[1].change, [pair("money", -3)]);
    assert!(options[1].require.is_empty(), "{:?}", options[1].require);
    assert!(options[2].change.is_empty() && options[2].require.is_empty());
}

#[test]
fn changes_and_checks_helpers_match_the_node_view() {
    let g = abbey();
    let Node::Action(a) = &g.nodes[&id("bribe")] else {
        panic!("bribe is an action");
    };
    let data = graph(&g);
    assert_eq!(larp_graph::changes(&a.change), node(&data, "bribe").change);
    assert_eq!(larp_graph::checks(&a.require), node(&data, "bribe").require);
}

// ---------- GraphData.counters ----------

#[test]
fn graph_lists_every_counter_in_file_order_with_resolved_defaults() {
    let data = graph(&abbey());
    assert_eq!(
        data.counters,
        [
            CounterView {
                id: "suspicion".to_owned(),
                name: "Podezření".to_owned(),
                start: 0,
                min: 0,
                max: 10,
                text: String::new(),
            },
            CounterView {
                id: "money".to_owned(),
                name: "Peníze".to_owned(),
                start: 5,
                min: -10,
                max: 20,
                text: "Kolik má gang v truhle.".to_owned(),
            },
        ]
    );
}

#[test]
fn graph_of_a_game_without_counters_lists_none() {
    let data = graph(&game(&[]));
    assert!(data.counters.is_empty(), "{:?}", data.counters);
}

// ---------- counter_texts ----------

#[test]
fn at_least_reads_as_greater_or_equal() {
    let g = one_action("require = { suspicion = { at_least = 3 } }\n");
    assert_eq!(texts(&g, "act"), ["Podezření ≥ 3"]);
}

#[test]
fn at_most_reads_as_less_or_equal() {
    let g = one_action("require = { suspicion = { at_most = 1 } }\n");
    assert_eq!(texts(&g, "act"), ["Podezření ≤ 1"]);
}

#[test]
fn both_bounds_read_as_a_range_with_an_en_dash() {
    let g = one_action("require = { money = { at_least = 1, at_most = 4 } }\n");
    assert_eq!(texts(&g, "act"), ["Peníze 1–4"]);
}

#[test]
fn positive_change_reads_with_a_plus() {
    let g = one_action("change = { suspicion = 2 }\n");
    assert_eq!(texts(&g, "act"), ["Podezření +2"]);
}

#[test]
fn negative_change_reads_with_a_minus_sign() {
    let g = one_action("change = { money = -1 }\n");
    assert_eq!(texts(&g, "act"), ["Peníze −1"]);
    assert!(
        !texts(&g, "act")[0].contains('-'),
        "a typographic minus (U+2212), not a hyphen"
    );
}

#[test]
fn zero_change_reads_as_plus_zero() {
    let g = one_action("change = { money = 0 }\n");
    assert_eq!(texts(&g, "act"), ["Peníze +0"]);
}

#[test]
fn most_negative_change_does_not_overflow() {
    let g = one_action("change = { money = -2147483648 }\n");
    assert_eq!(texts(&g, "act"), ["Peníze −2147483648"]);
}

#[test]
fn requirements_come_before_changes() {
    let g = abbey();
    assert_eq!(
        texts(&g, "bribe"),
        ["Peníze 1–4", "Podezření ≤ 1", "Peníze −1", "Podezření +2"]
    );
}

#[test]
fn texts_are_empty_without_counters() {
    let g = one_action("");
    assert!(texts(&g, "act").is_empty(), "{:?}", texts(&g, "act"));
}

#[test]
fn unknown_counter_falls_back_to_its_id() {
    // `load` drops games with unknown counters, so look up against a game without counters.
    let with = one_action("change = { suspicion = 2 }\nrequire = { money = { at_least = 1 } }\n");
    let without = game(&[]);
    let Node::Action(a) = &with.nodes[&id("act")] else {
        panic!("act is an action");
    };
    assert_eq!(
        larp_graph::counter_texts(&without, &a.change, &a.require),
        ["money ≥ 1", "suspicion +2"]
    );
}

#[test]
fn option_texts_use_the_option_keys() {
    let g = abbey();
    assert_eq!(option_texts(&g, "verdict", 0), ["Podezření ≥ 3"]);
    assert_eq!(option_texts(&g, "verdict", 1), ["Peníze −3"]);
    assert!(
        option_texts(&g, "verdict", 2).is_empty(),
        "{:?}",
        option_texts(&g, "verdict", 2)
    );
}

// ---------- organizer sheets ----------

#[test]
fn schedule_step_lists_counter_texts() {
    let g = abbey();
    let s = sheet(&g, "petra");
    assert_eq!(
        step(&s, "bribe").counters,
        ["Peníze 1–4", "Podezření ≤ 1", "Peníze −1", "Podezření +2"]
    );
}

#[test]
fn schedule_decision_lists_counters_per_option_not_on_the_step() {
    let g = abbey();
    let s = sheet(&g, "petra");
    let v = step(&s, "verdict");
    assert!(v.counters.is_empty(), "{:?}", v.counters);
    let per_option: Vec<&[String]> = v.options.iter().map(|o| o.counters.as_slice()).collect();
    assert_eq!(
        per_option,
        [
            &["Podezření ≥ 3".to_owned()][..],
            &["Peníze −3".to_owned()][..],
            &[][..]
        ]
    );
}
