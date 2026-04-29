# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — Phase 18: Wikilink Navigation

### Added

- IPC command `resolve_wikilink` — resolves wikilink targets to absolute file paths
- Registered in Tauri invoke_handler alongside existing vault_ops commands

## [0.1.0] - 2026-04-11

### Added

- Tauri 2 application shell with native desktop integration
- IPC command layer for frontend-backend communication
- 16 core commands: vault operations, file CRUD, search, and canvas commands
- Path traversal validation on all commands with zero-trust principles
- Content Security Policy (CSP) configuration in tauri.conf.json
- Support for `.forgeignore` file in sidebar directory scanning
- HTML5 in-page drag-and-drop compatibility via `dragDropEnabled: false` setting

### Changed

- Refactored monolithic commands.rs (1058 lines) into modular structure:
  - commands/mod.rs: command dispatcher
  - commands/vault_ops.rs: vault operations
  - commands/file_ops.rs: file CRUD operations
  - commands/scan.rs: directory scanning
- Extracted `start_watcher()` helper function (removed 35 lines of duplication)
- Extracted `reject_traversal()` helper function for DRY path validation across all commands
