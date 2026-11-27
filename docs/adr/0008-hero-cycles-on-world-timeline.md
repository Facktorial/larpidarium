# 8. Hero cycles are placed on the world timeline

Date: 2026-11-27

Supersedes the "cycles are segment groups" part of ADR 0007.

## Context

In Koloběh the world (mafia, corporation, police) runs the whole game, and each player at some point
goes through a short hero's journey (8 steps of the story circle) and dies in it. Several journeys
run at once. Modelling every step of every journey as a segment (8 × 9 = 72 bands) made the graph
unreadable and lost the link between a journey and the world time it happens in.

## Decision

- World time stays `segments.toml`. A cycle (`cycles.toml`) has a hero and a `from`/`to` range of
  world segments. Steps are defined once in `game.toml`.
- One integer time axis (`larp_model::time`): each world segment is `steps.len()` ticks, a cycle's
  steps split its range evenly. Lives, deaths at a step, `after` (reincarnation), phases and actions
  are all compared on this axis.
- Games with cycles use a timeline layout: segments as columns, cycles in lanes. Rust lays out the
  world and each cycle's inside (relative to the cycle); the browser places cycles in lanes and
  switches them between collapsed, step columns and story circle, because that is interactive.
  Positions of cycle nodes in `layout.toml` are relative to their cycle, and so are bends of lines
  inside one cycle; bends are hidden on lines that cross a cycle frame or touch a closed cycle.

## Consequences

- Games without cycles keep the band layout unchanged.
- A cycle's duration must be a whole number of world segments.
- The playthrough simulation ignores time; it still checks only item flow.
