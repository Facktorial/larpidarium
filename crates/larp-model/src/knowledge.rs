//! Who knows which piece of info, and from when.
//!
//! A person knows an info from the start when it is `starting` and they (or one of their factions)
//! own it. Later they learn it at one of their own actions or decisions: one that gives it (they
//! find it out) or one that needs it (someone tells them, at the latest then). Options of a
//! decision count with their label, because the person learns it only on that path.

use std::collections::HashSet;

use crate::Id;
use crate::model::{Game, Node, Ref};
use crate::time;

/// How a person comes to know an info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum How {
    /// They know it when the game starts.
    Start,
    /// One of their own nodes gives it.
    Finds,
    /// One of their own nodes needs it, so they are told by then.
    Told,
}

impl How {
    /// `"start"`, `"finds"` or `"told"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Finds => "finds",
            Self::Told => "told",
        }
    }
}

/// One way a person comes to know an info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Learn {
    /// The person.
    pub person: Id,
    /// The info.
    pub info: Id,
    /// How.
    pub how: How,
    /// The node where it happens (none for [`How::Start`]).
    pub node: Option<Id>,
    /// The decision option it depends on, if any.
    pub option: Option<String>,
    /// First tick of the node on the world timeline, when the game has segments.
    pub at: Option<u32>,
}

/// Every way every person comes to know every info: sorted by person, then info, then time
/// (start first, nodes without a time last), then how.
#[must_use]
pub fn knowledge(game: &Game) -> Vec<Learn> {
    let infos: HashSet<&Id> = game.info.iter().map(|i| i.id.get_ref()).collect();
    let mut out = Vec::new();
    for person in game.people.keys() {
        for info in &game.info {
            if info.starting && game.involves(&info.owners, person) {
                out.push(Learn {
                    person: person.clone(),
                    info: info.id.get_ref().clone(),
                    how: How::Start,
                    node: None,
                    option: None,
                    at: Some(0),
                });
            }
        }
        for (node_id, node) in &game.nodes {
            if !game.involves(node.owners(), person) {
                continue;
            }
            let at = time::node(game, node).map(|s| s.start);
            let mut add = |refs: &[Ref], how: How, option: Option<&str>| {
                for r in refs.iter().filter(|r| infos.contains(r.get_ref())) {
                    out.push(Learn {
                        person: person.clone(),
                        info: r.get_ref().clone(),
                        how,
                        node: Some(node_id.clone()),
                        option: option.map(str::to_owned),
                        at,
                    });
                }
            };
            match node {
                Node::Action(a) => {
                    add(&a.needs, How::Told, None);
                    add(&a.gives, How::Finds, None);
                }
                Node::Decision(d) => {
                    for o in &d.options {
                        add(&o.needs, How::Told, Some(&o.label));
                        add(&o.gives, How::Finds, Some(&o.label));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| {
        (&a.person, &a.info, a.at.is_none(), a.at, a.how, &a.node).cmp(&(
            &b.person,
            &b.info,
            b.at.is_none(),
            b.at,
            b.how,
            &b.node,
        ))
    });
    out.dedup();
    out
}

/// Info that `person` knows when the game starts.
#[must_use]
pub fn known_at_start<'a>(game: &'a Game, person: &Id) -> HashSet<&'a Id> {
    game.info
        .iter()
        .filter(|i| i.starting && game.involves(&i.owners, person))
        .map(|i| i.id.get_ref())
        .collect()
}
