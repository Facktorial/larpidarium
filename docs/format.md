# Game format (draft)

> `larpidarium check` implements this format. Keys may still change before version 1.0.
> A complete example: [`examples/lost-abbey/`](../examples/lost-abbey/).

A game is a folder. File names equal ids. Ids are lowercase ASCII, digits and single hyphens
(`abbey-gate`, `brother-anselm`). Names, titles and texts can be any language.

```text
my-game/
├── game.toml
├── actions/        one file per action or decision
├── people/         one file per person (character or organizer)
├── items.toml      items and their categories
├── rooms.toml
├── info.toml       pieces of information (rumors, secrets) that work like items
├── segments.toml   optional: acts, levels or side plots
├── players.toml    optional: real participants, when they play several characters in turn
├── cycles.toml     optional: hero cycles placed on the world timeline
├── map.toml        optional: floor plan and room tiles, written by the browser's map view
├── map-presets/    optional: saved floor plans, <id>.toml
├── layout.toml     optional, written by "Save layout" in the browser
└── templates/      optional: player.html.j2, organizer.html.j2, player.tex.j2, organizer.tex.j2

All files can also be edited from the browser with `larpidarium serve` (see below). Hidden files
(`.taplo.toml`, `.schemas/`) are ignored; `larpidarium schema` writes them for editor autocomplete.
```

## game.toml

```toml
[game]
title    = "Ztracené opatství"
language = "cs"            # "cs" or "en": labels on sheets and pages
```

## Actions and decisions

- `needs`: must be present; **stays** afterwards (reading a letter).
- `uses`: must be present; **is gone** afterwards (burning a letter). Only items can be used up.
- `gives`: appears afterwards.
- `text` is for organizers, `brief` is what players read in their destiny.

```toml
# actions/search-gate.toml
[action]
title  = "Prohledat bránu"
owners = ["brother-anselm"]       # people or rooms
needs  = []
gives  = ["rusty-key"]
text   = """
Prší. Brána je zamčená řetězem.
"""
brief  = "U brány se dá najít něco zajímavého."
```

```toml
# actions/crypt-door.toml
[decision]
title  = "Otevřít kryptu?"
owners = ["crypt"]

[[decision.option]]
label = "Otevřít"
needs = ["rusty-key"]
gives = ["abbot-bones"]

[[decision.option]]
label = "Spálit klíč v kovárně"
uses  = ["rusty-key"]          # the key is gone afterwards
gives = ["rumor-ghost"]
```

A decision is taken once, choosing one option.

Both `[action]` and `[decision]` may have `segment = "<segment id>"` (see below).

## Segments

Segments split a game into parts: the acts of an evening, the levels of an adventure, or side
plots that barely touch the rest. They change nothing in the logic; they organize the graph:
every segment is laid out in its own horizontal band (in the order of `segments.toml`), drawn as
a colored area, and arrows that cross from one segment to another are dashed. Actions without a
segment form the last band.

```toml
# segments.toml
[[segment]]
id    = "arrival"
name  = "I. Příjezd"
color = "#5b8def"                # optional, #rrggbb
text  = "Večer prvního dne."     # optional, for organizers
group = "Cyklus 1"               # optional: segments with the same group belong together

[[segment]]
id   = "night"
name = "II. Krypta"
```

```toml
# actions/read-letter.toml
[action]
segment = "arrival"
title   = "Přečíst dopis"
```

Segment ids share the id space with everything else: a segment cannot be called `crypt` if a room
already is.

## Editing in the browser

`larpidarium serve <game folder>` opens the graph with an editing panel. Each save rewrites only
the affected file (an action file, `items.toml`, `info.toml`, `segments.toml` or `layout.toml`),
keeping comments in front of keys and after values. People, rooms and categories are edited in the
files.

## People

```toml
# people/brother-anselm.toml
[person]
name    = "Bratr Anselm"
role    = "player"             # "player", "organizer" or "npc"
color   = "#c0392b"
goals   = ["Udržet kryptu zavřenou"]
secrets = ["Pohřbil opata"]

[person.sheet]                 # Part I of the character sheet
appearance  = "Vysoký, shrbený."
whereabouts = "Na začátku hry jsi v kapli."   # where the character is when the game starts
backstory   = """..."""

[[person.relation]]            # one per person this character has an opinion about
with = "novice-marta"
text = "Bojíš se, co ví."
```

Players need `backstory`, `whereabouts` and `goals`; `check` warns when one is missing.

## Hero cycles, lives and reincarnation

For games where small personal stories run inside a bigger world: the **world** is laid out on world
segments (its time), and a **cycle** is one hero's journey through fixed **steps**, placed on the world
timeline. Several cycles can run at the same time; other characters can join a cycle as helpers.

```toml
# game.toml
[game]
title = "Koloběh"
language = "cs"
steps = ["Ty", "Potřeba", "Odchod", "Hledání", "Nález", "Cena", "Návrat", "Změna"]
```

```toml
# cycles.toml
[[cycle]]
id    = "c-dokar"
name  = "Dokařův cyklus"
hero  = "dokar"        # the character who goes through it
from  = "noc-1"        # world segment where it starts
to    = "noc-2"        # optional, where it ends (default: from)
color = "#c0392b"      # optional, default: the hero's color
text  = "…"            # optional, for organizers
```

```toml
# actions/dokar-odchod.toml
[action]
cycle = "c-dokar"      # instead of `segment`
step  = 3              # 1 … number of steps
title = "Odchod z přístavu"
owners = ["dokar", "don"]
```

A cycle spanning `n` world segments divides them evenly between its steps, so "step 6 of a cycle in
night 2" is a definite moment on the world timeline.

### Lives

```toml
# players.toml
[[player]]
id   = "hrac-4"
name = "Hráč 4"
```

```toml
# people/zlodejka.toml
[person]
name          = "Markéta „Kavka“ Šimková"
role          = "player"
played_by     = "hrac-4"
after         = "dokar"     # starts when that character dies (reincarnation)
# enters      = "noc-2"     # …or from the start of a world segment; omit both = from the start
dies          = 6           # step of their own cycle; or a world segment id: dies = "noc-3"
outside_cycle = false       # true: never a hero (a boss, a god)

[[person.phase]]            # the sheet from this moment on
step    = 4                 # a step of their own cycle, or: segment = "noc-2"
goals   = ["Vrátit klíč"]   # optional, replaces goals
secrets = ["…"]             # optional, replaces secrets
[person.phase.sheet]        # optional, replaces or adds sheet sections
whereabouts = "Schováváš se v přístavu."
```

Player and cycle ids share the id space with everything else. `check` reports:

- errors: a cycle action without `step` or with both `segment` and `cycle`, a step that does not
  exist, cycles without `steps`, `dies = <step>` for a character who is not a hero, a death before the
  entry, both `enters` and `after`;
- warnings: acting outside one's life, phases outside it or twice at one moment, a hero who does not
  die in their cycle or is the hero of several, a player playing two characters at once or having no
  character at some moment, a player who is never a hero (when the game has cycles), `after` someone
  who never dies or is played by someone else.

### How it is shown

With cycles the graph switches to a **timeline layout**: world segments are columns left to right,
world actions on top, and below them each cycle sits in a lane under the time it happens. A cycle is
**collapsed** to a small story-circle icon (filled slices = steps with actions, † = the hero's death);
its buttons open it as **step columns** (for editing, nodes can be dragged) or as a **story circle**
(for presenting). Lines between a cycle and the world are merged at the cycle's frame; inside an open
cycle, labelled stubs continue to the nodes. `#cycles=c-dokar:columns,c-don:ring` in the URL opens
cycles directly.

The **Hráči** tab shows a timeline: players × world time, a bar per character, the cycle as a block
with one tick per step, † at the death. The **Cykly** tab lists cycles with hero, time, death and
helpers. Sheets show who plays the character, when they enter and die, and Part III "How you change"
with the phases in time order.

## Items, rooms, info

```toml
# items.toml
[[category]]
id    = "key"
color = "#d4a017"

[[item]]
id       = "rusty-key"
name     = "Rezavý klíč"
category = "key"
owners   = ["brother-anselm"]
starting = true                # exists at game start, no action needs to give it
text     = "Velký železný klíč od krypty."
```

Item keys: `id`, `name` (required), `category`, `owners`, `starting`, `ending`, `text`.
`ending = true` marks an outcome of the game, so nothing has to need it.

`rooms.toml` uses `[[room]]` with `id`, `name`, `color`. `info.toml` uses `[[info]]` with the same
keys as items; info can be needed and given exactly like items.

```toml
# info.toml
[[info]]
id     = "ending-confession"
name   = "Konec: přiznání"
text   = "Anselm se přizná a je předán biskupovi."
ending = true
```

## Map of the places

`map.toml` is a floor plan on a square grid: **walls** along the grid lines, **corridor** tiles, and
where each room sits. The **Mapa** tab draws it (black background, dotted grid, white walls, grey
corridors) and edits it: drag rooms, drag a room's corner to resize it, draw walls by dragging along
grid lines, paint corridors, erase both. With `serve` the map is saved into `map.toml` (the whole
file is rewritten, so comments in it are not kept); the static site offers it as a download. Rooms
that are not on the map yet are placed in a row below it.

While you draw, the map is kept in the browser (per game title), so closing the tab loses nothing;
"Zahodit rozkreslené" goes back to the game's `map.toml`. "Načíst ze souboru…" opens a downloaded
`map.toml` or preset file, so on the static site you can store a map as a file and continue later.

```toml
# map.toml
[map]
cells = [               # rows top to bottom: `.` empty, `=` corridor
  "",
  ".....=",
]
walls = [               # [x1, y1, x2, y2] between grid points, horizontal or vertical
  [1, 1, 13, 1],
  [1, 1, 1, 6],
]

[map.rooms]             # x, y = top-left cell; w, h = size in cells (default 1)
bar     = { x = 2, y = 2, w = 3, h = 2 }
pristav = { x = 6, y = 2, w = 3, h = 2 }
```

### Floor plan presets

A floor plan (walls and corridors, no rooms) can be saved as a preset and loaded again, e.g. the
venue's layout. Presets are files in the game folder, `map-presets/<id>.toml`, with the same `[map]`
table without `rooms`. "Uložit půdorys jako předlohu" in the Mapa tab writes one (with `serve`; the
static site downloads it), "Načíst půdorys…" replaces the current walls and corridors with a preset
and keeps the rooms. Copy the file to another game's `map-presets/` to reuse it.

`check` reports unknown tiles, walls that are not horizontal or vertical (also in presets) and
unknown rooms as errors, overlapping rooms as warnings.

## What `check` reports

| Problem                                                       | Level   |
| ------------------------------------------------------------- | ------- |
| TOML syntax error, unknown key, missing required key           | error   |
| File name that is not a valid id                               | error   |
| Action file with both or neither `[action]` / `[decision]`     | error   |
| Reference to an unknown id (with "did you mean" suggestion)    | error   |
| Reference of the wrong kind (a person used as an item, …)      | error   |
| The same id used twice                                         | error   |
| Item or info needed, but no action gives it and not `starting` | error   |
| Color not in `#rrggbb` form                                    | error   |
| Item or info given but never needed (and not `ending`)         | warning |
| Item or info not used by any action                            | warning |
| Ending that no playthrough can reach                            | error   |
| Playthrough that gets stuck without any ending                  | warning |
| Action that never happens / option never chosen in any playthrough | warning |
| Player with no action, or no decision of their own              | warning |
| Player sheet missing `backstory`, `whereabouts` or `goals`      | warning |
| Unknown file in the game folder                                | warning |

Warnings don't stop the game from being generated; errors do.
