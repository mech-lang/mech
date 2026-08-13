use std::collections::BTreeMap;

use mech_core::{LegacyValue, Ref, ResolvedOperationContract, hash_str, matrix::Matrix};
use mech_engine::{MechProgram, MechProgramConfig, SlotRole};
use mech_gpu::{
    ExecutionTarget, GpuBindingRole, GpuDiagnosticCode, GpuHost, SlotResidence, TransferDirection,
};

const PARTICLE_SOURCE: &str = r#"
~positions := host-positions
~velocities := host-velocities
origin := host-origin
attraction := host-attraction
drag := host-drag
dt := host-dt
acceleration := (origin - positions) * attraction
next-velocities := (velocities + acceleration * dt) * drag
next-positions := positions + next-velocities * dt
velocities = next-velocities
positions = next-positions
(positions, velocities)
"#;

const STANDALONE_PARTICLE_SOURCE: &str = r#"
particle-count := 10f32
particle-index := 1f32..=particle-count
particle-x := (particle-index / particle-count) * 2f32 - 1f32
particle-y := particle-x * particle-x - 0.5<f32>
~positions := [particle-x; particle-y]
~velocities := [(0f32 - particle-y); particle-x] * 0.18<f32>
acceleration := (0f32 - positions) * 0.34<f32>
next-velocities := (velocities + acceleration * 0.008333333<f32>) * 0.997<f32>
next-positions := positions + next-velocities * 0.008333333<f32>
velocities = next-velocities
positions = next-positions
(positions, velocities)
"#;

const PROJECT_BOOTSTRAP: &str = include_str!("../../../include/project.js");
const PARTICLE_HTML: &str = include_str!("../../../examples/gpu-particles/index.html");
const SERVED_PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");

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
    assert!(
        artifact.constants().len() >= 2,
        "state initializers are retained alongside captured scalar constants"
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
            .all(|transfer| transfer.direction != TransferDirection::Upload),
        "source-captured constants do not create a per-turn upload boundary"
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
    assert_eq!(
        program
            .bindings()
            .iter()
            .filter(|binding| binding.role() == GpuBindingRole::StateRead)
            .count(),
        2
    );
    assert_eq!(
        program
            .bindings()
            .iter()
            .filter(|binding| binding.role() == GpuBindingRole::StateWrite)
            .count(),
        2
    );
    for (_, slot, elements) in program.outputs() {
        assert_eq!(elements, 8);
        assert!(program.bindings().iter().any(|binding| {
            binding.role() == GpuBindingRole::StateWrite && binding.slot() == slot
        }));
    }

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
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("standalone particle source must be admitted: {error}"));
    assert!(
        program
            .bindings()
            .iter()
            .all(|binding| binding.role() != GpuBindingRole::Input),
        "initialization-only source values must not become turn inputs"
    );
    let position_slot = program
        .outputs()
        .find_map(|(name, slot, _)| (name == "result.0").then_some(slot))
        .expect("particle position output must exist");
    assert_eq!(
        program.output_dimensions(position_slot),
        Some([2, 10].as_slice())
    );
    let mut cpu = program
        .prepare_cpu(&BTreeMap::new())
        .expect("standalone CPU executor must prepare");
    let initial = cpu.outputs().expect("standalone initial outputs must read");
    assert_eq!(initial["result.0"].len(), 20);
    assert_eq!(initial["result.1"].len(), 20);
    assert!(initial["result.0"].iter().any(|value| *value != 0.0));
    assert!(initial["result.1"].iter().any(|value| *value != 0.0));
    let expected_x = (1..=10)
        .map(|index| (index as f32 / 10.0) * 2.0 - 1.0)
        .collect::<Vec<_>>();
    let expected_positions = expected_x
        .iter()
        .copied()
        .chain(expected_x.iter().map(|x| x * x - 0.5))
        .collect::<Vec<_>>();
    assert_close(&initial["result.0"], &expected_positions);

    cpu.dispatch_turns(6)
        .expect("standalone CPU executor must advance");
    let cycled = cpu.outputs().expect("cycled outputs must read");
    assert_ne!(cycled["result.0"], initial["result.0"]);
    assert_ne!(cycled["result.1"], initial["result.1"]);
}

#[test]
fn ordinary_f64_source_lowers_to_the_explicit_f32_gpu_profile() {
    let artifact = compile_source("~state := [2.0 4.0]\nstate = state / 2.0\nstate", []);
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("relaxed f32 GPU profile must admit f64 source: {error}"));
    assert!(program.wgsl().contains(" / "));

    let mut cpu = program
        .prepare_cpu(&BTreeMap::new())
        .expect("relaxed-profile CPU reference must prepare");
    cpu.dispatch_turns(1).expect("one turn must execute");
    let outputs = cpu.outputs().expect("output must be readable");
    assert_eq!(outputs.len(), 1);
    assert_close(outputs.values().next().unwrap(), &[1.0, 2.0]);
}

#[test]
fn particle_example_uses_the_shared_project_and_gpu_shim() {
    assert!(PARTICLE_HTML.contains("src=\"/_mech/project.js\""));
    assert!(!PARTICLE_HTML.contains("particle-gpu.js"));
    assert!(PROJECT_BOOTSTRAP.contains("mech.compileGpuProgram(source)"));
    assert!(PROJECT_BOOTSTRAP.contains("data-mech-gpu-renderer"));
    assert!(PROJECT_BOOTSTRAP.contains("this.output.dimensions[1] === 2"));
    assert!(PROJECT_BOOTSTRAP.contains("maxRenderedPoints = 250_000"));
    assert!(PROJECT_BOOTSTRAP.contains("timings.artifactCompilation / 1000"));
    assert!(!PROJECT_BOOTSTRAP.contains("seedParticles"));
    assert!(!PROJECT_BOOTSTRAP.contains("host-positions"));
    assert!(SERVED_PARTICLE_SOURCE.contains("particle-count := 2000000f32"));
    assert!(!SERVED_PARTICLE_SOURCE.contains("host-"));
}

#[test]
fn served_particle_field_stays_bounded_without_damping() {
    let source = SERVED_PARTICLE_SOURCE.replacen("2000000f32", "512f32", 1);
    let artifact = compile_source(&source, []);
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("served particle source must be admitted: {error}"));

    assert!(!program.wgsl().contains("sin("));
    assert!(!program.wgsl().contains("cos("));
    assert!(!program.wgsl().contains("%"));

    let inputs = BTreeMap::from([
        ("origin".to_owned(), vec![0.0]),
        ("attraction".to_owned(), vec![0.45]),
        ("nonlinear-attraction".to_owned(), vec![0.65]),
        ("dt".to_owned(), vec![0.02]),
    ]);
    let mut cpu = program
        .prepare_cpu(&inputs)
        .expect("particle field must prepare from its captured constants");
    let initial = cpu.outputs().expect("initial particle field must read");
    let initial_radius = root_mean_square_radius(&initial["result.0"]);

    cpu.dispatch_turns(2_000)
        .expect("conservative particle field must advance");
    let evolved = cpu.outputs().expect("evolved particle field must read");
    let evolved_radius = root_mean_square_radius(&evolved["result.0"]);

    assert!(evolved["result.0"].iter().all(|value| value.is_finite()));
    assert!(evolved["result.0"].iter().all(|value| value.abs() < 2.0));
    assert!(
        evolved_radius > initial_radius * 0.8,
        "particle field collapsed: initial RMS radius {initial_radius}, evolved {evolved_radius}"
    );
}

fn root_mean_square_radius(positions: &[f32]) -> f32 {
    let particles = positions.len() / 2;
    let squared_radius = (0..particles)
        .map(|index| {
            let x = positions[index];
            let y = positions[particles + index];
            x * x + y * y
        })
        .sum::<f32>();
    (squared_radius / particles as f32).sqrt()
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
        "+> math\nanswer := math/sin(left)",
        [("left", LegacyValue::F32(Ref::new(1.0)))],
    );
    let error = GpuHost
        .compile(&artifact)
        .expect_err("sine is outside the fused element-wise GPU capability set");

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
        "+> math\nsum := left + right\ncurved := math/sin(sum)\nresult := curved * scale\nresult",
        [
            ("left", LegacyValue::F32(Ref::new(1.0))),
            ("right", LegacyValue::F32(Ref::new(2.0))),
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
