use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    sync::Arc,
};

use mech_compute::{
    ComputeBackendFactory, ComputeDispatchDisposition, ComputeDispatchRequest, ComputeElementType,
    ComputeInitializerSet, ComputeInputUpdate, ComputeOutputSelection, ComputeProgram,
    ComputeSession, ComputeValue, TensorLayout,
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact, decode_program_artifact_bytecode_v1,
    encode_program_artifact_bytecode_v1,
};
use mech_gpu::{
    ComputeLowerer, CpuScalarBackendFactory, CpuSimdBackendFactory, FixedShapeKernel,
    GpuExecutionBindingRole, GpuExecutionPlan, GpuKernelPlanSource,
};

fn source_artifact(source: &str) -> ProgramArtifact {
    source_artifact_with_initializer(source, "7f32")
}

fn source_artifact_with_initializer(source: &str, initializer: &str) -> ProgramArtifact {
    let source = format!(
        "@worker := compute://worker/kernel{{:write(input/signal), :write(turn)}}\n@worker/input/signal <- {initializer}\n@worker/turn <- 1\n\ncalculation @compute\n-------------------------------------------------------------------------------\n{source}"
    );
    let document = mech_runtime::SourceDocument::parse_resolved(
        "test://canonical-input-publications",
        mech_syntax::document::Revision(0),
        Arc::<str>::from(source),
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{:?}",
        document.snapshot().diagnostics
    );
    mech_runtime::RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_mixed_document(&document)
        .unwrap()
        .compute
        .artifact
}

#[test]
fn published_input_is_current_before_first_public_scalar_dispatch() {
    let artifact =
        source_artifact("signal := 7f32\n~total := 1f32\ntotal = total + signal\nsignal\n");
    let inputs = BTreeMap::from([("signal".to_owned(), vec![7.0; 5])]);
    let kernel = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap();
    assert_eq!(kernel.instances(), 5);
    let program = kernel.compute_program();
    assert_eq!(program.interface().outputs.len(), 1);
    assert_eq!(program.interface().outputs[0].name.as_ref(), "result");
    let port = program.interface().input_named("signal").unwrap();
    assert_eq!(program.interface().outputs[0].slot, port.slot);
    assert_eq!(program.interface().outputs[0].schema, port.schema);
    let initializers =
        ComputeInitializerSet::new(BTreeMap::from([(port.id, ComputeValue::ScalarF32(7.0))]));
    let executable = CpuScalarBackendFactory::new().compile(program).unwrap();
    let mut session = executable.create_session(&initializers).unwrap();
    let snapshot = session.read_outputs(&ComputeOutputSelection::All).unwrap();
    let value = &snapshot.values[&program.interface().outputs[0].id];
    let ComputeValue::TensorF32 {
        dimensions,
        layout,
        values,
    } = value
    else {
        panic!("expected a complete batched output, got {value:?}");
    };
    assert_eq!(dimensions.as_ref(), &[5]);
    assert_eq!(*layout, TensorLayout::RowMajor);
    assert_eq!(values.as_ref(), &[7.0; 5]);
}

fn representations(artifact: ProgramArtifact) -> [ProgramArtifact; 2] {
    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}

fn state_owner_artifact(source: &str) -> ProgramArtifact {
    let document = mech_runtime::SourceDocument::parse_resolved(
        "test://canonical-state-publications",
        mech_syntax::document::Revision(0),
        source,
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap();
    assert!(document.is_strictly_clean());
    CanonicalSourceFrontend
        .compile_interactive_document_with_planning_contract(
            &document.document(),
            mech_stdlib::source_native_plan_catalog(),
            Default::default(),
            Default::default(),
            &BTreeSet::from(["signal".to_owned()]),
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
        .unwrap()
        .project_compute_output_paths(&BTreeSet::from([
            "candidate".to_owned(),
            "first".to_owned(),
            "second".to_owned(),
            "signal".to_owned(),
        ]))
        .unwrap()
        .compile_artifact()
        .unwrap()
}

fn tensor(dimensions: &[u64], values: Vec<f32>) -> ComputeValue {
    ComputeValue::TensorF32 {
        dimensions: dimensions.into(),
        layout: TensorLayout::RowMajor,
        values: values.into(),
    }
}

fn initialize(program: &ComputeProgram, value: ComputeValue) -> ComputeInitializerSet {
    ComputeInitializerSet::new(BTreeMap::from([(
        program.interface().input_named("signal").unwrap().id,
        value,
    )]))
}

fn update(session: &mut dyn ComputeSession, program: &ComputeProgram, value: ComputeValue) {
    session
        .update_inputs(&[ComputeInputUpdate {
            port: program.interface().input_named("signal").unwrap().id,
            value,
        }])
        .unwrap();
}

// Check the whole public value and the last SIMD-tail lane, not just population
// or a matching unnamed physical buffer.
fn assert_outputs(
    session: &mut dyn ComputeSession,
    program: &ComputeProgram,
    expected: &[(&str, &[u64], Vec<f32>)],
) {
    let snapshot = session.read_outputs(&ComputeOutputSelection::All).unwrap();
    assert_eq!(snapshot.values.len(), expected.len());
    assert_eq!(program.interface().outputs.len(), expected.len());
    for (port, (name, inner_dimensions, values)) in program.interface().outputs.iter().zip(expected)
    {
        assert_eq!(port.name.as_ref(), *name);
        assert_eq!(port.element, ComputeElementType::F32);
        assert_eq!(port.dimensions.as_ref(), *inner_dimensions);
        let mut dimensions = vec![5];
        dimensions.extend_from_slice(inner_dimensions);
        assert_eq!(
            snapshot.values[&port.id],
            tensor(&dimensions, values.clone()),
            "public output {name} from slot {:?}",
            port.slot,
        );
        let sample = session
            .read_outputs(&ComputeOutputSelection::Samples {
                ports: BTreeSet::from([port.id]),
                instance: 4,
            })
            .unwrap();
        assert_eq!(sample.values.len(), 1);
        let elements = values.len() / 5;
        let sample_values = values[4 * elements..].to_vec();
        let expected_sample = if inner_dimensions.is_empty() {
            assert_eq!(sample_values.len(), 1);
            ComputeValue::ScalarF32(sample_values[0])
        } else {
            tensor(inner_dimensions, sample_values)
        };
        assert_eq!(sample.values[&port.id], expected_sample);
    }
    let selected = program.interface().outputs.last().unwrap();
    let selection = session
        .read_outputs(&ComputeOutputSelection::Ports(BTreeSet::from(
            [selected.id],
        )))
        .unwrap();
    assert_eq!(selection.values.len(), 1);
    assert_eq!(
        selection.values[&selected.id],
        snapshot.values[&selected.id]
    );
}

fn assert_input_publication_plan(
    kernel: &FixedShapeKernel,
    inputs: &BTreeMap<String, Vec<f32>>,
    names: &[&str],
    inner_dimensions: &[u64],
) {
    let program = kernel.compute_program();
    let input = program.interface().input_named("signal").unwrap();
    let storage = program.fixed_shape_storage().unwrap();
    assert_eq!(kernel.instances(), 5);
    assert_eq!(storage.instances, 5);
    assert_eq!(storage.inputs.len(), 1);
    assert_eq!(storage.states.len(), 1);
    assert!(storage.publications.is_empty());
    assert_eq!(storage.inputs[0].slot, input.slot);
    assert!(storage.states.iter().all(|state| state.slot != input.slot));
    assert_eq!(input.dimensions.as_ref(), inner_dimensions);
    assert_eq!(program.interface().outputs.len(), names.len());
    for (output, name) in program.interface().outputs.iter().zip(names) {
        assert_eq!(output.name.as_ref(), *name);
        assert_eq!(output.slot, input.slot);
        assert_eq!(output.schema, input.schema);
        assert_eq!(output.dimensions.as_ref(), inner_dimensions);
    }
    let execution =
        GpuExecutionPlan::build(GpuKernelPlanSource::FixedShape(kernel), inputs).unwrap();
    assert_eq!(execution.states.len(), 1);
    assert!(
        execution
            .states
            .iter()
            .all(|state| state.slot != input.slot.0)
    );
    assert_eq!(execution.physical_outputs.len(), 1);
    let physical = &execution.physical_outputs[0];
    assert_eq!(physical.slot, input.slot.0);
    let binding = execution
        .bindings
        .iter()
        .find(|binding| Some(binding.binding) == physical.binding)
        .unwrap();
    assert_eq!(binding.role, GpuExecutionBindingRole::Input);
    assert_eq!(binding.slot, input.slot.0);
    assert_eq!(physical.aliases, names);
    assert_eq!(execution.outputs.len(), names.len());
    for (output, name) in execution.outputs.iter().zip(names) {
        assert_eq!(output.name, *name);
        assert_eq!(output.physical_output, physical.id);
    }
    let decoded: GpuExecutionPlan =
        serde_json::from_str(&serde_json::to_string(&execution).unwrap()).unwrap();
    assert_eq!(decoded, execution);
    decoded.validate().unwrap();
}

fn scalar_publication_forms(factory: &dyn ComputeBackendFactory) {
    for (publication, names) in [
        ("signal", vec!["result"]),
        ("alias := signal\nalias", vec!["result"]),
        ("(signal, signal)", vec!["result.0", "result.1"]),
        (
            "(signal, (signal, signal))",
            vec!["result.0", "result.1.0", "result.1.1"],
        ),
        (
            "alias := signal\npublished := ((alias, signal), alias)\npublished",
            vec!["result.0.0", "result.0.1", "result.1"],
        ),
    ] {
        // The input is deliberately not consumed by this unrelated recurrence.
        // It must still be live/current publication data, not a zero state.
        let source =
            format!("signal := 7f32\n~total := 1f32\ntotal = total + 1f32\n{publication}\n");
        for artifact in representations(source_artifact(&source)) {
            let inputs = BTreeMap::from([("signal".to_owned(), vec![7.0; 5])]);
            let kernel = ComputeLowerer
                .compile_broadcast(&artifact, &inputs)
                .unwrap();
            assert_input_publication_plan(&kernel, &inputs, &names, &[]);
            let program = kernel.compute_program();
            let executable = factory.compile(program).unwrap();
            let mut session = executable
                .create_session(&initialize(program, ComputeValue::ScalarF32(7.0)))
                .unwrap();
            let expected = |values: &[f32]| {
                names
                    .iter()
                    .map(|name| (*name, &[][..], values.to_vec()))
                    .collect::<Vec<_>>()
            };
            assert_outputs(session.as_mut(), program, &expected(&[7.0; 5]));
            let current = [11.0, 13.0, 17.0, 19.0, 23.0];
            update(session.as_mut(), program, tensor(&[5], current.to_vec()));
            assert_outputs(session.as_mut(), program, &expected(&current));
            let report = session
                .dispatch(&ComputeDispatchRequest::default())
                .unwrap();
            assert_eq!(report.disposition, ComputeDispatchDisposition::Completed);
            assert_eq!(report.completed_turns, 1);
            assert_outputs(session.as_mut(), program, &expected(&current));
        }
    }
}

#[test]
fn scalar_publication_forms_use_current_input_source_and_decoded_artifact() {
    scalar_publication_forms(&CpuScalarBackendFactory::new());
}

#[test]
fn simd_publication_forms_use_current_input_source_and_decoded_artifact() {
    scalar_publication_forms(&CpuSimdBackendFactory::new());
}

fn distinct_initializer(factory: &dyn ComputeBackendFactory) {
    let artifact = source_artifact(
        "signal := 7f32\n~total := 1f32\ntotal = total + signal\n(signal, total)\n",
    );
    for artifact in representations(artifact) {
        let current = vec![2.0, 3.0, 5.0, 7.0, 11.0];
        let kernel = ComputeLowerer
            .compile_broadcast(
                &artifact,
                &BTreeMap::from([("signal".to_owned(), current.clone())]),
            )
            .unwrap();
        let program = kernel.compute_program();
        let executable = factory.compile(program).unwrap();
        let mut session = executable
            .create_session(&initialize(program, tensor(&[5], current.clone())))
            .unwrap();
        assert_outputs(
            session.as_mut(),
            program,
            &[
                ("result.0", &[], current.clone()),
                // Mixed-document statement bindings expose the updated
                // expression here, not the recurrence owner's prior state.
                ("result.1", &[], vec![0.0; 5]),
            ],
        );
        let report = session
            .dispatch(&ComputeDispatchRequest::default())
            .unwrap();
        assert_eq!(report.disposition, ComputeDispatchDisposition::Completed);
        assert_outputs(
            session.as_mut(),
            program,
            &[
                ("result.0", &[], current.clone()),
                (
                    "result.1",
                    &[],
                    current.iter().map(|value| 1.0 + value).collect(),
                ),
            ],
        );
    }
}

#[test]
fn scalar_public_initializer_accepts_distinct_inferred_batch_lanes() {
    distinct_initializer(&CpuScalarBackendFactory::new());
}

#[test]
fn simd_public_initializer_accepts_distinct_inferred_batch_lanes() {
    distinct_initializer(&CpuSimdBackendFactory::new());
}

fn publication_lifecycle(factory: &dyn ComputeBackendFactory) {
    let artifact = state_owner_artifact(
        "signal := 7f32\n~first := 1f32\n~second := 9f32\ncandidate := first + signal\nsecond = first\nfirst = candidate\nbounded! := candidate < 100f32\n",
    );
    for artifact in representations(artifact) {
        // Inferred batches are covered by the mixed-source fixtures above;
        // interactive host-path projection retains actual state-owner ports.
        let kernel = ComputeLowerer.compile_batched(&artifact, 5).unwrap();
        let program = kernel.compute_program();
        let storage = program.fixed_shape_storage().unwrap();
        assert_eq!(storage.states.len(), 2);
        assert_eq!(storage.publications.len(), 1);
        assert_eq!(program.interface().outputs[1].slot, storage.states[0].slot);
        assert_eq!(program.interface().outputs[2].slot, storage.states[1].slot);
        assert_eq!(storage.states[0].initializer.as_ref(), &[1.0]);
        assert_eq!(storage.states[1].initializer.as_ref(), &[9.0]);
        let input = program.interface().input_named("signal").unwrap();
        assert_eq!(program.interface().outputs[3].slot, input.slot);
        let executable = factory.compile(program).unwrap();
        let mut session = executable
            .create_session(&initialize(program, ComputeValue::ScalarF32(7.0)))
            .unwrap();
        let expected = |input: &[f32], first: &[f32], second: &[f32], candidate: &[f32]| {
            vec![
                ("candidate", &[][..], candidate.to_vec()),
                ("first", &[][..], first.to_vec()),
                ("second", &[][..], second.to_vec()),
                ("signal", &[][..], input.to_vec()),
            ]
        };
        // Preparation and input updates do not secretly execute a turn.
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&[7.0; 5], &[1.0; 5], &[9.0; 5], &[0.0; 5]),
        );
        let current = [11.0, 13.0, 17.0, 19.0, 23.0];
        update(session.as_mut(), program, tensor(&[5], current.to_vec()));
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&current, &[1.0; 5], &[9.0; 5], &[0.0; 5]),
        );
        assert!(
            session
                .update_inputs(&[ComputeInputUpdate {
                    port: input.id,
                    value: tensor(&[4], vec![99.0; 4]),
                }])
                .is_err()
        );
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&current, &[1.0; 5], &[9.0; 5], &[0.0; 5]),
        );
        let report = session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::MIN))
            .unwrap();
        assert_eq!(report.disposition, ComputeDispatchDisposition::Completed);
        assert_eq!(report.completed_turns, 1);
        let first = [12.0, 14.0, 18.0, 20.0, 24.0];
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&current, &first, &[1.0; 5], &first),
        );
        update(session.as_mut(), program, ComputeValue::ScalarF32(2.0));
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&[2.0; 5], &first, &[1.0; 5], &first),
        );
        let report = session
            .dispatch(&ComputeDispatchRequest::default())
            .unwrap();
        assert_eq!(report.disposition, ComputeDispatchDisposition::Completed);
        let committed = [14.0, 16.0, 20.0, 22.0, 26.0];
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&[2.0; 5], &committed, &first, &committed),
        );
        let invalid = [2.0, 2.0, 2.0, 2.0, 1000.0];
        update(session.as_mut(), program, tensor(&[5], invalid.to_vec()));
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&invalid, &committed, &first, &committed),
        );
        let rejected = session
            .dispatch(&ComputeDispatchRequest::default())
            .unwrap();
        assert_eq!(rejected.disposition, ComputeDispatchDisposition::Rejected);
        assert_eq!(rejected.completed_turns, 0);
        assert_eq!(rejected.fault_count, 1);
        let fault = rejected.last_fault.unwrap();
        assert_eq!(fault.constraint.as_ref(), "bounded!");
        assert_eq!(
            fault.detail.as_ref(),
            "candidate rejected at batch instance 4"
        );
        // Latest accepted input is current even when candidate state is rejected.
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&invalid, &committed, &first, &committed),
        );
        update(session.as_mut(), program, ComputeValue::ScalarF32(1.0));
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&[1.0; 5], &committed, &first, &committed),
        );
        let recovered = session
            .dispatch(&ComputeDispatchRequest::default())
            .unwrap();
        assert_eq!(recovered.disposition, ComputeDispatchDisposition::Completed);
        assert_eq!(recovered.completed_turns, 1);
        assert_eq!(recovered.fault_count, 1);
        let final_state = [15.0, 17.0, 21.0, 23.0, 27.0];
        assert_outputs(
            session.as_mut(),
            program,
            &expected(&[1.0; 5], &final_state, &committed, &final_state),
        );
    }
}

#[test]
fn scalar_publications_preserve_input_and_committed_state_generations() {
    publication_lifecycle(&CpuScalarBackendFactory::new());
}

#[test]
fn simd_publications_preserve_input_and_committed_state_generations() {
    publication_lifecycle(&CpuSimdBackendFactory::new());
}

#[cfg(feature = "jit")]
#[test]
fn jit_publications_preserve_input_and_committed_state_generations() {
    publication_lifecycle(&mech_gpu::CpuJitBackendFactory::new());
}

fn matrix_publication(factory: &dyn ComputeBackendFactory) {
    let initializer = "[1f32 2f32 3f32; 4f32 5f32 6f32]";
    let source = format!(
        "signal := {initializer}\n~total := 1f32\ntotal = total + 1f32\n(signal, signal)\n"
    );
    let artifact = source_artifact_with_initializer(&source, initializer);
    for artifact in representations(artifact) {
        let current = (0..5)
            .flat_map(|lane| (1..=6).map(move |entry| (lane * 10 + entry) as f32))
            .collect::<Vec<_>>();
        // Lowerer's physical matrix inputs are per-lane column-major. Public
        // initializers/updates below are deliberately ordinary row-major.
        let physical = current
            .chunks_exact(6)
            .flat_map(|lane| [lane[0], lane[3], lane[1], lane[4], lane[2], lane[5]])
            .collect::<Vec<_>>();
        let inputs = BTreeMap::from([("signal".to_owned(), physical)]);
        let kernel = ComputeLowerer
            .compile_broadcast(&artifact, &inputs)
            .unwrap();
        assert_input_publication_plan(&kernel, &inputs, &["result.0", "result.1"], &[2, 3]);
        let program = kernel.compute_program();
        let executable = factory.compile(program).unwrap();
        let mut session = executable
            .create_session(&initialize(program, tensor(&[5, 2, 3], current.clone())))
            .unwrap();
        let expected = |values: &[f32]| {
            vec![
                ("result.0", &[2, 3][..], values.to_vec()),
                ("result.1", &[2, 3][..], values.to_vec()),
            ]
        };
        assert_outputs(session.as_mut(), program, &expected(&current));
        let next = current
            .iter()
            .map(|value| value + 100.0)
            .collect::<Vec<_>>();
        update(session.as_mut(), program, tensor(&[5, 2, 3], next.clone()));
        assert_outputs(session.as_mut(), program, &expected(&next));
        let report = session
            .dispatch(&ComputeDispatchRequest::default())
            .unwrap();
        assert_eq!(report.disposition, ComputeDispatchDisposition::Completed);
        assert_outputs(session.as_mut(), program, &expected(&next));
    }
}

#[test]
fn scalar_matrix_input_publication_preserves_inner_axes_and_batch_lanes() {
    matrix_publication(&CpuScalarBackendFactory::new());
}

#[test]
fn simd_matrix_input_publication_preserves_inner_axes_and_batch_lanes() {
    matrix_publication(&CpuSimdBackendFactory::new());
}

#[cfg(feature = "native")]
#[test]
fn native_gpu_publications_use_current_input_and_transactional_state() {
    let factory = mech_gpu::WgpuBackendFactory::new();
    let artifact =
        source_artifact("signal := 7f32\n~total := 1f32\ntotal = total + signal\nsignal\n");
    let kernel = ComputeLowerer
        .compile_broadcast(
            &artifact,
            &BTreeMap::from([("signal".to_owned(), vec![7.0; 5])]),
        )
        .unwrap();
    if let Err(error) = factory.supports(kernel.compute_program()) {
        if error.reason.contains("adapter")
            && std::env::var("MECH_REQUIRE_GPU").as_deref() != Ok("1")
        {
            eprintln!("SKIPPED native publication device coverage: {error}");
            return;
        }
        panic!("native publication coverage requires actual wgpu execution: {error}");
    }
    assert_eq!(factory.descriptor().id.as_str(), "wgpu");
    scalar_publication_forms(&factory);
    distinct_initializer(&factory);
    publication_lifecycle(&factory);
    matrix_publication(&factory);
}
