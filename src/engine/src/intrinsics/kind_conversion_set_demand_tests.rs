use super::super::*;

fn matrix(element: SchemaBody, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(columns)]
            .into_boxed_slice(),
    }
}
fn set(element: SchemaBody) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    }
}

#[test]
fn conversion_staging_is_admitted_exactly_before_managed_materialization() {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let body = matrix(scalar.clone(), 4);
    let cell = |values: &[u8]| {
        let optimized = ValueCell::from_schema_data(
            body.clone(),
            ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U8).collect()),
        )
        .unwrap();
        // from_schema_data may choose dense U8 storage. This regression
        // deliberately exercises zero-reconstruction canonical input instead.
        ValueCell::from_snapshot(optimized.snapshot().unwrap()).unwrap()
    };
    let source = cell(&[3, 1, 3, 2]);
    let target = set(scalar);
    let selected = plan_explicit_cast(
        &source.resolved_type().unwrap(),
        &ResolvedType::from_schema_body(&target, &[]).unwrap(),
    )
    .unwrap();
    let output = execute_conversion_plan(&source, &target, &selected).unwrap();
    assert!(source.requires_canonical_output_builder().unwrap());
    let reference =
        planned_type_conversion_specialized(source.clone(), output.clone(), selected.clone())
            .unwrap();
    source
        .replace(&cell(&[5, 4, 5, 4]).snapshot().unwrap())
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
    let resolved = BTreeMap::from([
        (
            (PortDirection::Input, 0u16),
            source.current_memory_footprint().unwrap(),
        ),
        ((PortDirection::Output, 0u16), prospective),
    ]);
    let base = mech_core::memory_plan::resolve_current_call_memory(
        &template,
        &current,
        &resolved,
        Some(&[output.current_memory_footprint().unwrap()]),
    )
    .unwrap();
    let witnesses = [MemoryFootprintWitness::Known(
        source.current_memory_footprint().unwrap(),
    )];
    let request = ExecutionDemandPlanningRequest {
        inputs: current.inputs(),
        input_witnesses: &witnesses,
    };
    assert_eq!(
        conversion_input_reconstruction_demand(&source, &output, &request).unwrap(),
        PlannedExecutionDemand::default()
    );
    let extra = conversion_execution_demand(&source, &output, &selected, &request).unwrap();
    assert!(
        extra.temporary_bytes > 0
            && extra.cloned_bytes > 0
            && extra.retained_nodes > 0
            && extra.work.compute > 0
    );
    let exact = base
        .demand
        .turn_peak_bytes
        .checked_add(extra.temporary_bytes)
        .unwrap();
    let make = |limit| {
        let mut plan = template.clone();
        plan.target.limits.max_temporary_bytes = Some(limit);
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
    let old = output.snapshot().unwrap();
    let version = output.published_version();
    let error = short.instance().solve_result().unwrap_err();
    assert!(format!("{error:?}").contains("TemporaryBytes"), "{error:?}");
    assert!(output.snapshot().unwrap().shares_frozen_storage(&old));
    assert_eq!(output.published_version(), version);
    drop(short);
    let admitted = make(exact);
    admitted.instance().solve_result().unwrap();
    assert_eq!(
        output.snapshot().unwrap().canonical_data_draft().unwrap(),
        ValueDataDraft::Set(vec![ValueDataDraft::U8(4), ValueDataDraft::U8(5)].into_boxed_slice())
    );
}

#[cfg(feature = "atom")]
#[test]
fn nested_packed_atom_population_is_exact_borrowed_and_absence_is_lazy() {
    use mech_core::snapshot::OptionDraft;
    let count = 4096;
    let atom = SchemaBody::Atom(NominalKey::from_bytes([73; 32]));
    let schema = SchemaBody::Option(Box::new(matrix(atom.clone(), count)));
    let cell = |present: bool| {
        ValueCell::from_schema_data(
            schema.clone(),
            ValueDataDraft::Option(OptionDraft {
                present,
                value: present.then(|| {
                    Box::new(ValueDataDraft::Matrix(
                        vec![ValueDataDraft::Atom; count as usize].into_boxed_slice(),
                    ))
                }),
            }),
        )
        .unwrap()
    };
    let present = cell(true);
    let absent = cell(false);
    let footprint = present.current_memory_footprint().unwrap();
    assert_eq!(footprint.logical_elements, 1);
    assert!(footprint.retained_nodes < count);
    assert_eq!(
        present.current_canonical_draft_population().unwrap(),
        Some((count + 2, 192))
    );
    assert_eq!(
        absent.current_canonical_draft_population().unwrap(),
        Some((1, 64))
    );
    let target = SchemaBody::Option(Box::new(set(atom)));
    let selected = plan_explicit_cast(
        &present.resolved_type().unwrap(),
        &ResolvedType::from_schema_body(&target, &[]).unwrap(),
    )
    .unwrap();
    let output = execute_conversion_plan(&present, &target, &selected).unwrap();
    let demand = |source: &ValueCell| {
        let inputs = [source.resolved_descriptor().unwrap()];
        let facts = [MemoryFootprintWitness::Known(
            source.current_memory_footprint().unwrap(),
        )];
        conversion_execution_demand(
            source,
            &output,
            &selected,
            &ExecutionDemandPlanningRequest {
                inputs: &inputs,
                input_witnesses: &facts,
            },
        )
        .unwrap()
    };
    let populated = demand(&present);
    let empty = demand(&absent);
    assert!(
        populated.temporary_bytes
            > empty.temporary_bytes + count * core::mem::size_of::<ValueDataDraft>() as u64
    );
    assert!(populated.work.compute > empty.work.compute);
}

#[cfg(all(feature = "f64", feature = "string"))]
#[test]
fn nested_string_conversion_funds_actual_publication_and_private_payloads() {
    use mech_core::snapshot::{F64Bits, OptionDraft};
    let count = 32;
    let source = ValueCell::from_schema_data(
        SchemaBody::Option(Box::new(matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            count,
        ))),
        ValueDataDraft::Option(OptionDraft {
            present: true,
            value: Some(Box::new(ValueDataDraft::Matrix(
                (1..=count)
                    .map(|bits| ValueDataDraft::F64(F64Bits::from_f64(f64::from_bits(bits))))
                    .collect(),
            ))),
        }),
    )
    .unwrap();
    assert_eq!(
        source.current_memory_footprint().unwrap().logical_elements,
        1
    );
    let target = SchemaBody::Option(Box::new(set(SchemaBody::String)));
    let selected = plan_explicit_cast(
        &source.resolved_type().unwrap(),
        &ResolvedType::from_schema_body(&target, &[]).unwrap(),
    )
    .unwrap();
    let output = execute_conversion_plan(&source, &target, &selected).unwrap();
    let actual = output.current_memory_footprint().unwrap();
    let bound = prospective_conversion_output_footprint(&output, &source, &selected)
        .unwrap()
        .unwrap();
    assert!(actual.payload_bytes > count * 300);
    assert!(bound.payload_bytes >= actual.payload_bytes);
    assert!(bound.encoded_bytes >= actual.encoded_bytes);
    assert!(bound.retained_nodes >= actual.retained_nodes);
    let function = planned_type_conversion_specialized(source, output, selected).unwrap();
    function.instance().solve_result().unwrap();
}
