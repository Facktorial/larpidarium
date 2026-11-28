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
├── blocks.toml     optional: shared texts and facts, used as [[id]] in any text
├── factions.toml   optional: groups of people (families, gangs, guilds)
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

Relationships between two player characters should go both ways: when one has a
`[[person.relation]]` about the other and the other has none back, `check` warns (relationships
with NPCs and organizers, or between characters whose lives never overlap, may be one-sided). The viewer's **Vztahy** tab draws all relationships as a
graph: a solid line where both have a view of each other, a dashed arrow where only one does; hover a
line for the texts, click a person to show only their relationships.

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

## Factions (`factions.toml`)

A faction is a group of people: a family, a gang, a guild, a crew.

```toml
[[faction]]
id = "montekove"
name = "Montekové"
color = "#2c3e50"
text = "Pro organizátory: drží přístav, dluží Kapuletům."
goals = ["Udržet přístav v rodině."]
secrets = ["Otec Montek prodal sklad Kapuletům."]

[faction.sheet]
history = "Montekové drží přístav od dob prvního majáku…"
```

```toml
# people/romeo.toml
[person]
name = "Romeo"
role = "player"
factions = ["montekove"]
```

- **Members** name the faction in their person file. A person can be in several factions.
- **Sheets.** Every member's sheet gets a "Tvá frakce" section: the other members, shared goals,
  shared secrets and the faction's `sheet` sections.
- **Ownership.** A faction can be an owner of actions, decisions and items (`owners =
  ["montekove"]`): it counts for every member, so the action is in every member's destiny and the
  item on every member's sheet. Blocks can be shown to a faction (`show = ["montekove"]`).
- **Organizer sheet.** Each faction gets one (`organizers/<id>.html`, PDF too): the nodes it owns.
- **Viewer.** Factions are in the owner list (filtering by a faction shows its own nodes and its
  members' nodes; filtering by a person also shows their factions' nodes), a **Frakce** tab lists
  members, and the People tab has a Frakce column. `[[faction-id]]` in a text shows its name.
- **Checks.** Unknown factions in `factions` are errors; a faction without members is a warning.

## Shared texts and facts (`blocks.toml`)

Write a text once and use it in many places:

```toml
[[block]]
id = "zvesti-z-pristavu"
title = "Zvěsti z přístavu"     # heading when shown on a sheet
text = """
V přístavu se šeptá, že [[don]] zaplatil celou stráž.
"""
show = ["barmanka", "taxikar"]  # these people's sheets get it as a section

[[block]]
id = "datum-vrazdy"             # a fact: short, no title, only used inside other texts
text = "13. března"
```

- **Links.** In any text (`text`, `brief`, goals, secrets, `[person.sheet]`, relations, phases,
  item, info, segment, cycle and block texts) `[[id]]` is replaced by:
  - a block's text (blocks may include blocks);
  - the name of a person, room, faction, item or piece of info.

  Change the block or the name once and every sheet changes. Brackets around anything that is not
  an id (`[[ poznámka ]]`) stay as they are.
- **On sheets.** A block with `show` appears on those players' sheets as its own section (titled
  by `title`, or "Co víš s ostatními").
- **Editing.** Sheets and the viewer show the expanded text; the browser editor shows and saves the
  text as written. The **Texty** tab lists every block, where it is used and whose sheets show it.
- **Checks.** An unknown `[[id]]` or a link to an action or segment is an error (with a "did you
  mean"); a block that includes itself is an error, and so is a chain of 16 or more blocks including each other or a block that grows past 200 000 bytes when expanded; a block used nowhere and shown to nobody is a
  warning. Blocks can have `status` and `writer` like everything else.

## Who knows what, and when

`check` and the viewer work out, for every person and every piece of info, when they can know it:

- **from the start**: the info is `starting` and they (or one of their factions) own it;
- **finds it out**: one of their own actions or decision options `gives` it;
- **is told**: one of their own actions or options `needs` it, so someone tells them by then.

The viewer's **Znalosti** tab is a table: one row per info, one column per person; each cell shows
the first moment (● start, 🔍 finds out, 💬 is told, with the segment or cycle step), "?" when it
happens only after some decision. Hover a cell for every way.

**Spoiler check.** When a text names an info with `[[info-id]]`, `check` warns if the reader can't
know it yet (organizers know the whole plot and are not checked):

- a person's sheet (goals, secrets, sections, relations, blocks shown to them, and their factions'
  goals, secrets and sections) is read before the game, so it may only name info they know from
  the start. The warning points at the file the text is written in (`people/…`, `blocks.toml`,
  `factions.toml`) and names the reader;
- the brief of one of their own actions may also name info they learn at that very action, or
  surely learn (not only after some decision option) at an action in an earlier segment. In a game
  without segments, any such action counts.

Phases are not checked. Names written as plain text are not checked; link the info to get the
check.

## Writing status

Any action, decision, person, block or faction can say how far it is written and who writes it:

```toml
[action]
title = "Ohledání kostí"
status = "review"   # draft | review | done
writer = "Petra"
```

Both keys are optional. `check` prints a `progress:` line once anything has a status (done out of
all, in review, drafts, untracked, and per writer). Game health in the viewer shows the same as a
bar; click a colour or a writer to show only those nodes, click an open part to jump to it. The
node editor in `serve` can set both.

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
| One-sided relationship between player characters               | warning |
| Faction without members                                        | warning |
| Unknown `[[link]]`, block loop, too deep or too big block       | error   |
| Block used nowhere and shown to nobody                         | warning |
| `[[info]]` in a text a person reads before they know the info  | warning |

Warnings don't stop the game from being generated; errors do.
