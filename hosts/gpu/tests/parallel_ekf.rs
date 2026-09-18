use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    sync::Arc,
};

use mech_compute::{
    BackendRequest, ComputeDispatchDisposition, ComputeDispatchRequest, ComputeInitializerSet,
    ComputeInputUpdate, ComputeOutputSelection, ComputePlatform, ComputeValue, TensorLayout,
};
use mech_core::ComputePlacement;
use mech_engine::{
    ProgramArtifact, decode_program_artifact_sections, encode_program_artifact_sections,
};
use mech_gpu::{
    BatchedExecutionError, ComputeLowerer, FixedShapeKernel, GpuExecutionBindingRole,
    GpuExecutionPlan, GpuKernelPlanSource, GpuPlanKernelKind, native_compute_backend_registry,
};
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

const SOURCE: &str = include_str!("../fixtures/ekf-kernel.mec");
const DRIVER_INPUT_NAMES: [&str; 7] = [
    "lane-dt",
    "lane-linear-velocity",
    "lane-angular-velocity",
    "lane-bearing",
    "lane-measurement-noise",
    "finite-limit",
    "covariance-symmetry-tolerance",
];
struct SourceFixture {
    driver: SourceDocument,
    compute: SourceDocument,
    mixed: SourceDocument,
}

fn source_tree(instances: usize) -> SourceFixture {
    source_tree_from(SOURCE, instances)
}

fn source_tree_from(source: &str, instances: usize) -> SourceFixture {
    let source = source.replacen("100000f32", &format!("{instances}f32"), 1);
    let compute_start = source
        .find("2. EKF step @compute\n")
        .expect("EKF source must contain its compute section");
    let driver_source = &source[..compute_start];
    let compute_source = format!(
        "Portable EKF Compute\n===============================================================================\n+> math\n===============================================================================\n\n{}",
        &source[compute_start..]
    );
    let mixed_source = format!(
        "{}\n@worker := compute://worker/kernel{{:write(input/dt), :write(input/linear-velocity), :write(input/angular-velocity), :write(input/bearing), :write(input/measurement-noise), :write(turn)}}\n\
         @worker/input/dt <- lane-dt\n\
         @worker/input/linear-velocity <- lane-linear-velocity\n\
         @worker/input/angular-velocity <- lane-angular-velocity\n\
         @worker/input/bearing <- lane-bearing\n\
         @worker/input/measurement-noise <- lane-measurement-noise\n\
         @worker/turn <- 1\n\n{}",
        driver_source,
        &source[compute_start..]
    );
    let parse = |uri, source| {
        let document =
            SourceDocument::parse_resolved(uri, Revision(0), source, ParseConfig::default())
                .expect("EKF retained document must parse");
        assert!(
            document.is_strictly_clean(),
            "{uri}: {:#?}",
            document.snapshot().diagnostics
        );
        document
    };
    SourceFixture {
        driver: parse("test://ekf-driver", driver_source),
        compute: parse("test://ekf-compute", &compute_source),
        mixed: parse("test://ekf-mixed", &mixed_source),
    }
}

fn evaluate_driver(tree: &SourceFixture) -> BTreeMap<String, Vec<f32>> {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .evaluate_static_document_symbols(&tree.driver, &DRIVER_INPUT_NAMES)
        .expect("Mech EKF arrays must evaluate")
        .into_iter()
        .map(|(name, value)| {
            let values = match value {
                RuntimeHostInputValue::F32(value) => vec![value],
                RuntimeHostInputValue::F32Matrix { values, .. } => values,
                value => panic!("{name} must contain f32 values, found {value:?}"),
            };
            (name, values)
        })
        .collect()
}

fn compile_compute(tree: &SourceFixture, _driver: &BTreeMap<String, Vec<f32>>) -> ProgramArtifact {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .compile_mixed_document(&tree.mixed)
        .expect("ordinary high-level EKF source must compile")
        .compute
        .artifact
}

fn source_inputs(
    driver: &BTreeMap<String, Vec<f32>>,
    artifact: &ProgramArtifact,
) -> BTreeMap<String, Vec<f32>> {
    artifact
        .inputs()
        .iter()
        .filter_map(|input| {
            let source_name = mech_engine::decode_source_input_name(&input.name)
                .unwrap_or_else(|| input.name.clone());
            let driver_name = match source_name.as_str() {
                "dt" => "lane-dt",
                "linear-velocity" => "lane-linear-velocity",
                "angular-velocity" => "lane-angular-velocity",
                "bearing" => "lane-bearing",
                "measurement-noise" => "lane-measurement-noise",
                name => name,
            };
            driver
                .get(driver_name)
                .map(|values| (source_name, values.clone()))
        })
        .collect()
}

fn source_evaluated_outputs(
    tree: &SourceFixture,
    driver: &BTreeMap<String, Vec<f32>>,
) -> (Vec<f32>, Vec<f32>) {
    let outputs =
        source_evaluated_values(tree, driver, &["corrected-state", "corrected-covariance"]);
    (
        outputs["corrected-state"].clone(),
        outputs["corrected-covariance"].clone(),
    )
}

fn source_evaluated_values(
    tree: &SourceFixture,
    driver: &BTreeMap<String, Vec<f32>>,
    names: &[&str],
) -> BTreeMap<String, Vec<f32>> {
    let inputs = driver
        .iter()
        .map(|(name, values)| {
            (
                name.clone(),
                RuntimeHostInputValue::F32(*values.first().expect("driver input is non-empty")),
            )
        })
        .collect();
    let outputs = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .evaluate_static_document_symbols_with_inputs(&tree.compute, &inputs, names)
        .expect("ordinary high-level EKF source must evaluate");
    outputs
        .into_iter()
        .map(|(name, value)| {
            let values = match value {
                RuntimeHostInputValue::F32(value) => vec![value],
                RuntimeHostInputValue::F32Matrix { values, .. } => values,
                value => panic!("{name} must contain f32 values, found {value:?}"),
            };
            (name, values)
        })
        .collect()
}

fn lowered_states_after_one_turn(
    tree: &SourceFixture,
    driver: &BTreeMap<String, Vec<f32>>,
) -> BTreeMap<usize, Vec<f32>> {
    let artifact = compile_compute(tree, driver);
    let inputs = source_inputs(driver, &artifact);
    let lowered = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .expect("generic fixed-shape operations must lower");
    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(1).unwrap();
    lowered
        .state_layout()
        .map(|(slot, elements)| (elements, cpu.state()[&slot].clone()))
        .collect()
}

fn canonical_mixed_artifact(uri: &str, source: &str) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(uri, Revision(0), source, ParseConfig::default())
        .expect("canonical mixed fixture must parse");
    assert!(
        document.is_strictly_clean(),
        "{:#?}",
        document.snapshot().diagnostics
    );
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_mixed_document(&document)
        .unwrap()
        .compute
        .artifact
}

#[test]
fn generic_lowering_matches_source_at_prediction_boundary() {
    let source = SOURCE
        .replacen("state = corrected-state", "state = predicted-state", 1)
        .replacen(
            "covariance = corrected-covariance",
            "covariance = predicted-covariance",
            1,
        );
    let tree = source_tree_from(&source, 1);
    let driver = evaluate_driver(&tree);
    let predicted =
        source_evaluated_values(&tree, &driver, &["predicted-state", "predicted-covariance"]);
    let lowered = lowered_states_after_one_turn(&tree, &driver);
    assert_close(&predicted["predicted-state"], &lowered[&3], 2.0e-5);
    assert_close(&predicted["predicted-covariance"], &lowered[&9], 2.0e-4);
}

#[test]
fn generic_lowering_matches_source_gain() {
    let source = SOURCE.replacen("state = corrected-state", "state = gain", 1);
    let tree = source_tree_from(&source, 1);
    let driver = evaluate_driver(&tree);
    let expected = source_evaluated_values(&tree, &driver, &["gain"]);
    let lowered = lowered_states_after_one_turn(&tree, &driver);
    assert_close(&expected["gain"], &lowered[&3], 2.0e-5);
}

#[test]
fn generic_lowering_matches_source_landmark_delta() {
    let source = SOURCE.replacen(
        "state = corrected-state",
        "landmark-diagnostic := [delta-x; delta-y; squared-range]\nstate = landmark-diagnostic",
        1,
    );
    let tree = source_tree_from(&source, 1);
    let driver = evaluate_driver(&tree);
    let expected = source_evaluated_values(&tree, &driver, &["landmark-diagnostic"]);
    let lowered = lowered_states_after_one_turn(&tree, &driver);
    assert_close(&expected["landmark-diagnostic"], &lowered[&3], 2.0e-5);
}

#[test]
fn rectangular_matrix_literals_cross_the_artifact_layout_boundary_correctly() {
    let artifact = canonical_mixed_artifact(
        "test://rectangular-projection",
        r#"GPU rectangular projection
===============================================================================
@worker := compute://worker/kernel{:write(input/input), :write(turn)}
@worker/input/input <- [1f32; 10f32; 100f32]
@worker/turn <- 1

rectangular projection @compute
-------------------------------------------------------------------------------
input := [1f32; 10f32; 100f32]
projection := [1f32 2f32 3f32
               4f32 5f32 6f32]
~result := [0f32; 0f32]
result = projection ** input
result
"#,
    );
    let activation_inputs = BTreeMap::from([("input".to_owned(), vec![1.0, 10.0, 100.0])]);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &activation_inputs)
        .unwrap();
    let initializers = compute_initializers(&program, &activation_inputs);
    let input_port = program
        .compute_program()
        .interface()
        .input_named("input")
        .unwrap()
        .id;
    let registry = native_compute_backend_registry();

    let mut backends = vec!["cpu-scalar", "cpu-simd"];
    if cfg!(feature = "jit") {
        backends.push("cpu-jit");
    }
    backends.push("wgpu");
    for backend in backends {
        let request = BackendRequest::parse(backend).unwrap();
        let factory = match registry.resolve(
            &request,
            ComputePlatform::Native,
            ComputePlacement::Compute,
            program.compute_program(),
        ) {
            Ok(factory) => factory,
            Err(error) if backend == "wgpu" && error.to_string().contains("adapter") => continue,
            Err(error) => panic!("{backend} rejected the rectangular program: {error}"),
        };
        let executable = factory.compile(program.compute_program()).unwrap();
        let mut session = executable.create_session(&initializers).unwrap();
        session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
            .unwrap();
        let initial =
            flattened_outputs(&session.read_outputs(&ComputeOutputSelection::All).unwrap());
        assert!(
            initial.values().any(|values| {
                values.len() == 2
                    && (values[0] - 321.0).abs() < 1.0e-6
                    && (values[1] - 654.0).abs() < 1.0e-6
            }),
            "{backend} did not publish the expected rectangular result: {initial:?}",
        );

        session
            .update_inputs(&[ComputeInputUpdate {
                port: input_port,
                value: ComputeValue::TensorF32 {
                    dimensions: vec![3, 1].into_boxed_slice(),
                    layout: TensorLayout::RowMajor,
                    values: Arc::from([2.0, 20.0, 200.0]),
                },
            }])
            .unwrap();
        session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
            .unwrap();
        let updated =
            flattened_outputs(&session.read_outputs(&ComputeOutputSelection::All).unwrap());
        assert!(
            updated.values().any(|values| {
                values.len() == 2
                    && (values[0] - 642.0).abs() < 1.0e-6
                    && (values[1] - 1308.0).abs() < 1.0e-6
            }),
            "{backend} did not preserve state across the rectangular live update: {updated:?}",
        );
    }
}

#[test]
fn matrix_right_hand_side_solve_is_portable_across_compute_backends() {
    let artifact = canonical_mixed_artifact(
        "test://portable-matrix-solve",
        r#"GPU portable matrix solve
===============================================================================
@worker := compute://worker/kernel{:write(input/coefficients), :write(input/rhs), :write(turn)}
@worker/input/coefficients <- [4f32 1f32; 2f32 3f32]
@worker/input/rhs <- [9f32 8f32 1f32; 7f32 5f32 2f32]
@worker/turn <- 1

portable matrix solve @compute
-------------------------------------------------------------------------------
coefficients := [4f32 1f32; 2f32 3f32]
rhs := [9f32 8f32 1f32; 7f32 5f32 2f32]
~result := [0f32 0f32 0f32
            0f32 0f32 0f32]
result = coefficients \ rhs
result
"#,
    );
    assert!(artifact.nodes().iter().any(|node| {
        node.as_operation()
            .expect("ordinary fixture operation")
            .operation
            .module_path
            .as_ref()
            == ["matrix"]
            && node
                .as_operation()
                .expect("ordinary fixture operation")
                .operation
                .operation_name
                == "solve"
    }));
    let activation_inputs = BTreeMap::from([
        ("coefficients".to_owned(), vec![4.0, 2.0, 1.0, 3.0]),
        ("rhs".to_owned(), vec![9.0, 8.0, 1.0, 7.0, 5.0, 2.0]),
    ]);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &activation_inputs)
        .unwrap();
    let initializers = compute_initializers(&program, &activation_inputs);
    let registry = native_compute_backend_registry();
    let expected = [1.9, -0.4, 1.3, 1.4, 2.6, -0.2];

    let mut backends = vec!["cpu-scalar", "cpu-simd"];
    if cfg!(feature = "jit") {
        backends.push("cpu-jit");
    }
    backends.push("wgpu");
    for backend in backends {
        let request = BackendRequest::parse(backend).unwrap();
        let factory = match registry.resolve(
            &request,
            ComputePlatform::Native,
            ComputePlacement::Compute,
            program.compute_program(),
        ) {
            Ok(factory) => factory,
            Err(error) if backend == "wgpu" && error.to_string().contains("adapter") => continue,
            Err(error) => panic!("{backend} rejected the matrix solve program: {error}"),
        };
        let executable = factory.compile(program.compute_program()).unwrap();
        let mut session = executable.create_session(&initializers).unwrap();
        session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
            .unwrap();
        let outputs =
            flattened_outputs(&session.read_outputs(&ComputeOutputSelection::All).unwrap());
        assert!(
            outputs.values().any(|values| values.len() == expected.len()
                && values
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| (actual - expected).abs() < 1.0e-5)),
            "{backend} did not publish the expected matrix solution: {outputs:?}",
        );
    }
}

fn source_program(instances: usize) -> (FixedShapeKernel, BTreeMap<String, Vec<f32>>) {
    source_program_from(SOURCE, instances)
}

fn source_program_from(
    source: &str,
    instances: usize,
) -> (FixedShapeKernel, BTreeMap<String, Vec<f32>>) {
    let tree = source_tree_from(source, instances);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .expect("generic fixed-shape operations must lower");
    (program, inputs)
}

#[test]
fn fixed_shape_physical_plan_expands_one_thousand_lane_resident_buffers() {
    let instances = 1_000;
    let (program, inputs) = source_program(instances);
    let physical_inputs = program.physical_inputs(&inputs).unwrap();
    let physical_states = program.physical_states();

    assert_eq!(program.instances(), instances as u32);
    assert!(
        physical_inputs
            .iter()
            .all(|input| input.elements == instances)
    );
    // Two recurrence buffers and their two derived output publications share
    // the same transactional ping-pong plan.
    assert_eq!(physical_states.len(), 4);
    assert_eq!(
        physical_states
            .iter()
            .filter(|state| state.elements_per_instance == 3 && state.elements == instances * 3)
            .count(),
        2,
    );
    assert_eq!(
        physical_states
            .iter()
            .filter(|state| state.elements_per_instance == 9 && state.elements == instances * 9)
            .count(),
        2,
    );
    let bindings = physical_inputs
        .iter()
        .map(|input| input.binding)
        .chain(
            physical_states
                .iter()
                .flat_map(|state| [state.read_binding, state.write_binding]),
        )
        .chain(program.integrity_buffer().map(|fault| fault.binding))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        bindings.len(),
        physical_inputs.len() + physical_states.len() * 2 + 1,
        "every browser/native fixed-shape buffer must have one unique binding",
    );
    assert_eq!(program.integrity_buffer().unwrap().words, 2);

    let plan = GpuExecutionPlan::build(GpuKernelPlanSource::FixedShape(&program), &inputs)
        .expect("EKF kernel must produce one portable physical GPU plan");
    assert_eq!(plan.kernel_kind, GpuPlanKernelKind::FixedShape);
    assert_eq!(plan.dispatch_elements, instances as u32);
    assert_eq!(plan.bindings.len(), bindings.len());
    assert_eq!(plan.states.len(), 4);
    assert_eq!(plan.constraints.len(), 3);
    assert_eq!(
        plan.bindings
            .iter()
            .filter(|binding| binding.role == GpuExecutionBindingRole::IntegrityFault)
            .map(|binding| binding.elements)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(
        plan.physical_outputs
            .iter()
            .all(|output| output.binding.is_none()),
        "fixed-shape outputs must read from the selected resident-state generation"
    );
}

#[test]
fn fixed_shape_execution_plan_groups_aliased_state_outputs_once() {
    let source = SOURCE.replacen("(state, covariance)", "(state, state, covariance)", 1);
    let (program, inputs) = source_program_from(&source, 8);
    let plan = GpuExecutionPlan::build(GpuKernelPlanSource::FixedShape(&program), &inputs)
        .expect("aliased EKF outputs must produce a physical execution plan");

    assert_eq!(plan.outputs.len(), 3);
    assert_eq!(plan.physical_outputs.len(), 2);
    assert!(
        plan.physical_outputs
            .iter()
            .any(|output| output.aliases.len() == 2),
        "logical state aliases must share one physical resident readback"
    );
}

fn compute_initializers(
    program: &FixedShapeKernel,
    inputs: &BTreeMap<String, Vec<f32>>,
) -> ComputeInitializerSet {
    ComputeInitializerSet::new(
        program
            .compute_program()
            .interface()
            .inputs
            .iter()
            .map(|port| {
                let values = &inputs[port.name.as_ref()];
                let value = if port.dimensions.is_empty() {
                    ComputeValue::ScalarF32(values[0])
                } else {
                    ComputeValue::TensorF32 {
                        dimensions: port.dimensions.clone(),
                        layout: TensorLayout::RowMajor,
                        values: Arc::from(values.clone()),
                    }
                };
                (port.id, value)
            })
            .collect(),
    )
}

fn flattened_outputs(
    outputs: &mech_compute::ComputeOutputSnapshot,
) -> BTreeMap<mech_compute::ComputePortId, Vec<f32>> {
    outputs
        .values
        .iter()
        .map(|(port, value)| {
            let values = match value {
                ComputeValue::ScalarF32(value) => vec![*value],
                ComputeValue::TensorF32 { values, .. } => values.to_vec(),
            };
            (*port, values)
        })
        .collect()
}

#[cfg(feature = "native")]
#[test]
fn registered_backends_share_one_thousand_lane_fixed_shape_conformance_contract() {
    let (program, inputs) = source_program(1_000);
    let initializers = compute_initializers(&program, &inputs);
    let registry = native_compute_backend_registry();
    let bearing = program
        .compute_program()
        .interface()
        .input_named("bearing")
        .expect("EKF must expose bearing")
        .id;
    let mut accepted = BTreeMap::new();

    let mut backends = vec!["cpu-scalar", "cpu-simd"];
    if cfg!(feature = "jit") {
        backends.push("cpu-jit");
    }
    backends.push("wgpu");
    for backend in backends {
        let request = BackendRequest::parse(backend).unwrap();
        let factory = match registry.resolve(
            &request,
            ComputePlatform::Native,
            ComputePlacement::Compute,
            program.compute_program(),
        ) {
            Ok(factory) => factory,
            Err(error) if backend == "wgpu" && error.to_string().contains("adapter") => continue,
            Err(error) => panic!("{backend} rejected the conformance program: {error}"),
        };
        assert_eq!(factory.descriptor().id.as_str(), backend);
        let executable = factory.compile(program.compute_program()).unwrap();
        let mut session = executable.create_session(&initializers).unwrap();

        let first = session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
            .expect("the declaration initializer turn must run");
        assert_eq!(first.completed_turns, 1);
        assert_eq!(first.disposition, ComputeDispatchDisposition::Completed);
        assert_eq!(first.fault_count, 0);

        session
            .update_inputs(&[ComputeInputUpdate {
                port: bearing,
                value: ComputeValue::ScalarF32(-0.4),
            }])
            .expect("the live scalar update must preserve the session");
        let second = session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
            .expect("the updated turn must run");
        assert_eq!(second.completed_turns, 1);
        assert_eq!(second.disposition, ComputeDispatchDisposition::Completed);
        let published = session
            .read_outputs(&ComputeOutputSelection::All)
            .expect("published outputs must be readable");
        assert!(!published.values.is_empty());
        let sampled_port = program.compute_program().interface().outputs[0].clone();
        let sampled = session
            .read_outputs(&ComputeOutputSelection::Samples {
                ports: BTreeSet::from([sampled_port.id]),
                instance: 0,
            })
            .expect("lane-zero output sampling must be readable");
        assert_eq!(sampled.values.len(), 1);
        let sampled_elements = match &sampled.values[&sampled_port.id] {
            ComputeValue::ScalarF32(_) => 1,
            ComputeValue::TensorF32 {
                dimensions, values, ..
            } => {
                assert_eq!(dimensions.as_ref(), sampled_port.dimensions.as_ref());
                values.len()
            }
        };
        assert_eq!(sampled_elements, sampled_port.elements().unwrap());
        let instance = 731_u32;
        let sampled = session
            .read_outputs(&ComputeOutputSelection::Samples {
                ports: BTreeSet::from([sampled_port.id]),
                instance,
            })
            .expect("an explicit nonzero batch sample must be readable");
        let sampled_values = match &sampled.values[&sampled_port.id] {
            ComputeValue::ScalarF32(value) => vec![*value],
            ComputeValue::TensorF32 { values, .. } => values.to_vec(),
        };
        let published_values = match &published.values[&sampled_port.id] {
            ComputeValue::ScalarF32(value) => vec![*value],
            ComputeValue::TensorF32 { values, .. } => values.to_vec(),
        };
        let elements = sampled_port.elements().unwrap();
        let start = instance as usize * elements;
        assert_close(
            &sampled_values,
            &published_values[start..start + elements],
            1.0e-6,
        );
        assert!(
            session
                .read_outputs(&ComputeOutputSelection::Samples {
                    ports: BTreeSet::from([sampled_port.id]),
                    instance: 1_000,
                })
                .is_err(),
            "{backend} accepted a sample outside the resident batch",
        );

        session
            .update_inputs(&[ComputeInputUpdate {
                port: bearing,
                value: ComputeValue::ScalarF32(f32::NAN),
            }])
            .unwrap();
        let rejected = session
            .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
            .expect("integrity rejection is a bounded compute result");
        assert_eq!(rejected.completed_turns, 0);
        assert_eq!(rejected.disposition, ComputeDispatchDisposition::Rejected);
        assert_eq!(rejected.fault_count, 1);
        assert_eq!(
            rejected.last_fault.as_ref().unwrap().constraint.as_ref(),
            "finite-candidate!"
        );
        let retained = session
            .read_outputs(&ComputeOutputSelection::All)
            .expect("the previous published estimate must remain readable");
        assert_eq!(flattened_outputs(&retained), flattened_outputs(&published));
        accepted.insert(backend, flattened_outputs(&published));
    }

    let scalar = &accepted["cpu-scalar"];
    for (backend, outputs) in accepted
        .iter()
        .filter(|(backend, _)| **backend != "cpu-scalar")
    {
        assert_eq!(
            outputs.keys().collect::<Vec<_>>(),
            scalar.keys().collect::<Vec<_>>()
        );
        for (port, expected) in scalar {
            assert_close(expected, &outputs[port], 1.0e-4);
        }
        assert!(!backend.is_empty());
    }
}

#[test]
fn high_level_ekf_source_evaluation_matches_generic_lowering() {
    let tree = source_tree(1);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    assert_eq!(
        artifact
            .inputs()
            .iter()
            .map(|input| {
                mech_engine::decode_source_input_name(&input.name)
                    .unwrap_or_else(|| input.name.clone())
            })
            .collect::<BTreeSet<_>>(),
        [
            "dt",
            "linear-velocity",
            "angular-velocity",
            "bearing",
            "measurement-noise",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    );
    let inputs = source_inputs(&driver, &artifact);
    assert_close(&inputs["dt"], &[0.1], 1.0e-7);
    assert_close(&inputs["linear-velocity"], &[1.0], 1.0e-7);
    assert_close(&inputs["angular-velocity"], &[0.015], 1.0e-7);
    assert_close(&inputs["bearing"], &[-0.55], 1.0e-7);
    assert_close(&inputs["measurement-noise"], &[0.25], 1.0e-7);
    let (expected_state, expected_covariance) = source_evaluated_outputs(&tree, &driver);

    let operation_names = artifact
        .nodes()
        .iter()
        .map(|node| {
            node.as_operation()
                .expect("ordinary fixture operation")
                .operation
                .module_path
                .iter()
                .chain(std::iter::once(
                    &node
                        .as_operation()
                        .expect("ordinary fixture operation")
                        .operation
                        .operation_name,
                ))
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect::<Vec<_>>();
    for expected in [
        "matrix/matmul",
        "matrix/transpose",
        "matrix/dot",
        "math/sin",
        "math/cos",
        "math/atan2",
    ] {
        assert!(
            operation_names.iter().any(|name| name == expected),
            "missing canonical semantic operation {expected}: {operation_names:?}",
        );
    }
    assert!(
        operation_names
            .iter()
            .all(|name| !name.starts_with("runtime/"))
    );
    assert!(
        operation_names
            .iter()
            .all(|name| !name.to_ascii_lowercase().contains("ekf")),
        "the artifact must not contain an EKF-specific operation"
    );

    let lowered = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .expect("generic fixed-shape operations must lower");
    assert!(lowered.wgsl().contains("@compute"));
    assert!(!lowered.wgsl().to_ascii_lowercase().contains("ekf"));

    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(1).unwrap();
    let state_by_elements = lowered
        .state_layout()
        .map(|(slot, elements)| (elements, cpu.state()[&slot].clone()))
        .collect::<BTreeMap<_, _>>();

    assert_close(&expected_state, &state_by_elements[&3], 2.0e-5);
    assert_close(&expected_covariance, &state_by_elements[&9], 2.0e-4);
}

#[test]
fn mech_arrays_define_the_broadcast_extent() {
    let tree = source_tree(7);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let regions = artifact.compute_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].name.as_ref(), "2. EKF step");
    assert_eq!(regions[0].placement, mech_core::ComputePlacement::Compute);
    let inputs = source_inputs(&driver, &artifact);
    assert_eq!(inputs["linear-velocity"].len(), 7);
    assert_eq!(inputs["angular-velocity"].len(), 7);
    assert_eq!(inputs["bearing"].len(), 7);
    assert_eq!(inputs["dt"].len(), 1);

    let lowered = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap();
    assert_eq!(lowered.instances(), 7);
    assert_eq!(lowered.workgroup_count(), 1);
    assert_eq!(lowered.simd_lanes(), 4);
    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(2).unwrap();
    let mut simd = lowered.prepare_simd_cpu(&inputs).unwrap();
    simd.dispatch_turns(2).unwrap();
    for (slot, expected) in cpu.state() {
        assert_close(expected, &simd.state()[slot], 1.0e-4);
    }
    #[cfg(feature = "jit")]
    {
        let mut jit = lowered.prepare_jit_cpu(&inputs).unwrap();
        jit.dispatch_turns(2).unwrap();
        for (slot, expected) in cpu.state() {
            assert_close(expected, &jit.state()[slot], 1.0e-4);
        }
    }
    let state_sizes = lowered
        .state_layout()
        .map(|(slot, elements)| cpu.state()[&slot].len() / elements)
        .collect::<Vec<_>>();
    assert_eq!(state_sizes, [7, 7, 7, 7]);
}

#[test]
fn lane_zero_recurrence_is_independent_of_broadcast_extent() {
    let (single_program, single_inputs) = source_program(1);
    let (batch_program, batch_inputs) = source_program(1_000);
    let mut single = single_program.prepare_cpu(&single_inputs).unwrap();
    let mut batch = batch_program.prepare_cpu(&batch_inputs).unwrap();

    for turn in 0..128 {
        let lane_zero_bearing = -0.55 + 0.012 * (turn as f32 * 0.17).sin();
        let single_update = BTreeMap::from([("bearing".to_owned(), vec![lane_zero_bearing])]);
        let mut batch_bearings = batch_inputs["bearing"].clone();
        batch_bearings[0] = lane_zero_bearing;
        let batch_update = BTreeMap::from([("bearing".to_owned(), batch_bearings)]);
        single.update_inputs(&single_update).unwrap();
        batch.update_inputs(&batch_update).unwrap();
        single.dispatch_turns(1).unwrap();
        batch.dispatch_turns(1).unwrap();

        for (slot, elements) in single_program.state_layout() {
            let single_lane = &single.state()[&slot][..elements];
            let batch_lane_zero = &batch.state()[&slot][..elements];
            assert_close(single_lane, batch_lane_zero, 0.0);
        }
    }
}

#[cfg(feature = "native")]
#[test]
fn native_gpu_lane_zero_is_independent_of_broadcast_extent() {
    let (single_program, single_inputs) = source_program(1);
    let (batch_program, batch_inputs) = source_program(1_000);
    let mut single = match single_program.prepare_resident(&single_inputs) {
        Ok(session) => session,
        Err(BatchedExecutionError::Native(message))
            if message.to_ascii_lowercase().contains("adapter") =>
        {
            return;
        }
        Err(error) => panic!("single-lane native GPU preparation failed: {error}"),
    };
    let mut batch = batch_program
        .prepare_resident(&batch_inputs)
        .expect("the same adapter must admit the 1,000-lane program");
    let state_slots = single_program
        .compute_program()
        .interface()
        .outputs
        .iter()
        .map(|output| output.slot)
        .collect::<BTreeSet<_>>();

    for turn in 0..128 {
        let lane_zero_bearing = -0.55 + 0.012 * (turn as f32 * 0.17).sin();
        let single_update = BTreeMap::from([("bearing".to_owned(), vec![lane_zero_bearing])]);
        let mut batch_bearings = batch_inputs["bearing"].clone();
        batch_bearings[0] = lane_zero_bearing;
        let batch_update = BTreeMap::from([("bearing".to_owned(), batch_bearings)]);
        single
            .update_inputs(&single_program, &single_update)
            .unwrap();
        batch.update_inputs(&batch_program, &batch_update).unwrap();
        single.dispatch_turns(1).unwrap();
        batch.dispatch_turns(1).unwrap();
        let single_sample = single.read_published_sample(&state_slots, 0).unwrap();
        let batch_sample = batch.read_published_sample(&state_slots, 0).unwrap();
        for slot in &state_slots {
            assert_close(&single_sample[slot], &batch_sample[slot], 1.0e-6);
        }
    }
}

#[test]
fn conflicting_mech_array_extents_are_rejected() {
    let tree = source_tree(7);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let mut inputs = source_inputs(&driver, &artifact);
    inputs.get_mut("bearing").unwrap().pop();

    let error = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.detail.contains("broadcast extent 6"))
    );
}

#[test]
fn empty_mech_array_is_rejected() {
    let tree = source_tree(7);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let mut inputs = source_inputs(&driver, &artifact);
    inputs.get_mut("bearing").unwrap().clear();

    let error = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.detail.contains("0 element(s)"))
    );
}

#[test]
fn source_integrity_constraints_survive_artifact_and_batch_lowering() {
    let tree = source_tree(2);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    assert_eq!(artifact.constraints().len(), 3);
    assert_eq!(
        artifact
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "finite-candidate!",
            "positive-covariance!",
            "symmetric-covariance!",
        ])
    );
    let encoded = encode_program_artifact_sections(&artifact).unwrap();
    let decoded = decode_program_artifact_sections(&encoded).unwrap();
    assert_eq!(
        decoded
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "finite-candidate!",
            "positive-covariance!",
            "symmetric-covariance!",
        ])
    );
    let renamed_source = SOURCE.replacen("finite-candidate! :=", "finite-estimate! :=", 1);
    let renamed_tree = source_tree_from(&renamed_source, 2);
    let renamed_driver = evaluate_driver(&renamed_tree);
    let renamed_artifact = compile_compute(&renamed_tree, &renamed_driver);
    assert_ne!(artifact.revision(), renamed_artifact.revision());
    assert!(SOURCE.contains("finite-candidate! :="));
    assert!(SOURCE.contains("positive-covariance! :="));
    assert!(SOURCE.contains("symmetric-covariance! :="));

    let inputs = source_inputs(&driver, &artifact);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .expect("generic source constraints must lower with the numeric region");
    assert_eq!(
        program.integrity_constraints().collect::<Vec<_>>(),
        artifact
            .constraints()
            .iter()
            .map(|constraint| constraint.constraint)
            .collect::<Vec<_>>()
    );
    assert!(program.wgsl().contains("integrity_code"));
}

#[test]
fn fixed_shape_source_and_bytecode_lower_to_the_same_resident_program() {
    let tree = source_tree(3);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let encoded = encode_program_artifact_sections(&artifact).unwrap();
    let decoded = decode_program_artifact_sections(&encoded).unwrap();
    let source = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap();
    let bytecode = ComputeLowerer.compile_broadcast(&decoded, &inputs).unwrap();

    assert_eq!(source.wgsl(), bytecode.wgsl());
    assert_eq!(
        source.concrete_execution_cases(),
        bytecode.concrete_execution_cases(),
        "source and bytecode lowering must declare the same concrete GPU surface",
    );
    assert!(!source.concrete_execution_cases().is_empty());
    assert!(source.concrete_execution_cases().iter().all(|case| {
        case.targets.contains(mech_core::ExecutionTarget::GpuBatch)
            && case.targets.iter().count() == 1
            && artifact.nodes()[case.node.get() as usize]
                .as_operation()
                .unwrap()
                .operation
                == &case.operation
    }));
    assert!(source.concrete_execution_cases().iter().any(|case| {
        case.operation.module_path.as_ref() == ["core"] && case.operation.operation_name == "assign"
    }));
    assert_eq!(
        source.compute_program().interface(),
        bytecode.compute_program().interface()
    );
    let mut source_session = source.prepare_cpu(&inputs).unwrap();
    let mut bytecode_session = bytecode.prepare_cpu(&inputs).unwrap();
    source_session.dispatch_turns(2).unwrap();
    bytecode_session.dispatch_turns(2).unwrap();
    for (slot, expected) in source_session.state() {
        assert_close(expected, &bytecode_session.state()[slot], 0.0);
    }
}

#[test]
fn source_constraints_report_the_failed_rule() {
    let (program, mut inputs) = source_program(2);
    inputs
        .get_mut("measurement-noise")
        .unwrap()
        .iter_mut()
        .for_each(|value| *value = -0.15);
    let mut non_positive = program.prepare_cpu(&inputs).unwrap();
    let error = non_positive.dispatch_turns(1).unwrap_err();
    let BatchedExecutionError::Integrity(fault) = error else {
        panic!("expected an integrity fault, found {error}");
    };
    assert_eq!(fault.constraint_name.as_ref(), "positive-covariance!");

    let asymmetric_source = SOURCE.replacen(
        "corrected-covariance := correction ** predicted-covariance ** correction' + (gain ** gain') * measurement-noise",
        "corrected-covariance-base := correction ** predicted-covariance ** correction' + (gain ** gain') * measurement-noise\ncorrected-covariance := corrected-covariance-base + [0f32 (bearing + 0.55<f32>) * 0.1<f32> 0f32; 0f32 0f32 0f32; 0f32 0f32 0f32]",
        1,
    );
    let (program, mut inputs) = source_program_from(&asymmetric_source, 2);
    inputs
        .get_mut("bearing")
        .unwrap()
        .iter_mut()
        .for_each(|value| *value = -0.5);
    let mut asymmetric = program.prepare_cpu(&inputs).unwrap();
    let error = asymmetric.dispatch_turns(1).unwrap_err();
    let BatchedExecutionError::Integrity(fault) = error else {
        panic!("expected an integrity fault, found {error}");
    };
    assert_eq!(fault.constraint_name.as_ref(), "symmetric-covariance!");
}

#[test]
fn checked_cpu_backends_reject_candidate_and_keep_published_estimate() {
    let (program, mut inputs) = source_program(8);
    inputs
        .get_mut("bearing")
        .unwrap()
        .iter_mut()
        .for_each(|value| *value = f32::NAN);

    let mut scalar = program.prepare_cpu(&inputs).unwrap();
    let scalar_published = scalar.state().clone();
    assert!(matches!(
        scalar.dispatch_turns(1).unwrap_err(),
        BatchedExecutionError::Integrity(_)
    ));
    assert_eq!(scalar.state(), &scalar_published);
    assert_eq!(scalar.fault_count(), 1);
    assert_eq!(scalar.last_fault().unwrap().attempted_turn, 1);
    assert_eq!(
        scalar.last_fault().unwrap().constraint_name.as_ref(),
        "finite-candidate!"
    );

    let mut simd = program.prepare_simd_cpu(&inputs).unwrap();
    let simd_published = simd.state().clone();
    assert!(matches!(
        simd.dispatch_turns(1).unwrap_err(),
        BatchedExecutionError::Integrity(_)
    ));
    assert_eq!(simd.state(), &simd_published);
    assert_eq!(simd.fault_count(), 1);
    assert_eq!(
        simd.last_fault().unwrap().constraint_name.as_ref(),
        "finite-candidate!"
    );

    #[cfg(feature = "jit")]
    {
        let mut jit = program.prepare_jit_cpu(&inputs).unwrap();
        let jit_published = jit.state().clone();
        assert!(matches!(
            jit.dispatch_turns(1).unwrap_err(),
            BatchedExecutionError::Integrity(_)
        ));
        assert_eq!(jit.state(), &jit_published);
        assert_eq!(jit.fault_count(), 1);
        assert_eq!(
            jit.last_fault().unwrap().constraint_name.as_ref(),
            "finite-candidate!"
        );
    }
}

#[cfg(feature = "native")]
#[test]
fn source_driven_broadcast_matches_the_native_gpu() {
    let tree = source_tree(32);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let lowered = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap();
    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(4).unwrap();
    let output_slots = lowered
        .compute_program()
        .interface()
        .outputs
        .iter()
        .map(|output| output.slot)
        .collect::<BTreeSet<_>>();
    let expected = cpu
        .state()
        .iter()
        .filter(|(slot, _)| output_slots.contains(slot))
        .map(|(slot, values)| (*slot, values.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut gpu = match lowered.prepare_resident(&inputs) {
        Ok(gpu) => gpu,
        Err(mech_gpu::BatchedExecutionError::Native(message))
            if message.to_ascii_lowercase().contains("adapter")
                && message.to_ascii_lowercase().contains("unavailable") =>
        {
            return;
        }
        Err(error) => panic!("native GPU preparation failed: {error}"),
    };
    let actual = gpu.run_turns(4).unwrap().state;
    for (slot, expected) in expected {
        assert_close(&expected, &actual[&slot], 1.0e-4);
    }
}

#[cfg(feature = "native")]
#[test]
fn checked_gpu_rejects_candidate_and_keeps_published_estimate() {
    let (program, mut inputs) = source_program(32);
    let mut gpu = match program.prepare_resident(&inputs) {
        Ok(gpu) => gpu,
        Err(BatchedExecutionError::Native(message))
            if message.to_ascii_lowercase().contains("adapter")
                && message.to_ascii_lowercase().contains("unavailable") =>
        {
            return;
        }
        Err(error) => panic!("native GPU preparation failed: {error}"),
    };
    let mut cpu = program.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(2).unwrap();
    gpu.dispatch_turns(2).unwrap();
    let (_, before) = gpu.read_published_state().unwrap();
    for (slot, values) in &before {
        assert_close(&cpu.state()[slot], values, 1.0e-4);
    }
    assert_eq!(gpu.fault_count(), 0);
    inputs
        .get_mut("bearing")
        .unwrap()
        .iter_mut()
        .for_each(|value| *value = f32::NAN);
    gpu.update_inputs(&program, &inputs).unwrap();
    assert!(matches!(
        gpu.dispatch_turns(1).unwrap_err(),
        BatchedExecutionError::Integrity(_)
    ));
    let (_, after) = gpu.read_published_state().unwrap();
    assert_eq!(after, before);
    assert_eq!(gpu.fault_count(), 1);
    assert_eq!(gpu.last_fault().unwrap().attempted_turn, 3);
    assert_eq!(
        gpu.last_fault().unwrap().constraint_name.as_ref(),
        "finite-candidate!"
    );
}

fn assert_close(expected: &[f32], actual: &[f32], tolerance: f32) {
    assert_eq!(expected.len(), actual.len());
    let max_error = expected
        .iter()
        .zip(actual)
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_error <= tolerance,
        "maximum absolute error {max_error} exceeds {tolerance}:\nexpected {expected:?}\nactual   {actual:?}"
    );
}

#[test]
fn canonical_derived_publications_commit_without_becoming_recurrence_state() {
    for (body, expected_factors, publication_count) in [
        ("total = total + signal\n(total, total)", [3.0, 3.0], 1),
        (
            "copy := signal * 2f32\ntotal = total + signal\n(copy, total)",
            [2.0, 3.0],
            2,
        ),
    ] {
        let body = body.replace("\n(", "\nbounded! := total < 100f32\n(");
        let source = format!(
            "@worker := compute://worker/kernel{{:write(input/signal), :write(turn)}}\n@worker/input/signal <- 1f32\n@worker/turn <- 1\n\ncalculation @compute\n-------------------------------------------------------------------------------\nsignal := 1f32\n~total := 0f32\n{body}\n"
        );
        let document = mech_runtime::SourceDocument::parse_resolved(
            "test://canonical-publications",
            mech_syntax::document::Revision(0),
            Arc::<str>::from(source),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let mixed = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .build_compiler()
            .unwrap()
            .compile_mixed_document(&document)
            .unwrap();
        let inputs = BTreeMap::from([("signal".to_owned(), vec![1.0, 2.0, 3.0, 4.0, 5.0])]);
        let kernel = ComputeLowerer
            .compile_broadcast(&mixed.compute.artifact, &inputs)
            .unwrap();
        let program = kernel.compute_program();
        let storage = program.fixed_shape_storage().unwrap();
        assert_eq!(storage.states.len(), 1);
        assert_eq!(storage.publications.len(), publication_count);
        assert_eq!(program.interface().states.len(), 1);
        assert_eq!(program.interface().outputs.len(), 2);
        let mut direct = kernel.prepare_cpu(&inputs).unwrap();
        direct.dispatch_turns(3).unwrap();
        for (output, factor) in program.interface().outputs.iter().zip(expected_factors) {
            assert_eq!(
                direct.state()[&output.slot],
                (1..=5).map(|x| x as f32 * factor).collect::<Vec<_>>()
            );
        }
        let before = direct.state().clone();
        direct
            .update_inputs(&BTreeMap::from([(
                "signal".to_owned(),
                vec![1.0, 2.0, 3.0, 4.0, 1000.0],
            )]))
            .unwrap();
        assert!(matches!(
            direct.dispatch_turns(1),
            Err(BatchedExecutionError::Integrity(_))
        ));
        assert_eq!(direct.last_fault().unwrap().instance, 4);
        assert_eq!(direct.state(), &before);
        // Registry initializers are one source value, broadcast across lanes.
        let initializers =
            compute_initializers(&kernel, &BTreeMap::from([("signal".to_owned(), vec![2.0])]));
        let registry = native_compute_backend_registry();
        let mut backends = vec!["cpu-scalar", "cpu-simd", "wgpu"];
        if cfg!(feature = "jit") {
            backends.push("cpu-jit");
        }
        for backend in backends {
            let factory = match registry.resolve(
                &BackendRequest::parse(backend).unwrap(),
                ComputePlatform::Native,
                ComputePlacement::Compute,
                program,
            ) {
                Ok(factory) => factory,
                Err(error) if backend == "wgpu" && error.to_string().contains("adapter") => {
                    continue;
                }
                Err(error) => panic!("{backend}: {error}"),
            };
            let executable = factory.compile(program).unwrap();
            let mut session = executable.create_session(&initializers).unwrap();
            let initial = session.read_outputs(&ComputeOutputSelection::All).unwrap();
            for values in flattened_outputs(&initial).values() {
                assert_eq!(values, &[0.0; 5]);
            }
            session
                .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(3).unwrap()))
                .unwrap();
            let actual =
                flattened_outputs(&session.read_outputs(&ComputeOutputSelection::All).unwrap());
            for (output, factor) in program.interface().outputs.iter().zip(expected_factors) {
                assert_eq!(
                    actual[&output.id],
                    vec![2.0 * factor; 5],
                    "{backend}: {}",
                    output.name
                );
            }
            session
                .update_inputs(&[ComputeInputUpdate {
                    port: program.interface().inputs[0].id,
                    value: ComputeValue::ScalarF32(1000.0),
                }])
                .unwrap();
            let rejected = session
                .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
                .unwrap();
            assert_eq!(
                rejected.disposition,
                ComputeDispatchDisposition::Rejected,
                "{backend}"
            );
            assert_eq!(
                flattened_outputs(&session.read_outputs(&ComputeOutputSelection::All).unwrap()),
                actual,
                "{backend}"
            );
        }
    }
}

#[test]
fn canonical_matrix_broadcast_uses_each_input_axis() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (rows, columns, column, reverse) in [
        (2, 3, true, false),
        (3, 2, false, false),
        (2, 3, false, true),
        (3, 2, true, true),
        (1, 4, true, false),
        (4, 1, false, true),
    ] {
        let matrix = (0..rows)
            .map(|row| {
                (0..columns)
                    .map(|col| format!("{}f32", row * columns + col + 1))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("; ");
        let count = if column { rows } else { columns };
        let offset_values = (1..=count).map(|i| i as f32 * 10.0).collect::<Vec<_>>();
        let offset = offset_values
            .iter()
            .map(|x| format!("{x}f32"))
            .collect::<Vec<_>>()
            .join(if column { "; " } else { " " });
        let expression = if reverse {
            "offset - matrix"
        } else {
            "matrix - offset"
        };
        let source = format!(
            "@worker := compute://worker/kernel{{:write(input/offset), :write(turn)}}\n@worker/input/offset <- [{offset}]\n@worker/turn <- 1\n\ncalculation @compute\n-------------------------------------------------------------------------------\noffset := [{offset}]\n~matrix := [{matrix}]\nmatrix = {expression}\nmatrix\n"
        );
        let document = mech_runtime::SourceDocument::parse_resolved(
            "test://canonical-broadcast",
            mech_syntax::document::Revision(0),
            Arc::<str>::from(source),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let mixed = compiler.compile_mixed_document(&document).unwrap();
        let inputs = BTreeMap::from([("offset".to_owned(), offset_values)]);
        let kernel = ComputeLowerer
            .compile_broadcast(&mixed.compute.artifact, &inputs)
            .unwrap();
        let expected = (0..rows * columns)
            .map(|index| {
                let offset = if column {
                    index / columns + 1
                } else {
                    index % columns + 1
                } as f32
                    * 10.0;
                if reverse {
                    offset - (index + 1) as f32
                } else {
                    (index + 1) as f32 - offset
                }
            })
            .collect::<Vec<_>>();
        let registry = native_compute_backend_registry();
        let initializers = compute_initializers(&kernel, &inputs);
        for backend in ["cpu-scalar", "cpu-simd", "wgpu"] {
            let program = kernel.compute_program();
            let factory = match registry.resolve(
                &BackendRequest::parse(backend).unwrap(),
                ComputePlatform::Native,
                ComputePlacement::Compute,
                program,
            ) {
                Ok(factory) => factory,
                Err(error) if backend == "wgpu" && error.to_string().contains("adapter") => {
                    continue;
                }
                Err(error) => panic!("{backend}: {error}"),
            };
            let mut session = factory
                .compile(program)
                .unwrap()
                .create_session(&initializers)
                .unwrap();
            session
                .dispatch(&ComputeDispatchRequest::new(NonZeroU32::new(1).unwrap()))
                .unwrap();
            let actual =
                flattened_outputs(&session.read_outputs(&ComputeOutputSelection::All).unwrap());
            assert_eq!(
                actual[&program.interface().outputs[0].id],
                expected,
                "{rows}x{columns} column={column} reverse={reverse} backend={backend}"
            );
        }
    }
}
