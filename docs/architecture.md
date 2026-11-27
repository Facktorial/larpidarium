# Architecture

## Data flow

```mermaid
flowchart LR
    subgraph input[game folder]
      G[game.toml]
      A[actions/*.toml]
      P[people/*.toml]
      I[items.toml · rooms.toml · info.toml]
      LY[layout.toml]
    end
    input --> parse --> validate --> M[(GameModel)]
    validate -- fails --> ERR["miette report<br/>file:line + hint"]
    M --> GR[larp-graph] --> J[graph.json] --> WEB["web/ renderer<br/>(from taraxaplan)"]
    GR --> SVG[graph.svg]
    M --> R[larp-render]
    R --> HTML["player + organizer<br/>sheets · HTML"]
    R --> TEX[.tex] --> PDF[larp-pdf · tectonic] --> OUT[PDFs]
    SVG --> TEX
```

## Crates

```mermaid
flowchart TD
    BIN["larpidarium (bin)<br/>clap · I/O · serve"]
    GRAPH["larp-graph<br/>edges · layout"]
    RENDER["larp-render<br/>minijinja html + tex"]
    PDFC["larp-pdf<br/>runs tectonic"]
    MODEL["larp-model<br/>types · parse · validate"]
    BIN --> GRAPH & RENDER & PDFC
    RENDER --> GRAPH
    GRAPH --> MODEL
    RENDER --> MODEL
    subgraph pure["pure core: no I/O"]
      GRAPH
      RENDER
      MODEL
    end
```

Arrows mean "depends on". The pure core takes values and returns values, which keeps it fast to
test. Only the binary reads and writes files and only `larp-pdf` spawns a process.

## Output layout (`dist/`, phase 2+)

```text
dist/
├── index.html          # graph page
├── graph.json          # nodes, edges, colors, positions
├── assets/             # renderer JS/CSS (adapted from taraxaplan)
├── players/<id>.html   # one sheet per character
├── organizers/<id>.html
└── pdf/                # after `larpidarium pdf`
```

## Graph semantics

An edge `A → B` exists when action (or decision option) A *gives* an item that action (or option)
B *needs*. Edges are never written by hand. Validation reports:

- **error:** reference to an unknown id (item, person, room, action);
- **error:** item needed but never given and not marked `starting = true`;
- **warning:** item given but never needed;
- **warning:** action unreachable from any starting item.

## Layout

`larp-graph` computes a left-to-right layered layout from the dependency order. Positions and
bend points saved from the browser ("Save layout") go to `layout.toml` and override the
computed ones for those nodes.
