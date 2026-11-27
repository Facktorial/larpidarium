# Every check runs through these recipes, so you, git hooks, Claude and CI run the same commands.
# `just` lists them. Install the tools once with ./scripts/install-tools.sh

set shell := ["bash", "-euo", "pipefail", "-c"]

# list recipes
default:
    @just --list

# one-time setup after cloning: enable git hooks
setup:
    git config core.hooksPath .githooks
    @echo "git hooks enabled (.githooks/)"

# format all Rust and TOML files
fmt:
    cargo fmt --all
    taplo fmt

# check formatting without changing files
fmt-check:
    cargo fmt --all --check
    taplo fmt --check

# example games against the JSON Schemas in schemas/ (rules in .taplo.toml)
schemas:
    taplo check "examples/*/*.toml" "examples/*/*/*.toml"

# spelling in code, docs and file names
typos:
    typos

# clippy with all warnings as errors
lint:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# unit, integration and doc tests
test:
    cargo nextest run --workspace --all-features --locked --no-tests=pass
    cargo test --doc --workspace --all-features --locked

# licenses, security advisories, banned and duplicate crates
deny:
    cargo deny --locked check

# dependencies declared but never used
machete:
    cargo machete

# API docs, failing on broken links or missing docs
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked

# HTML coverage report in target/llvm-cov/html
coverage:
    cargo llvm-cov nextest --workspace --all-features --no-tests=pass --html
    @echo "open target/llvm-cov/html/index.html"

# mutation testing: do the tests notice when code is broken? (slow)
mutants *ARGS:
    cargo mutants --workspace {{ARGS}}

# fast checks, used by the pre-push hook
check: fmt-check typos lint test

# everything CI runs
ci: fmt-check typos schemas lint test deny machete doc
