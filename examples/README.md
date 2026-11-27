# Example games

| Folder           | Purpose                                                                 |
| ---------------- | ----------------------------------------------------------------------- |
| `lost-abbey/`    | Clean reference game: passes `check` with no warnings, 3 endings.        |
| `kolobeh/`      | Koloběh: a noir port town of the mafia and a corporation over 4 nights; 8 players, 15 characters, 9 overlapping hero cycles (8-step story circle) placed on the world timeline. Shows `players.toml`, `cycles.toml`, `cycle`/`step` actions, `dies = <step>`, `after` (reincarnation), a character outside the cycles and sheet phases by step. Warning-free. |
| `evzentura/`    | Transcript of the Evžentura design notes as they are now: 5 floors, the characters with their organizers, only the interactions the notes describe. The warnings are the gaps still to design. |
| `flawed-abbey/`  | Same game with planted **warnings**. Builds; see them in "Stav hry".     |
| `broken-abbey/`  | `flawed-abbey` plus one planted **error**. `check` and `build` refuse it. |

Planted issues are marked with `# PLANTED ISSUE` comments in the files:

| Issue                         | Where                                         | Level |
| ----------------------------- | --------------------------------------------- | ----- |
| Unknown file                  | `notes.toml`                                  | warning |
| Item nobody needs or gives    | `items.toml` → `silver-chalice`               | warning |
| Info given but never needed   | `actions/ghost-appears.toml` → `ghost-chain`  | warning |
| Player without own decision, whereabouts, goals | `people/brother-tomas.toml`   | warning |
| Stuck playthrough             | `actions/crypt-door.toml` → option "Odejít"   | warning |
| Loop (actions wait on each other) | `actions/forge-key.toml`, `take-imprint.toml` | warning |
| Option that can never be chosen | `actions/hugo-verdict.toml` → "Zatknout padělatele" | warning |
| Unreachable ending (broken only) | `actions/pilgrimage.toml`                 | error |

```sh
cargo run -- check examples/flawed-abbey
cargo run -- build examples/flawed-abbey --open
cargo run -- check examples/broken-abbey
```
