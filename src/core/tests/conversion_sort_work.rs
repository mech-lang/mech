use mech_core::snapshot::SnapshotValidationContext;
use mech_core::*;

fn matrix(element: SchemaBody) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)].into_boxed_slice(),
    }
}
fn set(element: SchemaBody) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
    }
}
fn schema(body: SchemaBody) -> Schema {
    SchemaDraft {
        body,
        dimension_parameters: Box::new([]),
    }
    .finalize()
    .unwrap()
}
fn value(body: SchemaBody, data: ValueDataDraft) -> Value {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema(body)).unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    let table = built.into_parts().0;
    ValueDraft {
        schema: id,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&table))
    .unwrap()
}
fn plan(source: &Schema, target: &Schema) -> ConversionPlan {
    plan_explicit_cast(
        &ResolvedType::from_schema(source, &source.instantiate_shape(Box::new([])).unwrap())
            .unwrap(),
        &ResolvedType::from_schema(target, &target.instantiate_shape(Box::new([])).unwrap())
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn actual_packed_unicode_string_maximum_matches_shared_resident_units() {
    let source = schema(matrix(SchemaBody::String));
    let target = schema(set(SchemaBody::String));
    let selected = plan(&source, &target);
    let input = value(
        source.body().clone(),
        ValueDataDraft::Matrix(
            ["é🙂", "x", "é🙂"]
                .into_iter()
                .map(|s| ValueDataDraft::String(s.into()))
                .collect(),
        ),
    );
    let mut observed = 0;
    let actual = conversion_comparison_work(
        source.body(),
        target.body(),
        &selected.step,
        input.data(),
        4096,
        1,
        |work| {
            observed += work;
            Ok::<(), ()>(())
        },
    )
    .unwrap();
    let (fixed, compound) =
        conversion_sort_comparison_units(&SchemaBody::String, &SchemaBody::String, 3, 6).unwrap();
    assert_eq!((fixed, compound), (108, 0));
    assert_eq!(actual, fixed);
    assert_eq!(observed, 195);
    assert_eq!(
        conversion_comparison_work(
            source.body(),
            target.body(),
            &selected.step,
            input.data(),
            4096,
            1,
            |_| Err("denied")
        ),
        Err(ConversionPopulationError::Observer("denied"))
    );
}

#[test]
fn canonical_source_key_roundtrips_are_funded_in_addition_to_outer_sort() {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let key = set(scalar);
    let source = schema(matrix(key.clone()));
    let target = schema(set(key));
    let selected = plan(&source, &target);
    let child = |a, b| {
        ValueDataDraft::Set(vec![ValueDataDraft::U8(a), ValueDataDraft::U8(b)].into_boxed_slice())
    };
    let input = value(
        source.body().clone(),
        ValueDataDraft::Matrix(vec![child(1, 2), child(2, 3), child(1, 2)].into_boxed_slice()),
    );
    let payload = 4096;
    let actual = conversion_comparison_work(
        source.body(),
        target.body(),
        &selected.step,
        input.data(),
        payload,
        1,
        |_| Ok::<(), ()>(()),
    )
    .unwrap();
    // Outer compound-key heapsort plus three real two-element numeric Sets,
    // each rebuilt during normalization and final publication.
    assert_eq!(actual, 18 * payload + 3 * 12 * 2);
    assert_eq!(
        conversion_comparison_work(
            source.body(),
            target.body(),
            &selected.step,
            input.data(),
            u64::MAX,
            1,
            |_| Ok::<(), ()>(())
        ),
        Err(ConversionPopulationError::ArithmeticOverflow)
    );
}
