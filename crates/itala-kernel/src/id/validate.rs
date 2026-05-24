use crate::error::Error;
use crate::id::IdKind;

pub(super) fn plugin_id(s: &str) -> Result<(), Error> {
    if s.is_empty() || s.len() > 128 {
        return Err(Error::InvalidId {
            kind: IdKind::Plugin,
            raw: s.to_owned(),
        });
    }
    // RDN: at least two dot-separated segments of [a-z][a-z0-9-]*
    let mut dot_count = 0usize;
    for segment in s.split('.') {
        if !is_valid_kebab_segment(segment) {
            return Err(Error::InvalidId {
                kind: IdKind::Plugin,
                raw: s.to_owned(),
            });
        }
        dot_count += 1;
    }
    if dot_count < 2 {
        return Err(Error::InvalidId {
            kind: IdKind::Plugin,
            raw: s.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn block_id(s: &str) -> Result<(), Error> {
    if s.is_empty() || s.len() > 64 {
        return Err(Error::InvalidId {
            kind: IdKind::Block,
            raw: s.to_owned(),
        });
    }
    if !s
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(Error::InvalidId {
            kind: IdKind::Block,
            raw: s.to_owned(),
        });
    }
    Ok(())
}

pub fn capability_name(s: &str) -> Result<(), Error> {
    // At least two dot-separated segments of [a-z][a-z0-9-]*
    if s.is_empty() || s.len() > 256 {
        return Err(Error::InvalidCapabilityName { raw: s.to_owned() });
    }
    let mut dot_count = 0usize;
    for segment in s.split('.') {
        if !is_valid_kebab_segment(segment) {
            return Err(Error::InvalidCapabilityName { raw: s.to_owned() });
        }
        dot_count += 1;
    }
    if dot_count < 2 {
        return Err(Error::InvalidCapabilityName { raw: s.to_owned() });
    }
    Ok(())
}

fn is_valid_kebab_segment(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_id_valid() {
        assert!(plugin_id("com.example.foo").is_ok());
        assert!(plugin_id("org.itala.search").is_ok());
        assert!(plugin_id("io.github.user.plugin").is_ok());
    }

    #[test]
    fn plugin_id_invalid() {
        assert!(plugin_id("").is_err());
        assert!(plugin_id("foo").is_err()); // single segment
        assert!(plugin_id("foo.").is_err()); // trailing dot → empty segment
        assert!(plugin_id(".foo.bar").is_err()); // leading dot → empty segment
        assert!(plugin_id("Foo.bar").is_err()); // uppercase
        assert!(plugin_id("1foo.bar").is_err()); // leading digit
        assert!(plugin_id("foo.bar baz").is_err()); // space
        assert!(plugin_id(&"a.b".repeat(100)).is_err()); // > 128
    }

    #[test]
    fn block_id_valid() {
        assert!(block_id("intro").is_ok());
        assert!(block_id("my-block-1").is_ok());
        assert!(block_id("UPPER_CASE").is_ok());
        assert!(block_id(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn block_id_invalid() {
        assert!(block_id("").is_err());
        assert!(block_id("has space").is_err());
        assert!(block_id("has/slash").is_err());
        assert!(block_id("has.dot").is_err());
        assert!(block_id(&"a".repeat(65)).is_err());
        assert!(block_id("café").is_err()); // non-ASCII
    }

    #[test]
    fn capability_name_valid() {
        assert!(capability_name("host.vfs.read").is_ok());
        assert!(capability_name("editor.render-block").is_ok());
        assert!(capability_name("host.events.note-saved").is_ok());
    }

    #[test]
    fn capability_name_invalid() {
        assert!(capability_name("").is_err());
        assert!(capability_name("host").is_err()); // single segment
        assert!(capability_name("Host.vfs").is_err()); // uppercase
        assert!(capability_name("host.").is_err()); // trailing dot
    }
}
