use std::collections::{BTreeMap, BTreeSet};

use mech_core::{Body, MechCode, Program, Section, SectionElement};
use mech_engine::{
    ProgramArtifact, decode_program_artifact_sections, encode_program_artifact_sections,
};
use mech_gpu::{BatchedExecutionError, BatchedGpuProgram, GpuHost};
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

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
const COMPUTE_INPUT_NAMES: [&str; 7] = [
    "dt",
    "linear-velocity",
    "angular-velocity",
    "bearing",
    "measurement-noise",
    "finite-limit",
    "covariance-symmetry-tolerance",
];

fn source_tree(instances: usize) -> Program {
    source_tree_from(SOURCE, instances)
}

fn source_tree_from(source: &str, instances: usize) -> Program {
    let source = source.replacen("100000f32", &format!("{instances}f32"), 1);
    mech_syntax::parse(&source).expect("EKF array source must parse")
}

fn projected_tree(tree: &Program, compute: bool) -> Program {
    let imports = tree
        .body
        .sections
        .iter()
        .flat_map(|section| &section.elements)
        .filter_map(|element| {
            let SectionElement::MechCode(code) = element else {
                return None;
            };
            let imports = code
                .iter()
                .filter(|(code, _)| matches!(code, MechCode::Import(_)))
                .cloned()
                .collect::<Vec<_>>();
            (!imports.is_empty()).then_some(SectionElement::MechCode(imports))
        })
        .collect::<Vec<_>>();
    let selected = tree
        .body
        .sections
        .iter()
        .find(|section| (!section.annotations.is_empty()) == compute)
        .expect("EKF source must contain driver and compute sections")
        .clone();
    Program {
        title: tree.title.clone(),
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    annotations: Vec::new(),
                    elements: imports,
                },
                selected,
            ],
        },
    }
}

fn evaluate_driver(tree: &Program) -> BTreeMap<String, Vec<f32>> {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .evaluate_static_tree_symbols(&projected_tree(tree, false), &DRIVER_INPUT_NAMES)
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

fn compile_compute(tree: &Program, driver: &BTreeMap<String, Vec<f32>>) -> ProgramArtifact {
    let scalar_inputs = driver
        .iter()
        .map(|(name, values)| {
            (
                name.clone(),
                RuntimeHostInputValue::F32(*values.first().expect("driver input is non-empty")),
            )
        })
        .collect();
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .compile_tree_artifact_with_inputs(
            &projected_tree(tree, true),
            &scalar_inputs,
            &COMPUTE_INPUT_NAMES.into_iter().map(str::to_owned).collect(),
        )
        .expect("ordinary high-level EKF source must compile")
        .into_artifact()
}

fn source_inputs(
    driver: &BTreeMap<String, Vec<f32>>,
    artifact: &ProgramArtifact,
) -> BTreeMap<String, Vec<f32>> {
    artifact
        .inputs()
        .iter()
        .filter_map(|input| {
            let driver_name = match input.name.as_str() {
                "dt" => "lane-dt",
                "linear-velocity" => "lane-linear-velocity",
                "angular-velocity" => "lane-angular-velocity",
                "bearing" => "lane-bearing",
                "measurement-noise" => "lane-measurement-noise",
                name => name,
            };
            driver
                .get(driver_name)
                .map(|values| (input.name.clone(), values.clone()))
        })
        .collect()
}

fn source_evaluated_outputs(
    tree: &Program,
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
    tree: &Program,
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
        .evaluate_static_tree_symbols_with_inputs(&projected_tree(tree, true), &inputs, names)
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
    tree: &Program,
    driver: &BTreeMap<String, Vec<f32>>,
) -> BTreeMap<usize, Vec<f32>> {
    let artifact = compile_compute(tree, driver);
    let inputs = source_inputs(driver, &artifact);
    let lowered = GpuHost
        .compile_broadcast(&artifact, &inputs)
        .expect("generic fixed-shape operations must lower");
    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(1).unwrap();
    lowered
        .state_layout()
        .map(|(slot, elements)| (elements, cpu.state()[&slot].clone()))
        .collect()
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
    let tree = mech_syntax::parse(
        r#"
+> math

rectangular projection @compute
-------------------------------------------------------------------------------
input := source-input
projection := [1f32 2f32 3f32
               4f32 5f32 6f32]
~result := [0f32; 0f32]
result = projection ** input
result
"#,
    )
    .unwrap();
    let planning_inputs = BTreeMap::from([(
        "source-input".to_owned(),
        RuntimeHostInputValue::F32Matrix {
            rows: 3,
            columns: 1,
            values: vec![1.0, 10.0, 100.0],
        },
    )]);
    let product = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_tree_artifact_with_inputs(
            &tree,
            &planning_inputs,
            &BTreeSet::from(["input".to_owned()]),
        )
        .unwrap();
    let artifact = product.into_artifact();
    let activation_inputs = BTreeMap::from([("input".to_owned(), vec![1.0, 10.0, 100.0])]);
    let program = GpuHost
        .compile_broadcast(&artifact, &activation_inputs)
        .unwrap();
    let mut cpu = program.prepare_cpu(&activation_inputs).unwrap();
    cpu.dispatch_turns(1).unwrap();

    let result = program
        .state_layout()
        .find_map(|(slot, elements)| (elements == 2).then(|| &cpu.state()[&slot]))
        .expect("the result state must be present");
    assert_close(result, &[321.0, 654.0], 1.0e-6);
}

fn source_program(instances: usize) -> (BatchedGpuProgram, BTreeMap<String, Vec<f32>>) {
    source_program_from(SOURCE, instances)
}

fn source_program_from(
    source: &str,
    instances: usize,
) -> (BatchedGpuProgram, BTreeMap<String, Vec<f32>>) {
    let tree = source_tree_from(source, instances);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = GpuHost
        .compile_broadcast(&artifact, &inputs)
        .expect("generic fixed-shape operations must lower");
    (program, inputs)
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
            .map(|input| input.name.as_str())
            .collect::<BTreeSet<_>>(),
        [
            "dt",
            "linear-velocity",
            "angular-velocity",
            "bearing",
            "measurement-noise",
        ]
        .into_iter()
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
        .map(|node| node.operation.operation_name.as_str())
        .collect::<Vec<_>>();
    assert!(
        operation_names
            .iter()
            .any(|name| name.starts_with("MatMul"))
    );
    assert!(
        operation_names
            .iter()
            .any(|name| name.starts_with("Transpose"))
    );
    assert!(operation_names.iter().any(|name| name.starts_with("Dot")));
    assert!(
        operation_names
            .iter()
            .any(|name| name.starts_with("MathSin"))
    );
    assert!(
        operation_names
            .iter()
            .any(|name| name.starts_with("MathCos"))
    );
    assert!(operation_names.iter().any(|name| name.starts_with("Atan2")));
    assert!(
        operation_names
            .iter()
            .all(|name| !name.to_ascii_lowercase().contains("ekf")),
        "the artifact must not contain an EKF-specific operation"
    );

    let lowered = GpuHost
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
    assert_eq!(regions[0].name.as_ref(), "EKF step");
    assert_eq!(regions[0].placement, mech_core::ComputePlacement::Compute);
    let inputs = source_inputs(&driver, &artifact);
    assert_eq!(inputs["linear-velocity"].len(), 7);
    assert_eq!(inputs["angular-velocity"].len(), 7);
    assert_eq!(inputs["bearing"].len(), 7);
    assert_eq!(inputs["dt"].len(), 1);

    let lowered = GpuHost.compile_broadcast(&artifact, &inputs).unwrap();
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
    assert_eq!(state_sizes, [7, 7]);
}

#[test]
fn conflicting_mech_array_extents_are_rejected() {
    let tree = source_tree(7);
    let driver = evaluate_driver(&tree);
    let artifact = compile_compute(&tree, &driver);
    let mut inputs = source_inputs(&driver, &artifact);
    inputs.get_mut("bearing").unwrap().pop();

    let error = GpuHost.compile_broadcast(&artifact, &inputs).unwrap_err();
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

    let error = GpuHost.compile_broadcast(&artifact, &inputs).unwrap_err();
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
    let program = GpuHost
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
        "corrected-covariance-base := correction ** predicted-covariance ** correction' + (gain ** gain') * measurement-noise\ncorrected-covariance := corrected-covariance-base + [0f32 0.01<f32> 0f32; 0f32 0f32 0f32; 0f32 0f32 0f32]",
        1,
    );
    let (program, inputs) = source_program_from(&asymmetric_source, 2);
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
    let lowered = GpuHost.compile_broadcast(&artifact, &inputs).unwrap();
    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(4).unwrap();
    let expected = cpu.state().clone();
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
    inputs
        .get_mut("bearing")
        .unwrap()
        .iter_mut()
        .for_each(|value| *value = f32::NAN);
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
    let (_, before) = gpu.read_published_state().unwrap();
    assert!(matches!(
        gpu.dispatch_turns(1).unwrap_err(),
        BatchedExecutionError::Integrity(_)
    ));
    let (_, after) = gpu.read_published_state().unwrap();
    assert_eq!(after, before);
    assert_eq!(gpu.fault_count(), 1);
    assert_eq!(gpu.last_fault().unwrap().attempted_turn, 1);
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
