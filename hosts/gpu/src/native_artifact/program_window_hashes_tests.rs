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
    WindowHashes::new(
        source.as_ptr(),
        source.len(),
        width,
        count,
        0,
        origin(),
        work,
        temporary,
    )
    .unwrap()
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
            table.update(i as u64).unwrap();
        }
        table.validate(0, table.accepted, table.accepted).unwrap();
        assert_eq!(table.updated, (source.len() - width as usize + 1) as u64);
        assert_eq!(table.counts.values().sum::<u64>(), table.updated);
        assert_eq!(table.counts.len(), expected.len());
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
    }
}

#[test]
fn hash_windows_keep_full_width_ascii_keys_and_reject_replays() {
    let source = "\0abcdefghijklmnopq\0abcdefghijklmnopr";
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    let mut table = owner(source, 18, &memory, 1 << 40, 1 << 20);
    assert!(table.update(1).is_err());
    assert_eq!(table.updated, 0);
    table.update(0).unwrap();
    assert!(table.update(0).is_err());
    for index in 1..table.accepted {
        table.update(index).unwrap();
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
    assert!(table.update(0).is_err());
    assert_eq!(table.updated, 0);
    assert!(table.counts.is_empty());
    assert_eq!(memory.used_bytes(), before);
    drop(table);
    assert_eq!(memory.used_bytes(), 0);
    let mut table = owner(source, 1, &memory, 0, 1 << 20);
    assert!(table.update(0).is_err());
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
    let (id, ir, _, _) = window_codegen::define(&mut module, &region).unwrap();
    assert!(
        ir.contains("call_indirect"),
        "every window must update the table: {ir}"
    );
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
    // SAFETY: exact five-word emitted signature; module retains code below.
    let entry: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    let source = "ABACABADABACABA";
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    for width in [1, 2, 3, 4, 6, 12, 18] {
        let mut table = owner(source, width, &memory, 1 << 40, 1 << 20);
        let count = table.accepted;
        // SAFETY: stable exclusive owner and source remain live across entry.
        let completed = unsafe {
            entry(
                0,
                count,
                (&mut table as *mut WindowHashes) as u64,
                update as *const () as u64,
                width,
            )
        };
        table.validate(0, count, completed).unwrap();
        assert_eq!(table.counts.values().sum::<u64>(), count);
    }
    let mut table = owner(source, 1, &memory, 0, 1 << 20);
    let count = table.accepted;
    let completed = unsafe {
        entry(
            0,
            count,
            (&mut table as *mut WindowHashes) as u64,
            update as *const () as u64,
            1,
        )
    };
    assert_eq!(completed, 0);
    assert!(table.validate(0, count, completed).is_err());
    assert!(table.counts.is_empty());
}
