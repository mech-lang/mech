//! Exercise real generated entry, including restored metadata and failure exits.
use super::*;
use crate::native_artifact::program_abi::{CLOSED_TAIL_BEGIN, CLOSED_TAIL_END,
    MAP_OWNER_CALL, MAP_OWNER_SPAN};
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, PartialEq)]
enum Mode { Native, Decline, OneShort, Renew }
#[derive(Default, Debug)]
struct Observation {
    entries: u64, guarded: u64, rejected: u64, errors: Vec<&'static str>, bound: [u64; 7],
}
thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Native) };
    static OBS: RefCell<Observation> = RefCell::new(Observation::default());
}
fn note(f: impl FnOnce(&mut Observation)) { OBS.with(|v| f(&mut v.borrow_mut())); }
pub(super) fn record_bound_call(op: u64) {
    note(|r| if let Some(count) = r.bound.get_mut(op as usize) { *count += 1; });
}
unsafe extern "C" fn observe(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    // SAFETY: execute pins this context for the synchronous checked entry.
    let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
    let old = ctx.limits;
    let mode = MODE.with(Cell::get);
    if op == CLOSED_TAIL_BEGIN && mode == Mode::Decline { ctx.limits.max_temporary_bytes = Some(0); }
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
                        assert_eq!((observed.entries, observed.guarded, observed.rejected), (1, 0, 2));
                        // Every source window runs the fused range/projection,
                        // with no separate range or projection callback.
                        assert_eq!((observed.bound[1], observed.bound[2], observed.bound[6]), (0, 0, 7));
                    }
                    else { assert_eq!(observed.entries, 0); assert!(observed.guarded > 7); }
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
                assert_eq!((observed.bound[1], observed.bound[2], observed.bound[6]), (0, 0, 3));
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
                assert_eq!(probes, ((2 * text.len() - expected.len()) as u64, text.len() as u64));
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
