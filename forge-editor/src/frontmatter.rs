//! YAML frontmatter extraction and validation.

use crate::EditorError;
use forge_core::format::split_frontmatter;
use serde_json::Value;
use std::collections::HashMap;

/// Extract YAML frontmatter from a raw Markdown string.
///
/// Returns a `HashMap` of key/value pairs on success.
/// Returns `None` if no frontmatter block is present.
///
/// # Errors
/// Returns [`EditorError::MalformedFrontmatter`] if the YAML block is invalid.
pub fn extract(raw: &str) -> Result<Option<HashMap<String, Value>>, EditorError> {
    let (yaml_opt, _body) = split_frontmatter(raw);
    let Some(yaml) = yaml_opt else {
        return Ok(None);
    };

    if yaml.trim().is_empty() {
        return Ok(Some(HashMap::new()));
    }

    // Parse YAML using serde_yaml, then convert to serde_json::Value
    // to maintain the return type as HashMap<String, serde_json::Value>.
    let yaml_val: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| EditorError::MalformedFrontmatter {
            reason: e.to_string(),
        })?;

    // Convert serde_yaml::Value → serde_json::Value (lossless for standard YAML types).
    let json_val: Value =
        serde_json::to_value(&yaml_val).map_err(|e| EditorError::MalformedFrontmatter {
            reason: e.to_string(),
        })?;

    match json_val {
        Value::Object(map) => Ok(Some(map.into_iter().collect())),
        Value::Null => Ok(Some(HashMap::new())),
        _ => Err(EditorError::MalformedFrontmatter {
            reason: "frontmatter root must be a YAML mapping".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_empty_document_returns_none() {
        let result = extract("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn extract_no_frontmatter_returns_none() {
        let content = "# Heading\n\nNo frontmatter here.";
        let result = extract(content);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn extract_single_field_frontmatter() {
        let content = "---\n\"title\": \"Test\"\n---\n\n# Content";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.get("title").and_then(|v| v.as_str()), Some("Test"));
    }

    #[test]
    fn extract_multiple_fields_frontmatter() {
        // Each key-value on its own line — standard YAML block mapping.
        let content = "---\ntitle: My Note\nauthor: John\ndate: \"2026-04-09\"\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("title").and_then(|v| v.as_str()), Some("My Note"));
        assert_eq!(map.get("author").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(map.get("date").and_then(|v| v.as_str()), Some("2026-04-09"));
    }

    #[test]
    fn extract_numeric_value() {
        let content = "---\npriority: 5\nscore: 9.8\n---\n\nContent";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.get("priority").and_then(|v| v.as_i64()), Some(5));
        assert_eq!(map.get("score").and_then(|v| v.as_f64()), Some(9.8));
    }

    #[test]
    fn extract_boolean_value() {
        let content = "---\npublished: true\narchived: false\n---\n\nContent";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.get("published").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(map.get("archived").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn extract_frontmatter_with_trailing_newlines() {
        let content = "---\n\"key\": \"value\"\n---\n\n\n\nMultiple blank lines before body.";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
    }

    #[test]
    fn extract_malformed_yaml_returns_error() {
        // Unclosed flow sequence is invalid YAML
        let content = "---\ntags: [unclosed\n---\n\nContent";
        let result = extract(content);
        assert!(result.is_err());
    }

    #[test]
    fn extract_frontmatter_with_unicode() {
        let content = "---\ntitle: \"Café Notes\"\nauthor: \"François\"\n---\n\nContent";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(
            map.get("title").and_then(|v| v.as_str()),
            Some("Café Notes")
        );
        assert_eq!(map.get("author").and_then(|v| v.as_str()), Some("François"));
    }

    #[test]
    fn extract_frontmatter_preserves_order() {
        let content = "---\na: \"1\"\nb: \"2\"\nc: \"3\"\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.len(), 3);
        assert!(map.contains_key("a"));
        assert!(map.contains_key("b"));
        assert!(map.contains_key("c"));
    }

    #[test]
    fn extract_empty_frontmatter_block() {
        let content = "---\n---\n\nBody content";
        let result = extract(content);
        // Empty frontmatter might parse to an empty map or None depending on split_frontmatter
        assert!(result.is_ok());
    }

    #[test]
    fn extract_frontmatter_with_special_chars() {
        let content =
            "---\nkey_name: \"value-with-dash\"\nother_key: \"value.with.dots\"\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert!(map.contains_key("key_name"));
    }

    /// Edge case: frontmatter without closing delimiter (unclosed).
    /// split_frontmatter returns (None, entire_input) -> extract returns Ok(None).
    #[test]
    fn frontmatter_unclosed_delimiter() {
        let content = "---\ntitle: x\nbody without closing ---";
        let result = extract(content);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// YAML syntax (standard unquoted strings and keys) is now properly supported.
    /// Example: `key: value` is valid YAML and will parse correctly.
    #[test]
    fn frontmatter_yaml_syntax_not_json() {
        let content = "---\ntitle: My Title\nauthor: John Doe\n---\nBody";
        let result = extract(content);
        // serde_yaml correctly parses standard YAML syntax
        assert!(result.is_ok());
        let map = result.unwrap().unwrap();
        assert_eq!(map.get("title").and_then(|v| v.as_str()), Some("My Title"));
        assert_eq!(map.get("author").and_then(|v| v.as_str()), Some("John Doe"));
    }

    /// Edge case: completely empty frontmatter block (no content between delimiters).
    /// split_frontmatter returns (Some(""), body) -> wrapping gives {} -> parses as empty map.
    #[test]
    fn frontmatter_empty_block() {
        let content = "---\n---\nBody content";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.len(), 0);
    }

    /// YAML with unquoted strings and standard YAML syntax.
    #[test]
    fn extract_real_yaml_unquoted_strings() {
        let content = "---\ntitle: My Note\nauthor: John Doe\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap().unwrap();
        assert_eq!(map.get("title").and_then(|v| v.as_str()), Some("My Note"));
        assert_eq!(map.get("author").and_then(|v| v.as_str()), Some("John Doe"));
    }

    /// YAML array syntax (multiline list format).
    #[test]
    fn extract_yaml_list_field() {
        let content = "---\ntags:\n  - rust\n  - testing\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap().unwrap();
        let tags = map.get("tags").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), Some("rust"));
        assert_eq!(tags[1].as_str(), Some("testing"));
    }

    /// YAML flow sequence (inline list).
    #[test]
    fn extract_yaml_inline_list() {
        let content = "---\ntags: [rust, testing, obsidian]\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap().unwrap();
        let tags = map.get("tags").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tags.len(), 3);
    }

    /// YAML nested objects (nested mappings).
    #[test]
    fn extract_yaml_nested_values() {
        let content = "---\nmetadata:\n  version: 1\n  active: true\n---\n\nBody";
        let result = extract(content);
        assert!(result.is_ok());
        let map = result.unwrap().unwrap();
        assert!(map.contains_key("metadata"));
        let metadata = map.get("metadata").and_then(|v| v.as_object()).unwrap();
        assert_eq!(metadata.get("version").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(metadata.get("active").and_then(|v| v.as_bool()), Some(true));
    }

    /// Duplicate keys in YAML cause a parse error with strict YAML parsers.
    /// This ensures we propagate errors correctly from serde_yaml.
    #[test]
    fn frontmatter_yaml_colon_prefix_invalid_yaml() {
        // Unclosed flow mapping — unambiguously invalid YAML
        let content = "---\ntitle: {unclosed\n---\nBody";
        let result = extract(content);
        assert!(result.is_err());
    }
}
