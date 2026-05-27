//! Pinned `comrak` options for downstream Markdown consumers.
//!
//! F4 does not parse Markdown; this module exists so later sprints (graph,
//! editor render) all share one canonical options builder and don't drift.

use comrak::Options;

/// Default GFM-aligned Markdown options with `unsafe_` rendering disabled.
#[must_use]
pub fn default_options() -> Options<'static> {
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.autolink = true;
    opts.extension.tasklist = true;
    opts.extension.strikethrough = true;
    opts.extension.footnotes = true;
    opts.render.r#unsafe = false;
    opts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_gfm_extensions_and_safe_render() {
        let opts = default_options();
        assert!(opts.extension.table);
        assert!(opts.extension.autolink);
        assert!(opts.extension.tasklist);
        assert!(opts.extension.strikethrough);
        assert!(opts.extension.footnotes);
        assert!(!opts.render.r#unsafe);
    }
}
