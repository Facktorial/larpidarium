# 0005 A folder of TOML files, one per action and person

- **Status:** accepted, 2026-11-27

## Context

Several organizers edit a game at the same time, in git.

## Decision

`game.toml` plus one file per action and per person; small lists (items, rooms, info) in one file
each. The file name is the id. A JSON Schema generated from `larp-model` gives editors
autocomplete and inline errors.

## Consequences

- Small diffs, few merge conflicts.
- Renaming an id means renaming a file and updating references; `check` reports every broken one.
