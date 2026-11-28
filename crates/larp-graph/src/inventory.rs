//! Inventory: where every item and handout is, who gets it during the game, and a packing list
//! for organizers.
//!
//! A holder has an item from the start (it is `starting` and they own it), or from the moment a
//! node they own gives it, until a node they own uses it up.

use larp_model::model::{Node, Ref, Thing};
use larp_model::{Game, Id};
use serde::Serialize;

/// A node where an item changes hands, with the names of its owners.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Move {
    /// Node id.
    pub node: String,
    /// Node title.
    pub title: String,
    /// The decision option, if only one option does it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
    /// Display names of the node's owners.
    pub owners: Vec<String>,
}

/// One item or info with a handout, as the Inventory tab and the packing list show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryItem {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"item"` or `"info"`.
    pub kind: &'static str,
    /// Category id, or empty.
    pub category: String,
    /// The physical prop, or empty.
    pub prop: String,
    /// How many props.
    pub quantity: u32,
    /// Room name where it lies, or empty.
    pub location: String,
    /// Display names of who has it at the start (empty unless `starting`).
    pub holders: Vec<String>,
    /// Nodes that give it.
    pub given: Vec<Move>,
    /// Nodes that need it (it stays).
    pub needed: Vec<Move>,
    /// Nodes that use it up.
    pub used: Vec<Move>,
    /// Handout text, or empty.
    pub handout: String,
}

/// A place on the packing list: a room or a holder, and what goes there before the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackingPlace {
    /// Room, person or faction name; empty for things that nobody holds and no room has.
    pub name: String,
    /// Members of a faction (the people who share what the faction holds), else empty.
    pub members: Vec<String>,
    /// What to put there.
    pub items: Vec<PackItem>,
}

/// One line of the packing list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackItem {
    /// Item name.
    pub name: String,
    /// The physical prop, or empty.
    pub prop: String,
    /// How many, for this place or holder.
    pub quantity: u32,
    /// True when it is a handout card to print.
    pub handout: bool,
    /// Room where the prop waits, or empty (shown for things handed out during the game).
    pub location: String,
    /// The decision option it depends on (during the game).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
    /// Titles of the acts that hand it out, for a prop that waits in a room before the game.
    pub for_acts: Vec<String>,
}

/// An act on the packing list: what its owners hand out during the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackingAct {
    /// Node title.
    pub title: String,
    /// Display names of whoever runs it.
    pub owners: Vec<String>,
    /// What it hands out.
    pub items: Vec<PackItem>,
}

/// The packing list: before the game by place, during the game by act.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Packing {
    /// What lies in each room or with each holder when the game starts, by name; things nobody
    /// holds come last, under an empty name.
    pub before: Vec<PackingPlace>,
    /// What each act hands out, in world-time order (acts without a time last, then by id).
    pub during: Vec<PackingAct>,
    /// Handout cards to print, as (name, text).
    pub handouts: Vec<(String, String)>,
}

/// Every item, and every info that has a handout or a prop, in file order.
#[must_use]
pub fn inventory(game: &Game) -> Vec<InventoryItem> {
    let items = game.items.iter().map(|t| (t, "item"));
    let info = game
        .info
        .iter()
        .filter(|t| t.handout.is_some() || t.prop.is_some())
        .map(|t| (t, "info"));
    items
        .chain(info)
        .map(|(t, kind)| InventoryItem {
            id: t.id.get_ref().to_string(),
            name: t.name.clone(),
            kind,
            category: t
                .category
                .as_ref()
                .map(|c| c.get_ref().to_string())
                .unwrap_or_default(),
            prop: t.prop.clone().unwrap_or_default(),
            quantity: t.count(),
            location: t
                .location
                .as_ref()
                .map(|r| name_of(game, r.get_ref()))
                .unwrap_or_default(),
            holders: if t.starting {
                t.owners
                    .iter()
                    .map(|o| name_of(game, o.get_ref()))
                    .collect()
            } else {
                Vec::new()
            },
            given: moves(game, t, |a| &a.gives, |o| &o.gives),
            needed: moves(game, t, |a| &a.needs, |o| &o.needs),
            used: moves(game, t, |a| &a.uses, |o| &o.uses),
            handout: t.handout.clone().unwrap_or_default(),
        })
        .collect()
}

/// The packing list of the whole game.
///
/// `quantity` is per place: a starting item with two holders and `quantity = 3` is three props
/// for each of them.
#[must_use]
pub fn packing(game: &Game) -> Packing {
    let acts = acts_in_time(game);
    // During the game: what each act gives, option by option.
    let during = acts
        .iter()
        .filter_map(|(_, n)| {
            let items: Vec<PackItem> = gives(n)
                .filter_map(|(g, option)| {
                    things(game)
                        .find(|t| t.id.get_ref() == g.get_ref())
                        .map(|t| PackItem {
                            option: option.map(str::to_owned),
                            ..pack(game, t)
                        })
                })
                .collect();
            (!items.is_empty()).then(|| PackingAct {
                title: n.title().to_owned(),
                owners: n
                    .owners()
                    .iter()
                    .map(|o| name_of(game, o.get_ref()))
                    .collect(),
                items,
            })
        })
        .collect();
    let handouts = things(game)
        .filter_map(|t| t.handout.as_ref().map(|h| (t.name.clone(), h.clone())))
        .collect();
    Packing {
        before: before_game(game, &acts),
        during,
        handouts,
    }
}

/// Items, then info that has a handout or a prop: everything that can be packed.
fn things(game: &Game) -> impl Iterator<Item = &Thing> {
    game.items.iter().chain(
        game.info
            .iter()
            .filter(|t| t.handout.is_some() || t.prop.is_some()),
    )
}

fn pack(game: &Game, t: &Thing) -> PackItem {
    PackItem {
        name: t.name.clone(),
        prop: t.prop.clone().unwrap_or_default(),
        quantity: t.count(),
        handout: t.handout.is_some(),
        location: t
            .location
            .as_ref()
            .map(|r| name_of(game, r.get_ref()))
            .unwrap_or_default(),
        option: None,
        for_acts: Vec::new(),
    }
}

/// What goes into each room or to each holder before the game, by place id (`None`: nobody
/// holds it): starting things, and props that wait in a room until an act hands them out.
fn before_game(game: &Game, acts: &[(&Id, &Node)]) -> Vec<PackingPlace> {
    let mut before: Vec<(Option<Id>, PackingPlace)> = Vec::new();
    let mut put = |place: Option<&Id>, item: PackItem| {
        let place = place.cloned();
        if let Some((_, p)) = before.iter_mut().find(|(id, _)| *id == place) {
            p.items.push(item);
            return;
        }
        let (name, members) = place.as_ref().map_or_else(
            || (String::new(), Vec::new()),
            |id| {
                let members = game
                    .members(id)
                    .filter_map(|m| game.people.get(m).map(|p| p.name.clone()))
                    .collect();
                (name_of(game, id), members)
            },
        );
        before.push((
            place,
            PackingPlace {
                name,
                members,
                items: vec![item],
            },
        ));
    };
    for t in things(game) {
        let room = t.location.as_ref().map(Ref::get_ref);
        if t.starting {
            match room {
                Some(room) => put(Some(room), pack(game, t)),
                None if t.owners.is_empty() => put(None, pack(game, t)),
                None => {
                    for o in &t.owners {
                        put(Some(o.get_ref()), pack(game, t));
                    }
                }
            }
        } else if let Some(room) = room {
            let mut item = pack(game, t);
            item.location = String::new();
            item.for_acts = acts
                .iter()
                .filter(|(_, n)| gives(n).any(|(g, _)| g.get_ref() == t.id.get_ref()))
                .map(|(_, n)| n.title().to_owned())
                .collect();
            put(Some(room), item);
        }
    }
    let mut before: Vec<PackingPlace> = before.into_iter().map(|(_, p)| p).collect();
    before.sort_by(|a, b| (a.name.is_empty(), &a.name).cmp(&(b.name.is_empty(), &b.name)));
    for p in &mut before {
        p.items.sort_by(|a, b| a.name.cmp(&b.name));
    }
    before
}

/// Nodes in world-time order; nodes without a time last, ties by id.
fn acts_in_time(game: &Game) -> Vec<(&Id, &Node)> {
    let mut acts: Vec<(Option<u32>, &Id, &Node)> = game
        .nodes
        .iter()
        .map(|(id, n)| (larp_model::time::node(game, n).map(|s| s.start), id, n))
        .collect();
    acts.sort_by_key(|(at, id, _)| (at.is_none(), *at, *id));
    acts.into_iter().map(|(_, id, n)| (id, n)).collect()
}

/// What a node gives, with the decision option each gift depends on.
fn gives(node: &Node) -> Box<dyn Iterator<Item = (&Ref, Option<&str>)> + '_> {
    match node {
        Node::Action(a) => Box::new(a.gives.iter().map(|g| (g, None))),
        Node::Decision(d) => Box::new(
            d.options
                .iter()
                .flat_map(|o| o.gives.iter().map(|g| (g, Some(o.label.as_str())))),
        ),
    }
}

type Refs<T> = fn(&T) -> &Vec<Ref>;

fn moves(
    game: &Game,
    thing: &Thing,
    action: Refs<larp_model::model::Action>,
    option: Refs<larp_model::model::DecisionOption>,
) -> Vec<Move> {
    let id = thing.id.get_ref();
    let mut out = Vec::new();
    for (node_id, node) in &game.nodes {
        let owners = || {
            node.owners()
                .iter()
                .map(|o| name_of(game, o.get_ref()))
                .collect()
        };
        match node {
            Node::Action(a) => {
                if action(a).iter().any(|r| r.get_ref() == id) {
                    out.push(Move {
                        node: node_id.to_string(),
                        title: a.title.clone(),
                        option: None,
                        owners: owners(),
                    });
                }
            }
            Node::Decision(d) => {
                for o in d
                    .options
                    .iter()
                    .filter(|o| option(o).iter().any(|r| r.get_ref() == id))
                {
                    out.push(Move {
                        node: node_id.to_string(),
                        title: d.title.clone(),
                        option: Some(o.label.clone()),
                        owners: owners(),
                    });
                }
            }
        }
    }
    out
}

/// Display name of a person, room or faction.
fn name_of(game: &Game, id: &Id) -> String {
    game.people.get(id).map_or_else(
        || {
            game.rooms
                .iter()
                .map(|r| (&r.id, &r.name))
                .chain(game.factions.iter().map(|f| (&f.id, &f.name)))
                .find(|(rid, _)| rid.get_ref() == id)
                .map_or_else(|| id.to_string(), |(_, name)| name.clone())
        },
        |p| p.name.clone(),
    )
}
