#![no_main]

use libfuzzer_sys::fuzz_target;
use mech_syntax::document::{
    DocumentId, DocumentSession, Revision, TextEdit, TextRange, TextSize, TextSnapshot,
    parse_document,
};
use mech_syntax_fuzz::{boundaries, bounded_config, normalized};

const MAX_INITIAL_BYTES: usize = 16 * 1024;
const MAX_EDITS_PER_INPUT: usize = 32;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let split = usize::from(u16::from_le_bytes([data[0], data[1]])) % (data.len() - 1);
    let initial_len = split.min(data.len() - 2).min(MAX_INITIAL_BYTES);
    let initial_bytes = &data[2..2 + initial_len];
    let Ok(initial) = core::str::from_utf8(initial_bytes) else {
        return;
    };
    let config = bounded_config(
        initial_len.saturating_add(MAX_EDITS_PER_INPUT.saturating_mul(15)),
    );
    let mut session = DocumentSession::new(initial, config);
    let mut cursor = 2 + initial_len;

    for _ in 0..MAX_EDITS_PER_INPUT {
        if cursor + 4 > data.len() {
            break;
        }
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
        let update = session.apply_edits(&[TextEdit::replace(
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
        let (incremental_tree, incremental_diagnostics) = normalized(incremental);
        let (full_tree, full_diagnostics) = normalized(&full);
        assert!(
            incremental_tree == full_tree,
            "incremental tree diverged at revision {}, source length {}, edit {}..{} -> {:?}, \
             fallbacks {}, attempted roots {}",
            incremental.revision.0,
            current.len(),
            start,
            end,
            insertion,
            update.stats.document_fallbacks,
            update.stats.attempted_roots,
        );
        if incremental_diagnostics != full_diagnostics {
            let mismatch = incremental_diagnostics
                .iter()
                .zip(&full_diagnostics)
                .position(|(incremental, full)| incremental != full)
                .unwrap_or_else(|| {
                    incremental_diagnostics.len().min(full_diagnostics.len())
                });
            panic!(
                "incremental diagnostics diverged at revision {}, source length {}, \
                 edit {}..{} -> {:?}; counts {} vs {}, first mismatch {}: {:?} vs {:?}",
                incremental.revision.0,
                current.len(),
                start,
                end,
                insertion,
                incremental_diagnostics.len(),
                full_diagnostics.len(),
                mismatch,
                incremental_diagnostics.get(mismatch),
                full_diagnostics.get(mismatch),
            );
        }
    }
});
