//! Links in game texts: `[[id]]` includes a block from `blocks.toml`, or shows the name of a
//! person, room, faction, item or piece of info.
//!
//! The model keeps texts as written. [`expand`] gives a copy of the game with every link
//! replaced, for sheets and the viewer; the editor keeps working on the original.

use std::ops::Range;

use crate::Id;
use crate::model::{Game, Node};

/// How deep blocks may include blocks; deeper (or a loop) is reported by `check` and left as is.
pub const MAX_DEPTH: usize = 16;

/// One `[[id]]` in a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Byte range of the whole `[[id]]`.
    pub range: Range<usize>,
    /// The id between the brackets.
    pub id: Id,
}

/// Every `[[id]]` in `text` whose content is a valid id; other brackets are ordinary text.
#[must_use]
pub fn links(text: &str) -> Vec<Link> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(start) = text[from..].find("[[").map(|i| i + from) {
        let inner = start + 2;
        let Some(end) = text[inner..].find("]]").map(|i| i + inner) else {
            break;
        };
        match Id::parse(&text[inner..end]) {
            Ok(id) => {
                out.push(Link {
                    range: start..end + 2,
                    id,
                });
                from = end + 2;
            }
            // `[[[x]]` or `[[ not an id ]]`: skip one bracket and look again.
            Err(_) => from = start + 1,
        }
    }
    out
}

/// Where a text lives, for diagnostics: the file and what the text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place<'a> {
    /// Game file, such as `actions/crypt.toml`.
    pub file: &'a str,
    /// Field, such as `brief` or `sheet`.
    pub field: &'static str,
}

/// Calls `f` with every text a link can appear in, together with its place.
pub fn each_text(game: &Game, mut f: impl FnMut(Place<'_>, &str)) {
    for (id, node) in &game.nodes {
        let file = format!("actions/{id}.toml");
        let (text, brief) = match node {
            Node::Action(a) => (&a.text, &a.brief),
            Node::Decision(d) => (&d.text, &d.brief),
        };
        f(place(&file, "text"), text);
        f(place(&file, "brief"), brief);
    }
    for (id, person) in &game.people {
        let file = format!("people/{id}.toml");
        for t in person.goals.iter().chain(&person.secrets) {
            f(place(&file, "goals / secrets"), t);
        }
        for t in person.sheet.values() {
            f(place(&file, "sheet"), t);
        }
        for r in &person.relations {
            f(place(&file, "relation"), &r.text);
        }
        for phase in &person.phases {
            let lists = phase.goals.iter().chain(&phase.secrets).flatten();
            for t in lists.chain(phase.sheet.values()) {
                f(place(&file, "phase"), t);
            }
        }
    }
    for thing in &game.items {
        f(place("items.toml", "text"), &thing.text);
    }
    for thing in &game.info {
        f(place("info.toml", "text"), &thing.text);
    }
    for segment in &game.segments {
        f(place("segments.toml", "text"), &segment.text);
    }
    for cycle in &game.cycles {
        f(place("cycles.toml", "text"), &cycle.text);
    }
    for block in &game.blocks {
        f(place("blocks.toml", "text"), &block.text);
    }
    for faction in &game.factions {
        f(place("factions.toml", "text"), &faction.text);
        let lists = faction.goals.iter().chain(&faction.secrets);
        for t in lists.chain(faction.sheet.values()) {
            f(place("factions.toml", "goals / secrets / sheet"), t);
        }
    }
}

fn place<'a>(file: &'a str, field: &'static str) -> Place<'a> {
    Place { file, field }
}

/// Calls `f` on every text a link can appear in, mutably (the same texts as [`each_text`]).
fn each_text_mut(game: &mut Game, mut f: impl FnMut(&mut String)) {
    for node in game.nodes.values_mut() {
        match node {
            Node::Action(a) => {
                f(&mut a.text);
                f(&mut a.brief);
            }
            Node::Decision(d) => {
                f(&mut d.text);
                f(&mut d.brief);
            }
        }
    }
    for person in game.people.values_mut() {
        person
            .goals
            .iter_mut()
            .chain(&mut person.secrets)
            .chain(person.sheet.values_mut())
            .chain(person.relations.iter_mut().map(|r| &mut r.text))
            .for_each(&mut f);
        for phase in &mut person.phases {
            phase
                .goals
                .iter_mut()
                .chain(&mut phase.secrets)
                .flatten()
                .chain(phase.sheet.values_mut())
                .for_each(&mut f);
        }
    }
    game.items
        .iter_mut()
        .chain(&mut game.info)
        .for_each(|t| f(&mut t.text));
    game.segments.iter_mut().for_each(|s| f(&mut s.text));
    game.cycles.iter_mut().for_each(|c| f(&mut c.text));
    game.blocks.iter_mut().for_each(|b| f(&mut b.text));
    for faction in &mut game.factions {
        std::iter::once(&mut faction.text)
            .chain(&mut faction.goals)
            .chain(&mut faction.secrets)
            .chain(faction.sheet.values_mut())
            .for_each(&mut f);
    }
}

/// What `[[id]]` stands for: a block's text (not yet expanded) or a display name.
#[must_use]
pub fn target<'a>(game: &'a Game, id: &Id) -> Option<&'a str> {
    if let Some(b) = game.blocks.iter().find(|b| b.id.get_ref() == id) {
        return Some(&b.text);
    }
    if let Some(p) = game.people.get(id) {
        return Some(&p.name);
    }
    game.rooms
        .iter()
        .map(|r| (&r.id, &r.name))
        .chain(game.factions.iter().map(|f| (&f.id, &f.name)))
        .chain(
            game.items
                .iter()
                .chain(&game.info)
                .map(|t| (&t.id, &t.name)),
        )
        .find(|(rid, _)| rid.get_ref() == id)
        .map(|(_, name)| name.as_str())
}

/// `text` with every link replaced; unknown links and too deep nesting stay as written.
#[must_use]
pub fn expand_text(game: &Game, text: &str) -> String {
    expand_at(game, text, 0)
}

fn expand_at(game: &Game, text: &str, depth: usize) -> String {
    let found = links(text);
    if found.is_empty() || depth >= MAX_DEPTH {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for link in found {
        out.push_str(&text[last..link.range.start]);
        match target(game, &link.id) {
            Some(t) => out.push_str(&expand_at(game, t, depth + 1)),
            None => out.push_str(&text[link.range.clone()]),
        }
        last = link.range.end;
    }
    out.push_str(&text[last..]);
    out
}

/// A copy of `game` with every link in every text replaced, for sheets and the viewer.
#[must_use]
pub fn expand(game: &Game) -> Game {
    let mut out = game.clone();
    each_text_mut(&mut out, |t| {
        if t.contains("[[") {
            *t = expand_text(game, t);
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<String> {
        links(text).into_iter().map(|l| l.id.to_string()).collect()
    }

    #[test]
    fn finds_links_with_valid_ids_only() {
        assert_eq!(ids("a [[datum]] b [[jiny-blok]]"), ["datum", "jiny-blok"]);
        assert_eq!(
            ids("[[Velké]] [[ s mezerou ]] [[]] [x] ]] [["),
            Vec::<String>::new()
        );
        assert_eq!(ids("[[[datum]]]"), ["datum"]);
        assert_eq!(ids("žluťoučký [[kůň]] [[kun]]"), ["kun"]);
    }

    #[test]
    fn link_ranges_cover_the_brackets() {
        let text = "x [[ab]] y";
        let l = &links(text)[0];
        assert_eq!(&text[l.range.clone()], "[[ab]]");
    }
}
