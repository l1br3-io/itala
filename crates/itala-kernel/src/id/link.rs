use super::ulid_id;

ulid_id! {
    /// Unique identifier for a link between notes.
    ///
    /// ```
    /// use itala_kernel::LinkId;
    ///
    /// let id = LinkId::new();
    /// let s = id.to_string();
    /// assert!(s.starts_with("link_"));
    /// let parsed: LinkId = s.parse().unwrap();
    /// assert_eq!(id, parsed);
    /// ```
    pub struct LinkId {
        prefix: "link",
        kind: Link,
    }
}
