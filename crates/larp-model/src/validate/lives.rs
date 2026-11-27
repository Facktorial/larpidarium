//! Characters' lives and hero cycles on the shared time axis ([`crate::time`]).
//!
//! References that do not resolve are reported elsewhere; here they are skipped, so these checks
//! run even when the game has other errors.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Death, Game, Person};
use crate::time::{self, Span};
use crate::{Diagnostic, Id};

/// Name of the world segment at tick `t`.
fn when(game: &Game, t: u32) -> String {
    let i = usize::try_from(t / time::unit(game)).unwrap_or(usize::MAX);
    game.segments
        .get(i)
        .map_or_else(|| "the end".to_owned(), |s| s.name.clone())
}

fn step_name(game: &Game, step: u8) -> String {
    game.meta
        .steps
        .get(usize::from(step).saturating_sub(1))
        .map_or_else(|| step.to_string(), |s| format!("{step}. {s}"))
}

pub(super) fn check(game: &Game, diags: &mut Vec<Diagnostic>) {
    check_cycles(game, diags);
    check_nodes(game, diags);
    // Without segments there is no time axis; `enters`/`dies`/phases are unknown references then.
    if game.segments.is_empty() {
        return;
    }
    for (id, person) in &game.people {
        check_person(game, id, person, diags);
    }
    check_players(game, diags);
}

/// Cycles have steps, a valid time range, and placed actions.
fn check_cycles(game: &Game, diags: &mut Vec<Diagnostic>) {
    if !game.cycles.is_empty() && game.meta.steps.is_empty() {
        diags.push(
            Diagnostic::error("game.toml", None, "the game has cycles but no `steps`")
                .with_help("add `steps = [\"Ty\", \"Potřeba\", …]` under [game]"),
        );
    }
    for cycle in &game.cycles {
        let resolved = time::segment(game, cycle.from.get_ref()).is_some()
            && cycle
                .to
                .as_ref()
                .is_none_or(|t| time::segment(game, t.get_ref()).is_some());
        if resolved && time::cycle(game, cycle).is_none() {
            diags.push(Diagnostic::error(
                "cycles.toml",
                Some(cycle.id.span()),
                format!("cycle “{}” ends before it starts", cycle.name),
            ));
        }
        if let Some(hero) = game.people.get(cycle.hero.get_ref())
            && hero.outside_cycle
        {
            diags.push(Diagnostic::warning(
                "cycles.toml",
                Some(cycle.hero.span()),
                format!(
                    "“{}” is marked `outside_cycle` but is the hero of “{}”",
                    hero.name, cycle.name
                ),
            ));
        }
    }
    let mut heroes: BTreeMap<&Id, Vec<&str>> = BTreeMap::new();
    for cycle in &game.cycles {
        heroes
            .entry(cycle.hero.get_ref())
            .or_default()
            .push(&cycle.name);
    }
    for (hero, names) in heroes.iter().filter(|(_, n)| n.len() > 1) {
        diags.push(
            Diagnostic::warning(
                format!("people/{hero}.toml"),
                None,
                format!("hero of several cycles: {}", names.join(", ")),
            )
            .with_help("a character dies in their cycle; give the next cycle a new character"),
        );
    }
}

/// Every node is either in a segment or in a cycle step, never both.
fn check_nodes(game: &Game, diags: &mut Vec<Diagnostic>) {
    let steps = game.meta.steps.len();
    for (id, node) in &game.nodes {
        let file = format!("actions/{id}.toml");
        let Some((cycle, step)) = node.cycle() else {
            if let Some(step) = node.step() {
                diags.push(
                    Diagnostic::error(&file, Some(step.span()), "`step` needs `cycle`")
                        .with_help("set `cycle = \"…\"`, or remove `step` for a world action"),
                );
            }
            continue;
        };
        if node.segment().is_some() {
            diags.push(
                Diagnostic::error(
                    &file,
                    Some(cycle.span()),
                    "an action is either in a `segment` or in a `cycle`, not both",
                )
                .with_help("the cycle already places the action in time"),
            );
        }
        match step {
            None => diags.push(
                Diagnostic::error(&file, Some(cycle.span()), "cycle action has no `step`")
                    .with_help(format!("add `step = 1` … `step = {steps}`")),
            ),
            Some(s) if usize::from(*s.get_ref()) > steps || *s.get_ref() == 0 => {
                diags.push(Diagnostic::error(
                    &file,
                    Some(s.span()),
                    format!(
                        "step {} does not exist; the game has {steps} steps",
                        s.get_ref()
                    ),
                ));
            }
            Some(_) => {}
        }
    }
}

fn check_person(game: &Game, id: &Id, person: &Person, diags: &mut Vec<Diagnostic>) {
    let file = format!("people/{id}.toml");
    let hero_of = time::hero_cycle(game, id);
    if let Some(d) = &person.dies
        && let Death::Step(k) = d.get_ref()
    {
        match hero_of {
            None => {
                diags.push(
                    Diagnostic::error(
                        &file,
                        Some(d.span()),
                        format!("dies at step {k}, but is not the hero of any cycle"),
                    )
                    .with_help("use a segment id (`dies = \"…\"`), or make them a cycle's `hero`"),
                );
                return;
            }
            Some(_) if usize::from(*k) > game.meta.steps.len() => {
                diags.push(Diagnostic::error(
                    &file,
                    Some(d.span()),
                    format!("step {k} does not exist"),
                ));
                return;
            }
            Some(_) => {}
        }
    }
    if let Some(cycle) = hero_of
        && !matches!(
            person.dies.as_ref().map(toml::Spanned::get_ref),
            Some(Death::Step(_))
        )
    {
        diags.push(
            Diagnostic::warning(
                &file,
                None,
                format!("hero of “{}” does not die in it", cycle.name),
            )
            .with_help("set `dies = <step>` (1 … number of steps)"),
        );
    }
    if person.after.is_some() && person.enters.is_some() {
        diags.push(Diagnostic::error(
            &file,
            person.after.as_ref().map(toml::Spanned::span),
            "use either `enters` or `after`, not both",
        ));
    }
    check_after(game, person, &file, diags);
    let Some(life) = time::life(game, id, person) else {
        return;
    };
    // `after` someone who never dies already got a warning; the empty life is its consequence.
    let after_immortal = person
        .after
        .as_ref()
        .and_then(|prev| game.people.get(prev.get_ref()))
        .is_some_and(|p| p.death().is_none());
    if after_immortal {
        return;
    }
    if life.start >= life.end {
        diags.push(
            Diagnostic::error(
                &file,
                person.dies.as_ref().map(toml::Spanned::span),
                format!("dies before entering (in “{}”)", when(game, life.start)),
            )
            .with_help("segments are ordered as in segments.toml"),
        );
        return;
    }
    if let Some(cycle) = hero_of
        && let Some(span) = time::cycle(game, cycle)
        && !span.overlaps(life)
    {
        diags.push(Diagnostic::warning(
            &file,
            None,
            format!("is not alive during their cycle “{}”", cycle.name),
        ));
    }
    check_actions(game, id, person, life, diags);
    check_phases(game, id, person, life, hero_of.is_some(), diags);
}

/// `after`: the predecessor should die and be played by the same player.
fn check_after(game: &Game, person: &Person, file: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(prev) = &person.after
        && let Some(p) = game.people.get(prev.get_ref())
    {
        if p.death().is_none() {
            diags.push(Diagnostic::warning(
                file,
                Some(prev.span()),
                format!("comes after “{}”, who never dies", p.name),
            ));
        }
        if p.played_by.is_some()
            && p.played_by.as_ref().map(toml::Spanned::get_ref)
                != person.played_by.as_ref().map(toml::Spanned::get_ref)
        {
            diags.push(Diagnostic::warning(
                file,
                Some(prev.span()),
                format!("comes after “{}”, who is played by someone else", p.name),
            ));
        }
    }
}

/// Phases start inside the life, once per moment, with exactly one of `segment` / `step`.
fn check_phases(
    game: &Game,
    id: &Id,
    person: &Person,
    life: Span,
    is_hero: bool,
    diags: &mut Vec<Diagnostic>,
) {
    let file = format!("people/{id}.toml");
    let mut seen = BTreeSet::new();
    for phase in &person.phases {
        let span = phase
            .step
            .as_ref()
            .map(toml::Spanned::span)
            .or_else(|| phase.segment.as_ref().map(toml::Spanned::span));
        let label = phase.step.as_ref().map_or_else(
            || {
                phase
                    .segment
                    .as_ref()
                    .and_then(|s| time::segment(game, s.get_ref()))
                    .map(|s| when(game, s.start))
                    .unwrap_or_default()
            },
            |k| step_name(game, *k.get_ref()),
        );
        if phase.step.is_some() == phase.segment.is_some() {
            diags.push(Diagnostic::error(
                &file,
                span.clone(),
                "a phase needs exactly one of `segment` or `step`",
            ));
            continue;
        }
        if phase.step.is_some() && !is_hero {
            diags.push(Diagnostic::error(
                &file,
                span.clone(),
                "phase by `step`, but the character is not the hero of any cycle",
            ));
            continue;
        }
        if let Some(k) = &phase.step
            && (*k.get_ref() == 0 || usize::from(*k.get_ref()) > game.meta.steps.len())
        {
            diags.push(Diagnostic::error(
                &file,
                Some(k.span()),
                format!(
                    "step {} does not exist; the game has {} steps",
                    k.get_ref(),
                    game.meta.steps.len()
                ),
            ));
            continue;
        }
        let Some(at) = time::phase(game, id, phase) else {
            continue;
        };
        if !seen.insert(at) {
            diags.push(Diagnostic::warning(
                &file,
                span.clone(),
                format!("two phases start at “{label}”; merge them into one"),
            ));
        }
        if at < life.start || at >= life.end {
            diags.push(Diagnostic::warning(
                &file,
                span.clone(),
                format!("phase “{label}” is outside the character's life"),
            ));
        }
    }
}

/// A character owns no action outside their life.
fn check_actions(game: &Game, id: &Id, person: &Person, life: Span, diags: &mut Vec<Diagnostic>) {
    for (node_id, node) in &game.nodes {
        if !node.owners().iter().any(|o| o.get_ref() == id) {
            continue;
        }
        let Some(at) = time::node(game, node) else {
            continue;
        };
        if at.overlaps(life) {
            continue;
        }
        let what = if at.end <= life.start {
            "before entering"
        } else {
            "after dying"
        };
        let span = node
            .segment()
            .map(toml::Spanned::span)
            .or_else(|| node.cycle().map(|(c, _)| c.span()));
        diags.push(Diagnostic::warning(
            format!("actions/{node_id}.toml"),
            span,
            format!(
                "“{}” acts in “{}”, {what} (alive “{}” – “{}”)",
                person.name,
                when(game, at.start),
                when(game, life.start),
                when(game, life.end.saturating_sub(1)),
            ),
        ));
    }
}

/// A character with its id and life.
type Life<'a> = (&'a Id, &'a Person, Span);

/// No player plays two characters at once, no player is ever without a character, and every
/// player is a hero at least once when the game has cycles.
fn check_players(game: &Game, diags: &mut Vec<Diagnostic>) {
    let mut by_player: BTreeMap<&Id, Vec<Life<'_>>> = BTreeMap::new();
    for (id, person) in &game.people {
        if let (Some(p), Some(life)) = (&person.played_by, time::life(game, id, person)) {
            by_player
                .entry(p.get_ref())
                .or_default()
                .push((id, person, life));
        }
    }
    for player in &game.players {
        let warn = |message: String, help: &str| {
            Diagnostic::warning("players.toml", Some(player.id.span()), message).with_help(help)
        };
        let lives = by_player
            .get(player.id.get_ref())
            .map(Vec::as_slice)
            .unwrap_or_default();
        for (i, (_, a, la)) in lives.iter().enumerate() {
            for (_, b, lb) in lives.iter().skip(i + 1) {
                if la.overlaps(*lb) {
                    diags.push(warn(
                        format!(
                            "{} plays “{}” and “{}” at the same time, from “{}”",
                            player.name,
                            a.name,
                            b.name,
                            when(game, la.start.max(lb.start))
                        ),
                        "shift `enters` / `dies` so their lives do not overlap",
                    ));
                }
            }
        }
        let mut idle: Vec<String> = Vec::new();
        for t in 0..time::total(game) {
            if !lives.iter().any(|(_, _, l)| l.start <= t && t < l.end) {
                let name = when(game, t);
                if idle.last() != Some(&name) {
                    idle.push(name);
                }
            }
        }
        if !idle.is_empty() {
            diags.push(warn(
                format!("{} has no character in: {}", player.name, idle.join(", ")),
                "give them a new character there, or one outside the cycle",
            ));
        }
        if !game.cycles.is_empty()
            && !lives
                .iter()
                .any(|(id, _, _)| time::hero_cycle(game, id).is_some())
        {
            diags.push(warn(
                format!("{} is never the hero of a cycle", player.name),
                "make one of their characters the `hero` of a cycle in cycles.toml",
            ));
        }
    }
}
