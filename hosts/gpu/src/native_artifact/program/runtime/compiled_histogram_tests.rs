//! Exercise the real generated batch through its original admitted call.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
struct Observation {
    operation: GlobalHelper,
    result: u64,
    status: u64,
}
thread_local! {
    static ROWS: std::cell::RefCell<Vec<Observation>> = const { std::cell::RefCell::new(Vec::new()) };
    static MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}
unsafe extern "C" fn observe(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    let tag = GlobalHelper::decode(operation);
    let mode = MODE.with(std::cell::Cell::get);
    let first = if mode == 1 && tag == GlobalHelper::HistogramPrepare {
        1
    } else if mode == 2 && tag == GlobalHelper::HistogramFinish {
        first.wrapping_add(1)
    } else {
        first
    };
    let site = site + u64::from(mode == 3 && tag == GlobalHelper::HistogramFinish);
    let operation = if mode == 4 && tag == GlobalHelper::HistogramFinish {
        GlobalHelper::HistogramPrepare.raw()
    } else {
        operation
    };
    // SAFETY: delegate the synchronous generated helper's exact retained
    // context/result lane. Malformed metadata is rejected before dereference.
    let status = unsafe { hooks::helper(context, site, operation, first, second, third, result) };
    if matches!(
        tag,
        GlobalHelper::HistogramPrepare | GlobalHelper::HistogramFinish
    ) {
        // SAFETY: execute retains this initialized result word throughout entry.
        let result = if status == STATUS_OK {
            unsafe { *result }
        } else {
            0
        };
        ROWS.with(|rows| {
            rows.borrow_mut().push(Observation {
                operation: tag,
                result,
                status,
            })
        });
    }
    status
}
fn observed(
    jit: &crate::native_artifact::JitProgramArtifact,
    descriptor: &NativeProgramDescriptor,
    inputs: &[Value],
    output: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    mode: u8,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Vec<Observation>,
) {
    ROWS.with(|rows| rows.borrow_mut().clear());
    MODE.with(|slot| slot.set(mode));
    let entry = jit.test_checked_entry();
    let outcome = execute(
        descriptor,
        |raw| {
            // SAFETY: the genuine JIT owner remains alive; only transport is
            // observed, and its immutable frame field is restored before return.
            let previous = unsafe { (*raw).helper };
            unsafe { (*raw).helper = observe };
            let status = unsafe { entry(raw) };
            unsafe { (*raw).helper = previous };
            status
        },
        inputs,
        output,
        limits,
        memory,
    );
    MODE.with(|slot| slot.set(0));
    (
        outcome,
        ROWS.with(|rows| core::mem::take(&mut *rows.borrow_mut())),
    )
}
fn caps() -> NativeProgramExecutionLimits {
    let mut limits = tests::limits();
    limits.execution.max_work = 1_000_000_000_000;
    limits.library.max_compute_work = Some(1_000_000_000_000);
    limits.library.max_comparison_work = Some(1_000_000_000_000);
    limits.library.max_output_elements = Some(u64::MAX);
    limits.library.max_output_bytes = Some(u64::MAX);
    limits.library.max_cloned_bytes = Some(u64::MAX);
    limits.library.max_retained_nodes = Some(u64::MAX);
    limits
}
fn artifacts(shape: &str) -> [mech_engine::ProgramArtifact; 2] {
    let source = format!("+> map\nwords := input<{shape}>\nmap/histogram(words)\n");
    let artifact = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}
fn input(
    artifact: &mech_engine::ProgramArtifact,
    texts: &[String],
    rows: u64,
    memory: &ManagedMemoryBudget,
) -> Value {
    let schema = artifact.inputs()[0].schema;
    let shape = mech_core::shape_for_resolved_extents(
        artifact.schemas().get(schema).unwrap(),
        &[rows, texts.len() as u64 / rows],
    )
    .unwrap();
    let value = ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into(),
        data: ValueDataDraft::Matrix(texts.iter().cloned().map(ValueDataDraft::String).collect()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}
fn assert_counts(output: &[Option<Value>], texts: &[String]) {
    let mut expected = BTreeMap::<String, u64>::new();
    for text in texts {
        *expected.entry(text.clone()).or_default() += 1;
    }
    let ValueData::Map(map) = output[0].as_ref().unwrap().data() else {
        panic!("Map output")
    };
    let actual = map
        .entries()
        .iter()
        .map(|entry| {
            let (ValueData::String(key), ValueData::U64(count)) =
                (entry.key().data(), entry.value())
            else {
                panic!("String/U64 entry")
            };
            (key.to_string(), *count)
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected);
}

#[test]
fn original_decoded_and_restored_histograms_run_the_native_leaf_and_retire_every_owner() {
    for (shape, rows, sizes) in [
        ("[string]", 1, vec![0, 1, 2, 7, 31, 128]),
        ("[string]:2,3", 2, vec![6]),
    ] {
        for artifact in artifacts(shape) {
            let jit = crate::native_artifact::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: genuine descriptor and unchanged entry are paired.
            let restored = unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &image,
                    &mech_stdlib::source_catalog(),
                )
            }
            .unwrap();
            let call = jit
                .descriptor()
                .metadata()
                .calls
                .iter()
                .flatten()
                .find(|call| call.supports_compiled_string_histogram())
                .unwrap();
            let foreign_artifact = tests::artifact_source("input<string>\n");
            let foreign_memory = ManagedMemoryBudget::new(1 << 20);
            let foreign = tests::input(&foreign_artifact, &foreign_memory);
            assert!(call.compiled_histogram_strings(&foreign).is_none());
            for size in &sizes {
                let texts = (0..*size)
                    .map(|i| match i % 8 {
                        0 => "".into(),
                        1 => "é\0界".into(),
                        2 => "e\u{301}".into(),
                        3 => "🧑‍🚀".into(),
                        4 => "é".into(),
                        5 => format!("{}{:02}", "x".repeat(65), i % 13),
                        _ => format!("tail{}", i % 17),
                    })
                    .collect::<Vec<String>>();
                let memory = ManagedMemoryBudget::new(256 << 20);
                let args = [input(&artifact, &texts, rows, &memory)];
                assert_eq!(
                    call.compiled_histogram_strings(&args[0]).unwrap().len(),
                    *size
                );
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for descriptor in [jit.descriptor(), &restored] {
                    let mut output = vec![None];
                    let (result, seen) =
                        observed(&jit, descriptor, &args, &mut output, caps(), &memory, 0);
                    result.unwrap_or_else(|error| {
                        panic!("shape {shape}, size {size}, observations {seen:?}: {error:?}")
                    });
                    assert_eq!(seen.len(), 2, "{seen:?}");
                    assert_eq!(seen[0].operation, GlobalHelper::HistogramPrepare);
                    assert_ne!(seen[0].result, 0, "native batch declined: {seen:?}");
                    assert_eq!(seen[1].operation, GlobalHelper::HistogramFinish);
                    assert_eq!((seen[1].status, seen[1].result), (STATUS_OK, 1), "{seen:?}");
                    assert_counts(&output, &texts);
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
                drop(args);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn histogram_prepare_and_finish_refusal_keep_original_errors_and_allow_clean_retry() {
    for artifact in artifacts("[string]") {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let texts = (0..128)
            .map(|i| format!("é\0key{}", i % 17))
            .collect::<Vec<_>>();
        let memory = ManagedMemoryBudget::new(256 << 20);
        let args = [input(&artifact, &texts, 1, &memory)];
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut low_comparison = caps();
        low_comparison.library.max_comparison_work = Some(100_000);
        let mut output = vec![None];
        let (result, seen) = observed(
            &jit,
            jit.descriptor(),
            &args,
            &mut output,
            low_comparison,
            &memory,
            0,
        );
        result.unwrap();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert_eq!(seen[0].result, 0, "entry must decline before any bank loan");
        assert_counts(&output, &texts);
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            baseline
        );
        for mode in 0..=4 {
            let mut limits = caps();
            if mode == 0 {
                limits.library.max_output_elements = Some(0);
            }
            let mut output = vec![None];
            let (result, seen) = observed(
                &jit,
                jit.descriptor(),
                &args,
                &mut output,
                limits,
                &memory,
                mode,
            );
            let error = result.unwrap_err();
            if mode == 0 {
                assert!(
                    matches!(error, NativeProgramExecutionError::Library { .. }),
                    "{error:?}"
                );
                assert_ne!(seen[0].result, 0);
                assert_eq!(
                    seen[1].result, 0,
                    "private complete work cannot bypass publication admission"
                );
            } else {
                assert!(
                    matches!(
                        error,
                        NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
                    ),
                    "mode {mode}: {error:?}"
                );
            }
            assert!(output.iter().all(Option::is_none));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
            let mut retry = vec![None];
            let (result, seen) = observed(
                &jit,
                jit.descriptor(),
                &args,
                &mut retry,
                caps(),
                &memory,
                0,
            );
            result.unwrap();
            assert_eq!(seen[1].result, 1);
            assert_counts(&retry, &texts);
            drop(retry);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
    }
}

#[test]
fn adaptive_histogram_growth_and_refusal_replay_only_native_work_and_preserve_input() {
    for artifact in artifacts("[string]") {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let unique = (0..257)
            .map(|i| format!("{}é\0{i:04}", "x".repeat(i % 71)))
            .collect::<Vec<_>>();
        let texts = unique
            .iter()
            .chain(unique.iter().rev())
            .cloned()
            .collect::<Vec<_>>();
        let memory = ManagedMemoryBudget::new(256 << 20);
        let args = [input(&artifact, &texts, 1, &memory)];
        let original = args[0]
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for refuse_growth in [false, true, false] {
            let mut limits = caps();
            if refuse_growth {
                let source_bytes = args[0].memory_budget_retained_bytes(&memory).unwrap();
                limits.library.max_comparison_work =
                    Some(source_bytes.checked_mul(64 + 128).unwrap() - 1);
            }
            let mut output = vec![None];
            let (result, seen) = observed(
                &jit,
                jit.descriptor(),
                &args,
                &mut output,
                limits,
                &memory,
                0,
            );
            result.unwrap_or_else(|error| panic!("refuse={refuse_growth}, {seen:?}: {error:?}"));
            assert_ne!(seen[0].result, 0);
            if refuse_growth {
                assert_eq!(seen.len(), 2, "{seen:?}");
                assert_eq!(
                    seen[1].result, 0,
                    "growth must decline without publishing private counts"
                );
            } else {
                assert!(
                    seen.len() >= 5,
                    "native FULL must trigger multiple paid passes: {seen:?}"
                );
                assert!(
                    seen[1..seen.len() - 1]
                        .iter()
                        .all(|r| r.operation == GlobalHelper::HistogramFinish
                            && r.result > 1
                            && r.status == STATUS_OK)
                );
                assert_eq!(seen.last().unwrap().result, 1);
            }
            assert_counts(&output, &texts);
            assert_eq!(
                args[0]
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                original
            );
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
    }
}
