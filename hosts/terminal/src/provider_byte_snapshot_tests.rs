use super::*;
use mech_core::{DimensionLifetime, ValueCell};

#[test]
fn byte_snapshots_match_mutable_cell_publication_without_changing_schema() {
    for length in [0, 1, 2, 256, 65_537] {
        let bytes = (0..length).map(|index| index as u8).collect::<Vec<_>>();
        let expected = ValueCell::dynamic_matrix(
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            Box::new([1, length as u64]),
            bytes.iter().copied().map(ValueDataDraft::U8).collect(),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let actual = byte_row_value(&bytes, "test://bytes").unwrap();
        assert_eq!(actual.schema_key(), expected.schema_key());
        assert_eq!(actual.shape(), expected.shape());
        assert_eq!(
            actual.canonical_data_draft().unwrap(),
            expected.canonical_data_draft().unwrap()
        );
        assert_eq!(byte_row(&actual, "test://bytes").unwrap(), bytes);
        let schemas = actual.schemas().unwrap();
        actual.validate_against(&schemas).unwrap();
        assert_eq!(
            actual.value_hash(&schemas).unwrap(),
            expected.value_hash(&schemas).unwrap()
        );
        assert_eq!(
            actual.retained_footprint(&schemas).unwrap(),
            expected.retained_footprint(&schemas).unwrap()
        );
        let schema = schemas.get(actual.schema()).unwrap();
        assert_eq!(schema.dimension_parameters().len(), 2);
        assert!(
            schema
                .dimension_parameters()
                .iter()
                .all(|parameter| parameter.lifetime() == DimensionLifetime::Turn)
        );
    }
}
