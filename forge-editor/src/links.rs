//! Wikilink and hyperlink extraction from Markdown AST events.
//!
//! Provides `PulldownWikilinkExtractor`, a concrete implementation of
//! [`forge_core::WikilinkExtractor`] using `pulldown-cmark` for parsing.

use forge_core::link::Link;
use forge_core::WikilinkExtractor;
use pulldown_cmark::{Event, Tag};

/// Concrete implementation of [`WikilinkExtractor`] using `pulldown-cmark`.
///
/// This struct wraps the `pulldown-cmark` parser and provides a stateless
/// extractor for both wikilinks and standard Markdown hyperlinks.
#[derive(Debug, Clone, Default)]
pub struct PulldownWikilinkExtractor;

impl PulldownWikilinkExtractor {
    /// Create a new `PulldownWikilinkExtractor`.
    pub fn new() -> Self {
        Self
    }
}

impl WikilinkExtractor for PulldownWikilinkExtractor {
    fn extract(&self, text: &str) -> Vec<Link> {
        extract_links_from_str(text)
    }
}

/// Extract all links (wikilinks + hyperlinks) from a sequence of
/// `pulldown_cmark` events.
pub fn extract_links<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Link> {
    let mut links = Vec::new();

    for event in events {
        match event {
            // Standard Markdown links
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                links.push(Link::Hyperlink {
                    url: dest_url.into_string(),
                    title: if title.is_empty() {
                        None
                    } else {
                        Some(title.into_string())
                    },
                });
            }
            // Wikilinks appear as plain text: `[[target]]`
            // A full implementation will pre-process them with a custom parser.
            Event::Text(text) => {
                let raw = text.as_ref();
                extract_wikilinks_from_text(raw, &mut links);
            }
            _ => {}
        }
    }

    links
}

/// Convenience wrapper: parse `text` as Markdown and extract all links.
///
/// This is the primary entry point for callers that only have a raw string
/// (e.g. `forge-vault` during vault scan).  It avoids exposing `pulldown_cmark`
/// as a direct dependency of every consumer.
///
/// **Wikilinks** are extracted by scanning the raw text directly because
/// `pulldown_cmark` interprets `[[` as nested link syntax and never emits
/// the brackets as plain `Text` events.  Standard Markdown hyperlinks are
/// then extracted via the AST.
pub fn extract_links_from_str(text: &str) -> Vec<Link> {
    let mut links = Vec::new();

    // 1. Wikilinks: scan raw text (pulldown-cmark mangles `[[...]]`).
    extract_wikilinks_from_text(text, &mut links);

    // 2. Standard Markdown hyperlinks: use the pulldown-cmark AST.
    let parser = pulldown_cmark::Parser::new(text);
    for event in parser {
        if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
            dest_url, title, ..
        }) = event
        {
            links.push(Link::Hyperlink {
                url: dest_url.into_string(),
                title: if title.is_empty() {
                    None
                } else {
                    Some(title.into_string())
                },
            });
        }
    }

    links
}

/// Scan a plain-text segment for `[[...]]` patterns.
fn extract_wikilinks_from_text(text: &str, out: &mut Vec<Link>) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_single_wikilink() {
        let text = "Here is a [[note]] reference.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0],
            Link::Wikilink {
                target: "note".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn extract_wikilink_with_alias() {
        let text = "Check out [[target|custom alias]] for more info.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0],
            Link::Wikilink {
                target: "target".to_string(),
                alias: Some("custom alias".to_string()),
            }
        );
    }

    #[test]
    fn extract_multiple_wikilinks() {
        let text = "See [[note1]] and [[note2]] and [[note3]].";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 3);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "note1".to_string(),
                alias: None,
            }
        );
        assert_eq!(
            links[1].clone(),
            Link::Wikilink {
                target: "note2".to_string(),
                alias: None,
            }
        );
        assert_eq!(
            links[2].clone(),
            Link::Wikilink {
                target: "note3".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn extract_multiple_wikilinks_with_mixed_aliases() {
        let text = "[[first]] and [[second|alias]] then [[third]].";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 3);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "first".to_string(),
                alias: None,
            }
        );
        assert_eq!(
            links[1].clone(),
            Link::Wikilink {
                target: "second".to_string(),
                alias: Some("alias".to_string()),
            }
        );
        assert_eq!(
            links[2].clone(),
            Link::Wikilink {
                target: "third".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn wikilinks_in_markdown_heading() {
        let text = "# Heading with [[linked-note]]";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "linked-note".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn wikilinks_in_markdown_bold() {
        let text = "This is **bold [[note]] text**.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "note".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn wikilinks_in_markdown_list() {
        let text = "- Item 1 with [[note1]]\n- Item 2 with [[note2]]\n- Item 3";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 2);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "note1".to_string(),
                alias: None,
            }
        );
        assert_eq!(
            links[1].clone(),
            Link::Wikilink {
                target: "note2".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn empty_text_returns_empty_links() {
        let text = "";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn text_without_links_returns_empty_links() {
        let text = "This is just plain text with no links at all.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn malformed_wikilink_opening_bracket_only() {
        let text = "This has a [[ but no closing.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 0, "malformed wikilink should be ignored");
    }

    #[test]
    fn malformed_wikilink_incomplete_closing() {
        let text = "Text with [[unclosed link.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 0, "incomplete wikilink should be ignored");
    }

    #[test]
    fn markdown_hyperlink_standard() {
        let text = "Here is a [link](https://example.com) to a website.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Hyperlink {
                url: "https://example.com".to_string(),
                title: None,
            }
        );
    }

    #[test]
    fn markdown_hyperlink_with_title() {
        let text = "[Example](https://example.com \"Example Site\")";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Hyperlink {
                url: "https://example.com".to_string(),
                title: Some("Example Site".to_string()),
            }
        );
    }

    #[test]
    fn mixed_wikilinks_and_hyperlinks() {
        let text = "Check [[internal-note]] and [external](https://example.com).";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 2);
        assert!(matches!(links[0], Link::Wikilink { .. }));
        assert!(matches!(links[1], Link::Hyperlink { .. }));
    }

    #[test]
    fn wikilink_with_unicode_target() {
        let text = "Link to [[café-notes]] for details.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "café-notes".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn wikilink_with_unicode_alias() {
        let text = "See [[file|Ñotas Importantes]] for reference.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "file".to_string(),
                alias: Some("Ñotas Importantes".to_string()),
            }
        );
    }

    #[test]
    fn wikilink_with_spaces_in_target() {
        let text = "Reference [[spaced target name]] here.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "spaced target name".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn consecutive_wikilinks_no_space() {
        let text = "[[first]][[second]]";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn pipe_character_outside_wikilink_ignored() {
        let text = "This | is | not | a wikilink [[but|this|is]].";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "but".to_string(),
                alias: Some("this|is".to_string()),
            }
        );
    }

    #[test]
    fn wikilink_with_special_chars() {
        let text = "Link to [[note-with_special.chars]] works.";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "note-with_special.chars".to_string(),
                alias: None,
            }
        );
    }

    /// Edge case: nested wikilinks (pathological).
    /// Greedy first-match semantics: finds `[[a [[b]]` as target.
    #[test]
    fn wikilink_nested_pathological() {
        let text = "[[a [[b]] c]]";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        // Greedy match: first [[ at index 0, first ]] after that is at "[[b]]",
        // so target = "a [[b" (from position 2 to position of first ]])
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "a [[b".to_string(),
                alias: None,
            }
        );
    }

    /// Edge case: unclosed wikilink (no closing ]]). Should return empty (clean abort).
    #[test]
    fn wikilink_unclosed_returns_empty() {
        let text = "broken [[no-close of this link";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 0);
    }

    /// Edge case: empty wikilink target. Should extract as-is (zero-length target).
    #[test]
    fn wikilink_empty_target() {
        let text = "[[]]";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "".to_string(),
                alias: None,
            }
        );
    }

    /// Edge case: wikilink with pipe but empty alias. Alias is Some("").
    #[test]
    fn wikilink_pipe_empty_alias() {
        let text = "[[target|]]";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "target".to_string(),
                alias: Some("".to_string()),
            }
        );
    }

    /// Edge case: multiline wikilink (newline traversal in string). Target includes newline.
    #[test]
    fn wikilink_multiline_newline_in_target() {
        let text = "start [[abc\nwith newline]] end";
        let links = extract_links_from_str(text);
        assert_eq!(links.len(), 1);
        // String-based parsing allows \n in target
        assert_eq!(
            links[0].clone(),
            Link::Wikilink {
                target: "abc\nwith newline".to_string(),
                alias: None,
            }
        );
    }
}
