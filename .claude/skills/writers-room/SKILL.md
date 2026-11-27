---
name: writers-room
description: Two copywriter agents (copywriter-plot and copywriter-voice) write game content in parallel, critique and improve each other's drafts for a few rounds, then the result is applied to the game and checked. Use when the user asks to write or rework LARP content with both copywriters.
argument-hint: <game folder> <what to write> [rounds=2]
---

# Writers' room

Arguments: `$ARGUMENTS`: a game folder (e.g. `examples/lost-abbey`), a task in plain words
(e.g. "add a fourth player character, a herbalist with a secret"), and optionally `rounds=N`
(default 2, maximum 4).

## 0. Prepare

1. Pick a short kebab-case topic name from the task, e.g. `herbalist`.
2. Drafts go to `drafts/<game-name>/<topic>/`. If the folder exists, continue with the next round
   number instead of starting over.
3. Run `cargo run -q -- check <game folder>` and note the warning count; the result must not add
   warnings.

## 1. Round 1: both write, in parallel

Start **both agents in the same message** (two Agent tool calls side by side, so they run in
parallel), each with:

- the game folder, the task, the drafts folder, and "this is round 1, write `<plot|voice>-r1.md`".

Wait until both finish. Tell the user in one line each what the two proposals are.

## 2. Rounds 2..N: each responds to the other, in parallel

Again start both agents in one message:

- `copywriter-plot`: "Round <N>. Read `voice-r<N-1>.md` and your own `plot-r<N-1>.md`, respond to
  the voice draft, and write an improved `plot-r<N>.md`."
- `copywriter-voice`: the mirror image.

Stop early when both drafts say `Converged: yes`.

## 3. Merge and apply

1. Read the last two drafts. Prefer the plot draft's structure (actions, items, decisions, owners)
   and the voice draft's texts (`brief`, `text`, backstories, names). Where they disagree and the
   drafts give no answer, ask the user.
2. Apply the merged TOML to the game folder.
3. Run `cargo run -q -- check <game folder>`. Fix errors yourself. If new warnings appear, fix them
   or explain why they are intended.
4. Run `cargo run -q -- build <game folder>` so the sheets are regenerated.
5. Show the user: what changed (files), the warning count before and after, and the sheet(s) to look
   at (`dist/players/<id>.html`). Do not commit.

## Rules

- The agents never edit game files; only step 3 does.
- Keep every round's drafts; they are the history of the idea.
- Respect the user's language: game text in the game's language, conversation in the user's.
