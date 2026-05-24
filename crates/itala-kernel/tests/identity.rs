#![allow(missing_docs)]

use itala_kernel::{BlockId, Identity, LinkId, NoteId, PluginId, VaultId};

fn round_trip<I: Identity + PartialEq + std::fmt::Debug>(id: &I) {
    let s = id.to_string();
    let parsed: I = s.parse().expect("round-trip");
    assert_eq!(*id, parsed);
}

#[test]
fn all_identities_round_trip() {
    round_trip(&NoteId::new());
    round_trip(&VaultId::new());
    round_trip(&LinkId::new());
    round_trip(&BlockId::new("hello").unwrap());
    round_trip(&PluginId::new("com.example.plug").unwrap());
}

#[test]
fn kind_const_is_distinct_per_id() {
    use itala_kernel::IdKind;
    assert_eq!(NoteId::KIND, IdKind::Note);
    assert_eq!(VaultId::KIND, IdKind::Vault);
    assert_eq!(LinkId::KIND, IdKind::Link);
    assert_eq!(BlockId::KIND, IdKind::Block);
    assert_eq!(PluginId::KIND, IdKind::Plugin);
}
