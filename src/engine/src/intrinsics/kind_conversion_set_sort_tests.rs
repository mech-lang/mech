//! Real managed-call comparison admission, not a detached normalizer surrogate.
use super::super::*;
use mech_core::snapshot::{OptionDraft, ReifiedTypeDraft};

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
fn set(element: SchemaBody) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(3)),
    }
}
fn values(left: &[u8], right: &[u8]) -> ValueDataDraft {
    ValueDataDraft::Option(OptionDraft {
        present: true,
        value: Some(Box::new(ValueDataDraft::Matrix(
            [left, right]
                .into_iter()
                .map(|items| {
                    ValueDataDraft::Matrix(items.iter().copied().map(ValueDataDraft::U8).collect())
                })
                .collect(),
        ))),
    })
}

#[test]
fn all_source_conversion_owners_fund_actual_nested_groups_and_absence_is_lazy() {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let body = SchemaBody::Option(Box::new(matrix(matrix(scalar.clone(), 4), 2)));
    let target = SchemaBody::Option(Box::new(matrix(set(scalar), 2)));
    let source =
        ValueCell::from_schema_data(body.clone(), values(&[3, 1, 3, 2], &[8, 8, 6, 7])).unwrap();
    let selected = plan_explicit_cast(
        &source.resolved_type().unwrap(),
        &ResolvedType::from_schema_body(&target, &[]).unwrap(),
    )
    .unwrap();
    let output = execute_conversion_plan(&source, &target, &selected).unwrap();
    let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
    let named = TargetKinds(BTreeMap::from([(id, path)]));
    let target_kind = ReifiedKind::from_closed_kind(
        selected.target.kind(),
        selected.target.dimension_parameters(),
        &named,
    )
    .unwrap();
    let target_cell = ValueCell::from_schema_data(
        SchemaBody::ReifiedType,
        ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
            target_kind.canonical_bytes().to_vec().into_boxed_slice(),
        )),
    )
    .unwrap();
    let (planned, invocation) = planned_type_conversion_instance(
        source.clone(),
        output.clone(),
        selected.clone(),
        None,
        None,
    );
    let runtime = RuntimeKindConversion::new_invocation(invocation).unwrap();
    let reified = RuntimeReifiedKindConversion::new_invocation(FunctionInvocation::binary(
        output.clone(),
        source.clone(),
        target_cell.clone(),
    ))
    .unwrap();
    let inputs = [
        source.resolved_descriptor().unwrap(),
        target_cell.resolved_descriptor().unwrap(),
    ];
    let witnesses = [
        MemoryFootprintWitness::Known(source.current_memory_footprint().unwrap()),
        MemoryFootprintWitness::Known(target_cell.current_memory_footprint().unwrap()),
    ];
    // Two actual four-element inner groups; MatrixElements adds an ancestor
    // rebuild phase. Root logical_elements is only one under Option.
    assert_eq!(
        source.current_memory_footprint().unwrap().logical_elements,
        1
    );
    let expected = 2 * 3 * 4 * 3 * conversion_materialization_phases(&selected.step);
    for (implementation, count) in [(planned, 1), (runtime, 1), (reified, 2)] {
        let demand = implementation
            .planned_execution_demand(&ExecutionDemandPlanningRequest {
                inputs: &inputs[..count],
                input_witnesses: &witnesses[..count],
            })
            .unwrap();
        assert_eq!(demand.work.comparison, expected);
        assert_eq!(demand.work.canonicalization, expected);
    }
    let absent = ValueCell::from_schema_data(
        body,
        ValueDataDraft::Option(OptionDraft {
            present: false,
            value: None,
        }),
    )
    .unwrap();
    let absent_inputs = [absent.resolved_descriptor().unwrap()];
    let absent_witnesses = [MemoryFootprintWitness::Known(
        absent.current_memory_footprint().unwrap(),
    )];
    let empty = conversion_execution_demand(
        &absent,
        &output,
        &selected,
        &ExecutionDemandPlanningRequest {
            inputs: &absent_inputs,
            input_witnesses: &absent_witnesses,
        },
    )
    .unwrap();
    assert_eq!(empty.work.comparison, 0);
    assert_eq!(empty.work.canonicalization, 0);
}

#[test]
fn managed_comparison_exact_and_one_short_preserve_publication_and_release_owners() {
    let memory = mech_core::ManagedMemoryBudget::new(128 * 1024 * 1024);
    {
        let domain = mech_core::MemoryDomain::with_memory_budget(memory.clone()).unwrap();
        let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
        let body = SchemaBody::Option(Box::new(matrix(matrix(scalar.clone(), 4), 2)));
        let source_value =
            ValueCell::from_schema_data(body.clone(), values(&[3, 1, 3, 2], &[8, 8, 6, 7]))
                .unwrap()
                .snapshot()
                .unwrap();
        let source = ValueCell::from_snapshot_in(&domain, source_value).unwrap();
        let target = SchemaBody::Option(Box::new(matrix(set(scalar), 2)));
        let selected = plan_explicit_cast(
            &source.resolved_type().unwrap(),
            &ResolvedType::from_schema_body(&target, &[]).unwrap(),
        )
        .unwrap();
        let initial = execute_conversion_plan(&source, &target, &selected)
            .unwrap()
            .snapshot()
            .unwrap();
        let output = ValueCell::from_snapshot_in(&domain, initial).unwrap();
        let reference =
            planned_type_conversion_specialized(source.clone(), output.clone(), selected.clone())
                .unwrap();
        source
            .replace(
                &ValueCell::from_schema_data(body, values(&[5, 4, 5, 6], &[9, 9, 8, 7]))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        let template = reference.memory_plan().clone();
        let current = template.bound_call.clone();
        let mut prospective = prospective_conversion_output_footprint(&output, &source, &selected)
            .unwrap()
            .unwrap();
        prospective.schema_bytes = prospective.schema_bytes.max(
            output
                .retained_schema_table()
                .clone_allocation_bound_bytes()
                .unwrap(),
        );
        let source_footprint = source.current_memory_footprint().unwrap();
        let base = mech_core::memory_plan::resolve_current_call_memory(
            &template,
            &current,
            &BTreeMap::from([
                ((PortDirection::Input, 0u16), source_footprint),
                ((PortDirection::Output, 0u16), prospective),
            ]),
            Some(&[output.current_memory_footprint().unwrap()]),
        )
        .unwrap();
        let witnesses = [MemoryFootprintWitness::Known(source_footprint)];
        let extra = conversion_execution_demand(
            &source,
            &output,
            &selected,
            &ExecutionDemandPlanningRequest {
                inputs: current.inputs(),
                input_witnesses: &witnesses,
            },
        )
        .unwrap();
        assert!(extra.work.comparison > 0);
        let exact = base
            .demand
            .work
            .comparison
            .checked_add(extra.work.comparison)
            .unwrap();
        let make = |limit| {
            let mut plan = template.clone();
            plan.target.limits.max_comparison_work = Some(limit);
            SpecializedFunction::new(
                planned_type_conversion_instance(
                    source.clone(),
                    output.clone(),
                    selected.clone(),
                    None,
                    None,
                ),
                plan.bound_call.clone(),
                plan,
            )
            .unwrap()
        };
        let short = make(exact - 1);
        drop(reference);
        // Failed call preparation is itself a domain reclamation point.
        // Retire the deliberately dropped reference realization first, so
        // strict denial accounting compares the same live-owner population.
        domain.collect_retired().unwrap();
        let old = output.snapshot().unwrap();
        let version = output.published_version();
        let baseline = memory.used_bytes();
        let error = short.instance().solve_result().unwrap_err();
        assert!(format!("{error:?}").contains("ComparisonWork"), "{error:?}");
        assert!(output.snapshot().unwrap().shares_frozen_storage(&old));
        assert_eq!(output.published_version(), version);
        assert_eq!(memory.used_bytes(), baseline);
        drop(short);
        let admitted = make(exact);
        admitted.instance().solve_result().unwrap();
        let actual = output.snapshot().unwrap().canonical_data_draft().unwrap();
        assert_eq!(
            actual,
            ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(ValueDataDraft::Matrix(
                    vec![
                        ValueDataDraft::Set(
                            vec![
                                ValueDataDraft::U8(4),
                                ValueDataDraft::U8(5),
                                ValueDataDraft::U8(6)
                            ]
                            .into_boxed_slice()
                        ),
                        ValueDataDraft::Set(
                            vec![
                                ValueDataDraft::U8(7),
                                ValueDataDraft::U8(8),
                                ValueDataDraft::U8(9)
                            ]
                            .into_boxed_slice()
                        ),
                    ]
                    .into_boxed_slice()
                ))),
            })
        );
    }
    assert_eq!(memory.used_bytes(), 0);
}
