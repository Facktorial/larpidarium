# CLAUDE.md

Rules for AI agents working in this repository. Humans: see `CONTRIBUTING.md`.

## Project

larpidarium reads a LARP game written as a folder of TOML files, validates it, and produces a
static website (action graph + sheets) and LaTeX PDFs for players and organizers.
Architecture: `docs/architecture.md`. Format: `docs/format.md`. Decisions: `docs/adr/`.
Specs for upcoming work: `docs/game-logic.md`, `docs/sheets.md`.

## Commands

- `just ci` — everything CI runs. **Must pass before you say a task is done.**
- `just check` — faster subset (fmt, typos, clippy, tests).
- `just fmt` — format Rust and TOML.
- `just test` — nextest + doc tests. `just coverage`, `just mutants -f <file>` for test quality.
- Never edit `Cargo.lock` by hand. Ask before adding a dependency, and add it to
  `[workspace.dependencies]` in the root `Cargo.toml`, then `dep.workspace = true` in the crate.

## Code rules

- Rust 2024 edition, toolchain pinned in `rust-toolchain.toml`, MSRV in `Cargo.toml`.
- **I/O only in `crates/larpidarium` and `crates/larp-pdf`.** `larp-model`, `larp-graph` and
  `larp-render` take values and return values: no `std::fs`, no network, no process spawning,
  no printing.
- No `unwrap`, `expect`, `panic!`, `todo!` outside tests. Library errors are `thiserror` enums;
  the binary reports them with `miette`, including file and line for TOML problems.
- Every public item has a doc comment. Crate docs are the crate's `README.md`
  (`#![doc = include_str!("../README.md")]`), and its Rust code blocks are doc-tested.
- Ids are `larp_model::Id` (kebab-case ASCII), never raw `String`s across crate boundaries.
- User text can be Czech or English. Never assume ASCII in text; escape it for HTML and LaTeX
  in `larp-render`, and only there.
- Prefer plain functions and data over traits and generics until there are two real uses.
- Lints live in `[workspace.lints]`. Don't add `#[allow]` without a comment saying why.

## Tests

- New behavior comes with tests that would fail if it broke. Use the `test-writer` agent.
- Snapshot generated output with `insta`; property-test escaping and parsing with `proptest`.
- `examples/` holds a sample game used as a test fixture; keep it valid.

## Docs

- Changing the TOML format → update `docs/format.md` and the example game in the same change.
- Changing commands → update `crates/larpidarium/README.md`.
- A decision someone might question later → new file in `docs/adr/`.
- User-visible change → line in `CHANGELOG.md` under `## Unreleased`.

## Workflow

Plan first for anything larger than a small fix, and wait for approval. After `just ci` passes,
run the `reviewer` agent, fix what it finds, then show the human a summary. Never commit or push
unless asked. Commits follow Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`).

## Game content

Game texts (`examples/`, `drafts/`) are written by `copywriter-plot` and `copywriter-voice`; they write
only into `drafts/`, and the main session applies drafts to game files (see `/writers-room`).
`examples/lost-abbey` must stay warning-free; `flawed-abbey` and `broken-abbey` contain planted
issues on purpose and are test fixtures.

## Third-party code

`web/` will contain code adapted from taraxaplan (MIT, © 2026 Karlord19). Keep its license file
`web/LICENSE-taraxaplan` and the notice at the top of adapted files.
