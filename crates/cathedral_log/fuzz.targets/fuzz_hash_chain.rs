#![no_main]
use libfuzzer_sys::fuzz_target;
use cathedral_core::Hash;
use cathedral_log::chain::HashChain;

fuzz_target!(|data: &[u8]| {
    // Create a hash from the input
    let hash = Hash::compute(data);

    // Test that we can create a chain with this hash
    let mut chain = HashChain::new();
    chain.set_expected(hash);
    let _ = chain.push(hash);

    // Validate should not crash
    let _ = chain.validate();
    let _ = chain.root();
});
