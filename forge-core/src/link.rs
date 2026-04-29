//! Link types and extraction trait for Forgedsidian.
//!
//! Defines the canonical [`Link`] type (wikilinks + hyperlinks) and the
//! [`WikilinkExtractor`] trait that abstracts extraction logic away from
//! specific parsing implementations (e.g. `pulldown-cmark`).

/// A link extracted from a Markdown document.
///
/// Represents either an Obsidian-style wikilink (`[[target]]` or `[[target|alias]]`)
/// or a standard Markdown hyperlink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// An Obsidian-style wikilink: `[[target]]` or `[[target|alias]]`.
    ///
    /// # Fields
    /// - `target`: The note title or path being referenced.
    /// - `alias`: Optional display text; if `None`, use `target` as display.
    Wikilink {
        target: String,
        alias: Option<String>,
    },
    /// A standard Markdown hyperlink.
    ///
    /// # Fields
    /// - `url`: The href destination.
    /// - `title`: Optional link title attribute.
    Hyperlink { url: String, title: Option<String> },
}

/// Abstraction for extracting links from Markdown text.
///
/// Implementors provide a strategy for parsing wikilinks and hyperlinks
/// from a raw string. This allows different extraction strategies (e.g. regex,
/// `pulldown-cmark` AST, custom parser) to be swapped without modifying
/// vault or graph logic.
///
/// # DESIGN
/// - **Stateless**: The extractor is stateless; extraction depends only on input text.
/// - **Pure**: No async/IO required; this is a pure computation.
/// - **Deterministic**: Same input always produces same output.
pub trait WikilinkExtractor: Send + Sync {
    /// Extract all links (wikilinks + hyperlinks) from the given text.
    ///
    /// # Arguments
    /// * `text` - Raw Markdown or plain text to scan for links.
    ///
    /// # Returns
    /// A `Vec<Link>` of all detected wikilinks and hyperlinks.
    /// If no links are found, returns an empty `Vec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use forge_core::{Link, SimpleWikilinkExtractor, WikilinkExtractor};
    ///
    /// let extractor = SimpleWikilinkExtractor;
    /// let links = extractor.extract("See [[my-note]] for details.");
    /// assert_eq!(links.len(), 1);
    /// assert!(matches!(&links[0], Link::Wikilink { .. }));
    /// ```
    fn extract(&self, text: &str) -> Vec<Link>;
}

/// Minimal, dependency-free [`WikilinkExtractor`] that scans for `[[...]]`
/// patterns using plain string operations.
///
/// This is the default extractor used by [`crate`]-level consumers that do
/// not want to pull in a full Markdown parser. It recognises:
///
/// - `[[target]]`          — plain wikilink
/// - `[[target|alias]]`    — wikilink with display alias
///
/// It does NOT extract standard Markdown hyperlinks (`[text](url)`) — for
/// that, use `forge_editor::PulldownWikilinkExtractor`.
///
/// This struct is a zero-sized type (ZST): constructing it is free and it
/// holds no state. Safe to use across threads.
///
/// # Examples
///
/// ```
/// use forge_core::{Link, SimpleWikilinkExtractor, WikilinkExtractor};
///
/// let extractor = SimpleWikilinkExtractor;
/// let links = extractor.extract("See [[alpha]] and [[beta|B]].");
/// assert_eq!(links.len(), 2);
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleWikilinkExtractor;

impl WikilinkExtractor for SimpleWikilinkExtractor {
    fn extract(&self, text: &str) -> Vec<Link> {
        let mut out = Vec::new();
        let mut remaining = text;
        while let Some(start) = remaining.find("[[") {
            remaining = &remaining[start + 2..];
            if let Some(end) = remaining.find("]]") {
                let inner = &remaining[..end];
                let (target, alias) = if let Some(pipe) = inner.find('|') {
                    (&inner[..pipe], Some(inner[pipe + 1..].to_owned()))
                } else {
                    (inner, None)
                };
                out.push(Link::Wikilink {
                    target: target.to_owned(),
                    alias,
                });
                remaining = &remaining[end + 2..];
            } else {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_extracts_single_wikilink() {
        let links = SimpleWikilinkExtractor.extract("a [[note]] b");
        assert_eq!(links.len(), 1);
        assert!(matches!(&links[0], Link::Wikilink { target, alias: None } if target == "note"));
    }

    #[test]
    fn simple_extracts_wikilink_with_alias() {
        let links = SimpleWikilinkExtractor.extract("ref [[path/to/note|Display]]");
        match &links[0] {
            Link::Wikilink { target, alias } => {
                assert_eq!(target, "path/to/note");
                assert_eq!(alias.as_deref(), Some("Display"));
            }
            _ => panic!("expected Wikilink variant"),
        }
    }

    #[test]
    fn simple_handles_unclosed_wikilink() {
        let links = SimpleWikilinkExtractor.extract("broken [[no-close");
        assert!(links.is_empty());
    }

    #[test]
    fn simple_extracts_multiple() {
        let links = SimpleWikilinkExtractor.extract("[[a]] and [[b]] and [[c|C]]");
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn simple_ignores_hyperlinks() {
        let links = SimpleWikilinkExtractor.extract("[text](https://example.com)");
        assert!(links.is_empty());
    }
}
