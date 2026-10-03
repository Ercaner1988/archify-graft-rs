//! `wiring.bin` is read by other tools (pasli-beyin mirrors `WiringMeta`). The layout is
//! hand-specified and field order is the format, so this pins the exact bytes: a reader
//! that drifts from the field order fails here instead of silently misreading counts.
//! The bytes are the ones bincode 2 `standard()` used to write, so existing readers keep
//! working unchanged.

use graft_model::WiringMeta;

#[test]
fn wiring_meta_bytes_are_pinned() {
    let meta = WiringMeta {
        version: 1,
        node_count: 2,
        edge_count: 3,
        languages: vec!["rust".to_string()],
    };
    let bytes = meta.to_bytes();
    assert_eq!(bytes, [1, 2, 3, 1, 4, b'r', b'u', b's', b't']);
    assert_eq!(WiringMeta::from_bytes(&bytes), Some(meta));
}
