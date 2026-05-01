//! Typed frontmatter parsing for Obsidian-compatible Markdown notes.
//!
//! Provides zero-dependency parsing of YAML frontmatter blocks into strongly
//! typed fields. Supports both inline (`[a, b, c]`) and block (`- a`) list formats.
//!
//! # Design
//! - **Zero-dependency**: Pure string parsing, no `serde_yaml` or external YAML libraries.
//! - **Infallible**: Returns `Default` on any parse error; never panics.
//! - **Normalized**: Tags and aliases are lowercased and trimmed for consistency.
//! - **Obsidian-compatible**: Handles both standard YAML list syntaxes.

use crate::format::split_frontmatter;
use serde::{Deserialize, Serialize};

/// Parsed typed frontmatter from an Obsidian-compatible Markdown note.
///
/// Supports the two common YAML list formats for `tags` and `aliases`:
/// - Inline:  `tags: [rust, testing]`
/// - Block:   `tags:\n  - rust\n  - testing`
///
/// All tag/alias values are trimmed and lowercased for consistency.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    /// Frontmatter `tags:` list (normalized: trimmed, lowercased).
    pub tags: Vec<String>,
    /// Frontmatter `aliases:` list (normalized: trimmed, lowercased).
    pub aliases: Vec<String>,
    /// Frontmatter `title:` value (trimmed, original case preserved).
    pub title: Option<String>,
    /// Frontmatter `date:` or `created:` value (trimmed).
    pub date: Option<String>,
}

/// Extract a YAML list field value for a given key.
///
/// Handles both inline and block list formats:
/// - Inline: `tags: [rust, testing, obsidian]` → `["rust", "testing", "obsidian"]`
/// - Block:  `tags:\n  - rust\n  - testing` → `["rust", "testing"]`
///
/// Each value is normalized: trimmed, lowercased, and emptys are filtered.
/// Surrounding quotes are stripped if present.
fn parse_yaml_list(yaml: &str, key: &str) -> Vec<String> {
    let mut results = Vec::new();

    // Search for the key line: `key: ...`
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&format!("{key}:")) {
            // Extract what comes after `key:`
            let after_colon = &trimmed[key.len() + 1..].trim_start();

            // Check if it's an inline list: starts with `[`
            if after_colon.starts_with('[') {
                // Inline list format: `[item1, item2, ...]`
                if let Some(close_bracket) = after_colon.find(']') {
                    let list_content = &after_colon[1..close_bracket];
                    for item in list_content.split(',') {
                        let normalized = item
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .trim()
                            .to_lowercase();
                        if !normalized.is_empty() {
                            results.push(normalized);
                        }
                    }
                }
                return results;
            } else if after_colon.is_empty() || after_colon.ends_with(':') {
                // Block list format: `key:` with items on following lines
                // Collect all subsequent lines that start with `  -` or `- `
                let mut found_key = false;
                for block_line in yaml.lines() {
                    let block_trimmed = block_line.trim_start();
                    if block_trimmed.starts_with(&format!("{key}:")) {
                        found_key = true;
                        continue;
                    }
                    if found_key {
                        // Check if this line is part of the list (starts with `  -` or similar)
                        if let Some(stripped) = block_line.strip_prefix("  -") {
                            let item_text = stripped.trim_start();
                            if item_text.starts_with('-') {
                                // Pattern `---` not a list item, stop
                                break;
                            }
                            let normalized = item_text
                                .trim_matches('"')
                                .trim_matches('\'')
                                .trim()
                                .to_lowercase();
                            if !normalized.is_empty() {
                                results.push(normalized);
                            }
                        } else if let Some(stripped) = block_line.strip_prefix("- ") {
                            let normalized = stripped
                                .trim_matches('"')
                                .trim_matches('\'')
                                .trim()
                                .to_lowercase();
                            if !normalized.is_empty() {
                                results.push(normalized);
                            }
                        } else if block_line.trim().is_empty() {
                            // Empty line might continue list in some YAML; check next
                            continue;
                        } else if !block_line.starts_with(' ') && !block_line.trim().is_empty() {
                            // Non-indented non-empty line: end of this list
                            break;
                        }
                    }
                }
                return results;
            } else {
                // Single-line inline value without brackets (not a list)
                return results;
            }
        }
    }

    results
}

/// Extract a simple scalar YAML field for a given key.
///
/// Handles: `key: value`
///
/// Returns the value part after the colon, trimmed. Surrounding quotes are
/// stripped if present. Returns `None` if the key is absent or has no value.
fn parse_yaml_scalar(yaml: &str, key: &str) -> Option<String> {
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&format!("{key}:")) {
            let after_colon = &trimmed[key.len() + 1..].trim_start();
            if after_colon.is_empty() {
                return None;
            }
            let value = after_colon
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
            if value.is_empty() {
                return None;
            }
            return Some(value);
        }
    }
    None
}

/// Parse `NoteFrontmatter` from the raw Markdown content of a note.
///
/// Calls [`split_frontmatter`] to extract the YAML block, then parses known
/// fields with zero external dependencies (pure string parsing).
///
/// Returns [`NoteFrontmatter::default()`] (all fields empty/None) if:
/// - There is no frontmatter block
/// - A field is absent or unparsable
///
/// This function is **infallible** by design — a partially-valid frontmatter
/// is better than a broken vault open.
///
/// # Examples
///
/// ```
/// use forge_core::parse_frontmatter;
///
/// let raw = r#"---
/// title: My Note
/// tags: [rust, testing]
/// date: 2026-04-16
/// ---
/// # Body content
/// "#;
///
/// let fm = parse_frontmatter(raw);
/// assert_eq!(fm.title, Some("My Note".to_string()));
/// assert_eq!(fm.tags, vec!["rust".to_string(), "testing".to_string()]);
/// assert_eq!(fm.date, Some("2026-04-16".to_string()));
/// ```
pub fn parse_frontmatter(raw: &str) -> NoteFrontmatter {
    let (yaml_opt, _body) = split_frontmatter(raw);

    let yaml = match yaml_opt {
        Some(y) => y,
        None => return NoteFrontmatter::default(),
    };

    let tags = parse_yaml_list(yaml, "tags");
    let aliases = parse_yaml_list(yaml, "aliases");
    let title = parse_yaml_scalar(yaml, "title");
    // Try `date` first, fall back to `created`
    let date = parse_yaml_scalar(yaml, "date").or_else(|| parse_yaml_scalar(yaml, "created"));

    NoteFrontmatter {
        tags,
        aliases,
        title,
        date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== No frontmatter tests =====

    /// Test parse_frontmatter with no frontmatter block.
    #[test]
    fn test_no_frontmatter() {
        let raw = "# Just a heading\nNo frontmatter here";
        let fm = parse_frontmatter(raw);
        assert!(fm.tags.is_empty());
        assert!(fm.aliases.is_empty());
        assert_eq!(fm.title, None);
        assert_eq!(fm.date, None);
    }

    /// Test parse_frontmatter with empty frontmatter block.
    #[test]
    fn test_empty_frontmatter_block() {
        let raw = "---\n---\nBody content";
        let fm = parse_frontmatter(raw);
        assert!(fm.tags.is_empty());
        assert!(fm.aliases.is_empty());
        assert_eq!(fm.title, None);
        assert_eq!(fm.date, None);
    }

    // ===== Tags: inline list tests =====

    /// Test inline tags parsing with basic items.
    #[test]
    fn test_tags_inline_basic() {
        let raw = "---\ntags: [rust, testing, obsidian]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["rust", "testing", "obsidian"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test inline tags are lowercased.
    #[test]
    fn test_tags_inline_lowercased() {
        let raw = "---\ntags: [Rust, TESTING, Obsidian]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["rust", "testing", "obsidian"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test inline tags with spaces are trimmed.
    #[test]
    fn test_tags_inline_trimmed() {
        let raw = "---\ntags: [  rust  ,  testing  ,  obsidian  ]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["rust", "testing", "obsidian"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test inline tags with double quotes are stripped.
    #[test]
    fn test_tags_inline_quoted() {
        let raw = r#"---
tags: ["rust", "testing", "obsidian"]
---
Body"#;
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["rust", "testing", "obsidian"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test inline tags with single quotes are stripped.
    #[test]
    fn test_tags_inline_single_quoted() {
        let raw = "---\ntags: ['rust', 'testing', 'obsidian']\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["rust", "testing", "obsidian"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test empty inline tags list.
    #[test]
    fn test_tags_inline_empty() {
        let raw = "---\ntags: []\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert!(fm.tags.is_empty());
    }

    // ===== Tags: block list tests =====

    /// Test block-format tags parsing.
    #[test]
    fn test_tags_block_basic() {
        let raw = "---\ntags:\n  - rust\n  - testing\n  - obsidian\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["rust", "testing", "obsidian"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test block-format tags with spaces and quotes.
    #[test]
    fn test_tags_block_with_spaces_and_quotes() {
        let raw = r#"---
tags:
  - "My Tag"
  - "Another Tag"
  -   spaced-tag
---
Body"#;
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.tags,
            ["my tag", "another tag", "spaced-tag"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    // ===== Aliases tests =====

    /// Test aliases inline parsing.
    #[test]
    fn test_aliases_inline() {
        let raw = "---\naliases: [alt1, alt2, alt3]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.aliases,
            ["alt1", "alt2", "alt3"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// Test aliases block parsing.
    #[test]
    fn test_aliases_block() {
        let raw = "---\naliases:\n  - alias1\n  - alias2\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(
            fm.aliases,
            ["alias1", "alias2"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    // ===== Title tests =====

    /// Test title scalar extraction.
    #[test]
    fn test_title_basic() {
        let raw = "---\ntitle: My Note Title\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("My Note Title".to_string()));
    }

    /// Test title with quotes.
    #[test]
    fn test_title_quoted() {
        let raw = r#"---
title: "My Note Title"
---
Body"#;
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("My Note Title".to_string()));
    }

    /// Test title preserves case.
    #[test]
    fn test_title_case_preserved() {
        let raw = "---\ntitle: Rust ASYNC Testing\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Rust ASYNC Testing".to_string()));
    }

    /// Test missing title returns None.
    #[test]
    fn test_title_missing() {
        let raw = "---\ntags: [rust]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, None);
    }

    // ===== Date tests =====

    /// Test date field extraction.
    #[test]
    fn test_date_basic() {
        let raw = "---\ndate: 2026-04-16\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.date, Some("2026-04-16".to_string()));
    }

    /// Test created field as fallback when date is absent.
    #[test]
    fn test_created_as_fallback() {
        let raw = "---\ncreated: 2026-04-01\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.date, Some("2026-04-01".to_string()));
    }

    /// Test date takes precedence over created.
    #[test]
    fn test_date_precedence() {
        let raw = "---\ndate: 2026-04-16\ncreated: 2026-04-01\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.date, Some("2026-04-16".to_string()));
    }

    /// Test missing both date and created returns None.
    #[test]
    fn test_date_and_created_missing() {
        let raw = "---\ntitle: My Note\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.date, None);
    }

    // ===== Unicode and special characters =====

    /// Test tags with unicode characters.
    #[test]
    fn test_tags_unicode() {
        let raw = "---\ntags: [日本語, café, ñoño]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.tags.len(), 3);
        assert!(fm.tags.contains(&"日本語".to_string()));
        assert!(fm.tags.contains(&"café".to_string()));
        assert!(fm.tags.contains(&"ñoño".to_string()));
    }

    /// Test tags with underscores and hyphens.
    #[test]
    fn test_tags_underscores_hyphens() {
        let raw = "---\ntags: [my-tag, rust_async, test-case_2]\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert!(fm.tags.contains(&"my-tag".to_string()));
        assert!(fm.tags.contains(&"rust_async".to_string()));
        assert!(fm.tags.contains(&"test-case_2".to_string()));
    }

    // ===== Combined fields =====

    /// Test full frontmatter with all fields present.
    #[test]
    fn test_full_frontmatter() {
        let raw = r#"---
title: Complex Note
tags: [rust, async, testing]
aliases: [alt-name, code-example]
date: 2026-04-16
---
# Heading

Some body content here.
"#;
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Complex Note".to_string()));
        assert_eq!(fm.tags.len(), 3);
        assert_eq!(fm.aliases.len(), 2);
        assert_eq!(fm.date, Some("2026-04-16".to_string()));
    }

    /// Test note with only title, no tags.
    #[test]
    fn test_only_title() {
        let raw = "---\ntitle: Just Title\n---\nBody";
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Just Title".to_string()));
        assert!(fm.tags.is_empty());
        assert!(fm.aliases.is_empty());
        assert_eq!(fm.date, None);
    }

    /// Test body content is not included in tags.
    #[test]
    fn test_body_not_parsed_as_tags() {
        let raw = r#"---
tags: [note]
---
# Heading with tags: [body, tags, here]
Content with - list items
  - should not be parsed
"#;
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.tags, vec!["note"]);
    }

    /// Test mixed inline and block fields.
    #[test]
    fn test_mixed_inline_and_block_fields() {
        let raw = r#"---
title: Mixed Fields
tags: [inline-tag1, inline-tag2]
aliases:
  - alias1
  - alias2
date: 2026-04-16
---
Body"#;
        let fm = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Mixed Fields".to_string()));
        assert_eq!(fm.tags.len(), 2);
        assert_eq!(fm.aliases.len(), 2);
        assert_eq!(fm.date, Some("2026-04-16".to_string()));
    }

    /// Test frontmatter with extra metadata fields (ignored but safe).
    #[test]
    fn test_extra_metadata_fields_ignored() {
        let raw = r#"---
title: My Note
tags: [rust]
author: Baptiste
version: 1.0
custom_field: some_value
---
Body"#;
        let fm = parse_frontmatter(raw);
        // Extra fields are simply ignored, no error
        assert_eq!(fm.title, Some("My Note".to_string()));
        assert_eq!(fm.tags, vec!["rust"]);
    }

    /// Test that empty tag/alias values are filtered out.
    #[test]
    fn test_empty_values_filtered() {
        let raw = r#"---
tags: [rust, "", testing, '']
aliases: [alias1, , alias2]
---
Body"#;
        let fm = parse_frontmatter(raw);
        // Empty strings should be filtered out
        assert!(!fm.tags.is_empty());
        assert!(!fm.tags.contains(&String::new()));
    }
}
