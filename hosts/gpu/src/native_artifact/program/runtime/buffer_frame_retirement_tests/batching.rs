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

#[derive(Clone, Copy)]
enum Context {
    Root,
    Foreign,
    Nested,
    UnexpectedCanonical,
}

fn canonical_frame(
    schema: mech_core::SchemaId,
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
) -> Value {
    fn draft(body: &mech_core::SchemaBody) -> ValueDataDraft {
        use mech_core::{DimensionExpr, IntegerWidth, SchemaBody};
        match body {
            SchemaBody::Bool => ValueDataDraft::Bool(true),
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => ValueDataDraft::U32(7),
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => ValueDataDraft::U64(1),
            SchemaBody::Record(fields) => ValueDataDraft::Record(
                fields
                    .iter()
                    .map(|field| mech_core::snapshot::NamedValueDraft {
                        name: field.name.clone(),
                        value: draft(&field.schema),
                    })
                    .collect(),
            ),
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                let count = dimensions
                    .iter()
                    .map(|d| match d {
                        DimensionExpr::Constant(n) => *n as usize,
                        DimensionExpr::Parameter(_) => 1,
                        _ => panic!("unexpected fixture dimension"),
                    })
                    .product();
                ValueDataDraft::Matrix(vec![draft(element); count].into())
            }
            _ => panic!("unexpected fixture type"),
        }
    }
    let definition = artifact.schemas().get(schema).unwrap();
    let raw = ValueDraft {
        schema,
        shape_values: vec![1; definition.dimension_parameters().len()].into(),
        data: draft(definition.body()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut claim = memory
        .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    raw.into_memory_budget(&mut claim).unwrap()
}

fn trial(
    artifact: &mech_engine::ProgramArtifact,
    descriptor: &crate::native_artifact::NativeProgramDescriptor,
    slots: &[u64],
    allowance: u64,
    optimized: bool,
    context: Context,
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
    if matches!(context, Context::UnexpectedCanonical) {
        let slot = slots
            .iter()
            .copied()
            .find(|&slot| layout.fields_at(slot as usize).is_some())
            .unwrap();
        let payload = canonical_frame(metadata.types[slot as usize].schema, artifact, &memory);
        arena.put(slot as u32, &payload, &mut work).unwrap();
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
    let meter = if matches!(context, Context::Foreign) {
        &mut other
    } else {
        &mut work
    };
    let before = meter.used();
    if !matches!(context, Context::Foreign) {
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
    NativeValueArena::take_retirement_effect_count();
    let result = if matches!(context, Context::Nested) {
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
                Context::Root,
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
                    Context::Root
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
        for (input, context, allowance) in [
            (&malformed, Context::Root, full * 2),
            (&slots, Context::Foreign, full * 2),
            (&slots, Context::Nested, full - 1),
            (&slots, Context::Nested, full),
        ] {
            let optimized = trial(&artifact, jit.descriptor(), input, allowance, true, context);
            assert!(!optimized.selected);
            assert_eq!(
                optimized,
                trial(
                    &artifact,
                    jit.descriptor(),
                    input,
                    allowance,
                    false,
                    context
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
                Context::Root
            )
            .selected
        );
    }
}

#[test]
fn cleanup_batch_retires_only_selected_frame_storage_and_keeps_canonical_fields() {
    for artifact in artifacts(NESTED) {
        let (jit, slots, full, _) = fixture(&artifact);
        let mut selected = trial(
            &artifact,
            jit.descriptor(),
            &slots,
            full,
            true,
            Context::Root,
        );
        assert!(selected.selected);
        assert_eq!(
            NativeValueArena::take_retirement_effect_count(),
            1,
            "only the canonical managed field needs an arena effect"
        );
        let original = trial(
            &artifact,
            jit.descriptor(),
            &slots,
            full,
            false,
            Context::Root,
        );
        assert_eq!(NativeValueArena::take_retirement_effect_count(), 3);
        selected.selected = false;
        assert_eq!(selected, original);
    }
}

#[test]
fn cleanup_batch_declines_unexpected_canonical_frame_definition_and_recovers() {
    for artifact in artifacts(NESTED) {
        let (jit, slots, full, boundaries) = fixture(&artifact);
        for allowance in boundaries.into_iter().chain([full + 1]) {
            let selected = trial(
                &artifact,
                jit.descriptor(),
                &slots,
                allowance,
                true,
                Context::UnexpectedCanonical,
            );
            assert!(
                !selected.selected,
                "unexpected canonical presence retains ordered cleanup"
            );
            let calls = NativeValueArena::take_retirement_effect_count();
            let original = trial(
                &artifact,
                jit.descriptor(),
                &slots,
                allowance,
                false,
                Context::UnexpectedCanonical,
            );
            assert_eq!(calls, NativeValueArena::take_retirement_effect_count());
            assert_eq!(selected, original, "allowance {allowance}");
        }
        assert!(
            trial(
                &artifact,
                jit.descriptor(),
                &slots,
                full,
                true,
                Context::Root
            )
            .selected
        );
    }
}
