//! Criterion benchmarks for forge-graph operations.
//!
//! Run with: `cargo bench -p forge-graph`
//!
//! Benchmarks cover:
//! - NoteGraph building (add_note, add_link)
//! - Graph snapshot serialisation
//! - Snapshot JSON serialisation (serde_json)
//! - remove_note_edges performance
//!
//! NOTE: as of criterion 0.8 (Dependabot PR #19, 2026-05-03), `criterion::black_box`
//! is deprecated in favor of `std::hint::black_box` (stable since Rust 1.66). Switching
//! the import here keeps `cargo clippy --workspace --all-targets -- -D warnings` green
//! without changing call sites. (Picked up incidentally during R4 splash format fix.)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use forge_core::NoteId;
use forge_graph::NoteGraph;
use std::hint::black_box;

/// Build a graph with `n` nodes and approximately `n * link_ratio` edges.
fn build_test_graph(n: usize, link_ratio: f64) -> (NoteGraph, Vec<NoteId>) {
    let mut graph = NoteGraph::new();
    let ids: Vec<NoteId> = (0..n).map(|_| NoteId::new()).collect();

    for id in &ids {
        graph.add_note(*id);
    }

    // Deterministic pseudo-random linking using LCG.
    let num_links = (n as f64 * link_ratio) as usize;
    let mut seed: u64 = 12345;
    for _ in 0..num_links {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let from_idx = (seed >> 33) as usize % n;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let to_idx = (seed >> 33) as usize % n;
        if from_idx != to_idx {
            graph.add_link(ids[from_idx], ids[to_idx]);
        }
    }

    (graph, ids)
}

/// Benchmark: building a graph from scratch (add_note + add_link).
fn bench_graph_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_build");

    for &(n, ratio) in &[(50, 0.5), (200, 0.5), (500, 0.5), (500, 1.5)] {
        let ids: Vec<NoteId> = (0..n).map(|_| NoteId::new()).collect();
        let num_links = (n as f64 * ratio) as usize;

        // Pre-compute link pairs.
        let mut seed: u64 = 12345;
        let mut links = Vec::with_capacity(num_links);
        for _ in 0..num_links {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let from_idx = (seed >> 33) as usize % n;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let to_idx = (seed >> 33) as usize % n;
            if from_idx != to_idx {
                links.push((ids[from_idx], ids[to_idx]));
            }
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}n_{ratio}ratio")),
            &(ids.clone(), links.clone()),
            |b, (ids, links)| {
                b.iter(|| {
                    let mut graph = NoteGraph::new();
                    for id in ids {
                        graph.add_note(*id);
                    }
                    for (from, to) in links {
                        graph.add_link(*from, *to);
                    }
                    black_box(&graph);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: snapshot() serialisation (graph -> GraphSnapshot struct).
fn bench_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_snapshot");

    for &n in &[50, 200, 500] {
        let (graph, _ids) = build_test_graph(n, 0.5);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}_nodes")),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let snap = graph.snapshot();
                    black_box(snap);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: snapshot() -> serde_json::to_string (full IPC serialisation path).
fn bench_snapshot_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_snapshot_json");

    for &n in &[50, 200, 500] {
        let (graph, _ids) = build_test_graph(n, 0.5);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}_nodes")),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let snap = graph.snapshot();
                    let json = serde_json::to_string(&snap).expect("serialize");
                    black_box(json);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: remove_note_edges (incremental re-indexing hot path).
fn bench_remove_edges(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_note_edges");

    for &n in &[50, 200, 500] {
        // Build graph once, then benchmark removing edges for each node.
        let (base_graph, ids) = build_test_graph(n, 0.5);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}_nodes")),
            &(base_graph, ids.clone()),
            |b, (_base, ids)| {
                b.iter_batched(
                    || {
                        // Clone the base graph for each iteration.
                        // We can't clone NoteGraph directly, so rebuild.
                        let (g, _) = build_test_graph(ids.len(), 0.5);
                        g
                    },
                    |mut graph| {
                        // Remove edges from 10% of nodes.
                        let count = ids.len() / 10;
                        for id in &ids[..count] {
                            graph.remove_note_edges(*id);
                        }
                        black_box(&graph);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_graph_build,
    bench_snapshot,
    bench_snapshot_json,
    bench_remove_edges,
);
criterion_main!(benches);
