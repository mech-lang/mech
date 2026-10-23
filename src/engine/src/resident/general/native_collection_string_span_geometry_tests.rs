//! Prices describe actual shared-solver branches, not a benchmark allowance.
use super::{dimension_geometry, direct_selector_witnesses, selector_schema_geometry_bounds};
use mech_core::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, IntegerWidth, Schema, SchemaBody, SchemaDraft, ShapeInstance,
};

fn parameter(index: u32) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(index))
}

fn body(element: SchemaBody, dimensions: Vec<DimensionExpr>) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: dimensions.into_boxed_slice(),
    }
}

fn u64_body(dimensions: Vec<DimensionExpr>) -> SchemaBody {
    body(SchemaBody::UnsignedInteger(IntegerWidth::W64), dimensions)
}

fn declaration(
    index: u32,
    lower_bound: DimensionExpr,
    upper_bound: Option<DimensionExpr>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(index),
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Turn,
        lower_bound,
        upper_bound,
    }
}

fn schema(body: SchemaBody, declarations: Vec<DimensionParameterDeclaration>) -> Schema {
    SchemaDraft {
        dimension_parameters: declarations.into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap()
}

// Independent formula assertion: retain every existing metadata term and
// coefficient, varying ONLY the proved number of search evaluations.
fn expected_bounds(schema: &Schema, search_passes: u64) -> (u64, u64) {
    let SchemaBody::Matrix { dimensions, .. } = schema.body() else {
        panic!("test schema must be Matrix");
    };
    let parameters = schema.dimension_parameters().len() as u64;
    let (nodes, depth) = dimensions
        .iter()
        .chain(schema.dimension_parameters().iter().flat_map(|parameter| {
            core::iter::once(parameter.lower_bound()).chain(parameter.upper_bound())
        }))
        .map(|expression| dimension_geometry(expression).unwrap())
        .fold((0u64, 0u64), |(nodes, depth), (next_nodes, next_depth)| {
            (nodes + next_nodes, depth.max(next_depth))
        });
    let metadata = parameters * (3 * core::mem::size_of::<u64>() as u64 + 1)
        + 2 * core::mem::size_of::<(&DimensionExpr, u64)>() as u64
        + 3 * core::mem::size_of::<ShapeInstance>() as u64
        + 3 * core::mem::size_of::<Vec<u64>>() as u64
        + 512;
    let traversed = nodes * core::mem::size_of::<DimensionExpr>() as u64
        + parameters * core::mem::size_of::<u64>() as u64
        + core::mem::size_of::<Schema>() as u64;
    (
        metadata,
        traversed * (depth + search_passes + 2 * parameters + 16) * 8 + 256,
    )
}

#[test]
fn selector_direct_witness_classifier_accepts_only_the_proved_leaf_grammar() {
    assert!(direct_selector_witnesses(
        &u64_body(vec![DimensionExpr::Constant(1), parameter(0)]),
        1,
    ));
    for extent in [0, 1, 7, u64::MAX] {
        assert!(direct_selector_witnesses(
            &u64_body(vec![
                DimensionExpr::Constant(1),
                DimensionExpr::Constant(extent)
            ]),
            0,
        ));
    }
    let ordinary = u64_body(vec![DimensionExpr::Constant(1), parameter(0)]);
    assert!(!direct_selector_witnesses(&ordinary, 2));
    assert!(!direct_selector_witnesses(&ordinary, 0));
    assert!(!direct_selector_witnesses(
        &u64_body(vec![DimensionExpr::Constant(1), parameter(1)]),
        1,
    ));
    assert!(!direct_selector_witnesses(&SchemaBody::String, 0));
    for dimensions in [
        vec![parameter(0)],
        vec![
            DimensionExpr::Constant(1),
            parameter(0),
            DimensionExpr::Constant(1),
        ],
        vec![DimensionExpr::Constant(2), parameter(0)],
        vec![DimensionExpr::Constant(1), DimensionExpr::Hole],
    ] {
        assert!(!direct_selector_witnesses(&u64_body(dimensions), 1));
    }
    for element in [
        SchemaBody::Bool,
        SchemaBody::UnsignedInteger(IntegerWidth::W32),
    ] {
        assert!(!direct_selector_witnesses(
            &body(element, vec![DimensionExpr::Constant(1), parameter(0)]),
            1,
        ));
    }
    for compound in [
        DimensionExpr::Add(vec![parameter(0), DimensionExpr::Constant(1)].into_boxed_slice()),
        DimensionExpr::Multiply(vec![parameter(0), DimensionExpr::Constant(2)].into_boxed_slice()),
        DimensionExpr::Min(vec![parameter(0), DimensionExpr::Constant(8)].into_boxed_slice()),
        DimensionExpr::Max(vec![parameter(0), DimensionExpr::Constant(1)].into_boxed_slice()),
    ] {
        assert!(!direct_selector_witnesses(
            &u64_body(vec![DimensionExpr::Constant(1), compound]),
            1,
        ));
    }
}

#[test]
fn selector_direct_witness_price_removes_search_only_and_keeps_bounds_and_metadata() {
    let unbounded = schema(
        u64_body(vec![DimensionExpr::Constant(1), parameter(0)]),
        vec![declaration(0, DimensionExpr::Constant(0), None)],
    );
    let bounded = schema(
        u64_body(vec![DimensionExpr::Constant(1), parameter(0)]),
        vec![declaration(
            0,
            DimensionExpr::Constant(2),
            Some(DimensionExpr::Constant(8)),
        )],
    );
    let fixed = schema(
        u64_body(vec![DimensionExpr::Constant(1), DimensionExpr::Constant(7)]),
        vec![],
    );
    for schema in [&unbounded, &bounded, &fixed] {
        let actual = selector_schema_geometry_bounds(schema).unwrap();
        assert_eq!(actual, expected_bounds(schema, 0));
        let old = expected_bounds(schema, 2 * u64::BITS as u64);
        assert_eq!(actual.0, old.0, "no metadata reservation was discounted");
        assert!(
            actual.1 < old.1,
            "only unreachable search evaluations were removed"
        );
    }
    assert_eq!(
        selector_schema_geometry_bounds(&unbounded).unwrap().0,
        selector_schema_geometry_bounds(&bounded).unwrap().0,
    );
    assert!(
        selector_schema_geometry_bounds(&bounded).unwrap().1
            > selector_schema_geometry_bounds(&unbounded).unwrap().1,
        "the actual upper-bound expression remains funded",
    );
}

#[test]
fn selector_compound_and_dependent_schema_prices_keep_the_full_search_bound() {
    for expression in [
        DimensionExpr::Add(vec![parameter(0), DimensionExpr::Constant(1)].into_boxed_slice()),
        DimensionExpr::Multiply(vec![parameter(0), DimensionExpr::Constant(2)].into_boxed_slice()),
        DimensionExpr::Min(vec![parameter(0), DimensionExpr::Constant(8)].into_boxed_slice()),
        DimensionExpr::Max(vec![parameter(0), DimensionExpr::Constant(1)].into_boxed_slice()),
    ] {
        let schema = schema(
            u64_body(vec![DimensionExpr::Constant(1), expression]),
            vec![declaration(0, DimensionExpr::Constant(0), None)],
        );
        assert!(!direct_selector_witnesses(
            schema.body(),
            schema.dimension_parameters().len()
        ));
        assert_eq!(
            selector_schema_geometry_bounds(&schema),
            Some(expected_bounds(&schema, 2 * u64::BITS as u64)),
        );
    }
    let dependent = schema(
        u64_body(vec![DimensionExpr::Constant(1), parameter(1)]),
        vec![
            declaration(0, DimensionExpr::Constant(0), None),
            declaration(
                1,
                DimensionExpr::Add(
                    vec![parameter(0), DimensionExpr::Constant(1)].into_boxed_slice(),
                ),
                None,
            ),
        ],
    );
    assert_eq!(dependent.dimension_parameters().len(), 2);
    assert_eq!(
        selector_schema_geometry_bounds(&dependent),
        Some(expected_bounds(&dependent, 2 * u64::BITS as u64)),
    );
    let bound_tree = DimensionExpr::Add(
        vec![
            DimensionExpr::Max(vec![parameter(0), DimensionExpr::Constant(3)].into_boxed_slice()),
            DimensionExpr::Constant(1),
        ]
        .into_boxed_slice(),
    );
    assert_eq!(dimension_geometry(&bound_tree), Some((5, 3)));
}
