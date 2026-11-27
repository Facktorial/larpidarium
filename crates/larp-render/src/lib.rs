#![doc = include_str!("../README.md")]

mod labels;

use larp_graph::Destiny;
use larp_model::model::{Death, Language, Role};
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
    /// Name of the player who plays the character, or empty.
    pub played_by: String,
    /// Name of the segment the character enters in, or empty (from the start).
    pub enters: String,
    /// When the character dies (a segment name, or a step label like "6. Cena (cycle)"), or empty (lives to the end).
    pub dies: String,
    /// Works outside the hero's cycle.
    pub outside_cycle: bool,
    /// How the sheet changes per segment, in segment order.
    pub phases: Vec<PhaseView>,
}

/// One `[[person.phase]]`, ready to print.
#[derive(Debug, Serialize)]
pub struct PhaseView {
    /// When the phase starts: a segment name, or a step label like "2. Potřeba (cycle)".
    pub segment: String,
    /// New goals, if they change.
    pub goals: Option<Vec<String>>,
    /// New secrets, if they change.
    pub secrets: Option<Vec<String>>,
    /// Changed sheet sections as (key, text); known keys are translated in the template.
    pub sections: Vec<(String, String)>,
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
        played_by: String::new(),
        enters: String::new(),
        dies: String::new(),
        outside_cycle: false,
        phases: Vec::new(),
    };
    let view = with_life(game, person, p, view);
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

/// Adds the character's life (player, entry, death) and sheet phases to `view`.
fn with_life(game: &Game, id: &Id, p: &larp_model::model::Person, view: PersonView) -> PersonView {
    let segment_name = |id: &Id| {
        game.segments
            .iter()
            .find(|s| s.id.get_ref() == id)
            .map_or_else(|| id.to_string(), |s| s.name.clone())
    };
    let cycle = larp_model::time::hero_cycle(game, id);
    let step_name = |k: u8| {
        let name = game
            .meta
            .steps
            .get(usize::from(k).saturating_sub(1))
            .map_or_else(|| k.to_string(), |s| format!("{k}. {s}"));
        match cycle {
            Some(c) => format!("{name} ({})", c.name),
            None => name,
        }
    };
    let mut phases: Vec<&larp_model::model::Phase> = p.phases.iter().collect();
    phases.sort_by_key(|ph| larp_model::time::phase(game, id, ph).unwrap_or(u32::MAX));
    PersonView {
        played_by: p
            .played_by
            .as_ref()
            .map(|r| {
                game.players
                    .iter()
                    .find(|pl| pl.id.get_ref() == r.get_ref())
                    .map_or_else(|| r.get_ref().to_string(), |pl| pl.name.clone())
            })
            .unwrap_or_default(),
        enters: p
            .enters
            .as_ref()
            .map(|r| segment_name(r.get_ref()))
            .unwrap_or_default(),
        dies: match p.death() {
            Some(Death::Segment(s)) => segment_name(s),
            Some(Death::Step(k)) => step_name(*k),
            None => String::new(),
        },
        outside_cycle: p.outside_cycle,
        phases: phases
            .into_iter()
            .map(|ph| PhaseView {
                segment: match (&ph.step, &ph.segment) {
                    (Some(k), _) => step_name(*k.get_ref()),
                    (None, Some(s)) => segment_name(s.get_ref()),
                    (None, None) => String::new(),
                },
                goals: ph.goals.clone(),
                secrets: ph.secrets.clone(),
                sections: ph
                    .sheet
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            })
            .collect(),
        ..view
    }
}
