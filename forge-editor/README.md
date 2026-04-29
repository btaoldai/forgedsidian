# forge-editor

Markdown AST, inline editing logic and YAML frontmatter parsing.

## Purpose

Parses Markdown strings into an AST, extracts and validates YAML frontmatter, and identifies wikilinks and hyperlinks in note content. Provides the text processing foundation for inline edits and content analysis.

## Key modules

- `parser` — Markdown string to pulldown_cmark AST conversion
- `frontmatter` — YAML frontmatter extraction and validation
- `links` — wikilink (`[[target]]`) and standard hyperlink extraction
- `error` — editor-specific error types

## Dependencies

- Internal: forge-core
- External: pulldown_cmark, serde_yaml

## Usage

```rust
use forge_editor::parser::Parser;

let parser = Parser::new();
let (ast, frontmatter) = parser.parse(markdown_string)?;
let wikilinks = parser.extract_wikilinks(&ast)?;
```

## Related docs

- Diff API and inline editing planned for Phase 2+
- Integration point with forge-vault for content retrieval
- Link resolution handled by forge-graph queries
