//! Prepared measurements for native frame fields. These are immutable schema
//! facts and exact logical debits, never admission, allocation or visit permits.
use super::{
    BORROWED_FOOTPRINT_METADATA_WORK, BorrowedFootprintSchemaProof, ResidentBudgetMeter,
    borrowed_footprint_schema_supported, measure_native_scalar_frame_footprint,
    projected_canonical_value_footprint, shallow_borrowed_footprint_schema_proof,
};
use crate::native_artifact::{NativeScalarKind as Kind, NativeScalarValue as Scalar};
use mech_core::{
    Schema, SchemaBody, SchemaTable, Value, ValueData,
    snapshot::{SequenceView, ValueFootprint, visit_canonical_data_work},
};

#[derive(Debug)]
pub(crate) struct PreparedNativeFrameFootprint {
    proof: BorrowedFootprintSchemaProof,
    layout: Layout,
    work: u64,
}

#[derive(Debug)]
enum Layout {
    Scalar {
        kind: Kind,
        footprint: ValueFootprint,
    },
    Packed(Kind),
}

impl PreparedNativeFrameFootprint {
    pub(crate) fn derive(schema: &Schema) -> Option<Self> {
        let proof = shallow_borrowed_footprint_schema_proof(schema)?;
        let mut meter = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        let layout = if let Some(kind) = Kind::from_schema_body(schema.body()) {
            if !schema.dimension_parameters().is_empty() {
                return None;
            }
            // Use the maintained metered scalar measurement ONCE. Every valid
            // value of this closed scalar type has this exact logical footprint.
            let footprint = measure_native_scalar_frame_footprint(
                &mut meter,
                schema.body(),
                Scalar::from_bits(kind, 0)?,
            )
            .ok()?;
            Layout::Scalar { kind, footprint }
        } else if let SchemaBody::Matrix { element, .. } = schema.body() {
            let kind = Kind::from_schema_body(element)?;
            meter
                .charge_compute_work(BORROWED_FOOTPRINT_METADATA_WORK)
                .ok()?;
            if !borrowed_footprint_schema_supported(&mut meter, schema.body(), 0).ok()? {
                return None;
            }
            Layout::Packed(kind)
        } else {
            return None;
        };
        Some(Self {
            proof,
            layout,
            work: meter.accumulated.compute_work(),
        })
    }

    pub(crate) fn word(&self, bits: u64) -> Option<(ValueFootprint, u64)> {
        let Layout::Scalar { kind, footprint } = self.layout else {
            return None;
        };
        kind.accepts_bits(bits).then_some((footprint, self.work))
    }

    /// The selected call separately checks exact account membership and input
    /// position. This recipe uses the current table entry's cached semantic key;
    /// it never hashes a Schema or retains the current Value.
    pub(crate) fn value(
        &self,
        value: &Value,
        schemas: &SchemaTable,
    ) -> Option<(ValueFootprint, u64)> {
        let entry = schemas.entry(value.schema())?;
        if !self.proof.supported
            || entry.key() != self.proof.key
            || value.schema_key() != self.proof.key
        {
            return None;
        }
        let kind = match self.layout {
            Layout::Scalar { footprint, .. } => {
                return value
                    .shape()
                    .parameter_values()
                    .is_empty()
                    .then_some((footprint, self.work));
            }
            Layout::Packed(kind) => kind,
        };
        let ValueData::Matrix(matrix) = value.data() else {
            return None;
        };
        if !matches!(
            (kind, matrix.elements()),
            (Kind::Bool, SequenceView::Bool(_))
                | (Kind::U8, SequenceView::U8(_))
                | (Kind::U16, SequenceView::U16(_))
                | (Kind::U32, SequenceView::U32(_))
                | (Kind::U64, SequenceView::U64(_))
                | (Kind::I64, SequenceView::I64(_))
                | (Kind::F64, SequenceView::F64(_))
        ) {
            return None;
        }
        let mut footprint = ValueFootprint::zero();
        let mut work = self.work;
        // Core reports actual retained capacity and logical encoded length.
        // The checked storage kind bounds this to two DATA metadata chunks.
        visit_canonical_data_work(entry.schema().body(), value.data(), |chunk| {
            work = work
                .checked_add(
                    BORROWED_FOOTPRINT_METADATA_WORK
                        .checked_mul(chunk.node_count.max(1))
                        .ok_or(())?,
                )
                .ok_or(())?;
            footprint = footprint
                .checked_add(ValueFootprint {
                    encoded_bytes: chunk.encoded_bytes,
                    retained_bytes: chunk.retained_bytes,
                    node_count: chunk.node_count,
                })
                .map_err(|_| ())?;
            Ok::<_, ()>(())
        })
        .ok()?;
        Some((
            projected_canonical_value_footprint(footprint, value.shape().parameter_values().len())
                .ok()?,
            work,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resident::budget::measure_borrowed_canonical_value_footprint;
    use mech_core::{
        DimensionExpr as D, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, FloatWidth, IntegerWidth as W, SchemaDraft, SchemaTableBuilder,
        snapshot::UnsignedMatrixBuffer,
    };
    use std::sync::Arc;

    #[test]
    fn prepared_frame_scalar_footprints_keep_exact_bits_and_original_debits() {
        for (body, bits) in [
            (SchemaBody::Bool, vec![0, 1, 2]),
            (SchemaBody::UnsignedInteger(W::W8), vec![0, 255, 256]),
            (SchemaBody::UnsignedInteger(W::W16), vec![0, 65535, 65536]),
            (
                SchemaBody::UnsignedInteger(W::W32),
                vec![0, u32::MAX as u64, 1u64 << 32],
            ),
            (SchemaBody::UnsignedInteger(W::W64), vec![0, u64::MAX]),
            (
                SchemaBody::SignedInteger(W::W64),
                vec![0, i64::MIN as u64, i64::MAX as u64],
            ),
            (
                SchemaBody::FloatingPoint(FloatWidth::W64),
                vec![0, (-0.0f64).to_bits(), 0x7ff8_0000_0000_1234],
            ),
        ] {
            let schema = SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap();
            let recipe = PreparedNativeFrameFootprint::derive(&schema).unwrap();
            let kind = Kind::from_schema_body(schema.body()).unwrap();
            for bits in bits {
                let Some(value) = Scalar::from_bits(kind, bits) else {
                    assert_eq!(recipe.word(bits), None);
                    continue;
                };
                assert_eq!(value.to_bits(), bits);
                let mut ordinary = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
                let footprint =
                    measure_native_scalar_frame_footprint(&mut ordinary, schema.body(), value)
                        .unwrap();
                assert_eq!(
                    recipe.word(bits),
                    Some((footprint, ordinary.accumulated.compute_work()))
                );
                assert_eq!(footprint.node_count, ordinary.accumulated.retained_nodes());
            }
        }
    }

    #[test]
    fn prepared_frame_packed_footprints_use_fresh_rectangles_and_real_capacity() {
        macro_rules! check {
            ($width:ident, $variant:ident, $real:expr) => {{
                let mut builder = SchemaTableBuilder::new();
                let handle = builder
                    .insert(
                        SchemaDraft {
                            body: SchemaBody::Matrix {
                                element: Box::new(SchemaBody::UnsignedInteger(W::$width)),
                                dimensions: [
                                    D::Parameter(DimensionParameterId::new(0)),
                                    D::Parameter(DimensionParameterId::new(1)),
                                ]
                                .into(),
                            },
                            dimension_parameters: (0..2)
                                .map(|n| DimensionParameterDeclaration {
                                    id: DimensionParameterId::new(n),
                                    origin: DimensionParameterOrigin::Explicit,
                                    lifetime: DimensionLifetime::Turn,
                                    lower_bound: D::Constant(0),
                                    upper_bound: None,
                                })
                                .collect(),
                        }
                        .finalize()
                        .unwrap(),
                    )
                    .unwrap();
                let built = builder.finish().unwrap();
                let id = built.resolve(handle).unwrap();
                let table = Arc::new(built.table);
                let recipe = PreparedNativeFrameFootprint::derive(table.get(id).unwrap()).unwrap();
                assert_eq!(recipe.word(0), None);
                for (rows, columns, capacity) in [
                    (0u64, 7u64, 17usize),
                    (2, 3, 8),
                    (3, 2, 8),
                    (1, 0, $real),
                    (1, 8, $real),
                ] {
                    let count = (rows * columns) as usize;
                    let mut words = Vec::with_capacity(capacity);
                    words.resize(count, 7);
                    let value = Value::from_unsigned_matrix_buffer(
                        id,
                        table
                            .get(id)
                            .unwrap()
                            .instantiate_shape([rows, columns].into())
                            .unwrap(),
                        UnsignedMatrixBuffer::$variant(words),
                        table.clone(),
                    )
                    .unwrap();
                    let alias = value.clone();
                    let mut ordinary = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
                    let footprint =
                        measure_borrowed_canonical_value_footprint(&mut ordinary, &value, &table)
                            .unwrap();
                    assert_eq!(
                        recipe.value(&value, &table),
                        Some((footprint, ordinary.accumulated.compute_work()))
                    );
                    assert_eq!(recipe.value(&alias, &table), recipe.value(&value, &table));
                    assert_eq!(footprint.node_count, ordinary.accumulated.retained_nodes());
                    assert!(footprint.retained_bytes >= capacity as u64 * (W::$width as u64 / 8));
                    assert!(value.shares_frozen_storage(&alias));
                }
            }};
        }
        check!(W8, U8, 262144);
        check!(W16, U16, 262144);
        check!(W32, U32, 1500513);
        check!(W64, U64, 305257);
    }

    #[test]
    fn prepared_frame_footprint_keeps_unsupported_layouts_on_the_ordinary_path() {
        for body in [
            SchemaBody::String,
            SchemaBody::Tuple(vec![SchemaBody::Bool].into()),
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::String),
                dimensions: [D::Constant(1), D::Constant(0)].into(),
            },
        ] {
            let schema = SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap();
            assert!(PreparedNativeFrameFootprint::derive(&schema).is_none());
        }
    }
}
