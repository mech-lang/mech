//! Actual selected frame actions, original prices and failure-atomic owner moves.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_engine::native_artifact::{
    NativeBufferFrameAction as Action, NativeBufferFrameOperation as F,
};
#[path = "projection.rs"]
mod projection;

const FLAT: &str = "+> sequence\nwalk(a<[u32]>, b<[u32]>, n<u64>, flag<bool>) => <u64> | (a,b,n,flag) => ((n > 0u64)? | true => walk(a,b,n - 1u64,flag == false) | false => sequence/length(a) + sequence/length(b)).\nwalk(data<[u32]>, data, n<u64>, flag<bool>)\n";
const LIVE: &str = "+> sequence\n<child> := <{data<[u32]> n<u64> flag<bool>}>\nkeep(p<child>, choose<bool>) => (out<child>) :=\n  q := (choose? | true => p | false => p)\n  out := (choose? | true => q | false => p).\np := {data: data<[u32]>, n: n<u64>, flag: flag<bool>}\nq := keep(p, flag)\n(q.n + p.n, q.data, q.flag == true)\n";
const NESTED_LOCAL: &str = "+> sequence\n<child> := <{data<[u32]> n<u64> flag<bool>}>\n<parent> := <{item<child> marker<u64>}>\nwrap(data<[u32]>, n<u64>, flag<bool>) => (out<parent>) :=\n  a := {data: data, n: n, flag: flag}\n  out := {item: a, marker: n}.\ninspect(data<[u32]>, n<u64>, flag<bool>) => (out<u64>) :=\n  p := wrap(data, n, flag)\n  q := wrap(data, n, flag)\n  out := p.item.n + q.item.n + p.marker + q.marker + sequence/length(p.item.data) + sequence/length(q.item.data).\ninspect(data<[u32]>, n<u64>, flag<bool>)\n";
const CHECKED: &str = "+> sequence\n<child> := <{data<[u32]> n<u64> flag<bool>}>\nkeep(p<child>, choose<bool>) => (out<child>) :=\n  q := (choose? | true => p | false => p)\n  out := (choose? | true => q | false => p).\nchecked(data<[u32]>, n<u64>, flag<bool>, allowed<bool>) => (out<(u64,[u32],bool)>) :=\n  permission! := allowed\n  p := keep({data: data, n: n, flag: flag}, flag)\n  word := p.n + 1u64\n  out := (p.n + word, p.data, p.flag == false).\nchecked(data<[u32]>, n<u64>, flag<bool>, allowed<bool>)\n";

fn leaves(layout: &Layout, fields: &[FrameField]) -> Vec<FrameField> {
    fields
        .iter()
        .flat_map(|field| match layout.children(*field) {
            Some(children) => leaves(layout, children),
            None => vec![*field],
        })
        .collect()
}
fn candidate(layout: &Layout, sites: usize, dead: bool, nested: bool) -> Option<&Action> {
    (0..sites).find_map(|site| {
        let action = layout.action(site)?;
        let F::Copy { source, output } = action.operation() else {
            return None;
        };
        let fields = layout.fields(*source)?;
        let flat = leaves(layout, fields);
        (*source != *output
            && action.retire().contains(source) == dead
            && fields.iter().any(|f| layout.children(*f).is_some()) == nested
            && flat
                .iter()
                .any(|f| matches!(f.lane(), Lane::Managed { .. }))
            && flat.iter().any(|f| matches!(f.lane(), Lane::Scalar { .. })))
        .then_some(action)
    })
}
fn plan_value(
    plan: &mech_engine::native_artifact::NativeProgramArtifactPlan,
    slot: usize,
) -> NativeMatchValue {
    plan.blocks()
        .iter()
        .find_map(|block| match block.action() {
            mech_engine::native_artifact::NativeIterationAction::LibraryCall { output, .. }
            | mech_engine::native_artifact::NativeIterationAction::Structural { output, .. }
                if output.get() as usize == slot =>
            {
                Some(*output)
            }
            mech_engine::native_artifact::NativeIterationAction::Scalar(
                mech_engine::native_artifact::NativeMatchAction::Copy { output, .. },
            ) if output.get() as usize == slot => Some(*output),
            _ => None,
        })
        .expect("selected nested root producer")
}

fn pair(
    layout: &Layout,
    plan: &mech_engine::native_artifact::NativeProgramArtifactPlan,
    nested: bool,
) -> (NativeMatchValue, NativeMatchValue) {
    if !nested {
        let action = candidate(layout, plan.blocks().len(), true, false).unwrap();
        let F::Copy { source, output } = *action.operation() else {
            unreachable!()
        };
        return (source, output);
    }
    let roots = (0..plan.values().len())
        .filter_map(|slot| {
            let fields = layout.fields_at(slot)?;
            let flat = leaves(layout, fields);
            (fields.iter().any(|f| layout.children(*f).is_some())
                && flat
                    .iter()
                    .any(|f| matches!(f.lane(), Lane::Managed { .. }))
                && flat.iter().any(|f| {
                    matches!(
                        f.lane(),
                        Lane::Scalar {
                            kind: mech_engine::native_artifact::NativeScalarKind::Bool,
                            ..
                        }
                    )
                }))
            .then_some(slot)
        })
        .collect::<Vec<_>>();
    for &a in &roots {
        for &b in &roots {
            if a != b && plan.values()[a] == plan.values()[b] {
                return (plan_value(plan, a), plan_value(plan, b));
            }
        }
    }
    panic!("two selected nested roots with the same exact type");
}

fn fill(
    layout: &Layout,
    fields: &[FrameField],
    artifact: &mech_engine::ProgramArtifact,
    frames: &mut BufferFrames,
    memory: &ManagedMemoryBudget,
    seed: u32,
) {
    for (index, field) in leaves(layout, fields).iter().enumerate() {
        match field.lane() {
            Lane::Scalar { at, kind } => {
                let bits = if kind == mech_engine::native_artifact::NativeScalarKind::Bool {
                    u64::from(seed & 1)
                } else {
                    u64::from(seed)
                };
                let words = frames.inner.as_mut().unwrap().words.values_mut();
                words[at * 2] = bits;
                words[at * 2 + 1] = 1;
            }
            Lane::Managed { at } => {
                let definition = artifact.schemas().get(field.schema()).unwrap();
                let shape = mech_core::shape_for_resolved_extents(definition, &[1, 1]).unwrap();
                let raw = ValueDraft {
                    schema: field.schema(),
                    shape_values: shape.parameter_values().into(),
                    data: ValueDataDraft::Matrix(
                        vec![ValueDataDraft::U32(seed + index as u32)].into(),
                    ),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
                    .unwrap();
                frames.inner.as_mut().unwrap().owners.values_mut()[at] =
                    Some(raw.into_memory_budget(&mut reservation).unwrap());
            }
            Lane::Composite { .. } => unreachable!(),
        }
    }
}
type Snapshot = (Vec<u64>, Vec<Option<Value>>);
fn snapshot(frames: &BufferFrames) -> Snapshot {
    let inner = frames.inner.as_ref().unwrap();
    (
        inner.words.values().to_vec(),
        inner.owners.values().to_vec(),
    )
}
fn unchanged(frames: &BufferFrames, old: &Snapshot) {
    let inner = frames.inner.as_ref().unwrap();
    assert_eq!(inner.words.values(), old.0);
    for (now, old) in inner.owners.values().iter().zip(&old.1) {
        match (now, old) {
            (Some(now), Some(old)) => {
                assert!(now.shares_frozen_storage(old));
                assert_eq!(
                    now.canonical_data_draft().unwrap(),
                    old.canonical_data_draft().unwrap()
                );
            }
            (None, None) => {}
            _ => panic!("copy changed an owner before the complete preflight"),
        }
    }
}
fn copied(
    layout: &Layout,
    frames: &BufferFrames,
    from: &[FrameField],
    to: &[FrameField],
    old: &Snapshot,
) {
    for (from, to) in leaves(layout, from).iter().zip(leaves(layout, to)) {
        let (owner, bits) = frames.current_leaf(to).unwrap();
        match from.lane() {
            Lane::Managed { at } => assert!(
                owner
                    .unwrap()
                    .shares_frozen_storage(old.1[at].as_ref().unwrap())
            ),
            Lane::Scalar { at, .. } => assert_eq!(bits, old.0[at * 2]),
            Lane::Composite { .. } => unreachable!(),
        }
    }
}
fn moved(layout: &Layout, frames: &BufferFrames, fields: &[FrameField]) {
    for field in leaves(layout, fields) {
        if let Lane::Managed { at } = field.lane() {
            assert!(
                frames.inner.as_ref().unwrap().owners.values()[at].is_none(),
                "a last-use frame must transfer its owner instead of retaining a clone"
            );
        }
    }
}

#[test]
fn dead_frame_copy_transfers_flat_and_nested_owners_with_original_prices() {
    for (source, nested) in [(FLAT, false), (NESTED_LOCAL, true)] {
        for artifact in artifacts(source) {
            let plan = mech_engine::native_artifact::lower_native_program_artifact(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let layout = plan.facts().buffer_frames();
            let (source, output) = pair(&layout, &plan, nested);
            let from = layout.fields(source).unwrap();
            let to = layout.fields(output).unwrap();
            let recipes = FrameRetirements::derive(&layout, plan.values().len()).unwrap();
            let charge = 64 * old_walk(&layout, from, &mut Vec::new()) as u64 + 64;
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
            let mut frames = BufferFrames::new(&layout, u64::MAX, &memory, &mut work).unwrap();
            for seed in [7, 18, 29] {
                fill(&layout, from, &artifact, &mut frames, &memory, seed);
                fill(&layout, to, &artifact, &mut frames, &memory, 50);
                let before = snapshot(&frames);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut exact = NativeWorkMeter::new(charge).unwrap();
                frames
                    .copy::<true>(&layout, source, output, &mut exact)
                    .unwrap();
                assert_eq!(exact.used(), charge);
                copied(&layout, &frames, from, to, &before);
                moved(&layout, &frames, from);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                let (retirement, _) = recipes.get(source.get() as usize).unwrap();
                let mut exact = NativeWorkMeter::new(retirement).unwrap();
                assert!(frames.retire(&recipes, source.get(), &mut exact).unwrap());
                assert_eq!(exact.used(), retirement);
                assert!(frames.retirement_is_empty(&recipes, source.get()));
                copied(&layout, &frames, from, to, &before);
            }
            drop(frames);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn frame_copy_preflight_and_retirement_denials_preserve_aliases_and_retry() {
    for artifact in artifacts(NESTED_LOCAL) {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let layout = plan.facts().buffer_frames();
        let (source, output) = pair(&layout, &plan, true);
        let from = layout.fields(source).unwrap();
        let to = layout.fields(output).unwrap();
        let charge = 64 * old_walk(&layout, from, &mut Vec::new()) as u64 + 64;
        let recipes = FrameRetirements::derive(&layout, plan.values().len()).unwrap();
        let (retirement, _) = recipes.get(source.get() as usize).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut setup = NativeWorkMeter::new(u64::MAX).unwrap();
        let mut frames = BufferFrames::new(&layout, u64::MAX, &memory, &mut setup).unwrap();
        fill(&layout, from, &artifact, &mut frames, &memory, 9);
        fill(&layout, to, &artifact, &mut frames, &memory, 42);
        let published = snapshot(&frames);
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut denied = NativeWorkMeter::new(charge - 1).unwrap();
        assert!(
            frames
                .copy::<true>(&layout, source, output, &mut denied)
                .is_err()
        );
        assert_eq!(denied.used(), 0);
        unchanged(&frames, &published);
        let fields = leaves(&layout, from);
        let boolean = fields
            .iter()
            .rfind(|f| {
                matches!(
                    f.lane(),
                    Lane::Scalar {
                        kind: mech_engine::native_artifact::NativeScalarKind::Bool,
                        ..
                    }
                )
            })
            .unwrap();
        let Lane::Scalar { at, .. } = boolean.lane() else {
            unreachable!()
        };
        assert!(
            fields
                .iter()
                .position(|f| matches!(f.lane(), Lane::Managed { .. }))
                .unwrap()
                < fields
                    .iter()
                    .position(|f| f.lane() == boolean.lane())
                    .unwrap()
        );
        frames.inner.as_mut().unwrap().words.values_mut()[at * 2] = 2;
        let invalid = snapshot(&frames);
        let mut exact = NativeWorkMeter::new(charge).unwrap();
        assert!(
            frames
                .copy::<true>(&layout, source, output, &mut exact)
                .is_err()
        );
        assert_eq!(exact.used(), charge);
        unchanged(&frames, &invalid);
        frames.inner.as_mut().unwrap().words.values_mut()[at * 2] = 1;
        let mut prefix = NativeWorkMeter::new(charge + retirement - 1).unwrap();
        frames
            .copy::<true>(&layout, source, output, &mut prefix)
            .unwrap();
        assert!(frames.retire(&recipes, source.get(), &mut prefix).is_err());
        assert_eq!(prefix.used(), charge);
        copied(&layout, &frames, from, to, &published);
        moved(&layout, &frames, from);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained
        );
        fill(&layout, from, &artifact, &mut frames, &memory, 20);
        let fresh = snapshot(&frames);
        let mut retry = NativeWorkMeter::new(charge + retirement).unwrap();
        frames
            .copy::<true>(&layout, source, output, &mut retry)
            .unwrap();
        assert!(frames.retire(&recipes, source.get(), &mut retry).unwrap());
        assert_eq!(retry.used(), charge + retirement);
        copied(&layout, &frames, from, to, &fresh);
        drop((frames, fresh, published, invalid));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn live_and_identity_frame_copies_retain_owners_and_selected_lanes_are_disjoint() {
    let mut live = 0;
    for text in [FLAT, NESTED_LOCAL, LIVE] {
        for artifact in artifacts(text) {
            let plan = mech_engine::native_artifact::lower_native_program_artifact(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let layout = plan.facts().buffer_frames();
            let mut seen = std::collections::BTreeSet::new();
            for slot in 0..plan.values().len() {
                if let Some(fields) = layout.fields_at(slot) {
                    for field in leaves(&layout, fields) {
                        let key = match field.lane() {
                            Lane::Managed { at } => (false, at),
                            Lane::Scalar { at, .. } => (true, at),
                            Lane::Composite { .. } => unreachable!(),
                        };
                        assert!(seen.insert(key), "selected root layouts must not overlap");
                    }
                }
            }
            let Some(action) = candidate(&layout, plan.blocks().len(), false, false) else {
                continue;
            };
            live += 1;
            let F::Copy { source, output } = *action.operation() else {
                unreachable!()
            };
            let from = layout.fields(source).unwrap();
            let to = layout.fields(output).unwrap();
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
            let mut frames = BufferFrames::new(&layout, u64::MAX, &memory, &mut work).unwrap();
            fill(&layout, from, &artifact, &mut frames, &memory, 5);
            let before = snapshot(&frames);
            frames
                .copy::<true>(&layout, source, source, &mut work)
                .unwrap();
            unchanged(&frames, &before);
            frames
                .copy::<false>(&layout, source, output, &mut work)
                .unwrap();
            copied(&layout, &frames, from, from, &before);
            copied(&layout, &frames, from, to, &before);
            drop((frames, before));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
    assert!(
        live >= 2,
        "original and decoded must contain an actual live-source Copy"
    );
}

#[test]
fn checked_nested_frame_calls_preserve_publication_on_failure_and_recover() {
    for artifact in artifacts(CHECKED) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let plan = jit.descriptor().plan();
        assert!(
            candidate(
                &plan.facts().buffer_frames(),
                plan.blocks().len(),
                true,
                false
            )
            .is_some()
        );
        assert!(
            plan.iterations().is_empty(),
            "both Boolean visits exercise ordinary frame copies"
        );
        let memory = mode.memory_budget();
        let mut output = vec![None; artifact.outputs().len()];
        for (word, n, flag, allowed, error) in [
            (7, 2, true, true, None),
            (9, u64::MAX, false, false, Some("Integrity")),
            (11, u64::MAX, true, true, Some("Arithmetic")),
            (13, 7, false, true, None),
        ] {
            let args = artifact
                .inputs()
                .iter()
                .map(|port| {
                    let data = match mech_engine::decode_source_input_name(&port.name)
                        .unwrap()
                        .as_str()
                    {
                        "data" => ValueDataDraft::Matrix(vec![ValueDataDraft::U32(word)].into()),
                        "n" => ValueDataDraft::U64(n),
                        "flag" => ValueDataDraft::Bool(flag),
                        "allowed" => ValueDataDraft::Bool(allowed),
                        name => panic!("unexpected input {name}"),
                    };
                    let shape_values: Box<[u64]> = if matches!(data, ValueDataDraft::Matrix(_)) {
                        vec![
                            1;
                            artifact
                                .schemas()
                                .get(port.schema)
                                .unwrap()
                                .dimension_parameters()
                                .len()
                        ]
                        .into()
                    } else {
                        Box::new([])
                    };
                    let raw = ValueDraft {
                        schema: port.schema,
                        shape_values,
                        data,
                    }
                    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                    .unwrap();
                    let mut reservation = memory
                        .reserve_capacity(raw.memory_budget_admission_bytes(&memory).unwrap())
                        .unwrap();
                    raw.into_memory_budget(&mut reservation).unwrap()
                })
                .collect::<Vec<_>>();
            let aliases = args.clone();
            let before = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let prior = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let result = jit.evaluate_into(&args, &mut output, mode.execution(), &memory);
            if let Some(error) = error {
                assert!(format!("{:?}", result.unwrap_err()).contains(error));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                for (now, old) in output.iter().zip(prior.iter()) {
                    assert!(
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    );
                }
            } else {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    ValueDataDraft::Tuple(
                        vec![
                            ValueDataDraft::U64(n * 2 + 1),
                            ValueDataDraft::Matrix(vec![ValueDataDraft::U32(word)].into()),
                            ValueDataDraft::Bool(!flag)
                        ]
                        .into()
                    )
                );
            }
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
