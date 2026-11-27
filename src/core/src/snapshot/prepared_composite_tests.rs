use super::*;
use crate::{
    DimensionExpr as D, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId as Id, DimensionParameterOrigin, FloatWidth, SchemaBody as B, SchemaDraft,
    SchemaField, SchemaId, SchemaTable, SchemaTableBuilder, ShapeInstance,
};
use std::{cell::Cell, sync::Arc};

fn p(n: u32) -> D {
    D::Parameter(Id::new(n))
}
fn params(n: u32) -> Box<[DimensionParameterDeclaration]> {
    (0..n)
        .map(|n| DimensionParameterDeclaration {
            id: Id::new(n),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: D::Constant(0),
            upper_bound: None,
        })
        .collect()
}
fn matrix(axes: Vec<D>) -> B {
    B::Matrix {
        element: Box::new(B::FloatingPoint(FloatWidth::W64)),
        dimensions: axes.into(),
    }
}
fn draft(body: B, n: u32) -> SchemaDraft {
    SchemaDraft {
        body,
        dimension_parameters: params(n),
    }
}
fn arena(drafts: Vec<SchemaDraft>) -> (Arc<SchemaTable>, Vec<SchemaId>) {
    let mut b = SchemaTableBuilder::new();
    let hs: Vec<_> = drafts
        .into_iter()
        .map(|d| b.insert(d.finalize().unwrap()).unwrap())
        .collect();
    let built = b.finish().unwrap();
    let ids = hs.into_iter().map(|h| built.resolve(h).unwrap()).collect();
    (Arc::new(built.table), ids)
}
fn shape(s: &SchemaTable, id: SchemaId, values: &[u64]) -> ShapeInstance {
    s.get(id).unwrap().instantiate_shape(values.into()).unwrap()
}
fn ordinary(
    s: &Arc<SchemaTable>,
    output: SchemaId,
    children: &[(SchemaId, ShapeInstance)],
) -> Result<CompositeSnapshotConstructor, SnapshotValueError> {
    let shape = CompositeSnapshotConstructor::shape_for_children(output, children, s)?;
    CompositeSnapshotConstructor::bind(output, shape, children, Arc::clone(s))
}

#[test]
fn prepared_composite_fresh_rectangles_match_ordinary_with_independent_arenas_and_zero_axes() {
    for record in [false, true] {
        let first = matrix(vec![p(0), p(1)]);
        let second = matrix(vec![p(1), p(0)]);
        let body = if record {
            B::Record(
                vec![
                    SchemaField {
                        name: "first".into(),
                        schema: first.clone(),
                    },
                    SchemaField {
                        name: "second".into(),
                        schema: second.clone(),
                    },
                ]
                .into(),
            )
        } else {
            B::Tuple(vec![first.clone(), second.clone()].into())
        };
        let (s, ids) = arena(vec![draft(first, 2), draft(second, 2), draft(body, 2)]);
        let recipe = CompositeSnapshotRecipe::derive(ids[2], &ids[..2], Arc::clone(&s)).unwrap();
        for (rows, cols) in [
            (0, 0),
            (0, 7),
            (5, 0),
            (1, 1),
            (3, 1),
            (1, 5),
            (3, 5),
            (u64::MAX, 0),
        ] {
            let inputs = [
                (ids[0], shape(&s, ids[0], &[rows, cols])),
                (ids[1], shape(&s, ids[1], &[rows, cols])),
            ];
            let calls = Cell::new(0);
            let fast = recipe
                .bind(inputs.len(), |i| {
                    calls.set(calls.get() + 1);
                    inputs.get(i).map(|(id, s)| (*id, s))
                })
                .unwrap();
            let slow = ordinary(&s, ids[2], &inputs).unwrap();
            assert_eq!(fast.shape(), slow.shape());
            assert_eq!(fast.allocation_containers(), slow.allocation_containers());
            assert_eq!(calls.get(), 2);
        }
    }
}

#[test]
fn prepared_composite_checks_constant_shared_axes_and_exact_input_headers() {
    let child = matrix(vec![p(0), p(1)]);
    let (s, ids) = arena(vec![
        draft(child.clone(), 2),
        draft(
            B::Tuple(vec![matrix(vec![p(0), p(0)]), matrix(vec![D::Constant(2), p(0)])].into()),
            1,
        ),
        draft(B::Bool, 0),
    ]);
    let recipe =
        CompositeSnapshotRecipe::derive(ids[1], &[ids[0], ids[0]], Arc::clone(&s)).unwrap();
    for axes in [[2, 2], [2, 3], [1, 1], [0, 0]] {
        let inputs = [
            (ids[0], shape(&s, ids[0], &axes)),
            (ids[0], shape(&s, ids[0], &axes)),
        ];
        assert_eq!(
            recipe
                .bind(2, |i| Some((inputs[i].0, &inputs[i].1)))
                .is_ok(),
            ordinary(&s, ids[1], &inputs).is_ok()
        );
    }
    let valid = shape(&s, ids[0], &[2, 2]);
    assert!(recipe.bind(1, |_| Some((ids[0], &valid))).is_err());
    assert!(recipe.bind(3, |_| Some((ids[0], &valid))).is_err());
    assert!(recipe.bind(2, |_| None).is_err());
    assert!(recipe.bind(2, |_| Some((ids[2], &valid))).is_err());
}

#[test]
fn prepared_composite_rechecks_current_input_bounds_and_overflow() {
    let child = matrix(vec![p(0), p(1)]);
    let mut bounded = draft(child.clone(), 2);
    bounded.dimension_parameters[0].lower_bound = D::Constant(1);
    bounded.dimension_parameters[0].upper_bound = Some(D::Constant(4));
    let overflow = draft(
        matrix(vec![D::Multiply(vec![p(0), p(1)].into()), D::Constant(2)]),
        2,
    );
    let (s, ids) = arena(vec![
        draft(child.clone(), 2),
        bounded,
        overflow,
        draft(B::Tuple(vec![child].into()), 2),
    ]);
    for source in [ids[1], ids[2]] {
        let recipe = CompositeSnapshotRecipe::derive(ids[3], &[source], Arc::clone(&s)).unwrap();
        for values in [[0, 2], [5, 2], [2, 3], [u64::MAX, 0], [u64::MAX, 1]] {
            let borrowed = shape(&s, ids[0], &values);
            let inputs = [(source, borrowed)];
            assert_eq!(
                recipe.bind(1, |_| Some((source, &inputs[0].1))).is_ok(),
                ordinary(&s, ids[3], &inputs).is_ok()
            );
        }
    }
}

#[test]
fn prepared_composite_declines_unsupported_relations_and_nominal_substitution() {
    let child = matrix(vec![p(0), p(1)]);
    let mut bounded = draft(B::Tuple(vec![child.clone()].into()), 2);
    bounded.dimension_parameters[0].upper_bound = Some(D::Constant(4));
    let mut persistent = draft(B::Tuple(vec![child.clone()].into()), 2);
    persistent.dimension_parameters[0].lifetime = DimensionLifetime::Activation;
    let atom = |name: &str| {
        B::Atom(crate::NominalKey::from_path(
            crate::NominalKind::Atom,
            &crate::CanonicalNominalPath::new(vec![name.into()]).unwrap(),
        ))
    };
    let (s, ids) = arena(vec![
        draft(child.clone(), 2),
        bounded,
        persistent,
        draft(
            B::Tuple(
                vec![matrix(vec![
                    D::Add(vec![p(0), D::Constant(1)].into()),
                    p(1),
                ])]
                .into(),
            ),
            2,
        ),
        draft(B::Tuple(vec![child].into()), 3),
        draft(atom("left"), 0),
        draft(atom("right"), 0),
        draft(B::Tuple(vec![atom("left")].into()), 0),
    ]);
    for output in &ids[1..4] {
        assert!(CompositeSnapshotRecipe::derive(*output, &[ids[0]], Arc::clone(&s)).is_none());
    }
    assert_eq!(s.get(ids[4]).unwrap().dimension_parameters().len(), 2);
    assert!(CompositeSnapshotRecipe::derive(ids[4], &[ids[0]], Arc::clone(&s)).is_some());
    assert!(CompositeSnapshotRecipe::derive(ids[7], &[ids[6]], Arc::clone(&s)).is_none());
    let recipe = CompositeSnapshotRecipe::derive(ids[7], &[ids[5]], Arc::clone(&s)).unwrap();
    let wrong = shape(&s, ids[6], &[]);
    assert!(recipe.bind(1, |_| Some((ids[6], &wrong))).is_err());
}

#[test]
fn prepared_composite_publication_matches_ordinary_and_rejects_replaced_children() {
    let child = matrix(vec![p(0), p(1)]);
    let (s, ids) = arena(vec![
        draft(child.clone(), 2),
        draft(B::Tuple(vec![child].into()), 2),
    ]);
    let recipe = CompositeSnapshotRecipe::derive(ids[1], &[ids[0]], Arc::clone(&s)).unwrap();
    let bits = [
        0_u64,
        1_u64 << 63,
        0x7ff0_0000_0000_0001,
        0x7ff8_0000_0000_0001,
        0x3ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
    ];
    let v = ValueDraft {
        schema: ids[0],
        shape_values: vec![2, 3].into(),
        data: ValueDataDraft::Matrix(
            bits.iter()
                .map(|b| ValueDataDraft::F64(F64Bits::from_bits(*b)))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&s))
    .unwrap();
    let inputs = [(v.schema(), v.shape().clone())];
    let fast = recipe.bind(1, |_| Some((v.schema(), v.shape()))).unwrap();
    let slow = ordinary(&s, ids[1], &inputs).unwrap();
    let a = fast.construct(vec![v.clone()].into(), None).unwrap();
    let b = slow.construct(vec![v.clone()].into(), None).unwrap();
    assert_eq!(a.schema_key(), b.schema_key());
    assert_eq!(a.shape(), b.shape());
    assert!(schema_data_snapshot_eq(
        s.get(ids[1]).unwrap().body(),
        a.data(),
        b.data()
    ));
    let different = ValueDraft {
        schema: ids[0],
        shape_values: vec![3, 2].into(),
        data: ValueDataDraft::Matrix(
            bits.iter()
                .map(|b| ValueDataDraft::F64(F64Bits::from_bits(*b)))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&s))
    .unwrap();
    assert!(fast.construct(vec![different].into(), None).is_err());
}
