use std::collections::BTreeMap;

use mech_core::{LegacyValue, Ref, ResolvedOperationContract, hash_str, matrix::Matrix};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::{GpuDiagnosticCode, GpuHost};

const PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");

fn compile_source(
    source: &str,
    inputs: impl IntoIterator<Item = (&'static str, LegacyValue)>,
) -> mech_engine::ProgramArtifact {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    let symbols = program.interpreter().symbols();
    for (name, value) in inputs {
        let id = hash_str(name);
        symbols.borrow_mut().insert(id, value, false);
        symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name.to_owned());
    }
    program.run_string(source).expect("source must run");
    program
        .compile_program_product()
        .expect("source must compile")
        .into_parts()
        .0
}

fn particle_inputs() -> Vec<(&'static str, LegacyValue)> {
    vec![
        (
            "host-positions",
            LegacyValue::MatrixF32(Matrix::from_vec(
                vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
                4,
                2,
            )),
        ),
        (
            "host-velocities",
            LegacyValue::MatrixF32(Matrix::from_vec(vec![0.0; 8], 4, 2)),
        ),
        (
            "host-origin",
            LegacyValue::MatrixF32(Matrix::from_vec(vec![0.0; 8], 4, 2)),
        ),
        ("host-attraction", LegacyValue::F32(Ref::new(0.5))),
        ("host-drag", LegacyValue::F32(Ref::new(0.9))),
        ("host-dt", LegacyValue::F32(Ref::new(0.1))),
    ]
}

#[test]
fn particle_program_is_lowered_from_mech_to_fused_wgsl() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("particle source must be admitted: {error}"));

    assert!(
        program
            .wgsl()
            .contains("// Generated from a typed Mech ProgramArtifact.")
    );
    assert!(program.wgsl().contains("@compute @workgroup_size(64)"));
    assert!(!program.wgsl().contains("gravity"));
    assert_eq!(program.dispatch_elements(), 8);
    assert_eq!(program.workgroup_count(), 1);

    let mut inputs = BTreeMap::new();
    inputs.insert(
        "positions".to_owned(),
        vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
    );
    inputs.insert("velocities".to_owned(), vec![0.0; 8]);
    inputs.insert("origin".to_owned(), vec![0.0; 8]);
    inputs.insert("attraction".to_owned(), vec![0.5]);
    inputs.insert("drag".to_owned(), vec![0.9]);
    inputs.insert("dt".to_owned(), vec![0.1]);
    let outputs = program.run_cpu(&inputs).expect("CPU backend must run");

    let expected_velocities = [-0.045, 0.045, -0.09, 0.09, -0.0225, 0.0225, -0.18, 0.18];
    let expected_positions = [
        0.9955, -0.9955, 1.991, -1.991, 0.49775, -0.49775, 3.982, -3.982,
    ];
    assert_close(&outputs["result.1"], &expected_velocities);
    assert_close(&outputs["result.0"], &expected_positions);
}

#[cfg(feature = "native")]
#[test]
fn native_gpu_matches_the_cpu_backend_when_an_adapter_is_available() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    let program = GpuHost
        .compile(&artifact)
        .expect("particle source must be admitted");
    let inputs = BTreeMap::from([
        (
            "positions".to_owned(),
            vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
        ),
        ("velocities".to_owned(), vec![0.0; 8]),
        ("origin".to_owned(), vec![0.0; 8]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let cpu = program.run_cpu(&inputs).expect("CPU backend must run");
    let gpu = match program.run_gpu(&inputs) {
        Ok(gpu) => gpu,
        Err(mech_gpu::GpuExecutionError::AdapterUnavailable) => return,
        Err(error) => panic!("GPU dispatch failed: {error}"),
    };
    assert_eq!(
        cpu.keys().collect::<Vec<_>>(),
        gpu.keys().collect::<Vec<_>>()
    );
    for (name, cpu_values) in cpu {
        assert_close(&gpu[&name], &cpu_values);
    }
}

#[test]
fn unsupported_program_reports_why_instead_of_falling_back() {
    let artifact = compile_source(
        "answer := left / right",
        [
            ("left", LegacyValue::F32(Ref::new(1.0))),
            ("right", LegacyValue::F32(Ref::new(2.0))),
        ],
    );
    let error = GpuHost
        .compile(&artifact)
        .expect_err("division is outside the first GPU capability set");

    assert!(error.diagnostics().iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            GpuDiagnosticCode::OpaqueOperationContract | GpuDiagnosticCode::OperationUnsupported
        ) && diagnostic.node.is_some()
            && diagnostic.operation.is_some()
    }));
}

#[test]
fn particle_arithmetic_reaches_artifact_with_declared_contracts() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    assert!(!artifact.nodes().is_empty());
    for node in artifact.nodes() {
        if node.operation.operation_name.starts_with("VariableDefine") {
            continue;
        }
        if node.operation.module_path.as_ref() == ["core"]
            && node.operation.operation_name == "composite-pack"
        {
            continue;
        }
        assert!(matches!(
            artifact.contracts().get(node.contract),
            Some(ResolvedOperationContract::Declared(_))
        ));
    }
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "element {index}: expected {expected}, got {actual}"
        );
    }
}
