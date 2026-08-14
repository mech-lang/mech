use std::collections::{BTreeMap, BTreeSet};

use mech_core::{Body, LegacyValue, MechCode, Program, Ref, Section, SectionElement, hash_str};
use mech_engine::{
    ArtifactComputeRegion, MechProgram, MechProgramConfig, ProgramArtifact,
    decode_program_artifact_sections, encode_program_artifact_sections,
};
use mech_gpu::{BatchedExecutionError, BatchedGpuProgram, GpuHost};

const SOURCE: &str = include_str!("../fixtures/ekf-kernel.mec");
fn symbol_f32_values(program: &MechProgram, name: &str) -> Vec<f32> {
    program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("{name} must exist"))
        .borrow()
        .as_vecf32()
        .unwrap_or_else(|error| panic!("{name} must contain f32 values: {error:?}"))
}

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
        .find(|section| section.compute.is_some() == compute)
        .expect("EKF source must contain driver and compute sections")
        .clone();
    Program {
        title: tree.title.clone(),
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    compute: None,
                    elements: imports,
                },
                selected,
            ],
        },
    }
}

fn evaluate_driver(tree: &Program) -> MechProgram {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    program
        .run_tree(&projected_tree(tree, false))
        .expect("Mech EKF arrays must evaluate");
    program
}

fn seed_lane_inputs(target: &MechProgram, driver: &MechProgram, tree: &Program) {
    let compute_definitions = tree
        .body
        .sections
        .iter()
        .filter(|section| section.compute.is_some())
        .flat_map(|section| &section.elements)
        .filter_map(|element| match element {
            SectionElement::MechCode(code) => Some(code),
            _ => None,
        })
        .flatten()
        .filter_map(|(code, _)| match code {
            MechCode::Statement(mech_core::Statement::VariableDefine(definition)) => {
                Some(definition.var.name.hash())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let driver_symbols = driver.interpreter().symbols();
    let lane_values = {
        let symbols = driver_symbols.borrow();
        let dictionary = symbols.dictionary.borrow();
        symbols
            .symbols
            .iter()
            .filter(|(id, _)| !compute_definitions.contains(id))
            .filter_map(|(id, cell)| {
                let values = cell.borrow().as_vecf32().ok()?;
                Some((*id, dictionary.get(id)?.clone(), *values.first()?))
            })
            .collect::<Vec<_>>()
    };
    let target_symbols = target.interpreter().symbols();
    for (id, name, value) in lane_values {
        target_symbols
            .borrow_mut()
            .insert(id, LegacyValue::F32(Ref::new(value)), false);
        target_symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name);
    }
}

fn compile_compute(
    tree: &Program,
    driver: &MechProgram,
) -> (MechProgram, ProgramArtifact, Box<[ArtifactComputeRegion]>) {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    seed_lane_inputs(&program, driver, tree);
    program
        .run_tree(&projected_tree(tree, true))
        .expect("ordinary high-level EKF compute region must run");
    let (artifact, regions) = program
        .compile_program_artifact_with_regions()
        .expect("ordinary high-level EKF source must compile");
    (program, artifact, regions)
}

fn source_inputs(program: &MechProgram, artifact: &ProgramArtifact) -> BTreeMap<String, Vec<f32>> {
    let symbols = program.interpreter().symbols();
    let symbols = symbols.borrow();
    artifact
        .inputs()
        .iter()
        .filter_map(|input| {
            symbols.get(hash_str(&input.name)).map(|cell| {
                (
                    input.name.clone(),
                    cell.borrow()
                        .as_vecf32()
                        .unwrap_or_else(|error| panic!("{}: {error:?}", input.name)),
                )
            })
        })
        .collect()
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
    let (_, artifact, regions) = compile_compute(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
        .expect("generic fixed-shape operations must lower");
    (program, inputs)
}

#[test]
fn high_level_ekf_source_matches_ordinary_mech_after_generic_lowering() {
    let tree = source_tree(1);
    let driver = evaluate_driver(&tree);
    let (source_program, artifact, regions) = compile_compute(&tree, &driver);
    let expected_state = symbol_f32_values(&source_program, "state");
    let expected_covariance = symbol_f32_values(&source_program, "covariance");

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

    let inputs = source_inputs(&driver, &artifact);
    let lowered = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
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
    let (_, artifact, regions) = compile_compute(&tree, &driver);
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].name, "EKF step");
    assert_eq!(regions[0].placement, mech_core::ComputePlacement::Compute);
    let inputs = source_inputs(&driver, &artifact);
    assert_eq!(inputs["linear-velocity"].len(), 7);
    assert_eq!(inputs["angular-velocity"].len(), 7);
    assert_eq!(inputs["bearing"].len(), 7);
    assert_eq!(inputs["dt"].len(), 1);

    let lowered = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
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
    assert_eq!(state_sizes, [7, 7]);
}

#[test]
fn conflicting_mech_array_extents_are_rejected() {
    let tree = source_tree(7);
    let driver = evaluate_driver(&tree);
    let (_, artifact, regions) = compile_compute(&tree, &driver);
    let mut inputs = source_inputs(&driver, &artifact);
    inputs.get_mut("bearing").unwrap().pop();

    let error = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
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
    let (_, artifact, regions) = compile_compute(&tree, &driver);
    let mut inputs = source_inputs(&driver, &artifact);
    inputs.get_mut("bearing").unwrap().clear();

    let error = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
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
    let (_, artifact, regions) = compile_compute(&tree, &driver);
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
    let (_, renamed_artifact, _) = compile_compute(&renamed_tree, &renamed_driver);
    assert_ne!(artifact.revision(), renamed_artifact.revision());
    assert!(SOURCE.contains("finite-candidate! :="));
    assert!(SOURCE.contains("positive-covariance! :="));
    assert!(SOURCE.contains("symmetric-covariance! :="));

    let inputs = source_inputs(&driver, &artifact);
    let program = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
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
    let (_, artifact, regions) = compile_compute(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let lowered = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
        .unwrap();
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
