# larp-render

Fills minijinja templates with the game model.

- `templates/html/` and `templates/tex/`, loaded at runtime so organizers can restyle sheets.
- Two audiences: **player sheets** (one per character) and **organizer sheets** (schedule of
  everything a person or room owns).
- Czech and English labels, chosen by `language` in `game.toml`.
- LaTeX escaping of user text lives here ([`tex_escape`], applied to every printed value by the
  template engine, so a template cannot forget it).
- **No I/O:** returns strings.

Status: player sheets as HTML (`player_sheet_html`) and LaTeX (`player_sheets_tex`, one document
for one or many people) and organizer sheets (`organizer_sheet_html`, `organizer_sheets_tex`) work.

```
use larp_render::tex_escape;
assert_eq!(tex_escape("Kuchařka & syn, 100 %"), r"Kuchařka \& syn, 100 \%");
```
