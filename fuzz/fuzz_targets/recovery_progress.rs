#![no_main]

use libfuzzer_sys::fuzz_target;
use mech_syntax_fuzz::{assert_snapshot, parse_bounded};

fuzz_target!(|data: &[u8]| {
    let Ok(source) = core::str::from_utf8(data) else {
        return;
    };
    let snapshot = parse_bounded(source);
    assert_snapshot(source, &snapshot);
    assert!(snapshot.stats.source_bytes == source.len() as u64);
    if !source.is_empty() {
        assert!(snapshot.root.text_len.0 > 0);
    }
});
