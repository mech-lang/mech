use std::{collections::BTreeMap, env, time::Instant};

use mech_core::{LegacyValue, Ref, hash_str, matrix::Matrix};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::GpuHost;

const PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");

fn main() {
    let particles = env::args()
        .nth(1)
        .map(|argument| {
            argument
                .parse::<usize>()
                .expect("particle count must be an integer")
        })
        .unwrap_or(50_000);
    let cpu_turns = env::args()
        .nth(2)
        .map(|argument| {
            argument
                .parse::<usize>()
                .expect("CPU turn count must be an integer")
        })
        .unwrap_or(20)
        .max(1);
    let gpu_samples = env::args()
        .nth(3)
        .map(|argument| {
            argument
                .parse::<usize>()
                .expect("GPU sample count must be an integer")
        })
        .unwrap_or(5)
        .max(1);
    let elements = particles
        .checked_mul(2)
        .expect("particle count is too large");
    let positions = (0..elements)
        .map(|index| (index as f32 % 1024.0) / 1024.0 - 0.5)
        .collect::<Vec<_>>();
    let zeros = vec![0.0; elements];

    let compile_started = Instant::now();
    let artifact = compile_particle_artifact(particles, &positions, &zeros);
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("particle source must be admitted: {error}"));
    let compile_time = compile_started.elapsed();
    let inputs = BTreeMap::from([
        ("positions".to_owned(), positions),
        ("velocities".to_owned(), zeros.clone()),
        ("origin".to_owned(), zeros),
        ("attraction".to_owned(), vec![0.5]),
        ("drag".to_owned(), vec![0.999]),
        ("dt".to_owned(), vec![1.0 / 120.0]),
    ]);

    let reference = program.run_cpu(&inputs).expect("CPU warmup must run");
    let cpu_started = Instant::now();
    for _ in 0..cpu_turns {
        std::hint::black_box(program.run_cpu(&inputs).expect("CPU turn must run"));
    }
    let cpu_elapsed = cpu_started.elapsed();
    let cpu_per_turn = cpu_elapsed / cpu_turns as u32;

    let first_gpu = program
        .run_gpu_profiled(&inputs)
        .expect("GPU benchmark dispatch must run");
    let max_error = maximum_error(&reference, &first_gpu.outputs);
    assert!(
        max_error <= 1.0e-6,
        "GPU result differs from CPU by {max_error}"
    );
    let mut warm_samples = Vec::with_capacity(gpu_samples);
    for _ in 0..gpu_samples {
        warm_samples.push(
            program
                .run_gpu_profiled(&inputs)
                .expect("GPU benchmark dispatch must run"),
        );
    }
    warm_samples.sort_by_key(|profile| profile.total);
    let gpu = &warm_samples[warm_samples.len() / 2];
    println!("particles: {particles}");
    println!("elements per matrix: {elements}");
    println!("adapter: {}", first_gpu.adapter);
    println!("artifact + WGSL compile: {:.3} ms", millis(compile_time));
    println!(
        "CPU fused reference: {:.3} ms/turn ({cpu_turns} turns)",
        millis(cpu_per_turn)
    );
    println!("GPU cold one-shot total: {:.3} ms", millis(first_gpu.total));
    println!(
        "GPU warm one-shot median: {:.3} ms ({gpu_samples} samples)",
        millis(gpu.total)
    );
    println!("  adapter + device: {:.3} ms", millis(gpu.setup));
    println!(
        "  pipeline + upload: {:.3} ms",
        millis(gpu.pipeline_and_upload)
    );
    println!(
        "  dispatch + full readback: {:.3} ms",
        millis(gpu.dispatch_and_readback)
    );
    println!("GPU workgroups: {}", program.workgroup_count());
    println!("output matrices: {}", gpu.outputs.len());
    println!("maximum CPU/GPU absolute error: {max_error:.3e}");
}

fn compile_particle_artifact(
    particles: usize,
    positions: &[f32],
    zeros: &[f32],
) -> mech_engine::ProgramArtifact {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    let values = [
        (
            "host-positions",
            LegacyValue::MatrixF32(Matrix::from_vec(positions.to_vec(), particles, 2)),
        ),
        (
            "host-velocities",
            LegacyValue::MatrixF32(Matrix::from_vec(zeros.to_vec(), particles, 2)),
        ),
        (
            "host-origin",
            LegacyValue::MatrixF32(Matrix::from_vec(zeros.to_vec(), particles, 2)),
        ),
        ("host-attraction", LegacyValue::F32(Ref::new(0.5))),
        ("host-drag", LegacyValue::F32(Ref::new(0.999))),
        ("host-dt", LegacyValue::F32(Ref::new(1.0 / 120.0))),
    ];
    let symbols = program.interpreter().symbols();
    for (name, value) in values {
        let id = hash_str(name);
        symbols.borrow_mut().insert(id, value, false);
        symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name.to_owned());
    }
    program
        .run_string(PARTICLE_SOURCE)
        .expect("source must run");
    program
        .compile_program_product()
        .expect("source must compile")
        .into_parts()
        .0
}

fn millis(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn maximum_error(
    expected: &BTreeMap<String, Vec<f32>>,
    actual: &BTreeMap<String, Vec<f32>>,
) -> f32 {
    expected
        .iter()
        .flat_map(|(name, expected_values)| {
            let actual_values = actual.get(name).expect("GPU output name must match CPU");
            assert_eq!(expected_values.len(), actual_values.len());
            expected_values
                .iter()
                .zip(actual_values)
                .map(|(expected, actual)| (expected - actual).abs())
        })
        .fold(0.0, f32::max)
}
