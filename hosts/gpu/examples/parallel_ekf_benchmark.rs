use std::{collections::BTreeMap, env, time::Instant};

use mech_core::{Body, MechCode, Program, Section, SectionElement};
use mech_engine::ProgramArtifact;
use mech_gpu::ComputeLowerer;
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

const SOURCE: &str = include_str!("../fixtures/ekf-kernel.mec");
const COMPUTE_INPUT_NAMES: [&str; 7] = [
    "dt",
    "linear-velocity",
    "angular-velocity",
    "bearing",
    "measurement-noise",
    "finite-limit",
    "covariance-symmetry-tolerance",
];

fn main() {
    let requested_instances = argument(1, 100_000_usize).max(1);
    let cpu_turns = argument(2, 3_u32).max(1);
    let single_gpu_turns = argument(3, 20_u32).max(1);
    let checked_gpu_turns = argument(4, 120_u32).max(1);
    let validation_turns = 4;
    let jit_workers = 8;

    #[cfg(feature = "metal-native")]
    if env::var_os("MECH_METAL_ONLY").is_some() {
        run_metal_only(requested_instances, checked_gpu_turns);
        return;
    }

    let compile_started = Instant::now();
    let tree = source_tree(requested_instances);
    let driver = evaluate_driver(&tree);
    let artifact = compile_artifact(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap_or_else(|error| panic!("generic EKF source must be admitted: {error}"));
    let unchecked_program = program
        .without_integrity_constraints()
        .expect("unchecked control must preserve the fixed-shape plan");
    assert_eq!(
        program.integrity_constraints().count(),
        3,
        "the Mech artifact must carry all robot-state constraints"
    );
    let constraint_names = program
        .named_integrity_constraints()
        .map(|(_, name)| name)
        .collect::<Vec<_>>();
    let compile_time = compile_started.elapsed();
    let instances = program.instances() as usize;
    assert_eq!(
        instances, requested_instances,
        "the accelerator must infer the Mech array extent"
    );

    let mut cpu_validation = program.prepare_cpu(&inputs).unwrap();
    cpu_validation.dispatch_turns(validation_turns).unwrap();
    let expected = cpu_validation.state().clone();
    let mut simd_validation = program.prepare_simd_cpu(&inputs).unwrap();
    simd_validation.dispatch_turns(validation_turns).unwrap();
    let simd_max_error = maximum_error(&expected, simd_validation.state());
    assert!(
        simd_max_error <= 1.0e-4,
        "SIMD result differs from scalar CPU lowering by {simd_max_error}"
    );
    let mut jit_validation = program.prepare_jit_cpu(&inputs).unwrap();
    jit_validation.dispatch_turns(validation_turns).unwrap();
    let jit_max_error = maximum_error(&expected, jit_validation.state());
    assert!(
        jit_max_error <= 1.0e-4,
        "JIT result differs from scalar CPU lowering by {jit_max_error}"
    );
    let mut jit_parallel_validation = program.prepare_jit_cpu(&inputs).unwrap();
    jit_parallel_validation
        .dispatch_turns_parallel(validation_turns, jit_workers)
        .unwrap();
    let jit_parallel_max_error = maximum_error(&expected, jit_parallel_validation.state());
    assert!(
        jit_parallel_max_error <= 1.0e-4,
        "parallel JIT result differs from scalar CPU lowering by {jit_parallel_max_error}"
    );
    let mut gpu_validation = program.prepare_resident(&inputs).unwrap();
    let actual = gpu_validation.run_turns(validation_turns).unwrap();
    let max_error = maximum_error(&expected, &actual.state);
    assert!(
        max_error <= 1.0e-4,
        "GPU result differs from generic CPU lowering by {max_error}"
    );

    let mut cpu_warmup = program.prepare_cpu(&inputs).unwrap();
    cpu_warmup.dispatch_turns(5).unwrap();
    let mut cpu = program.prepare_cpu(&inputs).unwrap();
    let cpu_started = Instant::now();
    cpu.dispatch_turns(cpu_turns).unwrap();
    let cpu_per_turn = cpu_started.elapsed() / cpu_turns;
    let cpu_checksum = state_checksum(cpu.state());

    let mut simd_warmup = program.prepare_simd_cpu(&inputs).unwrap();
    simd_warmup.dispatch_turns(5).unwrap();
    let mut simd = program.prepare_simd_cpu(&inputs).unwrap();
    let simd_started = Instant::now();
    simd.dispatch_turns(cpu_turns).unwrap();
    let simd_per_turn = simd_started.elapsed() / cpu_turns;
    let simd_checksum = state_checksum(simd.state());

    let mut jit_warmup = program.prepare_jit_cpu(&inputs).unwrap();
    jit_warmup.dispatch_turns(5).unwrap();
    let jit_prepare_started = Instant::now();
    let mut jit = program.prepare_jit_cpu(&inputs).unwrap();
    let jit_prepare = jit_prepare_started.elapsed();
    let jit_started = Instant::now();
    jit.dispatch_turns(cpu_turns).unwrap();
    let jit_per_turn = jit_started.elapsed() / cpu_turns;
    let jit_checksum = state_checksum(jit.state());

    let mut jit_unchecked_warmup = unchecked_program.prepare_jit_cpu(&inputs).unwrap();
    jit_unchecked_warmup.dispatch_turns(5).unwrap();
    let mut jit_unchecked = unchecked_program.prepare_jit_cpu(&inputs).unwrap();
    let jit_unchecked_started = Instant::now();
    jit_unchecked.dispatch_turns(cpu_turns).unwrap();
    let jit_unchecked_per_turn = jit_unchecked_started.elapsed() / cpu_turns;
    let jit_unchecked_checksum = state_checksum(jit_unchecked.state());

    let mut jit_parallel_warmup = program.prepare_jit_cpu(&inputs).unwrap();
    jit_parallel_warmup
        .dispatch_turns_parallel(5, jit_workers)
        .unwrap();
    let mut jit_parallel = program.prepare_jit_cpu(&inputs).unwrap();
    let jit_parallel_started = Instant::now();
    jit_parallel
        .dispatch_turns_parallel(cpu_turns, jit_workers)
        .unwrap();
    let jit_parallel_per_turn = jit_parallel_started.elapsed() / cpu_turns;
    let jit_parallel_checksum = state_checksum(jit_parallel.state());

    let mut jit_parallel_unchecked_warmup = unchecked_program.prepare_jit_cpu(&inputs).unwrap();
    jit_parallel_unchecked_warmup
        .dispatch_turns_parallel(5, jit_workers)
        .unwrap();
    let mut jit_parallel_unchecked = unchecked_program.prepare_jit_cpu(&inputs).unwrap();
    let jit_parallel_unchecked_started = Instant::now();
    jit_parallel_unchecked
        .dispatch_turns_parallel(cpu_turns, jit_workers)
        .unwrap();
    let jit_parallel_unchecked_per_turn = jit_parallel_unchecked_started.elapsed() / cpu_turns;
    let jit_parallel_unchecked_checksum = state_checksum(jit_parallel_unchecked.state());

    let prepare_started = Instant::now();
    let mut gpu = program.prepare_resident(&inputs).unwrap();
    let resident_prepare = prepare_started.elapsed();
    gpu.dispatch_turns(1).unwrap();
    let single_started = Instant::now();
    for _ in 0..single_gpu_turns {
        gpu.dispatch_turns(1).unwrap();
    }
    let single_per_turn = single_started.elapsed() / single_gpu_turns;
    let checked_repeated = gpu.dispatch_turns(checked_gpu_turns).unwrap() / checked_gpu_turns;
    let (readback, state) = gpu.read_state().unwrap();
    std::hint::black_box(state);
    let mut gpu_unchecked = unchecked_program.prepare_resident(&inputs).unwrap();
    gpu_unchecked.dispatch_turns(1).unwrap();
    let gpu_unchecked_started = Instant::now();
    for _ in 0..checked_gpu_turns {
        gpu_unchecked
            .dispatch_turns(1)
            .expect("unchecked GPU turns must dispatch");
    }
    let gpu_unchecked_per_turn = gpu_unchecked_started.elapsed() / checked_gpu_turns;
    let (_, gpu_unchecked_state) = gpu_unchecked.read_state().unwrap();
    let gpu_unchecked_checksum = state_checksum(&gpu_unchecked_state);

    #[cfg(feature = "metal-native")]
    let (
        metal_checked_per_turn,
        metal_unchecked_per_turn,
        metal_checked_state,
        metal_unchecked_state,
    ) = {
        let mut metal_checked = program.prepare_metal(&inputs).unwrap();
        metal_checked.dispatch_turns(5).unwrap();
        let mut metal_checked = program.prepare_metal(&inputs).unwrap();
        let started = Instant::now();
        metal_checked.dispatch_turns(checked_gpu_turns).unwrap();
        let checked_per_turn = started.elapsed() / checked_gpu_turns;
        let checked_state = metal_checked.read_state().unwrap();

        let mut metal_unchecked = unchecked_program.prepare_metal(&inputs).unwrap();
        metal_unchecked.dispatch_turns(5).unwrap();
        let mut metal_unchecked = unchecked_program.prepare_metal(&inputs).unwrap();
        let started = Instant::now();
        metal_unchecked.dispatch_turns(checked_gpu_turns).unwrap();
        let unchecked_per_turn = started.elapsed() / checked_gpu_turns;
        let unchecked_state = metal_unchecked.read_state().unwrap();
        (
            checked_per_turn,
            unchecked_per_turn,
            checked_state,
            unchecked_state,
        )
    };
    #[cfg(feature = "metal-native")]
    let (metal_checked_max_error, metal_unchecked_max_error) = {
        let mut reference = program.prepare_cpu(&inputs).unwrap();
        reference.dispatch_turns(checked_gpu_turns).unwrap();
        (
            maximum_error(reference.state(), &metal_checked_state),
            maximum_error(reference.state(), &metal_unchecked_state),
        )
    };

    println!("EKF instances: {instances}");
    println!("batch extent authority: Mech input arrays");
    println!("source artifact nodes: {}", artifact.nodes().len());
    println!("generated WGSL bytes: {}", program.wgsl().len());
    if env::var_os("MECH_DUMP_WGSL").is_some() {
        println!(
            "--- generated WGSL ---\n{}--- end generated WGSL ---",
            program.wgsl()
        );
    }
    if env::var_os("MECH_DUMP_IR").is_some() {
        println!(
            "--- fixed scalar IR ---\n{:#?}\n--- end fixed scalar IR ---",
            program.compute_program().kernel()
        );
    }
    println!("GPU workgroups: {}", program.workgroup_count());
    println!("CPU SIMD width: {} f32 lanes", program.simd_lanes());
    println!(
        "Mech integrity constraints: {}",
        constraint_names.join(", ")
    );
    println!(
        "integrity failure: reject candidate, retain previous published estimate, record latest fault + count"
    );
    println!("adapter: {}", actual.adapter);
    println!(
        "source + artifact + scalarization: {:.3} ms",
        millis(compile_time)
    );
    println!("resident GPU prepare: {:.3} ms", millis(resident_prepare));
    println!("Cranelift JIT prepare: {:.3} ms", millis(jit_prepare));
    println!(
        "Mech scalar CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(cpu_per_turn)
    );
    println!(
        "Mech SIMD CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(simd_per_turn)
    );
    println!(
        "Mech Cranelift JIT CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_per_turn)
    );
    println!(
        "Mech Cranelift JIT CPU unchecked: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_unchecked_per_turn)
    );
    println!(
        "Mech Cranelift JIT CPU, {jit_workers} workers: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_parallel_per_turn)
    );
    println!(
        "Mech Cranelift JIT CPU unchecked, {jit_workers} workers: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_parallel_unchecked_per_turn)
    );
    println!(
        "resident GPU, one submission per turn: {:.3} ms/turn ({single_gpu_turns} turns)",
        millis(single_per_turn)
    );
    println!(
        "resident GPU, checked repeated turns: {:.3} ms/turn ({checked_gpu_turns} turns)",
        millis(checked_repeated)
    );
    println!(
        "Mech scalar throughput: {:.3} million EKF-turns/s",
        throughput(instances, cpu_per_turn)
    );
    println!(
        "Mech SIMD throughput: {:.3} million EKF-turns/s",
        throughput(instances, simd_per_turn)
    );
    println!(
        "Mech Cranelift JIT throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_per_turn)
    );
    println!(
        "Mech Cranelift JIT unchecked throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_unchecked_per_turn)
    );
    println!(
        "Mech Cranelift JIT, {jit_workers} workers throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_parallel_per_turn)
    );
    println!(
        "Mech Cranelift JIT unchecked, {jit_workers} workers throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_parallel_unchecked_per_turn)
    );
    println!(
        "GPU single-submit throughput: {:.3} million EKF-turns/s",
        throughput(instances, single_per_turn)
    );
    println!(
        "GPU checked repeated throughput: {:.3} million EKF-turns/s",
        throughput(instances, checked_repeated)
    );
    println!(
        "GPU unchecked repeated throughput: {:.3} million EKF-turns/s",
        throughput(instances, gpu_unchecked_per_turn)
    );
    #[cfg(feature = "metal-native")]
    {
        println!(
            "Mech direct Metal checked throughput: {:.3} million EKF-turns/s",
            throughput(instances, metal_checked_per_turn)
        );
        println!(
            "Mech direct Metal unchecked throughput: {:.3} million EKF-turns/s",
            throughput(instances, metal_unchecked_per_turn)
        );
    }
    println!("final state readback: {:.3} ms", millis(readback));
    println!("maximum CPU/GPU absolute error: {max_error:.3e}");
    println!("maximum scalar/SIMD absolute error: {simd_max_error:.3e}");
    println!("maximum scalar/JIT absolute error: {jit_max_error:.3e}");
    println!("maximum scalar/parallel JIT absolute error: {jit_parallel_max_error:.3e}");
    println!("Mech scalar checksum: {cpu_checksum:.9}");
    println!("Mech SIMD checksum: {simd_checksum:.9}");
    println!("Mech Cranelift JIT checksum: {jit_checksum:.9}");
    println!("Mech Cranelift JIT unchecked checksum: {jit_unchecked_checksum:.9}");
    println!("Mech Cranelift JIT, {jit_workers} workers checksum: {jit_parallel_checksum:.9}");
    println!(
        "Mech Cranelift JIT unchecked, {jit_workers} workers checksum: {jit_parallel_unchecked_checksum:.9}"
    );
    println!("Mech GPU unchecked checksum: {gpu_unchecked_checksum:.9}");
    #[cfg(feature = "metal-native")]
    {
        println!(
            "maximum scalar/direct Metal checked absolute error: {metal_checked_max_error:.3e}"
        );
        println!(
            "maximum scalar/direct Metal unchecked absolute error: {metal_unchecked_max_error:.3e}"
        );
        println!(
            "Mech direct Metal unchecked checksum: {:.9}",
            state_checksum(&metal_unchecked_state)
        );
    }
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

fn source_tree(instances: usize) -> Program {
    let source = SOURCE.replacen("100000f32", &format!("{instances}f32"), 1);
    mech_syntax::parse(&source).expect("EKF array source must parse")
}

fn evaluate_driver(tree: &Program) -> BTreeMap<String, Vec<f32>> {
    const INPUTS: [&str; 7] = [
        "lane-dt",
        "lane-linear-velocity",
        "lane-angular-velocity",
        "lane-bearing",
        "lane-measurement-noise",
        "finite-limit",
        "covariance-symmetry-tolerance",
    ];
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .evaluate_static_tree_symbols(&projected_tree(tree, false), &INPUTS)
        .expect("Mech EKF array inputs must evaluate")
        .into_iter()
        .map(|(name, value)| {
            let values = match value {
                RuntimeHostInputValue::F32(value) => vec![value],
                RuntimeHostInputValue::F32Matrix { values, .. } => values,
                value => panic!("{name} must contain f32 values, found {value:?}"),
            };
            (name, values)
        })
        .collect()
}

fn compile_artifact(tree: &Program, driver: &BTreeMap<String, Vec<f32>>) -> ProgramArtifact {
    let scalar_inputs = driver
        .iter()
        .map(|(name, values)| {
            (
                name.clone(),
                RuntimeHostInputValue::F32(*values.first().expect("driver input is non-empty")),
            )
        })
        .collect();
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("source compiler must build")
        .compile_tree_artifact_with_inputs(
            &projected_tree(tree, true),
            &scalar_inputs,
            &COMPUTE_INPUT_NAMES.into_iter().map(str::to_owned).collect(),
        )
        .expect("EKF source must compile to a typed artifact")
        .into_artifact()
}

fn source_inputs(
    driver: &BTreeMap<String, Vec<f32>>,
    artifact: &ProgramArtifact,
) -> BTreeMap<String, Vec<f32>> {
    artifact
        .inputs()
        .iter()
        .filter_map(|input| {
            let driver_name = match input.name.as_str() {
                "dt" => "lane-dt",
                "linear-velocity" => "lane-linear-velocity",
                "angular-velocity" => "lane-angular-velocity",
                "bearing" => "lane-bearing",
                "measurement-noise" => "lane-measurement-noise",
                name => name,
            };
            driver
                .get(driver_name)
                .map(|values| (input.name.clone(), values.clone()))
        })
        .collect()
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
        .find(|section| (!section.annotations.is_empty()) == compute)
        .unwrap_or_else(|| panic!("EKF source must contain the selected section"))
        .clone();
    Program {
        title: tree.title.clone(),
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    annotations: Vec::new(),
                    elements: imports,
                },
                selected,
            ],
        },
    }
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

fn state_checksum(state: &BTreeMap<mech_core::CellSlotId, Vec<f32>>) -> f64 {
    state
        .values()
        .flatten()
        .map(|value| f64::from(*value))
        .sum()
}

fn millis(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn throughput(instances: usize, duration: std::time::Duration) -> f64 {
    instances as f64 / duration.as_secs_f64() / 1_000_000.0
}

#[cfg(feature = "metal-native")]
fn run_metal_only(instances: usize, turns: u32) {
    let compile_started = Instant::now();
    let tree = source_tree(instances);
    let driver = evaluate_driver(&tree);
    let artifact = compile_artifact(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap_or_else(|error| panic!("generic EKF source must be admitted: {error}"));
    let unchecked_program = program
        .without_integrity_constraints()
        .expect("unchecked control must preserve the fixed-shape plan");
    assert_eq!(program.instances() as usize, instances);
    assert_eq!(program.integrity_constraints().count(), 3);
    let compile_time = compile_started.elapsed();

    // Keep pipeline/device setup and warmup outside the measured resident loop.
    // The timed session is recreated after warmup so its published state starts
    // from the same initial estimate as the Mojo control.
    let prepare_started = Instant::now();
    let mut checked_warmup = program.prepare_metal(&inputs).unwrap();
    checked_warmup.dispatch_turns(5).unwrap();
    let mut checked = program.prepare_metal(&inputs).unwrap();
    let checked_started = Instant::now();
    checked.dispatch_turns(turns).unwrap();
    let checked_per_turn = checked_started.elapsed() / turns;
    let checked_state = checked.read_state().unwrap();

    let mut unchecked_warmup = unchecked_program.prepare_metal(&inputs).unwrap();
    unchecked_warmup.dispatch_turns(5).unwrap();
    let mut unchecked = unchecked_program.prepare_metal(&inputs).unwrap();
    let unchecked_started = Instant::now();
    unchecked.dispatch_turns(turns).unwrap();
    let unchecked_per_turn = unchecked_started.elapsed() / turns;
    let unchecked_state = unchecked.read_state().unwrap();
    let prepare_time = prepare_started.elapsed();

    // Validate both publication lanes against an independently executed Mech
    // scalar reference. This comparison is outside the timing boundary.
    let mut reference = program.prepare_cpu(&inputs).unwrap();
    reference.dispatch_turns(turns).unwrap();
    let checked_error = maximum_error(reference.state(), &checked_state);
    let unchecked_error = maximum_error(reference.state(), &unchecked_state);
    assert!(
        checked_error <= 5.0e-4 && unchecked_error <= 5.0e-4,
        "direct Metal result diverged from scalar reference: checked={checked_error}, unchecked={unchecked_error}"
    );

    println!("Mech direct Metal-only benchmark (no WGPU session)");
    println!("EKF instances: {instances}");
    println!("turns: {turns}");
    println!("adapter: Apple Metal (direct command encoder)");
    println!(
        "source + artifact + scalarization: {:.3} ms",
        millis(compile_time)
    );
    println!(
        "Metal pipeline/session setup: {:.3} ms",
        millis(prepare_time)
    );
    println!(
        "Mech direct Metal checked: {:.3} ms/turn",
        millis(checked_per_turn)
    );
    println!(
        "Mech direct Metal unchecked: {:.3} ms/turn",
        millis(unchecked_per_turn)
    );
    println!(
        "Mech direct Metal checked throughput: {:.3} million EKF-turns/s",
        throughput(instances, checked_per_turn)
    );
    println!(
        "Mech direct Metal unchecked throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_per_turn)
    );
    println!("maximum scalar/direct Metal checked absolute error: {checked_error:.3e}");
    println!("maximum scalar/direct Metal unchecked absolute error: {unchecked_error:.3e}");
    println!(
        "Mech direct Metal unchecked checksum: {:.9}",
        state_checksum(&unchecked_state)
    );
}
