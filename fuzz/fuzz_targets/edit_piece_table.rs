#![no_main]

use libfuzzer_sys::fuzz_target;
use mech_syntax::document::{DocumentId, Revision, TextEdit, TextRange, TextSize, TextSnapshot};
use mech_syntax_fuzz::boundaries;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let initial_len = usize::from(data[0]) % data.len();
    let Ok(initial) = core::str::from_utf8(&data[1..initial_len.max(1).min(data.len())]) else {
        return;
    };
    let mut expected = initial.to_string();
    let mut actual = TextSnapshot::new(DocumentId(2), Revision(0), initial).unwrap();
    let mut cursor = initial_len.max(1).min(data.len());

    while cursor + 3 <= data.len() {
        let offsets = boundaries(&expected);
        let first = offsets[usize::from(data[cursor]) % offsets.len()];
        let second = offsets[usize::from(data[cursor + 1]) % offsets.len()];
        let length = usize::from(data[cursor + 2]) % 12;
        cursor += 3;
        let available = length.min(data.len() - cursor);
        let insertion_bytes = &data[cursor..cursor + available];
        cursor += available;
        let Ok(insertion) = core::str::from_utf8(insertion_bytes) else {
            continue;
        };
        let (start, end) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        expected.replace_range(start..end, insertion);
        actual = actual
            .apply_edits(&[TextEdit::replace(
                TextRange::new(TextSize(start as u32), TextSize(end as u32)),
                insertion,
            )])
            .unwrap();
        assert_eq!(actual.to_contiguous_string(), expected);
    }
});
