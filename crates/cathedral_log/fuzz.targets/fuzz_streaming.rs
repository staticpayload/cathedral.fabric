#![no_main]
use libfuzzer_sys::fuzz_target;
use cathedral_log::encoding::{CanonicalEncoder, CanonicalDecode};
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // Try to use the streaming decoder on this input
    let mut cursor = Cursor::new(data);
    let mut decoder = CanonicalDecoder::new(&mut cursor);

    // Try to decode multiple values - should not crash
    let count = 0;
    while let Ok(Some::<Vec<u8>>(_)) = decoder.decode() {
        // Successfully decoded a value
        let count = count + 1;
        if count > 100 {
            // Limit iterations to avoid infinite loops
            break;
        }
    }
});
