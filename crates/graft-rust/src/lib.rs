//! graft-rust: reads Rust source without a full parser — masks strings and comments,
//! finds items with their body spans, call sites, data-file use and test scopes.

mod calls;
mod mask;
mod scan;

pub use calls::{artifact_name, BodyFacts};
pub use mask::{mask, Literal, Masked};
pub use scan::{scan, Item, ItemKind, Scan};
