use std::{collections::BTreeMap, env, fs, hint::black_box, time::Duration, time::Instant};

use mech_core::{Body, MechCode, Program, Section, SectionElement};
use mech_gpu::{ComputeLowerer, GpuBindingRole};
use mech_runtime::RuntimeBuilder;

const PARTICLE_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");

fn main() {
    let particles = argument(1, 1_000_000usize);
    let cpu_samples = argument(2, 20usize).max(1);
    let gpu_samples = argument(3, 120usize).max(1);
    let timeline_seconds = argument(4, 0u64);
    let timeline_path = env::args().nth(5);
    let source = PARTICLE_SOURCE.replacen("1000000f32", &format!("{particles}f32"), 1);

    let compile_started = Instant::now();
    let tree = isolated_compute_tree(&source);
    let artifact = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .compile_tree_artifact(&tree)
        .expect("the particle compute region must compile")
        .into_artifact();
    let program = ComputeLowerer
        .compile(&artifact)
        .expect("the neutral compute region must lower");
    let inputs = default_inputs(&program);
    let compile_elapsed = compile_started.elapsed();

    let mut cpu_validation = program
        .prepare_cpu(&inputs)
        .expect("CPU validation session must prepare");
    cpu_validation
        .dispatch_turns(1)
        .expect("CPU validation turn must run");
    let cpu_validation_output = cpu_validation
        .outputs()
        .expect("CPU validation output must read");
    let mut gpu_validation = program
        .prepare_resident(&inputs)
        .expect("GPU validation session must prepare");
    gpu_validation
        .dispatch_turns(1)
        .expect("GPU validation turn must run");
    let (_, gpu_validation_output) = gpu_validation
        .read_outputs()
        .expect("GPU validation output must read");
    let maximum_error = maximum_error(&cpu_validation_output, &gpu_validation_output);

    if timeline_seconds > 0 {
        let path = timeline_path.expect("timeline output path is required");
        write_timeline(
            &program,
            &inputs,
            particles,
            Duration::from_secs(timeline_seconds),
            &path,
        );
        println!("timeline: {path}");
        println!("maximum CPU/GPU absolute error: {maximum_error:.3e}");
        return;
    }

    let mut cpu = program
        .prepare_cpu(&inputs)
        .expect("resident CPU session must prepare");
    cpu.dispatch_turns(1).expect("CPU warmup must run");
    let mut cpu_compute = Vec::with_capacity(cpu_samples);
    for _ in 0..cpu_samples {
        let started = Instant::now();
        cpu.dispatch_turns(1).expect("CPU turn must run");
        cpu_compute.push(started.elapsed());
    }

    let mut cpu_with_snapshot = program
        .prepare_cpu(&inputs)
        .expect("resident CPU snapshot session must prepare");
    cpu_with_snapshot
        .dispatch_turns(1)
        .expect("CPU snapshot warmup must run");
    let mut cpu_snapshot = Vec::with_capacity(cpu_samples);
    for _ in 0..cpu_samples {
        let started = Instant::now();
        cpu_with_snapshot
            .dispatch_turns(1)
            .expect("CPU snapshot turn must run");
        black_box(
            cpu_with_snapshot
                .output("result.0")
                .expect("position snapshot must read"),
        );
        cpu_snapshot.push(started.elapsed());
    }

    let gpu_prepare_started = Instant::now();
    let mut gpu = program
        .prepare_resident(&inputs)
        .expect("resident GPU session must prepare");
    let gpu_prepare = gpu_prepare_started.elapsed();
    gpu.dispatch_turns(1).expect("GPU warmup must run");
    let mut gpu_synchronized = Vec::with_capacity(gpu_samples);
    for _ in 0..gpu_samples {
        gpu_synchronized.push(gpu.dispatch_turns(1).expect("GPU turn must run"));
    }

    let mut gpu_batched = program
        .prepare_resident(&inputs)
        .expect("batched GPU session must prepare");
    gpu_batched
        .dispatch_turns(1)
        .expect("batched GPU warmup must run");
    let gpu_batch = gpu_batched
        .dispatch_turns(gpu_samples as u32)
        .expect("batched GPU turns must run");
    let readback_started = Instant::now();
    black_box(
        gpu_batched
            .read_outputs()
            .expect("final GPU outputs must read"),
    );
    let gpu_readback = readback_started.elapsed();

    println!("Mech particle compute backend benchmark");
    println!("particles: {particles}");
    println!("adapter: {}", gpu.adapter());
    println!("artifact compile: {:.3} ms", milliseconds(compile_elapsed));
    println!("CPU samples: {cpu_samples}");
    print_samples("CPU resident compute", particles, &mut cpu_compute);
    print_samples(
        "CPU compute + position snapshot",
        particles,
        &mut cpu_snapshot,
    );
    println!("GPU prepare: {:.3} ms", milliseconds(gpu_prepare));
    println!("GPU samples: {gpu_samples}");
    print_samples(
        "GPU synchronized resident turn",
        particles,
        &mut gpu_synchronized,
    );
    println!(
        "GPU batched resident: {:.3} ms/turn, {:.3} million particle-turns/s",
        milliseconds(gpu_batch) / gpu_samples as f64,
        throughput(particles, gpu_batch / gpu_samples as u32),
    );
    println!(
        "GPU final full readback: {:.3} ms",
        milliseconds(gpu_readback)
    );
    println!("maximum CPU/GPU absolute error: {maximum_error:.3e}");
    println!(
        "position snapshot bytes: {}",
        particles * 2 * size_of::<f32>()
    );
}

fn argument<T>(index: usize, default: T) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    env::args()
        .nth(index)
        .map(|value| value.parse().expect("benchmark argument must parse"))
        .unwrap_or(default)
}

fn isolated_compute_tree(source: &str) -> Program {
    let tree = mech_syntax::parse(source).expect("complete particle source must parse");
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
    let region = tree
        .body
        .sections
        .iter()
        .find(|section| !section.annotations.is_empty())
        .expect("particle source must contain a compute region")
        .clone();
    Program {
        title: None,
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    annotations: Vec::new(),
                    elements: imports,
                },
                region,
            ],
        },
    }
}

fn default_inputs(program: &mech_gpu::ElementwiseKernel) -> BTreeMap<String, Vec<f32>> {
    program
        .bindings()
        .iter()
        .filter(|binding| binding.role() == GpuBindingRole::Input)
        .map(|binding| (binding.name.clone(), vec![0.0; binding.elements as usize]))
        .collect()
}

fn maximum_error(
    expected: &BTreeMap<String, Vec<f32>>,
    actual: &BTreeMap<String, Vec<f32>>,
) -> f32 {
    expected
        .iter()
        .flat_map(|(name, expected)| {
            expected
                .iter()
                .zip(&actual[name])
                .map(|(expected, actual)| (expected - actual).abs())
        })
        .fold(0.0, f32::max)
}

fn print_samples(label: &str, particles: usize, samples: &mut [Duration]) {
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let p95 = samples[(samples.len() * 95 / 100).min(samples.len() - 1)];
    println!(
        "{label}: median {:.3} ms/turn, p95 {:.3} ms/turn, {:.3} million particle-turns/s",
        milliseconds(median),
        milliseconds(p95),
        throughput(particles, median),
    );
}

fn write_timeline(
    program: &mech_gpu::ElementwiseKernel,
    inputs: &BTreeMap<String, Vec<f32>>,
    particles: usize,
    duration: Duration,
    path: &str,
) {
    let mut rows = String::from(
        "backend,elapsed_seconds,window_turns,window_seconds,throughput_million_per_second\n",
    );

    let mut cpu = program
        .prepare_cpu(inputs)
        .expect("timeline CPU session must prepare");
    cpu.dispatch_turns(1).expect("timeline CPU warmup must run");
    let cpu_started = Instant::now();
    while cpu_started.elapsed() < duration {
        let window_started = Instant::now();
        cpu.dispatch_turns(1).expect("timeline CPU turn must run");
        let window = window_started.elapsed();
        rows.push_str(&format!(
            "cpu,{:.6},1,{:.6},{:.6}\n",
            cpu_started.elapsed().as_secs_f64(),
            window.as_secs_f64(),
            throughput(particles, window),
        ));
    }

    let mut gpu = program
        .prepare_resident(inputs)
        .expect("timeline GPU session must prepare");
    gpu.dispatch_turns(1).expect("timeline GPU warmup must run");
    let gpu_started = Instant::now();
    const GPU_WINDOW_TURNS: u32 = 100;
    while gpu_started.elapsed() < duration {
        let window_started = Instant::now();
        for _ in 0..GPU_WINDOW_TURNS {
            gpu.dispatch_turns(1)
                .expect("timeline synchronized GPU turn must run");
        }
        let window = window_started.elapsed();
        rows.push_str(&format!(
            "gpu,{:.6},{GPU_WINDOW_TURNS},{:.6},{:.6}\n",
            gpu_started.elapsed().as_secs_f64(),
            window.as_secs_f64(),
            particles as f64 * f64::from(GPU_WINDOW_TURNS) / window.as_secs_f64() / 1_000_000.0,
        ));
    }

    fs::write(path, rows).expect("timeline CSV must write");
}

fn throughput(particles: usize, duration: Duration) -> f64 {
    particles as f64 / duration.as_secs_f64() / 1_000_000.0
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
