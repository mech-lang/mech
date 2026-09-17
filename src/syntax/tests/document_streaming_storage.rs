//! B2 source storage qualification, separate from parser resumability gates.
use mech_syntax::document::{DocumentId, Revision, TextEdit, TextRange, TextSize, TextSnapshot};

fn empty() -> TextSnapshot {
    TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap()
}

fn verify(source: &TextSnapshot, text: &str) {
    assert_eq!(source.to_contiguous_string(), text);
    let one = TextSnapshot::new(source.document(), source.revision(), text).unwrap();
    assert_eq!(source.line_index(), one.line_index());
    for (offset, byte) in text.bytes().enumerate() {
        assert_eq!(source.byte_at(TextSize(offset as u32)), Some(byte));
    }
    assert_eq!(source.byte_at(source.byte_len()), None);
    for (start, _) in text.char_indices() {
        for (end, _) in text
            .char_indices()
            .chain(core::iter::once((text.len(), '\0')))
        {
            if start <= end {
                assert_eq!(
                    source
                        .text(TextRange::new(TextSize(start as u32), TextSize(end as u32)))
                        .unwrap(),
                    &text[start..end]
                );
            }
        }
    }
}

#[test]
fn every_unicode_split_preserves_old_source_and_line_snapshots() {
    let text = "a\r\nb\rc\ne\u{301}👩\u{200d}💻終\r\n";
    for split in text
        .char_indices()
        .map(|(i, _)| i)
        .chain(core::iter::once(text.len()))
    {
        let before = empty().append(&text[..split]).unwrap();
        let old_index = before.line_index().clone();
        let (after, work) = before.append_with_work(&text[split..]).unwrap();
        verify(&before, &text[..split]);
        verify(&after, text);
        assert_eq!(&old_index, before.line_index());
        assert_eq!(work.accepted_bytes as usize, text.len() - split);
        assert_eq!(work.source_bytes_copied, work.accepted_bytes);
        assert_eq!(work.index_bytes_scanned, work.accepted_bytes);
    }
}

#[test]
fn empty_append_preserves_revision_and_performs_no_storage_work() {
    let old = empty().append("x\r").unwrap();
    let (next, work) = old.append_with_work("").unwrap();
    assert_eq!(next.revision(), old.revision());
    assert_eq!(next.piece_count(), old.piece_count());
    assert_eq!(work, Default::default());
    let edited = old
        .apply_edits(&[TextEdit::insert(old.byte_len(), "")])
        .unwrap();
    assert_eq!(edited.revision(), old.revision());
}

#[test]
fn append_edit_append_rebuilds_a_valid_line_frontier() {
    let old = empty()
        .append("hello\r")
        .unwrap()
        .append("\nworld")
        .unwrap();
    let edited = old
        .apply_edits(&[TextEdit::replace(
            TextRange::new(TextSize(1), TextSize(9)),
            "💡\r",
        )])
        .unwrap();
    let text = edited.to_contiguous_string();
    let (next, work) = edited.append_with_work("\n終").unwrap();
    verify(&next, &(text.clone() + "\n終"));
    verify(&edited, &text);
    verify(&old, "hello\r\nworld");
    assert_eq!(work.index_bytes_scanned, 4);
}

#[test]
fn adjacent_insertions_at_eof_rebuild_the_complete_line_frontier() {
    let old = empty().append("a").unwrap();
    let edited = old
        .apply_edits(&[
            TextEdit::insert(old.byte_len(), "\r"),
            TextEdit::insert(old.byte_len(), "\nnext\n"),
        ])
        .unwrap();
    verify(&edited, "a\r\nnext\n");
    verify(&old, "a");
}

#[test]
fn retained_append_storage_has_bounded_cumulative_growth() {
    let mut previous = None;
    for n in [512usize, 1024, 2048, 4096] {
        let mut source = empty();
        let mut history = Vec::new();
        let mut nodes = 0;
        let mut copied = 0;
        let mut scanned = 0;
        for i in 0..n {
            let (next, work) = source.append_with_work("a\r\n").unwrap();
            let depth_bound = u64::from((i + 1).ilog2()) + 2;
            assert!(work.piece_nodes_allocated <= depth_bound);
            assert!(work.line_nodes_allocated <= depth_bound + 1);
            nodes += work.piece_nodes_allocated + work.line_nodes_allocated;
            copied += work.source_bytes_copied;
            scanned += work.index_bytes_scanned;
            history.push(source);
            source = next;
        }
        assert_eq!(source.byte_len().to_usize(), n * 3);
        assert_eq!(copied, (n * 3) as u64);
        assert_eq!(scanned, copied);
        assert_eq!(source.line_index().line_count(), n + 1);
        assert_eq!(source.to_contiguous_string(), "a\r\n".repeat(n));
        for (i, old) in history.iter().enumerate() {
            assert_eq!(old.byte_len().to_usize(), i * 3);
            assert_eq!(old.line_index().line_count(), i + 1);
            assert_eq!(
                old.line_index().line_start(i),
                Some(TextSize((i * 3) as u32))
            );
        }
        if let Some(previous) = previous {
            assert!(nodes <= previous * 3);
        }
        previous = Some(nodes);
    }
}
