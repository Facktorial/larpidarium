# 0003 Reuse the taraxaplan graph renderer

- **Status:** accepted, 2026-11-27

## Context

[taraxaplan](https://github.com/Karlord19/taraxaplan) (MIT, © 2026 Karlord19) is a plain-JS LARP
planner with a graph view we want: owner-colored node cards, SVG lines with draggable bend points,
pan/zoom, fit, and filtering by person or item.

## Decision

Adapt `graph.js` into `web/` as an ES module that renders `graph.json` produced by `larp-graph`.
Drop its edit modals, JSON save/load and print export; TOML files and Rust templates replace them.
Keep dragging; "Export layout" downloads `layout.toml`.

## Consequences

- Keep `web/LICENSE-taraxaplan` and a notice at the top of adapted files (MIT requires it).
- Fix during the port: user text inserted with `innerHTML` unescaped, random ids, global state.
- `larpidarium import` converts existing taraxaplan JSON saves.
