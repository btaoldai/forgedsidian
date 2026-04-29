# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — Phase 18: Wikilink Navigation

### Added

- `ipc::resolve_wikilink()` — IPC wrapper for wikilink target resolution
- `render_wikilinks()` — transforms `[[target]]` and `[[target|alias]]` into clickable `<a class="forge-wikilink">` tags with HTML entity escaping
- `extract_wikilink_at_cursor()` — detects wikilink under cursor position for Ctrl+Click navigation in Edit mode
- `navigate_wikilink()` — resolves target via IPC and opens note in TabManager
- `find_wikilink_ancestor()` — DOM traversal helper for click delegation in Preview mode
- CSS styling for `.forge-wikilink` (cyan dashed underline, hover glow)
- 14 unit tests for render_wikilinks and extract_wikilink_at_cursor

### Changed

- `md_to_html()` now pre-processes wikilinks before Markdown parsing and configures ammonia to preserve wikilink anchor tags
- Preview mode click handler now delegates wikilink clicks to navigation instead of switching to Edit mode
- Edit mode textarea now supports Ctrl+Click on wikilinks for navigation

## [0.1.0] - 2026-04-11

### Added

- Leptos 0.7 Client-Side Rendering (CSR) frontend framework
- Sidebar with lazy-loaded tree view for vault folder structure
- Markdown editor with live preview panel
- Graph view visualization of backlink relationships
- Canvas view for infinite canvas exploration
- Tauri IPC bridge for backend communication
- Drag-and-drop file and folder reordering using HTML5 DnD API
- Support for `.forgeignore` file in sidebar directory scanning
- Save indicator with fade animation and auto-clear after save
- Focus rings on all interactive elements (WCAG 2.1 AA compliance)
- Global dragover and drop listeners for WebView2 compatibility

### Changed

- Renamed crate from `src` to `forge-ui` for clarity and consistency
- Delete button opacity at rest changed from 0 to 0.3 for improved discoverability
