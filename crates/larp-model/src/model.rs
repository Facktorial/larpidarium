//! The game model: what a game folder means once it is parsed.
//!
//! References to other entities are [`Ref`]s: an [`Id`] together with its byte range in the
//! source file, so validation errors can point at the exact spot.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::Id;

/// An id that refers to another entity, with its position in the source file.
pub type Ref = Spanned<Id>;

/// A whole game: everything in one game folder.
#[derive(Debug, Clone)]
pub struct Game {
    /// Contents of `game.toml`.
    pub meta: GameMeta,
    /// Actions and decisions from `actions/*.toml`, keyed by file name.
    pub nodes: BTreeMap<Id, Node>,
    /// People from `people/*.toml`, keyed by file name.
    pub people: BTreeMap<Id, Person>,
    /// Rooms from `rooms.toml`.
    pub rooms: Vec<Room>,
    /// Segments (levels, acts, loosely connected parts) from `segments.toml`, in file order.
    pub segments: Vec<Segment>,
    /// Real participants from `players.toml`; characters name them in `played_by`.
    pub players: Vec<Player>,
    /// Hero cycles from `cycles.toml`, in file order.
    pub cycles: Vec<Cycle>,
    /// Tile map of the places from `map.toml`.
    pub map: Option<Map>,
    /// Saved floor plans from `map-presets/<id>.toml` (their `rooms` are ignored).
    pub map_presets: BTreeMap<Id, Map>,
    /// Item categories from `items.toml`.
    pub categories: Vec<Category>,
    /// Physical items from `items.toml`.
    pub items: Vec<Thing>,
    /// Pieces of information from `info.toml`.
    pub info: Vec<Thing>,
    /// Shared texts and facts from `blocks.toml`.
    pub blocks: Vec<Block>,
    /// Groups of people from `factions.toml`.
    pub factions: Vec<Faction>,
    /// Node positions and line bends saved from the browser (`layout.toml`).
    pub layout: Layout,
}

/// Contents of `layout.toml`, written by the browser's "Export layout" button.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    /// Pinned position of an action's top-left corner.
    #[serde(default)]
    pub node: BTreeMap<Id, Point>,
    /// Bend points of a line, keyed by edge id such as `crypt-door.0>examine-bones`.
    #[serde(default)]
    pub knees: BTreeMap<String, Vec<Point>>,
}

/// A position on the graph canvas, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    /// Distance from the left.
    pub x: f64,
    /// Distance from the top.
    pub y: f64,
}

/// Language of the game's texts; selects labels on generated pages and sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Czech.
    Cs,
    /// English.
    En,
}

/// The `[game]` table of `game.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameMeta {
    /// Name of the game.
    pub title: String,
    /// Language of the texts.
    pub language: Language,
    /// Names of the steps every cycle goes through (e.g. the 8 steps of the story circle).
    #[serde(default)]
    pub steps: Vec<String>,
}

/// One file in `actions/`: either a plain action or a decision.
#[derive(Debug, Clone)]
pub enum Node {
    /// Something that happens: needs some things, gives others.
    Action(Action),
    /// A choice whose options each need and give different things.
    Decision(Decision),
}

impl Node {
    /// Title shown on the graph node.
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Self::Action(a) => &a.title,
            Self::Decision(d) => &d.title,
        }
    }

    /// Segment the node belongs to, if any.
    #[must_use]
    pub fn segment(&self) -> Option<&Ref> {
        match self {
            Self::Action(a) => a.segment.as_ref(),
            Self::Decision(d) => d.segment.as_ref(),
        }
    }

    /// Cycle and step this node belongs to, if any.
    #[must_use]
    pub fn cycle(&self) -> Option<(&Ref, Option<&Spanned<u8>>)> {
        let (c, s) = match self {
            Self::Action(a) => (&a.cycle, &a.step),
            Self::Decision(d) => (&d.cycle, &d.step),
        };
        c.as_ref().map(|c| (c, s.as_ref()))
    }

    /// The `step` field, whether or not `cycle` is set.
    #[must_use]
    pub fn step(&self) -> Option<&Spanned<u8>> {
        match self {
            Self::Action(a) => a.step.as_ref(),
            Self::Decision(d) => d.step.as_ref(),
        }
    }

    /// Text for players.
    #[must_use]
    pub fn brief(&self) -> &str {
        match self {
            Self::Action(a) => &a.brief,
            Self::Decision(d) => &d.brief,
        }
    }

    /// People, rooms and factions responsible for this node.
    #[must_use]
    pub fn owners(&self) -> &[Ref] {
        match self {
            Self::Action(a) => &a.owners,
            Self::Decision(d) => &d.owners,
        }
    }

    /// Writing status.
    #[must_use]
    pub fn status(&self) -> Option<Status> {
        match self {
            Self::Action(a) => a.status,
            Self::Decision(d) => d.status,
        }
    }

    /// Who writes it.
    #[must_use]
    pub fn writer(&self) -> Option<&str> {
        match self {
            Self::Action(a) => a.writer.as_deref(),
            Self::Decision(d) => d.writer.as_deref(),
        }
    }
}

/// One `[[segment]]`: a part of the game such as a level, an act or a side plot.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    /// Unique id.
    pub id: Spanned<Id>,
    /// Display name.
    pub name: String,
    /// Background color of the segment's area in the graph, `#rrggbb`.
    pub color: Option<Spanned<String>>,
    /// Description for organizers.
    #[serde(default)]
    pub text: String,
    /// Display group, e.g. "Cyklus 1": segments with the same group are shown together.
    pub group: Option<String>,
}

impl Person {
    /// When the character dies, without the source span.
    #[must_use]
    pub fn death(&self) -> Option<&Death> {
        self.dies.as_ref().map(Spanned::get_ref)
    }
}

/// The `[map]` table of `map.toml`: a floor plan (walls and corridor tiles on a grid) and where the
/// rooms sit on it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    /// Grid rows, top to bottom: `.` empty, `=` corridor.
    #[serde(default)]
    pub cells: Vec<Spanned<String>>,
    /// Walls along grid lines: `[x1, y1, x2, y2]` between grid points, horizontal or vertical.
    #[serde(default)]
    pub walls: Vec<Spanned<[u16; 4]>>,
    /// Room placements by room id.
    #[serde(default)]
    pub rooms: BTreeMap<Id, Spanned<MapRoom>>,
}

/// Where a room sits on the map, in grid cells.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapRoom {
    /// Left column.
    pub x: u16,
    /// Top row.
    pub y: u16,
    /// Width in cells.
    #[serde(default = "one")]
    pub w: u16,
    /// Height in cells.
    #[serde(default = "one")]
    pub h: u16,
}

fn one() -> u16 {
    1
}

/// One `[[cycle]]` in `cycles.toml`: one hero's journey through the game's `steps`, placed on the
/// world timeline from segment `from` to segment `to`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cycle {
    /// Unique id.
    pub id: Spanned<Id>,
    /// Display name.
    pub name: String,
    /// The character who goes through the cycle (and usually dies in it).
    pub hero: Ref,
    /// World segment in which the cycle starts.
    pub from: Ref,
    /// World segment in which the cycle ends; defaults to `from`.
    pub to: Option<Ref>,
    /// Color of the cycle in the graph, `#rrggbb`.
    pub color: Option<Spanned<String>>,
    /// Description for organizers.
    #[serde(default)]
    pub text: String,
}

/// When a character dies: in a world segment (`dies = "noc-3"`) or at a step of the cycle they are
/// the hero of (`dies = 6`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Death {
    /// At the end of this world segment.
    Segment(Id),
    /// At this step (1-based) of the character's own cycle.
    Step(u8),
}

impl<'de> Deserialize<'de> for Death {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl serde::de::Visitor<'_> for V {
            type Value = Death;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a segment id or a step number")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Death, E> {
                Id::parse(v).map(Death::Segment).map_err(E::custom)
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Death, E> {
                u8::try_from(v)
                    .ok()
                    .filter(|s| *s >= 1)
                    .map(Death::Step)
                    .ok_or_else(|| E::custom("step numbers start at 1"))
            }
        }
        d.deserialize_any(V)
    }
}

/// One `[[player]]` in `players.toml`: a real participant who may play several characters in turn.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    /// Unique id.
    pub id: Spanned<Id>,
    /// Name of the participant.
    pub name: String,
}

/// One `[[person.phase]]`: how a character's sheet changes from a world segment or a cycle step on.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    /// The phase starts in this world segment.
    pub segment: Option<Ref>,
    /// The phase starts at this step (1-based) of the character's own cycle.
    pub step: Option<Spanned<u8>>,
    /// Replaces the person's goals from this segment on.
    pub goals: Option<Vec<String>>,
    /// Replaces the person's secrets from this segment on.
    pub secrets: Option<Vec<String>>,
    /// Sheet sections that replace (or add to) `[person.sheet]` from this segment on.
    #[serde(default)]
    pub sheet: BTreeMap<String, String>,
}

/// The `[action]` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// Short title.
    pub title: String,
    /// People, rooms or factions that own the action.
    #[serde(default)]
    pub owners: Vec<Ref>,
    /// Segment this action belongs to.
    pub segment: Option<Ref>,
    /// Cycle this action belongs to (instead of a segment).
    pub cycle: Option<Ref>,
    /// Step (1-based) of the cycle.
    pub step: Option<Spanned<u8>>,
    /// Items or info that must exist before the action can happen; they stay.
    #[serde(default)]
    pub needs: Vec<Ref>,
    /// Items that must exist and are used up by the action.
    #[serde(default)]
    pub uses: Vec<Ref>,
    /// Items or info the action produces.
    #[serde(default)]
    pub gives: Vec<Ref>,
    /// Longer description for organizers.
    #[serde(default)]
    pub text: String,
    /// What players read about this action in their destiny.
    #[serde(default)]
    pub brief: String,
    /// How far the writing is (`draft`, `review`, `done`); unset means not tracked.
    pub status: Option<Status>,
    /// Who writes it, for the progress panel.
    pub writer: Option<String>,
}

/// The `[decision]` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// Short title, usually a question.
    pub title: String,
    /// People, rooms or factions that own the decision.
    #[serde(default)]
    pub owners: Vec<Ref>,
    /// Segment this decision belongs to.
    pub segment: Option<Ref>,
    /// Cycle this decision belongs to (instead of a segment).
    pub cycle: Option<Ref>,
    /// Step (1-based) of the cycle.
    pub step: Option<Spanned<u8>>,
    /// Longer description for organizers.
    #[serde(default)]
    pub text: String,
    /// What players read about this decision in their destiny.
    #[serde(default)]
    pub brief: String,
    /// The possible outcomes, written as `[[decision.option]]`.
    #[serde(rename = "option", default)]
    pub options: Vec<DecisionOption>,
    /// How far the writing is (`draft`, `review`, `done`); unset means not tracked.
    pub status: Option<Status>,
    /// Who writes it, for the progress panel.
    pub writer: Option<String>,
}

/// One `[[decision.option]]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionOption {
    /// What the option is called on the graph.
    pub label: String,
    /// Items or info required for this option; they stay.
    #[serde(default)]
    pub needs: Vec<Ref>,
    /// Items required and used up by this option.
    #[serde(default)]
    pub uses: Vec<Ref>,
    /// Items or info this option produces.
    #[serde(default)]
    pub gives: Vec<Ref>,
}

/// One `[[block]]` from `blocks.toml`: a named text written once and used in many places.
///
/// Any text can include it as `[[id]]`. A block with `show` also appears as a section on those
/// people's sheets. A short block without a title works as a fact (a date, a name, a number).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    /// Id, used as `[[id]]` in texts.
    pub id: Spanned<Id>,
    /// Heading when the block is shown on a sheet.
    pub title: Option<String>,
    /// The text; may include other blocks.
    pub text: String,
    /// People whose sheets show this block as a section.
    #[serde(default)]
    pub show: Vec<Ref>,
    /// How far the writing is.
    pub status: Option<Status>,
    /// Who writes it.
    pub writer: Option<String>,
}

/// How far the writing of a node, person or other part is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Being written.
    Draft,
    /// Written, waiting for someone else to read it.
    Review,
    /// Finished.
    Done,
}

impl Status {
    /// The value as written in TOML.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Done => "done",
        }
    }
}

/// Whether a person is played by a player, an organizer, or is a non-player character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Gets a player sheet.
    Player,
    /// Gets an organizer schedule.
    Organizer,
    /// Played by an organizer; appears on organizer sheets.
    Npc,
}

/// The `[person]` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    /// Display name.
    pub name: String,
    /// Who plays this person.
    pub role: Role,
    /// Color of this person's graph nodes, `#rrggbb`.
    pub color: Option<Spanned<String>>,
    /// What the person wants.
    #[serde(default)]
    pub goals: Vec<String>,
    /// What the person hides.
    #[serde(default)]
    pub secrets: Vec<String>,
    /// Free-form sheet sections: `backstory`, `whereabouts`, `appearance`, …
    #[serde(default)]
    pub sheet: BTreeMap<String, String>,
    /// How this person sees other people, written as `[[person.relation]]`.
    #[serde(rename = "relation", default)]
    pub relations: Vec<Relation>,
    /// Factions the person belongs to (`factions.toml`).
    #[serde(default)]
    pub factions: Vec<Ref>,
    /// The player (from `players.toml`) who plays this character.
    pub played_by: Option<Ref>,
    /// First segment of the character's life; without it, the character exists from the start.
    pub enters: Option<Ref>,
    /// The character's life starts when this person dies (reincarnation); replaces `enters`.
    pub after: Option<Ref>,
    /// When the character dies; without it, they live to the end.
    pub dies: Option<Spanned<Death>>,
    /// The character works outside the hero's cycle (a boss, a director, a god).
    #[serde(default)]
    pub outside_cycle: bool,
    /// Sheet changes per segment, written as `[[person.phase]]`.
    #[serde(rename = "phase", default)]
    pub phases: Vec<Phase>,
    /// How far the writing is (`draft`, `review`, `done`); unset means not tracked.
    pub status: Option<Status>,
    /// Who writes it, for the progress panel.
    pub writer: Option<String>,
}

/// One `[[person.relation]]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    /// The other person.
    pub with: Ref,
    /// What this person thinks or knows about them.
    pub text: String,
}

/// One `[[faction]]` from `factions.toml`: a group of people (a family, a gang, a guild).
///
/// Members name it in their person file (`factions = ["montekove"]`). A faction can own actions
/// and items, which then belong to every member, and its goals, secrets and sheet sections appear
/// on every member's sheet.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Faction {
    /// Id.
    pub id: Spanned<Id>,
    /// Display name.
    pub name: String,
    /// Color as `#rrggbb`.
    pub color: Option<Spanned<String>>,
    /// Notes for organizers.
    #[serde(default)]
    pub text: String,
    /// Goals every member shares.
    #[serde(default)]
    pub goals: Vec<String>,
    /// Secrets every member knows.
    #[serde(default)]
    pub secrets: Vec<String>,
    /// More sheet sections for members, such as `history` or `rules`.
    #[serde(default)]
    pub sheet: BTreeMap<String, String>,
    /// How far the writing is.
    pub status: Option<Status>,
    /// Who writes it.
    pub writer: Option<String>,
}

impl Game {
    /// True when `owners` names `who` directly, or names a faction `who` belongs to.
    ///
    /// Use it wherever "does this person own that action or item" is asked, so faction
    /// ownership counts for every member.
    #[must_use]
    pub fn involves(&self, owners: &[Ref], who: &Id) -> bool {
        owners.iter().any(|o| {
            let o = o.get_ref();
            o == who
                || self
                    .people
                    .get(who)
                    .is_some_and(|p| p.factions.iter().any(|f| f.get_ref() == o))
        })
    }

    /// The members of faction `faction`, by id.
    pub fn members<'a>(&'a self, faction: &'a Id) -> impl Iterator<Item = &'a Id> + 'a {
        self.people
            .iter()
            .filter(move |(_, p)| p.factions.iter().any(|f| f.get_ref() == faction))
            .map(|(id, _)| id)
    }
}

/// One `[[room]]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Room {
    /// Unique id.
    pub id: Spanned<Id>,
    /// Display name.
    pub name: String,
    /// Color of this room's graph nodes, `#rrggbb`.
    pub color: Option<Spanned<String>>,
}

/// One `[[category]]` of items.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Unique id.
    pub id: Spanned<Id>,
    /// Underline color of items in this category, `#rrggbb`.
    pub color: Option<Spanned<String>>,
}

/// One `[[item]]` or `[[info]]`: something actions need and give.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thing {
    /// Unique id.
    pub id: Spanned<Id>,
    /// Display name.
    pub name: String,
    /// Category id (items only, optional).
    pub category: Option<Ref>,
    /// People, rooms or factions that hold it at the start.
    #[serde(default)]
    pub owners: Vec<Ref>,
    /// Exists when the game starts, so nothing has to give it.
    #[serde(default)]
    pub starting: bool,
    /// Marks an outcome of the game, so nothing has to need it.
    #[serde(default)]
    pub ending: bool,
    /// Description for sheets.
    #[serde(default)]
    pub text: String,
}
