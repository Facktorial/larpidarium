# 0009 Hand-written JSON Schemas for editor autocomplete

- **Status:** accepted, 2026-11-27

## Context

Authors edit the TOML files in a text editor. Taplo (VS Code: Even Better TOML) can suggest keys
and flag mistakes while typing if it has a JSON Schema for each kind of file. The format has eleven
kinds of files; `larp-model` already rejects unknown keys (`deny_unknown_fields`) and gives
diagnostics with file and line.

## Decision

`larpidarium schema DIR` writes the schemas, written by hand with `serde_json::json!` in
`crates/larpidarium/src/schema.rs`, to `DIR/.schemas/` together with a `.taplo.toml` that maps them
to file globs. The schemas are not generated from the model types (`schemars`).

- No new dependency, and the model types stay free of schema attributes.
- Descriptions are written for authors ("what players read about it on their sheet") instead of
  Rust doc comments written for developers.
- `Spanned<T>` and `Ref` would need custom `JsonSchema` implementations anyway.

To keep them from drifting:

- a test walks every key in the example games and fails if a schema does not describe it;
- a copy lives in `schemas/` and a test fails when it is stale;
- `just schemas` (also run in CI) checks the examples against the schemas with `taplo check`.

## Consequences

- A new field in the model must also be added to `schema.rs`. The tests catch it once an example
  uses the field, and docs/format.md already requires that.
- The schemas check structure only. References between files (does this person exist?) remain the
  job of `check`.
- Hidden files in a game folder (`.taplo.toml`) are skipped when the game is read.
