#!/usr/bin/env bash
# Installs the developer tools used by `just` recipes, git hooks and CI.
# Safe to re-run: already installed tools are skipped.
set -euo pipefail

if ! command -v cargo-binstall >/dev/null; then
  echo "==> installing cargo-binstall (downloads prebuilt binaries for the rest)"
  cargo install --locked cargo-binstall
fi

echo "==> installing tools"
cargo binstall --no-confirm --locked \
  just \
  taplo-cli \
  typos-cli \
  cargo-deny \
  cargo-nextest \
  cargo-llvm-cov \
  cargo-machete \
  cargo-mutants

rustup component add llvm-tools-preview

echo "==> done. Run 'just setup' inside the repository to enable git hooks."
