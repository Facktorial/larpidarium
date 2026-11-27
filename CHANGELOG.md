# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions: [SemVer](https://semver.org).

## Unreleased

### Added

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
