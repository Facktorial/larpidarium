# 0004 Graph edges are derived from items

- **Status:** accepted, 2026-11-27

## Context

taraxaplan connects actions through items: an action's outputs feed another action's
prerequisites. The alternative was explicit "exit" links between scenes.

## Decision

Edges are never written. `A → B` exists when A (or one of its decision options) gives an item that
B (or one of its options) needs.

## Consequences

- One source of truth: changing an item list changes the graph.
- The validator can find plot holes (needed but never given) and dead ends (given, never needed).
- Pure sequencing without an item needs a placeholder item or info.
