//! One time axis for world segments and cycle steps.
//!
//! Every world segment is `unit` ticks long, where `unit` is the number of cycle steps (at least 1).
//! A cycle spanning `n` segments gives each of its steps `n` ticks, so all positions are integers.

use crate::Id;
use crate::model::{Cycle, Death, Game, Node, Person};

/// A half-open range of ticks, `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// First tick.
    pub start: u32,
    /// One past the last tick.
    pub end: u32,
}

impl Span {
    /// Whether the two spans share at least one tick.
    #[must_use]
    pub fn overlaps(self, other: Span) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Ticks per world segment.
#[must_use]
pub fn unit(game: &Game) -> u32 {
    u32::try_from(game.meta.steps.len())
        .unwrap_or(u32::MAX)
        .max(1)
}

/// Length of the whole game in ticks.
#[must_use]
pub fn total(game: &Game) -> u32 {
    u32::try_from(game.segments.len()).unwrap_or(u32::MAX) * unit(game)
}

fn segment_index(game: &Game, id: &Id) -> Option<u32> {
    game.segments
        .iter()
        .position(|s| s.id.get_ref() == id)
        .and_then(|i| u32::try_from(i).ok())
}

/// Ticks of a world segment.
#[must_use]
pub fn segment(game: &Game, id: &Id) -> Option<Span> {
    let i = segment_index(game, id)?;
    let u = unit(game);
    Some(Span {
        start: i * u,
        end: (i + 1) * u,
    })
}

/// Ticks of a whole cycle, from the start of `from` to the end of `to`.
#[must_use]
pub fn cycle(game: &Game, cycle: &Cycle) -> Option<Span> {
    let from = segment(game, cycle.from.get_ref())?;
    let to = match &cycle.to {
        Some(t) => segment(game, t.get_ref())?,
        None => from,
    };
    (from.start < to.end).then_some(Span {
        start: from.start,
        end: to.end,
    })
}

/// Ticks of step `step` (1-based) of a cycle.
#[must_use]
pub fn step(game: &Game, c: &Cycle, step: u8) -> Option<Span> {
    let span = cycle(game, c)?;
    let steps = u32::try_from(game.meta.steps.len()).ok()?;
    let k = u32::from(step);
    if k == 0 || k > steps {
        return None;
    }
    let len = (span.end - span.start) / steps;
    let start = span.start + (k - 1) * len;
    Some(Span {
        start,
        end: start + len,
    })
}

/// The cycle with this id.
#[must_use]
pub fn find_cycle<'a>(game: &'a Game, id: &Id) -> Option<&'a Cycle> {
    game.cycles.iter().find(|c| c.id.get_ref() == id)
}

/// The first cycle whose hero is `person`.
#[must_use]
pub fn hero_cycle<'a>(game: &'a Game, person: &Id) -> Option<&'a Cycle> {
    game.cycles.iter().find(|c| c.hero.get_ref() == person)
}

/// When an action or decision happens: its world segment, or its cycle step.
#[must_use]
pub fn node(game: &Game, node: &Node) -> Option<Span> {
    if let Some((c, s)) = node.cycle() {
        let c = find_cycle(game, c.get_ref())?;
        return match s {
            Some(s) => step(game, c, *s.get_ref()),
            None => cycle(game, c),
        };
    }
    node.segment().and_then(|s| segment(game, s.get_ref()))
}

/// When a character is alive. `None` if a segment, cycle or step it refers to does not resolve.
#[must_use]
pub fn life(game: &Game, id: &Id, person: &Person) -> Option<Span> {
    life_at_depth(game, id, person, 0)
}

/// [`life`] with a bound on `after` chains, so a loop of `after`s cannot recurse forever.
fn life_at_depth(game: &Game, id: &Id, person: &Person, depth: usize) -> Option<Span> {
    let start = if let Some(prev) = &person.after {
        if depth > game.people.len() {
            return None;
        }
        let p = game.people.get(prev.get_ref())?;
        life_at_depth(game, prev.get_ref(), p, depth + 1)?.end
    } else {
        match &person.enters {
            Some(r) => segment(game, r.get_ref())?.start,
            None => 0,
        }
    };
    let end = match person.dies.as_ref().map(toml::Spanned::get_ref) {
        Some(Death::Segment(s)) => segment(game, s)?.end,
        Some(Death::Step(k)) => step(game, hero_cycle(game, id)?, *k)?.end,
        None => total(game),
    };
    Some(Span { start, end })
}

/// First tick of a sheet phase.
#[must_use]
pub fn phase(game: &Game, id: &Id, phase: &crate::model::Phase) -> Option<u32> {
    if let Some(k) = &phase.step {
        return step(game, hero_cycle(game, id)?, *k.get_ref()).map(|s| s.start);
    }
    phase
        .segment
        .as_ref()
        .and_then(|s| segment(game, s.get_ref()))
        .map(|s| s.start)
}
