//! Balance between player characters: how much each one has to do, know and play with. It only
//! informs; nothing here is a warning, since a quiet character can be a deliberate choice.

use std::collections::BTreeSet;

use larp_model::knowledge::{self, How};
use larp_model::model::{Node, Person, Ref, Role};
use larp_model::{Game, Id, time};
use serde::Serialize;

/// Numbers for one player character.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Balance {
    /// Person id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Actions they take part in (directly or through a faction).
    pub actions: usize,
    /// Decisions they take part in.
    pub decisions: usize,
    /// Goals, their own and their factions'.
    pub goals: usize,
    /// Secrets, their own and their factions'.
    pub secrets: usize,
    /// Other people they have a relation to.
    pub relations_out: usize,
    /// Other people who have a relation to them.
    pub relations_in: usize,
    /// Other people they meet in their actions and decisions.
    pub meets: usize,
    /// Info they know when the game starts.
    pub knows_at_start: usize,
    /// Info they find out or are told during the game.
    pub learns: usize,
    /// Items they hold at the start or get during the game.
    pub items: usize,
    /// Words of their own sheet text: goals, secrets, sheet sections, relations and phases
    /// (not their factions' texts).
    pub words: usize,
    /// Segments they live in but have nothing to do in, by name; empty when any of their nodes
    /// has no time (it could be anywhere).
    pub idle: Vec<String>,
}

/// The balance numbers of every player character, in id order.
#[must_use]
pub fn balance(game: &Game) -> Vec<Balance> {
    let learns = knowledge::knowledge(game);
    game.people
        .iter()
        .filter(|(_, p)| p.role == Role::Player)
        .map(|(id, p)| {
            let own: Vec<&Node> = game
                .nodes
                .values()
                .filter(|n| game.involves(n.owners(), id))
                .collect();
            let factions = || {
                game.factions
                    .iter()
                    .filter(|f| p.factions.iter().any(|r| r.get_ref() == f.id.get_ref()))
            };
            let meets: BTreeSet<&Id> = own
                .iter()
                .flat_map(|n| n.owners())
                .flat_map(|o| {
                    let o = o.get_ref();
                    if game.people.contains_key(o) {
                        vec![o]
                    } else {
                        game.members(o).collect()
                    }
                })
                .filter(|o| *o != id)
                .collect();
            let items: BTreeSet<&Id> = game
                .items
                .iter()
                .filter(|t| t.starting && game.involves(&t.owners, id))
                .map(|t| t.id.get_ref())
                .chain(own.iter().flat_map(|n| gifts(n)))
                .filter(|r| game.items.iter().any(|t| t.id.get_ref() == *r))
                .collect();
            let start = knowledge::known_at_start(game, id);
            let learned: BTreeSet<&Id> = learns
                .iter()
                .filter(|l| &l.person == id && l.how != How::Start)
                .map(|l| &l.info)
                .filter(|i| !start.contains(i))
                .collect();
            let words = p
                .goals
                .iter()
                .chain(&p.secrets)
                .chain(p.sheet.values())
                .chain(p.relations.iter().map(|r| &r.text))
                .chain(p.phases.iter().flat_map(|ph| {
                    ph.goals
                        .iter()
                        .flatten()
                        .chain(ph.secrets.iter().flatten())
                        .chain(ph.sheet.values())
                }))
                .map(|s| s.split_whitespace().count())
                .sum();
            Balance {
                id: id.to_string(),
                name: p.name.clone(),
                actions: own.iter().filter(|n| matches!(n, Node::Action(_))).count(),
                decisions: own
                    .iter()
                    .filter(|n| matches!(n, Node::Decision(_)))
                    .count(),
                goals: p.goals.len() + factions().map(|f| f.goals.len()).sum::<usize>(),
                secrets: p.secrets.len() + factions().map(|f| f.secrets.len()).sum::<usize>(),
                relations_out: p
                    .relations
                    .iter()
                    .map(|r| r.with.get_ref())
                    .filter(|w| *w != id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                relations_in: game
                    .people
                    .iter()
                    .filter(|(other, _)| *other != id)
                    .filter(|(_, o)| o.relations.iter().any(|r| r.with.get_ref() == id))
                    .count(),
                meets: meets.len(),
                knows_at_start: start.len(),
                learns: learned.len(),
                items: items.len(),
                words,
                idle: idle(game, id, p, &own),
            }
        })
        .collect()
}

/// Everything a node can give, over all its options.
fn gifts(n: &Node) -> Vec<&Id> {
    match n {
        Node::Action(a) => a.gives.iter().map(Ref::get_ref).collect(),
        Node::Decision(d) => d
            .options
            .iter()
            .flat_map(|o| &o.gives)
            .map(Ref::get_ref)
            .collect(),
    }
}

/// Segments the person is alive in where none of their nodes happens.
fn idle(game: &Game, id: &Id, p: &Person, own: &[&Node]) -> Vec<String> {
    let Some(life) = time::life(game, id, p) else {
        return Vec::new();
    };
    // A node without a time could be in any segment, so idleness is unknown.
    let Some(spans) = own
        .iter()
        .map(|n| time::node(game, n))
        .collect::<Option<Vec<time::Span>>>()
    else {
        return Vec::new();
    };
    game.segments
        .iter()
        .filter_map(|s| time::segment(game, s.id.get_ref()).map(|span| (s, span)))
        .filter(|(_, span)| span.overlaps(life) && !spans.iter().any(|n| n.overlaps(*span)))
        .map(|(s, _)| s.name.clone())
        .collect()
}
