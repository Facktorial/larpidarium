//! Checks of `blocks.toml` and of `[[id]]` links in texts: every link resolves, blocks don't
//! include themselves, and every block is used somewhere.

use std::collections::{HashMap, HashSet};

use super::{Entry, Kind, closest};
use crate::model::Game;
use crate::text::{self, MAX_DEPTH};
use crate::{Diagnostic, Id};

/// The most bytes a block's text may grow to once its links are expanded; more is an error, so a
/// block including another many times over cannot blow up the sheets.
const MAX_EXPANDED: usize = 200_000;

/// What a `[[id]]` may point to.
const LINKABLE: &[Kind] = &[
    Kind::Block,
    Kind::Person,
    Kind::Room,
    Kind::Faction,
    Kind::Item,
    Kind::Info,
];

pub(super) fn check(game: &Game, registry: &HashMap<Id, Entry>, diags: &mut Vec<Diagnostic>) {
    let mut used: HashSet<Id> = HashSet::new();
    text::each_text(game, |place, t| {
        for link in text::links(t) {
            let id = &link.id;
            match registry.get(id) {
                Some(e) if LINKABLE.contains(&e.kind) => {
                    used.insert(id.clone());
                }
                Some(e) => diags.push(
                    Diagnostic::error(
                        place.file,
                        None,
                        format!("[[{id}]] in {} links to {}", place.field, e.kind),
                    )
                    .with_help(
                        "links can show a block, a person, a room, a faction, an item or an info",
                    ),
                ),
                None => {
                    let mut d = Diagnostic::error(
                        place.file,
                        None,
                        format!("unknown link [[{id}]] in {}", place.field),
                    );
                    let candidates = registry
                        .iter()
                        .filter(|(_, e)| LINKABLE.contains(&e.kind))
                        .map(|(k, _)| k);
                    d = match closest(id, candidates) {
                        Some(similar) => d.with_help(format!("did you mean [[{similar}]]?")),
                        None => d.with_help("blocks are defined in blocks.toml"),
                    };
                    diags.push(d);
                }
            }
        }
    });
    let mut sizes: HashMap<Id, usize> = HashMap::new();
    for block in &game.blocks {
        let id = block.id.get_ref();
        let size = expanded_len(game, id, &mut HashSet::new(), &mut sizes);
        if size > MAX_EXPANDED {
            diags.push(
                Diagnostic::error(
                    "blocks.toml",
                    Some(block.id.span()),
                    format!("block `{id}` grows to {size} bytes when its links are expanded"),
                )
                .with_help(format!(
                    "a block may expand to at most {MAX_EXPANDED} bytes; include big blocks fewer times"
                )),
            );
        }
        if block.show.is_empty() && !used.contains(id) {
            diags.push(
                Diagnostic::warning(
                    "blocks.toml",
                    Some(block.id.span()),
                    format!("block `{id}` is not used"),
                )
                .with_help("include it as [[id]] in a text, or list people in `show`"),
            );
        }
        if let Some(path) = loop_from(game, id) {
            diags.push(
                Diagnostic::error(
                    "blocks.toml",
                    Some(block.id.span()),
                    format!("block `{id}` includes itself: {}", path.join(" → ")),
                )
                .with_help("a block may include other blocks, but not in a circle"),
            );
        } else if let Some(path) = too_deep(game, id) {
            diags.push(
                Diagnostic::error(
                    "blocks.toml",
                    Some(block.id.span()),
                    format!(
                        "blocks include each other too deep from `{id}`: {}",
                        path.join(" → ")
                    ),
                )
                .with_help(format!(
                    "a chain of blocks may be at most {} long; the rest stays as [[id]]",
                    MAX_DEPTH - 1
                )),
            );
        }
    }
}

/// Blocks that the text of block `id` includes.
fn includes(game: &Game, id: &Id) -> Vec<Id> {
    game.blocks
        .iter()
        .find(|b| b.id.get_ref() == id)
        .map(|b| {
            text::links(&b.text)
                .into_iter()
                .map(|l| l.id)
                .filter(|l| game.blocks.iter().any(|b| b.id.get_ref() == l))
                .collect()
        })
        .unwrap_or_default()
}

/// The chain of blocks back to `start`, if `start` includes itself (directly or through others).
fn loop_from(game: &Game, start: &Id) -> Option<Vec<String>> {
    // Depth-first search for a path back to `start`, remembering the path.
    let mut stack: Vec<(Id, Vec<String>)> = vec![(start.clone(), vec![start.to_string()])];
    let mut seen: HashSet<Id> = HashSet::new();
    while let Some((id, path)) = stack.pop() {
        for next in includes(game, &id) {
            let mut longer = path.clone();
            longer.push(next.to_string());
            if &next == start {
                return Some(longer);
            }
            if seen.insert(next.clone()) {
                stack.push((next, longer));
            }
        }
    }
    None
}

/// The longest chain of included blocks from `start`, if expansion would stop before its end.
///
/// A link in a node's text is expanded at depth 1, so a chain may be at most `MAX_DEPTH - 1`
/// blocks long for the last block's own links (names) to be replaced too. Loops are reported by
/// [`loop_from`] and cut here.
fn too_deep(game: &Game, start: &Id) -> Option<Vec<String>> {
    let mut memo: HashMap<Id, Vec<String>> = HashMap::new();
    let longest = longest_chain(game, start, &mut HashSet::new(), &mut memo);
    (longest.len() >= MAX_DEPTH).then_some(longest)
}

/// How long the text of block `id` is once expanded, in bytes, saturating; blocks on `path`
/// (a loop, reported elsewhere) count as their link.
fn expanded_len(
    game: &Game,
    id: &Id,
    path: &mut HashSet<Id>,
    memo: &mut HashMap<Id, usize>,
) -> usize {
    if let Some(&known) = memo.get(id) {
        return known;
    }
    let Some(block) = game.blocks.iter().find(|b| b.id.get_ref() == id) else {
        return 0;
    };
    path.insert(id.clone());
    let mut len = block.text.len();
    for link in text::links(&block.text) {
        let is_block = game.blocks.iter().any(|b| b.id.get_ref() == &link.id);
        let replaced = if is_block && !path.contains(&link.id) {
            expanded_len(game, &link.id, path, memo)
        } else {
            text::target(game, &link.id).map_or(link.range.len(), str::len)
        };
        len = len
            .saturating_sub(link.range.len())
            .saturating_add(replaced);
    }
    path.remove(id);
    memo.insert(id.clone(), len);
    len
}

/// The longest chain of blocks starting at `id`, as ids; blocks already on the path end it.
fn longest_chain(
    game: &Game,
    id: &Id,
    path: &mut HashSet<Id>,
    memo: &mut HashMap<Id, Vec<String>>,
) -> Vec<String> {
    if let Some(known) = memo.get(id) {
        return known.clone();
    }
    path.insert(id.clone());
    let mut best: Vec<String> = Vec::new();
    for next in includes(game, id) {
        if path.contains(&next) {
            continue;
        }
        let chain = longest_chain(game, &next, path, memo);
        if chain.len() > best.len() {
            best = chain;
        }
    }
    path.remove(id);
    let mut out = vec![id.to_string()];
    out.extend(best);
    memo.insert(id.clone(), out.clone());
    out
}
