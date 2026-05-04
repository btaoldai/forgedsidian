//! Criterion benchmarks for forge-vault core operations.
//!
//! Run with: `cargo bench -p forge-vault`
//!
//! Benchmarks cover:
//! - VaultStore::open() on vaults of varying sizes
//! - VaultIndex::search() query latency
//! - Graph snapshot serialisation
//! - Scan + diff (incremental detection)
//! - Link extraction throughput
//!
//! NOTE: as of criterion 0.8 (Dependabot PR #19, 2026-05-03), `criterion::black_box`
//! is deprecated in favor of `std::hint::black_box` (stable since Rust 1.66). Switching
//! the import here keeps `cargo clippy --workspace --all-targets -- -D warnings` green
//! without changing call sites. (Picked up incidentally during R4 splash format fix.)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use forge_vault::VaultStore;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Generate a temporary vault with `n` markdown files, some with wikilinks.
fn create_test_vault(n: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let root = dir.path().to_path_buf();

    for i in 0..n {
        let name = format!("note-{i:04}.md");
        let path = root.join(&name);

        // Every 3rd note links to the next two notes (creates a connected graph).
        let mut body = format!("# Note {i}\n\nThis is note number {i}.\n\n");
        if i % 3 == 0 && i + 2 < n {
            body.push_str(&format!(
                "See also [[note-{:04}]] and [[note-{:04}]].\n",
                i + 1,
                i + 2
            ));
        }
        // Add some searchable content for search benchmarks.
        body.push_str("Keywords: rust tauri leptos wasm forgexalith vault search index.\n");
        if i % 5 == 0 {
            body.push_str("Special topic: cybersecurity zero-trust architecture.\n");
        }

        fs::write(&path, &body).expect("write note");
    }

    (dir, root)
}

/// Benchmark: VaultStore::open() on vaults of different sizes.
fn bench_vault_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault_open");
    group.sample_size(10); // Disk I/O is slow — fewer samples.

    for &size in &[50, 200, 500] {
        let (_dir, root) = create_test_vault(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}_notes")),
            &root,
            |b, root| {
                b.iter(|| {
                    // Delete index between runs to force full rebuild.
                    let idx_path = root.join(".forge-index");
                    if idx_path.exists() {
                        let _ = fs::remove_dir_all(&idx_path);
                    }
                    let manifest_path = root.join(".forge-manifest.json");
                    if manifest_path.exists() {
                        let _ = fs::remove_file(&manifest_path);
                    }
                    let store = VaultStore::open(black_box(root)).expect("open vault");
                    black_box(store);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: VaultStore::open() when vault is unchanged (incremental — no I/O).
fn bench_vault_open_incremental(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault_open_incremental");
    group.sample_size(20);

    for &size in &[50, 200, 500] {
        let (_dir, root) = create_test_vault(size);

        // First open: creates manifest + index.
        let _store = VaultStore::open(&root).expect("initial open");
        drop(_store);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}_notes")),
            &root,
            |b, root| {
                b.iter(|| {
                    // Re-open with manifest: should be fast (diff only).
                    let store = VaultStore::open(black_box(root)).expect("open vault");
                    black_box(store);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: full-text search query latency.
fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    let (_dir, root) = create_test_vault(500);
    let store = VaultStore::open(&root).expect("open vault");

    for query in &["rust", "cybersecurity", "note-0042", "wasm forgexalith"] {
        group.bench_with_input(BenchmarkId::from_parameter(query), query, |b, q| {
            b.iter(|| {
                let results = store.search_notes(black_box(q), 20).expect("search");
                black_box(results);
            });
        });
    }
    group.finish();
}

/// Benchmark: graph snapshot serialisation.
fn bench_graph_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_snapshot");

    for &size in &[50, 200, 500] {
        let (_dir, root) = create_test_vault(size);
        let store = VaultStore::open(&root).expect("open vault");

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}_notes")),
            &store,
            |b, store| {
                b.iter(|| {
                    let snap = store.graph_snapshot();
                    black_box(snap);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: list_note_paths (directory scan, relative path computation).
fn bench_list_note_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_note_paths");
    group.sample_size(20);

    for &size in &[50, 200, 500] {
        let (_dir, root) = create_test_vault(size);
        let store = VaultStore::open(&root).expect("open vault");

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}_notes")),
            &store,
            |b, store| {
                b.iter(|| {
                    let paths = store.list_note_paths().expect("list");
                    black_box(paths);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_vault_open,
    bench_vault_open_incremental,
    bench_search,
    bench_graph_snapshot,
    bench_list_note_paths,
);
criterion_main!(benches);
