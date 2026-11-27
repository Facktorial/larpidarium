//! A character's destiny: their segment of the plot, for Part II of the character sheet.
//!
//! Other characters are never named: things that come from outside the character's own actions
//! are described as "you will receive / learn", and neighboring nodes in the mini graph have no
//! label.

use std::collections::{HashMap, HashSet};

use larp_model::model::{Node, Ref, Thing};
use larp_model::{Game, Id};
use serde::Serialize;

use crate::GraphData;

/// Where a needed thing comes from, from the character's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The character has it from the start.
    Carry,
    /// One of the character's own earlier actions gives it.
    Earlier,
    /// Someone or something else provides it.
    Receive,
}

/// A thing mentioned on the sheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThingRef {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub text: String,
    /// For needed or used things: where it comes from.
    pub source: Option<Source>,
}

/// One option of a decision the character makes.
#[derive(Debug, Clone, Serialize)]
pub struct DestinyOption {
    /// Label.
    pub label: String,
    /// Needed things (stay).
    pub needs: Vec<ThingRef>,
    /// Used-up things.
    pub uses: Vec<ThingRef>,
    /// Resulting things.
    pub gives: Vec<ThingRef>,
}

/// One of the character's own actions or decisions.
#[derive(Debug, Clone, Serialize)]
pub struct DestinyStep {
    /// Node id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Player-facing text.
    pub brief: String,
    /// Is a decision.
    pub decision: bool,
    /// Name of the segment the step belongs to, empty if none.
    pub segment: String,
    /// Needed things (stay).
    pub needs: Vec<ThingRef>,
    /// Used-up things.
    pub uses: Vec<ThingRef>,
    /// Resulting things.
    pub gives: Vec<ThingRef>,
    /// Options (decisions only).
    pub options: Vec<DestinyOption>,
}

/// Part II of a character sheet.
#[derive(Debug, Clone, Serialize)]
pub struct Destiny {
    /// Person id.
    pub person: String,
    /// Things the character has at the start.
    pub carries: Vec<ThingRef>,
    /// Own actions and decisions in time order.
    pub steps: Vec<DestinyStep>,
    /// The segment as a small graph.
    pub graph: MiniGraph,
}

/// A small graph ready to draw (SVG or `TikZ`), in pixels.
#[derive(Debug, Clone, Serialize)]
pub struct MiniGraph {
    /// Total width.
    pub width: f64,
    /// Total height.
    pub height: f64,
    /// Boxes.
    pub nodes: Vec<MiniNode>,
    /// Lines from the right edge of one box to the left edge of another.
    pub edges: Vec<MiniEdge>,
}

/// A box in the mini graph.
#[derive(Debug, Clone, Serialize)]
pub struct MiniNode {
    /// Node id.
    pub id: String,
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
    /// Short title; empty for other characters' nodes.
    pub label: String,
    /// Belongs to this character.
    pub own: bool,
    /// Is a decision.
    pub decision: bool,
}

/// A line in the mini graph.
#[derive(Debug, Clone, Serialize)]
pub struct MiniEdge {
    /// Start x.
    pub x1: f64,
    /// Start y.
    pub y1: f64,
    /// End x.
    pub x2: f64,
    /// End y.
    pub y2: f64,
}

const BOX_W: f64 = 130.0;
const BOX_H: f64 = 34.0;
const COL_GAP: f64 = 46.0;
const ROW_GAP: f64 = 14.0;
const PAD: f64 = 8.0;
const LABEL_CHARS: usize = 18;

/// Computes the destiny of `person`, using the node positions in `graph` for time order.
#[must_use]
pub fn destiny(game: &Game, graph: &GraphData, person: &Id) -> Destiny {
    let things: HashMap<&Id, &Thing> = game
        .items
        .iter()
        .chain(&game.info)
        .map(|t| (t.id.get_ref(), t))
        .collect();
    let owns = |refs: &[Ref]| refs.iter().any(|r| r.get_ref() == person);

    let carried: HashSet<&Id> = things
        .values()
        .filter(|t| t.starting && owns(&t.owners))
        .map(|t| t.id.get_ref())
        .collect();
    let mut carry_list: Vec<ThingRef> = carried
        .iter()
        .map(|id| thing_ref(&things, id, None))
        .collect();
    carry_list.sort_by(|a, b| a.name.cmp(&b.name));

    let position: HashMap<&str, (f64, f64)> = graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), crate::canvas_xy(graph, n)))
        .collect();
    let at = |id: &Id| position.get(id.as_str()).copied().unwrap_or((0.0, 0.0));
    let mut own: Vec<(&Id, &Node)> = game
        .nodes
        .iter()
        .filter(|(_, n)| owns(n.owners()))
        .collect();
    // Time first (world segment or cycle step), then the position in the graph.
    let tick = |n: &Node| larp_model::time::node(game, n).map_or(u32::MAX, |s| s.start);
    own.sort_by(|a, b| {
        let (pa, pb) = (at(a.0), at(b.0));
        tick(a.1)
            .cmp(&tick(b.1))
            .then(pa.0.total_cmp(&pb.0))
            .then(pa.1.total_cmp(&pb.1))
    });

    let own_ids: HashSet<&str> = own.iter().map(|(id, _)| id.as_str()).collect();
    Destiny {
        person: person.to_string(),
        carries: carry_list,
        steps: timeline(game, &own, &things, &carried),
        graph: mini_graph(game, graph, &own_ids),
    }
}

/// The character's own nodes as a timeline, with the source of every needed thing.
fn timeline<'a>(
    game: &Game,
    own: &[(&'a Id, &'a Node)],
    things: &HashMap<&Id, &Thing>,
    carried: &HashSet<&Id>,
) -> Vec<DestinyStep> {
    let mut earlier: HashSet<&Id> = HashSet::new();
    let mut steps = Vec::new();
    for (id, node) in own {
        let src = |r: &Ref, earlier: &HashSet<&Id>| {
            let tid = r.get_ref();
            if carried.contains(tid) {
                Source::Carry
            } else if earlier.contains(tid) {
                Source::Earlier
            } else {
                Source::Receive
            }
        };
        let needed = |refs: &[Ref], earlier: &HashSet<&Id>| -> Vec<ThingRef> {
            refs.iter()
                .map(|r| thing_ref(things, r.get_ref(), Some(src(r, earlier))))
                .collect()
        };
        let results = |refs: &[Ref]| -> Vec<ThingRef> {
            refs.iter()
                .map(|r| thing_ref(things, r.get_ref(), None))
                .collect()
        };
        let step = match node {
            Node::Action(a) => DestinyStep {
                id: id.to_string(),
                title: a.title.clone(),
                brief: a.brief.clone(),
                decision: false,
                segment: segment_name(game, node),
                needs: needed(&a.needs, &earlier),
                uses: needed(&a.uses, &earlier),
                gives: results(&a.gives),
                options: Vec::new(),
            },
            Node::Decision(d) => DestinyStep {
                id: id.to_string(),
                title: d.title.clone(),
                brief: d.brief.clone(),
                decision: true,
                segment: segment_name(game, node),
                needs: Vec::new(),
                uses: Vec::new(),
                gives: Vec::new(),
                options: d
                    .options
                    .iter()
                    .map(|o| DestinyOption {
                        label: o.label.clone(),
                        needs: needed(&o.needs, &earlier),
                        uses: needed(&o.uses, &earlier),
                        gives: results(&o.gives),
                    })
                    .collect(),
            },
        };
        let produced: Vec<&Id> = match node {
            Node::Action(a) => a.gives.iter().map(Ref::get_ref).collect(),
            Node::Decision(d) => d
                .options
                .iter()
                .flat_map(|o| &o.gives)
                .map(Ref::get_ref)
                .collect(),
        };
        earlier.extend(produced);
        steps.push(step);
    }
    steps
}

fn segment_name(game: &Game, node: &Node) -> String {
    if let Some((c, step)) = node.cycle() {
        let cycle = larp_model::time::find_cycle(game, c.get_ref())
            .map_or_else(|| c.get_ref().to_string(), |c| c.name.clone());
        let step = step.map(|s| *s.get_ref()).map_or_else(String::new, |k| {
            game.meta
                .steps
                .get(usize::from(k).saturating_sub(1))
                .map_or_else(|| format!(" · {k}"), |s| format!(" · {k}. {s}"))
        });
        return format!("{cycle}{step}");
    }
    node.segment()
        .and_then(|r| game.segments.iter().find(|s| s.id.get_ref() == r.get_ref()))
        .map(|s| s.name.clone())
        .unwrap_or_default()
}

fn thing_ref(things: &HashMap<&Id, &Thing>, id: &Id, source: Option<Source>) -> ThingRef {
    let t = things.get(id);
    ThingRef {
        id: id.to_string(),
        name: t.map_or_else(|| id.to_string(), |t| t.name.clone()),
        text: t.map(|t| t.text.clone()).unwrap_or_default(),
        source,
    }
}

/// Own nodes plus their direct neighbors, packed into a small left-to-right grid that keeps the
/// column order of the full graph.
fn mini_graph(game: &Game, graph: &GraphData, own: &HashSet<&str>) -> MiniGraph {
    let touches_own =
        |e: &&crate::Edge| own.contains(e.from.as_str()) || own.contains(e.to.as_str());
    let edges: Vec<&crate::Edge> = graph.edges.iter().filter(touches_own).collect();
    let mut included: HashSet<&str> = own.clone();
    for e in &edges {
        included.insert(&e.from);
        included.insert(&e.to);
    }

    let mut nodes: Vec<&crate::NodeView> = graph
        .nodes
        .iter()
        .filter(|n| included.contains(n.id.as_str()))
        .collect();
    let xy = |n: &crate::NodeView| crate::canvas_xy(graph, n);
    nodes.sort_by(|a, b| {
        let (pa, pb) = (xy(a), xy(b));
        pa.0.total_cmp(&pb.0).then(pa.1.total_cmp(&pb.1))
    });

    // Group by original column (x), keep vertical order inside a column.
    let mut columns: Vec<Vec<&crate::NodeView>> = Vec::new();
    let mut last_x = f64::NAN;
    for n in nodes {
        let x = xy(n).0;
        if columns.is_empty() || (x - last_x).abs() > 1.0 {
            columns.push(Vec::new());
            last_x = x;
        }
        if let Some(col) = columns.last_mut() {
            col.push(n);
        }
    }

    let tallest = columns.iter().map(Vec::len).max().unwrap_or(0);
    let column_height = |rows: usize| as_f64(rows) * (BOX_H + ROW_GAP) - ROW_GAP;
    let height = column_height(tallest).max(BOX_H) + 2.0 * PAD;
    let width = as_f64(columns.len()) * (BOX_W + COL_GAP) - COL_GAP + 2.0 * PAD;

    let mut boxes = Vec::new();
    let mut place: HashMap<&str, (f64, f64)> = HashMap::new();
    for (c, col) in columns.iter().enumerate() {
        let x = PAD + as_f64(c) * (BOX_W + COL_GAP);
        let mut y = PAD + (column_height(tallest) - column_height(col.len())) / 2.0;
        for n in col {
            let is_own = own.contains(n.id.as_str());
            place.insert(&n.id, (x, y));
            boxes.push(MiniNode {
                id: n.id.clone(),
                x,
                y,
                w: BOX_W,
                h: BOX_H,
                label: if is_own {
                    shorten(&n.title)
                } else {
                    String::new()
                },
                own: is_own,
                decision: game
                    .nodes
                    .iter()
                    .any(|(id, node)| id.as_str() == n.id && matches!(node, Node::Decision(_))),
            });
            y += BOX_H + ROW_GAP;
        }
    }

    let mut lines = Vec::new();
    let mut seen = HashSet::new();
    for e in edges {
        if !seen.insert((e.from.as_str(), e.to.as_str())) {
            continue;
        }
        if let (Some(&(x1, y1)), Some(&(x2, y2))) =
            (place.get(e.from.as_str()), place.get(e.to.as_str()))
        {
            lines.push(MiniEdge {
                x1: x1 + BOX_W,
                y1: y1 + BOX_H / 2.0,
                x2,
                y2: y2 + BOX_H / 2.0,
            });
        }
    }

    MiniGraph {
        width: width.max(BOX_W + 2.0 * PAD),
        height,
        nodes: boxes,
        edges: lines,
    }
}

fn shorten(title: &str) -> String {
    if title.chars().count() <= LABEL_CHARS {
        title.to_owned()
    } else {
        let mut s: String = title.chars().take(LABEL_CHARS - 1).collect();
        s.push('…');
        s
    }
}

fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}
