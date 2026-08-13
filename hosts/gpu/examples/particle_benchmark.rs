use std::{collections::BTreeMap, env, time::Instant};

use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::{GpuHost, GpuProgram};

const PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");
const PARTICLE_COUNT_DECLARATION: &str = "particle-count := 2000000f32";

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
    let resident_turns = env::args()
        .nth(4)
        .map(|argument| {
            argument
                .parse::<u32>()
                .expect("resident turn count must be an integer")
        })
        .unwrap_or(120)
        .max(1);
    let elements = particles
        .checked_mul(2)
        .expect("particle count is too large");
    let compile_started = Instant::now();
    let artifact = compile_particle_artifact(particles);
    let program = GpuHost
        .compile(&artifact)
        .unwrap_or_else(|error| panic!("particle source must be admitted: {error}"));
    let compile_time = compile_started.elapsed();
    assert_eq!(program.dispatch_elements(), elements as u64);
    let inputs = BTreeMap::new();
    let (initial_positions, initial_velocities) = initial_particle_state(&program);

    let reference = program.run_cpu(&inputs).expect("CPU warmup must run");
    let mut resident_cpu = program
        .prepare_cpu(&inputs)
        .expect("resident CPU session must prepare");
    let cpu_started = Instant::now();
    resident_cpu
        .dispatch_turns(cpu_turns as u32)
        .expect("resident CPU turns must run");
    std::hint::black_box(resident_cpu.outputs().expect("CPU outputs must read"));
    let cpu_elapsed = cpu_started.elapsed();
    let cpu_per_turn = cpu_elapsed / cpu_turns as u32;

    println!("particles: {particles}");
    println!("elements per state matrix: {elements}");
    println!("artifact + WGSL compile: {:.3} ms", millis(compile_time));
    println!(
        "CPU fused reference: {:.3} ms/turn ({cpu_turns} turns)",
        millis(cpu_per_turn)
    );

    let first_gpu = match program.run_gpu_profiled(&inputs) {
        Ok(profile) => profile,
        Err(error) => {
            println!("GPU unavailable: {error}");
            return;
        }
    };
    let max_error = maximum_error(&reference, &first_gpu.outputs);
    assert!(
        max_error <= 1.0e-6,
        "GPU result differs from CPU by {max_error}"
    );
    let adapter = first_gpu.adapter.clone();
    let cold_total = first_gpu.total;
    drop(reference);
    drop(first_gpu);
    let mut warm_samples = Vec::with_capacity(gpu_samples);
    for _ in 0..gpu_samples {
        let profile = program
            .run_gpu_profiled(&inputs)
            .expect("GPU benchmark dispatch must run");
        warm_samples.push((
            profile.total,
            profile.setup,
            profile.pipeline_and_upload,
            profile.dispatch_and_readback,
        ));
    }
    warm_samples.sort_by_key(|profile| profile.0);
    let gpu = &warm_samples[warm_samples.len() / 2];

    let resident_prepare_started = Instant::now();
    let mut resident = program
        .prepare_resident(&inputs)
        .expect("resident GPU session must prepare");
    let resident_prepare = resident_prepare_started.elapsed();
    let resident = resident
        .run_turns(resident_turns)
        .expect("resident GPU turns must run");
    let resident_error = resident_sample_error(
        &initial_positions,
        &initial_velocities,
        &resident.outputs,
        resident_turns,
    );
    assert!(
        resident_error <= 1.0e-4,
        "resident GPU result differs from sampled recurrence by {resident_error}"
    );
    println!("adapter: {adapter}");
    println!("GPU cold one-shot total: {:.3} ms", millis(cold_total));
    println!(
        "GPU warm one-shot median: {:.3} ms ({gpu_samples} samples)",
        millis(gpu.0)
    );
    println!("  adapter + device: {:.3} ms", millis(gpu.1));
    println!("  pipeline + upload: {:.3} ms", millis(gpu.2));
    println!("  dispatch + full readback: {:.3} ms", millis(gpu.3));
    println!("resident prepare: {:.3} ms", millis(resident_prepare));
    println!(
        "resident dispatch: {:.3} ms/turn ({resident_turns} turns)",
        millis(resident.dispatch) / f64::from(resident_turns)
    );
    println!(
        "resident throughput: {:.3} million particle-turns/s",
        particles as f64 * f64::from(resident_turns)
            / resident.dispatch.as_secs_f64()
            / 1_000_000.0
    );
    println!(
        "resident final readback: {:.3} ms",
        millis(resident.readback)
    );
    println!("GPU workgroups: {}", program.workgroup_count());
    println!("output matrices: {}", resident.outputs.len());
    println!("maximum CPU/GPU absolute error: {max_error:.3e}");
    println!("maximum resident sampled absolute error: {resident_error:.3e}");
}

fn compile_particle_artifact(particles: usize) -> mech_engine::ProgramArtifact {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    assert!(
        PARTICLE_SOURCE.contains(PARTICLE_COUNT_DECLARATION),
        "particle count declaration changed"
    );
    let replacement = format!("particle-count := {particles}f32");
    let source = PARTICLE_SOURCE.replacen(PARTICLE_COUNT_DECLARATION, &replacement, 1);
    program.run_string(&source).expect("source must run");
    program
        .compile_program_product()
        .expect("source must compile")
        .into_parts()
        .0
}

fn initial_particle_state(program: &GpuProgram) -> (Vec<f32>, Vec<f32>) {
    let initializers = program
        .state_initializers()
        .map(|(slot, _, values)| (slot, values))
        .collect::<BTreeMap<_, _>>();
    let outputs = program
        .outputs()
        .map(|(name, slot, _)| (name, slot))
        .collect::<BTreeMap<_, _>>();
    let positions = initializers[&outputs["result.0"]].to_vec();
    let velocities = initializers[&outputs["result.1"]].to_vec();
    (positions, velocities)
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

fn resident_sample_error(
    initial_positions: &[f32],
    initial_velocities: &[f32],
    outputs: &BTreeMap<String, Vec<f32>>,
    turns: u32,
) -> f32 {
    let positions = &outputs["result.0"];
    let velocities = &outputs["result.1"];
    let stride = (initial_positions.len() / 1024).max(1);
    (0..initial_positions.len())
        .step_by(stride)
        .take(1024)
        .map(|index| {
            let mut position = initial_positions[index];
            let mut velocity = initial_velocities[index];
            for _ in 0..turns {
                let acceleration = -position * (0.45 + position * position * 0.65);
                velocity += acceleration * 0.02;
                position += velocity * 0.02;
            }
            (positions[index] - position)
                .abs()
                .max((velocities[index] - velocity).abs())
        })
        .fold(0.0, f32::max)
}
