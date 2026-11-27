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
