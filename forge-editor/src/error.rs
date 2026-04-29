//! Editor-specific errors.

use thiserror::Error;

/// Top-level error type for `forge-editor`.
#[derive(Debug, Error)]
pub enum EditorError {
    /// The frontmatter block is present but malformed YAML.
    #[error("malformed frontmatter: {reason}")]
    MalformedFrontmatter { reason: String },

    /// A wikilink target references a note that does not exist.
    #[error("broken wikilink: [[{target}]] not found in vault")]
    BrokenWikilink { target: String },

    /// Core error propagated from `forge-core`.
    #[error("core error: {0}")]
    Core(#[from] forge_core::CoreError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_malformed_frontmatter_display() {
        let error = EditorError::MalformedFrontmatter {
            reason: "invalid YAML syntax".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("malformed frontmatter"));
        assert!(message.contains("invalid YAML syntax"));
    }

    #[test]
    fn error_broken_wikilink_display() {
        let error = EditorError::BrokenWikilink {
            target: "missing-note".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("broken wikilink"));
        assert!(message.contains("missing-note"));
    }

    #[test]
    fn error_debug_formatting() {
        let error = EditorError::MalformedFrontmatter {
            reason: "test reason".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MalformedFrontmatter"));
    }

    #[test]
    fn error_equality() {
        let error1 = EditorError::BrokenWikilink {
            target: "note1".to_string(),
        };
        let error2 = EditorError::BrokenWikilink {
            target: "note1".to_string(),
        };
        assert_eq!(error1.to_string(), error2.to_string());
    }

    #[test]
    fn error_malformed_frontmatter_with_complex_reason() {
        let reason = "expected key at line 5, found invalid character: @";
        let error = EditorError::MalformedFrontmatter {
            reason: reason.to_string(),
        };
        let message = error.to_string();
        assert!(message.contains(reason));
    }

    #[test]
    fn error_broken_wikilink_with_special_chars() {
        let error = EditorError::BrokenWikilink {
            target: "café-notes-123".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("café-notes-123"));
    }
}
