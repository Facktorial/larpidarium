//! Level 3: simulate every playthrough of a game.
//!
//! Rules of the simulation:
//!
//! - The game starts with every `starting` item and info.
//! - A plain action happens as soon as everything it `needs` and `uses` is present. It happens
//!   at most once. `uses` removes the item, `gives` adds things.
//! - When two ready actions compete for an item that one of them uses up, the order is a choice.
//! - A decision is taken only when no plain action can happen any more. Every ready decision and
//!   every option whose needs are met is a separate branch. A decision is taken at most once.
//! - Counters start at their `start` value. An action or option is ready only when every counter
//!   it `require`s is in range; `change` adds to counters, stopping at their `min` and `max`.
//!   Two ready actions compete when one changes a counter the other requires, or both change a
//!   counter that someone requires and that can hit its `min` or `max` (then order matters).
//!   Counters nobody requires do not change the course of the game and are not followed.
//! - Two ready actions also compete when something one of them can lead to (through what it
//!   gives or the counters it changes, over any number of steps) competes with the other.
//! - A playthrough ends when nothing can happen. Its endings are the `ending` things it produced.
//!
//! Identical game states are explored once, so the work is bounded by the number of distinct
//! states, not the number of paths. Exploration stops at [`MAX_STATES`].

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{Bound, Counter, Game, Node, Ref};
use crate::{Diagnostic, Id};

/// Upper bound on explored game states.
pub const MAX_STATES: usize = 10_000;
/// How many stuck playthroughs are reported.
const MAX_STUCK_REPORTS: usize = 3;

/// Summary of all playthroughs of a game.
#[derive(Debug, Clone, Default)]
pub struct Playthroughs {
    /// Number of distinct final states found.
    pub end_states: usize,
    /// Exploration stopped at [`MAX_STATES`]; the results may be incomplete.
    pub truncated: bool,
    /// Reachable endings, each with one example sequence of choices that leads there.
    pub endings: BTreeMap<Id, Vec<String>>,
}

struct Step {
    node: usize,
    label: String,
    needs: Vec<usize>,
    uses: Vec<usize>,
    gives: Vec<usize>,
    /// (counter, amount, min, max, order matters: two changes to it do not commute)
    change: Vec<(usize, i32, i32, i32, bool)>,
    require: Vec<(usize, Bound)>,
}

struct Decision {
    node: usize,
    options: Vec<Step>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct State {
    have: Vec<bool>,
    ever: Vec<bool>,
    done: Vec<bool>,
    counters: Vec<i32>,
}

enum Choice {
    Action(usize),
    Option(usize, usize),
}

struct Sim<'a> {
    thing_ids: Vec<&'a Id>,
    ending: Vec<bool>,
    node_files: Vec<String>,
    node_titles: Vec<String>,
    actions: Vec<Step>,
    decisions: Vec<Decision>,
    /// `conflict[i][j]`: the order of actions `i` and `j` can matter.
    conflict: Vec<Vec<bool>>,
}

impl Step {
    fn ready(&self, s: &State) -> bool {
        !s.done[self.node]
            && self.needs.iter().chain(&self.uses).all(|&t| s.have[t])
            && self.require.iter().all(|(c, b)| b.allows(s.counters[*c]))
    }

    fn apply(&self, s: &mut State) {
        for &t in &self.uses {
            s.have[t] = false;
        }
        for &t in &self.gives {
            s.have[t] = true;
            s.ever[t] = true;
        }
        for &(c, amount, min, max, _) in &self.change {
            // `check` refuses min > max before simulating; `max(min)` keeps clamp from panicking.
            s.counters[c] = s.counters[c]
                .saturating_add(amount)
                .clamp(min, max.max(min));
        }
        s.done[self.node] = true;
    }

    /// True when firing this step can make `other` ready: it gives something `other` needs or
    /// uses, or changes a counter `other` requires.
    fn enables(&self, other: &Step) -> bool {
        self.gives
            .iter()
            .any(|t| other.needs.contains(t) || other.uses.contains(t))
            || self
                .change
                .iter()
                .any(|(c, ..)| other.require.iter().any(|(r, _)| r == c))
    }

    /// True when firing one of the two steps could take away something the other needs, or the
    /// order of their counter changes matters.
    fn competes_with(&self, other: &Step) -> bool {
        let touches = |a: &Step, b: &Step| {
            a.uses
                .iter()
                .any(|t| b.needs.contains(t) || b.uses.contains(t))
        };
        let counts = |a: &Step, b: &Step| {
            a.change.iter().any(|&(c, .., order)| {
                b.require.iter().any(|(r, _)| *r == c)
                    || (order && b.change.iter().any(|(x, ..)| *x == c))
            })
        };
        touches(self, other) || touches(other, self) || counts(self, other) || counts(other, self)
    }
}

impl<'a> Sim<'a> {
    fn new(game: &'a Game) -> Self {
        let things: Vec<_> = game.items.iter().chain(&game.info).collect();
        let index: HashMap<&Id, usize> = things
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.get_ref(), i))
            .collect();
        let counter_index: HashMap<&Id, usize> = game
            .counters
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.get_ref(), i))
            .collect();
        // Counters nobody requires cannot change what happens, so they are not followed.
        let required: HashSet<&Id> = game
            .nodes
            .values()
            .flat_map(|n| match n {
                Node::Action(a) => vec![&a.require],
                Node::Decision(d) => d.options.iter().map(|o| &o.require).collect(),
            })
            .flat_map(|m| m.keys().map(Ref::get_ref))
            .collect();
        let change = |m: &BTreeMap<Ref, toml::Spanned<i32>>| -> Vec<(usize, i32, i32, i32, bool)> {
            m.iter()
                .filter(|(k, _)| required.contains(k.get_ref()))
                .filter_map(|(k, v)| {
                    let i = *counter_index.get(k.get_ref())?;
                    let c = &game.counters[i];
                    Some((i, *v.get_ref(), c.min(), c.max(), false))
                })
                .collect()
        };
        let require = |m: &BTreeMap<Ref, Bound>| -> Vec<(usize, Bound)> {
            m.iter()
                .filter_map(|(k, b)| counter_index.get(k.get_ref()).map(|&i| (i, *b)))
                .collect()
        };
        let refs = |r: &[Ref]| -> Vec<usize> {
            r.iter()
                .filter_map(|x| index.get(x.get_ref()).copied())
                .collect()
        };

        let mut sim = Sim {
            thing_ids: things.iter().map(|t| t.id.get_ref()).collect(),
            ending: things.iter().map(|t| t.ending).collect(),
            node_files: Vec::new(),
            node_titles: Vec::new(),
            actions: Vec::new(),
            decisions: Vec::new(),
            conflict: Vec::new(),
        };
        for (node, (id, n)) in game.nodes.iter().enumerate() {
            sim.node_files.push(format!("actions/{id}.toml"));
            sim.node_titles.push(n.title().to_owned());
            match n {
                Node::Action(a) => sim.actions.push(Step {
                    node,
                    label: a.title.clone(),
                    needs: refs(&a.needs),
                    uses: refs(&a.uses),
                    gives: refs(&a.gives),
                    change: change(&a.change),
                    require: require(&a.require),
                }),
                Node::Decision(d) => sim.decisions.push(Decision {
                    node,
                    options: d
                        .options
                        .iter()
                        .map(|o| Step {
                            node,
                            label: format!("{} → {}", d.title, o.label),
                            needs: refs(&o.needs),
                            uses: refs(&o.uses),
                            gives: refs(&o.gives),
                            change: change(&o.change),
                            require: require(&o.require),
                        })
                        .collect(),
                }),
            }
        }
        sim.mark_clamping(game);
        sim.conflict = sim.conflicts();
        sim
    }

    /// Marks changes to counters that can hit a bound: their order matters.
    fn mark_clamping(&mut self, game: &Game) {
        let steps = || {
            self.actions
                .iter()
                .chain(self.decisions.iter().flat_map(|d| &d.options))
        };
        let clamps: Vec<bool> = game
            .counters
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (up, down) = steps().flat_map(|s| &s.change).filter(|x| x.0 == i).fold(
                    (0i64, 0i64),
                    |(u, d), x| {
                        let v = i64::from(x.1);
                        (u + v.max(0), d + v.min(0))
                    },
                );
                let start = i64::from(c.start());
                start + up > i64::from(c.max()) || start + down < i64::from(c.min())
            })
            .collect();
        for s in self
            .actions
            .iter_mut()
            .chain(self.decisions.iter_mut().flat_map(|d| &mut d.options))
        {
            for x in &mut s.change {
                x.4 = clamps[x.0];
            }
        }
    }

    /// Pairs of actions whose order can matter: they compete, or something one can lead to
    /// competes with the other. O(n³) in the number of actions (a transitive closure), which
    /// is fine for games of a few hundred actions.
    fn conflicts(&self) -> Vec<Vec<bool>> {
        let n = self.actions.len();
        // leads[i]: actions that `i` can make ready, directly or through others.
        let mut leads: Vec<Vec<bool>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| i != j && self.actions[i].enables(&self.actions[j]))
                    .collect()
            })
            .collect();
        for k in 0..n {
            for i in 0..n {
                if leads[i][k] {
                    for j in 0..n {
                        if leads[k][j] {
                            leads[i][j] = true;
                        }
                    }
                }
            }
        }
        let competes: Vec<Vec<bool>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| i != j && self.actions[i].competes_with(&self.actions[j]))
                    .collect()
            })
            .collect();
        let via = |i: usize, j: usize| (0..n).any(|k| leads[i][k] && competes[k][j]);
        (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| i != j && (competes[i][j] || via(i, j) || via(j, i)))
                    .collect()
            })
            .collect()
    }

    fn start(&self, game: &Game) -> State {
        let have: Vec<bool> = game
            .items
            .iter()
            .chain(&game.info)
            .map(|t| t.starting)
            .collect();
        State {
            ever: have.clone(),
            have,
            done: vec![false; self.node_files.len()],
            counters: game.counters.iter().map(Counter::start).collect(),
        }
    }

    /// Fires every ready action whose order matters to no other ready action, until none is
    /// left. Returns the ready actions in conflict (their order is a choice).
    fn settle(&self, s: &mut State) -> Vec<usize> {
        loop {
            let ready: Vec<usize> = (0..self.actions.len())
                .filter(|&i| self.actions[i].ready(s))
                .collect();
            let (free, competing): (Vec<usize>, Vec<usize>) = ready
                .iter()
                .partition(|&&i| !ready.iter().any(|&j| self.conflict[i][j]));
            if free.is_empty() {
                return competing;
            }
            for i in free {
                self.actions[i].apply(s);
            }
        }
    }

    fn choices(&self, s: &State, competing: Vec<usize>) -> Vec<Choice> {
        let mut out: Vec<Choice> = competing.into_iter().map(Choice::Action).collect();
        for (d, dec) in self.decisions.iter().enumerate() {
            for (o, opt) in dec.options.iter().enumerate() {
                if opt.ready(s) {
                    out.push(Choice::Option(d, o));
                }
            }
        }
        out
    }
}

/// Simulates `game` and reports unreachable endings, stuck playthroughs and dead options.
pub(crate) fn check(game: &Game, diags: &mut Vec<Diagnostic>) -> Playthroughs {
    let sim = Sim::new(game);
    let has_endings = sim.ending.iter().any(|&e| e);

    let mut result = Playthroughs::default();
    let mut visited: HashSet<State> = HashSet::new();
    let mut taken: HashSet<(usize, usize)> = HashSet::new();
    let mut happened = vec![false; sim.node_files.len()];
    let mut stuck_paths: Vec<Vec<String>> = Vec::new();
    let mut stack = vec![(sim.start(game), Vec::<String>::new())];

    while let Some((mut state, path)) = stack.pop() {
        let competing = sim.settle(&mut state);
        if !visited.insert(state.clone()) {
            continue;
        }
        if visited.len() > MAX_STATES {
            result.truncated = true;
            break;
        }
        for (n, done) in state.done.iter().enumerate() {
            happened[n] |= *done;
        }
        let choices = sim.choices(&state, competing);
        if choices.is_empty() {
            result.end_states += 1;
            let mut reached_any = false;
            for (t, &ever) in state.ever.iter().enumerate() {
                if ever && sim.ending[t] {
                    reached_any = true;
                    result
                        .endings
                        .entry(sim.thing_ids[t].clone())
                        .or_insert_with(|| path.clone());
                }
            }
            if has_endings && !reached_any && stuck_paths.len() < MAX_STUCK_REPORTS {
                stuck_paths.push(path);
            }
            continue;
        }
        for choice in choices {
            let mut next = state.clone();
            let mut next_path = path.clone();
            match choice {
                Choice::Action(i) => {
                    let step = &sim.actions[i];
                    step.apply(&mut next);
                    next_path.push(step.label.clone());
                }
                Choice::Option(d, o) => {
                    let step = &sim.decisions[d].options[o];
                    step.apply(&mut next);
                    taken.insert((d, o));
                    next_path.push(step.label.clone());
                }
            }
            stack.push((next, next_path));
        }
    }

    report(game, &sim, &result, &taken, &happened, &stuck_paths, diags);
    result
}

fn report(
    game: &Game,
    sim: &Sim<'_>,
    result: &Playthroughs,
    taken: &HashSet<(usize, usize)>,
    happened: &[bool],
    stuck: &[Vec<String>],
    diags: &mut Vec<Diagnostic>,
) {
    if result.truncated {
        diags.push(Diagnostic::warning(
            "game.toml",
            None,
            format!("stopped simulating after {MAX_STATES} game states; playthrough results are incomplete"),
        ));
    }
    let incomplete = result.truncated;

    for (thing, file) in game
        .items
        .iter()
        .map(|t| (t, "items.toml"))
        .chain(game.info.iter().map(|t| (t, "info.toml")))
    {
        if thing.ending && !incomplete && !result.endings.contains_key(thing.id.get_ref()) {
            diags.push(
                Diagnostic::error(
                    file,
                    Some(thing.id.span()),
                    format!(
                        "ending `{}` cannot be reached in any playthrough",
                        thing.id.get_ref()
                    ),
                )
                .with_help("check the decisions and items on the way to the action that gives it"),
            );
        }
    }

    for path in stuck {
        let route = if path.is_empty() {
            "without any decision".to_owned()
        } else {
            format!("after: {}", path.join("; "))
        };
        diags.push(
            Diagnostic::warning(
                "game.toml",
                None,
                format!("a playthrough gets stuck without reaching any ending, {route}"),
            )
            .with_help("give the players a way forward from that state, or add an ending there"),
        );
    }

    if incomplete {
        return;
    }
    for step in &sim.actions {
        if !happened[step.node] {
            diags.push(
                Diagnostic::warning(
                    &sim.node_files[step.node],
                    None,
                    format!("“{}” can never happen in any playthrough", step.label),
                )
                .with_help(
                    "something it needs is never present at the same time, is used up first, or a counter it requires is never in range",
                ),
            );
        }
    }
    for (d, dec) in sim.decisions.iter().enumerate() {
        for (o, opt) in dec.options.iter().enumerate() {
            if !taken.contains(&(d, o)) {
                diags.push(
                    Diagnostic::warning(
                        &sim.node_files[dec.node],
                        None,
                        format!("option “{}” can never be chosen", opt.label),
                    )
                    .with_help(format!(
                        "when “{}” is decided, what this option needs is never all present (or a counter it requires is out of range)",
                        sim.node_titles[dec.node]
                    )),
                );
            }
        }
    }
}
