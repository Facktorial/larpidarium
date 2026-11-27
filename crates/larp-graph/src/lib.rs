#![doc = include_str!("../README.md")]

mod destiny;
mod layout;

pub use destiny::{
    Destiny, DestinyOption, DestinyStep, MiniEdge, MiniGraph, MiniNode, Source, ThingRef, destiny,
};

use std::collections::BTreeMap;

use larp_model::model::{Node, Point, Ref, Role, Thing};
use larp_model::{Diagnostic, Game, Id, Playthroughs, Severity};
use serde::Serialize;

/// Everything the browser needs to draw the graph. Serialized to `graph.json`.
#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    /// Game title.
    pub title: String,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Segments in display order.
    pub segments: Vec<SegmentView>,
    /// Item category ids, for the editor.
    pub categories: Vec<String>,
    /// People and rooms that can own nodes.
    pub owners: Vec<OwnerView>,
    /// Items and info that flow along the edges.
    pub things: Vec<ThingView>,
    /// Actions and decisions with their positions.
    pub nodes: Vec<NodeView>,
    /// Derived edges.
    pub edges: Vec<Edge>,
    /// Bend points per edge id, from `layout.toml`.
    pub knees: BTreeMap<String, Vec<Point>>,
    /// Warnings from `check`, for the "Game health" panel.
    pub findings: Vec<Finding>,
    /// Reachable endings with an example route; empty when the simulation did not run.
    pub endings: Vec<EndingView>,
    /// Number of distinct final states the simulation found.
    pub end_states: usize,
    /// The simulation stopped early.
    pub truncated: bool,
}

/// One problem reported by `check`.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// `"error"` or `"warning"`.
    pub severity: &'static str,
    /// What is wrong.
    pub message: String,
    /// How to fix it.
    pub help: Option<String>,
    /// File relative to the game folder.
    pub file: String,
    /// Affected node, when the problem is in an action file.
    pub node: Option<String>,
}

/// A reachable ending.
#[derive(Debug, Clone, Serialize)]
pub struct EndingView {
    /// Thing id.
    pub id: String,
    /// One sequence of choices that leads to it.
    pub route: Vec<String>,
}

/// A segment of the game.
#[derive(Debug, Clone, Serialize)]
pub struct SegmentView {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `#rrggbb`, or empty when the segment has no color.
    pub color: String,
    /// Description for organizers.
    pub text: String,
}

/// A person or room.
#[derive(Debug, Clone, Serialize)]
pub struct OwnerView {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"person"` or `"room"`.
    pub kind: &'static str,
    /// `"player"`, `"organizer"` or `"npc"` for people.
    pub role: Option<&'static str>,
    /// `#rrggbb`.
    pub color: String,
}

/// An item or a piece of info.
#[derive(Debug, Clone, Serialize)]
pub struct ThingView {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"item"` or `"info"`.
    pub kind: &'static str,
    /// Category color, if the item has a category with a color.
    pub color: Option<String>,
    /// Category id (items only).
    pub category: Option<String>,
    /// Owner ids (who has it at the start).
    pub owners: Vec<String>,
    /// Description.
    pub text: String,
    /// Exists at game start.
    pub starting: bool,
    /// Is an outcome of the game.
    pub ending: bool,
}

/// An action or decision.
#[derive(Debug, Clone, Serialize)]
pub struct NodeView {
    /// Id (file name).
    pub id: String,
    /// `"action"` or `"decision"`.
    pub kind: &'static str,
    /// Title.
    pub title: String,
    /// Description for organizers.
    pub text: String,
    /// Text for players.
    pub brief: String,
    /// Owner ids.
    pub owners: Vec<String>,
    /// Segment id, if the node belongs to one.
    pub segment: Option<String>,
    /// Needed thing ids (actions only).
    pub needs: Vec<String>,
    /// Used-up thing ids (actions only).
    pub uses: Vec<String>,
    /// Given thing ids (actions only).
    pub gives: Vec<String>,
    /// Options (decisions only).
    pub options: Vec<OptionView>,
    /// Left edge on the canvas.
    pub x: f64,
    /// Top edge on the canvas.
    pub y: f64,
}

/// One option of a decision.
#[derive(Debug, Clone, Serialize)]
pub struct OptionView {
    /// Label.
    pub label: String,
    /// Needed thing ids.
    pub needs: Vec<String>,
    /// Used-up thing ids.
    pub uses: Vec<String>,
    /// Given thing ids.
    pub gives: Vec<String>,
}

/// A line from the part of one node that gives things to the part of another that needs them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Edge {
    /// Stable id like `crypt-door.0>examine-bones`, used as key in `layout.toml`.
    pub id: String,
    /// Source node id.
    pub from: String,
    /// Source option index, `None` for an action's own outputs.
    pub from_port: Option<usize>,
    /// Target node id.
    pub to: String,
    /// Target option index, `None` for an action's own inputs.
    pub to_port: Option<usize>,
    /// Thing ids passed along this edge.
    pub items: Vec<String>,
    /// The edge connects two different segments (or a segment and an unsorted node).
    pub cross: bool,
}

/// One side of a node that gives or needs things: the action itself, or one decision option.
pub(crate) struct Port<'a> {
    pub(crate) node: &'a Id,
    pub(crate) index: Option<usize>,
    pub(crate) refs: Vec<&'a Ref>,
}

/// Builds the graph data for a checked game, with the findings and simulation result of `check`.
#[must_use]
pub fn build(
    game: &Game,
    diagnostics: &[Diagnostic],
    playthroughs: Option<&Playthroughs>,
) -> GraphData {
    let edges = edges(game);
    let positions = layout::positions(game, &edges);
    let nodes = game
        .nodes
        .iter()
        .map(|(id, node)| {
            let p = game
                .layout
                .node
                .get(id)
                .copied()
                .or_else(|| positions.get(id).copied())
                .unwrap_or(Point { x: 0.0, y: 0.0 });
            node_view(id, node, p)
        })
        .collect();

    GraphData {
        title: game.meta.title.clone(),
        language: match game.meta.language {
            larp_model::model::Language::Cs => "cs",
            larp_model::model::Language::En => "en",
        },
        segments: game
            .segments
            .iter()
            .map(|s| SegmentView {
                id: s.id.get_ref().to_string(),
                name: s.name.clone(),
                color: s
                    .color
                    .as_ref()
                    .map(|c| safe_color(c.get_ref()))
                    .unwrap_or_default(),
                text: s.text.clone(),
            })
            .collect(),
        categories: game
            .categories
            .iter()
            .map(|c| c.id.get_ref().to_string())
            .collect(),
        owners: owners(game),
        things: things(game),
        nodes,
        knees: game.layout.knees.clone(),
        edges,
        findings: diagnostics.iter().map(finding).collect(),
        endings: playthroughs
            .map(|p| {
                p.endings
                    .iter()
                    .map(|(id, route)| EndingView {
                        id: id.to_string(),
                        route: route.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        end_states: playthroughs.map_or(0, |p| p.end_states),
        truncated: playthroughs.is_some_and(|p| p.truncated),
    }
}

fn finding(d: &Diagnostic) -> Finding {
    Finding {
        severity: match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
        message: d.message.clone(),
        help: d.help.clone(),
        file: d.file.clone(),
        node: d
            .file
            .strip_prefix("actions/")
            .and_then(|f| f.strip_suffix(".toml"))
            .map(str::to_owned),
    }
}

/// Derives edges: one per (giving port, needing port) pair that shares at least one thing.
#[must_use]
pub fn edges(game: &Game) -> Vec<Edge> {
    let (sources, targets) = ports(game);
    let mut edges = Vec::new();
    for s in &sources {
        for t in targets.iter().filter(|t| t.node != s.node) {
            let items: Vec<String> = s
                .refs
                .iter()
                .filter(|r| t.refs.iter().any(|n| n.get_ref() == r.get_ref()))
                .map(|r| r.get_ref().to_string())
                .collect();
            if !items.is_empty() {
                edges.push(Edge {
                    id: format!("{}>{}", port_name(s), port_name(t)),
                    from: s.node.to_string(),
                    from_port: s.index,
                    to: t.node.to_string(),
                    to_port: t.index,
                    items,
                    cross: segment_of(game, s.node) != segment_of(game, t.node),
                });
            }
        }
    }
    edges
}

/// The node's segment if it is a declared one; unknown ids count as no segment, like in the
/// layout.
fn segment_of<'a>(game: &'a Game, node: &Id) -> Option<&'a Id> {
    game.nodes
        .get(node)
        .and_then(Node::segment)
        .map(Ref::get_ref)
        .filter(|id| game.segments.iter().any(|s| s.id.get_ref() == *id))
}

fn port_name(p: &Port<'_>) -> String {
    match p.index {
        Some(i) => format!("{}.{i}", p.node),
        None => p.node.to_string(),
    }
}

/// Giving ports and needing ports of all nodes.
pub(crate) fn ports(game: &Game) -> (Vec<Port<'_>>, Vec<Port<'_>>) {
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for (id, node) in &game.nodes {
        match node {
            Node::Action(a) => {
                sources.push(Port {
                    node: id,
                    index: None,
                    refs: a.gives.iter().collect(),
                });
                targets.push(Port {
                    node: id,
                    index: None,
                    refs: a.needs.iter().chain(&a.uses).collect(),
                });
            }
            Node::Decision(d) => {
                for (i, o) in d.options.iter().enumerate() {
                    sources.push(Port {
                        node: id,
                        index: Some(i),
                        refs: o.gives.iter().collect(),
                    });
                    targets.push(Port {
                        node: id,
                        index: Some(i),
                        refs: o.needs.iter().chain(&o.uses).collect(),
                    });
                }
            }
        }
    }
    (sources, targets)
}

fn ids(refs: &[Ref]) -> Vec<String> {
    refs.iter().map(|r| r.get_ref().to_string()).collect()
}

fn node_view(id: &Id, node: &Node, p: Point) -> NodeView {
    let (kind, text, needs, uses, gives, options) = match node {
        Node::Action(a) => (
            "action",
            &a.text,
            ids(&a.needs),
            ids(&a.uses),
            ids(&a.gives),
            Vec::new(),
        ),
        Node::Decision(d) => (
            "decision",
            &d.text,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            d.options
                .iter()
                .map(|o| OptionView {
                    label: o.label.clone(),
                    needs: ids(&o.needs),
                    uses: ids(&o.uses),
                    gives: ids(&o.gives),
                })
                .collect(),
        ),
    };
    NodeView {
        id: id.to_string(),
        kind,
        title: node.title().to_owned(),
        text: text.clone(),
        brief: node.brief().to_owned(),
        owners: ids(node.owners()),
        segment: node.segment().map(|r| r.get_ref().to_string()),
        needs,
        uses,
        gives,
        options,
        x: p.x,
        y: p.y,
    }
}

const DEFAULT_COLOR: &str = "#888888";

/// Colors go into CSS in the viewer; anything but `#rrggbb` is replaced by the default color.
/// (A valid game is already checked, but a draft shown by `serve` is not.)
fn safe_color(color: &str) -> String {
    let ok = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        color.to_owned()
    } else {
        DEFAULT_COLOR.to_owned()
    }
}

fn owners(game: &Game) -> Vec<OwnerView> {
    let people = game.people.iter().map(|(id, p)| OwnerView {
        id: id.to_string(),
        name: p.name.clone(),
        kind: "person",
        role: Some(match p.role {
            Role::Player => "player",
            Role::Organizer => "organizer",
            Role::Npc => "npc",
        }),
        color: p
            .color
            .as_ref()
            .map_or(DEFAULT_COLOR.to_owned(), |c| safe_color(c.get_ref())),
    });
    let rooms = game.rooms.iter().map(|r| OwnerView {
        id: r.id.get_ref().to_string(),
        name: r.name.clone(),
        kind: "room",
        role: None,
        color: r
            .color
            .as_ref()
            .map_or(DEFAULT_COLOR.to_owned(), |c| safe_color(c.get_ref())),
    });
    people.chain(rooms).collect()
}

fn things(game: &Game) -> Vec<ThingView> {
    let view = |t: &Thing, kind: &'static str| ThingView {
        id: t.id.get_ref().to_string(),
        name: t.name.clone(),
        kind,
        color: t.category.as_ref().and_then(|c| {
            game.categories
                .iter()
                .find(|cat| cat.id.get_ref() == c.get_ref())
                .and_then(|cat| cat.color.as_ref().map(|col| safe_color(col.get_ref())))
        }),
        category: t.category.as_ref().map(|c| c.get_ref().to_string()),
        owners: t.owners.iter().map(|o| o.get_ref().to_string()).collect(),
        text: t.text.clone(),
        starting: t.starting,
        ending: t.ending,
    };
    game.items
        .iter()
        .map(|t| view(t, "item"))
        .chain(game.info.iter().map(|t| view(t, "info")))
        .collect()
}
