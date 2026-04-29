//! # forge-graph
//!
//! Backlink graph, node layout algorithms and graph queries.
//!
//! Uses [`petgraph`] as the underlying directed graph structure.
//! Nodes are [`NoteId`]s; edges represent wikilinks.
//!
//! ## Modules
//! - [`graph`]  — the `NoteGraph` struct and mutation API
//! - [`layout`] — force-directed layout (Phase 2)
//! - [`query`]  — backlink and forward-link queries
//! - [`error`]  — graph-specific errors

pub mod error;
pub mod graph;
pub mod layout;
pub mod query;

pub use error::GraphError;
pub use graph::NoteGraph;
