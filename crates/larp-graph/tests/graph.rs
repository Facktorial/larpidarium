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

#[test]
fn players_lives_and_segment_groups_are_exported() {
    let mut files = chain();
    files.push((
        "segments.toml",
        "[[segment]]\nid = \"one\"\nname = \"One\"\ngroup = \"Cycle 1\"\n\n[[segment]]\nid = \"two\"\nname = \"Two\"\n",
    ));
    files.push(("players.toml", "[[player]]\nid = \"p1\"\nname = \"Pat\"\n"));
    files.push((
        "people/ann.toml",
        "[person]\nname = \"Ann\"\nrole = \"player\"\nplayed_by = \"p1\"\nenters = \"one\"\ndies = \"two\"\noutside_cycle = true\n",
    ));
    let data = build_game(&game(&files));
    assert_eq!(data.players.len(), 1);
    assert_eq!(data.players[0].id, "p1");
    assert_eq!(data.players[0].name, "Pat");
    assert_eq!(data.segments[0].group.as_deref(), Some("Cycle 1"));
    assert_eq!(data.segments[1].group, None);
    let ann = data.owners.iter().find(|o| o.id == "ann").unwrap();
    assert_eq!(ann.played_by.as_deref(), Some("p1"));
    assert_eq!(ann.enters.as_deref(), Some("one"));
    assert_eq!(ann.dies.as_deref(), Some("two"));
    assert!(ann.outside_cycle);
}

// ---------- hero cycles (timeline layout) ----------

/// Three world segments, three steps (so 3 ticks per segment), and Ann's cycle over segments
/// "two" and "three" (ticks 3..9, 2 ticks per step). Ann dies at step `dies`. File names are
/// chosen so that alphabetical order differs from time order. `w-mid` starts at the same tick as
/// step 1, so it has no owner and does not appear in Ann's destiny.
fn cycled(dies: u8) -> Game {
    let action = |title: &str, place: &str| {
        format!("[action]\ntitle = \"{title}\"\nowners = [\"ann\"]\n{place}\n")
    };
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"Koloběh\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\", \"Odchod\"]\n"
                .to_owned(),
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"Noc 1\"\n\n[[segment]]\nid = \"two\"\nname = \"Noc 2\"\n\n[[segment]]\nid = \"three\"\nname = \"Noc 3\"\n"
                .to_owned(),
        ),
        (
            "players.toml",
            "[[player]]\nid = \"p1\"\nname = \"Pat\"\n".to_owned(),
        ),
        (
            "cycles.toml",
            "[[cycle]]\nid = \"c1\"\nname = \"Annin cyklus\"\nhero = \"ann\"\nfrom = \"two\"\nto = \"three\"\ncolor = \"#c0392b\"\n"
                .to_owned(),
        ),
        (
            "people/ann.toml",
            format!(
                "[person]\nname = \"Anna Řeháková\"\nrole = \"player\"\nplayed_by = \"p1\"\ndies = {dies}\n"
            ),
        ),
        ("actions/w-late.toml", action("Svět 3", "segment = \"three\"")),
        (
            "actions/w-mid.toml",
            "[action]\ntitle = \"Svět 2\"\nsegment = \"two\"\n".to_owned(),
        ),
        ("actions/w-early.toml", action("Svět 1", "segment = \"one\"")),
        ("actions/a-step3.toml", action("Krok 3", "cycle = \"c1\"\nstep = 3")),
        ("actions/b-step2.toml", action("Krok 2", "cycle = \"c1\"\nstep = 2")),
        ("actions/c-step1.toml", action("Krok 1", "cycle = \"c1\"\nstep = 1")),
    ];
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    game(&files)
}

fn node<'a>(graph: &'a GraphData, id: &str) -> &'a larp_graph::NodeView {
    graph.nodes.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn timeline_is_present_when_the_game_has_cycles() {
    let graph = build_game(&cycled(3));
    let t = graph.timeline.as_ref().unwrap();
    assert_eq!((t.unit, t.total), (3, 9));
    assert!(t.loose.is_none(), "every node has a segment or a cycle");
    assert_eq!(graph.steps, ["Ty", "Potřeba", "Odchod"]);
}

#[test]
fn timeline_is_absent_without_cycles() {
    let graph = build_game(&segmented());
    assert!(graph.timeline.is_none());
    assert!(graph.cycles.is_empty());
    assert!(
        graph.segments.iter().all(|s| s.x == 0.0 && s.width == 0.0),
        "band layout has no segment columns"
    );
}

#[test]
fn segment_columns_go_left_to_right_without_overlapping() {
    let graph = build_game(&cycled(3));
    let s = &graph.segments;
    assert_eq!(s.len(), 3);
    for pair in s.windows(2) {
        assert!(pair[0].width > 0.0);
        assert!(
            pair[0].x + pair[0].width <= pair[1].x,
            "{} must end before {} starts",
            pair[0].name,
            pair[1].name
        );
    }
}

#[test]
fn world_nodes_lie_inside_their_segment_column() {
    let graph = build_game(&cycled(3));
    for (node_id, seg) in [("w-early", 0), ("w-mid", 1), ("w-late", 2)] {
        let n = node(&graph, node_id);
        let col = &graph.segments[seg];
        assert_eq!(n.cycle, None);
        assert!(
            col.x <= n.x && n.x < col.x + col.width,
            "{node_id} at x={} is outside column {}..{}",
            n.x,
            col.x,
            col.x + col.width
        );
    }
    assert!(node(&graph, "w-early").x < node(&graph, "w-mid").x);
    assert!(node(&graph, "w-mid").x < node(&graph, "w-late").x);
}

#[test]
fn cycle_nodes_carry_cycle_and_step() {
    let graph = build_game(&cycled(3));
    for (node_id, step) in [("c-step1", 1), ("b-step2", 2), ("a-step3", 3)] {
        let n = node(&graph, node_id);
        assert_eq!(n.cycle.as_deref(), Some("c1"), "{node_id}");
        assert_eq!(n.step, Some(step), "{node_id}");
        assert_eq!(n.segment, None, "{node_id}");
    }
}

#[test]
fn cycle_nodes_are_placed_in_their_step_column_relative_to_the_cycle() {
    let graph = build_game(&cycled(3));
    let c = &graph.cycles[0];
    assert_eq!(c.step_columns.len(), 3);
    for pair in c.step_columns.windows(2) {
        assert!(pair[0].0 + pair[0].1 <= pair[1].0, "step columns overlap");
    }
    assert!(
        c.step_columns[0].0.abs() < f64::EPSILON,
        "relative to the cycle"
    );
    for (node_id, step) in [("c-step1", 0), ("b-step2", 1), ("a-step3", 2)] {
        let n = node(&graph, node_id);
        let (left, width) = c.step_columns[step];
        assert!(
            left <= n.x && n.x < left + width,
            "{node_id} at x={} is outside step column {left}..{}",
            n.x,
            left + width
        );
        assert!(n.x < c.inner_width, "{node_id} is relative to the cycle");
    }
    assert!(node(&graph, "c-step1").x < node(&graph, "b-step2").x);
    assert!(node(&graph, "b-step2").x < node(&graph, "a-step3").x);
}

#[test]
fn cycle_view_spans_its_segment_columns() {
    let graph = build_game(&cycled(3));
    assert_eq!(graph.cycles.len(), 1);
    let c = &graph.cycles[0];
    let (two, three) = (&graph.segments[1], &graph.segments[2]);
    assert_eq!(
        (c.id.as_str(), c.name.as_str(), c.hero.as_str()),
        ("c1", "Annin cyklus", "ann")
    );
    assert_eq!((c.from.as_str(), c.to.as_str()), ("two", "three"));
    assert!((c.x0 - two.x).abs() < f64::EPSILON, "x0 = start of “two”");
    assert!(
        (c.x1 - (three.x + three.width)).abs() < f64::EPSILON,
        "x1 = end of “three”"
    );
    assert_eq!(c.span, Some((3, 9)));
    assert_eq!(c.death_step, Some(3));
    assert_eq!(c.color, "#c0392b");
}

#[test]
fn owner_view_has_life_hero_of_and_death_step() {
    let graph = build_game(&cycled(3));
    let ann = graph.owners.iter().find(|o| o.id == "ann").unwrap();
    // step 3 of a cycle over ticks 3..9 is ticks 7..9
    assert_eq!(ann.life, Some((0, 9)));
    assert_eq!(ann.hero_of.as_deref(), Some("c1"));
    assert_eq!(ann.dies_step, Some(3));
    assert_eq!(ann.dies, None, "a step death is not a segment death");
}

#[test]
fn owner_view_life_ends_at_an_earlier_death_step() {
    let graph = build_game(&cycled(2));
    let ann = graph.owners.iter().find(|o| o.id == "ann").unwrap();
    // step 2 is ticks 5..7
    assert_eq!(ann.life, Some((0, 7)));
    assert_eq!(ann.dies_step, Some(2));
    assert_eq!(graph.cycles[0].death_step, Some(2));
}

#[test]
fn destiny_orders_cycle_steps_and_world_actions_by_time() {
    let g = cycled(3);
    let graph = build_game(&g);
    let d = larp_graph::destiny(&g, &graph, &larp_model::Id::parse("ann").unwrap());
    let ids: Vec<&str> = d.steps.iter().map(|s| s.id.as_str()).collect();
    // ticks: w-early 0, step 1 at 3, step 2 at 5, w-late 6, step 3 at 7
    assert_eq!(ids, ["w-early", "c-step1", "b-step2", "w-late", "a-step3"]);
}

#[test]
fn destiny_labels_cycle_steps_with_cycle_and_step_name() {
    let g = cycled(3);
    let graph = build_game(&g);
    let d = larp_graph::destiny(&g, &graph, &larp_model::Id::parse("ann").unwrap());
    let step2 = d.steps.iter().find(|s| s.id == "b-step2").unwrap();
    assert!(step2.segment.contains("Annin cyklus"), "{}", step2.segment);
    assert!(step2.segment.contains("Potřeba"), "{}", step2.segment);
    let world = d.steps.iter().find(|s| s.id == "w-late").unwrap();
    assert_eq!(world.segment, "Noc 3");
}

#[test]
fn map_is_exported() {
    let rooms = "[[room]]\nid = \"bar\"\nname = \"Bar\"\n";
    let map = "[map]\ncells = [\".=\"]\nwalls = [[0, 0, 4, 0]]\n\n[map.rooms]\nbar = { x = 2, y = 3, w = 4 }\n";
    let preset = "[map]\nwalls = [[1, 1, 1, 5]]\n";
    let g = game(&[
        ("game.toml", GAME),
        ("rooms.toml", rooms),
        ("map.toml", map),
        ("map-presets/sal.toml", preset),
    ]);
    let data = build_game(&g);
    assert_eq!(data.map_presets["sal"].walls, vec![[1, 1, 1, 5]]);
    assert!(data.map_presets["sal"].rooms.is_empty());
    let view = data.map.unwrap();
    assert_eq!(view.cells, vec![".=".to_owned()]);
    assert_eq!(view.walls, vec![[0, 0, 4, 0]]);
    assert_eq!(view.rooms["bar"], (2, 3, 4, 1));
    let g = game(&[("game.toml", GAME), ("rooms.toml", rooms)]);
    assert!(build_game(&g).map.is_none());
}
