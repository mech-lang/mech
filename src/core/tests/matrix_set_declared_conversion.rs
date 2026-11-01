use mech_core::snapshot::{EnumDraft, SnapshotCanonicalizationBudget, SnapshotValidationContext};
use mech_core::*;

fn matrix(element: SchemaBody, rows: u64, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    }
}
fn set(element: SchemaBody) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    }
}
fn schema(body: SchemaBody, dimensions: Box<[DimensionParameterDeclaration]>) -> Schema {
    SchemaDraft {
        body,
        dimension_parameters: dimensions,
    }
    .finalize()
    .unwrap()
}
fn fixture(
    source: Schema,
    source_shape: ShapeInstance,
    data: ValueDataDraft,
    target: Schema,
) -> (SchemaTable, Value, SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let source_id = builder.insert(source).unwrap();
    let target_id = builder.insert(target).unwrap();
    let build = builder.finish().unwrap();
    let source_id = build.resolve(source_id).unwrap();
    let target_id = build.resolve(target_id).unwrap();
    let table = build.into_parts().0;
    let source = ValueDraft {
        schema: source_id,
        shape_values: source_shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&table))
    .unwrap();
    (table, source, target_id)
}
fn plan(table: &SchemaTable, source: &Value, target: SchemaId) -> ConversionPlan {
    let target = table.get(target).unwrap();
    plan_explicit_cast(
        &ResolvedType::from_schema(source.validate_against(table).unwrap(), source.shape())
            .unwrap(),
        &ResolvedType::from_schema(target, &shape_for_declared_lower_bounds(target).unwrap())
            .unwrap(),
    )
    .unwrap()
}
fn convert(table: &SchemaTable, source: &Value, target: SchemaId) -> Value {
    let target_schema = table.get(target).unwrap();
    let plan = plan(table, source, target);
    let candidate = conversion_output_shape(
        source.validate_against(table).unwrap(),
        source.shape(),
        target_schema,
        &plan,
    )
    .unwrap();
    let data = execute_conversion_draft_with_schema(
        source.canonical_data_draft().unwrap(),
        &plan,
        target_schema,
        &candidate,
        &SnapshotValidationContext::new(table),
    )
    .unwrap();
    let shape = conversion_output_shape_for_data(target_schema, &candidate, &data).unwrap();
    ValueDraft {
        schema: target,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(table))
    .unwrap()
}
fn event() -> SchemaBody {
    SchemaBody::Enum {
        key: NominalKey::from_bytes([31; 32]),
        variants: vec![
            EnumVariantSchema {
                name: "none".into(),
                payload: None,
            },
            EnumVariantSchema {
                name: "word".into(),
                payload: Some(SchemaBody::String),
            },
        ]
        .into_boxed_slice(),
    }
}
fn enumeration(ordinal: u32, text: Option<&str>) -> ValueDataDraft {
    ValueDataDraft::Enum(EnumDraft {
        ordinal,
        payload: text.map(|text| Box::new(ValueDataDraft::String(text.into()))),
    })
}
fn key(values: &[u8]) -> ValueDataDraft {
    ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U8).collect())
}

fn parameters(count: u32) -> Box<[DimensionParameterDeclaration]> {
    (0..count)
        .map(|index| DimensionParameterDeclaration {
            id: DimensionParameterId::new(index),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(16)),
        })
        .collect()
}
fn axis(index: u32) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(index))
}

#[test]
fn declared_nominal_enum_set_conversion_retains_variants_and_payload_identity() {
    let element = event();
    let source = schema(matrix(element.clone(), 1, 4), Box::new([]));
    let shape = source.instantiate_shape(Box::new([])).unwrap();
    let (table, source, target) = fixture(
        source,
        shape,
        ValueDataDraft::Matrix(
            vec![
                enumeration(1, Some("é")),
                enumeration(0, None),
                enumeration(1, Some("é")),
                enumeration(1, Some("é")),
            ]
            .into_boxed_slice(),
        ),
        schema(set(element.clone()), Box::new([])),
    );
    let output = convert(&table, &source, target);
    assert_eq!(
        output.canonical_data_draft().unwrap(),
        ValueDataDraft::Set(
            vec![
                enumeration(0, None),
                enumeration(1, Some("é")),
                enumeration(1, Some("é")),
            ]
            .into_boxed_slice()
        )
    );
    assert!(
        matches!(table.get(target).unwrap().body(), SchemaBody::Set { element: actual, .. }
        if actual.as_ref() == &element)
    );
    assert!(matches!(
        plan(&table, &source, target).step,
        ConversionStep::MatrixToSet(_)
    ));
    // A nominal key is not permission to replace its declared variant schema.
    let mut wrong = event();
    if let SchemaBody::Enum { variants, .. } = &mut wrong {
        variants[1].payload = Some(SchemaBody::Bool);
    }
    let wrong = schema(set(wrong), Box::new([]));
    assert_eq!(
        conversion_output_shape(
            source.validate_against(&table).unwrap(),
            source.shape(),
            &wrong,
            &plan(&table, &source, target)
        ),
        Err(ConversionExecutionError::ConversionShapeMismatch)
    );
}

#[path = "declared_set_conversion_cases/geometry.rs"]
mod geometry;
#[path = "declared_set_conversion_cases/isolation.rs"]
mod isolation;
#[path = "declared_set_conversion_cases/nested.rs"]
mod nested;
#[path = "declared_set_conversion_cases/shared_axes.rs"]
mod shared_axes;
