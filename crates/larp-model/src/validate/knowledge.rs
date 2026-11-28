//! Spoiler check: a text a person reads must not name, through `[[id]]`, an info they don't know
//! yet. Organizers know the whole plot and are not checked.
//!
//! - Their sheet (goals, secrets, sections, relations, blocks shown to them, their factions' goals,
//!   secrets and sections) is read before the game, so it may only name info they start with.
//! - The brief of one of their own nodes may also name info they learn at that node, or surely
//!   learn (not only after some decision option) at a node in an earlier segment. Without segments
//!   there is no time, so any such node counts.

use std::collections::HashSet;

use super::person_file;
use crate::knowledge::{self, How, Learn};
use crate::model::{Game, Role};
use crate::text;
use crate::{Diagnostic, Id};

pub(super) fn check(game: &Game, diags: &mut Vec<Diagnostic>) {
    let infos: HashSet<&Id> = game.info.iter().map(|i| i.id.get_ref()).collect();
    if infos.is_empty() {
        return;
    }
    let learns = knowledge::knowledge(game);
    // `learns` is sorted by person, so each person's part is one run.
    for mine in learns.chunk_by(|a, b| a.person == b.person) {
        let Some(first) = mine.first() else {
            continue;
        };
        let id = &first.person;
        check_person(game, &infos, id, mine, diags);
    }
    // People who learn nothing at all still have sheets and briefs to check.
    let learning: HashSet<&Id> = learns.iter().map(|l| &l.person).collect();
    for id in game.people.keys() {
        if !learning.contains(id) {
            check_person(game, &infos, id, &[], diags);
        }
    }
}

fn check_person(
    game: &Game,
    infos: &HashSet<&Id>,
    id: &Id,
    mine: &[Learn],
    diags: &mut Vec<Diagnostic>,
) {
    let Some(person) = game.people.get(id) else {
        return;
    };
    if person.role == Role::Organizer {
        return;
    }
    let start = knowledge::known_at_start(game, id);
    let own_file = person_file(id);
    // Every text on the sheet, with the file it is written in.
    let mut sheet: Vec<(String, &str)> = person
        .goals
        .iter()
        .chain(&person.secrets)
        .chain(person.sheet.values())
        .chain(person.relations.iter().map(|r| &r.text))
        .map(|t| (own_file.clone(), t.as_str()))
        .collect();
    sheet.extend(
        game.blocks
            .iter()
            .filter(|b| game.involves(&b.show, id))
            .map(|b| ("blocks.toml".to_owned(), b.text.as_str())),
    );
    sheet.extend(
        game.factions
            .iter()
            .filter(|f| {
                person
                    .factions
                    .iter()
                    .any(|r| r.get_ref() == f.id.get_ref())
            })
            .flat_map(|f| f.goals.iter().chain(&f.secrets).chain(f.sheet.values()))
            .map(|t| ("factions.toml".to_owned(), t.as_str())),
    );
    let mut named: Vec<Id> = Vec::new();
    for (file, t) in sheet {
        for info in linked_info(game, infos, t) {
            if !start.contains(&info) && !named.contains(&info) {
                diags.push(spoiler(game, file.clone(), id, &info, "sheet", mine));
                named.push(info);
            }
        }
    }
    let timeless = game.segments.is_empty();
    for (node_id, node) in &game.nodes {
        if !game.involves(node.owners(), id) {
            continue;
        }
        let at = crate::time::node(game, node).map(|s| s.start);
        for info in linked_info(game, infos, node.brief()) {
            let known = start.contains(&info)
                || mine.iter().any(|l| {
                    l.info == info
                        && (l.node.as_ref() == Some(node_id)
                            || (l.option.is_none()
                                && match (l.at, at) {
                                    (Some(learned), Some(now)) => learned < now,
                                    _ => timeless,
                                }))
                });
            if !known {
                let place = format!("brief of “{}”", node.title());
                diags.push(spoiler(
                    game,
                    format!("actions/{node_id}.toml"),
                    id,
                    &info,
                    &place,
                    mine,
                ));
            }
        }
    }
}

/// Info named in `t` by `[[id]]`, also through the blocks it includes.
fn linked_info(game: &Game, infos: &HashSet<&Id>, t: &str) -> Vec<Id> {
    let mut out = Vec::new();
    let mut seen: HashSet<Id> = HashSet::new();
    let mut todo: Vec<String> = vec![t.to_owned()];
    while let Some(t) = todo.pop() {
        for link in text::links(&t) {
            if infos.contains(&link.id) {
                if !out.contains(&link.id) {
                    out.push(link.id);
                }
            } else if seen.insert(link.id.clone())
                && let Some(b) = game.blocks.iter().find(|b| b.id.get_ref() == &link.id)
            {
                todo.push(b.text.clone());
            }
        }
    }
    out
}

fn spoiler(
    game: &Game,
    file: String,
    person: &Id,
    info: &Id,
    place: &str,
    mine: &[Learn],
) -> Diagnostic {
    let later = || {
        mine.iter()
            .filter(|l| &l.info == info && l.how != How::Start)
    };
    let first = later()
        .find(|l| l.option.is_none())
        .or_else(|| later().next());
    let help = match first.and_then(|l| l.node.as_ref().map(|n| (l, n))) {
        Some((l, node)) => {
            let title = game.nodes.get(node).map_or(node.as_str(), |n| n.title());
            let how = if l.how == How::Finds {
                "finds it out"
            } else {
                "is told"
            };
            let after = l
                .option
                .as_ref()
                .map(|o| format!(" after choosing “{o}”"))
                .unwrap_or_default();
            format!(
                "`{person}` {how} only at “{title}”{after}; remove the link, or give them the info earlier"
            )
        }
        None => format!(
            "`{person}` never learns it; make it `starting` with `{person}` among its owners, or remove the link"
        ),
    };
    Diagnostic::warning(
        file,
        None,
        format!("[[{info}]] in the {place} of `{person}` names info they don't know yet"),
    )
    .with_help(help)
}
