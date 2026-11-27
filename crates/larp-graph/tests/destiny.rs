//! A character's destiny: carried things and the mini graph of Part II.
//!
//! Node positions are pinned with `layout.toml`, so the mini graph geometry is exact.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]
// Mini graph coordinates are sums of small integers, exact in `f64`.
#![allow(clippy::float_cmp)]

use larp_graph::{Destiny, MiniGraph, MiniNode, build, destiny};
use larp_model::{Game, Id, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Osud\"\nlanguage = \"cs\"\n";
const PEOPLE: [(&str, &str); 2] = [
    (
        "people/ann.toml",
        "[person]\nname = \"Anna Řehořová\"\nrole = \"player\"\n",
    ),
    (
        "people/bob.toml",
        "[person]\nname = \"Bob Čech\"\nrole = \"player\"\n",
    ),
];
const ITEMS: &str = "[[item]]\nid = \"key\"\nname = \"Klíč\"\nowners = [\"ann\"]\nstarting = true\n\n\
    [[item]]\nid = \"torch\"\nname = \"Pochodeň\"\nowners = [\"bob\"]\nstarting = true\n\n\
    [[item]]\nid = \"lamp\"\nname = \"Lampa\"\nowners = [\"ann\"]\n";
const INFO: &str = "[[info]]\nid = \"clue\"\nname = \"Stopa\"\n\n[[info]]\nid = \"clue2\"\nname = \"Druhá stopa\"\n\n\
    [[info]]\nid = \"map\"\nname = \"Mapa\"\n\n[[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n";

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

/// Bob's `hint` and `hint2` (one column, two rows) feed Ann's `search` (21-character title), which
/// feeds her decision `escape` (18-character title). Bob's `far` is not connected to Ann.
fn fixture() -> Game {
    let mut files = vec![
        ("game.toml", GAME),
        ("items.toml", ITEMS),
        ("info.toml", INFO),
    ];
    files.extend(PEOPLE);
    files.extend([
        (
            "actions/hint.toml",
            "[action]\ntitle = \"Nápověda\"\nowners = [\"bob\"]\ngives = [\"clue\"]\n",
        ),
        (
            "actions/hint2.toml",
            "[action]\ntitle = \"Druhá nápověda\"\nowners = [\"bob\"]\ngives = [\"clue2\"]\n",
        ),
        (
            "actions/search.toml",
            "[action]\ntitle = \"Prohledat celou kapli\"\nowners = [\"ann\"]\nneeds = [\"clue\", \"clue2\", \"key\"]\ngives = [\"map\"]\n",
        ),
        (
            "actions/escape.toml",
            "[decision]\ntitle = \"Útěk přes kryptu č\"\nowners = [\"ann\"]\n\n[[decision.option]]\nlabel = \"Jít\"\nneeds = [\"map\"]\ngives = [\"end\"]\n",
        ),
        (
            "actions/far.toml",
            "[action]\ntitle = \"Daleko\"\nowners = [\"bob\"]\nneeds = [\"torch\"]\n",
        ),
        (
            "layout.toml",
            "[node.hint]\nx = 50\ny = 0\n\n[node.hint2]\nx = 50\ny = 100\n\n\
             [node.search]\nx = 300\ny = 0\n\n[node.escape]\nx = 600\ny = 40\n\n[node.far]\nx = 900\ny = 0\n",
        ),
    ]);
    game(&files)
}

fn destiny_of(g: &Game, person: &str) -> Destiny {
    let graph = build(g, &[], None);
    destiny(g, &graph, &Id::parse(person).unwrap())
}

fn node<'a>(mini: &'a MiniGraph, id: &str) -> &'a MiniNode {
    mini.nodes.iter().find(|n| n.id == id).unwrap()
}

// ---------- carried things ----------

#[test]
fn carries_only_starting_things_the_character_owns() {
    let d = destiny_of(&fixture(), "ann");
    let carried: Vec<&str> = d.carries.iter().map(|t| t.id.as_str()).collect();
    // Not Bob's starting torch, not Ann's lamp that is not a starting item.
    assert_eq!(carried, ["key"]);
    assert_eq!(d.carries[0].name, "Klíč");
}

// ---------- mini graph ----------

#[test]
fn mini_graph_includes_own_nodes_and_direct_neighbors_only() {
    let d = destiny_of(&fixture(), "ann");
    let mut ids: Vec<&str> = d.graph.nodes.iter().map(|n| n.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, ["escape", "hint", "hint2", "search"]);
}

#[test]
fn mini_graph_marks_decisions_only_on_decision_nodes() {
    let d = destiny_of(&fixture(), "ann");
    assert!(node(&d.graph, "escape").decision);
    assert!(!node(&d.graph, "search").decision);
    assert!(!node(&d.graph, "hint").decision);
}

#[test]
fn mini_graph_places_boxes_in_columns_with_padding_and_gaps() {
    // Boxes are 130 × 34, columns 46 apart, rows 14 apart, 8 of padding around everything.
    let d = destiny_of(&fixture(), "ann");
    let g = &d.graph;
    let at = |id: &str| {
        let n = node(g, id);
        (n.x, n.y, n.w, n.h)
    };
    assert_eq!(at("hint"), (8.0, 8.0, 130.0, 34.0));
    assert_eq!(at("hint2"), (8.0, 56.0, 130.0, 34.0));
    // A one-box column is centered against the two-box column.
    assert_eq!(at("search"), (184.0, 32.0, 130.0, 34.0));
    assert_eq!(at("escape"), (360.0, 32.0, 130.0, 34.0));
    assert_eq!((g.width, g.height), (498.0, 98.0));
}

#[test]
fn mini_graph_edges_run_from_right_edge_to_left_edge_at_mid_height() {
    let d = destiny_of(&fixture(), "ann");
    let g = &d.graph;
    let mut edges: Vec<(f64, f64, f64, f64)> =
        g.edges.iter().map(|e| (e.x1, e.y1, e.x2, e.y2)).collect();
    edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        edges,
        [
            (138.0, 25.0, 184.0, 49.0),
            (138.0, 73.0, 184.0, 49.0),
            (314.0, 49.0, 360.0, 49.0),
        ]
    );
}

#[test]
fn mini_graph_fits_every_box_inside_its_size() {
    let d = destiny_of(&fixture(), "ann");
    let g = &d.graph;
    let pad = g.nodes.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
    assert!(pad > 0.0);
    let right = g.nodes.iter().map(|n| n.x + n.w).fold(0.0, f64::max);
    let bottom = g.nodes.iter().map(|n| n.y + n.h).fold(0.0, f64::max);
    let top = g.nodes.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
    assert_eq!(g.width, right + pad);
    assert_eq!(g.height, bottom + pad);
    assert_eq!(top, pad);
}

#[test]
fn mini_graph_of_a_character_without_nodes_is_one_empty_box_in_size() {
    let d = destiny_of(&fixture(), "ann");
    let reference = &d.graph;
    let pad = reference.nodes[0].x;
    let (w, h) = (reference.nodes[0].w, reference.nodes[0].h);

    let mut files = vec![("game.toml", GAME)];
    files.extend(PEOPLE);
    let lonely = destiny_of(&game(&files), "ann");
    assert!(lonely.graph.nodes.is_empty());
    assert!(lonely.graph.edges.is_empty());
    assert_eq!(lonely.graph.width, w + 2.0 * pad);
    assert_eq!(lonely.graph.height, h + 2.0 * pad);
}

#[test]
fn mini_graph_labels_own_nodes_and_shortens_long_titles_by_characters() {
    let d = destiny_of(&fixture(), "ann");
    // 21 characters: cut to 17 and an ellipsis.
    assert_eq!(node(&d.graph, "search").label, "Prohledat celou k…");
    // Exactly 18 characters (more bytes, thanks to the diacritics) stay whole.
    assert_eq!(node(&d.graph, "escape").label, "Útěk přes kryptu č");
    assert_eq!(
        node(&d.graph, "hint").label,
        "",
        "other people's nodes have no label"
    );
}

/// Two own, unconnected actions at the given `x` positions (and different `y`).
fn two_columns(x_a: &str, x_b: &str) -> Destiny {
    let layout = format!("[node.a]\nx = {x_a}\ny = 0\n\n[node.b]\nx = {x_b}\ny = 100\n");
    let mut files = vec![("game.toml", GAME), ("layout.toml", layout.as_str())];
    files.extend(PEOPLE);
    files.extend([
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"ann\"]\n",
        ),
        (
            "actions/b.toml",
            "[action]\ntitle = \"B\"\nowners = [\"ann\"]\n",
        ),
    ]);
    destiny_of(&game(&files), "ann")
}

fn same_column(d: &Destiny) -> bool {
    let (a, b) = (node(&d.graph, "a"), node(&d.graph, "b"));
    a.x == b.x && a.y != b.y
}

#[test]
fn mini_graph_keeps_nodes_with_the_same_x_in_one_column() {
    assert!(same_column(&two_columns("300", "300")));
}

#[test]
fn mini_graph_treats_x_within_one_pixel_as_one_column() {
    assert!(same_column(&two_columns("300", "300.5")));
    assert!(same_column(&two_columns("300", "301")));
}

#[test]
fn mini_graph_splits_distant_x_into_columns_left_to_right() {
    let d = two_columns("300", "302");
    let (a, b) = (node(&d.graph, "a"), node(&d.graph, "b"));
    assert!(a.x < b.x);
    assert_eq!(a.y, b.y);
}

#[test]
fn mini_graph_splits_columns_on_either_side_of_zero() {
    let d = two_columns("-200", "100");
    let (a, b) = (node(&d.graph, "a"), node(&d.graph, "b"));
    assert!(a.x < b.x);
}
