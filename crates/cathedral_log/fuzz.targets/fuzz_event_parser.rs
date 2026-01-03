#![no_main]
use libfuzzer_sys::fuzz_target;
use cathedral_log::encoding::CanonicalDecode;
use cathedral_log::event::Event;

fuzz_target!(|data: &[u8]| {
    // Try to decode as Event - should not crash
    if let Ok(_event) = Event::decode(data) {
        // Successfully decoded an event
    }
});
