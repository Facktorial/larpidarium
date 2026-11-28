#![doc = include_str!("../README.md")]

mod labels;

use larp_graph::{Destiny, Packing, Profile, Schedule};
use larp_model::model::{Death, Language, Role};
use larp_model::{Game, Id};
use minijinja::{AutoEscape, Environment, Error, ErrorKind, Value};
use serde::Serialize;

pub use labels::labels;

/// The built-in player sheet template (minijinja, HTML).
pub const PLAYER_SHEET_HTML: &str = include_str!("../templates/html/player.html.j2");

/// The built-in player sheet template for PDFs (minijinja, LaTeX for tectonic/XeLaTeX).
pub const PLAYER_SHEET_TEX: &str = include_str!("../templates/tex/player.tex.j2");

/// The built-in organizer sheet template (minijinja, HTML).
pub const ORGANIZER_SHEET_HTML: &str = include_str!("../templates/html/organizer.html.j2");

/// The built-in organizer sheet template for PDFs (minijinja, LaTeX).
pub const ORGANIZER_SHEET_TEX: &str = include_str!("../templates/tex/organizer.tex.j2");

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

impl From<Error> for RenderError {
    fn from(e: Error) -> Self {
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
    /// Blocks from `blocks.toml` that list this person (or one of their factions) in `show`,
    /// as (title, text).
    pub blocks: Vec<(String, String)>,
    /// The person's factions with what every member gets.
    pub factions: Vec<FactionView>,
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

/// A faction as its members see it on their sheet.
#[derive(Debug, Clone, Serialize)]
pub struct FactionView {
    /// Display name.
    pub name: String,
    /// Color as `#rrggbb`, or empty.
    pub color: String,
    /// Goals every member shares.
    pub goals: Vec<String>,
    /// Secrets every member knows.
    pub secrets: Vec<String>,
    /// More sections, as (key, text).
    pub sheet: Vec<(String, String)>,
    /// Display names of the other members.
    pub members: Vec<String>,
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
    let ctx = SheetContext {
        game_title: &game.meta.title,
        language: language(game),
        t: labels(game.meta.language),
        person: person_view(game, person)?,
        destiny,
    };
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.add_template("player.html", template.unwrap_or(PLAYER_SHEET_HTML))?;
    Ok(env.get_template("player.html")?.render(&ctx)?)
}

/// Part I and the life of `person`, ready for a template.
fn person_view(game: &Game, person: &Id) -> Result<PersonView, RenderError> {
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
        blocks: game
            .blocks
            .iter()
            .filter(|b| game.involves(&b.show, person))
            .map(|b| (b.title.clone().unwrap_or_default(), b.text.clone()))
            .collect(),
        factions: p
            .factions
            .iter()
            .filter_map(|r| game.factions.iter().find(|f| f.id.get_ref() == r.get_ref()))
            .map(|f| FactionView {
                name: f.name.clone(),
                // Checked by `check`; checked again because it goes into a `style` attribute.
                color: f
                    .color
                    .as_ref()
                    .map(|c| c.get_ref().clone())
                    .filter(|c| {
                        c.len() == 7
                            && c.starts_with('#')
                            && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
                    })
                    .unwrap_or_default(),
                goals: f.goals.clone(),
                secrets: f.secrets.clone(),
                sheet: f
                    .sheet
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                members: game
                    .members(f.id.get_ref())
                    .filter(|m| *m != person)
                    .filter_map(|m| game.people.get(m).map(|p| p.name.clone()))
                    .collect(),
            })
            .collect(),
        played_by: String::new(),
        enters: String::new(),
        dies: String::new(),
        outside_cycle: false,
        phases: Vec::new(),
    };
    Ok(with_life(game, person, p, view))
}

fn language(game: &Game) -> &'static str {
    match game.meta.language {
        Language::Cs => "cs",
        Language::En => "en",
    }
}

/// One person's sheet inside a LaTeX document.
#[derive(Debug, Serialize)]
pub struct TexSheet<'a> {
    /// Part I.
    pub person: PersonView,
    /// Part II.
    pub destiny: &'a Destiny,
    /// The person's color as `RRGGBB` for `\definecolor{accent}{HTML}{…}`.
    pub accent: String,
}

/// Everything the LaTeX template can use.
#[derive(Debug, Serialize)]
pub struct TexContext<'a> {
    /// Game title.
    pub game_title: &'a str,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Translated labels, see [`labels`].
    pub t: std::collections::BTreeMap<&'static str, &'static str>,
    /// The sheets, one per page group, in the given order.
    pub sheets: Vec<TexSheet<'a>>,
}

/// Renders one LaTeX document with the sheets of `people` (each with its destiny), one after
/// another on new pages. `template` overrides [`PLAYER_SHEET_TEX`]. All values printed by the
/// template are escaped with [`tex_escape`].
///
/// # Errors
///
/// [`RenderError::UnknownPerson`] if a person is not in `game`, [`RenderError::Template`] if the
/// template is invalid.
pub fn player_sheets_tex(
    game: &Game,
    people: &[(&Id, &Destiny)],
    template: Option<&str>,
) -> Result<String, RenderError> {
    let mut sheets = Vec::with_capacity(people.len());
    for (id, destiny) in people {
        let person = person_view(game, id)?;
        let accent = accent(&person.color, "7a5c3e");
        sheets.push(TexSheet {
            person,
            destiny,
            accent,
        });
    }
    let ctx = TexContext {
        game_title: &game.meta.title,
        language: language(game),
        t: labels(game.meta.language),
        sheets,
    };
    let mut env = tex_env();
    env.add_template("player.tex", template.unwrap_or(PLAYER_SHEET_TEX))?;
    Ok(env.get_template("player.tex")?.render(&ctx)?)
}

/// A template engine that escapes every printed value with [`tex_escape`].
fn tex_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Custom("tex"));
    env.set_formatter(|out, state, value| {
        let text = if value.is_undefined() || value.is_none() {
            String::new()
        } else if value.is_safe() || state.auto_escape() == AutoEscape::None {
            value.to_string()
        } else {
            tex_escape(&value_text(value))
        };
        out.write_str(&text)
            .map_err(|e| Error::new(ErrorKind::WriteFailure, e.to_string()))
    });
    env.add_filter("lines", |text: String| {
        Value::from_safe_string(tex_lines(&text))
    });
    env
}

/// Escapes `text` for LaTeX like [`tex_escape`] and keeps its line breaks: a single newline
/// becomes `\newline`, a blank line still starts a paragraph. For letters and notes, where every
/// line matters. Templates use it as the `lines` filter.
///
/// ```
/// assert_eq!(
///     larp_render::tex_lines("Milý pane,\npíši 50 %.\n\nOpat"),
///     "Milý pane,\\newline\npíši 50 \\%.\n\nOpat"
/// );
/// ```
#[must_use]
pub fn tex_lines(text: &str) -> String {
    text.replace("\r\n", "\n")
        .trim()
        .split("\n\n")
        .map(|para| {
            para.trim()
                .lines()
                .map(tex_escape)
                .collect::<Vec<_>>()
                .join("\\newline\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `RRGGBB` from a `#rrggbb` color, or the default accent.
fn accent(color: &str, default: &str) -> String {
    color
        .strip_prefix('#')
        .filter(|h| h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(default)
        .to_ascii_uppercase()
}

/// Everything an organizer sheet template can use.
#[derive(Debug, Serialize)]
pub struct OrganizerContext<'a> {
    /// Game title.
    pub game_title: &'a str,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Translated labels, see [`labels`].
    pub t: std::collections::BTreeMap<&'static str, &'static str>,
    /// The schedule.
    pub s: &'a Schedule,
}

/// Renders the organizer sheet for `schedule`. `template` overrides [`ORGANIZER_SHEET_HTML`].
///
/// # Errors
///
/// [`RenderError::Template`] if the template is invalid.
pub fn organizer_sheet_html(
    game: &Game,
    schedule: &Schedule,
    template: Option<&str>,
) -> Result<String, RenderError> {
    let ctx = OrganizerContext {
        game_title: &game.meta.title,
        language: language(game),
        t: labels(game.meta.language),
        s: schedule,
    };
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.add_template("organizer.html", template.unwrap_or(ORGANIZER_SHEET_HTML))?;
    Ok(env.get_template("organizer.html")?.render(&ctx)?)
}

/// The packing list page (HTML). Copy it to `<game>/templates/packing.html.j2` to customize it.
pub const PACKING_HTML: &str = include_str!("../templates/html/packing.html.j2");

/// The packing list (LaTeX).
pub const PACKING_TEX: &str = include_str!("../templates/tex/packing.tex.j2");

/// Handout cards to print (HTML).
pub const HANDOUTS_HTML: &str = include_str!("../templates/html/handouts.html.j2");

/// Handout cards to print (LaTeX).
pub const HANDOUTS_TEX: &str = include_str!("../templates/tex/handouts.tex.j2");

/// Everything the packing list and handout templates can use.
#[derive(Debug, Serialize)]
pub struct PackingContext<'a> {
    /// Game title.
    pub game_title: &'a str,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Translated labels, see [`labels`].
    pub t: std::collections::BTreeMap<&'static str, &'static str>,
    /// The packing list and handouts.
    pub p: &'a Packing,
}

/// A character's profile page (HTML), for organizers.
pub const PROFILE_HTML: &str = include_str!("../templates/html/profile.html.j2");

/// Everything the profile template can use.
#[derive(Debug, Serialize)]
pub struct ProfileContext<'a> {
    /// Game title.
    pub game_title: &'a str,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Translated labels, see [`labels`].
    pub t: std::collections::BTreeMap<&'static str, &'static str>,
    /// The profile.
    pub p: &'a Profile,
}

/// Renders a character's profile page; `template` overrides [`PROFILE_HTML`].
///
/// # Errors
///
/// [`RenderError::Template`] if the template is invalid.
pub fn profile_html(
    game: &Game,
    profile: &Profile,
    template: Option<&str>,
) -> Result<String, RenderError> {
    let ctx = ProfileContext {
        game_title: &game.meta.title,
        language: language(game),
        t: labels(game.meta.language),
        p: profile,
    };
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.add_template("profile.html", template.unwrap_or(PROFILE_HTML))?;
    Ok(env.get_template("profile.html")?.render(&ctx)?)
}

/// Which printable page to render from a [`Packing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The packing list for organizers.
    Packing,
    /// Handout cards for players.
    Handouts,
}

/// Renders the packing list or the handout cards as HTML; `template` overrides
/// [`PACKING_HTML`] / [`HANDOUTS_HTML`].
///
/// # Errors
///
/// [`RenderError::Template`] if the template is invalid.
pub fn packing_html(
    game: &Game,
    packing: &Packing,
    page: Page,
    template: Option<&str>,
) -> Result<String, RenderError> {
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    let default = match page {
        Page::Packing => PACKING_HTML,
        Page::Handouts => HANDOUTS_HTML,
    };
    env.add_template("page.html", template.unwrap_or(default))?;
    Ok(env
        .get_template("page.html")?
        .render(packing_context(game, packing))?)
}

/// Renders the packing list or the handout cards as LaTeX, escaped as in
/// [`player_sheets_tex`]; `template` overrides [`PACKING_TEX`] / [`HANDOUTS_TEX`].
///
/// # Errors
///
/// [`RenderError::Template`] if the template is invalid.
pub fn packing_tex(
    game: &Game,
    packing: &Packing,
    page: Page,
    template: Option<&str>,
) -> Result<String, RenderError> {
    let mut env = tex_env();
    let default = match page {
        Page::Packing => PACKING_TEX,
        Page::Handouts => HANDOUTS_TEX,
    };
    env.add_template("page.tex", template.unwrap_or(default))?;
    Ok(env
        .get_template("page.tex")?
        .render(packing_context(game, packing))?)
}

fn packing_context<'a>(game: &'a Game, packing: &'a Packing) -> PackingContext<'a> {
    PackingContext {
        game_title: &game.meta.title,
        language: language(game),
        t: labels(game.meta.language),
        p: packing,
    }
}

/// One organizer sheet inside a LaTeX document.
#[derive(Debug, Serialize)]
pub struct TexSchedule<'a> {
    /// The schedule.
    pub schedule: &'a Schedule,
    /// Color as `RRGGBB`.
    pub accent: String,
}

/// Everything the organizer LaTeX template can use.
#[derive(Debug, Serialize)]
pub struct TexOrganizerContext<'a> {
    /// Game title.
    pub game_title: &'a str,
    /// `"cs"` or `"en"`.
    pub language: &'static str,
    /// Translated labels, see [`labels`].
    pub t: std::collections::BTreeMap<&'static str, &'static str>,
    /// The sheets in the given order.
    pub sheets: Vec<TexSchedule<'a>>,
}

/// Renders one LaTeX document with the organizer sheets of `schedules`, each starting on a new
/// front page. `template` overrides [`ORGANIZER_SHEET_TEX`]; values are escaped as in
/// [`player_sheets_tex`].
///
/// # Errors
///
/// [`RenderError::Template`] if the template is invalid.
pub fn organizer_sheets_tex(
    game: &Game,
    schedules: &[&Schedule],
    template: Option<&str>,
) -> Result<String, RenderError> {
    let ctx = TexOrganizerContext {
        game_title: &game.meta.title,
        language: language(game),
        t: labels(game.meta.language),
        sheets: schedules
            .iter()
            .map(|s| TexSchedule {
                schedule: s,
                accent: accent(&s.color, "3e5c7a"),
            })
            .collect(),
    };
    let mut env = tex_env();
    env.add_template("organizer.tex", template.unwrap_or(ORGANIZER_SHEET_TEX))?;
    Ok(env.get_template("organizer.tex")?.render(&ctx)?)
}

fn value_text(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

/// Escapes text for LaTeX: the ten special characters, `"` (active in Czech babel) and square
/// brackets (so text can follow `\item`). Line breaks and tabs are kept (a blank line starts a
/// paragraph); other control characters, which `XeTeX` rejects, are dropped.
///
/// ```
/// assert_eq!(larp_render::tex_escape("50 % & $5_a"), r"50 \% \& \$5\_a");
/// assert_eq!(larp_render::tex_escape("„Ř“ {x}"), r"„Ř“ \{x\}");
/// ```
#[must_use]
pub fn tex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '\\' => out.push_str(r"\textbackslash{}"),
            '{' | '}' | '$' | '&' | '#' | '%' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str(r"\textasciicircum{}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '"' => out.push_str(r"\textquotedbl{}"),
            '[' => out.push_str("{[}"),
            ']' => out.push_str("{]}"),
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            _ => out.push(c),
        }
    }
    out
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
