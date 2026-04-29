# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — Phase 18: Wikilink Navigation

### Added

- `resolve_wikilink(&self, target: &str) -> Option<String>` — case-insensitive stem matching with heading fragment support and relative path resolution
- Integration tests for resolve_wikilink (7 test cases covering exact match, case-insensitive, heading fragments, non-existent targets, relative paths)

## [0.1.0] - 2026-04-11

### Added

- VaultStore: central abstraction for vault lifecycle management
- Tantivy full-text indexing engine with async index operations
- Incremental re-indexing via manifest diff detection
- Backlink graph construction from parsed wikilinks
- VaultWatcher: file system watcher with rename detection and debouncing
- Support for multiple simultaneous watchers per vault instance
