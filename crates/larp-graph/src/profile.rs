//! A character's profile for organizers: everything about them in one place, with spoilers, and
//! a timeline of their life, phases and actions in world-time order.

use larp_model::knowledge::{self, How};
use larp_model::model::{Death, Node, Ref, Role};
use larp_model::{Game, Id, time};
use serde::Serialize;

/// One moment on a character's timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Moment {
    /// When: a segment name, or a cycle and step; empty when the node has no time.
    pub when: String,
    /// `"enters"`, `"phase"`, `"action"`, `"decision"` or `"dies"`.
    pub kind: &'static str,
    /// Node id (actions and decisions only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// Title of the node (empty for other moments).
    pub title: String,
    /// What the player reads (actions and decisions), or a phase's new goals.
    pub brief: String,
    /// Notes for organizers (actions and decisions), or a phase's new secrets.
    pub text: String,
    /// Other people, rooms and factions in it.
    pub with: Vec<String>,
    /// Decision options, by label.
    pub options: Vec<String>,
    /// Info the character finds out here (with option labels when only some options give it).
    pub finds: Vec<String>,
    /// Info the character is told here at the latest.
    pub told: Vec<String>,
    /// Items the node gives (with option labels when only some options give them).
    pub gets: Vec<String>,
    /// Items the node uses up (with option labels when only some options use them).
    pub uses: Vec<String>,
    /// First tick on the world timeline, if known.
    #[serde(skip)]
    pub at: Option<u32>,
    /// One past the last tick, if known (shorter moments come first at the same start).
    #[serde(skip)]
    pub end: Option<u32>,
}

/// Everything about one character, for their profile page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Profile {
    /// Person id.
    pub id: String,
    /// Name.
    pub name: String,
    /// `"player"`, `"organizer"` or `"npc"`.
    pub role: &'static str,
    /// Color as `#rrggbb`, or empty.
    pub color: String,
    /// Name of the player who plays them, or empty.
    pub played_by: String,
    /// Faction names.
    pub factions: Vec<String>,
    /// Goals.
    pub goals: Vec<String>,
    /// Secrets.
    pub secrets: Vec<String>,
    /// Sheet sections as (key, text).
    pub sheet: Vec<(String, String)>,
    /// Their relations as (other name, text).
    pub relations: Vec<(String, String)>,
    /// What others think of them, as (other name, text).
    pub seen_by: Vec<(String, String)>,
    /// Items and info they have when the game starts.
    pub starts_with: Vec<String>,
    /// Their life, phases and actions in world-time order.
    pub timeline: Vec<Moment>,
    /// Writing status, or empty.
    pub status: &'static str,
    /// Writer, or empty.
    pub writer: String,
}

/// The profile of `id`, or `None` if there is no such person.
#[must_use]
pub fn profile(game: &Game, id: &Id) -> Option<Profile> {
    let p = game.people.get(id)?;
    let name = |r: &Id| owner_name(game, r);
    let thing_name = |r: &Id| {
        game.items
            .iter()
            .chain(&game.info)
            .find(|t| t.id.get_ref() == r)
            .map_or_else(|| r.to_string(), |t| t.name.clone())
    };
    let starts_with = game
        .items
        .iter()
        .chain(&game.info)
        .filter(|t| t.starting && game.involves(&t.owners, id))
        .map(|t| t.name.clone())
        .collect();
    let seen_by = game
        .people
        .iter()
        .filter(|(other, _)| *other != id)
        .flat_map(|(_, o)| {
            o.relations
                .iter()
                .filter(|r| r.with.get_ref() == id)
                .map(move |r| (o.name.clone(), r.text.clone()))
        })
        .collect();
    let mut timeline = node_moments(game, id, &name, &thing_name);
    timeline.extend(life_moments(game, id));
    timeline.sort_by_key(|m| {
        let order = match m.kind {
            "enters" => 0,
            "phase" => 1,
            "dies" => 3,
            _ => 2,
        };
        (m.at.is_none(), m.at, order, m.end)
    });
    Some(Profile {
        id: id.to_string(),
        name: p.name.clone(),
        role: match p.role {
            Role::Player => "player",
            Role::Organizer => "organizer",
            Role::Npc => "npc",
        },
        // Checked by `check`; checked again because it goes into a `<style>` block.
        color: p
            .color
            .as_ref()
            .map(|c| c.get_ref().clone())
            .filter(|c| {
                c.len() == 7
                    && c.starts_with('#')
                    && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
            })
            .unwrap_or_default(),
        played_by: p
            .played_by
            .as_ref()
            .map(|r| {
                game.players
                    .iter()
                    .find(|x| x.id.get_ref() == r.get_ref())
                    .map_or_else(|| r.get_ref().to_string(), |x| x.name.clone())
            })
            .unwrap_or_default(),
        factions: p.factions.iter().map(|f| name(f.get_ref())).collect(),
        goals: p.goals.clone(),
        secrets: p.secrets.clone(),
        sheet: p
            .sheet
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        relations: p
            .relations
            .iter()
            .map(|r| (name(r.with.get_ref()), r.text.clone()))
            .collect(),
        seen_by,
        starts_with,
        timeline,
        status: p.status.map_or("", larp_model::model::Status::as_str),
        writer: p.writer.clone().unwrap_or_default(),
    })
}

/// The character's own actions and decisions (directly or through a faction).
fn node_moments(
    game: &Game,
    id: &Id,
    name: &dyn Fn(&Id) -> String,
    thing_name: &dyn Fn(&Id) -> String,
) -> Vec<Moment> {
    let learns: Vec<knowledge::Learn> = knowledge::knowledge(game)
        .into_iter()
        .filter(|l| &l.person == id)
        .collect();
    game.nodes
        .iter()
        .filter(|(_, n)| game.involves(n.owners(), id))
        .map(|(node_id, n)| {
            let options_total = match n {
                Node::Action(_) => 0,
                Node::Decision(d) => d.options.len(),
            };
            let learned = |how: How| {
                let found = learns
                    .iter()
                    .filter(|l| l.how == how && l.node.as_ref() == Some(node_id))
                    .map(|l| (l.info.clone(), l.option.clone()));
                labelled(found, options_total, thing_name)
            };
            let items = |pick: fn(&larp_model::model::DecisionOption) -> &Vec<Ref>,
                         plain: &[Ref]| {
                let found: Vec<(Id, Option<String>)> = match n {
                    Node::Action(_) => plain.iter().map(|r| (r.get_ref().clone(), None)).collect(),
                    Node::Decision(d) => d
                        .options
                        .iter()
                        .flat_map(|o| {
                            pick(o)
                                .iter()
                                .map(move |r| (r.get_ref().clone(), Some(o.label.clone())))
                        })
                        .collect(),
                };
                let found = found
                    .into_iter()
                    .filter(|(r, _)| game.items.iter().any(|t| t.id.get_ref() == r));
                labelled(found, options_total, thing_name)
            };
            let (gives, uses, options): (&[Ref], &[Ref], Vec<String>) = match n {
                Node::Action(a) => (&a.gives, &a.uses, Vec::new()),
                Node::Decision(d) => (
                    &[],
                    &[],
                    d.options.iter().map(|o| o.label.clone()).collect(),
                ),
            };
            Moment {
                when: crate::destiny::when(game, n),
                kind: match n {
                    Node::Action(_) => "action",
                    Node::Decision(_) => "decision",
                },
                node: Some(node_id.to_string()),
                title: n.title().to_owned(),
                brief: n.brief().to_owned(),
                text: match n {
                    Node::Action(a) => a.text.clone(),
                    Node::Decision(d) => d.text.clone(),
                },
                with: n
                    .owners()
                    .iter()
                    .filter(|o| o.get_ref() != id)
                    .map(|o| name(o.get_ref()))
                    .collect(),
                options,
                finds: learned(How::Finds),
                told: learned(How::Told),
                gets: items(|o| &o.gives, gives),
                uses: items(|o| &o.uses, uses),
                at: time::node(game, n).map(|s| s.start),
                end: time::node(game, n).map(|s| s.end),
            }
        })
        .collect()
}

/// Names of things found at a node, each once, in first-seen order. A thing that only some options
/// of a decision bring gets their labels, as "Dopis (Ukázat / Spálit)"; one that every option (or
/// the node itself) brings is just its name.
fn labelled(
    found: impl Iterator<Item = (Id, Option<String>)>,
    options_total: usize,
    thing_name: &dyn Fn(&Id) -> String,
) -> Vec<String> {
    let mut groups: Vec<(Id, Vec<Option<String>>)> = Vec::new();
    for (id, option) in found {
        match groups.iter_mut().find(|(g, _)| *g == id) {
            Some((_, list)) => list.push(option),
            None => groups.push((id, vec![option])),
        }
    }
    groups
        .into_iter()
        .map(|(id, options)| {
            let mut seen = std::collections::HashSet::new();
            let labels: Vec<String> = options
                .iter()
                .flatten()
                .filter(|l| seen.insert(*l))
                .cloned()
                .collect();
            if options.iter().any(Option::is_none) || labels.len() >= options_total {
                thing_name(&id)
            } else {
                format!("{} ({})", thing_name(&id), labels.join(" / "))
            }
        })
        .collect()
}

/// Entering the game, sheet phases and death.
fn life_moments(game: &Game, id: &Id) -> Vec<Moment> {
    let Some(p) = game.people.get(id) else {
        return Vec::new();
    };
    let life = time::life(game, id, p);
    let mut out = Vec::new();
    // Steps count in the character's own cycle.
    let step_when = |step: u8| {
        let name = game
            .meta
            .steps
            .get(usize::from(step).saturating_sub(1))
            .map_or_else(|| step.to_string(), |s| format!("{step}. {s}"));
        match time::hero_cycle(game, id) {
            Some(c) => format!("{} · {name}", c.name),
            None => name,
        }
    };
    let segment_name = |r: &Id| {
        game.segments
            .iter()
            .find(|s| s.id.get_ref() == r)
            .map_or_else(|| r.to_string(), |s| s.name.clone())
    };
    if let Some(enters) = &p.enters {
        let at = time::segment(game, enters.get_ref()).map(|s| s.start);
        out.push(moment(
            segment_name(enters.get_ref()),
            "enters",
            String::new(),
            at,
        ));
    } else if let Some(before) = &p.after {
        // Enters when the predecessor dies.
        let name = game
            .people
            .get(before.get_ref())
            .map_or_else(|| before.get_ref().to_string(), |b| b.name.clone());
        out.push(moment(
            format!("← {name}"),
            "enters",
            String::new(),
            life.map(|l| l.start),
        ));
    }
    for phase in &p.phases {
        let when = match (&phase.segment, &phase.step) {
            (Some(s), _) => segment_name(s.get_ref()),
            (None, Some(step)) => step_when(*step.get_ref()),
            (None, None) => String::new(),
        };
        // A phase changes the sheet: its new goals read as the brief, new secrets as the notes.
        let mut m = moment(when, "phase", String::new(), time::phase(game, id, phase));
        m.brief = phase
            .goals
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        m.text = phase
            .secrets
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        out.push(m);
    }
    if let Some(dies) = &p.dies {
        let when = match dies.get_ref() {
            Death::Segment(s) => segment_name(s),
            Death::Step(n) => step_when(*n),
        };
        out.push(moment(
            when,
            "dies",
            String::new(),
            life.map(|l| l.end.saturating_sub(1)),
        ));
    }
    out
}

fn moment(when: String, kind: &'static str, title: String, at: Option<u32>) -> Moment {
    Moment {
        when,
        kind,
        node: None,
        title,
        brief: String::new(),
        text: String::new(),
        with: Vec::new(),
        options: Vec::new(),
        finds: Vec::new(),
        told: Vec::new(),
        gets: Vec::new(),
        uses: Vec::new(),
        at,
        end: at.map(|a| a + 1),
    }
}

fn owner_name(game: &Game, id: &Id) -> String {
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
