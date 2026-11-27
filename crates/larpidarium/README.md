# larpidarium (binary)

The command-line program. It is the only crate besides `larp-pdf` that does I/O: it reads the
game folder, calls the pure crates, and writes `dist/`.

| Command                  | Does                                                    | Phase |
| ------------------------ | ------------------------------------------------------- | ----- |
| `larpidarium check DIR`  | Validate a game folder and print errors                 | ✅    |
| `larpidarium schema DIR` | Editor autocomplete: JSON Schemas + `.taplo.toml` in the game | ✅    |
| `larpidarium build DIR`  | Write the static site to `dist/` (`--open`, `--out`)    | ✅    |
| `larpidarium serve DIR`  | Edit in the browser, live reload (`--port`, `--open`)   | ✅    |
| `larpidarium import F`   | Convert a taraxaplan JSON save into a game folder       | 2     |
| `larpidarium pdf DIR`    | Player and organizer sheets as PDF via tectonic (`--out`, `--tex-only`) | ✅ |

## `schema`: autocomplete in the editor

```sh
larpidarium schema my-game      # writes my-game/.schemas/*.schema.json and my-game/.taplo.toml
```

Open the game folder in VS Code with the **Even Better TOML** extension (or any editor using the
Taplo language server): keys are suggested with descriptions, and wrong keys or values (an unknown
`role`, a typo in a field name, a color that is not `#rrggbb`) are underlined while you type. An
existing `.taplo.toml` that larpidarium did not write is left alone; the command prints the rules to
add. Run it again after updating larpidarium. The schemas describe the structure; references between
files (does this person exist?) are still checked by `check`.

## `pdf`: printed sheets

```sh
larpidarium pdf examples/lost-abbey                 # dist/pdf/<id>.pdf + _all-players.pdf
larpidarium pdf examples/lost-abbey --tex-only      # only the .tex files, no tectonic needed
```

Writes a LaTeX document per player character and `_all-players` with every sheet, and the same
for organizer sheets (organizers, non-player characters and rooms) in `organizers/` and
`_all-organizers` (every sheet in one document, each starting on an odd page for duplex printing),
then compiles them with [tectonic](https://tectonic-typesetting.github.io).
The first run downloads the LaTeX packages it needs, later runs work offline. Without tectonic the
command stops with an install hint; the `.tex` files are already written.

The sheet has the same parts as the HTML one: who you are, destiny (with the mini graph drawn in
TikZ) and how you change. To restyle, copy a template from `crates/larp-render/templates/` into
`<game>/templates/`: `tex/player.tex.j2` → `player.tex.j2`, `tex/organizer.tex.j2` →
`organizer.tex.j2`, and likewise `player.html.j2` / `organizer.html.j2` for the web sheets. All game
text is escaped for you.

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
