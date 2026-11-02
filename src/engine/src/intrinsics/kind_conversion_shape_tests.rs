//! Actual managed owner metadata, declared Set cardinality and old constraints.
use super::*;

struct TargetKinds(BTreeMap<KindId, CanonicalNominalPath>);
impl NamedKindPathResolver for TargetKinds {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        self.0.get(&id)
    }
}
fn matrix(element: SchemaBody, count: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(count)]
            .into_boxed_slice(),
    }
}
fn values(items: &[u8]) -> ValueDataDraft {
    ValueDataDraft::Matrix(items.iter().copied().map(ValueDataDraft::U8).collect())
}

#[test]
fn all_managed_conversion_owners_plan_the_actual_declared_nested_target() {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let body = SchemaBody::Option(Box::new(matrix(matrix(scalar.clone(), 4), 2)));
    let target = SchemaBody::Option(Box::new(matrix(
        SchemaBody::Set {
            element: Box::new(scalar),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(3)),
        },
        2,
    )));
    let source = ValueCell::from_schema_data(
        body.clone(),
        ValueDataDraft::Option(OptionDraft {
            present: true,
            value: Some(Box::new(ValueDataDraft::Matrix(
                vec![values(&[3, 1, 3, 2]), values(&[8, 8, 6, 7])].into_boxed_slice(),
            ))),
        }),
    )
    .unwrap();
    let plan = plan_explicit_cast(
        &source.resolved_type().unwrap(),
        &ResolvedType::from_schema_body(&target, &[]).unwrap(),
    )
    .unwrap();
    let output = execute_conversion_plan(&source, &target, &plan).unwrap();
    let expected = planned_conversion_output_shape(&output, &source, &plan)
        .unwrap()
        .unwrap();
    assert_eq!(
        current_conversion_target_body(&output, &source, &plan).unwrap(),
        target
    );
    let (planned, invocation) =
        planned_type_conversion_instance(source.clone(), output.clone(), plan.clone(), None, None);
    let runtime = RuntimeKindConversion::new_invocation(invocation).unwrap();
    let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
    let named = TargetKinds(BTreeMap::from([(id, path)]));
    let kind = ReifiedKind::from_closed_kind(
        plan.target.kind(),
        plan.target.dimension_parameters(),
        &named,
    )
    .unwrap();
    let target_cell = ValueCell::from_schema_data(
        SchemaBody::ReifiedType,
        ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
            kind.canonical_bytes().to_vec().into_boxed_slice(),
        )),
    )
    .unwrap();
    let reified = RuntimeReifiedKindConversion::new_invocation(FunctionInvocation::binary(
        output.clone(),
        source.clone(),
        target_cell,
    ))
    .unwrap();
    let old = output.snapshot().unwrap();
    let version = output.published_version();
    for implementation in [planned, runtime, reified] {
        let shapes = implementation.planned_output_shapes().unwrap().unwrap();
        assert_eq!(shapes.as_ref(), &[expected.clone()]);
        assert!(output.snapshot().unwrap().shares_frozen_storage(&old));
        assert_eq!(output.published_version(), version);
    }

    // Only current metadata is required. An absent Option has no Matrix
    // payload to inspect, but still carries the SAME declared nested axes.
    let absent = ValueCell::from_schema_data(
        body,
        ValueDataDraft::Option(OptionDraft {
            present: false,
            value: None,
        }),
    )
    .unwrap();
    let (planned, _) = planned_type_conversion_instance(absent, output.clone(), plan, None, None);
    assert_eq!(
        planned.planned_output_shapes().unwrap().unwrap().as_ref(),
        &[expected]
    );
    assert!(output.snapshot().unwrap().shares_frozen_storage(&old));
}

#[test]
fn ordinary_non_set_conversion_constraints_and_default_shape_hook_are_preserved() {
    let source = ValueCell::from_schema_data(
        matrix(SchemaBody::UnsignedInteger(IntegerWidth::W8), 4),
        values(&[1, 2, 3, 4]),
    )
    .unwrap();
    let output = |n: u64| {
        ValueCell::from_schema_data(
            matrix(SchemaBody::UnsignedInteger(IntegerWidth::W16), n),
            ValueDataDraft::Matrix((0..n).map(|_| ValueDataDraft::U16(0)).collect()),
        )
        .unwrap()
    };
    let good = output(4);
    let runtime =
        RuntimeKindConversion::new_invocation(FunctionInvocation::unary(good, source.clone()))
            .unwrap();
    assert!(runtime.planned_output_shapes().unwrap().is_none());
    // U16 construction may select dense storage whose snapshots are rebuilt.
    // Use actual retained canonical storage for a frozen-identity assertion.
    let bad = ValueCell::from_snapshot(output(3).snapshot().unwrap()).unwrap();
    assert!(bad.requires_canonical_output_builder().unwrap());
    let old = bad.snapshot().unwrap();
    let version = bad.published_version();
    assert!(
        RuntimeKindConversion::new_invocation(FunctionInvocation::unary(bad.clone(), source))
            .is_err()
    );
    assert!(bad.snapshot().unwrap().shares_frozen_storage(&old));
    assert_eq!(bad.published_version(), version);
}
