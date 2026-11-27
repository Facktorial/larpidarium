//! Organizer sheets: everything one organizer, non-player character or room runs, in time order,
//! and how busy they are. Unlike a player's destiny this is full of spoilers: it names who else
//! takes part and where every needed thing comes from.

use std::collections::{BTreeMap, HashMap};

use larp_model::model::{Node, Ref, Role, Thing};
use larp_model::{Game, Id};
use serde::Serialize;

/// How many actions and decisions an owner has, per world segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Load {
    /// All owned actions and decisions.
    pub total: usize,
    /// Of those, decisions.
    pub decisions: usize,
    /// Count per world segment, in segment order (segments with none are included as 0).
    pub per_segment: Vec<(String, usize)>,
    /// The busiest segment and its count; `None` without segments or nodes.
    pub peak: Option<(String, usize)>,
}

/// A thing on an organizer sheet, with who provides it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemRef {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub text: String,
    /// `"item"` or `"info"`.
    pub kind: &'static str,
    /// Names of whoever provides it: owners of the nodes that give it, or the starting owners.
    pub from: Vec<String>,
}

/// One option of a decision on an organizer sheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScheduleOption {
    /// Label.
    pub label: String,
    /// Needed and kept.
    pub needs: Vec<ItemRef>,
    /// Needed and used up.
    pub uses: Vec<ItemRef>,
    /// Produced.
    pub gives: Vec<ItemRef>,
}

/// One action or decision on an organizer sheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScheduleStep {
    /// Node id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Organizer text (`text`).
    pub text: String,
    /// Player text (`brief`).
    pub brief: String,
    /// Whether it is a decision.
    pub decision: bool,
    /// When, if more precise than the block: "cycle · step".
    pub when: String,
    /// The other owners, by name.
    pub with: Vec<String>,
    /// Needed and kept.
    pub needs: Vec<ItemRef>,
    /// Needed and used up.
    pub uses: Vec<ItemRef>,
    /// Produced.
    pub gives: Vec<ItemRef>,
    /// Decision options.
    pub options: Vec<ScheduleOption>,
}

/// The steps of one world segment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScheduleBlock {
    /// Segment name; empty for nodes outside any segment.
    pub segment: String,
    /// Steps in time order.
    pub steps: Vec<ScheduleStep>,
}

/// Everything an organizer sheet shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Schedule {
    /// Owner id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"person"` or `"room"`.
    pub kind: &'static str,
    /// `"organizer"`, `"npc"` or `"player"` for people, `None` for rooms.
    pub role: Option<&'static str>,
    /// `#rrggbb` or empty.
    pub color: String,
    /// How busy.
    pub load: Load,
    /// Items to have ready: what the owner starts with and the items its nodes hand out.
    pub prepare: Vec<ItemRef>,
    /// The owner's nodes by world segment.
    pub blocks: Vec<ScheduleBlock>,
}

/// Owners that get an organizer sheet: organizers, non-player characters and rooms, by id.
#[must_use]
pub fn schedule_owners(game: &Game) -> Vec<&Id> {
    let mut ids: Vec<&Id> = game
        .people
        .iter()
        .filter(|(_, p)| p.role != Role::Player)
        .map(|(id, _)| id)
        .chain(game.rooms.iter().map(|r| r.id.get_ref()))
        .collect();
    ids.sort();
    ids
}

/// Display name and load of every owner that gets an organizer sheet, in id order.
#[must_use]
pub fn loads(game: &Game) -> Vec<(String, Load)> {
    schedule_owners(game)
        .into_iter()
        .map(|id| (owner_name(game, id), load(game, &owned(game, id))))
        .collect()
}

/// The organizer sheet of `owner` (a person or a room).
#[must_use]
pub fn schedule(game: &Game, owner: &Id) -> Schedule {
    let things: HashMap<&Id, &Thing> = game
        .items
        .iter()
        .chain(&game.info)
        .map(|t| (t.id.get_ref(), t))
        .collect();
    let providers = providers(game);
    let item = |r: &Ref| item_ref(game, &things, &providers, r.get_ref());
    let items = |refs: &[Ref]| refs.iter().map(item).collect::<Vec<_>>();
    // What a step gives comes from this step itself, so "from" would only repeat its owners.
    let results = |refs: &[Ref]| {
        refs.iter()
            .map(|r| ItemRef {
                from: Vec::new(),
                ..item(r)
            })
            .collect::<Vec<_>>()
    };

    let nodes = owned(game, owner);
    let mut blocks: Vec<ScheduleBlock> = Vec::new();
    for (id, node) in &nodes {
        let segment = world_segment(game, node)
            .and_then(|i| game.segments.get(i))
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let with = node
            .owners()
            .iter()
            .map(Ref::get_ref)
            .filter(|o| *o != owner)
            .map(|o| owner_name(game, o))
            .collect();
        let when = if node.cycle().is_some() {
            crate::destiny::when(game, node)
        } else {
            String::new()
        };
        let step = match node {
            Node::Action(a) => ScheduleStep {
                id: id.to_string(),
                title: a.title.clone(),
                text: a.text.clone(),
                brief: a.brief.clone(),
                decision: false,
                when,
                with,
                needs: items(&a.needs),
                uses: items(&a.uses),
                gives: results(&a.gives),
                options: Vec::new(),
            },
            Node::Decision(d) => ScheduleStep {
                id: id.to_string(),
                title: d.title.clone(),
                text: d.text.clone(),
                brief: d.brief.clone(),
                decision: true,
                when,
                with,
                needs: Vec::new(),
                uses: Vec::new(),
                gives: Vec::new(),
                options: d
                    .options
                    .iter()
                    .map(|o| ScheduleOption {
                        label: o.label.clone(),
                        needs: items(&o.needs),
                        uses: items(&o.uses),
                        gives: results(&o.gives),
                    })
                    .collect(),
            },
        };
        match blocks.last_mut() {
            Some(b) if b.segment == segment => b.steps.push(step),
            _ => blocks.push(ScheduleBlock {
                segment,
                steps: vec![step],
            }),
        }
    }

    let (kind, role, color) = identity(game, owner);
    Schedule {
        id: owner.to_string(),
        name: owner_name(game, owner),
        kind,
        role,
        color: color.unwrap_or_default(),
        load: load(game, &nodes),
        prepare: prepare(game, owner, &nodes, &item),
        blocks,
    }
}

/// `"person"`/`"room"`, the person's role and the color.
fn identity(game: &Game, owner: &Id) -> (&'static str, Option<&'static str>, Option<String>) {
    match game.people.get(owner) {
        Some(p) => (
            "person",
            Some(match p.role {
                Role::Player => "player",
                Role::Organizer => "organizer",
                Role::Npc => "npc",
            }),
            p.color.as_ref().map(|c| c.get_ref().clone()),
        ),
        None => (
            "room",
            None,
            game.rooms
                .iter()
                .find(|r| r.id.get_ref() == owner)
                .and_then(|r| r.color.as_ref().map(|c| c.get_ref().clone())),
        ),
    }
}

/// The owner's nodes in time order (world segment or cycle step, then id).
fn owned<'a>(game: &'a Game, owner: &Id) -> Vec<(&'a Id, &'a Node)> {
    let mut nodes: Vec<(&Id, &Node)> = game
        .nodes
        .iter()
        .filter(|(_, n)| n.owners().iter().any(|r| r.get_ref() == owner))
        .collect();
    let tick = |n: &Node| larp_model::time::node(game, n).map_or(u32::MAX, |s| s.start);
    nodes.sort_by_key(|(id, n)| (tick(n), *id));
    nodes
}

/// Index of the world segment a node happens in (a cycle node: the segment its step falls in).
fn world_segment(game: &Game, node: &Node) -> Option<usize> {
    let span = larp_model::time::node(game, node)?;
    usize::try_from(span.start / larp_model::time::unit(game)).ok()
}

fn load(game: &Game, nodes: &[(&Id, &Node)]) -> Load {
    let mut per: Vec<(String, usize)> = game.segments.iter().map(|s| (s.name.clone(), 0)).collect();
    for (_, n) in nodes {
        if let Some(slot) = world_segment(game, n).and_then(|i| per.get_mut(i)) {
            slot.1 += 1;
        }
    }
    let peak = per
        .iter()
        .filter(|(_, c)| *c > 0)
        .fold(None::<&(String, usize)>, |best, x| match best {
            Some(b) if b.1 >= x.1 => Some(b),
            _ => Some(x),
        })
        .cloned();
    Load {
        total: nodes.len(),
        decisions: nodes
            .iter()
            .filter(|(_, n)| matches!(n, Node::Decision(_)))
            .count(),
        per_segment: per,
        peak,
    }
}

/// Who provides each thing: owners of nodes giving it, else its starting owners.
fn providers(game: &Game) -> BTreeMap<&Id, Vec<&Id>> {
    let mut map: BTreeMap<&Id, Vec<&Id>> = BTreeMap::new();
    for node in game.nodes.values() {
        let gives: Vec<&Ref> = match node {
            Node::Action(a) => a.gives.iter().collect(),
            Node::Decision(d) => d.options.iter().flat_map(|o| &o.gives).collect(),
        };
        for g in gives {
            let list = map.entry(g.get_ref()).or_default();
            for o in node.owners() {
                if !list.contains(&o.get_ref()) {
                    list.push(o.get_ref());
                }
            }
        }
    }
    for t in game.items.iter().chain(&game.info) {
        if t.starting {
            let list = map.entry(t.id.get_ref()).or_default();
            for o in &t.owners {
                if !list.contains(&o.get_ref()) {
                    list.push(o.get_ref());
                }
            }
        }
    }
    map
}

fn item_ref(
    game: &Game,
    things: &HashMap<&Id, &Thing>,
    providers: &BTreeMap<&Id, Vec<&Id>>,
    id: &Id,
) -> ItemRef {
    let t = things.get(id);
    ItemRef {
        id: id.to_string(),
        name: t.map_or_else(|| id.to_string(), |t| t.name.clone()),
        text: t.map(|t| t.text.clone()).unwrap_or_default(),
        kind: if game.info.iter().any(|i| i.id.get_ref() == id) {
            "info"
        } else {
            "item"
        },
        from: providers
            .get(id)
            .map(|v| v.iter().map(|o| owner_name(game, o)).collect())
            .unwrap_or_default(),
    }
}

/// Items (not info) the owner starts with or hands out, by name.
fn prepare(
    game: &Game,
    owner: &Id,
    nodes: &[(&Id, &Node)],
    item: &dyn Fn(&Ref) -> ItemRef,
) -> Vec<ItemRef> {
    let mut ids: Vec<&Ref> = game
        .items
        .iter()
        .filter(|t| t.starting && t.owners.iter().any(|o| o.get_ref() == owner))
        .map(|t| &t.id)
        .collect();
    for (_, n) in nodes {
        match n {
            Node::Action(a) => ids.extend(&a.gives),
            Node::Decision(d) => ids.extend(d.options.iter().flat_map(|o| &o.gives)),
        }
    }
    let mut out: Vec<ItemRef> = ids
        .into_iter()
        .filter(|r| game.items.iter().any(|t| t.id.get_ref() == r.get_ref()))
        .map(item)
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

fn owner_name(game: &Game, id: &Id) -> String {
    game.people.get(id).map_or_else(
        || {
            game.rooms
                .iter()
                .find(|r| r.id.get_ref() == id)
                .map_or_else(|| id.to_string(), |r| r.name.clone())
        },
        |p| p.name.clone(),
    )
}
