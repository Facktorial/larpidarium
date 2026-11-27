#![doc = include_str!("../README.md")]

mod labels;

use larp_graph::Destiny;
use larp_model::model::{Language, Role};
use larp_model::{Game, Id};
use minijinja::{AutoEscape, Environment};
use serde::Serialize;

pub use labels::labels;

/// The built-in player sheet template (minijinja, HTML).
pub const PLAYER_SHEET_HTML: &str = include_str!("../templates/html/player.html.j2");

/// Why a sheet could not be rendered.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The person id is not in the game.
    #[error("unknown person `{0}`")]
    UnknownPerson(String),
    /// The template has a syntax error or failed while rendering.
    #[error("template error: {0}")]
    Template(String),
}

impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        let mut msg = e.to_string();
        if let Some(detail) = e.detail() {
            msg.push_str(": ");
            msg.push_str(detail);
        }
        Self::Template(msg)
    }
}

/// Everything a sheet template can use.
#[derive(Debug, Serialize)]
pub struct SheetContext<'a> {
    /// Game title.
    pub game_title: &'a str,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Translated labels, see [`labels`].
    pub t: std::collections::BTreeMap<&'static str, &'static str>,
    /// Part I.
    pub person: PersonView,
    /// Part II.
    pub destiny: &'a Destiny,
}

/// Part I of the sheet: the hand-written parts of the person file.
#[derive(Debug, Serialize)]
pub struct PersonView {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `"player"`, `"organizer"` or `"npc"`.
    pub role: &'static str,
    /// `#rrggbb` or empty.
    pub color: String,
    /// `sheet.appearance`.
    pub appearance: String,
    /// `sheet.whereabouts`.
    pub whereabouts: String,
    /// `sheet.backstory`.
    pub backstory: String,
    /// Goals.
    pub goals: Vec<String>,
    /// Secrets.
    pub secrets: Vec<String>,
    /// Relations, with the other person's display name.
    pub relations: Vec<RelationView>,
    /// Other `[person.sheet]` keys, as (key, text).
    pub extra: Vec<(String, String)>,
}

/// One relation.
#[derive(Debug, Serialize)]
pub struct RelationView {
    /// The other person's name.
    pub name: String,
    /// The text.
    pub text: String,
}

const KNOWN_SHEET_KEYS: [&str; 3] = ["appearance", "whereabouts", "backstory"];

/// Renders the player sheet of `person`. `template` overrides [`PLAYER_SHEET_HTML`].
///
/// # Errors
///
/// [`RenderError::UnknownPerson`] if `person` is not in `game`, [`RenderError::Template`] if the
/// template is invalid.
pub fn player_sheet_html(
    game: &Game,
    person: &Id,
    destiny: &Destiny,
    template: Option<&str>,
) -> Result<String, RenderError> {
    let p = game
        .people
        .get(person)
        .ok_or_else(|| RenderError::UnknownPerson(person.to_string()))?;
    let sheet = |key: &str| p.sheet.get(key).cloned().unwrap_or_default();
    let view = PersonView {
        id: person.to_string(),
        name: p.name.clone(),
        role: match p.role {
            Role::Player => "player",
            Role::Organizer => "organizer",
            Role::Npc => "npc",
        },
        color: p
            .color
            .as_ref()
            .map(|c| c.get_ref().clone())
            .unwrap_or_default(),
        appearance: sheet("appearance"),
        whereabouts: sheet("whereabouts"),
        backstory: sheet("backstory"),
        goals: p.goals.clone(),
        secrets: p.secrets.clone(),
        relations: p
            .relations
            .iter()
            .map(|r| RelationView {
                name: game
                    .people
                    .get(r.with.get_ref())
                    .map_or_else(|| r.with.get_ref().to_string(), |o| o.name.clone()),
                text: r.text.clone(),
            })
            .collect(),
        extra: p
            .sheet
            .iter()
            .filter(|(k, _)| !KNOWN_SHEET_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    };
    let language = match game.meta.language {
        Language::Cs => "cs",
        Language::En => "en",
    };
    let ctx = SheetContext {
        game_title: &game.meta.title,
        language,
        t: labels(game.meta.language),
        person: view,
        destiny,
    };

    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.add_template("player.html", template.unwrap_or(PLAYER_SHEET_HTML))?;
    Ok(env.get_template("player.html")?.render(&ctx)?)
}
