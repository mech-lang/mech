use std::collections::{BTreeMap, BTreeSet};

use mech_core::{Body, LegacyValue, MechCode, Program, Ref, Section, SectionElement, hash_str};
use mech_engine::{ArtifactComputeRegion, MechProgram, MechProgramConfig, ProgramArtifact};
use mech_gpu::GpuHost;

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
    let source = SOURCE.replacen("100000f32", &format!("{instances}f32"), 1);
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
    let mut cpu = lowered.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(2).unwrap();
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
            if message.contains("AdapterUnavailable") =>
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
