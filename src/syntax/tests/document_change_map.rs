use mech_syntax::document::{
  Affinity, ChangeMap, TextEdit, TextRange, TextSize,
};

#[test]
fn maps_offsets_and_ranges_across_multiple_edits() {
  let changes = ChangeMap::new(&[
    TextEdit::insert(TextSize(2), "XX"),
    TextEdit::replace(
      TextRange::new(TextSize(5), TextSize(8)),
      "Y",
    ),
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
