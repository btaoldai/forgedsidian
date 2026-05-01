//! Markdown → AST conversion via `pulldown-cmark`.

use pulldown_cmark::{Options, Parser};

/// Parse a Markdown string and collect all events into a `Vec`.
///
/// The returned events can be used for rendering, link extraction, or further
/// AST transformations.
pub fn parse(markdown: &str) -> Vec<pulldown_cmark::Event<'_>> {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    Parser::new_ext(markdown, opts).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::Event;

    #[test]
    fn parse_empty_markdown() {
        let events = parse("");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn parse_simple_text() {
        let events = parse("Hello, world!");
        assert!(!events.is_empty());
        // Should contain at least a paragraph start and text event
        assert!(events.iter().any(|e| matches!(e, Event::Text(_))));
    }

    #[test]
    fn parse_heading() {
        let events = parse("# My Heading");
        assert!(!events.is_empty());
        // Heading should generate Start(Tag::Heading(...))
        assert!(events.iter().any(|e| matches!(e, Event::Start(_))));
    }

    #[test]
    fn parse_bold_text() {
        let events = parse("**bold text**");
        assert!(!events.is_empty());
        // Bold should generate emphasis events
        let has_emphasis = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_emphasis);
    }

    #[test]
    fn parse_italic_text() {
        let events = parse("*italic text*");
        assert!(!events.is_empty());
        let has_emphasis = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_emphasis);
    }

    #[test]
    fn parse_code_block() {
        let events = parse("```\ncode block\n```");
        assert!(!events.is_empty());
        // Code block should have Start/End tag events
        let has_code = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_code);
    }

    #[test]
    fn parse_inline_code() {
        let events = parse("Use `cargo build` to compile.");
        assert!(!events.is_empty());
        // Inline code should be present in events
        assert!(!events.is_empty());
    }

    #[test]
    fn parse_unordered_list() {
        let events = parse("- Item 1\n- Item 2\n- Item 3");
        assert!(!events.is_empty());
        let has_list = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_list);
    }

    #[test]
    fn parse_ordered_list() {
        let events = parse("1. First\n2. Second\n3. Third");
        assert!(!events.is_empty());
        let has_list = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_list);
    }

    #[test]
    fn parse_blockquote() {
        let events = parse("> This is a quote\n> with multiple lines");
        assert!(!events.is_empty());
        let has_blockquote = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_blockquote);
    }

    #[test]
    fn parse_table_with_tables_enabled() {
        let events = parse("| Col1 | Col2 |\n|------|------|\n| A    | B    |");
        // Tables are enabled via Options::ENABLE_TABLES
        assert!(!events.is_empty());
    }

    #[test]
    fn parse_strikethrough_with_strikethrough_enabled() {
        let events = parse("~~deleted text~~");
        // Strikethrough is enabled via Options::ENABLE_STRIKETHROUGH
        assert!(!events.is_empty());
    }

    #[test]
    fn parse_tasklist_with_tasklist_enabled() {
        let events = parse("- [x] Completed task\n- [ ] Incomplete task");
        // Tasklists are enabled via Options::ENABLE_TASKLISTS
        assert!(!events.is_empty());
    }

    #[test]
    fn parse_multiline_markdown() {
        let markdown = "# Title\n\nSome paragraph.\n\n**Bold** and *italic*.";
        let events = parse(markdown);
        assert!(!events.is_empty());
    }

    #[test]
    fn parse_with_unicode() {
        let events = parse("Café, naïve, résumé: café");
        assert!(!events.is_empty());
        let has_text = events.iter().any(|e| matches!(e, Event::Text(_)));
        assert!(has_text);
    }

    #[test]
    fn parse_mixed_markdown_elements() {
        let markdown =
            "# Heading\n\nParagraph with **bold** and *italic*.\n\n- List item\n\n> Quote";
        let events = parse(markdown);
        assert!(!events.is_empty());
    }

    #[test]
    fn parse_hyperlink() {
        let events = parse("[Click here](https://example.com)");
        assert!(!events.is_empty());
        let has_link = events.iter().any(|e| matches!(e, Event::Start(_)));
        assert!(has_link);
    }

    #[test]
    fn parse_image() {
        let events = parse("![alt text](image.png)");
        assert!(!events.is_empty());
    }
}
