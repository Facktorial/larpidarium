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
//! - A playthrough ends when nothing can happen. Its endings are the `ending` things it produced.
//!
//! Identical game states are explored once, so the work is bounded by the number of distinct
//! states, not the number of paths. Exploration stops at [`MAX_STATES`].

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{Game, Node, Ref};
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
}

impl Step {
    fn ready(&self, s: &State) -> bool {
        !s.done[self.node] && self.needs.iter().chain(&self.uses).all(|&t| s.have[t])
    }

    fn apply(&self, s: &mut State) {
        for &t in &self.uses {
            s.have[t] = false;
        }
        for &t in &self.gives {
            s.have[t] = true;
            s.ever[t] = true;
        }
        s.done[self.node] = true;
    }

    /// True when firing one of the two steps could take away something the other needs.
    fn competes_with(&self, other: &Step) -> bool {
        let touches = |a: &Step, b: &Step| {
            a.uses
                .iter()
                .any(|t| b.needs.contains(t) || b.uses.contains(t))
        };
        touches(self, other) || touches(other, self)
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
                        })
                        .collect(),
                }),
            }
        }
        sim
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
        }
    }

    /// Fires every ready action that competes with no other ready action, until none is left.
    /// Returns the ready actions that compete (their order is a choice).
    fn settle(&self, s: &mut State) -> Vec<usize> {
        loop {
            let ready: Vec<usize> = (0..self.actions.len())
                .filter(|&i| self.actions[i].ready(s))
                .collect();
            let (free, competing): (Vec<usize>, Vec<usize>) = ready.iter().partition(|&&i| {
                !ready
                    .iter()
                    .any(|&j| j != i && self.actions[i].competes_with(&self.actions[j]))
            });
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
                    "something it needs is never present at the same time, or is used up first",
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
                        "when “{}” is decided, what this option needs is never all present",
                        sim.node_titles[dec.node]
                    )),
                );
            }
        }
    }
}
