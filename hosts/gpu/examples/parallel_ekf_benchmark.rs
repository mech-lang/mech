use std::{collections::BTreeMap, env, time::Instant};

use mech_core::{Body, MechCode, Program, Section, SectionElement};
use mech_engine::ProgramArtifact;
use mech_gpu::ComputeLowerer;
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

// This fixture deliberately matches the Taichi harness: only the three lane
// arrays are runtime inputs; dt, noise, and validation thresholds are fixed.
const SOURCE: &str = include_str!("../fixtures/ekf-kernel-taichi-comparable.mec");
const COMPUTE_INPUT_NAMES: [&str; 3] = ["linear-velocity", "angular-velocity", "bearing"];

fn main() {
    let requested_instances = argument(1, 100_000_usize).max(1);
    let cpu_turns = argument(2, 3_u32).max(1);
    let single_gpu_turns = argument(3, 20_u32).max(1);
    let checked_gpu_turns = argument(4, 120_u32).max(1);
    let validation_turns = 4;
    let parallel_workers = env::var("MECH_PARALLEL_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|workers| *workers > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        });

    let compile_started = Instant::now();
    let tree = source_tree(requested_instances);
    let driver = evaluate_driver(&tree);
    let artifact = compile_artifact(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .unwrap_or_else(|error| panic!("generic EKF source must be admitted: {error}"));
    if env::var_os("MECH_DUMP_WGSL").is_some() {
        std::fs::write("/tmp/mech-ekf.wgsl", program.wgsl())
            .expect("MECH_DUMP_WGSL should point to a writable temporary path");
    }
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
    let mut gpu_validation = program.prepare_resident(&inputs).unwrap();
    let actual = gpu_validation.run_turns(validation_turns).unwrap();
    let max_error = maximum_error(&expected, &actual.state);
    assert!(
        max_error <= 1.0e-4,
        "GPU result differs from generic CPU lowering by {max_error}"
    );
    let unchecked_program = program.without_integrity_constraints();
    assert_eq!(
        unchecked_program.integrity_constraints().count(),
        0,
        "the unchecked GPU artifact must not carry integrity predicates"
    );
    let mut gpu_unchecked_validation = unchecked_program
        .prepare_resident_unchecked(&inputs)
        .unwrap();
    let unchecked_actual = gpu_unchecked_validation
        .run_turns(validation_turns)
        .unwrap();
    let unchecked_max_error = maximum_error(&expected, &unchecked_actual.state);
    assert!(
        unchecked_max_error <= 1.0e-4,
        "unchecked GPU result differs from generic CPU lowering by {unchecked_max_error}"
    );
    let mut gpu_unchecked_fused_validation = unchecked_program
        .prepare_resident_unchecked_fused(&inputs, validation_turns)
        .unwrap();
    gpu_unchecked_fused_validation
        .dispatch_unchecked_fused()
        .unwrap();
    let (_, unchecked_fused_state) = gpu_unchecked_fused_validation.read_state().unwrap();
    let unchecked_fused_max_error = maximum_error(&expected, &unchecked_fused_state);
    assert!(
        unchecked_fused_max_error <= 1.0e-4,
        "fused unchecked GPU result differs from generic CPU lowering by {unchecked_fused_max_error}"
    );

    // The full comparison below intentionally exercises every backend. For
    // native Metal tuning, an opt-in short mode avoids measuring the setup and
    // hot loops of the other backends before the direct API control sample.
    #[cfg(feature = "native-metal")]
    if env::var_os("MECH_NATIVE_METAL_ONLY").is_some() {
        let mut reference = program.prepare_cpu(&inputs).unwrap();
        reference.dispatch_turns(single_gpu_turns).unwrap();
        let expected = reference.state().clone();
        let measure = |kernel: &mech_gpu::FixedShapeKernel| {
            let mut warmup = kernel.prepare_native_metal(&inputs).unwrap();
            warmup.dispatch_turns(5).unwrap();
            let mut session = kernel.prepare_native_metal(&inputs).unwrap();
            let started = Instant::now();
            for _ in 0..single_gpu_turns {
                session.dispatch_turns(1).unwrap();
            }
            let per_turn = started.elapsed() / single_gpu_turns;
            let state = session.read_state().unwrap();
            (
                per_turn,
                state_checksum(&state),
                maximum_error(&expected, &state),
                session.threads_per_threadgroup(),
            )
        };
        let (checked_per_turn, checked_checksum, checked_error, threadgroup) = measure(&program);
        let (unchecked_per_turn, unchecked_checksum, unchecked_error, unchecked_threadgroup) =
            measure(&unchecked_program);
        println!("native Metal tuning mode: direct API, per-turn completion wait");
        println!("native Metal threadgroup size: {threadgroup}");
        assert_eq!(threadgroup, unchecked_threadgroup);
        println!(
            "Mech native Metal checked one-turn throughput: {:.3} million EKF-turns/s",
            throughput(instances, checked_per_turn)
        );
        println!(
            "Mech native Metal unchecked one-turn throughput: {:.3} million EKF-turns/s",
            throughput(instances, unchecked_per_turn)
        );
        println!("Mech native Metal checked checksum: {checked_checksum:.9}");
        println!("Mech native Metal unchecked checksum: {unchecked_checksum:.9}");
        println!("maximum CPU/native Metal checked absolute error: {checked_error:.3e}");
        println!("maximum CPU/native Metal unchecked absolute error: {unchecked_error:.3e}");
        return;
    }

    let mut cpu_warmup = program.prepare_cpu(&inputs).unwrap();
    cpu_warmup.dispatch_turns(5).unwrap();
    let mut cpu = program.prepare_cpu(&inputs).unwrap();
    let cpu_started = Instant::now();
    cpu.dispatch_turns(cpu_turns).unwrap();
    let cpu_per_turn = cpu_started.elapsed() / cpu_turns;
    let cpu_checksum = state_checksum(cpu.state());

    let mut cpu_unchecked_warmup = unchecked_program.prepare_cpu(&inputs).unwrap();
    cpu_unchecked_warmup.dispatch_turns(5).unwrap();
    let mut cpu_unchecked = unchecked_program.prepare_cpu(&inputs).unwrap();
    let cpu_unchecked_started = Instant::now();
    cpu_unchecked.dispatch_turns(cpu_turns).unwrap();
    let cpu_unchecked_per_turn = cpu_unchecked_started.elapsed() / cpu_turns;
    let cpu_unchecked_checksum = state_checksum(cpu_unchecked.state());
    let cpu_unchecked_max_error = maximum_error(cpu.state(), cpu_unchecked.state());
    assert!(
        cpu_unchecked_max_error <= 1.0e-4,
        "unchecked scalar CPU result differs from checked scalar CPU lowering by {cpu_unchecked_max_error}"
    );

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

    let mut jit_checked_fast_warmup = program.prepare_jit_cpu_checked_fast(&inputs).unwrap();
    jit_checked_fast_warmup.dispatch_turns(5).unwrap();
    let mut jit_checked_fast = program.prepare_jit_cpu_checked_fast(&inputs).unwrap();
    let jit_checked_fast_started = Instant::now();
    jit_checked_fast.dispatch_turns(cpu_turns).unwrap();
    let jit_checked_fast_per_turn = jit_checked_fast_started.elapsed() / cpu_turns;
    let jit_checked_fast_checksum = state_checksum(jit_checked_fast.state());

    let mut jit_unchecked_warmup = program.prepare_jit_cpu_unchecked(&inputs).unwrap();
    jit_unchecked_warmup.dispatch_turns(5).unwrap();
    let mut jit_unchecked = program.prepare_jit_cpu_unchecked(&inputs).unwrap();
    let jit_unchecked_started = Instant::now();
    jit_unchecked.dispatch_turns(cpu_turns).unwrap();
    let jit_unchecked_per_turn = jit_unchecked_started.elapsed() / cpu_turns;
    let jit_unchecked_checksum = state_checksum(jit_unchecked.state());
    let mut jit_unchecked_fast_warmup = program.prepare_jit_cpu_unchecked_fast(&inputs).unwrap();
    jit_unchecked_fast_warmup.dispatch_turns(5).unwrap();
    let mut jit_unchecked_fast = program.prepare_jit_cpu_unchecked_fast(&inputs).unwrap();
    let jit_unchecked_fast_started = Instant::now();
    jit_unchecked_fast.dispatch_turns(cpu_turns).unwrap();
    let jit_unchecked_fast_per_turn = jit_unchecked_fast_started.elapsed() / cpu_turns;
    let jit_unchecked_fast_checksum = state_checksum(jit_unchecked_fast.state());

    let mut jit_simd_validation = program.prepare_jit_simd_cpu(&inputs).unwrap();
    jit_simd_validation
        .dispatch_turns(validation_turns)
        .unwrap();
    let jit_simd_max_error = maximum_error(&expected, jit_simd_validation.state());
    assert!(
        jit_simd_max_error <= 1.0e-4,
        "SIMD JIT result differs from scalar CPU lowering by {jit_simd_max_error}"
    );

    let mut jit_simd_parallel_validation = program.prepare_jit_simd_cpu(&inputs).unwrap();
    jit_simd_parallel_validation
        .dispatch_turns_parallel(validation_turns, parallel_workers)
        .unwrap();
    let jit_simd_parallel_max_error =
        maximum_error(&expected, jit_simd_parallel_validation.state());
    assert!(
        jit_simd_parallel_max_error <= 1.0e-4,
        "parallel SIMD JIT result differs from scalar CPU lowering by {jit_simd_parallel_max_error}"
    );

    let mut jit_simd_warmup = program.prepare_jit_simd_cpu(&inputs).unwrap();
    jit_simd_warmup.dispatch_turns(5).unwrap();
    let mut jit_simd = program.prepare_jit_simd_cpu(&inputs).unwrap();
    let jit_simd_started = Instant::now();
    jit_simd.dispatch_turns(cpu_turns).unwrap();
    let jit_simd_per_turn = jit_simd_started.elapsed() / cpu_turns;
    let jit_simd_checksum = state_checksum(jit_simd.state());

    let mut jit_simd_checked_fast_warmup =
        program.prepare_jit_simd_cpu_checked_fast(&inputs).unwrap();
    jit_simd_checked_fast_warmup.dispatch_turns(5).unwrap();
    let mut jit_simd_checked_fast = program.prepare_jit_simd_cpu_checked_fast(&inputs).unwrap();
    let jit_simd_checked_fast_started = Instant::now();
    jit_simd_checked_fast.dispatch_turns(cpu_turns).unwrap();
    let jit_simd_checked_fast_per_turn = jit_simd_checked_fast_started.elapsed() / cpu_turns;
    let jit_simd_checked_fast_checksum = state_checksum(jit_simd_checked_fast.state());

    let mut jit_simd_unchecked_warmup = program.prepare_jit_simd_cpu_unchecked(&inputs).unwrap();
    jit_simd_unchecked_warmup.dispatch_turns(5).unwrap();
    let mut jit_simd_unchecked = program.prepare_jit_simd_cpu_unchecked(&inputs).unwrap();
    let jit_simd_unchecked_started = Instant::now();
    jit_simd_unchecked.dispatch_turns(cpu_turns).unwrap();
    let jit_simd_unchecked_per_turn = jit_simd_unchecked_started.elapsed() / cpu_turns;
    let jit_simd_unchecked_checksum = state_checksum(jit_simd_unchecked.state());

    let mut jit_simd_unchecked_fast_warmup = program
        .prepare_jit_simd_cpu_unchecked_fast(&inputs)
        .unwrap();
    jit_simd_unchecked_fast_warmup.dispatch_turns(5).unwrap();
    let mut jit_simd_unchecked_fast = program
        .prepare_jit_simd_cpu_unchecked_fast(&inputs)
        .unwrap();
    let jit_simd_unchecked_fast_started = Instant::now();
    jit_simd_unchecked_fast.dispatch_turns(cpu_turns).unwrap();
    let jit_simd_unchecked_fast_per_turn = jit_simd_unchecked_fast_started.elapsed() / cpu_turns;
    let jit_simd_unchecked_fast_checksum = state_checksum(jit_simd_unchecked_fast.state());

    let mut jit_simd_parallel_warmup = program.prepare_jit_simd_cpu(&inputs).unwrap();
    jit_simd_parallel_warmup
        .dispatch_turns_parallel(5, parallel_workers)
        .unwrap();
    let mut jit_simd_parallel = program.prepare_jit_simd_cpu(&inputs).unwrap();
    let jit_simd_parallel_started = Instant::now();
    jit_simd_parallel
        .dispatch_turns_parallel(cpu_turns, parallel_workers)
        .unwrap();
    let jit_simd_parallel_per_turn = jit_simd_parallel_started.elapsed() / cpu_turns;
    let jit_simd_parallel_checksum = state_checksum(jit_simd_parallel.state());

    let mut jit_simd_parallel_unchecked_fast_warmup = program
        .prepare_jit_simd_cpu_unchecked_fast(&inputs)
        .unwrap();
    jit_simd_parallel_unchecked_fast_warmup
        .dispatch_turns_parallel(5, parallel_workers)
        .unwrap();
    let mut jit_simd_parallel_unchecked_fast = program
        .prepare_jit_simd_cpu_unchecked_fast(&inputs)
        .unwrap();
    let jit_simd_parallel_unchecked_fast_started = Instant::now();
    jit_simd_parallel_unchecked_fast
        .dispatch_turns_parallel(cpu_turns, parallel_workers)
        .unwrap();
    let jit_simd_parallel_unchecked_fast_per_turn =
        jit_simd_parallel_unchecked_fast_started.elapsed() / cpu_turns;
    let jit_simd_parallel_unchecked_fast_checksum =
        state_checksum(jit_simd_parallel_unchecked_fast.state());

    let mut jit_simd_parallel_unchecked_fast_block_warmup = program
        .prepare_jit_simd_cpu_unchecked_fast(&inputs)
        .unwrap();
    jit_simd_parallel_unchecked_fast_block_warmup
        .dispatch_turns_parallel_unchecked_fast(5, parallel_workers)
        .unwrap();
    let mut jit_simd_parallel_unchecked_fast_block = program
        .prepare_jit_simd_cpu_unchecked_fast(&inputs)
        .unwrap();
    let jit_simd_parallel_unchecked_fast_block_started = Instant::now();
    jit_simd_parallel_unchecked_fast_block
        .dispatch_turns_parallel_unchecked_fast(cpu_turns, parallel_workers)
        .unwrap();
    let jit_simd_parallel_unchecked_fast_block_per_turn =
        jit_simd_parallel_unchecked_fast_block_started.elapsed() / cpu_turns;
    let jit_simd_parallel_unchecked_fast_block_checksum =
        state_checksum(jit_simd_parallel_unchecked_fast_block.state());
    let jit_simd_parallel_unchecked_fast_block_max_error = maximum_error(
        jit_simd_unchecked_fast.state(),
        jit_simd_parallel_unchecked_fast_block.state(),
    );
    assert!(
        jit_simd_parallel_unchecked_fast_block_max_error <= 1.0e-4,
        "batched parallel unchecked SIMD-JIT result differs from single-thread unchecked result by {jit_simd_parallel_unchecked_fast_block_max_error}"
    );

    let mut jit_simd_parallel_checked_fused_warmup = program.prepare_jit_simd_cpu(&inputs).unwrap();
    jit_simd_parallel_checked_fused_warmup
        .dispatch_turns_parallel_checked_fused(5, parallel_workers)
        .unwrap();
    let mut jit_simd_parallel_checked_fused = program.prepare_jit_simd_cpu(&inputs).unwrap();
    let jit_simd_parallel_checked_fused_started = Instant::now();
    jit_simd_parallel_checked_fused
        .dispatch_turns_parallel_checked_fused(cpu_turns, parallel_workers)
        .unwrap();
    let jit_simd_parallel_checked_fused_per_turn =
        jit_simd_parallel_checked_fused_started.elapsed() / cpu_turns;
    let jit_simd_parallel_checked_fused_checksum =
        state_checksum(jit_simd_parallel_checked_fused.state());
    let jit_simd_parallel_checked_fused_max_error =
        maximum_error(cpu.state(), jit_simd_parallel_checked_fused.state());
    assert!(
        jit_simd_parallel_checked_fused_max_error <= 1.0e-4,
        "checked fused parallel SIMD-JIT result differs from scalar CPU lowering by {jit_simd_parallel_checked_fused_max_error}"
    );

    let jit_validation_overhead =
        (jit_per_turn.as_secs_f64() / jit_unchecked_per_turn.as_secs_f64() - 1.0) * 100.0;
    let jit_checked_fast_validation_overhead =
        (jit_checked_fast_per_turn.as_secs_f64() / jit_unchecked_fast_per_turn.as_secs_f64() - 1.0)
            * 100.0;
    let jit_simd_validation_overhead =
        (jit_simd_per_turn.as_secs_f64() / jit_simd_unchecked_per_turn.as_secs_f64() - 1.0) * 100.0;
    let jit_simd_checked_fast_validation_overhead = (jit_simd_checked_fast_per_turn.as_secs_f64()
        / jit_simd_unchecked_fast_per_turn.as_secs_f64()
        - 1.0)
        * 100.0;

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

    // Checksums use a fresh session so warm-up and throughput samples cannot
    // silently change the number of turns being compared with other languages.
    let mut gpu_checksum_session = program.prepare_resident(&inputs).unwrap();
    gpu_checksum_session
        .dispatch_turns(checked_gpu_turns)
        .unwrap();
    let (_, gpu_checksum_state) = gpu_checksum_session.read_state().unwrap();
    let gpu_checksum = state_checksum(&gpu_checksum_state);

    let mut gpu_unchecked_single = unchecked_program
        .prepare_resident_unchecked(&inputs)
        .unwrap();
    gpu_unchecked_single.dispatch_turns(1).unwrap();
    let unchecked_single_started = Instant::now();
    for _ in 0..single_gpu_turns {
        gpu_unchecked_single.dispatch_turns(1).unwrap();
    }
    let unchecked_single_per_turn = unchecked_single_started.elapsed() / single_gpu_turns;

    let mut gpu_unchecked_in_place_single = unchecked_program
        .prepare_resident_unchecked_in_place(&inputs)
        .unwrap();
    gpu_unchecked_in_place_single.dispatch_turns(1).unwrap();
    let unchecked_in_place_single_started = Instant::now();
    for _ in 0..single_gpu_turns {
        gpu_unchecked_in_place_single.dispatch_turns(1).unwrap();
    }
    let unchecked_in_place_single_per_turn =
        unchecked_in_place_single_started.elapsed() / single_gpu_turns;

    let mut gpu_unchecked_batch = unchecked_program
        .prepare_resident_unchecked(&inputs)
        .unwrap();
    gpu_unchecked_batch.dispatch_turns(5).unwrap();
    let unchecked_batch_per_turn = gpu_unchecked_batch
        .dispatch_turns(checked_gpu_turns)
        .unwrap()
        / checked_gpu_turns;

    let mut gpu_unchecked_in_place_batch = unchecked_program
        .prepare_resident_unchecked_in_place(&inputs)
        .unwrap();
    gpu_unchecked_in_place_batch.dispatch_turns(5).unwrap();
    let unchecked_in_place_batch_per_turn = gpu_unchecked_in_place_batch
        .dispatch_turns(checked_gpu_turns)
        .unwrap()
        / checked_gpu_turns;

    let mut gpu_unchecked_warmup = unchecked_program
        .prepare_resident_unchecked_fused(&inputs, checked_gpu_turns)
        .unwrap();
    gpu_unchecked_warmup.dispatch_unchecked_fused().unwrap();
    let unchecked_prepare_started = Instant::now();
    let mut gpu_unchecked = unchecked_program
        .prepare_resident_unchecked_fused(&inputs, checked_gpu_turns)
        .unwrap();
    let unchecked_prepare = unchecked_prepare_started.elapsed();
    gpu_unchecked.dispatch_unchecked_fused().unwrap();
    let unchecked_repeated = gpu_unchecked.dispatch_unchecked_fused().unwrap() / checked_gpu_turns;
    let (_, unchecked_state) = gpu_unchecked.read_state().unwrap();
    std::hint::black_box(unchecked_state);

    let mut gpu_unchecked_checksum_session = unchecked_program
        .prepare_resident_unchecked_fused(&inputs, checked_gpu_turns)
        .unwrap();
    gpu_unchecked_checksum_session
        .dispatch_unchecked_fused()
        .unwrap();
    let (_, gpu_unchecked_checksum_state) = gpu_unchecked_checksum_session.read_state().unwrap();
    let gpu_unchecked_checksum = state_checksum(&gpu_unchecked_checksum_state);

    #[cfg(feature = "native-metal")]
    let native_expected = {
        let mut reference = program.prepare_cpu(&inputs).unwrap();
        reference.dispatch_turns(single_gpu_turns).unwrap();
        reference.state().clone()
    };

    #[cfg(feature = "native-metal")]
    let (metal_checked_per_turn, metal_checked_checksum, metal_checked_error) = {
        let mut warmup = program.prepare_native_metal(&inputs).unwrap();
        warmup.dispatch_turns(5).unwrap();
        let mut session = program.prepare_native_metal(&inputs).unwrap();
        let started = Instant::now();
        for _ in 0..single_gpu_turns {
            session.dispatch_turns(1).unwrap();
        }
        let per_turn = started.elapsed() / single_gpu_turns;
        let state = session.read_state().unwrap();
        let checksum = state_checksum(&state);
        let error = maximum_error(&native_expected, &state);
        (per_turn, checksum, error)
    };

    #[cfg(feature = "native-metal")]
    let (metal_unchecked_per_turn, metal_unchecked_checksum, metal_unchecked_error) = {
        let mut warmup = unchecked_program.prepare_native_metal(&inputs).unwrap();
        warmup.dispatch_turns(5).unwrap();
        let mut session = unchecked_program.prepare_native_metal(&inputs).unwrap();
        let started = Instant::now();
        for _ in 0..single_gpu_turns {
            session.dispatch_turns(1).unwrap();
        }
        let per_turn = started.elapsed() / single_gpu_turns;
        let state = session.read_state().unwrap();
        let checksum = state_checksum(&state);
        let error = maximum_error(&native_expected, &state);
        (per_turn, checksum, error)
    };

    println!("EKF instances: {instances}");
    println!("batch extent authority: Mech input arrays");
    println!("source artifact nodes: {}", artifact.nodes().len());
    println!("generated WGSL bytes: {}", program.wgsl().len());
    println!(
        "generated unchecked fused WGSL bytes: {}",
        unchecked_program.wgsl().len()
    );
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
        "Mech scalar unchecked CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(cpu_unchecked_per_turn)
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
        "Mech Cranelift JIT checked fast CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_checked_fast_per_turn)
    );
    println!(
        "Mech Cranelift JIT unchecked CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_unchecked_per_turn)
    );
    println!(
        "Mech Cranelift JIT unchecked fast CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_unchecked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT checked fast CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_checked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT unchecked CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_unchecked_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT unchecked fast CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_unchecked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel CPU ({parallel_workers} workers): {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_parallel_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel unchecked fast CPU ({parallel_workers} workers): {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_parallel_unchecked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel unchecked fast block CPU ({parallel_workers} workers): {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_parallel_unchecked_fast_block_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel checked fused block CPU ({parallel_workers} workers): {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_simd_parallel_checked_fused_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel checked fused block throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_parallel_checked_fused_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel checked fused block checksum: {jit_simd_parallel_checked_fused_checksum:.9}"
    );
    println!(
        "resident GPU, checked one-turn API call: {:.3} ms/turn ({single_gpu_turns} turns)",
        millis(single_per_turn)
    );
    println!(
        "resident GPU, checked repeated API call (per-turn validation): {:.3} ms/turn ({checked_gpu_turns} turns)",
        millis(checked_repeated)
    );
    println!(
        "resident GPU, unchecked one-turn API call: {:.3} ms/turn ({single_gpu_turns} turns)",
        millis(unchecked_single_per_turn)
    );
    println!(
        "resident GPU, unchecked repeated dispatches: {:.3} ms/turn ({checked_gpu_turns} turns)",
        millis(unchecked_batch_per_turn)
    );
    println!(
        "resident GPU, unchecked one submission: {:.3} ms/turn ({checked_gpu_turns} turns)",
        millis(unchecked_repeated)
    );
    println!(
        "Mech scalar throughput: {:.3} million EKF-turns/s",
        throughput(instances, cpu_per_turn)
    );
    println!(
        "Mech scalar unchecked throughput: {:.3} million EKF-turns/s",
        throughput(instances, cpu_unchecked_per_turn)
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
        "Mech Cranelift JIT checked fast throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_checked_fast_per_turn)
    );
    println!(
        "Mech Cranelift JIT unchecked throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_unchecked_per_turn)
    );
    println!(
        "Mech Cranelift JIT unchecked fast throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_unchecked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT checked fast throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_checked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT unchecked throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_unchecked_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT unchecked fast throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_unchecked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_parallel_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel unchecked fast throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_parallel_unchecked_fast_per_turn)
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel unchecked fast block throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_simd_parallel_unchecked_fast_block_per_turn)
    );
    println!("JIT integrity validation time overhead: {jit_validation_overhead:.2}%");
    println!(
        "JIT checked-fast validation time overhead: {jit_checked_fast_validation_overhead:.2}%"
    );
    println!("SIMD-JIT integrity validation time overhead: {jit_simd_validation_overhead:.2}%");
    println!(
        "SIMD-JIT checked-fast validation time overhead: {jit_simd_checked_fast_validation_overhead:.2}%"
    );
    println!(
        "GPU checked one-turn throughput: {:.3} million EKF-turns/s",
        throughput(instances, single_per_turn)
    );
    println!(
        "GPU checked repeated throughput (per-turn validation): {:.3} million EKF-turns/s",
        throughput(instances, checked_repeated)
    );
    println!(
        "GPU unchecked ping-pong one-turn throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_single_per_turn)
    );
    println!(
        "GPU unchecked one-turn throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_single_per_turn)
    );
    println!(
        "GPU unchecked repeated throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_batch_per_turn)
    );
    println!(
        "GPU unchecked in-place one-turn throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_in_place_single_per_turn)
    );
    println!(
        "GPU unchecked in-place repeated throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_in_place_batch_per_turn)
    );
    println!(
        "GPU unchecked one-submit throughput: {:.3} million EKF-turns/s",
        throughput(instances, unchecked_repeated)
    );
    #[cfg(feature = "native-metal")]
    {
        println!(
            "Mech native Metal checked one-turn throughput: {:.3} million EKF-turns/s",
            throughput(instances, metal_checked_per_turn)
        );
        println!(
            "Mech native Metal unchecked one-turn throughput: {:.3} million EKF-turns/s",
            throughput(instances, metal_unchecked_per_turn)
        );
    }
    println!(
        "resident GPU unchecked prepare: {:.3} ms",
        millis(unchecked_prepare)
    );
    println!("final state readback: {:.3} ms", millis(readback));
    println!("maximum CPU/GPU absolute error: {max_error:.3e}");
    println!("maximum CPU/GPU unchecked absolute error: {unchecked_max_error:.3e}");
    println!("maximum CPU/GPU fused unchecked absolute error: {unchecked_fused_max_error:.3e}");
    println!("maximum scalar/SIMD absolute error: {simd_max_error:.3e}");
    println!("maximum scalar/JIT absolute error: {jit_max_error:.3e}");
    println!("maximum scalar/SIMD-JIT absolute error: {jit_simd_max_error:.3e}");
    println!("maximum scalar/parallel SIMD-JIT absolute error: {jit_simd_parallel_max_error:.3e}");
    println!("Mech scalar checksum: {cpu_checksum:.9}");
    println!("Mech scalar unchecked checksum: {cpu_unchecked_checksum:.9}");
    println!("Mech SIMD checksum: {simd_checksum:.9}");
    println!("Mech Cranelift JIT checksum: {jit_checksum:.9}");
    println!("Mech Cranelift JIT checked fast checksum: {jit_checked_fast_checksum:.9}");
    println!("Mech Cranelift JIT unchecked checksum: {jit_unchecked_checksum:.9}");
    println!("Mech Cranelift JIT unchecked fast checksum: {jit_unchecked_fast_checksum:.9}");
    println!("Mech Cranelift SIMD-JIT checksum: {jit_simd_checksum:.9}");
    println!("Mech Cranelift SIMD-JIT checked fast checksum: {jit_simd_checked_fast_checksum:.9}");
    println!("Mech Cranelift SIMD-JIT unchecked checksum: {jit_simd_unchecked_checksum:.9}");
    println!(
        "Mech Cranelift SIMD-JIT unchecked fast checksum: {jit_simd_unchecked_fast_checksum:.9}"
    );
    println!("Mech Cranelift SIMD-JIT parallel checksum: {jit_simd_parallel_checksum:.9}");
    println!(
        "Mech Cranelift SIMD-JIT parallel unchecked fast checksum: {jit_simd_parallel_unchecked_fast_checksum:.9}"
    );
    println!(
        "Mech Cranelift SIMD-JIT parallel unchecked fast block checksum: {jit_simd_parallel_unchecked_fast_block_checksum:.9}"
    );
    println!("Mech GPU checked checksum: {gpu_checksum:.9}");
    println!("Mech GPU unchecked checksum: {gpu_unchecked_checksum:.9}");
    #[cfg(feature = "native-metal")]
    {
        println!("Mech native Metal checked checksum: {metal_checked_checksum:.9}");
        println!("Mech native Metal unchecked checksum: {metal_unchecked_checksum:.9}");
        println!("maximum CPU/native Metal checked absolute error: {metal_checked_error:.3e}");
        println!("maximum CPU/native Metal unchecked absolute error: {metal_unchecked_error:.3e}");
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
    const INPUTS: [&str; 3] = [
        "lane-linear-velocity",
        "lane-angular-velocity",
        "lane-bearing",
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
                "linear-velocity" => "lane-linear-velocity",
                "angular-velocity" => "lane-angular-velocity",
                "bearing" => "lane-bearing",
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
