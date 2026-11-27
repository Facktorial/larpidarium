//! Turning the files of a game folder into a [`Game`].

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::model::{
    Action, Category, Cycle, Decision, Game, GameMeta, Layout, Map, Node, Person, Player, Room,
    Segment, Thing,
};
use crate::playthrough::Playthroughs;
use crate::validate::validate;
use crate::{Diagnostic, Id, Severity};

/// One file of a game folder, already read into memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Path relative to the game folder with `/` separators, such as `actions/crypt-door.toml`.
    pub path: String,
    /// The file contents.
    pub text: String,
}

/// Result of [`load`]: the game if it has no errors, and every problem found.
#[derive(Debug, Clone)]
pub struct Checked {
    /// `Some` exactly when `diagnostics` contains no errors.
    pub game: Option<Game>,
    /// The game as far as it could be read, when it has errors (then `game` is `None`).
    ///
    /// Files that failed to parse are missing from it and it may contain dangling references.
    /// Meant for editors that must keep showing a game while it is being fixed; never use it to
    /// generate output for players.
    pub draft: Option<Game>,
    /// Errors and warnings, errors first, then in file order.
    pub diagnostics: Vec<Diagnostic>,
    /// Result of simulating every playthrough; `None` when errors prevented the simulation.
    pub playthroughs: Option<Playthroughs>,
}

impl Checked {
    /// Number of diagnostics with the given severity.
    #[must_use]
    pub fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GameFile {
    game: GameMeta,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeFile {
    action: Option<Action>,
    decision: Option<Decision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonFile {
    person: Person,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemsFile {
    #[serde(default)]
    category: Vec<Category>,
    #[serde(default)]
    item: Vec<Thing>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomsFile {
    #[serde(default)]
    room: Vec<Room>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapFile {
    map: Map,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CyclesFile {
    #[serde(default)]
    cycle: Vec<Cycle>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayersFile {
    #[serde(default)]
    player: Vec<Player>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SegmentsFile {
    #[serde(default)]
    segment: Vec<Segment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InfoFile {
    #[serde(default)]
    info: Vec<Thing>,
}

/// Parses and validates all files of a game folder.
///
/// Recognized paths: `game.toml` (required), `items.toml`, `rooms.toml`, `info.toml`, `segments.toml`, `players.toml`, `cycles.toml`, `map.toml`,
/// `actions/<id>.toml`, `people/<id>.toml` and `layout.toml`.
/// Any other path produces a warning.
#[must_use]
pub fn load(files: &[SourceFile]) -> Checked {
    let mut diags = Vec::new();
    let mut meta = None;
    let mut nodes = BTreeMap::new();
    let mut people = BTreeMap::new();
    let mut rooms = Vec::new();
    let mut categories = Vec::new();
    let mut items = Vec::new();
    let mut info = Vec::new();
    let mut layout = Layout::default();
    let mut segments = Vec::new();
    let mut players = Vec::new();
    let mut cycles = Vec::new();
    let mut map = None;
    let mut map_presets = BTreeMap::new();

    for file in files {
        match file.path.as_str() {
            "game.toml" => meta = parse::<GameFile>(file, &mut diags).map(|f| f.game),
            "items.toml" => {
                if let Some(f) = parse::<ItemsFile>(file, &mut diags) {
                    categories = f.category;
                    items = f.item;
                }
            }
            "rooms.toml" => {
                if let Some(f) = parse::<RoomsFile>(file, &mut diags) {
                    rooms = f.room;
                }
            }
            "info.toml" => {
                if let Some(f) = parse::<InfoFile>(file, &mut diags) {
                    info = f.info;
                }
            }
            "map.toml" => {
                if let Some(f) = parse::<MapFile>(file, &mut diags) {
                    map = Some(f.map);
                }
            }
            "cycles.toml" => {
                if let Some(f) = parse::<CyclesFile>(file, &mut diags) {
                    cycles = f.cycle;
                }
            }
            "players.toml" => {
                if let Some(f) = parse::<PlayersFile>(file, &mut diags) {
                    players = f.player;
                }
            }
            "segments.toml" => {
                if let Some(f) = parse::<SegmentsFile>(file, &mut diags) {
                    segments = f.segment;
                }
            }
            "layout.toml" => {
                if let Some(l) = parse::<Layout>(file, &mut diags) {
                    layout = l;
                }
            }
            path => {
                if let Some(stem) = path
                    .strip_prefix("actions/")
                    .and_then(|p| p.strip_suffix(".toml"))
                {
                    if let Some(node) = load_node(file, stem, &mut diags) {
                        nodes.extend([node]);
                    }
                } else if let Some(stem) = path
                    .strip_prefix("map-presets/")
                    .and_then(|p| p.strip_suffix(".toml"))
                {
                    let id = file_id(file, stem, &mut diags);
                    if let (Some(id), Some(f)) = (id, parse::<MapFile>(file, &mut diags)) {
                        map_presets.insert(id, f.map);
                    }
                } else if let Some(stem) = path
                    .strip_prefix("people/")
                    .and_then(|p| p.strip_suffix(".toml"))
                {
                    let id = file_id(file, stem, &mut diags);
                    let person = parse::<PersonFile>(file, &mut diags);
                    if let (Some(id), Some(p)) = (id, person) {
                        people.insert(id, p.person);
                    }
                } else {
                    diags.push(
                        Diagnostic::warning(path, None, "unknown file, ignored")
                            .with_help("game files are game.toml, items.toml, rooms.toml, info.toml, segments.toml, players.toml, cycles.toml, map.toml, map-presets/*.toml, actions/*.toml and people/*.toml"),
                    );
                }
            }
        }
    }

    if meta.is_none() && !files.iter().any(|f| f.path == "game.toml") {
        diags.push(
            Diagnostic::error("game.toml", None, "game.toml is missing").with_help(
                "create game.toml with a [game] table containing `title` and `language`",
            ),
        );
    }

    let mut playthroughs = None;
    let game = match meta {
        Some(meta) => {
            let game = Game {
                meta,
                nodes,
                people,
                rooms,
                segments,
                players,
                cycles,
                map,
                map_presets,
                categories,
                items,
                info,
                layout,
            };
            if !has_errors(&diags) {
                playthroughs = validate(&game, &mut diags);
            }
            Some(game)
        }
        None => None,
    };

    diags.sort_by_key(|d| std::cmp::Reverse(d.severity));
    let (game, draft) = if has_errors(&diags) {
        (None, game)
    } else {
        (game, None)
    };
    Checked {
        game,
        draft,
        diagnostics: diags,
        playthroughs,
    }
}

fn has_errors(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.severity == Severity::Error)
}

fn parse<T: DeserializeOwned>(file: &SourceFile, diags: &mut Vec<Diagnostic>) -> Option<T> {
    match toml::from_str(&file.text) {
        Ok(value) => Some(value),
        Err(e) => {
            diags.push(Diagnostic::error(
                &file.path,
                e.span(),
                e.message().trim_end().to_owned(),
            ));
            None
        }
    }
}

fn file_id(file: &SourceFile, stem: &str, diags: &mut Vec<Diagnostic>) -> Option<Id> {
    match Id::parse(stem) {
        Ok(id) => Some(id),
        Err(e) => {
            diags.push(
                Diagnostic::error(
                    &file.path,
                    None,
                    format!("file name is not a valid id: {e}"),
                )
                .with_help("rename the file, e.g. `crypt-door.toml`"),
            );
            None
        }
    }
}

fn load_node(file: &SourceFile, stem: &str, diags: &mut Vec<Diagnostic>) -> Option<(Id, Node)> {
    let id = file_id(file, stem, diags);
    let parsed = parse::<NodeFile>(file, diags)?;
    let node = match (parsed.action, parsed.decision) {
        (Some(a), None) => Node::Action(a),
        (None, Some(d)) => Node::Decision(d),
        (Some(_), Some(_)) => {
            diags.push(Diagnostic::error(
                &file.path,
                None,
                "file has both [action] and [decision]; use one file for each",
            ));
            return None;
        }
        (None, None) => {
            diags.push(Diagnostic::error(
                &file.path,
                None,
                "file has neither an [action] nor a [decision] table",
            ));
            return None;
        }
    };
    Some((id?, node))
}
