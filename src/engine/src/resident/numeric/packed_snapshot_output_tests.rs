//! Exact old/new numeric leaf comparisons, real typed selection and denial.
use super::*;
use mech_core::{
    DimensionExpr as D, FloatWidth as F, IntegerWidth as W, SchemaDraft, SchemaTable,
    SchemaTableBuilder, Value,
};

struct Inputs<'a>(&'a [ResidentValueRef<'a>]);
impl ResidentKernelInputs for Inputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.0.get(index).copied()
    }
}
fn matrix(element: &SchemaBody, rows: usize, columns: usize) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element.clone()),
        dimensions: vec![D::Constant(rows as u64), D::Constant(columns as u64)].into_boxed_slice(),
    }
}
fn table(bodies: impl IntoIterator<Item = SchemaBody>) -> (SchemaTable, Vec<SchemaId>) {
    let mut b = SchemaTableBuilder::new();
    let handles = bodies
        .into_iter()
        .map(|body| {
            b.insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let built = b.finish().unwrap();
    let ids = handles
        .into_iter()
        .map(|h| built.resolve(h).unwrap())
        .collect();
    (built.into_parts().0, ids)
}
fn value(schemas: &SchemaTable, id: SchemaId, cells: Vec<ValueDataDraft>) -> Value {
    ValueDraft {
        schema: id,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(cells.into_boxed_slice()),
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .unwrap()
}
fn kernel(
    executor: mech_core::ResidentKernelExecutor,
    schemas: &SchemaTable,
    id: SchemaId,
    parameters: Vec<u64>,
) -> BoundResidentKernel {
    BoundResidentKernel::new(executor, parameters.into_boxed_slice())
        .with_snapshot_schemas(schemas.clone())
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: id,
            schema_key: schemas.entry(id).unwrap().key(),
            shape: schemas
                .get(id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
}
fn data(values: &[Option<Value>]) -> Vec<Option<ValueDataDraft>> {
    values
        .iter()
        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
        .collect()
}
fn parity(bound: &BoundResidentKernel, arguments: &[ResidentValueRef<'_>], expect_packed: bool) {
    let mut fast = [None];
    let mut old = [None];
    for _ in 0..2 {
        packed_snapshot_output::constructions();
        let a = bound.execute(&Inputs(arguments), ResidentValueMut::Snapshot(&mut fast));
        let constructions = packed_snapshot_output::constructions();
        let b = packed_snapshot_output::without(|| {
            bound.execute(&Inputs(arguments), ResidentValueMut::Snapshot(&mut old))
        });
        assert_eq!(a, b);
        assert_eq!(data(&fast), data(&old));
        if a.is_ok() {
            assert_eq!(
                constructions > 0,
                expect_packed,
                "actual constructor selection"
            );
        }
    }
}
fn cases() -> Vec<(SchemaBody, Vec<ValueDataDraft>)> {
    macro_rules! ints {
        ($body:expr,$variant:ident) => {
            (
                $body,
                (1..=6).map(|n| ValueDataDraft::$variant(n as _)).collect(),
            )
        };
    }
    vec![
        (
            SchemaBody::Bool,
            [false, true, true, false, true, false]
                .map(ValueDataDraft::Bool)
                .into(),
        ),
        ints!(SchemaBody::Index, Index),
        ints!(SchemaBody::UnsignedInteger(W::W8), U8),
        ints!(SchemaBody::UnsignedInteger(W::W16), U16),
        ints!(SchemaBody::UnsignedInteger(W::W32), U32),
        ints!(SchemaBody::UnsignedInteger(W::W64), U64),
        ints!(SchemaBody::UnsignedInteger(W::W128), U128),
        ints!(SchemaBody::SignedInteger(W::W8), I8),
        ints!(SchemaBody::SignedInteger(W::W16), I16),
        ints!(SchemaBody::SignedInteger(W::W32), I32),
        ints!(SchemaBody::SignedInteger(W::W64), I64),
        ints!(SchemaBody::SignedInteger(W::W128), I128),
        (
            SchemaBody::FloatingPoint(F::W32),
            [
                0x80000000, 0x7fc00037, 0x7f800000, 0xff800000, 0, 0x3fc00000,
            ]
            .map(|b| ValueDataDraft::F32(F32Bits::from_bits(b)))
            .into(),
        ),
        (
            SchemaBody::FloatingPoint(F::W64),
            [
                0x8000000000000000,
                0x7ff8000000000037,
                0x7ff0000000000000,
                0xfff0000000000000,
                0,
                0x3ff8000000000000,
            ]
            .map(|b| ValueDataDraft::F64(F64Bits::from_bits(b)))
            .into(),
        ),
    ]
}

#[test]
fn packed_snapshot_transpose_keeps_fourteen_kinds_exact_bits_and_empty_axes() {
    for (element, cells) in cases() {
        for (rows, columns) in [(2, 3), (0, 3), (2, 0), (0, 0)] {
            let (schemas, ids) = table([
                matrix(&element, rows, columns),
                matrix(&element, columns, rows),
            ]);
            let cells = if rows * columns == 0 {
                vec![]
            } else {
                cells.clone()
            };
            let input = [Some(value(&schemas, ids[0], cells.clone()))];
            let before = data(&input);
            let bound = kernel(transpose_snapshot, &schemas, ids[1], vec![]);
            parity(&bound, &[ResidentValueRef::Snapshot(&input)], true);
            let mut out = [None];
            bound
                .execute(
                    &Inputs(&[ResidentValueRef::Snapshot(&input)]),
                    ResidentValueMut::Snapshot(&mut out),
                )
                .unwrap();
            let expected = (0..columns)
                .flat_map(|column| (0..rows).map(move |row| row * columns + column))
                .map(|at| cells[at].clone())
                .collect::<Vec<_>>();
            assert_eq!(
                data(&out),
                vec![Some(ValueDataDraft::Matrix(expected.into_boxed_slice()))]
            );
            assert_eq!(data(&input), before);
        }
    }
}

#[test]
fn packed_snapshot_sums_keep_typed_zero_order_overflow_and_old_publications() {
    for (element, cells) in cases()
        .into_iter()
        .filter(|(e, _)| !matches!(e, SchemaBody::Bool | SchemaBody::Index))
    {
        for (rows, columns) in [(2, 3), (0, 3), (2, 0), (0, 0)] {
            for column in [false, true] {
                let out_shape = if column { (rows, 1) } else { (1, columns) };
                let (schemas, ids) = table([
                    matrix(&element, rows, columns),
                    matrix(&element, out_shape.0, out_shape.1),
                ]);
                let input = [Some(value(
                    &schemas,
                    ids[0],
                    if rows * columns == 0 {
                        vec![]
                    } else {
                        cells.clone()
                    },
                ))];
                let bound = kernel(sum_snapshot, &schemas, ids[1], vec![u64::from(column), 1]);
                parity(&bound, &[ResidentValueRef::Snapshot(&input)], true);
            }
        }
    }
    let element = SchemaBody::UnsignedInteger(W::W8);
    let (schemas, ids) = table([matrix(&element, 1, 2), matrix(&element, 1, 1)]);
    let good = [Some(value(
        &schemas,
        ids[0],
        vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)],
    ))];
    let bad = [Some(value(
        &schemas,
        ids[0],
        vec![ValueDataDraft::U8(255), ValueDataDraft::U8(1)],
    ))];
    let bound = kernel(sum_snapshot, &schemas, ids[1], vec![1, 1]);
    let mut out = [None];
    bound
        .execute(
            &Inputs(&[ResidentValueRef::Snapshot(&good)]),
            ResidentValueMut::Snapshot(&mut out),
        )
        .unwrap();
    let before = data(&out);
    assert_eq!(
        bound.execute(
            &Inputs(&[ResidentValueRef::Snapshot(&bad)]),
            ResidentValueMut::Snapshot(&mut out)
        ),
        Err(ResidentKernelError::Arithmetic)
    );
    assert_eq!(data(&out), before);
    parity(&bound, &[ResidentValueRef::Snapshot(&bad)], true);
    bound
        .execute(
            &Inputs(&[ResidentValueRef::Snapshot(&good)]),
            ResidentValueMut::Snapshot(&mut out),
        )
        .unwrap();
}

#[test]
fn packed_snapshot_broadcast_products_unary_and_finite_denials_match_drafts() {
    let element = SchemaBody::FloatingPoint(F::W64);
    let f64s = |xs: &[f64]| {
        xs.iter()
            .copied()
            .map(|x| ValueDataDraft::F64(F64Bits::from_f64(x)))
            .collect::<Vec<_>>()
    };
    let (schemas, ids) = table([
        matrix(&element, 2, 1),
        matrix(&element, 1, 3),
        matrix(&element, 2, 3),
        matrix(&element, 3, 2),
        matrix(&element, 2, 2),
    ]);
    let left = [Some(value(&schemas, ids[0], f64s(&[0.0, -1.0])))];
    let right = [Some(value(&schemas, ids[1], f64s(&[2.0, 3.0, 0.5])))];
    for operation in [
        SemanticArithmetic::Add,
        SemanticArithmetic::Subtract,
        SemanticArithmetic::Multiply,
        SemanticArithmetic::Divide,
        SemanticArithmetic::Remainder,
        SemanticArithmetic::Power,
    ] {
        let bound = kernel(
            snapshot_numeric_binary,
            &schemas,
            ids[2],
            vec![operation as u64, 0, 1, 1, 1, 2, 1, 1, 3],
        );
        parity(
            &bound,
            &[
                ResidentValueRef::Snapshot(&left),
                ResidentValueRef::Snapshot(&right),
            ],
            true,
        );
    }
    let lhs = [Some(value(
        &schemas,
        ids[2],
        f64s(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
    ))];
    let rhs = [Some(value(
        &schemas,
        ids[3],
        f64s(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
    ))];
    let matmul = kernel(matrix_multiply_snapshot, &schemas, ids[4], vec![]);
    parity(
        &matmul,
        &[
            ResidentValueRef::Snapshot(&lhs),
            ResidentValueRef::Snapshot(&rhs),
        ],
        true,
    );
    let mut product = [None];
    matmul
        .execute(
            &Inputs(&[
                ResidentValueRef::Snapshot(&lhs),
                ResidentValueRef::Snapshot(&rhs),
            ]),
            ResidentValueMut::Snapshot(&mut product),
        )
        .unwrap();
    assert_eq!(
        data(&product),
        vec![Some(ValueDataDraft::Matrix(
            f64s(&[22.0, 28.0, 49.0, 64.0]).into_boxed_slice()
        ))]
    );
    for executor in [
        square_root_f32 as mech_core::ResidentKernelExecutor,
        cosine_f32 as mech_core::ResidentKernelExecutor,
        absolute_f32 as mech_core::ResidentKernelExecutor,
    ] {
        parity(
            &kernel(executor, &schemas, ids[2], vec![]),
            &[ResidentValueRef::Snapshot(&lhs)],
            true,
        );
    }
    let unary = kernel(square_root_f32, &schemas, ids[2], vec![]);
    let args = [ResidentValueRef::Snapshot(&lhs)];
    for dimension in 0..7 {
        for cap in [0, 1, 16, 128, 4096, 65535] {
            let mut plan =
                crate::resident::budget::detached_turn_plan(mech_core::ResourceDemand::default())
                    .unwrap();
            match dimension {
                0 => plan.budget_limits.max_output_elements = Some(cap),
                1 => plan.budget_limits.max_output_bytes = Some(cap),
                2 => plan.budget_limits.max_temporary_bytes = Some(cap),
                3 => plan.budget_limits.max_cloned_bytes = Some(cap),
                4 => plan.budget_limits.max_retained_nodes = Some(cap),
                5 => plan.budget_limits.max_compute_work = Some(cap),
                _ => plan.budget_limits.max_comparison_work = Some(cap),
            }
            let mut fast = product.clone();
            let mut old = fast.clone();
            let before = data(&fast);
            let a = crate::resident::budget::with_resident_turn_plan(plan.clone(), || {
                unary.execute(&Inputs(&args), ResidentValueMut::Snapshot(&mut fast))
            });
            let b = packed_snapshot_output::without(|| {
                crate::resident::budget::with_resident_turn_plan(plan, || {
                    unary.execute(&Inputs(&args), ResidentValueMut::Snapshot(&mut old))
                })
            });
            assert_eq!(a, b, "dimension {dimension} cap {cap}");
            assert_eq!(data(&fast), data(&old));
            if a.is_err() {
                assert_eq!(data(&fast), before);
            }
        }
    }
}

#[test]
fn packed_snapshot_rejections_and_constrained_fallbacks_keep_canonical_validation() {
    for (element, cells) in [
        (
            SchemaBody::String,
            vec![
                ValueDataDraft::String("a".into()),
                ValueDataDraft::String("b".into()),
            ],
        ),
        (
            SchemaBody::IntegerInterval(mech_core::IntegerInterval::Unsigned {
                width: W::W8,
                lower: 1,
                upper: 10,
                upper_inclusive: true,
            }),
            vec![ValueDataDraft::U8(2), ValueDataDraft::U8(3)],
        ),
        (
            SchemaBody::Complex(F::W32),
            vec![complex32_to_draft(0.0, -0.0), complex32_to_draft(-0.0, 0.0)],
        ),
    ] {
        let (schemas, ids) = table([matrix(&element, 1, 2), matrix(&element, 2, 1)]);
        let input = [Some(value(&schemas, ids[0], cells))];
        parity(
            &kernel(transpose_snapshot, &schemas, ids[1], vec![]),
            &[ResidentValueRef::Snapshot(&input)],
            false,
        );
    }
    let u8body = SchemaBody::UnsignedInteger(W::W8);
    let u16body = SchemaBody::UnsignedInteger(W::W16);
    let (schemas, ids) = table([
        matrix(&u8body, 1, 2),
        matrix(&u16body, 2, 1),
        matrix(&u8body, 1, 1),
    ]);
    let input = [Some(value(
        &schemas,
        ids[0],
        vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)],
    ))];
    for output_id in [ids[1], ids[2]] {
        let bound = kernel(transpose_snapshot, &schemas, output_id, vec![]);
        let mut fast = [None];
        let mut old = [None];
        let a = bound.execute(
            &Inputs(&[ResidentValueRef::Snapshot(&input)]),
            ResidentValueMut::Snapshot(&mut fast),
        );
        let b = packed_snapshot_output::without(|| {
            bound.execute(
                &Inputs(&[ResidentValueRef::Snapshot(&input)]),
                ResidentValueMut::Snapshot(&mut old),
            )
        });
        let expected = if output_id == ids[1] {
            ResidentKernelError::InvalidOutput
        } else {
            ResidentKernelError::InvalidShape
        };
        assert_eq!(a, Err(expected));
        assert_eq!(a, b);
        assert!(fast[0].is_none() && old[0].is_none());
    }
}
