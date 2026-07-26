//! Frontmatter delimiter splitting.
//!
//! Standard YAML-frontmatter convention: the file may start with `---\n`,
//! contain YAML until the next `---` (or `...`) line, then the Markdown body
//! follows. Body-internal `---` rules are *not* interpreted as frontmatter.

/// Result of splitting a Markdown document into frontmatter and body.
pub struct Split<'a> {
    /// YAML frontmatter slice, if a frontmatter block was present.
    pub frontmatter: Option<&'a str>,
    /// Byte range of the body within the original input.
    pub body_range: std::ops::Range<usize>,
}

/// Splits `input` into optional frontmatter and body byte range.
///
/// Rules:
/// - Frontmatter must start at offset 0 with `---` followed by a newline
///   (CRLF or LF).
/// - Frontmatter ends at the next line that is exactly `---` or `...`
///   (modulo trailing whitespace).
/// - Body starts at the beginning of the next line.
/// - If the opening delimiter is absent, or no closing delimiter is found,
///   the entire input is the body (no frontmatter is recognized).
#[must_use]
pub fn split_frontmatter(input: &str) -> Split<'_> {
    const OPEN: &str = "---";
    if !starts_with_delim(input, OPEN) {
        return Split {
            frontmatter: None,
            body_range: 0..input.len(),
        };
    }

    // Skip past the opening delimiter line.
    let yaml_start = match input[OPEN.len()..].find('\n') {
        Some(i) => OPEN.len() + i + 1,
        None => {
            return Split {
                frontmatter: None,
                body_range: 0..input.len(),
            };
        }
    };

    // Walk subsequent lines, tracking byte offsets manually so we handle
    // CRLF correctly (str::lines() strips the \r but reports substring
    // indices that don't account for it).
    let mut line_start = yaml_start;
    while line_start < input.len() {
        let rest = &input[line_start..];
        let line_end_rel = rest.find('\n').unwrap_or(rest.len());
        let line_end_abs = line_start + line_end_rel;
        let trimmed = rest[..line_end_rel].trim_end_matches(['\r', ' ']);

        if trimmed == "---" || trimmed == "..." {
            // Closing delimiter found.
            let yaml = &input[yaml_start..line_start];
            let yaml = yaml.strip_suffix('\n').unwrap_or(yaml);
            let yaml = yaml.strip_suffix('\r').unwrap_or(yaml);
            // Body starts after the closing delimiter's `\n`, if any.
            let body_start = if line_end_rel < rest.len() {
                line_end_abs + 1
            } else {
                line_end_abs
            };
            return Split {
                frontmatter: Some(yaml),
                body_range: body_start..input.len(),
            };
        }

        if line_end_rel < rest.len() {
            line_start = line_end_abs + 1;
        } else {
            break;
        }
    }

    Split {
        frontmatter: None,
        body_range: 0..input.len(),
    }
}

/// True if `input` starts with `delim` followed by a newline.
fn starts_with_delim(input: &str, delim: &str) -> bool {
    if !input.starts_with(delim) {
        return false;
    }
    matches!(input[delim.len()..].chars().next(), Some('\n' | '\r'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter() {
        let s = split_frontmatter("hello world");
        assert!(s.frontmatter.is_none());
        assert_eq!(&"hello world"[s.body_range], "hello world");
    }

    #[test]
    fn empty_frontmatter() {
        let s = split_frontmatter("---\n---\nbody");
        assert_eq!(s.frontmatter, Some(""));
        assert_eq!(&"---\n---\nbody"[s.body_range], "body");
    }

    #[test]
    fn simple_frontmatter() {
        let input = "---\ntitle: Hi\n---\n# Body\n";
        let s = split_frontmatter(input);
        assert_eq!(s.frontmatter, Some("title: Hi"));
        assert_eq!(&input[s.body_range], "# Body\n");
    }

    #[test]
    fn ellipsis_closes_frontmatter() {
        let input = "---\ntitle: Hi\n...\nbody";
        let s = split_frontmatter(input);
        assert_eq!(s.frontmatter, Some("title: Hi"));
        assert_eq!(&input[s.body_range], "body");
    }

    #[test]
    fn dashes_inside_body_are_not_frontmatter() {
        let input = "# Title\n\n---\nnot fm\n";
        let s = split_frontmatter(input);
        assert!(s.frontmatter.is_none());
        assert_eq!(&input[s.body_range], input);
    }

    #[test]
    fn dashes_without_trailing_newline_are_not_frontmatter() {
        let input = "---title";
        let s = split_frontmatter(input);
        assert!(s.frontmatter.is_none());
    }

    #[test]
    fn unterminated_frontmatter_falls_back_to_body() {
        let input = "---\ntitle: Hi\nno close\n";
        let s = split_frontmatter(input);
        assert!(s.frontmatter.is_none());
    }

    #[test]
    fn crlf_line_endings() {
        let input = "---\r\ntitle: Hi\r\n---\r\nbody\r\n";
        let s = split_frontmatter(input);
        assert_eq!(s.frontmatter, Some("title: Hi"));
        assert_eq!(&input[s.body_range], "body\r\n");
    }

    #[test]
    fn frontmatter_with_trailing_whitespace_on_close() {
        let input = "---\ntitle: Hi\n---   \nbody";
        let s = split_frontmatter(input);
        assert_eq!(s.frontmatter, Some("title: Hi"));
        assert_eq!(&input[s.body_range], "body");
    }

    #[test]
    fn frontmatter_with_no_body() {
        let input = "---\ntitle: Hi\n---\n";
        let s = split_frontmatter(input);
        assert_eq!(s.frontmatter, Some("title: Hi"));
        assert_eq!(&input[s.body_range], "");
    }

    #[test]
    fn frontmatter_with_multiline_yaml() {
        let input = "---\ntitle: Hi\ntags:\n  - a\n  - b\n---\nbody\n";
        let s = split_frontmatter(input);
        assert_eq!(s.frontmatter, Some("title: Hi\ntags:\n  - a\n  - b"));
        assert_eq!(&input[s.body_range], "body\n");
    }
}
