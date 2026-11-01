//! Finalize detached scalar inputs without creating mutable execution cells.
use mech_core::{
    MResult, MechError, SchemaBody, SchemaDraft, SchemaTableBuilder, Value,
    snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft},
};

fn snapshot(body: SchemaBody, data: ValueDataDraft) -> MResult<Value> {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()?,
    )?;
    let built = builder.finish()?;
    let schema = built.resolve(handle)?;
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .map_err(|error| {
        MechError::new(mech_core::ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })
}

pub(super) fn unit() -> MResult<Value> {
    snapshot(
        SchemaBody::Tuple(Box::new([])),
        ValueDataDraft::Tuple(Box::new([])),
    )
}

pub(super) fn index(value: usize) -> MResult<Value> {
    snapshot(SchemaBody::Index, ValueDataDraft::Index(value as u64))
}

#[allow(
    unused_macros,
    reason = "minimal runtime profiles enable no optional scalars"
)]
macro_rules! scalar {
    ($feature:literal, $name:ident, $type:ty, $body:expr, $variant:ident, $convert:expr) => {
        #[cfg(feature = $feature)]
        pub(super) fn $name(value: $type) -> MResult<Value> {
            snapshot($body, ValueDataDraft::$variant(($convert)(value)))
        }
    };
}

scalar!("bool", bool, bool, SchemaBody::Bool, Bool, |value| value);
scalar!(
    "u8",
    u8,
    u8,
    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8),
    U8,
    |value| value
);
scalar!(
    "u16",
    u16,
    u16,
    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W16),
    U16,
    |value| value
);
scalar!(
    "u32",
    u32,
    u32,
    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32),
    U32,
    |value| value
);
scalar!(
    "u64",
    u64,
    u64,
    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
    U64,
    |value| value
);
scalar!(
    "u128",
    u128,
    u128,
    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W128),
    U128,
    |value| value
);
scalar!(
    "i8",
    i8,
    i8,
    SchemaBody::SignedInteger(mech_core::IntegerWidth::W8),
    I8,
    |value| value
);
scalar!(
    "i16",
    i16,
    i16,
    SchemaBody::SignedInteger(mech_core::IntegerWidth::W16),
    I16,
    |value| value
);
scalar!(
    "i32",
    i32,
    i32,
    SchemaBody::SignedInteger(mech_core::IntegerWidth::W32),
    I32,
    |value| value
);
scalar!(
    "i64",
    i64,
    i64,
    SchemaBody::SignedInteger(mech_core::IntegerWidth::W64),
    I64,
    |value| value
);
scalar!(
    "i128",
    i128,
    i128,
    SchemaBody::SignedInteger(mech_core::IntegerWidth::W128),
    I128,
    |value| value
);
scalar!(
    "f32",
    f32,
    f32,
    SchemaBody::FloatingPoint(mech_core::FloatWidth::W32),
    F32,
    mech_core::snapshot::F32Bits::from_f32
);
scalar!(
    "f64",
    f64,
    f64,
    SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
    F64,
    mech_core::snapshot::F64Bits::from_f64
);

#[cfg(test)]
#[path = "input_scalar_tests.rs"]
mod tests;
