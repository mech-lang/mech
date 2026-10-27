//! Real generated source bodies, checked images, ordering and child authority.
use crate::native_artifact::program::protocol::abi::{PARALLEL_RUN, PHASE_TURN, PHASE_WORKER};
use crate::native_artifact::program::runtime::*;
use std::cell::Cell;

mod credit_tests;
mod schedule_tests;

thread_local! {
    static BATCHES: Cell<usize> = const { Cell::new(0) };
    static FAIL_PUSH: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "C" fn observe(
    raw: *mut c_void,
    site: u64,
    op: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    // SAFETY: the checked parent entry owns this exact live context. Workers
    // retain their ordinary helper; the observer changes only parent pushes.
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    let old = ctx.limits;
    if op == COLLECTION
        && FAIL_PUSH.with(Cell::get)
        && ctx
            .descriptor
            .metadata()
            .parallel
            .regions
            .iter()
            .any(|region| u64::from(region.push) == site)
    {
        ctx.limits.max_output_bytes = Some(0);
    }
    let status = unsafe { helper(raw, site, op, first, second, third, result) };
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    ctx.limits = old;
    if op == PARALLEL_RUN && status == STATUS_OK && unsafe { *result } == 1 {
        BATCHES.with(|count| count.set(count.get() + 1));
    }
    status
}

fn source(tail: &str, expression: &str) -> String {
    format!(
        r#"{}
+> number
+> text
+> sequence
count-text(s<string>, width<u64>, n<u64>) => (output<string>) :=
  h := #Count(s, width, n, empty)
  output := {}.
text := seed<string>
width := sequence/length(text) - 7u64
{}
"#,
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT,
        tail,
        expression
    )
}

fn input(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Value {
    let value = ValueDraft {
        schema: artifact.inputs()[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String("ab:ab:ab".into()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn observed(
    descriptor: &NativeProgramDescriptor,
    entry: crate::native_artifact::program::protocol::abi::Entry,
    args: &[Value],
    outputs: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    workers: usize,
    fail_push: bool,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    observed_with_nodes(
        descriptor,
        entry,
        args,
        outputs,
        memory,
        workers,
        fail_push,
        crate::native_artifact::program::runtime::tests::limits()
            .library
            .max_retained_nodes
            .unwrap(),
    )
}

fn observed_with_nodes(
    descriptor: &NativeProgramDescriptor,
    entry: crate::native_artifact::program::protocol::abi::Entry,
    args: &[Value],
    outputs: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    workers: usize,
    fail_push: bool,
    nodes: u64,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    struct Restore(Option<usize>, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
                .with(|value| value.set(self.0));
            FAIL_PUSH.with(|value| value.set(self.1));
        }
    }
    let _restore = Restore(
        crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
            .with(|value| value.replace(Some(workers))),
        FAIL_PUSH.with(|value| value.replace(fail_push)),
    );
    BATCHES.with(|value| value.set(0));
    let mut limits = crate::native_artifact::program::runtime::tests::limits();
    limits.library.max_retained_nodes = Some(nodes);
    execute(
        descriptor,
        |frame| {
            let previous = unsafe { (*frame).helper };
            unsafe {
                (*frame).helper = observe;
            }
            let status = unsafe { entry(frame) };
            unsafe {
                (*frame).helper = previous;
            }
            status
        },
        args,
        outputs,
        limits,
        memory,
    )
}

#[test]
fn parallel_source_bodies_keep_all_seven_maps_updates_captures_and_order() {
    let source = source(
        "number/format(h{\"a\"}, 0u64)",
        r#"
first-items := [4u64 8u64]
second-items := [1u64 2u64 3u64 4u64 5u64]
first := text/join([count-text(text, width, n) | n <- first-items], "|")
second := text/join([count-text(text, width, n) | n <- second-items], ",")
first + ";" + second
"#,
    );
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(jit.descriptor().metadata().parallel.regions.len(), 2);
        assert_eq!(
            jit.descriptor().metadata().parallel.regions[0]
                .group
                .as_ref(),
            &[0, 1]
        );
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: unchanged image and checked entry share this retained build.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = vec![input(&artifact, &memory)];
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let mut serial = vec![None; artifact.outputs().len()];
            let mut parallel = vec![None; artifact.outputs().len()];
            // Seven complete private contexts need a larger finite inventory
            // than the default single-invocation fixture. Byte/work caps stay
            // unchanged; the small-ceiling test below verifies serial fallback.
            let serial_report = observed_with_nodes(
                descriptor,
                jit.test_checked_entry(),
                &args,
                &mut serial,
                &memory,
                1,
                false,
                1_048_576,
            )
            .unwrap();
            assert_eq!(BATCHES.with(Cell::get), 0);
            assert_eq!(
                (
                    serial_report.parallel_regions,
                    serial_report.parallel_tasks,
                    serial_report.parallel_peak_workers
                ),
                (0, 0, 0)
            );
            assert_eq!(serial_report.parallel_batches, 0);
            let report = observed_with_nodes(
                descriptor,
                jit.test_checked_entry(),
                &args,
                &mut parallel,
                &memory,
                8,
                false,
                1_048_576,
            )
            .unwrap();
            assert_eq!(BATCHES.with(Cell::get), 1);
            assert_eq!((report.parallel_regions, report.parallel_tasks), (2, 7));
            assert_eq!(report.parallel_batches, 1);
            assert!((1..=7).contains(&report.parallel_peak_workers));
            assert_eq!(
                (report.hash_table_regions, report.hash_table_updates),
                (7, 27)
            );
            assert_eq!(
                (report.hash_table_regions, report.hash_table_updates),
                (
                    serial_report.hash_table_regions,
                    serial_report.hash_table_updates
                )
            );
            assert_eq!(report.packed_ssa_regions, serial_report.packed_ssa_regions);
            assert_eq!((report.frozen_map_regions, report.frozen_map_reads), (7, 7));
            assert_eq!(
                (
                    serial_report.frozen_map_regions,
                    serial_report.frozen_map_reads
                ),
                (7, 7)
            );
            for (left, right) in serial.iter().zip(&parallel) {
                assert_eq!(
                    left.as_ref()
                        .unwrap()
                        .canonical_snapshot_bytes(artifact.schemas())
                        .unwrap(),
                    right
                        .as_ref()
                        .unwrap()
                        .canonical_snapshot_bytes(artifact.schemas())
                        .unwrap()
                );
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
fn parallel_narrow_compact_domains_preserve_all_seven_source_maps_and_shared_cleanup() {
    let source = source(
        "number/format(h{\"A\"}, 0u64)",
        r#"
first-items := [4u64 8u64]
second-items := [1u64 2u64 3u64 4u64 5u64]
first := text/join([count-text(text, width, n) | n <- first-items], "|")
second := text/join([count-text(text, width, n) | n <- second-items], ",")
first + ";" + second
"#,
    )
    .replace(
        "text := seed<string>",
        r#"letters := {"a":"A", "b":"B", ":":"C"}
text := text/join([letters{char} | char <- seed<string>], "")"#,
    );
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(
            jit.test_emissions().loop_bounds.0,
            2,
            "both worker source bodies have guarded versions"
        );
        assert_eq!(jit.descriptor().metadata().parallel.regions.len(), 2);
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: restored metadata and the retained generated entry share this build.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            let domains = descriptor
                .metadata()
                .closed_tails
                .regions
                .iter()
                .filter_map(|region| region.packed.as_ref())
                .map(|proof| {
                    (
                        proof.packing_domain().bits(),
                        proof.packing_domain().shift(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(domains, [(2, Some(0)), (2, Some(0))]);
            for proof in descriptor
                .metadata()
                .closed_tails
                .regions
                .iter()
                .filter_map(|region| region.packed.as_ref())
            {
                assert_eq!(
                    crate::native_artifact::program::runtime::maps::packed::slot_bytes(
                        proof.packing_domain()
                    ),
                    16
                );
            }
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = vec![input(&artifact, &memory)];
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let mut serial = vec![None; artifact.outputs().len()];
            let serial_report = observed_with_nodes(
                descriptor,
                jit.test_checked_entry(),
                &args,
                &mut serial,
                &memory,
                1,
                false,
                1_048_576,
            )
            .unwrap();
            let mut parallel = vec![None; artifact.outputs().len()];
            let report = observed_with_nodes(
                descriptor,
                jit.test_checked_entry(),
                &args,
                &mut parallel,
                &memory,
                8,
                false,
                1_048_576,
            )
            .unwrap();
            assert_eq!(
                (
                    report.parallel_batches,
                    report.parallel_regions,
                    report.parallel_tasks
                ),
                (1, 2, 7)
            );
            assert_eq!(
                (
                    report.packed_map_regions,
                    report.packed_ssa_regions,
                    report.packed_map_updates
                ),
                (7, 7, 27)
            );
            assert_eq!(
                (
                    serial_report.packed_map_regions,
                    serial_report.packed_ssa_regions,
                    serial_report.packed_map_updates
                ),
                (7, 7, 27)
            );
            assert_eq!(
                (
                    serial_report.guarded_loop_regions,
                    report.guarded_loop_regions
                ),
                (7, 7),
                "every child must actually select the fast version, not only compile it"
            );
            for (left, right) in serial.iter().zip(&parallel) {
                assert_eq!(
                    left.as_ref()
                        .unwrap()
                        .canonical_snapshot_bytes(artifact.schemas())
                        .unwrap(),
                    right
                        .as_ref()
                        .unwrap()
                        .canonical_snapshot_bytes(artifact.schemas())
                        .unwrap()
                );
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
    let source = source(
        "number/format(h{\"a\"} + 8u64 / (8u64 - n), 0u64)",
        r#"
first-items := [1u64 4u64]
second-items := [8u64 8u64]
first := text/join([count-text(text, width, n) | n <- first-items], "|")
second := text/join([count-text(text, width, n) | n <- second-items], ",")
first + ";" + second
"#,
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    for fail_push in [false, true] {
        let mut errors = Vec::new();
        for workers in [1, 8] {
            let mut output = vec![None; artifact.outputs().len()];
            let error = observed_with_nodes(
                jit.descriptor(),
                jit.test_checked_entry(),
                &args,
                &mut output,
                &memory,
                workers,
                fail_push,
                1_048_576,
            )
            .unwrap_err();
            if workers == 8 {
                assert_eq!(BATCHES.with(Cell::get), 1);
            }
            assert!(output.iter().all(Option::is_none));
            errors.push(format!("{error:?}"));
            drop(output);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        assert_eq!(errors[0], errors[1], "source error order differs");
        assert!(errors[0].contains(if fail_push {
            "OutputBytes"
        } else {
            "Arithmetic"
        }));
    }
}

#[test]
fn parallel_string_query_elements_preserve_variable_widths_and_guarded_map_lookup() {
    let source = format!(
        r#"{}
+> number
+> text
+> sequence
key-count(s<string>, width<u64>, n<u64>) => (output<string>) :=
  h := #Count(s, width, n, empty)
  output := number/format(sequence/length(h_keys), 0u64).
query-count(s<string>, query<string>, n<u64>) => (output<string>) :=
  h := #Count(s, sequence/length(query), n, empty)
  count := (query ∈ h)?
    | true => h{{query}}
    | false => 0u64.
  output := number/format(count, 0u64) + ":" + query.
text := seed<string>
n := sequence/length(text)
first-items := [1u64 2u64]
queries := ["a" "ab" "ab:" ":ab:" "zzzzz"]
first := text/join([key-count(text, width, n) | width <- first-items], "|")
second := text/join([query-count(text, query, n) | query <- queries], ",")
first + ";" + second
"#,
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    assert_eq!(jit.descriptor().metadata().parallel.regions.len(), 2);
    assert_eq!(
        jit.descriptor().metadata().parallel.regions[0]
            .group
            .as_ref(),
        &[0, 1]
    );
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut serial = vec![None; artifact.outputs().len()];
    let serial_report = observed_with_nodes(
        jit.descriptor(),
        jit.test_checked_entry(),
        &args,
        &mut serial,
        &memory,
        1,
        false,
        1_048_576,
    )
    .unwrap();
    assert_eq!(
        (
            serial_report.hash_table_regions,
            serial_report.hash_table_updates
        ),
        (7, 45)
    );
    assert!(matches!(serial.last().unwrap().as_ref().unwrap().data(),
        ValueData::String(text) if text.as_ref() == "3|3;3:a,3:ab,2:ab:,1::ab:,0:zzzzz"));
    // Repeat the shared-account path while preserving the same ordered source
    // result. Both hit and missing-key branches execute after private map exit.
    for _ in 0..3 {
        let mut parallel = vec![None; artifact.outputs().len()];
        let report = observed_with_nodes(
            jit.descriptor(),
            jit.test_checked_entry(),
            &args,
            &mut parallel,
            &memory,
            8,
            false,
            1_048_576,
        )
        .unwrap();
        assert_eq!(
            (
                report.parallel_batches,
                report.parallel_regions,
                report.parallel_tasks
            ),
            (1, 2, 7)
        );
        assert_eq!(
            (report.hash_table_regions, report.hash_table_updates),
            (7, 45)
        );
        for (left, right) in serial.iter().zip(&parallel) {
            assert_eq!(
                left.as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                right
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            );
        }
        drop(parallel);
    }
    drop(serial);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
}

#[test]
fn insufficient_combined_worker_setup_uses_serial_source_body() {
    let source = source(
        "number/format(h{\"a\"}, 0u64)",
        r#"
first-items := [4u64 8u64]
second-items := [1u64 2u64 3u64 4u64 5u64]
first := text/join([count-text(text, width, n) | n <- first-items], "|")
second := text/join([count-text(text, width, n) | n <- second-items], ",")
first + ";" + second
"#,
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let floor = crate::native_artifact::program::runtime::session::Prepared::worker_setup_nodes(
        jit.descriptor(),
    )
    .unwrap();
    // One caller plus serial dynamic state fits; neither seven nor the later
    // five simultaneous child constructor inventories can fit this ceiling.
    let nodes = floor.checked_mul(2).unwrap().checked_add(4096).unwrap();
    assert!(floor * 5 > nodes);
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut serial = vec![None; artifact.outputs().len()];
    let mut fallback = vec![None; artifact.outputs().len()];
    let serial_report = observed_with_nodes(
        jit.descriptor(),
        jit.test_checked_entry(),
        &args,
        &mut serial,
        &memory,
        1,
        false,
        nodes,
    )
    .unwrap();
    let report = observed_with_nodes(
        jit.descriptor(),
        jit.test_checked_entry(),
        &args,
        &mut fallback,
        &memory,
        8,
        false,
        nodes,
    )
    .unwrap();
    assert_eq!(BATCHES.with(Cell::get), 0);
    assert_eq!(
        (
            report.parallel_batches,
            report.parallel_regions,
            report.parallel_tasks
        ),
        (0, 0, 0)
    );
    assert_eq!(
        (report.hash_table_regions, report.hash_table_updates),
        (7, 27)
    );
    assert_eq!(
        (report.hash_table_regions, report.hash_table_updates),
        (
            serial_report.hash_table_regions,
            serial_report.hash_table_updates
        )
    );
    for (left, right) in serial.iter().zip(&fallback) {
        assert_eq!(
            left.as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            right
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap()
        );
    }
    drop((serial, fallback));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
}

#[test]
fn insufficient_combined_worker_bytes_uses_serial_source_body() {
    let source = source(
        "number/format(h{\"a\"}, 0u64)",
        r#"
items := [4u64 8u64 1u64 2u64 3u64 4u64 5u64]
text/join([count-text(text, width, n) | n <- items], "|")
"#,
    );
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let floor =
            crate::native_artifact::program::runtime::session::Prepared::worker_setup_bytes(
                jit.descriptor(),
            )
            .unwrap();
        // One seven-task comprehension isolates the fixed constructor floor:
        // a declined sibling group must not retry a smaller parallel subgroup
        // whose later dynamic map coexistence is a different admission case.
        // Seven stacks and scheduler transport fit, together with the parent
        // and ample serial workspace. Seven complete child inventories do not.
        assert!(floor * 4 > 65536);
        let bytes = 7 * (2 * 1024 * 1024 + 65536) + floor * 3 + 65536;
        let memory = ManagedMemoryBudget::new(bytes);
        let args = vec![input(&artifact, &memory)];
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let mut serial = vec![None; artifact.outputs().len()];
        let mut fallback = vec![None; artifact.outputs().len()];
        let serial_report = observed_with_nodes(
            jit.descriptor(),
            jit.test_checked_entry(),
            &args,
            &mut serial,
            &memory,
            1,
            false,
            1_048_576,
        )
        .unwrap();
        let report = observed_with_nodes(
            jit.descriptor(),
            jit.test_checked_entry(),
            &args,
            &mut fallback,
            &memory,
            8,
            false,
            1_048_576,
        )
        .unwrap();
        assert_eq!(BATCHES.with(Cell::get), 0);
        assert_eq!((report.parallel_batches, report.parallel_tasks), (0, 0));
        assert_eq!(
            (report.hash_table_regions, report.hash_table_updates),
            (7, 27)
        );
        assert_eq!(
            (report.hash_table_regions, report.hash_table_updates),
            (
                serial_report.hash_table_regions,
                serial_report.hash_table_updates
            )
        );
        for (left, right) in serial.iter().zip(&fallback) {
            assert_eq!(
                left.as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                right
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            );
        }
        drop((serial, fallback));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        drop(args);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn independent_singleton_regions_share_one_parallel_batch() {
    let source = source(
        "number/format(h{\"a\"}, 0u64)",
        r#"
first-items := [4u64]
second-items := [8u64]
first := text/join([count-text(text, width, n) | n <- first-items], "|")
second := text/join([count-text(text, width, n) | n <- second-items], ",")
first + ";" + second
"#,
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    assert_eq!(
        jit.descriptor().metadata().parallel.regions[0]
            .group
            .as_ref(),
        &[0, 1]
    );
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut output = vec![None; artifact.outputs().len()];
    let report = observed(
        jit.descriptor(),
        jit.test_checked_entry(),
        &args,
        &mut output,
        &memory,
        8,
        false,
    )
    .unwrap();
    assert_eq!(
        (
            report.parallel_batches,
            report.parallel_regions,
            report.parallel_tasks
        ),
        (1, 2, 2)
    );
    assert!((1..=2).contains(&report.parallel_peak_workers));
    assert!(matches!(output.last().unwrap().as_ref().unwrap().data(),
        ValueData::String(text) if text.as_ref() == "2;3"));
    drop(output);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
}

#[test]
fn result_dependent_sibling_capture_keeps_separate_source_groups() {
    let source = source(
        "number/format(h{\"a\"}, 0u64)",
        r#"
first-items := [4u64 8u64]
second-items := [1u64 2u64]
first := text/join([count-text(text, width, n) | n <- first-items], "|")
dependent := first + text
text/join([count-text(dependent, width, n) | n <- second-items], ",")
"#,
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    assert_eq!(jit.descriptor().metadata().parallel.regions.len(), 2);
    assert_eq!(
        jit.descriptor().metadata().parallel.regions[0]
            .group
            .as_ref(),
        &[0]
    );
    assert_eq!(
        jit.descriptor().metadata().parallel.regions[1]
            .group
            .as_ref(),
        &[1]
    );
}

#[test]
fn worker_phase_requires_owned_child_state_before_entering_source_body() {
    let source = source(
        "number/format(h{\"a\"}, 0u64)",
        "text/join([count-text(text, width, n) | n <- [4u64 8u64]], \"|\")",
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = vec![input(&artifact, &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut output = vec![None; artifact.outputs().len()];
    let entry = jit.test_checked_entry();
    let begin = jit.descriptor().metadata().parallel.regions[0].begin;
    let error = execute(
        jit.descriptor(),
        |frame| {
            // SAFETY: mutate only this test's retained frame and restore its fields.
            let frame_ref = unsafe { &mut *frame };
            if frame_ref.phase != PHASE_TURN {
                return unsafe { entry(frame) };
            }
            let original = (
                frame_ref.phase,
                frame_ref.resume_site,
                frame_ref.available_lines,
                frame_ref.input_eof,
            );
            (
                frame_ref.phase,
                frame_ref.resume_site,
                frame_ref.available_lines,
                frame_ref.input_eof,
            ) = (PHASE_WORKER, u64::from(begin), 0, 2);
            let status = unsafe { entry(frame) };
            let frame_ref = unsafe { &mut *frame };
            (
                frame_ref.phase,
                frame_ref.resume_site,
                frame_ref.available_lines,
                frame_ref.input_eof,
            ) = original;
            status
        },
        &args,
        &mut output,
        crate::native_artifact::program::runtime::tests::limits(),
        &memory,
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("worker entry without owned child state"));
    assert!(output.iter().all(Option::is_none));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
}
