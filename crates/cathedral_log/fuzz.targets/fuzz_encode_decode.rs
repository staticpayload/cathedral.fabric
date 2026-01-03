#![no_main]
use libfuzzer_sys::fuzz_target;
use cathedral_log::encoding::{CanonicalEncode, CanonicalDecode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FuzzStruct {
    a: u64,
    b: Vec<u8>,
    c: Option<String>,
}

impl CanonicalEncode for FuzzStruct {}

fuzz_target!(|data: &[u8]| {
    // Try to decode as FuzzStruct - should not crash
    if let Ok(decoded) = FuzzStruct::decode(data) {
        // If we can decode, we should be able to re-encode
        let encoded = decoded.encode();
        // And roundtrip should work
        if let Ok(roundtrip) = FuzzStruct::decode(&encoded) {
            assert_eq!(decoded, roundtrip);
        }
    }
});
