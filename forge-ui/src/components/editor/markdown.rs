//! Markdown processing with wikilink support.
//!
//! This module provides conversion from Markdown to sanitized HTML,
//! with clickable wikilinks preserved through the sanitization pipeline.

/// Replace `[[target]]` and `[[target|alias]]` with clickable anchor tags.
///
/// Produces `<a class="forge-wikilink" data-target="target">display</a>`.
/// The `data-target` attribute stores the raw wikilink target for IPC
/// resolution. Runs BEFORE ammonia sanitization to ensure the tags survive.
pub fn render_wikilinks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(start) = remaining.find("[[") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + 2..];

        if let Some(end) = remaining.find("]]") {
            let inner = &remaining[..end];
            let (target, display) = if let Some(pipe) = inner.find('|') {
                (&inner[..pipe], &inner[pipe + 1..])
            } else {
                (inner, inner)
            };
            // Escape HTML entities in target and display to prevent injection.
            let safe_target = target
                .replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let safe_display = display
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            result.push_str(&format!(
                "<a class=\"forge-wikilink\" data-target=\"{}\">{}</a>",
                safe_target, safe_display
            ));
            remaining = &remaining[end + 2..];
        } else {
            // Malformed — restore the opening brackets and stop.
            result.push_str("[[");
            break;
        }
    }
    result.push_str(remaining);
    result
}

/// Convert Markdown to sanitized HTML with clickable wikilinks.
///
/// Processing order:
/// 1. Replace `[[wikilinks]]` with `<a class="forge-wikilink">` tags
/// 2. Parse Markdown to HTML via pulldown-cmark
/// 3. Sanitize via ammonia (configured to preserve wikilink anchors)
pub fn md_to_html(markdown: &str) -> String {
    // Step 1: render wikilinks BEFORE markdown parsing.
    // This ensures `[[target]]` is not mangled by pulldown-cmark.
    let with_links = render_wikilinks(markdown);

    // Step 2: parse Markdown.
    let mut opts = pulldown_cmark::Options::empty();
    opts.insert(pulldown_cmark::Options::ENABLE_TABLES);
    opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    opts.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
    let parser = pulldown_cmark::Parser::new_ext(&with_links, opts);
    let mut raw_html = String::with_capacity(with_links.len() * 2);
    pulldown_cmark::html::push_html(&mut raw_html, parser);

    // Step 3: sanitize — allow our wikilink <a> tags through.
    // Note: add_allowed_classes handles the "class" attribute internally,
    // so we must NOT also add "class" via add_tag_attributes (they conflict).
    let mut builder = ammonia::Builder::default();
    builder
        .add_tag_attributes("a", &["data-target", "href"])
        .add_allowed_classes("a", &["forge-wikilink"]);
    builder.clean(&raw_html).to_string()
}

/// Detect if a file is Markdown by extension.
pub fn is_markdown(path: &str) -> bool {
    path.to_lowercase().ends_with(".md")
}
