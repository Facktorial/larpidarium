# Example games

| Folder           | Purpose                                                                 |
| ---------------- | ----------------------------------------------------------------------- |
| `lost-abbey/`    | Clean reference game: passes `check` with no warnings, 3 endings.        |
| `evzentura/`    | Bigger game from the organizers' design notes: a five-floor team adventure in Hell, 5 segments, 40 actions, 34 people (organizers play all characters, no player sheets). Warning-free. |
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
