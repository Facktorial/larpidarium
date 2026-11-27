# 7. Character lives are ranges of segments

Date: 2026-11-27

## Context

Some games run as repeated life-cycles (e.g. the 8-step story circle). Participants reincarnate:
when a character dies, the same person continues as a new one. Character sheets change between
steps of the cycle, and some characters stand outside the cycle.

## Decision

- Cycles are not a new concept: each step is an ordinary segment, and `group` on segments only
  groups them for display. Segment order in `segments.toml` is the timeline.
- A character's life is `enters ..= dies`, both segment ids, both optional.
- The real participant is a separate `[[player]]` in `players.toml`; characters point to it with
  `played_by`. A person with `role = "player"` is still a character, so existing games are
  unchanged.
- Sheet changes are `[[person.phase]]` entries that override goals, secrets or sheet sections
  from a segment on, instead of separate person files per step.

## Consequences

- Lives and phases need `segments.toml`; without it their segment ids are unknown references.
- The checks (acting outside a life, overlaps, idle players) are warnings, so unfinished designs
  still build.
- Playthrough simulation ignores lives; a dead character's actions are only flagged, not removed.
