//! CATHEDRAL.FABRIC Event Log
//!
//! Canonical encoding, hash-chained, append-only event structures.
//! All events are deterministically encoded for cross-platform reproducibility.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod event;
pub mod encoding;
pub mod chain;
pub mod stream;
pub mod cursor;

pub use event::{Event, EventKind};
pub use encoding::{CanonicalEncode, CanonicalDecode};
pub use chain::{HashChain, ChainError, ChainValidator};
pub use stream::{EventStream, StreamWriter, StreamError};
pub use cursor::{Cursor, Direction};

#[cfg(test)]
mod tests {
    use super::*;
    use cathedral_core::Hash;

    #[test]
    fn test_hash_chain_basic() {
        let mut chain = HashChain::new();
        let h1 = Hash::compute(b"event1");
        let h2 = Hash::compute(b"event2");

        chain.push(h1);
        assert!(chain.validate());

        chain.push(h2);
        assert!(chain.validate());
    }
}
