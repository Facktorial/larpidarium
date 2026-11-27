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
    /// Item categories from `items.toml`.
    pub categories: Vec<Category>,
    /// Physical items from `items.toml`.
    pub items: Vec<Thing>,
    /// Pieces of information from `info.toml`.
    pub info: Vec<Thing>,
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

    /// Text for players.
    #[must_use]
    pub fn brief(&self) -> &str {
        match self {
            Self::Action(a) => &a.brief,
            Self::Decision(d) => &d.brief,
        }
    }

    /// People and rooms responsible for this node.
    #[must_use]
    pub fn owners(&self) -> &[Ref] {
        match self {
            Self::Action(a) => &a.owners,
            Self::Decision(d) => &d.owners,
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
}

/// The `[action]` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// Short title.
    pub title: String,
    /// People or rooms that own the action.
    #[serde(default)]
    pub owners: Vec<Ref>,
    /// Segment this action belongs to.
    pub segment: Option<Ref>,
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
}

/// The `[decision]` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// Short title, usually a question.
    pub title: String,
    /// People or rooms that own the decision.
    #[serde(default)]
    pub owners: Vec<Ref>,
    /// Segment this decision belongs to.
    pub segment: Option<Ref>,
    /// Longer description for organizers.
    #[serde(default)]
    pub text: String,
    /// What players read about this decision in their destiny.
    #[serde(default)]
    pub brief: String,
    /// The possible outcomes, written as `[[decision.option]]`.
    #[serde(rename = "option", default)]
    pub options: Vec<DecisionOption>,
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
    /// People or rooms that hold it at the start.
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
