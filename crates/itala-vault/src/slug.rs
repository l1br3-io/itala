//! Title-to-slug conversion for note paths.
//!
//! Lowercase, hyphen-separated, ASCII-only. Non-ASCII characters are dropped
//! rather than transliterated (transliteration is locale-dependent and would
//! surprise users). Collisions are resolved by the caller, not by `slugify`,
//! by appending a ULID prefix to the filename.

use smol_str::SmolStr;

/// Converts `title` into a lowercase, hyphen-separated, ASCII-only slug.
///
/// Adjacent non-alphanumeric runs collapse to a single `-`. Leading and
/// trailing hyphens are stripped. The empty title maps to `"untitled"`.
///
/// ```
/// use itala_vault::slug::slugify;
/// assert_eq!(slugify("Hello, World!"), "hello-world");
/// assert_eq!(slugify("  multiple   spaces "), "multiple-spaces");
/// assert_eq!(slugify(""), "untitled");
/// ```
#[must_use]
pub fn slugify(title: &str) -> SmolStr {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = true; // start in "dash" state so leading dashes get stripped
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("untitled");
    }
    SmolStr::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
    }

    #[test]
    fn collapses_repeated_separators() {
        assert_eq!(slugify("a---b"), "a-b");
        assert_eq!(slugify("  multiple   spaces "), "multiple-spaces");
    }

    #[test]
    fn empty_title() {
        assert_eq!(slugify(""), "untitled");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(slugify("---"), "untitled");
    }

    #[test]
    fn drops_non_ascii() {
        // Non-ASCII dropped; adjacent alphanumerics stay separated by dashes.
        assert_eq!(slugify("café"), "caf");
        assert_eq!(slugify("Japanese 日本語"), "japanese");
    }

    #[test]
    fn preserves_digits() {
        assert_eq!(slugify("Note 42"), "note-42");
    }
}
