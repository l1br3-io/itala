use super::ulid_id;

ulid_id! {
    /// Unique identifier for a note.
    ///
    /// Wraps a [`ulid::Ulid`] for monotonic sortability and global uniqueness. Formatted
    /// with the `note_` prefix so IDs are self-describing in logs and on disk.
    ///
    /// ```
    /// use itala_kernel::NoteId;
    ///
    /// let id = NoteId::new();
    /// let s = id.to_string();
    /// assert!(s.starts_with("note_"));
    /// let parsed: NoteId = s.parse().unwrap();
    /// assert_eq!(id, parsed);
    /// ```
    pub struct NoteId {
        prefix: "note",
        kind: Note,
    }
}
