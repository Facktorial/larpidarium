//! Edges and layout derived from small in-memory games.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{GraphData, build};
use larp_model::{Game, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"en\"\n";
const INFO: &str = "[[info]]\nid = \"clue\"\nname = \"Clue\"\n\n[[info]]\nid = \"end\"\nname = \"End\"\nending = true\n";

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

fn build_game(game: &Game) -> GraphData {
    build(game, &[], None)
}

fn chain() -> Vec<(&'static str, &'static str)> {
    vec![
        ("game.toml", GAME),
        ("info.toml", INFO),
        (
            "actions/find.toml",
            "[action]\ntitle = \"Find\"\ngives = [\"clue\"]\n",
        ),
        (
            "actions/solve.toml",
            "[action]\ntitle = \"Solve\"\nneeds = [\"clue\"]\ngives = [\"end\"]\n",
        ),
    ]
}

#[test]
fn shared_item_creates_one_edge() {
    let graph = build_game(&game(&chain()));
    assert_eq!(graph.edges.len(), 1);
    let e = &graph.edges[0];
    assert_eq!(
        (e.id.as_str(), e.from.as_str(), e.to.as_str()),
        ("find>solve", "find", "solve")
    );
    assert_eq!(e.items, ["clue"]);
    assert_eq!((e.from_port, e.to_port), (None, None));
}

#[test]
fn decision_options_are_separate_ports() {
    let decision = "[decision]\ntitle = \"Choose\"\n\n[[decision.option]]\nlabel = \"Skip\"\n\n[[decision.option]]\nlabel = \"Use\"\nneeds = [\"clue\"]\ngives = [\"end\"]\n";
    let graph = build_game(&game(&[
        ("game.toml", GAME),
        ("info.toml", INFO),
        (
            "actions/find.toml",
            "[action]\ntitle = \"Find\"\ngives = [\"clue\"]\n",
        ),
        ("actions/choose.toml", decision),
    ]));
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].id, "find>choose.1");
    assert_eq!(graph.edges[0].to_port, Some(1));
}

#[test]
fn layout_puts_receivers_right_of_givers() {
    let graph = build_game(&game(&chain()));
    let x = |id: &str| graph.nodes.iter().find(|n| n.id == id).unwrap().x;
    assert!(x("solve") > x("find"));
}

#[test]
fn layout_toml_overrides_positions_and_keeps_knees() {
    let mut files = chain();
    files.push((
        "layout.toml",
        "[node.find]\nx = 500\ny = 7\n\n[knees]\n\"find>solve\" = [{ x = 1, y = 2 }]\n",
    ));
    let graph = build_game(&game(&files));
    let find = graph.nodes.iter().find(|n| n.id == "find").unwrap();
    assert_eq!((find.x, find.y), (500.0, 7.0));
    assert_eq!(graph.knees["find>solve"].len(), 1);
}

#[test]
fn loops_do_not_hang_the_layout() {
    let info = "[[info]]\nid = \"egg\"\nname = \"Egg\"\nstarting = true\n\n[[info]]\nid = \"hen\"\nname = \"Hen\"\n";
    let graph = build_game(&game(&[
        ("game.toml", GAME),
        ("info.toml", info),
        (
            "actions/lay.toml",
            "[action]\ntitle = \"Lay\"\nneeds = [\"hen\"]\ngives = [\"egg\"]\n",
        ),
        (
            "actions/hatch.toml",
            "[action]\ntitle = \"Hatch\"\nneeds = [\"egg\"]\ngives = [\"hen\"]\n",
        ),
    ]));
    assert_eq!(graph.edges.len(), 2);
    assert!(
        graph
            .nodes
            .iter()
            .all(|n| n.x.is_finite() && n.y.is_finite())
    );
}

// ---------- destiny ----------

fn person_game() -> Game {
    let person = "[person]\nname = \"Ann\"\nrole = \"player\"\n";
    let items = "[[item]]\nid = \"key\"\nname = \"Key\"\nowners = [\"ann\"]\nstarting = true\n";
    let info = "[[info]]\nid = \"clue\"\nname = \"Clue\"\n\n[[info]]\nid = \"map\"\nname = \"Map\"\n\n[[info]]\nid = \"end\"\nname = \"End\"\nending = true\n";
    game(&[
        ("game.toml", GAME),
        ("people/ann.toml", person),
        ("items.toml", items),
        ("info.toml", info),
        // someone else's action gives the clue
        (
            "actions/hint.toml",
            "[action]\ntitle = \"Hint\"\ngives = [\"clue\"]\n",
        ),
        (
            "actions/search.toml",
            "[action]\ntitle = \"Search\"\nowners = [\"ann\"]\nneeds = [\"clue\"]\ngives = [\"map\"]\nbrief = \"Look around.\"\n",
        ),
        (
            "actions/escape.toml",
            "[decision]\ntitle = \"Escape?\"\nowners = [\"ann\"]\n\n[[decision.option]]\nlabel = \"Go\"\nneeds = [\"map\"]\nuses = [\"key\"]\ngives = [\"end\"]\n",
        ),
    ])
}

#[test]
fn destiny_lists_own_nodes_in_time_order() {
    let g = person_game();
    let graph = build_game(&g);
    let d = larp_graph::destiny(&g, &graph, &larp_model::Id::parse("ann").unwrap());
    let titles: Vec<&str> = d.steps.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, ["Search", "Escape?"]);
    assert_eq!(d.steps[0].brief, "Look around.");
    assert!(d.steps[1].decision);
    assert_eq!(d.carries.len(), 1);
    assert_eq!(d.carries[0].name, "Key");
}

#[test]
fn destiny_describes_sources_without_naming_others() {
    use larp_graph::Source;
    let g = person_game();
    let graph = build_game(&g);
    let d = larp_graph::destiny(&g, &graph, &larp_model::Id::parse("ann").unwrap());
    assert_eq!(
        d.steps[0].needs[0].source,
        Some(Source::Receive),
        "clue comes from someone else"
    );
    let go = &d.steps[1].options[0];
    assert_eq!(
        go.needs[0].source,
        Some(Source::Earlier),
        "map comes from her own search"
    );
    assert_eq!(
        go.uses[0].source,
        Some(Source::Carry),
        "she carries the key"
    );
}

#[test]
fn mini_graph_anonymizes_neighbors() {
    let g = person_game();
    let graph = build_game(&g);
    let d = larp_graph::destiny(&g, &graph, &larp_model::Id::parse("ann").unwrap());
    let hint = d.graph.nodes.iter().find(|n| n.id == "hint").unwrap();
    assert!(!hint.own);
    assert!(hint.label.is_empty(), "other people's nodes have no label");
    assert_eq!(d.graph.nodes.iter().filter(|n| n.own).count(), 2);
    assert_eq!(d.graph.edges.len(), 2);
}

// ---------- segments ----------

fn segmented() -> Game {
    game(&[
        ("game.toml", GAME),
        ("info.toml", INFO),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"One\"\n\n[[segment]]\nid = \"two\"\nname = \"Two\"\ncolor = \"#aa0000\"\n",
        ),
        // `late` is listed first alphabetically but belongs to the second segment
        (
            "actions/late.toml",
            "[action]\ntitle = \"Late\"\nsegment = \"two\"\nneeds = [\"clue\"]\ngives = [\"end\"]\n",
        ),
        (
            "actions/early.toml",
            "[action]\ntitle = \"Early\"\nsegment = \"one\"\ngives = [\"clue\"]\n",
        ),
        ("actions/loose.toml", "[action]\ntitle = \"Loose\"\n"),
    ])
}

#[test]
fn segments_are_exported_in_file_order_with_colors() {
    let graph = build_game(&segmented());
    let names: Vec<&str> = graph.segments.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["One", "Two"]);
    assert_eq!(graph.segments[1].color, "#aa0000");
    let late = graph.nodes.iter().find(|n| n.id == "late").unwrap();
    assert_eq!(late.segment.as_deref(), Some("two"));
}

#[test]
fn edges_between_segments_are_marked() {
    let graph = build_game(&segmented());
    assert_eq!(graph.edges.len(), 1);
    assert!(graph.edges[0].cross);
}

#[test]
fn each_segment_gets_its_own_band_in_order() {
    let graph = build_game(&segmented());
    let y = |id: &str| graph.nodes.iter().find(|n| n.id == id).unwrap().y;
    assert!(y("early") < y("late"), "segment one is above segment two");
    assert!(y("late") < y("loose"), "nodes without a segment come last");
    let x = |id: &str| graph.nodes.iter().find(|n| n.id == id).unwrap().x;
    assert!(
        (x("early") - x("late")).abs() < f64::EPSILON,
        "bands are laid out independently, so both start in the first column"
    );
}

#[test]
fn unknown_segments_count_as_no_segment() {
    // An unknown segment id is an error, so this game only exists as a draft (as in `serve`).
    let files: Vec<SourceFile> = [
        ("game.toml", GAME),
        ("info.toml", INFO),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"One\"\n",
        ),
        (
            "actions/typo.toml",
            "[action]\ntitle = \"Typo\"\nsegment = \"act-nine\"\ngives = [\"clue\"]\n",
        ),
        (
            "actions/loose.toml",
            "[action]\ntitle = \"Loose\"\nneeds = [\"clue\"]\ngives = [\"end\"]\n",
        ),
    ]
    .iter()
    .map(|(path, text)| SourceFile {
        path: (*path).to_owned(),
        text: (*text).to_owned(),
    })
    .collect();
    let draft = load(&files).draft.unwrap();
    let graph = build_game(&draft);
    assert!(
        !graph.edges[0].cross,
        "both nodes are drawn in the no-segment band"
    );
    let typo = graph.nodes.iter().find(|n| n.id == "typo").unwrap();
    assert_eq!(
        typo.segment.as_deref(),
        Some("act-nine"),
        "the raw id is kept for the editor"
    );
}
