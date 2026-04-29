# Testing workflow

This page documents the manual end-to-end test workflow used to validate Forgedsidian before tagging a release. It complements the automated test suite (239 tests at baseline -- see `cargo test --workspace`).

## Goal

Validate the integrated behaviour of the application after a non-trivial change: vault open, indexing, search, graph, canvas, file operations, and IPC commands.

## Prerequisites

- Rust 1.88+ with the `wasm32-unknown-unknown` target.
- Trunk (`cargo install trunk --locked`).
- Tauri CLI v2 (`cargo install tauri-cli --version "^2.0" --locked`).
- A test vault (any folder with a few `.md` files).

## Block 0 -- automated checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo check --workspace
cargo test --workspace
```

Expected: all four commands pass. The clippy step may emit warnings (uninlined format args, etc.) but must not fail.

## Block 1 -- vault lifecycle (manual smoke)

1. Launch in dev mode:
   ```bash
   cargo tauri dev
   ```
2. Click "Open Vault" and select the test folder.
3. Wait for the initial Tantivy index to complete (progress events in the console; expect ~20-30 s per 1000 notes on a modern laptop, ~3 minutes for ~9 000 notes).
4. Verify in the console:
   - `vault opened root=... notes=N links=L tags=T`
   - `vault watcher started`
5. Verify in the UI:
   - File tree is populated.
   - Tags panel shows the extracted tags.
   - Search bar returns ranked results.

## Block 2 -- editor

1. Click a note in the file tree.
2. Toggle Preview / Edit mode.
3. Type some text; expect auto-save to fire after 500 ms (no spinner; check the file's `mtime` on disk).
4. Insert a wikilink `[[other-note]]` and click it in Preview mode -- it should navigate.
5. Multi-tab: `Ctrl+W`, `Ctrl+Tab`, `Ctrl+Shift+Tab`.

## Block 3 -- graph view

1. Open the GraphView from the toolbar.
2. Verify the force-directed layout converges.
3. Drag a node; connected nodes should follow softly (pin pattern).
4. Click a node; it should open the corresponding note.
5. Zoom in / out (mouse wheel); pan (click-and-drag).

## Block 4 -- canvas

1. Open the CanvasView.
2. Drop a few cards (notes) on the canvas.
3. Pan / zoom the canvas; cards should remain in their absolute positions.
4. Try the drawing layer: pen, line, rectangle, circle. Switch colours. Test undo and clear.

## Block 5 -- IPC and file watcher

1. Outside the app, edit one of the notes in your file manager (rename or change content).
2. Verify in the app that the change is detected (file tree updates, index re-runs incrementally).
3. Delete a note from the app; verify it disappears from the file tree, search, and graph.

## Block 6 -- audits

```bash
cargo audit
cargo deny check
```

Expected:
- 0 CVE.
- ~24 unmaintained / unsound warnings (documented in `deny.toml`).
- All licences in the allow-list.

## Block 7 -- release build (optional)

```bash
cargo build --workspace --release
```

Expected: completes in 5-15 minutes depending on the machine. The `cargo tauri build` step (which produces the installer) is not required for source releases but is required for binary distributions.

## Reporting

When using this workflow as part of a PR review, paste the relevant CLI output and any screenshots into the PR description. For UI bugs, attach a short screen recording when possible.
