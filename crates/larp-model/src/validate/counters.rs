//! Checks of `counters.toml` and of `change` / `require` on actions and options: counters exist,
//! their ranges make sense, and every requirement can be met.

use std::collections::{HashMap, HashSet};

use super::{Entry, Kind, check_refs, node_file};
use crate::model::{Bound, Counter, Game, Node, Ref};
use crate::{Diagnostic, Id};

/// One action or option with its counter keys.
struct Uses<'a> {
    file: String,
    change: Vec<&'a Ref>,
    require: Vec<(&'a Ref, Bound)>,
}

pub(super) fn check(game: &Game, registry: &HashMap<Id, Entry>, diags: &mut Vec<Diagnostic>) {
    for c in &game.counters {
        check_range(c, diags);
    }
    let uses = uses(game);
    let counters: HashMap<&Id, &Counter> =
        game.counters.iter().map(|c| (c.id.get_ref(), c)).collect();
    let mut changed: HashSet<&Id> = HashSet::new();
    for u in &uses {
        let change: Vec<Ref> = u.change.iter().map(|r| (*r).clone()).collect();
        let require: Vec<Ref> = u.require.iter().map(|(r, _)| (*r).clone()).collect();
        check_refs(
            &change,
            &u.file,
            &[Kind::Counter],
            "`change` key",
            registry,
            diags,
        );
        check_refs(
            &require,
            &u.file,
            &[Kind::Counter],
            "`require` key",
            registry,
            diags,
        );
        changed.extend(u.change.iter().map(|r| r.get_ref()));
        for (r, bound) in &u.require {
            check_bound(
                &u.file,
                r,
                *bound,
                counters.get(r.get_ref()).copied(),
                diags,
            );
        }
    }
    for c in &game.counters {
        if !changed.contains(c.id.get_ref()) {
            diags.push(
                Diagnostic::warning(
                    "counters.toml",
                    Some(c.id.span()),
                    format!("counter `{}` is never changed", c.id.get_ref()),
                )
                .with_help(format!(
                    "add `change = {{ {} = 1 }}` to an action or option, or remove the counter",
                    c.id.get_ref()
                )),
            );
        }
    }
}

/// `min <= max` and `start` within them.
fn check_range(c: &Counter, diags: &mut Vec<Diagnostic>) {
    let (min, max, start) = (c.min(), c.max(), c.start());
    if min > max {
        let span = c.max.as_ref().or(c.min.as_ref()).map(toml::Spanned::span);
        diags.push(
            Diagnostic::error(
                "counters.toml",
                span,
                format!("counter `{}` has min {min} above max {max}", c.id.get_ref()),
            )
            .with_help("min defaults to 0 and max to 10"),
        );
    } else if !(min..=max).contains(&start) {
        let span = c
            .start
            .as_ref()
            .map_or_else(|| c.id.span(), toml::Spanned::span);
        diags.push(
            Diagnostic::error(
                "counters.toml",
                Some(span),
                format!(
                    "counter `{}` starts at {start}, outside its range {min} to {max}",
                    c.id.get_ref()
                ),
            )
            .with_help("start defaults to 0, min to 0 and max to 10"),
        );
    }
}

/// A requirement must name a bound and fit the counter's range.
fn check_bound(
    file: &str,
    r: &Ref,
    bound: Bound,
    counter: Option<&Counter>,
    diags: &mut Vec<Diagnostic>,
) {
    let id = r.get_ref();
    let mut problem = |message: String, help: &str| {
        diags.push(Diagnostic::error(file, Some(r.span()), message).with_help(help));
    };
    match (bound.at_least, bound.at_most) {
        (None, None) => problem(
            format!("requirement on `{id}` has no `at_least` or `at_most`"),
            "write it as `{ at_least = 3 }`, `{ at_most = 1 }` or both",
        ),
        (Some(l), Some(m)) if l > m => problem(
            format!("requirement on `{id}` asks for at least {l} and at most {m}"),
            "no value can be both; swap them or drop one",
        ),
        _ => {
            let Some(c) = counter else { return };
            let (min, max) = (c.min(), c.max());
            if bound.at_least.is_some_and(|l| l > max) || bound.at_most.is_some_and(|m| m < min) {
                problem(
                    format!(
                        "requirement on `{id}` can never be met: the counter stays between {min} and {max}"
                    ),
                    "change the requirement, or the counter's min or max in counters.toml",
                );
            }
        }
    }
}

fn uses(game: &Game) -> Vec<Uses<'_>> {
    let mut out = Vec::new();
    for (id, node) in &game.nodes {
        match node {
            Node::Action(a) => out.push(Uses {
                file: node_file(id),
                change: a.change.keys().collect(),
                require: a.require.iter().map(|(k, b)| (k, *b)).collect(),
            }),
            Node::Decision(d) => out.extend(d.options.iter().map(|o| Uses {
                file: node_file(id),
                change: o.change.keys().collect(),
                require: o.require.iter().map(|(k, b)| (k, *b)).collect(),
            })),
        }
    }
    out
}
