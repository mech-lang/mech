use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, NativeValueArena};

#[derive(Debug, PartialEq)]
struct Outcome {
    error: Option<String>,
    spent: u64,
    words: Vec<u64>,
    owners: Vec<Option<mech_core::ValueDataDraft>>,
    arena: Vec<Option<mech_core::ValueDataDraft>>,
    selected: bool,
}

fn trial(
    artifact: &mech_engine::ProgramArtifact,
    descriptor: &crate::native_artifact::NativeProgramDescriptor,
    slots: &[u64],
    allowance: u64,
    optimized: bool,
    foreign: bool,
    nested: bool,
) -> Outcome {
    let metadata = descriptor.metadata();
    let layout = &metadata.buffer_frames;
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let mut work = NativeWorkMeter::new(1_000_000_000_000).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 1000, &memory)
        .unwrap();
    let mut frames = BufferFrames::new(layout, 32 * 1024 * 1024, &memory, &mut work).unwrap();
    let mut arena =
        NativeValueArena::new(&metadata.types, 32 * 1024 * 1024, &memory, &mut work).unwrap();
    frames.inner.as_mut().unwrap().words.values_mut().fill(1);
    let mut observed = Vec::new();
    for &slot in slots {
        let Some(ty) = metadata.types.get(slot as usize) else {
            continue;
        };
        if let Some(fields) = layout.fields_at(slot as usize) {
            fill_owners(layout, fields, artifact, &mut frames, &memory);
        } else if matches!(
            artifact.schemas().get(ty.schema).unwrap().body(),
            mech_core::SchemaBody::Matrix { .. }
        ) {
            let payload = value(ty.schema, artifact, &memory);
            arena.put(slot as u32, &payload, &mut work).unwrap();
        }
        observed.push(slot as u32);
    }
    // Independent caller roots keep payload content and publication alive.
    let aliases = frames.inner.as_ref().unwrap().owners.values().to_vec();
    let arena_aliases = observed
        .iter()
        .filter_map(|&slot| arena.native_entry_preview(slot, &work).unwrap().cloned())
        .collect::<Vec<_>>();
    let published = aliases.iter().flatten().next().unwrap().clone();
    let published_data = published.canonical_data_draft().unwrap();
    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
    let mut other = NativeWorkMeter::new(1_000_000_000_000).unwrap();
    let meter = if foreign { &mut other } else { &mut work };
    let before = meter.used();
    if !foreign {
        library.restrict(before + allowance, 1000).unwrap();
    }
    let mut run = || -> Result<bool, NativeProgramExecutionError> {
        if optimized {
            super::super::super::retirement::run(
                slots,
                &metadata.frame_retirements,
                &mut frames,
                &mut arena,
                meter,
            )
        } else {
            // The maintained old entries are the oracle, including their error
            // wrapping and every individual denied contribution.
            meter.charge(NativeExecutionPhase::Setup, amount(slots.len(), 16, 64)?)?;
            for &slot in slots {
                frames.retire(&metadata.frame_retirements, slot as u32, meter)?;
                arena.retire_current(slot as u32, meter)?;
            }
            Ok(false)
        }
    };
    let result = if nested {
        library.with_call_limits(allowance, 0, run).unwrap()
    } else {
        run()
    };
    let spent = meter.used() - before;
    let outcome = Outcome {
        error: result.as_ref().err().map(|e| format!("{e:?}")),
        selected: result.unwrap_or(false),
        spent,
        words: frames.inner.as_ref().unwrap().words.values().to_vec(),
        owners: frames
            .inner
            .as_ref()
            .unwrap()
            .owners
            .values()
            .iter()
            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
            .collect(),
        arena: observed
            .iter()
            .map(|&slot| {
                arena
                    .native_entry_preview(slot, &work)
                    .unwrap()
                    .map(|v| v.canonical_data_draft().unwrap())
            })
            .collect(),
    };
    assert_eq!(published.canonical_data_draft().unwrap(), published_data);
    assert!(
        aliases
            .iter()
            .flatten()
            .any(|v| v.shares_frozen_storage(&published))
    );
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        held
    );
    drop((
        frames,
        arena,
        aliases,
        arena_aliases,
        published,
        work,
        other,
        library,
    ));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
    outcome
}

fn fixture(
    artifact: &mech_engine::ProgramArtifact,
) -> (JitProgramArtifact, Vec<u64>, u64, Vec<u64>) {
    let jit = JitProgramArtifact::compile_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let meta = jit.descriptor().metadata();
    let frames = (0..meta.types.len())
        .filter(|&slot| meta.buffer_frames.fields_at(slot).is_some())
        .collect::<Vec<_>>();
    let matrix = (0..meta.types.len())
        .find(|&slot| {
            meta.buffer_frames.fields_at(slot).is_none()
                && matches!(
                    artifact
                        .schemas()
                        .get(meta.types[slot].schema)
                        .unwrap()
                        .body(),
                    mech_core::SchemaBody::Matrix { .. }
                )
        })
        .unwrap();
    let slots = vec![frames[0] as u64, matrix as u64, frames[1] as u64];
    let contributions = [
        64 + slots.len() as u64 * 16,
        meta.frame_retirements.get(frames[0]).unwrap().0,
        32,
        32,
        meta.frame_retirements.get(frames[1]).unwrap().0,
        32,
    ];
    let mut cumulative = 0;
    let mut boundaries = vec![0];
    for price in contributions {
        cumulative += price;
        boundaries.extend([cumulative - 1, cumulative]);
    }
    boundaries.sort();
    boundaries.dedup();
    (jit, slots, cumulative, boundaries)
}

#[test]
fn cleanup_batch_matches_every_original_charge_boundary_and_spent_prefix() {
    for artifact in artifacts(NESTED) {
        let (jit, slots, full, boundaries) = fixture(&artifact);
        for allowance in boundaries.into_iter().chain([full + 1, full * 2]) {
            let mut selected = trial(
                &artifact,
                jit.descriptor(),
                &slots,
                allowance,
                true,
                false,
                false,
            );
            assert_eq!(selected.selected, allowance >= full);
            selected.selected = false;
            assert_eq!(
                selected,
                trial(
                    &artifact,
                    jit.descriptor(),
                    &slots,
                    allowance,
                    false,
                    false,
                    false
                ),
                "allowance {allowance}"
            );
        }
    }
}

#[test]
fn cleanup_batch_keeps_malformed_foreign_nested_paths_and_recovers() {
    for artifact in artifacts(NESTED) {
        let (jit, slots, full, _) = fixture(&artifact);
        let mut malformed = slots.clone();
        *malformed.last_mut().unwrap() = u64::from(u32::MAX);
        for (input, foreign, nested, allowance) in [
            (&malformed, false, false, full * 2),
            (&slots, true, false, full * 2),
            (&slots, false, true, full - 1),
            (&slots, false, true, full),
        ] {
            let optimized = trial(
                &artifact,
                jit.descriptor(),
                input,
                allowance,
                true,
                foreign,
                nested,
            );
            assert!(!optimized.selected);
            assert_eq!(
                optimized,
                trial(
                    &artifact,
                    jit.descriptor(),
                    input,
                    allowance,
                    false,
                    foreign,
                    nested
                )
            );
        }
        assert!(
            trial(
                &artifact,
                jit.descriptor(),
                &slots,
                full,
                true,
                false,
                false
            )
            .selected
        );
    }
}
