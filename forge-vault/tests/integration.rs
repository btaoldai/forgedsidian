//! Integration tests for the vault pipeline: open → index → search → graph.
//!
//! These tests verify the end-to-end functionality of VaultStore, VaultIndex,
//! and VaultWatcher using temporary directories as isolated vault roots.

use forge_vault::store::VaultStore;
use forge_vault::watcher::VaultWatcher;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Create a .md file in a directory with the given content.
fn create_note(dir: &Path, name: &str, content: &str) {
    let path = dir.join(format!("{}.md", name));
    fs::write(&path, content).expect("failed to write note");
}

// ---------------------------------------------------------------------------
// Tests: Vault opening and basic operations
// ---------------------------------------------------------------------------

#[test]
fn test_open_vault_empty_dir() {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let store = VaultStore::open(tmp.path()).expect("failed to open empty vault");

    // Search on an empty vault should return no results.
    let results = store
        .search_notes("anything", 10)
        .expect("search failed on empty vault");
    assert!(
        results.is_empty(),
        "expected no search results in empty vault"
    );

    // Graph snapshot should have no nodes or edges.
    let snap = store.graph_snapshot();
    assert_eq!(snap.nodes.len(), 0, "expected 0 nodes in empty vault");
    assert_eq!(snap.edges.len(), 0, "expected 0 edges in empty vault");
}

#[test]
fn test_open_vault_with_notes() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create three test notes.
    create_note(
        tmp.path(),
        "apple",
        "This is about apples. Crisp and juicy.",
    );
    create_note(tmp.path(), "banana", "Bananas are yellow and soft.");
    create_note(tmp.path(), "carrot", "Carrots are orange and crunchy.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault with notes");

    // Search for "apple" — should find the apple note.
    let results = store
        .search_notes("apple", 10)
        .expect("search for 'apple' failed");
    assert!(!results.is_empty(), "expected to find 'apple' in vault");
    assert!(
        results[0].contains("apple.md"),
        "expected apple.md in top result, got: {}",
        results[0]
    );

    // Graph should have 3 nodes (one per note).
    let snap = store.graph_snapshot();
    assert_eq!(snap.nodes.len(), 3, "expected 3 nodes in graph");
}

// ---------------------------------------------------------------------------
// Tests: Search relevance
// ---------------------------------------------------------------------------

#[test]
fn test_search_relevance() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create notes with different relevance to the query.
    // "Rust" in the title should rank higher than in the body.
    create_note(
        tmp.path(),
        "Rust",
        "This is a note about Rust programming language.",
    );
    create_note(
        tmp.path(),
        "Learning",
        "Rust is a great language for systems programming.",
    );

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Search for "Rust" — the title match (Rust.md) should rank first.
    let results = store
        .search_notes("Rust", 10)
        .expect("search for 'Rust' failed");
    assert!(!results.is_empty(), "expected search results for 'Rust'");
    assert!(
        results[0].contains("Rust.md"),
        "expected Rust.md (title match) to rank first, got: {}",
        results[0]
    );
}

// ---------------------------------------------------------------------------
// Tests: Wikilinks and graph construction
// ---------------------------------------------------------------------------

#[test]
fn test_graph_wikilinks() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create note A with a wikilink to B.
    create_note(tmp.path(), "A", "This is note A with a link: [[B]]");
    // Create note B (target of the wikilink).
    create_note(tmp.path(), "B", "This is note B.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Graph snapshot should have 2 nodes and 1 edge (A -> B).
    let snap = store.graph_snapshot();
    assert_eq!(snap.nodes.len(), 2, "expected 2 nodes in graph");
    assert_eq!(snap.edges.len(), 1, "expected 1 edge in graph (A -> B)");
}

#[test]
fn test_graph_no_dangling_links() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a note with a wikilink to a non-existent note.
    create_note(
        tmp.path(),
        "A",
        "This is note A with a dangling link: [[NonExistent]]",
    );

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Graph should have 1 node (A) and 0 edges (dangling link is ignored).
    let snap = store.graph_snapshot();
    assert_eq!(snap.nodes.len(), 1, "expected 1 node in graph");
    assert_eq!(
        snap.edges.len(),
        0,
        "expected 0 edges (dangling link should not create an edge)"
    );
}

// ---------------------------------------------------------------------------
// Tests: Incremental indexing
// ---------------------------------------------------------------------------

#[test]
fn test_reindex_file() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create an initial note.
    create_note(tmp.path(), "document", "Initial content with keyword-A.");

    let mut store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Search for "keyword-A" — should find the note.
    let results = store
        .search_notes("keyword-A", 10)
        .expect("initial search failed");
    assert!(
        !results.is_empty(),
        "expected to find 'keyword-A' initially"
    );

    // Modify the file on disk: add "keyword-B", remove "keyword-A".
    let note_path = tmp.path().join("document.md");
    fs::write(&note_path, "Updated content with keyword-B.").expect("failed to update note");

    // Re-index the file.
    store.reindex_file(&note_path).expect("reindex_file failed");

    // Search for "keyword-A" — should now find nothing.
    let results_a = store
        .search_notes("keyword-A", 10)
        .expect("search after reindex failed");
    assert!(
        results_a.is_empty(),
        "expected 'keyword-A' to no longer be found after reindex"
    );

    // Search for "keyword-B" — should find the note.
    let results_b = store
        .search_notes("keyword-B", 10)
        .expect("search for 'keyword-B' failed");
    assert!(
        !results_b.is_empty(),
        "expected to find 'keyword-B' after reindex"
    );
}

// ---------------------------------------------------------------------------
// Tests: File removal
// ---------------------------------------------------------------------------

#[test]
fn test_remove_file() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create two notes.
    create_note(tmp.path(), "note1", "This is note 1 with unique-content-1.");
    create_note(tmp.path(), "note2", "This is note 2 with unique-content-2.");

    let mut store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Verify both notes are indexed.
    let results_1 = store
        .search_notes("unique-content-1", 10)
        .expect("search for note1 failed");
    assert!(!results_1.is_empty(), "expected to find note1 initially");

    // Remove note1 from disk.
    let note1_path = tmp.path().join("note1.md");
    fs::remove_file(&note1_path).expect("failed to delete note1.md");

    // Remove note1 from the index.
    store.remove_file(&note1_path).expect("remove_file failed");

    // Search for "unique-content-1" — should find nothing.
    let results = store
        .search_notes("unique-content-1", 10)
        .expect("search after removal failed");
    assert!(
        results.is_empty(),
        "expected 'unique-content-1' to no longer be found after removal"
    );

    // Search for "unique-content-2" — should still find note2.
    let results_2 = store
        .search_notes("unique-content-2", 10)
        .expect("search for note2 failed");
    assert!(
        !results_2.is_empty(),
        "expected to find note2 after removing note1"
    );
}

// ---------------------------------------------------------------------------
// Tests: Hidden directories
// ---------------------------------------------------------------------------

#[test]
fn test_hidden_dirs_skipped() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a regular note.
    create_note(tmp.path(), "visible", "This is a visible note.");

    // Create a hidden .git directory with a .md file inside.
    let git_dir = tmp.path().join(".git");
    fs::create_dir(&git_dir).expect("failed to create .git dir");
    create_note(&git_dir, "hidden", "This is in .git and should be ignored.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Search for "hidden" — should find nothing (file in .git should be skipped).
    let results = store
        .search_notes("hidden", 10)
        .expect("search for 'hidden' failed");
    assert!(
        results.is_empty(),
        "expected files in .git to be skipped during vault scan"
    );

    // Search for "visible" — should find the regular note.
    let results = store
        .search_notes("visible", 10)
        .expect("search for 'visible' failed");
    assert!(
        !results.is_empty(),
        "expected to find the visible note outside .git"
    );

    // Graph should have only 1 node (the visible note).
    let snap = store.graph_snapshot();
    assert_eq!(
        snap.nodes.len(),
        1,
        "expected 1 node (hidden files in .git should be ignored)"
    );
}

// ---------------------------------------------------------------------------
// Tests: Reading notes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_read_note() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    let content = "# Test Note\n\nThis is the body content of the note.";
    create_note(tmp.path(), "test", content);

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    let note_path = tmp.path().join("test.md");
    let note = store.read_note(&note_path).await.expect("read_note failed");

    assert_eq!(note.body, content, "note body should match written content");
    assert_eq!(
        note.path, note_path,
        "note path should match the requested path"
    );
}

// ---------------------------------------------------------------------------
// Tests: Wikilink resolution
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_wikilink_exact_stem_match() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create two notes with different casings.
    create_note(tmp.path(), "My Note", "Content for my note.");
    create_note(tmp.path(), "Another", "Different content.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Resolve with exact stem match (case-insensitive).
    let resolved = store.resolve_wikilink("My Note");
    assert!(
        resolved.is_some(),
        "expected to resolve 'My Note' to a file"
    );
    assert!(
        resolved.as_ref().unwrap().contains("My Note.md"),
        "expected resolved path to contain 'My Note.md'"
    );

    // Also test lowercase variant.
    let resolved_lower = store.resolve_wikilink("my note");
    assert!(
        resolved_lower.is_some(),
        "expected case-insensitive match for 'my note'"
    );
    assert_eq!(
        resolved, resolved_lower,
        "case-insensitive resolution should match"
    );
}

#[test]
fn test_resolve_wikilink_with_heading_fragment() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a note.
    create_note(tmp.path(), "Architecture", "# Design\n# Implementation");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Resolve with heading fragment — fragment should be stripped for path resolution.
    let resolved = store.resolve_wikilink("Architecture#Design");
    assert!(
        resolved.is_some(),
        "expected to resolve 'Architecture#Design' by stripping the heading"
    );
    assert!(
        resolved.as_ref().unwrap().contains("Architecture.md"),
        "resolved path should point to Architecture.md, not the fragment"
    );
}

#[test]
fn test_resolve_wikilink_non_existent() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    create_note(tmp.path(), "Note1", "This exists.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Try to resolve a non-existent target.
    let resolved = store.resolve_wikilink("NonExistent");
    assert!(
        resolved.is_none(),
        "expected None when resolving a non-existent note"
    );
}

#[test]
fn test_resolve_wikilink_empty_target() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    create_note(tmp.path(), "Note", "Content.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Try to resolve an empty target.
    let resolved = store.resolve_wikilink("");
    assert!(
        resolved.is_none(),
        "expected None when resolving an empty target"
    );
}

#[test]
fn test_resolve_wikilink_heading_only() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    create_note(tmp.path(), "Note", "Content.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Try to resolve only a heading (no stem before the #).
    let resolved = store.resolve_wikilink("#SomeHeading");
    assert!(
        resolved.is_none(),
        "expected None when resolving only a heading"
    );
}

#[test]
fn test_resolve_wikilink_relative_path() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a subdirectory with a note.
    fs::create_dir(tmp.path().join("subdir")).expect("failed to create subdir");
    create_note(&tmp.path().join("subdir"), "nested_note", "This is nested.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Resolve with relative path.
    let resolved = store.resolve_wikilink("subdir/nested_note");
    assert!(
        resolved.is_some(),
        "expected to resolve relative path 'subdir/nested_note'"
    );
    assert!(
        resolved.as_ref().unwrap().contains("subdir"),
        "resolved path should contain 'subdir'"
    );
    assert!(
        resolved.as_ref().unwrap().contains("nested_note.md"),
        "resolved path should contain 'nested_note.md'"
    );

    // Also test with backslashes (Windows-style).
    let resolved_backslash = store.resolve_wikilink("subdir\\nested_note");
    assert_eq!(
        resolved, resolved_backslash,
        "backslash should be normalized to forward slash"
    );
}

#[test]
fn test_resolve_wikilink_relative_path_with_extension() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    fs::create_dir(tmp.path().join("docs")).expect("failed to create docs dir");
    create_note(&tmp.path().join("docs"), "reference", "Reference material.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Resolve with .md extension included (should still match).
    let resolved = store.resolve_wikilink("docs/reference.md");
    assert!(
        resolved.is_some(),
        "expected to resolve 'docs/reference.md' even with extension"
    );
}

// ---------------------------------------------------------------------------
// Tests: Tag index
// ---------------------------------------------------------------------------

#[test]
fn test_list_tags_empty_vault() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Open a vault with no notes — list_tags() should return empty.
    let store = VaultStore::open(tmp.path()).expect("failed to open empty vault");
    let tags = store.list_tags();
    assert!(tags.is_empty(), "expected no tags in an empty vault");
}

#[test]
fn test_list_tags_from_notes() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create notes with frontmatter tags.
    create_note(
        tmp.path(),
        "note_rust",
        "---\ntags:\n  - rust\n  - testing\n---\nNote about Rust.",
    );
    create_note(
        tmp.path(),
        "note_devops",
        "---\ntags:\n  - devops\n  - docker\n---\nNote about DevOps.",
    );
    // Note without tags.
    create_note(tmp.path(), "note_plain", "No frontmatter here.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");
    let tags = store.list_tags();

    // Should contain exactly the 4 unique tags, sorted alphabetically.
    assert_eq!(tags.len(), 4, "expected 4 distinct tags, got: {:?}", tags);
    assert_eq!(tags, vec!["devops", "docker", "rust", "testing"]);
}

#[test]
fn test_notes_by_tag() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    let rust_note_name = "rust_note";
    let other_note_name = "other_note";

    create_note(
        tmp.path(),
        rust_note_name,
        "---\ntags:\n  - rust\n  - systems\n---\nRust systems note.",
    );
    create_note(
        tmp.path(),
        other_note_name,
        "---\ntags:\n  - python\n---\nPython note.",
    );

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    let rust_notes = store.notes_by_tag("rust");
    assert_eq!(rust_notes.len(), 1, "expected exactly 1 note tagged 'rust'");
    assert!(
        rust_notes[0].to_string_lossy().contains(rust_note_name),
        "expected rust_note to be in notes_by_tag('rust'), got: {:?}",
        rust_notes
    );

    let python_notes = store.notes_by_tag("python");
    assert_eq!(
        python_notes.len(),
        1,
        "expected exactly 1 note tagged 'python'"
    );
    assert!(
        python_notes[0].to_string_lossy().contains(other_note_name),
        "expected other_note to be in notes_by_tag('python')"
    );

    let missing_notes = store.notes_by_tag("nonexistent-tag");
    assert!(
        missing_notes.is_empty(),
        "expected empty vec for a non-existent tag"
    );
}

#[test]
fn test_notes_by_tag_case_insensitive() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Tag written in mixed case in frontmatter — parse_frontmatter normalises to lowercase.
    create_note(
        tmp.path(),
        "mixed_case_note",
        "---\ntags:\n  - Rust\n  - TESTING\n---\nNote with mixed-case tags.",
    );

    let store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Query with lowercase — should find the note regardless of frontmatter casing.
    let rust_notes = store.notes_by_tag("rust");
    assert_eq!(
        rust_notes.len(),
        1,
        "expected 1 note for tag 'rust' (normalised from 'Rust')"
    );

    // Query with uppercase — should also match (query is normalised before lookup).
    let rust_upper = store.notes_by_tag("RUST");
    assert_eq!(
        rust_upper.len(),
        1,
        "expected 1 note for tag 'RUST' (query normalised to 'rust')"
    );

    // Both queries must resolve to the same note.
    assert_eq!(
        rust_notes, rust_upper,
        "case-insensitive queries should return the same note"
    );

    // list_tags() should contain the normalised lowercase version only.
    let tags = store.list_tags();
    assert!(
        tags.contains(&"rust".to_string()),
        "list_tags should contain 'rust'"
    );
    assert!(
        tags.contains(&"testing".to_string()),
        "list_tags should contain 'testing'"
    );
    assert!(
        !tags.contains(&"Rust".to_string()),
        "list_tags should NOT contain 'Rust' (raw case)"
    );
}

// ---------------------------------------------------------------------------
// Tests: Watcher
// ---------------------------------------------------------------------------

#[test]
fn test_watcher_starts() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create the watcher — it should return Ok immediately.
    let result = VaultWatcher::start(tmp.path());
    assert!(
        result.is_ok(),
        "VaultWatcher::start should succeed on a valid directory"
    );

    let (_watcher, _rx) = result.expect("watcher creation failed");
    // The watcher should now be running. Drop it — this stops the watcher.
    // No assertion needed; the test passes if the watcher starts without error.
}

// ---------------------------------------------------------------------------
// Tests: Edge cases (Phase 17 hardening)
// ---------------------------------------------------------------------------

#[test]
fn test_unicode_file_paths() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create notes with Unicode filenames.
    create_note(tmp.path(), "test_café", "Coffee-related note.");
    create_note(tmp.path(), "日本語", "Japanese note content.");
    create_note(tmp.path(), "emoji_🚀", "Rocket emoji in name.");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault with Unicode names");

    // List all note paths — should include all 3 Unicode filenames.
    let paths = store.list_note_paths().expect("list_note_paths failed");
    assert_eq!(paths.len(), 3, "expected 3 notes with Unicode filenames");

    // Verify each Unicode filename is present and decodable as UTF-8.
    let paths_set: std::collections::HashSet<_> = paths.iter().collect();
    assert!(
        paths_set.iter().any(|p| p.contains("café")),
        "expected 'café' filename in paths"
    );
    assert!(
        paths_set.iter().any(|p| p.contains("日本語")),
        "expected '日本語' filename in paths"
    );
    assert!(
        paths_set.iter().any(|p| p.contains("🚀")),
        "expected '🚀' filename in paths"
    );

    // Perform a search to ensure the full pipeline handles Unicode.
    let results = store
        .search_notes("Coffee", 10)
        .expect("search failed on Unicode vault");
    assert!(!results.is_empty(), "expected search to find café note");
}

#[cfg(unix)]
#[test]
fn test_symlinks_rejected_unix() {
    use std::os::unix::fs as unix_fs;

    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a regular note inside the vault.
    create_note(tmp.path(), "regular_note", "This is a regular note.");

    // Create a temporary external directory (outside the vault).
    let external_tmp = TempDir::new().expect("failed to create external temp dir");
    create_note(
        external_tmp.path(),
        "external_target",
        "This is outside the vault.",
    );

    // Create a symlink inside the vault pointing to the external note.
    let symlink_path = tmp.path().join("symlink_to_external.md");
    unix_fs::symlink(
        external_tmp.path().join("external_target.md"),
        &symlink_path,
    )
    .expect("failed to create symlink");

    let store = VaultStore::open(tmp.path()).expect("failed to open vault with symlink");

    // List note paths — should NOT include the symlink.
    let paths = store.list_note_paths().expect("list_note_paths failed");
    assert_eq!(
        paths.len(),
        1,
        "expected only the regular note, symlinks should be excluded or not indexed"
    );
    assert!(
        paths[0].contains("regular_note"),
        "expected regular_note to be present"
    );

    // Verify the symlink target is not searchable.
    let results = store
        .search_notes("external_target", 10)
        .expect("search failed");
    assert!(
        results.is_empty(),
        "expected symlink targets to not be indexed"
    );
}

#[test]
fn test_large_note_file() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a large note file (~6 MB of repeated text).
    let large_content = "a".repeat(1024 * 6144); // 6 MB
    let path = tmp.path().join("big_note.md");
    fs::write(&path, &large_content).expect("failed to write large note");

    // Also create a smaller indexable note for reference.
    create_note(
        tmp.path(),
        "small_note",
        "This is a small note with keyword-marker.",
    );

    let start = std::time::Instant::now();
    let store = VaultStore::open(tmp.path()).expect("failed to open vault with large file");
    let elapsed = start.elapsed();

    // Verify the vault opened successfully.
    let paths = store.list_note_paths().expect("list_note_paths failed");
    assert_eq!(
        paths.len(),
        2,
        "expected both large and small notes to be indexed"
    );

    // Verify search works (should find the small note).
    let results = store
        .search_notes("keyword-marker", 10)
        .expect("search failed");
    assert!(!results.is_empty(), "expected search to find small_note");

    // Verify the large file didn't cause a panic or excessive delay (< 30 seconds).
    assert!(
        elapsed.as_secs() < 30,
        "vault opening took too long with large file: {:?}",
        elapsed
    );
}

#[test]
fn test_path_traversal_defense() {
    let tmp = TempDir::new().expect("failed to create temp dir");

    // Create a regular note.
    create_note(tmp.path(), "legitimate_note", "Legitimate content.");

    let mut store = VaultStore::open(tmp.path()).expect("failed to open vault");

    // Attempt to reindex a path with traversal sequences.
    // Use absolute paths to simulate an attacker-controlled relative path.
    // Note: forge-vault works with absolute paths, so we construct a path
    // that nominally looks like it tries to traverse up and out.
    let traversal_path = tmp.path().join("../../../etc/passwd");

    // The operation should either fail safely with VaultError, or silently
    // reject the path (by not finding it). We test that it does NOT read
    // a system file.
    let result = store.reindex_file(&traversal_path);
    match result {
        Ok(()) => {
            // If it succeeds, verify the vault was not compromised.
            let paths = store.list_note_paths().expect("list_note_paths failed");
            // Only the legitimate note should be indexed.
            assert_eq!(
                paths.len(),
                1,
                "vault should not be modified by path traversal attempt"
            );
        }
        Err(_) => {
            // Error is the expected behavior — path traversal should be rejected.
            // No assertion needed; the error is the success condition.
        }
    }
}
