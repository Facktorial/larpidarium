# 0006 Editing in the browser through a local server

- **Status:** accepted, 2026-11-27. Amends [0001](0001-static-site.md).

## Context

Authors want to add actions, items and segments in the browser instead of a text editor. A static
page cannot write files. Options were: keep the static page and export TOML snippets to paste
(clumsy), a WASM app with the File System Access API (Chromium only, no validation in Rust), or a
small local server.

## Decision

`larpidarium serve` is a synchronous `tiny_http` server bound to 127.0.0.1. It regenerates every
page from the game folder on each request, and writes edits with `toml_edit` so hand-written
comments survive. The viewer gets an edit UI only when the server says so (`window.LARP_EDIT`).
Writes require a custom header (forces a CORS preflight, which the server never grants) and a
`localhost` Host header (stops DNS rebinding). Reads are protected too: a foreign page could
otherwise include `/graph-data.js` as a script and read every secret of the game, so requests whose
`Sec-Fetch-Site` is `cross-site` or whose `Origin`/`Referer` is not localhost are refused. The
static `build` output stays read-only.

Edits are accepted even when they leave the game with errors (an ending added before the action
that reaches it), because authoring passes through such states; the loader keeps such a game as a
`draft` so the page can still show it. Only edits that cannot be expressed (invalid ids, files that
are not valid TOML) are refused.

## Consequences

- Two new dependencies: `tiny_http` and `toml_edit`.
- No async runtime, no file watcher: the page polls a content hash every 1.5 s, which is cheap for
  a folder of small files.
- Comments in front of keys and after values are kept. Decision options are updated by position:
  removing the first option moves its comment to the next one, and the last option's comment is
  dropped.
- "Save layout" merges only the nodes moved in the browser into `layout.toml` (other entries and
  comments stay; `[knees]` is replaced). A pinned node no longer follows the automatic layout, so
  after moving a node to another segment, delete its `[node.<id>]` entry or drag it into place.
- Character sheets are only served from a game without errors.
