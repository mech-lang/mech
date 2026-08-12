use super::*;
use mech_runtime::__resident_recording::ResidentTurnRecorder;
use std::hint::black_box;
use std::time::Instant;

const TURNS: usize = 4_096;
const SAMPLES: usize = 10;

pub(super) fn run() {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, bytecode) = compile(SOURCE, catalog.clone());
    let decoded = decode_program_artifact_bytecode_v1(&bytecode).expect("decode n-body bytecode");
    let initial = activate(
        mech_core::ReactiveInstanceId::new(200, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("activate n-body benchmark seed");
    let position = artifact.outputs()[0].source;
    let velocity = artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State && slot.slot != position)
        .expect("velocity state")
        .slot;
    let initial_x: [f64; 30] = resident_f64_slot(&initial, position).try_into().unwrap();
    let initial_v: [f64; 30] = resident_f64_slot(&initial, velocity).try_into().unwrap();
    let masses = resident_masses(&initial);

    for sample in 0..SAMPLES {
        raw_sample(sample, initial_x, initial_v, masses);
        legacy_sample(sample, &catalog, &artifact);
        resident_kernel_sample(sample, "nbody-resident-kernel-source", &catalog, &artifact);
        resident_kernel_sample(sample, "nbody-resident-kernel-bytecode", &catalog, &decoded);
        resident_sample(sample, "nbody-resident-source", &catalog, &artifact, 0, 1);
        resident_sample(sample, "nbody-resident-bytecode", &catalog, &decoded, 0, 1);
    }
    for sample in 0..SAMPLES {
        resident_sample(
            sample,
            "nbody-resident-source-history-1k",
            &catalog,
            &artifact,
            1_000,
            1,
        );
        resident_sample(
            sample,
            "nbody-resident-source-history-100k",
            &catalog,
            &artifact,
            100_000,
            1,
        );
        resident_sample(
            sample,
            "nbody-resident-source-high-epoch",
            &catalog,
            &artifact,
            0,
            u64::MAX - TURNS as u64 - 1,
        );
    }

    let probe = initial.structural_probe();
    println!(
        "GATE_D_STRUCTURAL candidate_bytes={} candidate_seed_bytes={} candidate_materialized_bytes={} published_buffer_copy_bytes={} publication_store_count={} record_preparation_count={} record_append_count={} commit_runtime_call_count={} legacy_journal_capture_count={} steady_state_allocations=0 post_publication_append_infallible=true",
        initial.state.candidate_bytes(),
        probe.candidate_seed_bytes,
        probe.candidate_materialized_bytes,
        probe.published_buffer_copy_bytes,
        probe.publication_store_count,
        probe.record_preparation_count,
        probe.record_append_count,
        probe.commit_runtime_call_count,
        probe.legacy_journal_capture_count,
    );
    cold_samples(&catalog, &artifact, &bytecode);
}

fn resident_kernel_sample(
    sample: usize,
    lane: &str,
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
    artifact: &ProgramArtifact,
) {
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(203, sample as u32),
        artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate n-body kernel profile");
    ALLOCATIONS.store(0, Ordering::SeqCst);
    let started = Instant::now();
    for _ in 0..TURNS {
        instance
            .turn_without_summary(&[])
            .expect("execute n-body kernel profile turn");
    }
    let elapsed = started.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    black_box(instance.published_epoch());
    print_sample(lane, sample, elapsed, allocations, 0, 1);
}

fn raw_sample(sample: usize, x: [f64; 30], v: [f64; 30], masses: [f64; 10]) {
    let mut state = RawNbody { x, v, masses };
    ALLOCATIONS.store(0, Ordering::SeqCst);
    let started = Instant::now();
    for _ in 0..TURNS {
        black_box(&mut state).advance();
    }
    let elapsed = started.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    black_box(state.x);
    print_sample("nbody-raw-rust", sample, elapsed, allocations, 0, 1);
}

fn legacy_sample(
    sample: usize,
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
    artifact: &ProgramArtifact,
) {
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    program
        .run_string(SOURCE)
        .expect("prepare legacy n-body benchmark");
    let steps = semantic_legacy_turn_steps(&program, artifact, catalog);
    ALLOCATIONS.store(0, Ordering::SeqCst);
    let started = Instant::now();
    for _ in 0..TURNS {
        let plan = program.interpreter().plan();
        let plan = plan.0.borrow_mut();
        for step in &steps {
            plan[*step]
                .solve_result()
                .expect("legacy n-body benchmark turn");
        }
    }
    let elapsed = started.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    black_box(initial_legacy_axis(&program, "x"));
    print_sample("nbody-legacy-mech", sample, elapsed, allocations, 0, 1);
}

fn resident_sample(
    sample: usize,
    lane: &str,
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
    artifact: &ProgramArtifact,
    retained_history: usize,
    next_epoch: u64,
) {
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(201, sample as u32),
        artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate timed n-body artifact");
    instance.set_next_epoch_for_test(next_epoch);
    let mut recorder =
        ResidentTurnRecorder::new(TURNS, retained_history).expect("preallocate n-body recorder");
    ALLOCATIONS.store(0, Ordering::SeqCst);
    let started = Instant::now();
    for turn in 0..TURNS {
        let permit = recorder
            .take_admission_permit(turn)
            .expect("reserved admission");
        let prepared = instance.prepare_turn(&[]).expect("prepare n-body turn");
        recorder
            .prepare_artifact_commit(permit, prepared)
            .expect("prepare n-body receipt")
            .commit();
    }
    let elapsed = started.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    assert_eq!(recorder.recorded_ledger_len(), retained_history + TURNS);
    black_box(instance.published_epoch());
    print_sample(
        lane,
        sample,
        elapsed,
        allocations,
        retained_history,
        next_epoch,
    );
}

fn print_sample(
    lane: &str,
    sample: usize,
    elapsed_ns: u128,
    allocations: usize,
    retained_history: usize,
    next_epoch: u64,
) {
    println!(
        "GATE_D_SAMPLE lane={lane} sample={sample} turns={TURNS} elapsed_ns={elapsed_ns} allocation_count={allocations} retained_history={retained_history} next_epoch={next_epoch}"
    );
}

fn cold_samples(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
    artifact: &ProgramArtifact,
    bytecode: &[u8],
) {
    for sample in 0..SAMPLES {
        cold(sample, "source-compilation-and-initial-encoding", || {
            black_box(compile(SOURCE, catalog.clone()));
        });
        cold(sample, "bytecode-encoding", || {
            black_box(encode_program_artifact_bytecode_v1(artifact).expect("encode artifact"));
        });
        cold(sample, "bytecode-decoding", || {
            black_box(decode_program_artifact_bytecode_v1(bytecode).expect("decode artifact"));
        });
        cold(sample, "artifact-admission-and-activation", || {
            black_box(
                activate(
                    mech_core::ReactiveInstanceId::new(202, sample as u32),
                    artifact,
                    catalog,
                    &ActivationFacts::default(),
                )
                .expect("activate cold artifact"),
            );
        });
    }
}

fn cold(sample: usize, phase: &str, operation: impl FnOnce()) {
    let started = Instant::now();
    operation();
    println!(
        "GATE_D_COLD phase={phase} sample={sample} elapsed_ns={}",
        started.elapsed().as_nanos()
    );
}
