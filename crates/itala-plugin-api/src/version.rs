//! The host API version this contract targets.
//!
//! Plugins declare an `api_range` in their manifest that must match this value.
//! Bump the major version for breaking API changes; minor versions stay
//! backwards-compatible.

use itala_kernel::manifest::HOST_API_VERSION;
use semver::Version;

/// Returns the current plugin API version.
///
/// This mirrors [`itala_kernel::manifest::HOST_API_VERSION`] and is the value
/// plugin manifests are checked against at load time.
#[must_use]
pub const fn api_version() -> Version {
    HOST_API_VERSION
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const V: fn() = check::<Version>;
    const _: [(); 0] = [(); {
        let _ = V;
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_is_0_1() {
        assert_eq!(api_version(), Version::new(0, 1, 0));
    }
}
