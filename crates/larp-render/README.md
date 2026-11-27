# larp-render

Fills minijinja templates with the game model.

- `templates/html/` and `templates/tex/`, loaded at runtime so organizers can restyle sheets.
- Two audiences: **player sheets** (one per character) and **organizer sheets** (schedule of
  everything a person or room owns).
- Czech and English labels, chosen by `language` in `game.toml`.
- LaTeX escaping of user text lives here and is property-tested.
- **No I/O:** returns strings.

Status: HTML player sheets work (`player_sheet_html`); LaTeX and organizer sheets are planned.
