use super::*;
use crate::native_artifact::{JitProgramArtifact, program_compiler::window_codegen};
use cranelift_module::default_libcall_names;

fn origin() -> NativeMatchOrigin {
    let artifact = super::super::super::tests::artifact_source(include_str!(
        "../../tests/fixtures/native/window-histogram.mec"
    ));
    let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    jit.descriptor().plan().collections()[0].origin()
}
fn owner(
    source: &str,
    width: u64,
    memory: &mech_core::ManagedMemoryBudget,
    work_limit: u64,
    temporary: u64,
) -> WindowHashes {
    let work = NativeLibraryWorkBudget::new(work_limit, work_limit, memory).unwrap();
    let count = if width > 0 && width <= source.len() as u64 {
        source.len() as u64 - width + 1
    } else {
        0
    };
    WindowHashes::new(source.len(), width, count, 0, origin(), work, temporary, 1 << 30).unwrap()
}

#[test]
fn hash_windows_update_every_key_for_all_seven_widths_and_release_all_backing() {
    let source = (0..512)
        .map(|i| match (i * i + i / 3) % 4 {
            0 => 'A',
            1 => 'C',
            2 => 'G',
            _ => 'T',
        })
        .collect::<String>();
    let memory = mech_core::ManagedMemoryBudget::new(16 << 20);
    for width in [1, 2, 3, 4, 6, 12, 18] {
        let mut table = owner(&source, width, &memory, 1 << 50, 8 << 20);
        let mut expected = std::collections::BTreeMap::new();
        for (i, bytes) in source.as_bytes().windows(width as usize).enumerate() {
            *expected.entry(bytes.to_vec()).or_insert(0u64) += 1;
            table.update(i as u64, pack(bytes).unwrap()).unwrap();
        }
        table.validate(0, table.accepted, table.accepted).unwrap();
        assert_eq!(table.updated, (source.len() - width as usize + 1) as u64);
        assert_eq!(table.counts.values().sum::<u64>(), table.updated);
        assert_eq!(table.counts.len(), expected.len());
        assert_eq!(table.entries.nodes(), expected.len() as u64);
        assert_eq!(memory.retained_nodes(), table.metadata_nodes.nodes() + table.entries.nodes());
        for (key, count) in expected {
            assert_eq!(table.count(&key).unwrap(), count);
        }
        assert_eq!(table.count(b"not a window").unwrap(), 0);
        assert_eq!(
            table.counts.allocation_size() as u64,
            table.backing.capacity_bytes()
        );
        drop(table);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_nodes(), 0);
    }
}

#[test]
fn hash_window_node_admission_covers_owner_entries_and_rehash_peak() {
    let source = "abcdefghijklmnop";
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    let mut table = owner(source, 1, &memory, 1 << 50, 1 << 20);
    let fixed = table.metadata_nodes.nodes();
    table.retained_limit = fixed + 2;
    table.update(0, pack(b"a").unwrap()).unwrap();
    table.update(1, pack(b"b").unwrap()).unwrap();
    let before = (memory.used_bytes(), memory.retained_nodes());
    assert!(matches!(table.update(2, pack(b"c").unwrap()), Err(Error::Collection { origin: at, .. }) if at == table.origin));
    assert_eq!((table.updated, table.counts.len(), table.entries.nodes()), (2, 2, 2));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
    table.retained_limit = 1 << 30;
    for index in 2..table.counts.capacity() {
        table.update(index as u64, pack(&source.as_bytes()[index..index+1]).unwrap()).unwrap();
    }
    let count = table.counts.len() as u64;
    assert_eq!(count as usize, table.counts.capacity());
    let peak = fixed + count + 1 + count + 1;
    table.retained_limit = peak - 1;
    let before = (memory.used_bytes(), memory.retained_nodes());
    assert!(table.update(count, pack(&source.as_bytes()[count as usize..count as usize+1]).unwrap()).is_err());
    assert_eq!((table.updated, table.counts.len() as u64, table.entries.nodes()), (count, count, count));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
    table.retained_limit = peak;
    table.update(count, pack(&source.as_bytes()[count as usize..count as usize+1]).unwrap()).unwrap();
    assert_eq!(memory.retained_nodes(), fixed + count + 1);
    drop(table);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));

    // The same coexistence bound must be enforced by the shared account,
    // independently of the component limit used above.
    for limit in [peak - 1, peak] {
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20).with_limits(1 << 20, limit);
        let mut table = owner(source, 1, &memory, 1 << 50, 1 << 20);
        for index in 0..count {
            table.update(index, pack(&source.as_bytes()[index as usize..index as usize+1]).unwrap()).unwrap();
        }
        let before = (memory.used_bytes(), memory.retained_nodes());
        let next = table.update(count, pack(&source.as_bytes()[count as usize..count as usize+1]).unwrap());
        if limit < peak {
            assert!(next.is_err());
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            assert_eq!(table.updated, count);
        } else {
            next.unwrap();
            assert_eq!(memory.retained_nodes(), fixed + count + 1);
        }
        drop(table);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn hash_window_denied_owner_nodes_release_pending_byte_claims() {
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
    assert!(WindowHashes::new(1, 1, 1, 0, origin(), work, 1 << 20, 0).is_err());
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    let memory = memory.with_limits(1 << 20, 0);
    let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
    assert!(WindowHashes::new(1, 1, 1, 0, origin(), work, 1 << 20, 1 << 30).is_err());
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn hash_windows_keep_full_width_ascii_keys_and_reject_replays() {
    let source = "\0abcdefghijklmnopq\0abcdefghijklmnopr";
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    let mut table = owner(source, 18, &memory, 1 << 40, 1 << 20);
    assert!(table.update(1, 0).is_err());
    assert_eq!(table.updated, 0);
    table
        .update(0, pack(&source.as_bytes()[..18]).unwrap())
        .unwrap();
    assert!(table.update(0, 0).is_err());
    for index in 1..table.accepted {
        table
            .update(
                index,
                pack(&source.as_bytes()[index as usize..index as usize + 18]).unwrap(),
            )
            .unwrap();
    }
    assert_eq!(table.count(&source.as_bytes()[..18]).unwrap(), 1);
    assert_eq!(table.count(&source.as_bytes()[18..]).unwrap(), 1);
    assert!(table.validate(1, table.accepted, table.accepted).is_err());
    assert!(
        table
            .validate(0, table.accepted, table.accepted - 1)
            .is_err()
    );
}

#[test]
fn hash_windows_deny_growth_and_work_before_mutating_table() {
    let source = "ACGT";
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    let mut table = owner(
        source,
        1,
        &memory,
        1 << 40,
        size_of::<WindowHashes>() as u64,
    );
    let before = memory.used_bytes();
    assert!(table.update(0, 0).is_err());
    assert_eq!(table.updated, 0);
    assert!(table.counts.is_empty());
    assert_eq!(memory.used_bytes(), before);
    drop(table);
    assert_eq!(memory.used_bytes(), 0);
    let mut table = owner(source, 1, &memory, 0, 1 << 20);
    assert!(table.update(0, 0).is_err());
    assert_eq!(table.updated, 0);
    assert_eq!(table.counts.allocation_size(), 0);
}

#[test]
fn hash_window_generated_loop_performs_library_updates_and_stops_on_failure() {
    let artifact = super::super::super::tests::artifact_source(include_str!(
        "../../tests/fixtures/native/window-histogram.mec"
    ));
    let catalog = mech_stdlib::source_catalog();
    let plan =
        mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog).unwrap();
    let region = plan.window_histogram_regions().remove(0);
    let builder =
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let (id, ir, stack, _) = window_codegen::define(&mut module, &region).unwrap();
    assert!(
        stack >= 128 * 16,
        "batch scratch must be in the admitted stack: {stack}"
    );
    assert!(
        ir.contains("call_indirect"),
        "every window must update the table: {ir}"
    );
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
    // SAFETY: exact five-word emitted signature; module retains code below.
    let entry: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    let source = (0..512)
        .map(|i| char::from(((i * 73) % 128) as u8))
        .collect::<String>();
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    for width in [0, 1, 2, 3, 4, 6, 9, 10, 12, 18] {
        let mut table = owner(&source, width, &memory, 1 << 40, 1 << 20);
        let count = table.accepted;
        // SAFETY: stable exclusive owner and source remain live across entry.
        let completed = unsafe {
            entry(
                source.as_ptr() as u64,
                source.len() as u64,
                (&mut table as *mut WindowHashes) as u64,
                update_batch as *const () as u64,
                width,
            )
        };
        table.validate(0, count, completed).unwrap();
        assert_eq!(table.counts.values().sum::<u64>(), count);
        if width > 0 {
            let mut expected = std::collections::BTreeMap::new();
            for bytes in source.as_bytes().windows(width as usize) {
                *expected.entry(bytes.to_vec()).or_insert(0u64) += 1;
            }
            let actual = table
                .entries()
                .map(|(key, count)| {
                    let bytes = (0..width)
                        .map(|i| ((key >> ((width - i - 1) * 7)) & 127) as u8)
                        .collect::<Vec<_>>();
                    (bytes, count)
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(actual, expected, "width {width}");
        }
    }
    let mut table = owner(&source, 1, &memory, 0, 1 << 20);
    let count = table.accepted;
    let completed = unsafe {
        entry(
            source.as_ptr() as u64,
            source.len() as u64,
            (&mut table as *mut WindowHashes) as u64,
            update_batch as *const () as u64,
            1,
        )
    };
    assert_eq!(completed, 0);
    assert!(table.validate(0, count, completed).is_err());
    assert!(table.counts.is_empty());
}

#[test]
fn hash_window_batch_bounds_and_partial_failure_prevent_publication() {
    let source = "A".repeat(129);
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    for (start, count) in [(0, 0), (0, 129), (1, 1), (128, 2), (u64::MAX, 1)] {
        let mut table = owner(&source, 1, &memory, 1 << 40, 1 << 20);
        // The rejected batch must not dereference its key pointer.
        let status = unsafe { update_batch(&mut table, start, std::ptr::null(), count) };
        assert_eq!(status, 0);
        assert_eq!(table.updated, 0);
        assert!(table.counts.is_empty());
        assert!(table.validate(0, table.accepted, 0).is_err());
    }
    let mut table = owner(&source, 1, &memory, 1 << 40, 1 << 20);
    // One valid pair followed by a key outside the width proof. A partly
    // updated table must remain unpublishable after the callback fails.
    let pairs = [u64::from(b'A'), 0, u64::from(b'A'), 1];
    assert_eq!(unsafe { update_batch(&mut table, 0, pairs.as_ptr(), 2) }, 0);
    assert_eq!(table.updated, 1);
    assert!(table.validate(0, table.accepted, 1).is_err());
    drop(table);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn hash_windows_original_and_decoded_programs_report_actual_table_work() {
    use mech_core::{ValueDataDraft as Data, ValueDraft, snapshot::SnapshotValidationContext};
    for (source, widths) in [
        (
            include_str!("../../tests/fixtures/native/window-histogram.mec"),
            &[1, 2][..],
        ),
        (
            include_str!("../../tests/fixtures/native/window-query.mec"),
            &[3, 4, 6, 12, 18][..],
        ),
    ] {
        let original = super::super::super::tests::artifact_source(source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            for width in widths {
                let text = "ABACABADABACABADABACABADABACABA";
                let query = &text[..*width as usize];
                let memory = mech_core::ManagedMemoryBudget::new(256 << 20);
                let args = artifact
                    .inputs()
                    .iter()
                    .map(|input| {
                        let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                        let data = match name.as_str() {
                            "input" => Data::String(text.into()),
                            "width" => Data::U64(*width),
                            "query" => Data::String(query.into()),
                            _ => panic!("unexpected input {name}"),
                        };
                        let value = ValueDraft {
                            schema: input.schema,
                            shape_values: Box::new([]),
                            data,
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap();
                        let mut claim = memory
                            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                            .unwrap();
                        value.into_memory_budget(&mut claim).unwrap()
                    })
                    .collect::<Vec<_>>();
                let mut output = vec![None; artifact.outputs().len()];
                let mut limits = super::super::super::tests::limits();
                limits.library.max_compute_work = Some(1 << 50);
                limits.library.max_comparison_work = Some(1 << 50);
                limits.execution.max_work = 1 << 52;
                let report = jit
                    .evaluate_into(&args, &mut output, limits, &memory)
                    .unwrap();
                assert_eq!(report.hash_table_regions, 1);
                assert_eq!(report.hash_table_updates, text.len() as u64 - width + 1);
                if *width > 2 {
                    let expected = text
                        .as_bytes()
                        .windows(*width as usize)
                        .filter(|part| *part == query.as_bytes())
                        .count() as u64;
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(expected)
                    );
                }
                drop(output);
                drop(args);
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn hash_window_amortized_work_matches_the_original_collision_and_growth_envelope() {
    let source = (0..3000)
        .map(|i| char::from(((i * 73) % 128) as u8))
        .collect::<String>();
    let memory = mech_core::ManagedMemoryBudget::new(16 << 20);
    let mut table = owner(&source, 18, &memory, 1 << 50, 8 << 20);
    let (mut compute, mut comparisons) = (0, 0);
    for (i, bytes) in source.as_bytes().windows(18).enumerate() {
        let key = pack(bytes).unwrap();
        let probes = table.counts.capacity() as u64 * 4 + 32;
        compute += probes * 64 + 18 * 32;
        comparisons += probes;
        if !table.counts.contains_key(&key) && table.counts.len() == table.counts.capacity() {
            let target = (table.counts.capacity() * 2).max(4);
            compute += allocation_bound(target).unwrap() * 32 + (target as u64).pow(2) * 64;
        }
        table.update(i as u64, key).unwrap();
    }
    table.validate(0, table.accepted, table.accepted).unwrap();
    assert_eq!(table.prepaid_updates, 0);
    assert_eq!(
        table.work.used(),
        (compute, comparisons),
        "grouping must never discount or renew work"
    );
}
