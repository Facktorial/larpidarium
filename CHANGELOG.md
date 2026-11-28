# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions: [SemVer](https://semver.org).

## Unreleased

### Added

- Factions in `factions.toml`: members write `factions = [...]`, every member's sheet gets the
  faction's goals, secrets and sections, factions can own actions and items for all members, get
  an organizer sheet, and show up in the viewer (owner list, Frakce tab, People column).
- Relationships: `check` warns when a player character has a relation with another player character
  who has none back, and the viewer has a Vztahy tab with a relationship graph.
- Who knows what: a Znalosti tab in the viewer shows for every info who knows it from the start,
  who finds it out and who is told, and when; `check` warns when a sheet or a brief names an info
  (`[[info-id]]`) before the reader can know it.
- The example game lost-abbey has a faction (Konvent), shared blocks and both-way relationships.
- Shared texts and facts in `blocks.toml`: write `[[id]]` in any text to include a block or show
  the name of a person, room, item or info. Blocks with `show` appear on those players' sheets.
  `check` reports unknown links, blocks that include themselves and unused blocks; the viewer has
  a Texty tab, and the editor keeps the links as written.
- Writing status: `status = "draft" | "review" | "done"` and `writer` on actions, decisions and
  people. `check` prints the progress; Game health shows a progress bar, writers and what is still
  open, and filters the graph by status or writer; the browser editor sets both.
- `larpidarium schema DIR`: JSON Schemas of every game file and a `.taplo.toml`, so editors with
  Taplo (VS Code: Even Better TOML) autocomplete keys and underline mistakes. The schemas are also
  in `schemas/`, applied to the examples, checked in CI (`just schemas`).
- Organizer sheets for every organizer, non-player character and room: load per segment, items to
  have ready, and the running order with who takes part, organizer text, what players read and who
  provides each needed thing. As HTML (`organizers/<id>.html`, 📋 in the viewer) and PDF
  (`pdf/organizers/`, `_all-organizers.pdf`). `check` prints the load of each.
- `larpidarium pdf`: player sheets as LaTeX documents (one per player and `_all-players`), compiled
  to PDF with tectonic; `--tex-only` writes just the `.tex`. Same three parts as the HTML sheet,
  with the destiny mini graph in TikZ (wide graphs split into rows); customizable with
  `<game>/templates/player.tex.j2`. In `_all-players.pdf` every sheet starts on a front page with
  page 1, for duplex printing. Missing glyphs are reported; CI compiles the example sheets.

## phase-4 - 2026-11-27

### Added

- "From here on" in the graph: mark actions or decision options as happened (✓ in the details
  panel); what they needed and everything earlier in time is marked as past, impossible actions are
  greyed out, and with a character selected the bar lists what still awaits them.
- Map of the places (`map.toml`, **Mapa** tab): a floor plan with walls drawn along the grid lines
  and corridor tiles, rooms as draggable, resizable tiles; saved by `serve`, downloadable from the
  static site. Floor plans can be saved as presets (`map-presets/<id>.toml`) and loaded again. The drawing is
  kept in the browser until saved, and maps or presets can be loaded from a file.
- Viewer tabs: Places, Characters, Players (players × segments grid), Items & info, Segments and
  Problems tables next to the graph, sortable and filterable; rows link back into the graph.
- Hero cycles: `steps` in game.toml, `cycles.toml`, `cycle` + `step` on actions. A cycle is one
  hero's journey placed on the world timeline; several can overlap.
- Lives and reincarnation: `players.toml`, `played_by`, `enters`, `after`, `dies` (world segment
  or step of the hero's cycle), `outside_cycle` and `[[person.phase]]` (by segment or step) on
  people. `check` warns about acting outside a life, overlapping or missing characters of a player,
  heroes who do not die in their cycle and players who are never a hero; sheets show the life and a
  Part III with phases.
- Timeline layout for games with cycles: world segments as columns, cycles in lanes below,
  collapsible to a story-circle icon or opened as step columns or a story circle; merged lines with
  stubs between cycles and the world. Players tab as a timeline, new Cycles tab.
- `examples/evzentura`: transcript of the Evžentura design notes (five floors, characters and the interactions the notes describe); warnings mark what is still to be designed.
- Segments (`segments.toml`, `segment = "…"` on actions): acts, levels or loosely connected side
  plots. Each segment gets its own band and colored area in the graph, a filter in the sidebar
  that zooms to it, and dashed arrows where the plot crosses between segments.
- `larpidarium serve`: the graph on http://localhost with editing in the browser (actions,
  decisions, items, info, segments, saving the layout) and live reload. Edits keep comments in the
  TOML files.
- Character sheets: `build` writes `dist/players/<id>.html` per player. Part I from the person file,
  Part II "destiny" computed from the graph: timeline with sources ("you carry it", "you get it
  earlier", "you will receive or learn it") and a mini graph where other characters' nodes are
  anonymous. Template overridable per game via `templates/player.html.j2`. Linked from the graph
  sidebar (📄).
- `examples/flawed-abbey` (planted warnings) and `examples/broken-abbey` (planted error).
- Copywriter agents `copywriter-plot` and `copywriter-voice`, and the `/writers-room` command.
- Game logic level 3: `check` simulates every playthrough (all decision combinations, competing
  used-up items), prints reachable endings with an example route, and reports unreachable endings,
  stuck playthroughs, actions that never happen and options never chosen.
- Levels 4–5: warnings for players without actions, without own decisions, or with missing
  `backstory` / `whereabouts` / `goals`.
- Format: `uses` (item is used up), `brief` (text for players), `[[person.relation]]`,
  `sheet.whereabouts` / `sheet.appearance`.
- Browser: "Game health" panel with findings and reachable endings; "Uses up" lines on nodes;
  player and organizer text in the details panel.
- Example game: Marta's letter decision (show or burn), Hugo's verdict, third ending.
- `larpidarium build DIR [--out dist] [--open]`: static site with the action graph. Works offline
  (also as `file://`). Viewer adapted from taraxaplan: pan, zoom, drag nodes, bend lines,
  highlight a person's or item's storyline, details panel, Czech/English labels, light/dark mode.
- "Save layout.toml" in the browser; `build` uses saved positions and bends.
- `larp-graph`: edges derived from shared items, left-to-right layered layout.
- `larpidarium check DIR`: parses a game folder and reports errors and warnings with file, line,
  code snippet and fix hints (unknown references with "did you mean", duplicate ids, plot holes,
  loops, invalid colors).
- Game model in `larp-model`: actions, decisions, people, rooms, items, info, categories.
- Example game `examples/lost-abbey` (Czech).
- Workspace skeleton with five crates, CLI command stubs and `larp_model::Id`.
- Quality tooling: rustfmt, taplo, typos, clippy pedantic, cargo-deny, nextest, llvm-cov,
  machete, cargo-mutants; `justfile`, git hooks, GitHub Actions, Dependabot.
- Claude Code setup: `CLAUDE.md`, permissions, format hook, test-writer / reviewer / docs-keeper agents.
- Documentation: architecture, format draft, workflow guide, ADRs 0001–0005.
