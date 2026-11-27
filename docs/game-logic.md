# Game logic checks

`larpidarium check` validates in levels. Each level runs only when the previous one has no errors,
so the reported problems are root causes, not knock-on effects.

| Level | What it proves                                   | Status      |
| ----- | ------------------------------------------------ | ----------- |
| 1     | Files are valid: syntax, keys, ids, references   | ✅ done     |
| 2     | Plot flow: every need has a source, no loops     | ✅ done     |
| 3     | Playthroughs: decisions lead to endings          | ✅ done     |
| 4     | People: everyone has something to do             | 🚧 partly   |
| 5     | Sheets: complete and spoiler-safe                | 🚧 partly   |

## Level 1: structure (done)

TOML syntax, unknown keys, file names as ids, unknown or wrong-kind references with "did you mean",
duplicate ids, colors.

## Level 2: flow (done)

Needed but never given (error), given but never needed (warning, unless `ending = true`), unused
items, actions that can never happen because their needs form a loop.

## Level 3: playthroughs (done)

Simulate the game: start with `starting` things, run every action whose needs are met, and at each
decision branch into one path per option (options are mutually exclusive).

- **error:** an `ending` reachable on no path;
- **warning:** a path that reaches no ending (players get stuck);
- **warning:** a decision option that changes nothing reachable (a fake choice);
- **info:** list of paths: which choices lead to which endings.
- Items listed in `needs` stay; items in `uses` are removed. Two ready actions that compete for a
  used-up item are a choice, so both orders are explored.
- Plain actions happen as soon as they can; decisions are taken only when no plain action can
  happen any more, and every ready decision and enabled option is explored.

Identical states are explored once; exploration stops after 10 000 states with a warning.
`check` prints every reachable ending with one example route.

## Level 4: people (partly done)

- ✅ **warning:** a player who owns no action or decision (nothing to do);
- ✅ **warning:** a player with no decision of their own (no agency);
- ✅ **error:** a relation pointing at an unknown person;
- planned **warning:** a player idle for a long stretch of layers in the middle of the game;
- ✅ **info:** load per organizer, non-player character and room: `check` prints a `load:` line
  (actions and the busiest segment); organizer sheets show the table per segment.

## Level 5: sheets (partly done)

- ✅ **warning:** player without `backstory`, `goals` or `whereabouts`;
- ✅ **warning:** player with an empty destiny (same as owning no action);
- planned with the sheets: a player sheet that would show info the character never receives.

## Where results appear

Terminal output, and the "Game health" panel at the top of the browser sidebar: findings (click
one to select its node), reachable endings with their routes (click one to highlight its storyline).
