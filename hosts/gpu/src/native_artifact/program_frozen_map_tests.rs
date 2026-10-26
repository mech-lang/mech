//! Execute real generated queries against complete private maps and compare
//! the canonical publication fallback. Physical publication work may differ.
use super::*;
use crate::native_artifact::program_abi::{FROZEN_MAP_COPY, FROZEN_MAP_READ, MAP_OWNER_END};
use std::cell::Cell;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Stale,
    ExitLimit,
}
thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Normal) };
    static SAVED: Cell<u64> = const { Cell::new(0) };
    static REPLAYS: Cell<u64> = const { Cell::new(0) };
    static FAILURE_SITE: Cell<Option<usize>> = const { Cell::new(None) };
}
unsafe extern "C" fn observed(
    raw: *mut c_void,
    site: u64,
    op: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    let old = ctx.limits;
    let frozen_exit = op == MAP_OWNER_END
        && ctx
            .control
            .descriptor
            .metadata()
            .frozen_maps
            .exits
            .get(site as usize)
            .copied()
            .flatten()
            .is_some();
    if frozen_exit && MODE.with(Cell::get) == Mode::ExitLimit {
        ctx.limits.max_output_bytes = Some(0);
    }
    let first =
        if matches!(op, FROZEN_MAP_COPY | FROZEN_MAP_READ) && MODE.with(Cell::get) == Mode::Stale {
            SAVED.with(Cell::get)
        } else {
            first
        };
    if matches!(op, FROZEN_MAP_COPY | FROZEN_MAP_READ) {
        let spent = (ctx.control.work.used(), ctx.library.used());
        let ordinary = if op == FROZEN_MAP_COPY { MOVE } else { LIBRARY };
        assert!(
            ctx.helper(site, ordinary, [0; 3]).is_err(),
            "private map cannot enter an ordinary managed helper"
        );
        assert_eq!((ctx.control.work.used(), ctx.library.used()), spent);
    }
    let status = unsafe { helper(raw, site, op, first, second, third, result) };
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    ctx.limits = old;
    if status != STATUS_OK {
        FAILURE_SITE.with(|value| value.set(Some(site as usize)));
    }
    if status == STATUS_OK
        && frozen_exit
        && unsafe { *result } != 0
        && MODE.with(Cell::get) == Mode::Normal
    {
        SAVED.with(|saved| saved.set(unsafe { *result }));
    }
    if status == STATUS_OK && matches!(op, FROZEN_MAP_COPY | FROZEN_MAP_READ) {
        let spent = (ctx.control.work.used(), ctx.library.used());
        assert!(
            ctx.helper(site, op, [first, second, third]).is_err(),
            "same visit cannot replay a private read/copy"
        );
        assert_eq!((ctx.control.work.used(), ctx.library.used()), spent);
        REPLAYS.with(|count| count.set(count.get() + 1));
    }
    status
}
fn caps() -> NativeProgramExecutionLimits {
    let mut limits = tests::limits();
    limits.execution.max_work = 1 << 42;
    limits.library.max_compute_work = Some(1 << 40);
    limits.library.max_comparison_work = Some(1 << 36);
    limits.library.max_retained_nodes = Some(1 << 22);
    limits
}
fn run(
    descriptor: &NativeProgramDescriptor,
    entry: crate::native_artifact::program_abi::Entry,
    args: &[Value],
    output: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    disabled: bool,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            map_owners::frozen::TEST_DISABLE.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(map_owners::frozen::TEST_DISABLE.with(|value| value.replace(disabled)));
    execute(
        descriptor,
        |frame| {
            let previous = unsafe { (*frame).helper };
            unsafe {
                (*frame).helper = observed;
            }
            let status = unsafe { entry(frame) };
            unsafe {
                (*frame).helper = previous;
            }
            status
        },
        args,
        output,
        caps(),
        memory,
    )
}
fn string_input(
    artifact: &mech_engine::ProgramArtifact,
    index: usize,
    text: &str,
    memory: &ManagedMemoryBudget,
) -> Value {
    let value = ValueDraft {
        schema: artifact.inputs()[index].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}
fn query_inputs(
    artifact: &mech_engine::ProgramArtifact,
    text: &str,
    query: &str,
    memory: &ManagedMemoryBudget,
) -> [Value; 2] {
    assert_eq!(artifact.inputs().len(), 2);
    [
        string_input(artifact, 0, text, memory),
        string_input(artifact, 1, query, memory),
    ]
}
fn source(text: &str, width: usize, increment: u64, expression: &str) -> String {
    let expression = if expression.contains('?') {
        format!("{expression}.")
    } else {
        expression.to_owned()
    };
    format!(
        "{}\n+> sequence\nlookup(text<string>, query<string>) => (result<u64>) :=\n  h := #Count(text, {width}u64, {}u64, empty)\n  alias := h\n  selected := {expression}\n  result := selected.\nlookup(text<string>, query<string>)\n",
        structural_projection_tests::COUNT
            .replace("previous + 1u64", &format!("previous + {increment}u64")),
        text.len()
    )
}

fn require_selected(descriptor: &NativeProgramDescriptor) {
    assert_eq!(descriptor.metadata().frozen_maps.regions.len(), 1);
}

#[test]
fn frozen_prepared_constant_map_keeps_canonical_transport_across_activation() {
    let source = format!(
        "{}\nlookup(query<string>) => (result<u64>) :=\n  counts := #Count(\"abcabc\", 1u64, 6u64, empty)\n  result := counts{{query}}.\nlookup(query<string>)\n",
        structural_projection_tests::COUNT
    );
    let artifact = tests::artifact_source(&source);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    assert!(
        descriptor
            .metadata()
            .closed_tails
            .regions
            .iter()
            .any(|r| r.packed.is_some())
    );
    assert!(descriptor.metadata().plan.activation_boundary().is_some());
    assert!(
        descriptor.metadata().frozen_maps.regions.is_empty(),
        "private maps cannot cross activation"
    );
    let memory = ManagedMemoryBudget::new(256 << 20);
    let args = [string_input(&artifact, 0, "a", &memory)];
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut output = vec![None; artifact.outputs().len()];
    MODE.with(|v| v.set(Mode::Normal));
    let report = run(
        descriptor,
        jit.test_checked_entry(),
        &args,
        &mut output,
        &memory,
        false,
    )
    .unwrap();
    assert_eq!((report.frozen_map_regions, report.frozen_map_reads), (0, 0));
    assert!(matches!(
        output.last().unwrap().as_ref().unwrap().data(),
        ValueData::U64(2)
    ));
    drop(output);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    drop(args);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn frozen_queries_keep_complete_maps_arbitrary_strings_zero_values_and_restored_execution() {
    for (text, width, increment, expression, queries) in [
        (
            "abcabc",
            1,
            1,
            "(query ∈ alias)? | true => alias{query} | false => 0u64",
            vec![("a", 2), ("c", 2), ("e", 0), ("é", 0), ("", 0), ("ab", 0)],
        ),
        (
            "abcabc",
            1,
            0,
            "(query ∈ alias)? | true => 1u64 | false => 0u64",
            vec![("a", 1), ("e", 0)],
        ),
        ("abcabc", 1, 0, "alias{query}", vec![("a", 0), ("c", 0)]),
        (
            "abcdefghijklmnopqrAbcdefghijklmnopqr",
            18,
            1,
            "(query ∈ alias)? | true => alias{query} | false => 0u64",
            vec![
                ("abcdefghijklmnopqr", 1),
                ("Abcdefghijklmnopqr", 1),
                ("zbcdefghijklmnopqr", 0),
            ],
        ),
    ] {
        let original = tests::artifact_source(&source(text, width, increment, expression));
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit =
                crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            require_selected(jit.descriptor());
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            let restored =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            for descriptor in [jit.descriptor(), &restored] {
                for &(query, expected) in &queries {
                    let memory = ManagedMemoryBudget::new(256 << 20);
                    let args = query_inputs(&artifact, text, query, &memory);
                    let baseline = (memory.used_bytes(), memory.retained_nodes());
                    let mut values = Vec::new();
                    for disabled in [false, true] {
                        let mut output = vec![None; artifact.outputs().len()];
                        MODE.with(|v| v.set(Mode::Normal));
                        let report = run(
                            descriptor,
                            jit.test_checked_entry(),
                            &args,
                            &mut output,
                            &memory,
                            disabled,
                        )
                        .unwrap();
                        assert_eq!(
                            (
                                report.packed_map_regions,
                                report.packed_ssa_regions,
                                report.packed_map_updates
                            ),
                            (1, 1, (text.len() - width + 1) as u64)
                        );
                        assert_eq!(report.frozen_map_regions, u64::from(!disabled));
                        if !disabled {
                            assert!(report.frozen_map_reads >= 1);
                        } else {
                            assert_eq!(report.frozen_map_reads, 0);
                        }
                        assert!(
                            matches!(output.last().unwrap().as_ref().unwrap().data(), ValueData::U64(n) if *n == expected)
                        );
                        values.push(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_snapshot_bytes(artifact.schemas())
                                .unwrap(),
                        );
                        drop(output);
                        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                    }
                    assert_eq!(values[0], values[1]);
                    drop(args);
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
            }
        }
    }
    assert!(REPLAYS.with(Cell::get) > 0);
}

#[test]
fn frozen_missing_get_keeps_original_library_origin_and_releases_every_owner() {
    let artifact = tests::artifact_source(&source("abcabc", 1, 0, "alias{query}"));
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    require_selected(descriptor);
    let site = descriptor
        .metadata()
        .frozen_maps
        .reads
        .iter()
        .position(Option::is_some)
        .unwrap();
    for disabled in [false, true] {
        let memory = ManagedMemoryBudget::new(256 << 20);
        let args = query_inputs(&artifact, "abcabc", "z", &memory);
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let mut output = vec![None; artifact.outputs().len()];
        let error = run(
            descriptor,
            jit.test_checked_entry(),
            &args,
            &mut output,
            &memory,
            disabled,
        )
        .unwrap_err();
        assert!(
            matches!(error, NativeProgramExecutionError::Library { origin, error: NativeCollectionInvocationError::Kernel(mech_core::ResidentKernelError::InvalidInput) }
            if origin == descriptor.metadata().plan.blocks()[site].origin()),
            "{error:?}"
        );
        assert!(output.iter().all(Option::is_none));
        drop(output);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    }
}

#[test]
fn frozen_tokens_cannot_cross_invocations_and_logical_caps_fail_at_original_exit() {
    let artifact = tests::artifact_source(&source("abcabc", 1, 1, "alias{query}"));
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    let exit = descriptor.metadata().frozen_maps.regions[0].exit;
    let memory = ManagedMemoryBudget::new(256 << 20);
    let args = query_inputs(&artifact, "abcabc", "a", &memory);
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    for mode in [Mode::Normal, Mode::Stale, Mode::ExitLimit] {
        MODE.with(|v| v.set(mode));
        FAILURE_SITE.with(|v| v.set(None));
        let mut output = vec![None; artifact.outputs().len()];
        let result = run(
            descriptor,
            jit.test_checked_entry(),
            &args,
            &mut output,
            &memory,
            false,
        );
        if mode == Mode::Normal {
            assert_eq!(result.unwrap().frozen_map_regions, 1);
            assert_ne!(SAVED.with(Cell::get), 0);
        } else {
            assert!(result.is_err());
            assert!(output.iter().all(Option::is_none));
            let failed = FAILURE_SITE.with(Cell::get).unwrap();
            if mode == Mode::ExitLimit {
                assert_eq!(failed, exit);
            } else {
                assert!(
                    descriptor.metadata().frozen_maps.copies[failed].is_some()
                        || descriptor.metadata().frozen_maps.reads[failed].is_some()
                );
            }
        }
        drop(output);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    }
    MODE.with(|v| v.set(Mode::Normal));
}

#[test]
fn frozen_unsupported_enumeration_executes_canonical_fallback_and_wire_proof_cannot_be_forged() {
    let catalog = mech_stdlib::source_catalog();
    let selected = tests::artifact_source(&source("abcabc", 1, 1, "alias{query}"));
    let selected_jit =
        crate::native_artifact::JitProgramArtifact::compile(&selected, &catalog).unwrap();
    assert_eq!(
        selected_jit
            .descriptor()
            .metadata()
            .frozen_maps
            .regions
            .len(),
        1
    );
    let image = selected_jit
        .descriptor()
        .encode_build_owned_image()
        .unwrap();
    let original: serde_json::Value = serde_json::from_slice(&image).unwrap();
    for altered in 0..3 {
        let mut wire = original.clone();
        let frozen = &mut wire["descriptor"]["frozen_maps"];
        match altered {
            0 => frozen["regions"][0]["aliases"] = serde_json::json!([]),
            1 => frozen["reads"] = serde_json::json!([]),
            _ => frozen["regions"][0]["parent"] = serde_json::json!(u32::MAX),
        }
        let bytes = serde_json::to_vec(&wire).unwrap();
        // SAFETY: malformed metadata is confined to the loader and never paired
        // with generated execution; the new proof must reject it before return.
        assert!(
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.is_err()
        );
    }
    let artifact = tests::artifact_source(&source(
        "abcabc",
        1,
        1,
        "sequence/length(map/keys(alias)) + alias{query}",
    ));
    let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
    assert!(jit.descriptor().metadata().frozen_maps.regions.is_empty());
    let memory = ManagedMemoryBudget::new(256 << 20);
    let args = query_inputs(&artifact, "abcabc", "a", &memory);
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let mut output = vec![None; artifact.outputs().len()];
    MODE.with(|v| v.set(Mode::Normal));
    let report = run(
        jit.descriptor(),
        jit.test_checked_entry(),
        &args,
        &mut output,
        &memory,
        false,
    )
    .unwrap();
    assert_eq!(
        (
            report.packed_map_regions,
            report.packed_map_updates,
            report.frozen_map_regions
        ),
        (1, 6, 0)
    );
    assert_eq!(report.frozen_map_reads, 0);
    assert!(matches!(
        output.last().unwrap().as_ref().unwrap().data(),
        ValueData::U64(5)
    ));
    drop(output);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
}
