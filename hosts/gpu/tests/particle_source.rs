use std::collections::BTreeMap;

use mech_core::{LegacyValue, Ref, ResolvedOperationContract, hash_str, matrix::Matrix};
use mech_engine::{MechProgram, MechProgramConfig, SlotRole};
use mech_gpu::{ExecutionTarget, GpuDiagnosticCode, GpuHost, SlotResidence, TransferDirection};

const PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particle-kernel.mec");

const STANDALONE_PARTICLE_SOURCE: &str =
    include_str!("../../../examples/gpu-particles/particles.mec");

fn compile_source(
    source: &str,
    inputs: impl IntoIterator<Item = (&'static str, LegacyValue)>,
) -> mech_engine::ProgramArtifact {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
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
        .compile_program_artifact()
        .expect("source must compile")
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
        ("host-origin", LegacyValue::F32(Ref::new(0.0))),
        ("host-attraction", LegacyValue::F32(Ref::new(0.5))),
        ("host-drag", LegacyValue::F32(Ref::new(0.9))),
        ("host-dt", LegacyValue::F32(Ref::new(0.1))),
    ]
}

#[test]
fn particle_program_is_lowered_from_mech_to_fused_wgsl() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    assert_eq!(
        artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
            .count(),
        2
    );
    assert_eq!(
        artifact.constants().len(),
        2,
        "state initializers are retained"
    );
    let placement = GpuHost.plan(&artifact);
    assert!(placement.fully_accelerated);
    assert_eq!(placement.gpu_regions.len(), 1);
    assert_eq!(
        placement
            .slots
            .iter()
            .filter(|slot| slot.residence == SlotResidence::DeviceState)
            .count(),
        2
    );
    assert!(
        placement
            .transfers
            .iter()
            .any(|transfer| transfer.direction == TransferDirection::Upload)
    );
    assert_eq!(
        placement
            .transfers
            .iter()
            .filter(|transfer| transfer.direction == TransferDirection::Readback)
            .count(),
        2
    );
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
    inputs.insert("origin".to_owned(), vec![0.0]);
    inputs.insert("attraction".to_owned(), vec![0.5]);
    inputs.insert("drag".to_owned(), vec![0.9]);
    inputs.insert("dt".to_owned(), vec![0.1]);
    let outputs = program.run_cpu(&inputs).expect("CPU backend must run");

    let expected_velocities = [-0.045, -0.0225, 0.045, 0.0225, -0.09, -0.18, 0.09, 0.18];
    let expected_positions = [
        0.9955, 0.49775, -0.9955, -0.49775, 1.991, 3.982, -1.991, -3.982,
    ];
    assert_close(&outputs["result.1"], &expected_velocities);
    assert_close(&outputs["result.0"], &expected_positions);
}

#[test]
fn standalone_particle_program_needs_no_host_inputs() {
    let artifact = compile_source(STANDALONE_PARTICLE_SOURCE, []);
    assert!(
        artifact.inputs().is_empty(),
        "unexpected inputs: {:?}",
        artifact.inputs()
    );
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("standalone particle source must be admitted: {error}"));
    let mut cpu = program
        .prepare_cpu(&BTreeMap::new())
        .expect("standalone CPU executor must prepare");
    let initial = cpu.outputs().expect("standalone initial outputs must read");
    assert_eq!(initial["result.0"].len(), 18);
    assert_eq!(initial["result.1"].len(), 18);
    assert!(initial["result.0"].iter().any(|value| *value != 0.0));
    assert!(initial["result.1"].iter().all(|value| *value == 0.0));

    cpu.dispatch_turns(6)
        .expect("standalone CPU executor must advance");
    let cycled = cpu.outputs().expect("cycled outputs must read");
    assert_close(&cycled["result.0"], &initial["result.0"]);
    assert_close(&cycled["result.1"], &initial["result.1"]);
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
        ("origin".to_owned(), vec![0.0]),
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

#[cfg(feature = "native")]
#[test]
fn resident_gpu_feeds_particle_outputs_into_the_next_turn() {
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
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let mut cpu = program.prepare_cpu(&inputs).expect("CPU must prepare");
    cpu.dispatch_turns(3).expect("CPU turns must run");
    let expected = cpu.outputs().expect("CPU outputs must read");

    let initial_inputs = BTreeMap::from([
        (
            "positions".to_owned(),
            vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 4.0, -4.0],
        ),
        ("velocities".to_owned(), vec![0.0; 8]),
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let mut resident = match program.prepare_resident(&initial_inputs) {
        Ok(resident) => resident,
        Err(mech_gpu::GpuExecutionError::AdapterUnavailable) => return,
        Err(error) => panic!("resident GPU preparation failed: {error}"),
    };
    let gpu = resident.run_turns(3).expect("resident turns must run");
    assert_close(&gpu.outputs["result.0"], &expected["result.0"]);
    assert_close(&gpu.outputs["result.1"], &expected["result.1"]);
}

#[test]
fn resident_cpu_advances_artifact_state_without_host_feedback() {
    let artifact = compile_source(PARTICLE_SOURCE, particle_inputs());
    let program = GpuHost
        .compile(&artifact)
        .expect("particle source must be admitted");
    let inputs = BTreeMap::from([
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.9]),
        ("dt".to_owned(), vec![0.1]),
    ]);
    let mut cpu = program.prepare_cpu(&inputs).expect("CPU must prepare");
    cpu.dispatch_turns(3).expect("CPU turns must run");
    let outputs = cpu.outputs().expect("CPU outputs must read");

    let mut position = 1.0_f32;
    let mut velocity = 0.0_f32;
    for _ in 0..3 {
        velocity = (velocity + (0.0 - position) * 0.5 * 0.1) * 0.9;
        position += velocity * 0.1;
    }
    assert!((outputs["result.0"][0] - position).abs() < 1.0e-6);
    assert!((outputs["result.1"][0] - velocity).abs() < 1.0e-6);
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
    let placement = GpuHost.plan(&artifact);
    assert!(!placement.fully_accelerated);
    assert!(placement.nodes.iter().any(|node| {
        node.target == ExecutionTarget::Cpu && node.reason.contains("no GPU lowering")
    }));
}

#[test]
fn mixed_graph_reports_gpu_regions_and_cpu_transfer_boundaries() {
    let artifact = compile_source(
        "sum := left + right\nquotient := sum / divisor\nresult := quotient * scale\nresult",
        [
            ("left", LegacyValue::F32(Ref::new(1.0))),
            ("right", LegacyValue::F32(Ref::new(2.0))),
            ("divisor", LegacyValue::F32(Ref::new(3.0))),
            ("scale", LegacyValue::F32(Ref::new(4.0))),
        ],
    );
    let placement = GpuHost.plan(&artifact);

    assert!(!placement.fully_accelerated);
    assert_eq!(placement.gpu_regions.len(), 2);
    assert_eq!(
        placement
            .nodes
            .iter()
            .filter(|node| node.target == ExecutionTarget::Cpu)
            .count(),
        1
    );
    assert!(placement.transfers.iter().any(|transfer| {
        transfer.direction == TransferDirection::Readback && transfer.consumer.is_some()
    }));
    assert!(placement.transfers.iter().any(|transfer| {
        transfer.direction == TransferDirection::Upload && transfer.consumer.is_some()
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
