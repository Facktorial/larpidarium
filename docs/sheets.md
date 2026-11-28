# Character sheets (draft)

> HTML player sheets are implemented (`larpidarium build`). PDF and organizer sheets are not yet.
> Decisions made on 2026-11-27 are marked ✅.

Every person with `role = "player"` gets a sheet. Organizers and NPC players get an organizer
version (see the end). A sheet has two parts:

```mermaid
flowchart LR
    P["people/&lt;id&gt;.toml"] --> I["Part I · Who you are<br/>whereabouts and motivations"]
    G[(action graph)] --> X["extract the character's segment"] --> II["Part II · Destiny<br/>timeline + mini graph"]
    I --> S[sheet]
    II --> S
    S --> H[HTML]
    S --> L["LaTeX → PDF"]
```

## Part I: Who you are (whereabouts and motivations)

Written by hand in the person file. Nothing here is computed.

| Section                | Source key                      |
| ---------------------- | ------------------------------- |
| Name, role             | `name`, `role`                  |
| Appearance             | `sheet.appearance`              |
| Backstory              | `sheet.backstory`               |
| Where you are at start | `sheet.whereabouts`             |
| Goals                  | `goals`                         |
| Secrets                | `secrets`                       |
| Relations              | `[[person.relation]]` ✅        |
| What you carry         | items/info with `owners` containing this person and `starting = true` (computed) |

```toml
# people/brother-anselm.toml
[person.sheet]
appearance  = "Vysoký, shrbený, vždy s klíčem na krku."
whereabouts = "Na začátku hry jsi v kryptě a modlíš se."

[[person.relation]]
with = "novice-marta"            # checked: must be a person
text = "Marta ti připomíná sestru. Nevěříš jí."
```

## Part II: Destiny

Computed from the graph: the **segment** of the plot this character takes part in.

1. **Own nodes:** every action and decision where the character is an owner.
2. **Inputs:** for each thing an own node needs: "you carry it" (starting item of this character),
   "you got it earlier" (another own node), or "you will learn / receive it". ✅ Other characters
   are **not named**.
3. **Outputs:** what each own node gives. ✅ The sheet does not say who needs it next.
4. **Choices:** owned decisions with their options and what each option leads to.
5. **Order:** nodes sorted by graph layer, so Part II reads as a timeline (early → late).
6. ✅ **Both** a timeline and a **mini graph**: the same segment drawn small, own nodes
   highlighted, neighbors as anonymous grey boxes. SVG in HTML, TikZ in the PDF, from the same
   coordinates.

```mermaid
flowchart LR
    subgraph seg["Anselm's segment"]
      C{{"Přiznání<br/>(your decision)"}}
    end
    K["Rezavý klíč<br/>(you carry it)"] -.-> D["Otevřít kryptu?<br/>(someone else)"]
    E["Prozkoumat kosti<br/>(someone else)"] -- "Opat byl zavražděn" --> C
    C -- "Přiznat se" --> E1([Konec: přiznání])
    C -- "Uprchnout" --> E2([Konec: útěk])
```

✅ Player-facing text is the action's `brief` key; `text` stays for organizers only.
Items listed in `uses` are shown as "you will give up / use up …".

## Organizer sheet (done)

Same data, other angle, for every organizer, non-player character and room
(`larp_graph::schedule`). Full of spoilers on purpose:

- **Load:** number of actions and decisions, a table per world segment, the busiest segment.
- **Have ready:** items (not info) the owner starts with or hands out.
- **Running order:** the owner's nodes grouped by world segment, in time order. Each shows when
  (cycle and step for cycle nodes), who else takes part, the organizer `text`, what players read
  (`brief`), what it needs, with **who provides it**, uses up and gives; decisions with every option.

## Output

- HTML: `dist/players/<id>.html` (📄) and `dist/organizers/<id>.html` (📋), linked from the graph
  sidebar and the People tab; `serve` renders them live.
- PDF (`larpidarium pdf`): `dist/pdf/<id>.pdf`, `dist/pdf/organizers/<id>.pdf`, and
  `_all-players.pdf` / `_all-organizers.pdf` for printing, via minijinja `.tex.j2` templates and
  tectonic; Czech/English labels.
- Templates can be overridden per game in `templates/`: `player.html.j2`, `organizer.html.j2`,
  `profile.html.j2`, `packing.html.j2`, `handouts.html.j2`, `player.tex.j2`, `organizer.tex.j2`,
  `packing.tex.j2`, `handouts.tex.j2`.
