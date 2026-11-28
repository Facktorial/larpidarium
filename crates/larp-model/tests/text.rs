//! `[[id]]` links in texts: finding them (`links`) and replacing them (`expand_text`, `expand`).

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::fmt::Write as _;

use larp_model::model::Node;
use larp_model::text::{MAX_DEPTH, each_text, expand, expand_text, links, target};
use larp_model::{Game, Id, SourceFile, load};
use proptest::prelude::*;

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";

/// The game, or the draft when the files have errors (expansion must work on both).
fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    let checked = load(&files);
    let diagnostics = format!("{:?}", checked.diagnostics);
    checked
        .game
        .or(checked.draft)
        .unwrap_or_else(|| panic!("game did not load: {diagnostics}"))
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

/// `blocks.toml` with one untitled block per (id, text).
fn blocks_toml(blocks: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (id, text) in blocks {
        writeln!(out, "[[block]]\nid = \"{id}\"\ntext = \"{text}\"\n").unwrap();
    }
    out
}

/// A chain of `n` blocks `b0` → `b1` → … → `b{n-1}`, the last one saying `konec`.
fn chain(n: usize) -> Game {
    let texts: Vec<(String, String)> = (0..n)
        .map(|i| {
            let text = if i + 1 == n {
                "konec".to_owned()
            } else {
                format!("[[b{}]]", i + 1)
            };
            (format!("b{i}"), text)
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    game(&[("blocks.toml", &blocks_toml(&refs))])
}

// ---------- links ----------

/// `text` with every `[[` collapsed until none is left.
fn without_double_brackets(mut text: String) -> String {
    while text.contains("[[") {
        text = text.replace("[[", "[");
    }
    text
}

fn valid_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,8}(-[a-z0-9]{1,8}){0,3}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn links_never_panics_on_arbitrary_text(text in any::<String>()) {
        let _ = links(&text);
    }

    #[test]
    fn links_never_panics_on_bracket_soup(text in "[\\[\\]a-zřů -]{0,40}") {
        let _ = links(&text);
    }

    #[test]
    fn every_link_range_slices_to_brackets_around_its_id(
        text in "([\\[\\]a-z0-9 čřů-]|\\[\\[[a-z][a-z0-9-]{0,6}\\]\\]){0,20}",
    ) {
        let mut previous_end = 0;
        for link in links(&text) {
            let expected = format!("[[{}]]", link.id);
            prop_assert_eq!(&text[link.range.clone()], expected.as_str());
            prop_assert!(link.range.start >= previous_end, "links overlap in {:?}", text);
            previous_end = link.range.end;
        }
    }

    #[test]
    fn finds_a_valid_id_between_any_texts_without_brackets(
        before in "[^\\[\\]]{0,20}",
        name in valid_id(),
        after in "[^\\[\\]]{0,20}",
    ) {
        let text = format!("{before}[[{name}]]{after}");
        let found = links(&text);
        prop_assert_eq!(found.len(), 1);
        prop_assert_eq!(found[0].id.as_str(), name.as_str());
        prop_assert_eq!(found[0].range.clone(), before.len()..before.len() + name.len() + 4);
    }

    #[test]
    fn text_without_double_bracket_has_no_links(text in any::<String>()) {
        let text = without_double_brackets(text);
        prop_assert!(links(&text).is_empty(), "{:?}", text);
    }

    #[test]
    fn expand_text_leaves_text_without_links_unchanged(text in any::<String>()) {
        let text = without_double_brackets(text);
        let g = game(&[]);
        prop_assert_eq!(expand_text(&g, &text), text);
    }
}

#[test]
fn links_returns_nothing_for_empty_text() {
    assert_eq!(links(""), []);
}

#[test]
fn links_ignores_brackets_around_text_that_is_not_an_id() {
    assert_eq!(links("[[ poznámka ]] [[Velké]] [[]] [[kůň]] [[a--b]]"), []);
}

#[test]
fn links_finds_an_id_inside_extra_brackets() {
    let text = "[[[datum]]]";
    let found = links(text);
    assert_eq!(found.len(), 1);
    assert_eq!(&text[found[0].range.clone()], "[[datum]]");
}

#[test]
fn links_finds_every_link_in_order() {
    let found: Vec<String> = links("[[a]] a [[b-2]] a [[a]]")
        .into_iter()
        .map(|l| l.id.to_string())
        .collect();
    assert_eq!(found, ["a", "b-2", "a"]);
}

// ---------- target and expansion ----------

fn everything() -> Game {
    game(&[
        (
            "blocks.toml",
            "[[block]]\nid = \"zvesti\"\ntitle = \"Zvěsti\"\ntext = \"Šeptá se, že [[don]] zaplatil stráž do [[datum]].\"\n\n\
             [[block]]\nid = \"datum\"\ntext = \"13. března\"\n",
        ),
        (
            "people/don.toml",
            "[person]\nname = \"Don Řehoř\"\nrole = \"npc\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"pristav\"\nname = \"Přístav\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Rezavý klíč\"\nstarting = true\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"zprava\"\nname = \"Zpráva z lodi\"\nstarting = true\n",
        ),
        (
            "actions/ukaz.toml",
            "[action]\ntitle = \"Ukaž\"\nowners = [\"don\"]\nsegment = \"jidelna\"\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"jidelna\"\nname = \"Jídelna\"\n",
        ),
    ])
}

#[test]
fn target_of_a_block_is_its_unexpanded_text() {
    let g = everything();
    assert_eq!(
        target(&g, &id("zvesti")),
        Some("Šeptá se, že [[don]] zaplatil stráž do [[datum]].")
    );
}

#[test]
fn target_of_people_rooms_items_and_info_is_their_name() {
    let g = everything();
    assert_eq!(target(&g, &id("don")), Some("Don Řehoř"));
    assert_eq!(target(&g, &id("pristav")), Some("Přístav"));
    assert_eq!(target(&g, &id("klic")), Some("Rezavý klíč"));
    assert_eq!(target(&g, &id("zprava")), Some("Zpráva z lodi"));
}

#[test]
fn target_of_actions_segments_and_unknown_ids_is_none() {
    let g = everything();
    assert_eq!(target(&g, &id("ukaz")), None);
    assert_eq!(target(&g, &id("jidelna")), None);
    assert_eq!(target(&g, &id("nikdo")), None);
}

#[test]
fn expand_text_replaces_people_rooms_items_and_info_with_names() {
    let g = everything();
    assert_eq!(
        expand_text(&g, "[[don]] v [[pristav]] má [[klic]] a [[zprava]]"),
        "Don Řehoř v Přístav má Rezavý klíč a Zpráva z lodi"
    );
}

#[test]
fn expand_text_replaces_nested_blocks() {
    let g = everything();
    assert_eq!(
        expand_text(&g, "Víš: [[zvesti]]"),
        "Víš: Šeptá se, že Don Řehoř zaplatil stráž do 13. března."
    );
}

#[test]
fn expand_text_keeps_czech_text_around_links() {
    let g = everything();
    assert_eq!(
        expand_text(&g, "Příliš žluťoučký kůň [[datum]] úpěl ďábelské ódy."),
        "Příliš žluťoučký kůň 13. března úpěl ďábelské ódy."
    );
}

#[test]
fn expand_text_leaves_unknown_links_and_non_ids_as_written() {
    let g = everything();
    assert_eq!(
        expand_text(
            &g,
            "[[nikdo]] [[ poznámka ]] [[ukaz]] [[jidelna]] [[datum]]"
        ),
        "[[nikdo]] [[ poznámka ]] [[ukaz]] [[jidelna]] 13. března"
    );
}

#[test]
fn expand_text_stops_a_loop_of_blocks() {
    let g = game(&[(
        "blocks.toml",
        &blocks_toml(&[("vejce", "a [[slepice]]"), ("slepice", "b [[vejce]]")]),
    )]);
    let out = expand_text(&g, "[[vejce]]");
    // One replacement per level, and the level that is too deep stays as written.
    assert_eq!(
        out.matches("a ").count() + out.matches("b ").count(),
        MAX_DEPTH
    );
    assert!(
        out.ends_with("[[vejce]]") || out.ends_with("[[slepice]]"),
        "{out}"
    );
}

#[test]
fn expand_text_stops_a_block_that_includes_itself() {
    let g = game(&[("blocks.toml", &blocks_toml(&[("sam", "ř[[sam]]")]))]);
    assert_eq!(
        expand_text(&g, "[[sam]]"),
        format!("{}[[sam]]", "ř".repeat(MAX_DEPTH))
    );
}

#[test]
fn expand_text_resolves_a_chain_of_max_depth_blocks() {
    let g = chain(MAX_DEPTH);
    assert_eq!(expand_text(&g, "[[b0]]"), "konec");
}

#[test]
fn expand_text_leaves_the_link_one_level_past_max_depth() {
    let g = chain(MAX_DEPTH + 1);
    assert_eq!(expand_text(&g, "[[b0]]"), format!("[[b{MAX_DEPTH}]]"));
}

/// A game with `[[datum]]` in every kind of text that `docs/format.md` lists.
const DATUM: &str = "[[datum]]";
const EXPANDED: &str = "13. března";

fn linked_everywhere() -> Game {
    game(&[
        (
            "blocks.toml",
            "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n\n\
             [[block]]\nid = \"obal\"\ntext = \"[[datum]]\"\nshow = [\"ann\"]\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"s1\"\nname = \"Jedna\"\ntext = \"[[datum]]\"\n",
        ),
        (
            "cycles.toml",
            "[[cycle]]\nid = \"kolo\"\nname = \"Kolo\"\nhero = \"ann\"\nfrom = \"s1\"\ntext = \"[[datum]]\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Klíč\"\nstarting = true\ntext = \"[[datum]]\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"zprava\"\nname = \"Zpráva\"\nstarting = true\ntext = \"[[datum]]\"\n",
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\ngoals = [\"[[datum]]\"]\nsecrets = [\"[[datum]]\"]\n\n\
             [person.sheet]\nbackstory = \"[[datum]]\"\nwhereabouts = \"[[datum]]\"\n\n\
             [[person.relation]]\nwith = \"bob\"\ntext = \"[[datum]]\"\n\n\
             [[person.phase]]\nsegment = \"s1\"\ngoals = [\"[[datum]]\"]\nsecrets = [\"[[datum]]\"]\n\
             [person.phase.sheet]\nwhereabouts = \"[[datum]]\"\n",
        ),
        (
            "people/bob.toml",
            "[person]\nname = \"Bob\"\nrole = \"npc\"\n",
        ),
        (
            "actions/akce.toml",
            "[action]\ntitle = \"Akce\"\nowners = [\"ann\"]\nneeds = [\"klic\", \"zprava\"]\ntext = \"[[datum]]\"\nbrief = \"[[datum]]\"\n",
        ),
        (
            "actions/volba.toml",
            "[decision]\ntitle = \"Volba\"\nowners = [\"ann\"]\ntext = \"[[datum]]\"\nbrief = \"[[datum]]\"\n\n\
             [[decision.option]]\nlabel = \"Ano\"\n",
        ),
    ])
}

/// Texts with a link in [`linked_everywhere`]: 4 node texts, goal, secret, 2 sheet keys,
/// relation, phase goal, phase secret, phase sheet, item, info, segment, cycle and block `obal`.
const LINKED_TEXTS: usize = 17;

#[test]
fn each_text_visits_every_linkable_text() {
    let g = linked_everywhere();
    let mut seen = Vec::new();
    each_text(&g, |place, t| {
        if t.contains(DATUM) {
            seen.push(format!("{} ({})", place.file, place.field));
        }
    });
    assert_eq!(seen.len(), LINKED_TEXTS, "{seen:#?}");
    for expected in [
        "actions/akce.toml (text)",
        "actions/akce.toml (brief)",
        "actions/volba.toml (text)",
        "actions/volba.toml (brief)",
        "people/ann.toml (goals / secrets)",
        "people/ann.toml (sheet)",
        "people/ann.toml (relation)",
        "people/ann.toml (phase)",
        "items.toml (text)",
        "info.toml (text)",
        "segments.toml (text)",
        "cycles.toml (text)",
        "blocks.toml (text)",
    ] {
        assert!(
            seen.iter().any(|s| s == expected),
            "{expected} in {seen:#?}"
        );
    }
}

#[test]
fn expand_leaves_no_link_in_any_text() {
    let shown = expand(&linked_everywhere());
    let mut texts = 0;
    each_text(&shown, |place, t| {
        assert!(!t.contains("[["), "{} ({}): {t}", place.file, place.field);
        if t == EXPANDED {
            texts += 1;
        }
    });
    // Every linked text, plus the text of block `datum` itself.
    assert_eq!(texts, LINKED_TEXTS + 1);
}

#[test]
fn expand_replaces_links_in_nodes() {
    let shown = expand(&linked_everywhere());
    let Some(Node::Action(a)) = shown.nodes.get(&id("akce")) else {
        panic!("akce is not an action");
    };
    assert_eq!((a.text.as_str(), a.brief.as_str()), (EXPANDED, EXPANDED));
    let Some(Node::Decision(d)) = shown.nodes.get(&id("volba")) else {
        panic!("volba is not a decision");
    };
    assert_eq!((d.text.as_str(), d.brief.as_str()), (EXPANDED, EXPANDED));
}

#[test]
fn expand_replaces_links_in_people_relations_and_phases() {
    let shown = expand(&linked_everywhere());
    let ann = &shown.people[&id("ann")];
    assert_eq!(ann.goals, [EXPANDED]);
    assert_eq!(ann.secrets, [EXPANDED]);
    assert!(ann.sheet.values().all(|t| t == EXPANDED), "{:?}", ann.sheet);
    assert_eq!(ann.relations[0].text, EXPANDED);
    let phase = &ann.phases[0];
    assert_eq!(phase.goals.as_deref(), Some(&[EXPANDED.to_owned()][..]));
    assert_eq!(phase.secrets.as_deref(), Some(&[EXPANDED.to_owned()][..]));
    assert_eq!(phase.sheet["whereabouts"], EXPANDED);
}

#[test]
fn expand_replaces_links_in_items_info_segments_cycles_and_blocks() {
    let shown = expand(&linked_everywhere());
    assert_eq!(shown.items[0].text, EXPANDED);
    assert_eq!(shown.info[0].text, EXPANDED);
    assert_eq!(shown.segments[0].text, EXPANDED);
    assert_eq!(shown.cycles[0].text, EXPANDED);
    assert!(shown.blocks.iter().all(|b| b.text == EXPANDED));
}

#[test]
fn expand_keeps_the_original_game_as_written() {
    let g = linked_everywhere();
    let _ = expand(&g);
    assert_eq!(g.items[0].text, DATUM);
    assert_eq!(g.blocks[1].text, DATUM);
}

#[test]
fn expand_does_not_change_names_or_titles() {
    let g = game(&[
        ("blocks.toml", &blocks_toml(&[("datum", "13. března")])),
        (
            "actions/akce.toml",
            "[action]\ntitle = \"[[datum]]\"\ntext = \"[[datum]]\"\n",
        ),
    ]);
    let shown = expand(&g);
    assert_eq!(shown.nodes[&id("akce")].title(), DATUM);
}
