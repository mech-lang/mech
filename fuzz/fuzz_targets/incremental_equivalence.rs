#![no_main]

use libfuzzer_sys::fuzz_target;
use mech_syntax::document::{
    DocumentId, DocumentSession, Revision, TextEdit, TextRange, TextSize, TextSnapshot,
    parse_document,
};
use mech_syntax_fuzz::{boundaries, bounded_config, normalized};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let split = usize::from(u16::from_le_bytes([data[0], data[1]])) % (data.len() - 1);
    let initial_bytes = &data[2..2 + split.min(data.len() - 2)];
    let Ok(initial) = core::str::from_utf8(initial_bytes) else {
        return;
    };
    let config = bounded_config(data.len().saturating_mul(4));
    let mut session = DocumentSession::new(initial, config);
    let mut cursor = 2 + split.min(data.len() - 2);

    while cursor + 4 <= data.len() {
        let current = session.snapshot().source.to_contiguous_string();
        let offsets = boundaries(&current);
        let start = offsets[usize::from(data[cursor]) % offsets.len()];
        let end = offsets[usize::from(data[cursor + 1]) % offsets.len()];
        let insertion_len = usize::from(data[cursor + 2]) % 16;
        cursor += 3;
        let available = insertion_len.min(data.len() - cursor);
        let insertion_bytes = &data[cursor..cursor + available];
        cursor += available.max(1).min(data.len() - cursor);
        let Ok(insertion) = core::str::from_utf8(insertion_bytes) else {
            continue;
        };
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        session.apply_edits(&[TextEdit::replace(
            TextRange::new(TextSize(start as u32), TextSize(end as u32)),
            insertion,
        )]);

        let incremental = session.snapshot();
        let full = parse_document(
            TextSnapshot::new(
                DocumentId(1),
                Revision(incremental.revision.0),
                incremental.source.to_contiguous_string().as_str(),
            )
            .unwrap(),
            config,
        );
        assert_eq!(normalized(incremental), normalized(&full));
    }
});
