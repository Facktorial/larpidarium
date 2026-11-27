//! Automatic left-to-right layered layout, one horizontal band per segment.
//!
//! Inside a band, each node goes into a column one further right than every node of the same
//! band it receives things from, and nodes in a column are ordered by the average row of their
//! predecessors. Bands are stacked top to bottom in the order of `segments.toml`; nodes without a
//! segment form the last band. Edges between bands do not affect the layout.

use std::collections::{BTreeMap, HashMap};

use larp_model::model::{Node, Point, Ref};
use larp_model::{Game, Id};

use crate::Edge;

/// Horizontal distance between column starts (node width is 200 px in the viewer).
const COLUMN_WIDTH: f64 = 320.0;
/// Vertical gap between nodes in a column.
const ROW_GAP: f64 = 50.0;
/// Vertical gap between segment bands, including room for the band's title.
const BAND_GAP: f64 = 140.0;

/// Computes a position for every node.
pub(crate) fn positions<'a>(game: &'a Game, edges: &[Edge]) -> HashMap<&'a Id, Point> {
    // Band order: segments in file order, then nodes without (or with an unknown) segment.
    let band_of = |node: &Node| -> usize {
        node.segment()
            .and_then(|r| {
                game.segments
                    .iter()
                    .position(|s| s.id.get_ref() == r.get_ref())
            })
            .unwrap_or(game.segments.len())
    };
    let mut bands: BTreeMap<usize, Vec<&Id>> = BTreeMap::new();
    for (id, node) in &game.nodes {
        bands.entry(band_of(node)).or_default().push(id);
    }

    let mut out = HashMap::new();
    let mut top = 0.0;
    for ids in bands.values() {
        let (placed, height) = band(game, ids, edges);
        for (id, p) in placed {
            out.insert(
                id,
                Point {
                    x: p.x,
                    y: p.y + top,
                },
            );
        }
        top += height + BAND_GAP;
    }
    out
}

/// Lays out one band; returns positions relative to the band's top and the band's height.
fn band<'a>(game: &Game, ids: &[&'a Id], edges: &[Edge]) -> (Vec<(&'a Id, Point)>, f64) {
    let index: HashMap<&str, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let links: Vec<(usize, usize)> = edges
        .iter()
        .filter_map(|e| Some((*index.get(e.from.as_str())?, *index.get(e.to.as_str())?)))
        .collect();

    // Longest-path layering. Bounded by the node count so loops cannot run forever.
    let mut level = vec![0_usize; ids.len()];
    for _ in 0..ids.len() {
        let mut changed = false;
        for &(from, to) in &links {
            if level[to] <= level[from] && level[from] < ids.len() {
                level[to] = level[from] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut columns: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, &l) in level.iter().enumerate() {
        columns.entry(l).or_default().push(i);
    }

    // Order each column by the average row of its predecessors.
    let mut row = vec![0.0_f64; ids.len()];
    for nodes in columns.values_mut() {
        let key = |n: usize| {
            let preds: Vec<f64> = links
                .iter()
                .filter(|(_, to)| *to == n)
                .map(|(from, _)| row[*from])
                .collect();
            if preds.is_empty() {
                f64::MAX
            } else {
                preds.iter().sum::<f64>() / as_f64(preds.len())
            }
        };
        nodes.sort_by(|a, b| key(*a).total_cmp(&key(*b)));
        for (r, &n) in nodes.iter().enumerate() {
            row[n] = as_f64(r);
        }
    }

    // Stack nodes in each column and center columns against the tallest one.
    let heights: Vec<f64> = ids
        .iter()
        .map(|id| estimate_height(&game.nodes[*id]))
        .collect();
    let column_height =
        |nodes: &[usize]| nodes.iter().map(|&n| heights[n] + ROW_GAP).sum::<f64>() - ROW_GAP;
    let tallest = columns
        .values()
        .map(|c| column_height(c))
        .fold(0.0, f64::max);

    let mut out = Vec::new();
    for (&l, nodes) in &columns {
        let mut y = (tallest - column_height(nodes)) / 2.0;
        for &n in nodes {
            out.push((
                ids[n],
                Point {
                    x: as_f64(l) * COLUMN_WIDTH,
                    y,
                },
            ));
            y += heights[n] + ROW_GAP;
        }
    }
    (out, tallest)
}

/// Rough rendered height of a node in pixels; only needs to avoid overlaps.
fn estimate_height(node: &Node) -> f64 {
    let text_lines = |chars: usize| as_f64(chars.div_ceil(24).max(1));
    let list = |refs: &[Ref]| {
        if refs.is_empty() {
            0.0
        } else {
            let chars: usize = refs.iter().map(|r| r.get_ref().as_str().len() + 2).sum();
            18.0 * text_lines(chars + 6)
        }
    };
    let header = 22.0 + 18.0 + 20.0 * text_lines(node.title().chars().count()) + 10.0;
    match node {
        Node::Action(a) => header + list(&a.needs) + list(&a.uses) + list(&a.gives) + 10.0,
        Node::Decision(d) => {
            header
                + d.options
                    .iter()
                    .map(|o| 32.0 + list(&o.needs) + list(&o.uses) + list(&o.gives))
                    .sum::<f64>()
        }
    }
}

fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Width of an action card in the viewer.
const NODE_W: f64 = 200.0;
/// Horizontal gap between world-segment columns.
const SEGMENT_GAP: f64 = 120.0;
/// Narrowest world-segment column.
const MIN_SEGMENT_W: f64 = 360.0;
/// Gap between the step columns inside a cycle.
const STEP_GAP: f64 = 50.0;

/// Positions for a game with cycles: world segments as columns left to right (time), and every
/// cycle laid out on its own in step columns, relative to the cycle's top-left corner.
#[derive(Debug, Default)]
pub(crate) struct Timeline<'a> {
    /// World nodes: absolute; cycle nodes: relative to their cycle.
    pub positions: HashMap<&'a Id, Point>,
    /// Per world segment (file order): left edge and width of its column.
    pub columns: Vec<(f64, f64)>,
    /// Column for nodes without a segment or cycle, if there are any.
    pub loose: Option<(f64, f64)>,
    /// Height of the world area (above the cycle lanes).
    pub world_height: f64,
    /// Per cycle id: left edge of its time range, right edge, inner width and height.
    pub cycles: HashMap<&'a Id, CycleBox>,
    /// Per cycle id: left edge and width of each step column, relative to the cycle.
    pub steps: HashMap<&'a Id, Vec<(f64, f64)>>,
}

/// Where a cycle sits on the time axis and how big its expanded content is.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CycleBox {
    pub x0: f64,
    pub x1: f64,
    pub inner_width: f64,
    pub inner_height: f64,
}

/// Width of a band laid out by [`band`].
fn band_width(placed: &[(&Id, Point)]) -> f64 {
    placed.iter().map(|(_, p)| p.x + NODE_W).fold(0.0, f64::max)
}

pub(crate) fn timeline<'a>(game: &'a Game, edges: &[Edge]) -> Timeline<'a> {
    let mut out = Timeline::default();
    let mut world: BTreeMap<usize, Vec<&Id>> = BTreeMap::new();
    let mut in_cycle: HashMap<&Id, BTreeMap<u8, Vec<&Id>>> = HashMap::new();
    let loose_key = game.segments.len();
    for (id, node) in &game.nodes {
        if let Some((c, step)) = node.cycle()
            && game.cycles.iter().any(|x| x.id.get_ref() == c.get_ref())
        {
            let step = step.map_or(1, |s| *s.get_ref());
            in_cycle
                .entry(c.get_ref())
                .or_default()
                .entry(step)
                .or_default()
                .push(id);
            continue;
        }
        let key = node
            .segment()
            .and_then(|r| {
                game.segments
                    .iter()
                    .position(|s| s.id.get_ref() == r.get_ref())
            })
            .unwrap_or(loose_key);
        world.entry(key).or_default().push(id);
    }

    let mut x = 0.0;
    for i in 0..=game.segments.len() {
        let ids = world.get(&i).map(Vec::as_slice).unwrap_or_default();
        if i == loose_key && ids.is_empty() {
            break;
        }
        let (placed, height) = band(game, ids, edges);
        let width = band_width(&placed).max(MIN_SEGMENT_W);
        let offset = (width - band_width(&placed)) / 2.0;
        for (id, p) in placed {
            out.positions.insert(
                id,
                Point {
                    x: x + offset + p.x,
                    y: p.y,
                },
            );
        }
        out.world_height = out.world_height.max(height);
        if i == loose_key {
            out.loose = Some((x, width));
        } else {
            out.columns.push((x, width));
        }
        x += width + SEGMENT_GAP;
    }

    for cycle in &game.cycles {
        let id = cycle.id.get_ref();
        let column = |r: &Id| {
            game.segments
                .iter()
                .position(|s| s.id.get_ref() == r)
                .and_then(|i| out.columns.get(i).copied())
        };
        let from = column(cycle.from.get_ref()).unwrap_or((0.0, MIN_SEGMENT_W));
        let to = cycle
            .to
            .as_ref()
            .and_then(|t| column(t.get_ref()))
            .unwrap_or(from);
        let steps = in_cycle.remove(id).unwrap_or_default();
        let mut sx = 0.0;
        let mut height: f64 = 0.0;
        let mut cols = Vec::new();
        for k in 1..=u8::try_from(game.meta.steps.len().max(1)).unwrap_or(u8::MAX) {
            let ids = steps.get(&k).map(Vec::as_slice).unwrap_or_default();
            let (placed, h) = band(game, ids, edges);
            let w = band_width(&placed).max(NODE_W);
            for (nid, p) in placed {
                out.positions.insert(
                    nid,
                    Point {
                        x: sx + p.x,
                        y: p.y,
                    },
                );
            }
            cols.push((sx, w));
            height = height.max(h);
            sx += w + STEP_GAP;
        }
        out.steps.insert(id, cols);
        out.cycles.insert(
            id,
            CycleBox {
                x0: from.0,
                x1: to.0 + to.1,
                inner_width: (sx - STEP_GAP).max(0.0),
                inner_height: height,
            },
        );
    }
    out
}
