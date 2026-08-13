use std::collections::BTreeMap;

use mech_core::{LegacyValue, Ref, hash_str};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::GpuHost;

const SOURCE: &str = include_str!("../fixtures/ekf-kernel.mec");

fn insert_f32(program: &MechProgram, name: &str, value: f32) {
    let id = hash_str(name);
    let symbols = program.interpreter().symbols();
    symbols
        .borrow_mut()
        .insert(id, LegacyValue::F32(Ref::new(value)), false);
    symbols
        .borrow()
        .dictionary
        .borrow_mut()
        .insert(id, name.to_owned());
}

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

fn inputs() -> BTreeMap<String, Vec<f32>> {
    BTreeMap::from([
        ("dt".to_owned(), vec![0.1]),
        ("linear-velocity".to_owned(), vec![1.0]),
        ("angular-velocity".to_owned(), vec![0.015]),
        ("bearing".to_owned(), vec![-0.55]),
        ("measurement-noise".to_owned(), vec![0.25]),
    ])
}

#[test]
fn high_level_ekf_source_matches_ordinary_mech_after_generic_lowering() {
    let mut source_program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    insert_f32(&source_program, "host-linear-velocity", 1.0);
    insert_f32(&source_program, "host-angular-velocity", 0.015);
    insert_f32(&source_program, "host-bearing", -0.55);

    source_program
        .run_string(SOURCE)
        .expect("ordinary high-level EKF source must run");
    let expected_state = symbol_f32_values(&source_program, "state");
    let expected_covariance = symbol_f32_values(&source_program, "covariance");
    let artifact = source_program
        .compile_program_artifact()
        .expect("ordinary high-level EKF source must compile");

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
        .compile_batched(&artifact, 1)
        .expect("generic fixed-shape operations must lower");
    assert!(lowered.wgsl().contains("@compute"));
    assert!(!lowered.wgsl().to_ascii_lowercase().contains("ekf"));

    let mut cpu = lowered.prepare_cpu(&inputs()).unwrap();
    cpu.dispatch_turns(1).unwrap();
    let state_by_elements = lowered
        .state_layout()
        .map(|(slot, elements)| (elements, cpu.state()[&slot].clone()))
        .collect::<BTreeMap<_, _>>();

    assert_close(&expected_state, &state_by_elements[&3], 2.0e-5);
    assert_close(&expected_covariance, &state_by_elements[&9], 2.0e-4);
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
