# larpidarium (binary)

The command-line program. It is the only crate besides `larp-pdf` that does I/O: it reads the
game folder, calls the pure crates, and writes `dist/`.

| Command                  | Does                                                    | Phase |
| ------------------------ | ------------------------------------------------------- | ----- |
| `larpidarium check DIR`  | Validate a game folder and print errors                 | ✅    |
| `larpidarium schema`     | Print the JSON Schema for editor autocomplete           | 1     |
| `larpidarium build DIR`  | Write the static site to `dist/` (`--open`, `--out`)    | ✅    |
| `larpidarium serve DIR`  | Edit in the browser, live reload (`--port`, `--open`)   | ✅    |
| `larpidarium import F`   | Convert a taraxaplan JSON save into a game folder       | 2     |
| `larpidarium pdf DIR`    | Write player and organizer PDFs                         | 4     |

## `serve`: editing in the browser

```sh
larpidarium serve examples/lost-abbey --open     # http://localhost:7878
```

The same graph page as `build`, plus an **Editing** panel: new actions, decisions, items, info and
segments, and **Edit** / **Delete** in the details of every node. Each save writes the TOML file
(comments and formatting of untouched lines are kept), checks the game again and reloads the page
with the number of errors and warnings. Changes made in a text editor appear in the browser
within two seconds.

- Listens on 127.0.0.1 only. Writes need an `X-Larpidarium` header and a `localhost` Host, so other
  web pages cannot change your files.
- A game with errors is still shown, with the errors in "Game health", so you can fix it step by
  step. Only an unreadable `game.toml` shows a plain list of problems.
- "Save layout into the game" merges the nodes you moved into `layout.toml`; the rest keep the
  automatic layout.
- "Save map" in the **Mapa** tab writes `map.toml` as a whole: hand-written comments in that file
  are not kept. "Save floor plan as preset" writes `map-presets/<name>.toml`. In the static site
  both buttons download the file instead.
- Character sheets need a game without errors; while there are errors they show a message.
- Other web pages can neither change nor read your game while `serve` runs.
- People, rooms and categories are still edited in the TOML files. Ids cannot be renamed in the
  browser.

The page from `build` stays read-only: nothing on game day can change the game by accident.
