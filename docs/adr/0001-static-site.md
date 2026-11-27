# 0001 Static site instead of a running server

- **Status:** accepted, 2026-11-27

## Context

The output is used during a LARP, often without reliable internet. Options were a static site,
a live web server (axum) and a Rust WASM app (Leptos).

## Decision

`larpidarium build` generates plain files in `dist/`. `larpidarium serve` is only for authoring:
it rebuilds on file changes and reloads the browser.

## Consequences

- The game works from a USB stick or any static host (GitHub Pages).
- No browser can write back to TOML; edits made in the browser (node positions) are exported as a
  file that the organizer saves into the game folder.
