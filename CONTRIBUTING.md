# Contributing

## Setup

1. Install Rust with [rustup](https://rustup.rs). The right toolchain version installs itself
   from `rust-toolchain.toml` on the first `cargo` command.
2. `./scripts/install-tools.sh` installs the checkers.
3. `just setup` enables the git hooks in `.githooks/`.
4. Editor: rust-analyzer, plus the "Even Better TOML" (taplo) extension for TOML autocomplete.

## Everyday commands

| Command          | When                                                   |
| ---------------- | ------------------------------------------------------ |
| `just fmt`       | Format everything                                      |
| `just check`     | Quick confidence while working                         |
| `just ci`        | Before pushing; identical to CI                        |
| `just coverage`  | See which lines tests don't reach                      |
| `just mutants`   | See which bugs tests wouldn't catch (slow)             |

## Quality gates

| Stage        | Runs                                              | Where              |
| ------------ | ------------------------------------------------- | ------------------ |
| every edit   | rustfmt, taplo fmt                                | Claude Code hook   |
| pre-commit   | fmt check, taplo check, typos                     | `.githooks/`       |
| pre-push     | clippy, tests, cargo-deny                         | `.githooks/`       |
| CI           | all above + coverage, machete, docs, MSRV         | GitHub Actions     |
| weekly       | cargo-mutants, Dependabot                         | GitHub Actions     |

A hook can be skipped once with `--no-verify`, but CI runs the same checks.

## Branches and commits

- Work on a branch, open a pull request to `main`; CI must be green.
- [Conventional Commits](https://www.conventionalcommits.org): `feat:`, `fix:`, `docs:`, `test:`,
  `refactor:`, `chore:`.
- Add a line to `CHANGELOG.md` under `## Unreleased` for user-visible changes.

## Where things go

See [`docs/architecture.md`](docs/architecture.md). Short version: file and network access only in
the `larpidarium` binary and `larp-pdf`; everything else is pure and unit-tested.
