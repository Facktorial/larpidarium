//! Checks that need the whole game: references, duplicate ids, plot holes, colors.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Range;

use toml::Spanned;

use crate::model::{Game, Node, Ref, Role, Thing};
use crate::playthrough::{self, Playthroughs};
use crate::{Diagnostic, Id, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Action,
    Person,
    Room,
    Category,
    Segment,
    Item,
    Info,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Action => "an action",
            Self::Person => "a person",
            Self::Room => "a room",
            Self::Category => "a category",
            Self::Segment => "a segment",
            Self::Item => "an item",
            Self::Info => "an info",
        })
    }
}

struct Entry {
    kind: Kind,
    file: String,
}

/// One action, or one option of a decision, as seen by the plot-flow checks.
struct Step<'a> {
    file: String,
    needs: &'a [Ref],
    uses: &'a [Ref],
    gives: &'a [Ref],
}

const OWNER: &[Kind] = &[Kind::Person, Kind::Room];
const THING: &[Kind] = &[Kind::Item, Kind::Info];

/// Runs all checks. Returns the playthrough summary when the game had no errors to simulate.
pub(crate) fn validate(game: &Game, diags: &mut Vec<Diagnostic>) -> Option<Playthroughs> {
    let registry = register_ids(game, diags);
    let steps = steps(game);

    for (id, node) in &game.nodes {
        let file = node_file(id);
        check_refs(node.owners(), &file, OWNER, "owner", &registry, diags);
        if let Some(segment) = node.segment() {
            check_refs(
                std::slice::from_ref(segment),
                &file,
                &[Kind::Segment],
                "segment",
                &registry,
                diags,
            );
        }
    }
    for step in &steps {
        check_refs(step.needs, &step.file, THING, "need", &registry, diags);
        check_refs(
            step.uses,
            &step.file,
            &[Kind::Item],
            "use",
            &registry,
            diags,
        );
        check_refs(step.gives, &step.file, THING, "gift", &registry, diags);
    }
    for (id, person) in &game.people {
        let relations: Vec<Ref> = person.relations.iter().map(|r| r.with.clone()).collect();
        check_refs(
            &relations,
            &person_file(id),
            &[Kind::Person],
            "relation",
            &registry,
            diags,
        );
    }
    for (thing, file) in things(game) {
        check_refs(&thing.owners, file, OWNER, "owner", &registry, diags);
        if let Some(category) = &thing.category {
            check_refs(
                std::slice::from_ref(category),
                file,
                &[Kind::Category],
                "category",
                &registry,
                diags,
            );
        }
    }

    check_flow(game, &steps, diags);
    check_colors(game, diags);
    check_people(game, diags);
    for id in game
        .layout
        .node
        .keys()
        .filter(|id| !game.nodes.contains_key(*id))
    {
        diags.push(Diagnostic::warning(
            "layout.toml",
            None,
            format!("position for unknown action `{id}`, ignored"),
        ));
    }

    // Level 3 only makes sense once every reference resolves and every need has a source.
    if diags.iter().any(|d| d.severity == Severity::Error) {
        None
    } else {
        Some(playthrough::check(game, diags))
    }
}

/// Levels 4 and 5: every player has something to do and a complete sheet.
fn check_people(game: &Game, diags: &mut Vec<Diagnostic>) {
    for (id, person) in game.people.iter().filter(|(_, p)| p.role == Role::Player) {
        let file = person_file(id);
        let owned: Vec<&Node> = game
            .nodes
            .values()
            .filter(|n| n.owners().iter().any(|o| o.get_ref() == id))
            .collect();
        if owned.is_empty() {
            diags.push(
                Diagnostic::warning(
                    &file,
                    None,
                    "player owns no action or decision; their destiny would be empty",
                )
                .with_help(format!(
                    "add \"{id}\" to `owners` of the actions this character takes part in"
                )),
            );
        } else if !owned.iter().any(|n| matches!(n, Node::Decision(_))) {
            diags.push(
                Diagnostic::warning(&file, None, "player has no decision of their own")
                    .with_help("players enjoy a game more when some choices are theirs"),
            );
        }
        for key in ["backstory", "whereabouts"] {
            if person.sheet.get(key).is_none_or(|v| v.trim().is_empty()) {
                diags.push(
                    Diagnostic::warning(
                        &file,
                        None,
                        format!("player sheet has no `{key}` (Part I of the character sheet)"),
                    )
                    .with_help(format!(
                        "add `{key} = \"\"\"...\"\"\"` under [person.sheet]"
                    )),
                );
            }
        }
        if person.goals.is_empty() {
            diags.push(Diagnostic::warning(&file, None, "player has no goals"));
        }
    }
}

fn node_file(id: &Id) -> String {
    format!("actions/{id}.toml")
}

fn person_file(id: &Id) -> String {
    format!("people/{id}.toml")
}

fn things(game: &Game) -> impl Iterator<Item = (&Thing, &'static str)> {
    let items = game.items.iter().map(|t| (t, "items.toml"));
    let info = game.info.iter().map(|t| (t, "info.toml"));
    items.chain(info)
}

fn steps(game: &Game) -> Vec<Step<'_>> {
    let mut steps = Vec::new();
    for (id, node) in &game.nodes {
        match node {
            Node::Action(a) => steps.push(Step {
                file: node_file(id),
                needs: &a.needs,
                uses: &a.uses,
                gives: &a.gives,
            }),
            Node::Decision(d) => steps.extend(d.options.iter().map(|o| Step {
                file: node_file(id),
                needs: &o.needs,
                uses: &o.uses,
                gives: &o.gives,
            })),
        }
    }
    steps
}

fn register_ids(game: &Game, diags: &mut Vec<Diagnostic>) -> HashMap<Id, Entry> {
    let mut registry: HashMap<Id, Entry> = HashMap::new();
    let mut add = |id: &Id, kind: Kind, file: String, span: Option<Range<usize>>| {
        if let Some(existing) = registry.get(id) {
            diags.push(
                Diagnostic::error(
                    &file,
                    span,
                    format!(
                        "id `{id}` is already used by {} in {}",
                        existing.kind, existing.file
                    ),
                )
                .with_help("ids must be unique across actions, people, rooms, items and info"),
            );
        } else {
            registry.insert(id.clone(), Entry { kind, file });
        }
    };

    for id in game.nodes.keys() {
        add(id, Kind::Action, node_file(id), None);
    }
    for id in game.people.keys() {
        add(id, Kind::Person, person_file(id), None);
    }
    for room in &game.rooms {
        add(
            room.id.get_ref(),
            Kind::Room,
            "rooms.toml".into(),
            Some(room.id.span()),
        );
    }
    for segment in &game.segments {
        add(
            segment.id.get_ref(),
            Kind::Segment,
            "segments.toml".into(),
            Some(segment.id.span()),
        );
    }
    for category in &game.categories {
        add(
            category.id.get_ref(),
            Kind::Category,
            "items.toml".into(),
            Some(category.id.span()),
        );
    }
    for (thing, file) in things(game) {
        let kind = if file == "items.toml" {
            Kind::Item
        } else {
            Kind::Info
        };
        add(thing.id.get_ref(), kind, file.into(), Some(thing.id.span()));
    }
    registry
}

fn check_refs(
    refs: &[Ref],
    file: &str,
    allowed: &[Kind],
    role: &str,
    registry: &HashMap<Id, Entry>,
    diags: &mut Vec<Diagnostic>,
) {
    let expected = allowed
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" or ");
    for r in refs {
        let id = r.get_ref();
        match registry.get(id) {
            Some(entry) if allowed.contains(&entry.kind) => {}
            Some(entry) => diags.push(Diagnostic::error(
                file,
                Some(r.span()),
                format!("`{id}` is {}, but a {role} must be {expected}", entry.kind),
            )),
            None => {
                let mut d = Diagnostic::error(
                    file,
                    Some(r.span()),
                    format!("unknown {role} `{id}`, expected {expected}"),
                );
                let candidates = registry
                    .iter()
                    .filter(|(_, e)| allowed.contains(&e.kind))
                    .map(|(k, _)| k);
                if let Some(similar) = closest(id, candidates) {
                    d = d.with_help(format!("did you mean `{similar}`?"));
                }
                diags.push(d);
            }
        }
    }
}

fn check_flow(game: &Game, steps: &[Step<'_>], diags: &mut Vec<Diagnostic>) {
    let given: HashSet<&Id> = steps
        .iter()
        .flat_map(|s| s.gives)
        .map(Spanned::get_ref)
        .collect();
    let needed: HashSet<&Id> = steps
        .iter()
        .flat_map(|s| s.needs.iter().chain(s.uses))
        .map(Spanned::get_ref)
        .collect();
    let starting: HashSet<&Id> = things(game)
        .filter(|(t, _)| t.starting)
        .map(|(t, _)| t.id.get_ref())
        .collect();

    for (thing, file) in things(game) {
        let id = thing.id.get_ref();
        let available = starting.contains(id) || given.contains(id);
        if needed.contains(id) && !available {
            for step in steps {
                for r in step
                    .needs
                    .iter()
                    .chain(step.uses)
                    .filter(|r| r.get_ref() == id)
                {
                    diags.push(
                        Diagnostic::error(
                            &step.file,
                            Some(r.span()),
                            format!("`{id}` is needed here, but nothing gives it"),
                        )
                        .with_help(format!(
                            "add `{id}` to `gives` of some action, or set `starting = true` in {file}"
                        )),
                    );
                }
            }
        } else if available && !needed.contains(id) && !thing.ending {
            diags.push(
                Diagnostic::warning(
                    file,
                    Some(thing.id.span()),
                    format!("`{id}` is never needed by any action"),
                )
                .with_help("add it to some `needs`, or set `ending = true` if it is an outcome"),
            );
        } else if !available && !needed.contains(id) {
            diags.push(Diagnostic::warning(
                file,
                Some(thing.id.span()),
                format!("`{id}` is not used by any action"),
            ));
        }
    }
}

fn check_colors(game: &Game, diags: &mut Vec<Diagnostic>) {
    let mut check = |color: Option<&Spanned<String>>, file: &str| {
        if let Some(c) = color
            && !is_hex_color(c.get_ref())
        {
            diags.push(
                Diagnostic::error(
                    file,
                    Some(c.span()),
                    format!("`{}` is not a color", c.get_ref()),
                )
                .with_help("write colors as #rrggbb, e.g. \"#c0392b\""),
            );
        }
    };
    for (id, person) in &game.people {
        check(person.color.as_ref(), &person_file(id));
    }
    for room in &game.rooms {
        check(room.color.as_ref(), "rooms.toml");
    }
    for category in &game.categories {
        check(category.color.as_ref(), "items.toml");
    }
    for segment in &game.segments {
        check(segment.color.as_ref(), "segments.toml");
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// The candidate with the smallest edit distance to `id`, if it is close enough to be a typo.
fn closest<'a>(id: &Id, candidates: impl Iterator<Item = &'a Id>) -> Option<&'a Id> {
    let target = id.as_str();
    candidates
        .map(|c| (edit_distance(target, c.as_str()), c))
        .filter(|(d, c)| *d <= 2.max(c.as_str().len() / 4))
        .min_by(|(d1, c1), (d2, c2)| d1.cmp(d2).then_with(|| c1.cmp(c2)))
        .map(|(_, c)| c)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("rusty-key", "rusty-key"), 0);
        assert_eq!(edit_distance("rusty-kee", "rusty-key"), 1);
        assert_eq!(edit_distance("rusty-ke", "rusty-key"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn closest_ignores_unrelated_ids() {
        let ids: Vec<Id> = ["rusty-key", "sealed-letter"]
            .iter()
            .map(|s| Id::parse(s).unwrap())
            .collect();
        let typo = Id::parse("rusty-kee").unwrap();
        assert_eq!(closest(&typo, ids.iter()).unwrap().as_str(), "rusty-key");
        let unrelated = Id::parse("dragon").unwrap();
        assert!(closest(&unrelated, ids.iter()).is_none());
    }

    #[test]
    fn hex_colors() {
        assert!(is_hex_color("#c0392B"));
        assert!(!is_hex_color("c0392b"));
        assert!(!is_hex_color("#c039"));
        assert!(!is_hex_color("#gggggg"));
    }
}
