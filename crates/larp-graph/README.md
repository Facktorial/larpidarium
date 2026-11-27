# larp-graph

Turns a checked `GameModel` into the graph the web page draws.

- **Edges are derived:** when one action *gives* an item that another action *needs*, there is an edge.
- **Layout:** left-to-right layered layout, with positions and bend points ("knees") pinned in
  `layout.toml` taking priority.
- **Output:** `graph.json` for the taraxaplan-based renderer, and SVG for the PDF booklet.
- **No I/O.**

Status: edges, layered layout and `layout.toml` overrides work; SVG export is planned.
