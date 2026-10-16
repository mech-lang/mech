//! Provider schemas retain their lifetimes, bounds, and actual observation geometry.
use super::*;
use mech_core::{
    DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, FloatWidth, SchemaDraft, SchemaTableBuilder,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};

const SOURCE: &str = "@typed := test://typed/value{:read(data)}\nsample := @typed/data\nsample\n";

#[derive(Clone, Copy)]
enum Axis {
    Fixed(u64),
    Parameter(DimensionLifetime, u64),
}

fn planned(rows: u64, columns: u64, axes: [Axis; 2]) -> Value {
    let mut parameters = Vec::new();
    let dimensions = axes
        .into_iter()
        .map(|axis| match axis {
            Axis::Fixed(extent) => DimensionExpr::Constant(extent),
            Axis::Parameter(lifetime, upper) => {
                let id = DimensionParameterId::new(parameters.len() as u32);
                parameters.push(DimensionParameterDeclaration {
                    id,
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(upper)),
                });
                DimensionExpr::Parameter(id)
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let mut schemas = SchemaTableBuilder::new();
    let schema = schemas
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions,
                },
                dimension_parameters: parameters.into_boxed_slice(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = schemas.finish().unwrap();
    let schema = built.resolve(schema).unwrap();
    let shape =
        mech_core::shape_for_resolved_extents(built.table.get(schema).unwrap(), &[rows, columns])
            .unwrap();
    ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            (0..rows * columns)
                .map(|i| ValueDataDraft::F64(F64Bits::from_f64(i as f64 + 0.5)))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .unwrap()
}

#[derive(Debug)]
struct SchemaProvider {
    planned: Value,
    reads: Arc<AtomicUsize>,
}
impl RuntimeResourceProvider for SchemaProvider {
    fn scheme(&self) -> &str {
        "test"
    }
    fn base_uris(&self) -> Vec<String> {
        vec!["test://typed/value".to_owned()]
    }
    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }
    fn plan_read(&self, _: RuntimeResourceReadRequest) -> MResult<Value> {
        Ok(self.planned.clone())
    }
    fn read(&self, _: RuntimeResourceReadRequest) -> MResult<Value> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.planned.clone())
    }
}
fn configured(value: Value, reads: Arc<AtomicUsize>) -> crate::MechRuntime {
    let mut result = runtime();
    result
        .register_resource_provider(Box::new(SchemaProvider {
            planned: value,
            reads,
        }))
        .unwrap();
    let subject = result.runtime_context().unwrap().subject;
    result
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(90_150),
            subject,
            "test://typed/value/data",
            ["read"],
        )))
        .unwrap();
    result
}
fn loaded(value: Value, source: &str, decoded: bool) -> (crate::MechRuntime, Arc<AtomicUsize>) {
    let reads = Arc::new(AtomicUsize::new(0));
    let mut result = configured(value.clone(), Arc::clone(&reads));
    result
        .load_source_program(source, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    if decoded {
        let ActiveProgramExecution::ResidentExternal(execution) = &result.active_program else {
            panic!()
        };
        let bytes = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();
        result = configured(value, Arc::clone(&reads));
        result
            .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)
            .unwrap();
    }
    (result, reads)
}
fn submit(runtime: &mut crate::MechRuntime, rows: usize, columns: usize) {
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            RuntimeHostInputValue::F64Matrix {
                rows,
                columns,
                values: (0..rows * columns).map(|i| i as f64 + 0.5).collect(),
            },
        ))
        .unwrap();
}
fn output(runtime: &crate::MechRuntime) -> Value {
    runtime
        .program_output_value()
        .unwrap()
        .unwrap()
        .value()
        .clone()
}
fn assert_same_output(runtime: &crate::MechRuntime, expected: &Value) {
    let actual = output(runtime);
    assert_eq!(actual.schema_key(), expected.schema_key());
    assert_eq!(actual.shape(), expected.shape());
    assert_eq!(
        actual.value_hash(&actual.schemas().unwrap()).unwrap(),
        expected.value_hash(&expected.schemas().unwrap()).unwrap()
    );
}
fn execution(runtime: &crate::MechRuntime) -> &ResidentExternalExecution {
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!()
    };
    execution
}
fn accept(runtime: &mut crate::MechRuntime, rows: usize, columns: usize) {
    submit(runtime, rows, columns);
    assert!(matches!(
        runtime.drain_resident_host_inputs(1).unwrap().turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    let actual = output(runtime);
    assert_eq!(canonical_matrix_shape(&actual), (rows, columns));
    assert_eq!(
        canonical_f64_matrix(&actual),
        (0..rows * columns)
            .map(|i| i as f64 + 0.5)
            .collect::<Vec<_>>()
    );
    let fact = &execution(runtime)
        .coordinator
        .input_facts()
        .last()
        .unwrap()
        .1
        .facts[0];
    assert_eq!(canonical_matrix_shape(&fact.value), (rows, columns));
    assert_eq!(fact.shape, *fact.value.shape());
}
fn reject_geometry(runtime: &mut crate::MechRuntime, rows: usize, columns: usize) {
    let prior = output(runtime);
    let epoch = execution(runtime).coordinator.instance().published_epoch();
    submit(runtime, rows, columns);
    assert!(runtime.drain_resident_host_inputs(1).is_err());
    assert_eq!(
        execution(runtime).coordinator.instance().published_epoch(),
        epoch
    );
    assert_same_output(runtime, &prior);
}

#[test]
fn resource_schema_planning_preserves_identity_lifetimes_and_bounds() {
    for axes in [
        [Axis::Fixed(1), Axis::Fixed(2)],
        [Axis::Fixed(1), Axis::Parameter(DimensionLifetime::Turn, 8)],
        [
            Axis::Fixed(1),
            Axis::Parameter(DimensionLifetime::Activation, 8),
        ],
        [
            Axis::Parameter(DimensionLifetime::Activation, 3),
            Axis::Parameter(DimensionLifetime::Turn, 8),
        ],
    ] {
        let planned = planned(1, 2, axes);
        for decoded in [false, true] {
            let (runtime, reads) = loaded(planned.clone(), SOURCE, decoded);
            let execution = execution(&runtime);
            assert_eq!(execution.coordinator.instance().plan.inputs.len(), 1);
            let input = &execution.coordinator.instance().plan.inputs[0];
            let actual = execution.artifact.schemas().get(input.schema).unwrap();
            let expected_schemas = planned.schemas().unwrap();
            let expected = expected_schemas.get(planned.schema()).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual.key(), planned.schema_key());
            assert_eq!(reads.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn resource_schema_turn_observations_retain_actual_shapes_in_source_and_bytecode() {
    for decoded in [false, true] {
        let (mut runtime, reads) = loaded(
            planned(
                1,
                0,
                [Axis::Fixed(1), Axis::Parameter(DimensionLifetime::Turn, 8)],
            ),
            SOURCE,
            decoded,
        );
        let baseline_reads = reads.load(Ordering::SeqCst);
        for width in [0, 3, 1, 8, 0] {
            accept(&mut runtime, 1, width);
        }
        assert_eq!(reads.load(Ordering::SeqCst), baseline_reads);
        reject_geometry(&mut runtime, 1, 9);
    }
}

#[test]
fn resource_schema_fixed_and_activation_axes_reject_geometry_changes() {
    for axes in [
        [Axis::Fixed(1), Axis::Fixed(2)],
        [
            Axis::Fixed(1),
            Axis::Parameter(DimensionLifetime::Activation, 8),
        ],
    ] {
        for decoded in [false, true] {
            let (mut runtime, _) = loaded(planned(1, 2, axes), SOURCE, decoded);
            accept(&mut runtime, 1, 2);
            reject_geometry(&mut runtime, 1, 3);
        }
    }
}

#[test]
fn resource_schema_mixed_axes_keep_activation_extent_and_allow_turn_changes() {
    for decoded in [false, true] {
        for bad_shape in [(3, 1), (2, 9)] {
            let axes = [
                Axis::Parameter(DimensionLifetime::Activation, 3),
                Axis::Parameter(DimensionLifetime::Turn, 8),
            ];
            let (mut runtime, _) = loaded(planned(2, 0, axes), SOURCE, decoded);
            for width in [0, 3, 1, 8, 0] {
                accept(&mut runtime, 2, width);
            }
            reject_geometry(&mut runtime, bad_shape.0, bad_shape.1);
        }
    }
}

#[test]
fn resource_schema_rejected_turn_keeps_published_value_and_accepts_next_shape() {
    let source = "@typed := test://typed/value{:read(data)}\nsample := @typed/data\nsafe! := sequence/length(sample) <= 3u64\nsample\n";
    for decoded in [false, true] {
        let (mut runtime, reads) = loaded(
            planned(
                1,
                0,
                [Axis::Fixed(1), Axis::Parameter(DimensionLifetime::Turn, 8)],
            ),
            source,
            decoded,
        );
        accept(&mut runtime, 1, 2);
        let prior = output(&runtime);
        let epoch = execution(&runtime).coordinator.instance().published_epoch();
        submit(&mut runtime, 1, 4);
        assert!(matches!(
            runtime.drain_resident_host_inputs(1).unwrap().turn,
            Some(crate::ResidentExternalTurnOutcome::Rejected { .. })
        ));
        assert_same_output(&runtime, &prior);
        assert_eq!(
            execution(&runtime).coordinator.instance().published_epoch(),
            epoch
        );
        let fact = &execution(&runtime)
            .coordinator
            .input_facts()
            .last()
            .unwrap()
            .1
            .facts[0];
        assert_eq!(canonical_matrix_shape(&fact.value), (1, 4));
        assert_eq!(fact.shape, *fact.value.shape());
        accept(&mut runtime, 1, 1);
        assert_eq!(reads.load(Ordering::SeqCst), 0);
    }
}

const SAMPLE_SOURCE: &str = "@typed := test://typed/value{:read(data)}\nobserved := @typed/data\ntrigger := true\n~selected := 0u64\n~> trigger { selected = sequence/length(observed) }\nselected\n";
fn sample(runtime: &mut crate::MechRuntime, width: usize) {
    submit(runtime, 1, width);
    assert!(
        runtime
            .drain_resident_host_inputs(1)
            .unwrap()
            .turn
            .is_none()
    );
}
fn assert_count(runtime: &crate::MechRuntime, expected: u64) {
    assert!(matches!(output(runtime).data(), ValueData::U64(actual) if *actual == expected));
}
fn assert_last_fact_width(runtime: &crate::MechRuntime, width: usize) {
    let fact = &execution(runtime)
        .coordinator
        .input_facts()
        .last()
        .unwrap()
        .1
        .facts[0];
    assert_eq!(canonical_matrix_shape(&fact.value), (1, width));
    assert_eq!(fact.shape, *fact.value.shape());
}
#[test]
fn resource_schema_compatible_replacement_migrates_latest_turn_geometry() {
    let planned = planned(
        1,
        0,
        [Axis::Fixed(1), Axis::Parameter(DimensionLifetime::Turn, 8)],
    );
    let (mut previous, _) = loaded(planned.clone(), SAMPLE_SOURCE, false);
    sample(&mut previous, 3);
    previous.step_active_program().unwrap();
    assert_count(&previous, 3);
    let (mut replacement, reads) = loaded(planned, SAMPLE_SOURCE, true);
    let baseline_reads = reads.load(Ordering::SeqCst);
    replacement
        .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
        .unwrap();
    // No new host packet: the replacement must use the migrated width-three
    // observation rather than its width-zero representative planning value.
    replacement.step_active_program().unwrap();
    assert_count(&replacement, 3);
    assert_last_fact_width(&replacement, 3);
    sample(&mut replacement, 1);
    replacement.step_active_program().unwrap();
    assert_count(&replacement, 1);
    assert_eq!(reads.load(Ordering::SeqCst), baseline_reads);
}

#[test]
fn resource_schema_sampled_shapes_survive_rejected_steps_and_recover() {
    let source = SAMPLE_SOURCE.replace("selected\n", "safe! := selected <= 3u64\nselected\n");
    for decoded in [false, true] {
        let (mut runtime, reads) = loaded(
            planned(
                1,
                0,
                [Axis::Fixed(1), Axis::Parameter(DimensionLifetime::Turn, 8)],
            ),
            &source,
            decoded,
        );
        let baseline_reads = reads.load(Ordering::SeqCst);
        sample(&mut runtime, 3);
        runtime.step_active_program().unwrap();
        assert_count(&runtime, 3);
        assert_last_fact_width(&runtime, 3);
        let epoch = execution(&runtime).coordinator.instance().published_epoch();
        sample(&mut runtime, 5);
        assert!(runtime.step_active_program().is_err());
        assert_count(&runtime, 3);
        assert_eq!(
            execution(&runtime).coordinator.instance().published_epoch(),
            epoch
        );
        assert_last_fact_width(&runtime, 5);
        sample(&mut runtime, 2);
        runtime.step_active_program().unwrap();
        assert_count(&runtime, 2);
        assert_last_fact_width(&runtime, 2);
        assert_eq!(reads.load(Ordering::SeqCst), baseline_reads);
    }
}

#[test]
fn resource_schema_replay_checks_actual_geometry_and_original_payload_hash() {
    let (mut runtime, _) = loaded(
        planned(
            1,
            0,
            [Axis::Fixed(1), Axis::Parameter(DimensionLifetime::Turn, 8)],
        ),
        SOURCE,
        false,
    );
    for width in [3, 1, 0] {
        accept(&mut runtime, 1, width);
    }
    let execution = execution(&runtime);
    let mut facts = mech_engine::__resident::ActivationFacts::default();
    for input in execution.coordinator.instance().plan.inputs.iter() {
        facts
            .slot_shape_seeds
            .insert(input.artifact_slot, input.shape.clone());
    }
    let instance = mech_engine::__resident::activate_external(
        execution.coordinator.instance().id,
        &execution.artifact,
        &mech_stdlib::source_catalog(),
        &facts,
        mech_engine::__resident::ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut replay = external::ResidentExternalCoordinator::new_replay(
        instance,
        Arc::clone(&execution.artifact),
        execution.coordinator.replay_bootstrap(),
        crate::ResidentDurabilityPolicy::Retained,
        external::ResidentExternalLimits::default(),
    )
    .unwrap();
    let batches = execution
        .coordinator
        .input_facts()
        .map(|(_, batch)| batch.clone())
        .collect::<Vec<_>>();
    let records = execution
        .coordinator
        .receipts()
        .map(|(_, record)| record.clone())
        .collect::<Vec<_>>();
    assert_eq!(batches.len(), 3);
    assert_eq!(records.len(), 3);
    for (batch, record) in batches.iter().zip(&records) {
        // Keep the original batch hash: replacing the fact must be rejected
        // before canonical rebinding can normalize away forged evidence.
        let mut forged = batch.clone();
        forged.facts[0].payload_hash = mech_core::ValueHash::from_bytes([91; 32]);
        assert!(replay.execute_replay_batch(Some(&forged), record).is_err());
        let mut forged = batch.clone();
        let input = &execution.coordinator.instance().plan.inputs[0];
        let schema = execution.artifact.schemas().get(input.schema).unwrap();
        let width = canonical_matrix_shape(&batch.facts[0].value).1;
        forged.facts[0].shape =
            mech_core::shape_for_resolved_extents(schema, &[1, (width + 1) as u64]).unwrap();
        assert!(replay.execute_replay_batch(Some(&forged), record).is_err());
        assert!(matches!(
            replay.execute_replay_batch(Some(batch), record).unwrap(),
            crate::ResidentExternalTurnOutcome::Accepted { .. }
        ));
    }
    assert_eq!(
        canonical_matrix_shape(&replay.instance().copied_output(0).unwrap()),
        (1, 0)
    );
}
