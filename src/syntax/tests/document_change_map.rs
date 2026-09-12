use mech_syntax::document::{Affinity, ChangeMap, TextEdit, TextRange, TextSize};

#[test]
fn maps_offsets_and_ranges_across_multiple_edits() {
    let changes = ChangeMap::new(&[
        TextEdit::insert(TextSize(2), "XX"),
        TextEdit::replace(TextRange::new(TextSize(5), TextSize(8)), "Y"),
    ]);
    assert_eq!(
        changes.map_offset(TextSize(2), Affinity::Before),
        TextSize(2)
    );
    assert_eq!(
        changes.map_offset(TextSize(2), Affinity::After),
        TextSize(4)
    );
    assert_eq!(
        changes.map_offset(TextSize(8), Affinity::After),
        TextSize(8)
    );
    assert_eq!(
        changes.map_range(TextRange::new(TextSize(0), TextSize(10))),
        TextRange::new(TextSize(0), TextSize(10))
    );
    assert_eq!(
        changes.new_changed_range(),
        TextRange::new(TextSize(2), TextSize(8))
    );
}

// Deliberately simple reference to test the indexed mapping independently,
// including adjacent replacements and multiple inserts at one boundary.
fn sequential_offset(edits: &[TextEdit], offset: u32, after: bool) -> u32 {
    let mut delta = 0_i64;
    for edit in edits {
        if offset < edit.delete.start.0 || (offset == edit.delete.start.0 && !after) {
            break;
        }
        if offset < edit.delete.end.0 || (offset == edit.delete.end.0 && !after) {
            return (i64::from(edit.delete.start.0)
                + delta
                + if after { edit.insert.len() as i64 } else { 0 }) as u32;
        }
        delta += edit.insert.len() as i64 - i64::from(edit.delete.len().0);
    }
    (i64::from(offset) + delta) as u32
}

proptest::proptest! {
    #[test]
    fn indexed_mapping_matches_ordered_edit_semantics(
        pieces in proptest::collection::vec((0_u8..5, 0_u8..5, 0_u8..5), 0..32),
    ) {
        let mut cursor = 0_u32;
        let edits = pieces.into_iter().map(|(gap, deleted, inserted)| {
            let start = cursor + u32::from(gap);
            cursor = start + u32::from(deleted);
            TextEdit::replace(TextRange::new(TextSize(start), TextSize(cursor)), "x".repeat(inserted as usize))
        }).collect::<Vec<_>>();
        let map = ChangeMap::new(&edits);
        for offset in 0..=cursor + 1 {
            for after in [false, true] {
                proptest::prop_assert_eq!(
                    map.map_offset(TextSize(offset), if after { Affinity::After } else { Affinity::Before }).0,
                    sequential_offset(&edits, offset, after),
                );
            }
        }
    }
}
