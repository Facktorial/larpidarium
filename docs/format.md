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
├── layout.toml     optional, written by "Save layout" in the browser
└── templates/      optional: player.html.j2 overrides the character sheet

All files can also be edited from the browser with `larpidarium serve` (see below).
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
