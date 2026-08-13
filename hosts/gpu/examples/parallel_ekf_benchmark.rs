use std::{collections::BTreeMap, env, time::Instant};

use mech_core::{LegacyValue, Ref, hash_str};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::GpuHost;

const SOURCE: &str = include_str!("../fixtures/ekf-kernel.mec");

fn main() {
    let instances = argument(1, 100_000_usize);
    let cpu_turns = argument(2, 3_u32).max(1);
    let single_gpu_turns = argument(3, 20_u32).max(1);
    let batched_gpu_turns = argument(4, 120_u32).max(1);
    let validation_turns = 4;

    let compile_started = Instant::now();
    let artifact = compile_artifact();
    let instances_u32 = u32::try_from(instances).expect("filter count must fit u32");
    let program = GpuHost
        .compile_batched(&artifact, instances_u32)
        .unwrap_or_else(|error| panic!("generic EKF source must be admitted: {error}"));
    let compile_time = compile_started.elapsed();
    let inputs = inputs(instances);

    let mut cpu_validation = program.prepare_cpu(&inputs).unwrap();
    cpu_validation.dispatch_turns(validation_turns).unwrap();
    let expected = cpu_validation.state().clone();
    let mut gpu_validation = program.prepare_resident(&inputs).unwrap();
    let actual = gpu_validation.run_turns(validation_turns).unwrap();
    let max_error = maximum_error(&expected, &actual.state);
    assert!(
        max_error <= 1.0e-4,
        "GPU result differs from generic CPU lowering by {max_error}"
    );

    let mut cpu = program.prepare_cpu(&inputs).unwrap();
    cpu.dispatch_turns(1).unwrap();
    let cpu_started = Instant::now();
    cpu.dispatch_turns(cpu_turns).unwrap();
    let cpu_per_turn = cpu_started.elapsed() / cpu_turns;

    let prepare_started = Instant::now();
    let mut gpu = program.prepare_resident(&inputs).unwrap();
    let resident_prepare = prepare_started.elapsed();
    gpu.dispatch_turns(1).unwrap();
    let single_started = Instant::now();
    for _ in 0..single_gpu_turns {
        gpu.dispatch_turns(1).unwrap();
    }
    let single_per_turn = single_started.elapsed() / single_gpu_turns;
    let batched = gpu.dispatch_turns(batched_gpu_turns).unwrap() / batched_gpu_turns;
    let (readback, state) = gpu.read_state().unwrap();
    std::hint::black_box(state);

    println!("EKF instances: {instances}");
    println!("source artifact nodes: {}", artifact.nodes().len());
    println!("generated WGSL bytes: {}", program.wgsl().len());
    println!("GPU workgroups: {}", program.workgroup_count());
    println!("adapter: {}", actual.adapter);
    println!(
        "source + artifact + scalarization: {:.3} ms",
        millis(compile_time)
    );
    println!("resident GPU prepare: {:.3} ms", millis(resident_prepare));
    println!(
        "generic resident CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(cpu_per_turn)
    );
    println!(
        "resident GPU, one submission per turn: {:.3} ms/turn ({single_gpu_turns} turns)",
        millis(single_per_turn)
    );
    println!(
        "resident GPU, one batched submission: {:.3} ms/turn ({batched_gpu_turns} turns)",
        millis(batched)
    );
    println!(
        "CPU throughput: {:.3} million EKF-turns/s",
        throughput(instances, cpu_per_turn)
    );
    println!(
        "GPU single-submit throughput: {:.3} million EKF-turns/s",
        throughput(instances, single_per_turn)
    );
    println!(
        "GPU batched throughput: {:.3} million EKF-turns/s",
        throughput(instances, batched)
    );
    println!("final state readback: {:.3} ms", millis(readback));
    println!("maximum CPU/GPU absolute error: {max_error:.3e}");
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .map(|argument| {
            argument
                .parse()
                .ok()
                .expect("benchmark argument must parse")
        })
        .unwrap_or(default)
}

fn compile_artifact() -> mech_engine::ProgramArtifact {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    let symbols = program.interpreter().symbols();
    for (name, value) in [
        ("host-linear-velocity", 1.0_f32),
        ("host-angular-velocity", 0.015_f32),
        ("host-bearing", -0.55_f32),
    ] {
        let id = hash_str(name);
        symbols
            .borrow_mut()
            .insert(id, LegacyValue::F32(Ref::new(value)), false);
        symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name.to_owned());
    }
    program.run_string(SOURCE).expect("EKF source must run");
    program
        .compile_program_artifact()
        .expect("EKF source must compile to a typed artifact")
}

fn inputs(instances: usize) -> BTreeMap<String, Vec<f32>> {
    let denominator = instances.max(1) as f32;
    let linear_velocity = (0..instances)
        .map(|index| {
            let phase = std::f32::consts::TAU * index as f32 / denominator;
            1.0 + 0.05 * (phase * 3.0).sin()
        })
        .collect();
    let angular_velocity = (0..instances)
        .map(|index| {
            let phase = std::f32::consts::TAU * index as f32 / denominator;
            0.015 * (1.0 + 0.1 * (phase * 2.0).cos())
        })
        .collect();
    let bearing = (0..instances)
        .map(|index| {
            let phase = std::f32::consts::TAU * index as f32 / denominator;
            -0.55 + 0.01 * (phase * 7.0).sin() + 0.005 * (phase * 11.0).cos()
        })
        .collect();
    BTreeMap::from([
        ("dt".to_owned(), vec![0.1]),
        ("linear-velocity".to_owned(), linear_velocity),
        ("angular-velocity".to_owned(), angular_velocity),
        ("bearing".to_owned(), bearing),
        ("measurement-noise".to_owned(), vec![0.25]),
    ])
}

fn maximum_error(
    expected: &BTreeMap<mech_core::CellSlotId, Vec<f32>>,
    actual: &BTreeMap<mech_core::CellSlotId, Vec<f32>>,
) -> f32 {
    expected
        .iter()
        .flat_map(|(slot, expected)| {
            expected
                .iter()
                .zip(&actual[slot])
                .map(|(left, right)| (left - right).abs())
        })
        .fold(0.0_f32, f32::max)
}

fn millis(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn throughput(instances: usize, duration: std::time::Duration) -> f64 {
    instances as f64 / duration.as_secs_f64() / 1_000_000.0
}
