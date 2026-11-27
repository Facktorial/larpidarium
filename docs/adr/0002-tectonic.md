# 0002 Tectonic for LaTeX

- **Status:** accepted, 2026-11-27

## Context

PDFs are made with LaTeX. No TeX distribution is installed on the development machine and CI
runners don't have one either.

## Decision

Use the `tectonic` program (XeTeX engine, downloads packages on demand), called as a process from
`larp-pdf`. Templates use `fontspec` and `polyglossia`/`babel` for Czech.

## Consequences

- One small binary to install; reproducible in CI.
- Unicode and Czech diacritics work without input encoding tricks.
- Packages that require LuaTeX are not available.
