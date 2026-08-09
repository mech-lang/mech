#![cfg(feature = "serde")]

use mech_core::snapshot::{
    Complex32Bits, Complex64Bits, EnumDraft, F32Bits, F64Bits, MapEntryDraft, NamedValueDraft,
    OptionDraft, ReifiedTypeDraft, TableColumnDraft, ValueDataDraft, ValueDraft,
};
use serde::{Serialize, de::DeserializeOwned};

fn assert_open_serde<T: Serialize + DeserializeOwned>() {}

fn declaration_prefix<'a>(source: &'a str, declaration: &str) -> &'a str {
    let end = source
        .find(declaration)
        .expect("reviewed declaration exists");
    let start = source[..end].rfind("\n\n").map_or(0, |offset| offset + 2);
    &source[start..end]
}

#[test]
fn open_snapshot_inputs_remain_serde_round_trippable() {
    assert_open_serde::<ValueDraft>();
    assert_open_serde::<ValueDataDraft>();
    assert_open_serde::<NamedValueDraft>();
    assert_open_serde::<TableColumnDraft>();
    assert_open_serde::<MapEntryDraft>();
    assert_open_serde::<OptionDraft>();
    assert_open_serde::<EnumDraft>();
    assert_open_serde::<ReifiedTypeDraft>();
    assert_open_serde::<F32Bits>();
    assert_open_serde::<F64Bits>();
    assert_open_serde::<Complex32Bits>();
    assert_open_serde::<Complex64Bits>();
}

#[test]
fn finalized_snapshot_storage_has_no_deserialize_wire_format() {
    let data = include_str!("../src/snapshot/data.rs");
    let validation = include_str!("../src/snapshot/validation.rs");
    let constants = include_str!("../src/snapshot/constants.rs");

    for (source, declaration) in [
        (validation, "pub struct Value"),
        (data, "pub enum ValueData"),
        (data, "pub struct Rational64Value"),
        (data, "pub struct RecordValue"),
        (data, "pub struct MatrixValue"),
        (data, "pub struct TableValue"),
        (data, "pub struct SetValue"),
        (data, "pub struct MapValue"),
        (data, "pub enum ReifiedType"),
        (data, "pub struct ReifiedKind"),
        (constants, "pub struct ConstantHandle"),
        (constants, "pub struct ConstantStore"),
        (constants, "pub struct ConstantEntry"),
    ] {
        assert!(
            !declaration_prefix(source, declaration).contains("Deserialize"),
            "{declaration} must be constructed through validated snapshot APIs"
        );
    }

    assert!(
        !declaration_prefix(data, "pub enum ValueData").contains("Serialize"),
        "the private packed ValueData layout is not a durable wire format"
    );
}
