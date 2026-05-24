use super::ulid_id;

ulid_id! {
    /// Unique identifier for a vault.
    ///
    /// ```
    /// use itala_kernel::VaultId;
    ///
    /// let id = VaultId::new();
    /// let s = id.to_string();
    /// assert!(s.starts_with("vault_"));
    /// let parsed: VaultId = s.parse().unwrap();
    /// assert_eq!(id, parsed);
    /// ```
    pub struct VaultId {
        prefix: "vault",
        kind: Vault,
    }
}
