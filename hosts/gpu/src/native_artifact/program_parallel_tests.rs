//! Real generated source bodies, checked images, ordering and child authority.
use super::*;
use crate::native_artifact::program_abi::{PARALLEL_RUN, PHASE_WORKER, PHASE_TURN};
use std::cell::Cell;

thread_local! {
    static BATCHES: Cell<usize> = const { Cell::new(0) };
    static FAIL_PUSH: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "C" fn observe(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    // SAFETY: the checked parent entry owns this exact live context. Workers
    // retain their ordinary helper; the observer changes only parent pushes.
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    let old = ctx.limits;
    if op == COLLECTION && FAIL_PUSH.with(Cell::get)
        && ctx.control.descriptor.parallel.regions.iter().any(|region| u64::from(region.push) == site) {
        ctx.limits.max_output_bytes = Some(0);
    }
    let status = unsafe { helper(raw, site, op, first, second, third, result) };
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    ctx.limits = old;
    if op == PARALLEL_RUN && status == STATUS_OK && unsafe { *result } == 1 {
        BATCHES.with(|count| count.set(count.get() + 1));
    }
    status
}

fn source(tail: &str, expression: &str) -> String {
    format!(r#"{}
+> number
+> text
+> sequence
count-text(s<string>, width<u64>, n<u64>) => (output<string>) :=
  h := #Count(s, width, n, empty)
  output := {}.
text := seed<string>
width := sequence/length(text) - 7u64
{}
"#, structural_projection_tests::COUNT, tail, expression)
}

fn input(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Value {
    let value = ValueDraft { schema: artifact.inputs()[0].schema, shape_values: Box::new([]),
        data: ValueDataDraft::String("ab:ab:ab".into()) }
        .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
    let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap()).unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn observed(descriptor: &NativeProgramDescriptor, entry: super::super::program_abi::Entry,
    args: &[Value], outputs: &mut [Option<Value>], memory: &ManagedMemoryBudget,
    workers: usize, fail_push: bool) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError>
{
    struct Restore(Option<usize>, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            parallel::TEST_WORKERS.with(|value| value.set(self.0));
            FAIL_PUSH.with(|value| value.set(self.1));
        }
    }
    let _restore = Restore(parallel::TEST_WORKERS.with(|value| value.replace(Some(workers))),
        FAIL_PUSH.with(|value| value.replace(fail_push)));
    BATCHES.with(|value| value.set(0));
    execute(descriptor, |frame| {
        let previous = unsafe { (*frame).helper };
        unsafe { (*frame).helper = observe; }
        let status = unsafe { entry(frame) };
        unsafe { (*frame).helper = previous; }
        status
    }, args, outputs, tests::limits(), memory)
}

#[test]
fn parallel_source_bodies_keep_all_seven_maps_updates_captures_and_order() {
    let source = source("number/format(h{\"a\"}, 0u64)", r#"
first := text/join([count-text(text, width, n) | n <- [4u64 8u64]], "|")
second := text/join([count-text(text, width, n) | n <- [1u64 2u64 3u64 4u64 5u64]], ",")
first + ";" + second
"#);
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(jit.descriptor().parallel.regions.len(), 2);
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: unchanged image and checked entry share this retained build.
        let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = vec![input(&artifact, &memory)];
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let mut serial = vec![None; artifact.outputs().len()];
            let mut parallel = vec![None; artifact.outputs().len()];
            let serial_report = observed(descriptor, jit.test_checked_entry(), &args, &mut serial, &memory, 1, false).unwrap();
            assert_eq!(BATCHES.with(Cell::get), 0);
            let report = observed(descriptor, jit.test_checked_entry(), &args, &mut parallel, &memory, 8, false).unwrap();
            assert_eq!(BATCHES.with(Cell::get), 2);
            assert_eq!((report.hash_table_regions, report.hash_table_updates), (7, 27));
            assert_eq!((report.hash_table_regions, report.hash_table_updates),
                (serial_report.hash_table_regions, serial_report.hash_table_updates));
            assert_eq!(report.packed_ssa_regions, serial_report.packed_ssa_regions);
            for (left, right) in serial.iter().zip(&parallel) {
                assert_eq!(left.as_ref().unwrap().canonical_snapshot_bytes(artifact.schemas()).unwrap(),
                    right.as_ref().unwrap().canonical_snapshot_bytes(artifact.schemas()).unwrap());
            }
            assert!(matches!(parallel.last().unwrap().as_ref().unwrap().data(),
                ValueData::String(text) if text.as_ref() == "2|3;1,1,1,2,2"));
            drop((serial, parallel));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn prior_push_failure_precedes_later_worker_arithmetic_and_retires_pending_results() {
    let source = source("number/format(h{\"a\"} + 8u64 / (8u64 - n), 0u64)",
        "text/join([count-text(text, width, n) | n <- [4u64 8u64]], \"|\")");
    let artifact = tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    for fail_push in [false, true] {
        let mut errors = Vec::new();
        for workers in [1, 8] {
            let mut output = vec![None; artifact.outputs().len()];
            let error = observed(jit.descriptor(), jit.test_checked_entry(), &args, &mut output,
                &memory, workers, fail_push).unwrap_err();
            if workers == 8 { assert_eq!(BATCHES.with(Cell::get), 1); }
            assert!(output.iter().all(Option::is_none));
            errors.push(format!("{error:?}"));
            drop(output);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        assert_eq!(errors[0], errors[1], "source error order differs");
        assert!(errors[0].contains(if fail_push { "OutputBytes" } else { "Arithmetic" }));
    }
}

#[test]
fn worker_phase_requires_owned_child_state_before_entering_source_body() {
    let source = source("number/format(h{\"a\"}, 0u64)",
        "text/join([count-text(text, width, n) | n <- [4u64 8u64]], \"|\")");
    let artifact = tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut output = vec![None; artifact.outputs().len()];
    let entry = jit.test_checked_entry();
    let begin = jit.descriptor().parallel.regions[0].begin;
    let error = execute(jit.descriptor(), |frame| {
        // SAFETY: mutate only this test's retained frame and restore its fields.
        let frame_ref = unsafe { &mut *frame };
        if frame_ref.phase != PHASE_TURN { return unsafe { entry(frame) }; }
        let original = (frame_ref.phase, frame_ref.resume_site, frame_ref.available_lines, frame_ref.input_eof);
        (frame_ref.phase, frame_ref.resume_site, frame_ref.available_lines, frame_ref.input_eof) =
            (PHASE_WORKER, u64::from(begin), 0, 2);
        let status = unsafe { entry(frame) };
        let frame_ref = unsafe { &mut *frame };
        (frame_ref.phase, frame_ref.resume_site, frame_ref.available_lines, frame_ref.input_eof) = original;
        status
    }, &args, &mut output, tests::limits(), &memory).unwrap_err();
    assert!(format!("{error:?}").contains("worker entry without owned child state"));
    assert!(output.iter().all(Option::is_none));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
}
