//! `wiring.bin` is read by other tools (pasli-beyin mirrors `WiringMeta`). bincode is
//! not self-describing, so this pins the exact bytes: a reader that drifts from the
//! field order fails here instead of silently misreading counts.

use graft_model::WiringMeta;

#[test]
fn wiring_meta_bytes_are_pinned() {
    let meta = WiringMeta {
        version: 1,
        node_count: 2,
        edge_count: 3,
        languages: vec!["rust".to_string()],
    };
    let bytes = bincode::serde::encode_to_vec(&meta, bincode::config::standard()).unwrap();
    assert_eq!(bytes, [1, 2, 3, 1, 4, b'r', b'u', b's', b't']);

    let (back, used): (WiringMeta, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(back, meta);
    assert_eq!(used, bytes.len());
}
