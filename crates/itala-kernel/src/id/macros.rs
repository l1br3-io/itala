/// Emit a ULID-backed identity newtype with all standard impls and colocated tests.
///
/// # Usage
///
/// ```ignore
/// ulid_id! {
///     /// Doc comment forwarded to the struct.
///     pub struct FooId {
///         prefix: "foo",
///         kind: Foo,
///     }
/// }
/// ```
///
/// The `kind` identifier must be a variant of [`crate::id::IdKind`].
/// The macro emits the struct, all trait impls, and a `#[cfg(test)] mod tests` block.
macro_rules! ulid_id {
    (
        $(#[$attr:meta])*
        pub struct $name:ident {
            prefix: $prefix:literal,
            kind: $kind:ident $(,)?
        }
    ) => {
        $(#[$attr])*
        #[derive(
            Clone,
            Copy,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(::ulid::Ulid);

        impl $name {
            /// Generate a fresh, unique identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(::ulid::Ulid::new())
            }

            /// Return the underlying [`::ulid::Ulid`].
            #[must_use]
            pub const fn as_ulid(&self) -> ::ulid::Ulid {
                self.0
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, concat!($prefix, "_{}"), self.0)
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::error::Error;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                let raw = s
                    .strip_prefix(concat!($prefix, "_"))
                    .ok_or_else(|| $crate::error::Error::InvalidId {
                        kind: $crate::id::IdKind::$kind,
                        raw: s.to_owned(),
                    })?;
                let ulid = raw.parse::<::ulid::Ulid>().map_err(|_| $crate::error::Error::InvalidId {
                    kind: $crate::id::IdKind::$kind,
                    raw: s.to_owned(),
                })?;
                ::std::result::Result::Ok(Self(ulid))
            }
        }

        impl ::std::convert::From<::ulid::Ulid> for $name {
            fn from(u: ::ulid::Ulid) -> Self {
                Self(u)
            }
        }

        impl $crate::id::sealed::Sealed for $name {}

        impl $crate::id::Identity for $name {
            const KIND: $crate::id::IdKind = $crate::id::IdKind::$kind;
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use ::proptest::prelude::*;

            #[test]
            fn display_has_prefix() {
                let id = $name::new();
                assert!(id.to_string().starts_with(concat!($prefix, "_")));
            }

            #[test]
            fn round_trip_fixed() {
                let id = $name::new();
                let s = id.to_string();
                let parsed: $name = s.parse().unwrap();
                assert_eq!(id, parsed);
            }

            #[test]
            fn rejects_wrong_prefix() {
                // Hardcoded wrong prefix avoids using another ID type as sentinel,
                // which would create a circular dependency for the type being defined.
                let wrong = concat!("wrongtype_", "01ARZ3NDEKTSV4RRFFQ69G5FAV");
                let err = wrong.parse::<$name>();
                assert!(err.is_err());
            }

            #[test]
            fn rejects_bare_ulid() {
                let err = ::ulid::Ulid::new().to_string().parse::<$name>();
                assert!(err.is_err());
            }

            proptest! {
                #[test]
                fn proptest_round_trip(bytes in any::<[u8; 16]>()) {
                    let u = ::ulid::Ulid::from(u128::from_be_bytes(bytes));
                    let id = $name::from(u);
                    let s = id.to_string();
                    let parsed: $name = s.parse().unwrap();
                    prop_assert_eq!(id, parsed);
                }
            }
        }
    };
}

pub(crate) use ulid_id;
