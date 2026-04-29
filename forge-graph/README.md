# forge-graph

Backlink graph, node layout algorithms and graph queries.

## Purpose

Builds and maintains a directed graph of note relationships (wikilinks), computes layout coordinates for visualization, and provides efficient queries for backlinks and forward links. Uses petgraph as the underlying graph structure with NoteIds as nodes and wikilinks as directed edges.

## Key modules

- `graph` — `NoteGraph` struct and mutation API (add/remove nodes and edges)
- `layout` — force-directed layout algorithms for graph visualization (Phase 2)
- `query` — backlink and forward-link traversal queries
- `error` — graph-specific error types

## Dependencies

- Internal: forge-core
- External: petgraph, tokio

## Usage

```rust
use forge_graph::NoteGraph;

let mut graph = NoteGraph::new();
graph.add_note(note_id);
let backlinks = graph.backlinks(note_id)?;
let neighbors = graph.forward_links(note_id)?;
```

## Related docs

- Force-directed layout planned for Phase 2
- Integration with forge-vault for graph updates on file changes
- Dependency inversion issue: forge-graph currently depends on forge-editor and forge-vault (tracked for next sprint)
