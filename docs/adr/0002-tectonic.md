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

## Implementation (phase 5)

- `larp-render` writes the `.tex` (minijinja template `templates/tex/player.tex.j2`); every value
  the template prints goes through `tex_escape` via the engine's formatter, not a filter.
- `larpidarium pdf` writes one document per player and `_all-players` with every sheet; `larp-pdf`
  runs tectonic on each. `--tex-only` works without tectonic, so the test suite checks the LaTeX
without it; a separate CI job installs tectonic and compiles the example games.
- Phase 6: organizer sheets use `templates/tex/organizer.tex.j2` the same way; `pdf` writes them to
  `organizers/<id>.tex` and `_all-organizers.tex`.
