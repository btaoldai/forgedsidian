//! File format constants and helpers.

/// The standard frontmatter delimiter used in Obsidian-compatible Markdown.
pub const FRONTMATTER_DELIMITER: &str = "---";

/// File extensions recognised as vault notes.
pub const NOTE_EXTENSIONS: &[&str] = &["md", "markdown"];

/// Splits a raw Markdown document into an optional frontmatter block and the
/// note body.
///
/// Returns `(frontmatter_yaml, body_markdown)`.  If no frontmatter is found
/// the first element is `None` and the second is the entire input.
pub fn split_frontmatter(raw: &str) -> (Option<&str>, &str) {
    let raw = raw.trim_start();
    if !raw.starts_with(FRONTMATTER_DELIMITER) {
        return (None, raw);
    }
    // Skip the opening `---\n`
    let after_open = &raw[FRONTMATTER_DELIMITER.len()..];
    if let Some(close_pos) = after_open.find(&format!("\n{FRONTMATTER_DELIMITER}")) {
        let yaml = after_open[..close_pos].trim_start_matches('\n');
        let body_start = close_pos + 1 + FRONTMATTER_DELIMITER.len();
        let body = after_open
            .get(body_start..)
            .unwrap_or("")
            .trim_start_matches('\n');
        (Some(yaml), body)
    } else {
        (None, raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify split_frontmatter with valid YAML frontmatter.
    #[test]
    fn test_split_frontmatter_valid_yaml() {
        let input = "---\ntitle: Hello\ndate: 2026-04-09\n---\n# Body\nContent here";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, Some("title: Hello\ndate: 2026-04-09"));
        assert_eq!(body, "# Body\nContent here");
    }

    /// Verify split_frontmatter with minimal frontmatter.
    #[test]
    fn test_split_frontmatter_minimal() {
        let input = "---\n---\nJust body";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, Some(""));
        assert_eq!(body, "Just body");
    }

    /// Verify split_frontmatter with no frontmatter returns None.
    #[test]
    fn test_split_frontmatter_no_frontmatter() {
        let input = "# This is a heading\nNo frontmatter here";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, None);
        assert_eq!(body, input);
    }

    /// Verify split_frontmatter with unclosed frontmatter returns None.
    #[test]
    fn test_split_frontmatter_unclosed() {
        let input = "---\ntitle: Hello\nNo closing delimiter\nJust body";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, None);
        assert_eq!(body, input);
    }

    /// Verify split_frontmatter with leading whitespace is trimmed.
    #[test]
    fn test_split_frontmatter_leading_whitespace() {
        let input = "   \n---\nkey: value\n---\nBody content";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, Some("key: value"));
        assert_eq!(body, "Body content");
    }

    /// Verify split_frontmatter with multiple newlines after closing delimiter.
    #[test]
    fn test_split_frontmatter_multiple_newlines_after_close() {
        let input = "---\nfrontmatter\n---\n\n\nBody starts here";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, Some("frontmatter"));
        assert_eq!(body, "Body starts here");
    }

    /// Verify split_frontmatter with empty body.
    #[test]
    fn test_split_frontmatter_empty_body() {
        let input = "---\nkey: value\n---\n";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, Some("key: value"));
        assert_eq!(body, "");
    }

    /// Verify split_frontmatter with complex YAML content.
    #[test]
    fn test_split_frontmatter_complex_yaml() {
        let input = "---\ntitle: My Note\ntags:\n  - rust\n  - testing\nmetadata:\n  version: 1.0\n---\n## Heading\n\nSome **bold** text";
        let (fm, body) = split_frontmatter(input);

        assert!(fm.is_some());
        let fm_content = fm.unwrap();
        assert!(fm_content.contains("title: My Note"));
        assert!(fm_content.contains("tags:"));
        assert_eq!(body, "## Heading\n\nSome **bold** text");
    }

    /// Verify split_frontmatter with dashes in body content does not interfere.
    #[test]
    fn test_split_frontmatter_dashes_in_body() {
        let input = "---\ntitle: Test\n---\n---\nThis has dashes\n---\nMore content";
        let (fm, body) = split_frontmatter(input);

        assert_eq!(fm, Some("title: Test"));
        assert_eq!(body, "---\nThis has dashes\n---\nMore content");
    }

    /// Verify that NOTE_EXTENSIONS constant contains expected values.
    #[test]
    fn test_note_extensions_constant() {
        assert_eq!(NOTE_EXTENSIONS.len(), 2);
        assert!(NOTE_EXTENSIONS.contains(&"md"));
        assert!(NOTE_EXTENSIONS.contains(&"markdown"));
    }

    /// Verify that FRONTMATTER_DELIMITER constant is correct.
    #[test]
    fn test_frontmatter_delimiter_constant() {
        assert_eq!(FRONTMATTER_DELIMITER, "---");
    }
}
