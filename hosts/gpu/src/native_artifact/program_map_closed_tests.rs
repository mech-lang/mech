//! Exercise real generated entry, including restored metadata and failure exits.
use super::*;
use crate::native_artifact::program_abi::{CLOSED_TAIL_BEGIN, CLOSED_TAIL_END,
    MAP_OWNER_CALL, MAP_OWNER_SPAN};
use std::cell::{Cell, RefCell};

#[path = "program_packed_narrow_codegen_tests.rs"]
mod narrow_codegen;

#[derive(Clone, Copy, PartialEq)]
enum Mode { Native, Decline, OneShort, Renew, TightCall, TightWrite, LimitedInsert }
#[derive(Default, Debug)]
struct Observation {
    entries: u64, packed: u64, guarded: u64, rejected: u64, errors: Vec<&'static str>, bound: [u64; 15],
}
thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Native) };
    static OBS: RefCell<Observation> = RefCell::new(Observation::default());
}
fn note(f: impl FnOnce(&mut Observation)) { OBS.with(|v| f(&mut v.borrow_mut())); }
pub(super) fn record_bound_call(op: u64) {
    note(|r| if let Some(count) = r.bound.get_mut(op as usize) { *count += 1; });
}
pub(super) fn record_packed_entry() { note(|r| r.packed += 1); }
unsafe extern "C" fn observe(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    // SAFETY: execute pins this context for the synchronous checked entry.
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    let old = ctx.limits;
    let mode = MODE.with(Cell::get);
    if op == CLOSED_TAIL_BEGIN && mode == Mode::Decline { ctx.limits.max_temporary_bytes = Some(0); }
    if op == CLOSED_TAIL_BEGIN && mode == Mode::TightCall { ctx.limits.max_comparison_work = Some(0); }
    // One byte plus the initial eight-slot census admits a read (65), but
    // refuses the first write (195). The root account retains its own limit.
    if op == CLOSED_TAIL_BEGIN && mode == Mode::TightWrite { ctx.limits.max_comparison_work = Some(65); }
    // Allows eight distinct keys and the first capacity growth, then refuses
    // the later 16-to-32 growth at the actual source insertion (op7).
    if op == CLOSED_TAIL_BEGIN && mode == Mode::LimitedInsert { ctx.limits.max_comparison_work = Some(600); }
    if matches!(op, MAP_OWNER_CALL | MAP_OWNER_SPAN) { note(|r| r.guarded += 1); }
    let first = if op == CLOSED_TAIL_END && mode == Mode::Renew { u64::MAX } else { first };
    // SAFETY: unchanged maintained helper and its exact result lane.
    let status = unsafe { helper(raw, site, op, first, second, third, result) };
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    ctx.limits = old;
    if op == CLOSED_TAIL_BEGIN && status == STATUS_OK && unsafe { *result } == 1 {
        note(|r| r.entries += 1);
        let id = ctx.control.descriptor.closed_tails.initializers[site as usize].unwrap();
        let proof = &ctx.control.descriptor.closed_tails.regions[id];
        let used = ctx.library.used();
        let previous = ctx.control.previous;
        if ctx.visit(u64::from(proof.tail.header())).is_ok()
            || ctx.helper(site, MAP_OWNER_CALL, [0; 3]).is_ok()
            || ctx.library.used() != used || ctx.control.previous != previous
        { note(|r| r.errors.push("ordinary callback entered a bound native loop")); }
        else { note(|r| r.rejected += 2); }
        if mode == Mode::OneShort {
            ctx.library.restrict(used.0 + proof.compute - 1, used.1).unwrap();
        }
    }
    status
}

#[test]
fn packed_compact_domain_executes_source_map_join_and_publishes_all_keys() {
    let source = format!(r#"{}
+> text
letters := {{"a":"A", "c":"C", "g":"G", "t":"T"}}
sequence := text/join([letters{{char}} | char <- "acgtacgtacgtacgt"], "")
#Count(sequence, 3u64, 16u64, empty)
"#, structural_projection_tests::COUNT);
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let domains = jit.descriptor().closed_tails.regions.iter()
            .filter_map(|region| region.packed.as_ref())
            .map(|proof| (proof.packing_domain().bits(), proof.packing_domain().shift()))
            .collect::<Vec<_>>();
        assert_eq!(domains, [(2, Some(1))]);
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: these descriptor bytes and native code have the same owner.
        let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None];
            let entry = jit.test_checked_entry();
            let report = execute(descriptor, |frame| unsafe { entry(frame) },
                &[], &mut output, tests::limits(), &memory).unwrap();
            assert_eq!((report.packed_map_regions, report.packed_ssa_regions, report.packed_map_updates), (1, 1, 14));
            let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
            let pairs = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                _ => panic!("String/U64 map"),
            }).collect::<Vec<_>>();
            assert_eq!(pairs, [("ACG".into(), 4), ("CGT".into(), 4), ("GTA".into(), 3), ("TAC".into(), 3)]);
            drop(output);
            assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
        }
    }
}

#[test]
fn packed_domain_boundaries_execute_rolling_keys_and_restore_descriptors_losslessly() {
    let cross_word = format!("A{}I{}ACDEFGHI", "B".repeat(16), "B".repeat(16));
    // Runtime input keeps the full ASCII representation. Include NUL, DEL and
    // both word halves without embedding control bytes in a source literal.
    let ascii = String::from_utf8((0..128u8).filter(|&byte| !matches!(byte, 10 | 13)).collect()).unwrap();
    for (text, width, bits, shift, runtime_input) in [
        ("a".repeat(40), 18usize, 1u8, Some(0u8), false),
        ("abc".repeat(12), 3, 2, Some(0), false),
        (cross_word.clone(), 16, 4, Some(0), false),
        (cross_word, 17, 4, Some(0), false),
        (ascii, 18, 7, Some(0), true),
        // The first two aligned keys share their low word and differ only in
        // their high word. The runtime's insert/rehash and generated probes
        // must agree on the full-width hash, including after restoration.
        ("abcdefghijklmnopqrAbcdefghijklmnopqr".repeat(2), 18, 7, Some(0), true),
        ("ABDH".repeat(12), 18, 7, Some(0), false),
    ] {
        let argument = if runtime_input { "text<string>".to_owned() } else { format!("\"{text}\"") };
        let source = format!("{}\n#Count({argument}, {width}u64, {}u64, empty)\n",
            structural_projection_tests::COUNT, text.len());
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let mut expected = std::collections::BTreeMap::new();
        for key in text.as_bytes().windows(width) {
            *expected.entry(std::str::from_utf8(key).unwrap().to_owned()).or_insert(0u64) += 1;
        }
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: descriptor and retained generated entry share this checked build.
            let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
            for descriptor in [jit.descriptor(), &restored] {
                let proof = descriptor.closed_tails.regions.iter().find_map(|region| region.packed.as_ref()).unwrap();
                assert_eq!((proof.packing_domain().bits(), proof.packing_domain().shift()), (bits, shift));
                assert_eq!(crate::native_artifact::program_packed_map::slot_bytes(proof.packing_domain()),
                    if bits <= 3 { 16 } else { 24 });
                if runtime_input { assert!(proof.domain().is_none()); }
                if text.starts_with("ABDH") { assert_eq!(proof.domain().unwrap().shift(), None); }
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let inputs = if runtime_input {
                    assert_eq!(artifact.inputs().len(), 1);
                    let value = ValueDraft { schema: artifact.inputs()[0].schema,
                        shape_values: Box::new([]), data: ValueDataDraft::String(text.clone()) }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
                    let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
                    vec![value.into_memory_budget(&mut reservation).unwrap()]
                } else { Vec::new() };
                let mut output = vec![None];
                let mut limits = tests::limits();
                limits.execution.max_work = 1 << 40;
                limits.library.max_compute_work = Some(1 << 36);
                limits.library.max_comparison_work = Some(1 << 34);
                let entry = jit.test_checked_entry();
                let report = execute(descriptor, |frame| unsafe { entry(frame) },
                    &inputs, &mut output, limits, &memory).unwrap();
                assert_eq!((report.packed_map_regions, report.packed_ssa_regions, report.packed_map_updates),
                    (1, 1, (text.len() - width + 1) as u64));
                let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
                let actual = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                    (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                    _ => panic!("String/U64 map"),
                }).collect::<std::collections::BTreeMap<_, _>>();
                assert_eq!(actual, expected, "width {width}, bits {bits}, runtime input {runtime_input}");
                drop((inputs, output));
                assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn closed_map_binds_source_once_rejects_reentry_and_keeps_finite_fuel() {
    let source = format!("{}\n#Count(\"ab:ab:ab\", 2u64, 8u64, empty)\n",
        structural_projection_tests::COUNT.replace("previous + 1u64", "previous + 7u64"));
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this descriptor image belongs to the exact retained JIT.
        let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            assert_eq!(descriptor.closed_tails.regions.iter().filter(|p| p.map.is_some()).count(), 1);
            let mut visits = Vec::new();
            for mode in [Mode::Native, Mode::Decline, Mode::OneShort, Mode::Renew] {
                MODE.with(|v| v.set(mode));
                OBS.with(|v| *v.borrow_mut() = Observation::default());
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None];
                let entry = jit.test_checked_entry();
                let outcome = execute(descriptor, |frame| {
                    // SAFETY: replace only our own callback, restoring it before
                    // the session checks its immutable frame.
                    let old = unsafe { (*frame).helper };
                    unsafe { (*frame).helper = observe; }
                    let status = unsafe { entry(frame) };
                    unsafe { (*frame).helper = old; }
                    status
                }, &[], &mut output, tests::limits(), &memory);
                let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
                assert!(observed.errors.is_empty(), "{observed:?}");
                if matches!(mode, Mode::OneShort | Mode::Renew) {
                    let error = outcome.unwrap_err();
                    if mode == Mode::Renew { assert!(format!("{error:?}").contains("fuel was renewed")); }
                    assert!(output.iter().all(Option::is_none));
                    assert_eq!((observed.entries, observed.guarded, observed.rejected), (1, 0, 2));
                } else {
                    let report = outcome.unwrap();
                    assert_eq!((report.hash_table_regions, report.hash_table_updates), (1, 7));
                    let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
                    let pairs = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                        (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                        _ => panic!("String/U64 map"),
                    }).collect::<Vec<_>>();
                    assert_eq!(pairs, vec![(":a".into(), 14), ("ab".into(), 21), ("b:".into(), 14)]);
                    if mode == Mode::Native {
                        assert_eq!((report.packed_map_regions, report.packed_map_updates), (1, 7));
                        assert_eq!(report.packed_ssa_regions, 1);
                        assert_eq!(observed.bound[13], 1);
                        assert_eq!((observed.entries, observed.guarded, observed.rejected), (1, 0, 2));
                        // Existing keys run entirely in generated code; only
                        // the three distinct keys require storage insertion.
                        assert_eq!(observed.packed, 1);
                        assert_eq!(&observed.bound[1..7], &[0; 6]);
                        assert_eq!(observed.bound[7], 3);
                    }
                    else {
                        assert_eq!((report.packed_map_regions, report.packed_map_updates), (0, 0));
                        assert_eq!(report.packed_ssa_regions, 0);
                        assert_eq!(observed.entries, 0); assert!(observed.guarded > 7);
                    }
                    visits.push(report.visits);
                }
                drop(output);
                assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
            }
            assert!(visits[0] < visits[1], "native/guarded visits: {visits:?}");
        }
    }
}

#[test]
fn closed_map_inside_comprehension_rebinds_each_invocation() {
    let source = format!(r#"{}
+> number
+> text
count-text(n<u64>) => (output<string>) :=
  h := #Count("ab:ab:ab", 1u64, n, empty)
  output := number/format(h{{"a"}}, 0u64).
text/join([count-text(n) | n <- [4u64 8u64]], "")
"#, structural_projection_tests::COUNT);
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(jit.descriptor().closed_tails.regions.iter()
            .any(|region| region.map.is_some() && region.tail.parent().is_some()));
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: retain exact compilation provenance for this restored image.
        let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            MODE.with(|v| v.set(Mode::Native));
            OBS.with(|v| *v.borrow_mut() = Observation::default());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None];
            let entry = jit.test_checked_entry();
            let report = execute(descriptor, |frame| {
                // SAFETY: observe the actual synchronous entry and restore its frame.
                let old = unsafe { (*frame).helper };
                unsafe { (*frame).helper = observe; }
                let status = unsafe { entry(frame) };
                unsafe { (*frame).helper = old; }
                status
            }, &[], &mut output, tests::limits(), &memory).unwrap();
            assert_eq!((report.hash_table_regions, report.hash_table_updates), (2, 12));
            assert!(matches!(output[0].as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "23"));
            let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
            assert!(observed.errors.is_empty(), "{observed:?}");
            assert_eq!((observed.entries, observed.guarded, observed.rejected), (2, 0, 4));
            drop(output);
            assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
        }
    }
}

#[test]
fn closed_map_rechecks_changed_widths_and_current_string_bounds() {
    for invalid in [false, true] {
        let source = if invalid {
            format!("{}\n#Count(\"abc\", 2u64, 4u64, empty)\n", structural_projection_tests::COUNT)
        } else {
            format!("{}\n#Count(\"abcdef\", 1u64, 3u64, empty)\n",
                structural_projection_tests::COUNT.replace("(index + width)", "(index + index + 1u64)"))
        };
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: the descriptor and generated entry share this exact build.
            let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
            for descriptor in [jit.descriptor(), &restored] {
                MODE.with(|v| v.set(Mode::Native));
                OBS.with(|v| *v.borrow_mut() = Observation::default());
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None];
                let entry = jit.test_checked_entry();
                let outcome = execute(descriptor, |frame| {
                    // SAFETY: the observer is confined to the owned synchronous entry.
                    let old = unsafe { (*frame).helper };
                    unsafe { (*frame).helper = observe; }
                    let status = unsafe { entry(frame) };
                    unsafe { (*frame).helper = old; }
                    status
                }, &[], &mut output, tests::limits(), &memory);
                let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!((observed.entries, observed.guarded), (1, 0));
                assert_eq!((observed.bound[1], observed.bound[2]), (0, 0));
                assert_eq!((observed.packed, observed.bound[6]), if invalid { (1, 1) } else { (0, 3) });
                if invalid {
                    let selection = descriptor.map_owner_words.as_ref().unwrap().spans()[0].selection_site();
                    let expected_origin = descriptor.plan.blocks()[selection as usize].origin();
                    assert!(matches!(&outcome, Err(NativeProgramExecutionError::Library {
                        origin,
                        error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::InvalidInput),
                    }) if *origin == expected_origin), "{outcome:?}");
                    assert!(output.iter().all(Option::is_none));
                } else {
                    let report = outcome.unwrap();
                    assert_eq!((report.hash_table_regions, report.hash_table_updates), (1, 3));
                    let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
                    let pairs = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                        (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                        _ => panic!("String/U64 map"),
                    }).collect::<Vec<_>>();
                    assert_eq!(pairs, vec![("a".into(), 1), ("bc".into(), 1), ("cde".into(), 1)]);
                }
                drop(output);
                assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn closed_map_reuses_only_current_lookup_and_keeps_zero_values_and_growth() {
    for text in ["aaaaa", "abcdefghijklmnop"] {
        let source = format!("{}\n#Count(\"{text}\", 1u64, {}u64, empty)\n",
            structural_projection_tests::COUNT.replace("previous + 1u64", "9u64 - previous"), text.len());
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let mut expected = std::collections::BTreeMap::new();
        for character in text.chars() {
            let key = character.to_string();
            let previous = expected.get(&key).copied().unwrap_or(0);
            expected.insert(key, 9 - previous);
        }
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: retain the exact compilation and its generated entry.
            let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
            for descriptor in [jit.descriptor(), &restored] {
                MODE.with(|v| v.set(Mode::Native));
                OBS.with(|v| *v.borrow_mut() = Observation::default());
                crate::native_artifact::owned_map_binding::take_native_probe_counts();
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None];
                let entry = jit.test_checked_entry();
                let report = execute(descriptor, |frame| {
                    // SAFETY: observe only the synchronous owned native entry.
                    let old = unsafe { (*frame).helper };
                    unsafe { (*frame).helper = observe; }
                    let status = unsafe { entry(frame) };
                    unsafe { (*frame).helper = old; }
                    status
                }, &[], &mut output, tests::limits(), &memory).unwrap();
                let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!((observed.entries, observed.guarded), (1, 0));
                assert_eq!((report.hash_table_regions, report.hash_table_updates), (1, text.len() as u64));
                let probes = crate::native_artifact::owned_map_binding::take_native_probe_counts();
                if observed.packed == 1 {
                    assert_eq!(probes, (0, 0));
                    assert_eq!(&observed.bound[1..7], &[0; 6]);
                    assert_eq!(observed.bound[7], expected.len() as u64);
                } else {
                    assert_eq!(probes, ((2 * text.len() - expected.len()) as u64, text.len() as u64));
                }
                let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
                let actual = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                    (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                    _ => panic!("String/U64 map"),
                }).collect::<std::collections::BTreeMap<_, _>>();
                assert_eq!(actual, expected);
                drop(output);
                assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn packed_map_transports_computed_and_copied_invariant_offsets() {
    for width in ["computed", "copied"] {
        let source = format!("{}\ncomputed := seed<u64> + 1u64\ncopied := computed\n#Count(\"ababab\", {width}, 6u64, empty)\n",
            structural_projection_tests::COUNT);
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: the restored descriptor shares this retained generated entry.
            let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
            for descriptor in [jit.descriptor(), &restored] {
                let proof = descriptor.closed_tails.regions.iter().find_map(|region| region.packed.as_ref()).unwrap();
                let end = proof.end_offset().expect("computed invariant offset");
                assert!(descriptor.plan.blocks().iter().any(|block| matches!(block.action(),
                    NativeIterationAction::Scalar(NativeMatchAction::Primitive { output, .. }
                        | NativeMatchAction::Copy { output, .. }) if output.get() == end)));
                for seed in [0u64, 1] {
                    MODE.with(|v| v.set(Mode::Native));
                    OBS.with(|v| *v.borrow_mut() = Observation::default());
                    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                    assert_eq!(artifact.inputs().len(), 1);
                    let value = ValueDraft { schema: artifact.inputs()[0].schema,
                        shape_values: Box::new([]), data: ValueDataDraft::U64(seed) }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
                    let mut reservation = memory.reserve_capacity(
                        value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
                    let inputs = [value.into_memory_budget(&mut reservation).unwrap()];
                    drop(reservation);
                    let mut output = vec![None];
                    let entry = jit.test_checked_entry();
                    let report = execute(descriptor, |frame| {
                        // SAFETY: observe only this synchronous owned entry.
                        let old = unsafe { (*frame).helper };
                        unsafe { (*frame).helper = observe; }
                        let status = unsafe { entry(frame) };
                        unsafe { (*frame).helper = old; }
                        status
                    }, &inputs, &mut output, tests::limits(), &memory).unwrap();
                    let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
                    assert!(observed.errors.is_empty(), "{observed:?}");
                    assert_eq!((observed.entries, observed.packed, observed.guarded), (1, 1, 0));
                    assert_eq!((report.hash_table_regions, report.hash_table_updates), (1, 6 - seed));
                    let mut expected = std::collections::BTreeMap::new();
                    for key in b"ababab".windows(seed as usize + 1) {
                        *expected.entry(std::str::from_utf8(key).unwrap().to_owned()).or_insert(0u64) += 1;
                    }
                    let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
                    let actual = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                        (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                        _ => panic!("String/U64 map"),
                    }).collect::<std::collections::BTreeMap<_, _>>();
                    assert_eq!(actual, expected);
                    drop((inputs, output));
                    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
                }
            }
        }
    }
}

#[test]
fn packed_map_checks_call_caps_only_for_executed_operations() {
    for n in [0u64, 1] {
        let source = format!("{}\n#Count(\"a\", 1u64, {n}u64, empty)\n",
            structural_projection_tests::COUNT);
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: retain the exact compilation and generated entry.
            let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
            for descriptor in [jit.descriptor(), &restored] {
                MODE.with(|v| v.set(Mode::TightCall));
                OBS.with(|v| *v.borrow_mut() = Observation::default());
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None];
                let entry = jit.test_checked_entry();
                let outcome = execute(descriptor, |frame| {
                    // SAFETY: restrict only the captured per-call cap while
                    // this synchronous owner is created; root limits stay fixed.
                    let old = unsafe { (*frame).helper };
                    unsafe { (*frame).helper = observe; }
                    let status = unsafe { entry(frame) };
                    unsafe { (*frame).helper = old; }
                    status
                }, &[], &mut output, tests::limits(), &memory);
                let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!((observed.entries, observed.packed, observed.guarded), (1, 1, 0));
                assert_eq!(observed.bound[7], 0, "a denied read cannot reach insertion");
                if n == 0 {
                    let report = outcome.unwrap();
                    assert_eq!(report.hash_table_updates, 0);
                    assert!(matches!(output[0].as_ref().unwrap().data(), ValueData::Map(map) if map.entries().is_empty()));
                } else {
                    assert!(outcome.is_err());
                    assert!(output.iter().all(Option::is_none));
                }
                drop(output);
                assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
            }
        }
    }
}

#[test]
fn packed_ssa_preserves_carriers_across_refill_growth_and_branch_merges() {
    let text = (0..4099).map(|index| char::from(b'a' + ((index * 7 + index / 26) % 26) as u8))
        .collect::<String>();
    let source = format!("{}\n#Count(\"{text}\", 3u64, {}u64, empty)\n",
        structural_projection_tests::COUNT.replace("previous + 1u64", "previous + 7u64"), text.len());
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    let mut expected = std::collections::BTreeMap::new();
    for key in text.as_bytes().windows(3) {
        *expected.entry(std::str::from_utf8(key).unwrap().to_owned()).or_insert(0u64) += 7;
    }
    assert!(expected.len() > 56, "fixture must cross several table growth boundaries");
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        MODE.with(|v| v.set(Mode::Native));
        OBS.with(|v| *v.borrow_mut() = Observation::default());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None];
        let entry = jit.test_checked_entry();
        let mut limits = tests::limits();
        // This larger fixture retains full-chain prices for thousands of
        // updates. Keep finite bounds without changing the small cap tests.
        limits.execution.max_work = 1 << 40;
        limits.library.max_compute_work = Some(1 << 36);
        limits.library.max_comparison_work = Some(1 << 34);
        let report = execute(jit.descriptor(), |frame| {
            // SAFETY: observe only this synchronous owned generated entry.
            let old = unsafe { (*frame).helper };
            unsafe { (*frame).helper = observe; }
            let status = unsafe { entry(frame) };
            unsafe { (*frame).helper = old; }
            status
        }, &[], &mut output, limits, &memory).unwrap();
        let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
        assert!(observed.errors.is_empty(), "{observed:?}");
        assert_eq!((report.packed_map_regions, report.packed_ssa_regions,
            report.packed_map_updates), (1, 1, (text.len() - 2) as u64));
        assert_eq!(observed.bound[13], 1, "SSA must enter once, not once per traversal");
        assert!(observed.bound[0] >= 5, "fixture must refill finite fuel repeatedly");
        assert_eq!(observed.bound[7], expected.len() as u64);
        assert_eq!(&observed.bound[1..7], &[0; 6]);
        let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
        let actual = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
            (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
            _ => panic!("String/U64 map"),
        }).collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(actual, expected);
        drop(output);
        assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
    }
}

#[path = "program_packed_origin_tests.rs"]
mod cold_origins;
#[path = "program_scalar_edge_fact_tests.rs"]
mod edge_facts;

#[test]
fn packed_generated_controls_compare_colliding_keys_across_wrap_and_growth() {
    use crate::native_artifact::program_packed_map::hash;
    let mut candidates: [Vec<[u8; 2]>; 128] = std::array::from_fn(|_| Vec::new());
    for low in 0..16384u64 {
        let key = [(low >> 7) as u8, (low & 127) as u8];
        if key.iter().any(|&byte| matches!(byte, 10 | 13)) { continue; }
        let mixed = hash(low, 0);
        if mixed & 15 == 15 { candidates[(mixed >> 57) as usize].push(key); }
    }
    let keys = candidates.into_iter().find(|keys| keys.len() >= 9).expect("collision fixture");
    let keys = &keys[..9];
    let text = keys.iter().chain(keys.iter().rev()).flat_map(|key| key.iter().copied()).collect::<Vec<_>>();
    let text = String::from_utf8(text).unwrap();
    // Independent fixed-width projections also exercise the fresh-key path;
    // every source pair is retained, including zero-valued key bytes.
    let count = structural_projection_tests::COUNT.replace(
        ":Next(index + 1u64, increment", ":Next(index + 2u64, increment");
    let source = format!("{count}\n#Count(text<string>, 2u64, {}u64, empty)\n", text.len());
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let proof = jit.descriptor().closed_tails.regions.iter().find_map(|r| r.packed.as_ref()).unwrap();
        assert_eq!(proof.packing_domain().bits(), 7, "external text keeps the full ASCII domain");
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        assert_eq!(artifact.inputs().len(), 1);
        let value = ValueDraft { schema: artifact.inputs()[0].schema, shape_values: Box::new([]),
            data: ValueDataDraft::String(text.clone().into()) }
            .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
        let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
        let inputs = [value.into_memory_budget(&mut reservation).unwrap()];
        drop(reservation);
        let mut output = vec![None];
        let entry = jit.test_checked_entry();
        let mut limits = tests::limits();
        limits.execution.max_work = 1 << 36;
        limits.library.max_compute_work = Some(1 << 32);
        limits.library.max_comparison_work = Some(1 << 30);
        let report = execute(jit.descriptor(), |frame| unsafe { entry(frame) },
            &inputs, &mut output, limits, &memory).unwrap();
        assert_eq!((report.packed_map_regions, report.packed_ssa_regions, report.packed_map_updates), (1, 1, 18));
        let ValueData::Map(map) = output[0].as_ref().unwrap().data() else { panic!("map"); };
        let actual = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
            (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
            _ => panic!("String/U64 map"),
        }).collect::<std::collections::BTreeMap<_, _>>();
        let expected = keys.iter().map(|key| (std::str::from_utf8(key).unwrap().to_owned(), 2))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(actual, expected);
        drop(output); drop(inputs);
        assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
    }
}
