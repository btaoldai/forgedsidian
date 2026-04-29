//! Test suite for wikilink rendering, extraction, and file type detection.
//!
//! All 21 tests operate on pure `&str` functions with no DOM dependencies:
//! - `render_wikilinks`: String transformation (7 tests)
//! - `extract_wikilink_at_cursor`: String parsing (7 tests)
//! - `is_external_file`: String pattern matching (7 tests)
//!
//! **Execution**: Tests run natively on all platforms via `cargo test --lib -p forge-ui`.
//! No browser harness required. `wasm-bindgen-test` is listed in dev-dependencies
//! for WASM compatibility, but these tests use the native `#[test]` attribute.

#[cfg(test)]
mod tests {
    use super::super::markdown::{render_wikilinks, md_to_html, is_markdown};
    use super::super::wikilink::{extract_wikilink_at_cursor, is_external_file};

    // --- render_wikilinks ---

    #[test]
    fn render_single_wikilink() {
        let result = render_wikilinks("See [[my note]] here.");
        assert!(result.contains("data-target=\"my note\""));
        assert!(result.contains(">my note</a>"));
        assert!(result.contains("class=\"forge-wikilink\""));
    }

    #[test]
    fn render_wikilink_with_alias() {
        let result = render_wikilinks("Check [[target|display text]] out.");
        assert!(result.contains("data-target=\"target\""));
        assert!(result.contains(">display text</a>"));
    }

    #[test]
    fn render_multiple_wikilinks() {
        let result = render_wikilinks("[[first]] and [[second]]");
        assert!(result.contains("data-target=\"first\""));
        assert!(result.contains("data-target=\"second\""));
    }

    #[test]
    fn render_no_wikilinks() {
        let text = "Just plain text with no links.";
        assert_eq!(render_wikilinks(text), text);
    }

    #[test]
    fn render_malformed_wikilink() {
        let result = render_wikilinks("Broken [[unclosed link.");
        assert!(result.contains("[["));
        assert!(!result.contains("forge-wikilink"));
    }

    #[test]
    fn render_wikilink_escapes_html() {
        let result = render_wikilinks("[[<script>alert(1)</script>]]");
        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
    }

    #[test]
    fn render_wikilink_escapes_xss_payloads() {
        // Test img with onerror handler — must be escaped.
        let result = render_wikilinks("[[<img src=x onerror=alert(1)>]]");
        assert!(!result.contains("<img"));
        assert!(!result.contains("onerror="));
        assert!(result.contains("&lt;img"));

        // Test svg with onload handler — must be escaped.
        let result = render_wikilinks("[[<svg onload=alert(1)>]]");
        assert!(!result.contains("<svg"));
        assert!(!result.contains("onload="));
        assert!(result.contains("&lt;svg"));

        // Test javascript: protocol — must be escaped.
        let result = render_wikilinks("[[javascript:alert(1)]]");
        assert!(!result.contains("javascript:"));
        assert!(result.contains("&lt;"));

        // Test anchor with javascript: protocol — must be escaped.
        let result = render_wikilinks("[[<a href=\"javascript:void(0)\">click</a>]]");
        assert!(!result.contains("<a href="));
        assert!(!result.contains("javascript:"));
        assert!(result.contains("&lt;a"));
    }

    #[test]
    fn render_wikilink_with_heading() {
        let result = render_wikilinks("See [[note#heading]] for details.");
        assert!(result.contains("data-target=\"note#heading\""));
    }

    // --- extract_wikilink_at_cursor ---

    #[test]
    fn cursor_inside_wikilink() {
        let text = "Hello [[my note]] world";
        // cursor at position 10 is inside "my note"
        assert_eq!(extract_wikilink_at_cursor(text, 10), Some("my note".to_string()));
    }

    #[test]
    fn cursor_outside_wikilink() {
        let text = "Hello [[my note]] world";
        // cursor at position 2 is in "Hello"
        assert_eq!(extract_wikilink_at_cursor(text, 2), None);
    }

    #[test]
    fn cursor_inside_aliased_wikilink() {
        let text = "See [[target|alias]] here";
        assert_eq!(extract_wikilink_at_cursor(text, 8), Some("target".to_string()));
    }

    #[test]
    fn cursor_between_wikilinks() {
        let text = "[[first]] gap [[second]]";
        // cursor in "gap" at position 12
        assert_eq!(extract_wikilink_at_cursor(text, 12), None);
    }

    #[test]
    fn cursor_at_start() {
        let text = "[[note]]";
        assert_eq!(extract_wikilink_at_cursor(text, 0), None);
    }

    #[test]
    fn cursor_past_end() {
        let text = "[[note]]";
        assert_eq!(extract_wikilink_at_cursor(text, 100), None);
    }

    // --- is_external_file ---

    #[test]
    fn html_is_external() {
        assert!(is_external_file("strategie-revenus-2026.html"));
        assert!(is_external_file("page.htm"));
        assert!(is_external_file("Folder/Report.HTML"));
    }

    #[test]
    fn md_is_not_external() {
        assert!(!is_external_file("my-note"));
        assert!(!is_external_file("my-note.md"));
        assert!(!is_external_file("folder/deep/note.md"));
    }

    #[test]
    fn media_files_are_external() {
        assert!(is_external_file("image.png"));
        assert!(is_external_file("photo.jpg"));
        assert!(is_external_file("diagram.svg"));
        assert!(is_external_file("video.mp4"));
        assert!(is_external_file("track.mp3"));
    }

    #[test]
    fn office_files_are_external() {
        assert!(is_external_file("report.pdf"));
        assert!(is_external_file("budget.xlsx"));
        assert!(is_external_file("slides.pptx"));
        assert!(is_external_file("letter.docx"));
        assert!(is_external_file("data.csv"));
    }

    #[test]
    fn heading_fragment_stripped_before_check() {
        assert!(is_external_file("page.html#section1"));
        assert!(!is_external_file("note#heading"));
        assert!(!is_external_file("note.md#heading"));
    }

    #[test]
    fn archive_files_are_external() {
        assert!(is_external_file("backup.zip"));
        assert!(is_external_file("archive.7z"));
        assert!(is_external_file("package.tar"));
        assert!(is_external_file("logs.gz"));
    }

    #[test]
    fn no_extension_is_not_external() {
        assert!(!is_external_file("README"));
        assert!(!is_external_file("Makefile"));
        assert!(!is_external_file("folder/note"));
    }
}
