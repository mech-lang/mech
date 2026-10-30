use super::*;
use crate::native_artifact::program::prepare::frame_retirement::{FrameRetirements, Operation};
#[path = "buffer_frame_retirement_tests/batching.rs"]
mod batching;

const NESTED: &str = "+> sequence\n<child> := <{data<[u32]> n<u64> flag<bool>}>\n<parent> := <{first<child> sibling<child>}>\nwrap(data<[u32]>, n<u64>, flag<bool>) => (out<parent>) :=\n  a := {data: data, n: n, flag: flag}\n  b := {data: data, n: n + 1u64, flag: flag == false}\n  out := {first: a, sibling: b}.\np := wrap(data<[u32]>, n<u64>, flag<bool>)\n(p.first.n + p.sibling.n, p.first.data, p.sibling.flag == true)\n";

#[derive(Debug, Eq, PartialEq)]
enum Leaf {
    Scalar(usize),
    Managed(usize),
}
// Independent old recursive traversal, including composite nodes in the price.
fn old_walk(layout: &Layout, fields: &[FrameField], leaves: &mut Vec<Leaf>) -> usize {
    fields
        .iter()
        .map(|field| match field.lane() {
            Lane::Scalar { at, .. } => {
                leaves.push(Leaf::Scalar(at));
                1
            }
            Lane::Managed { at } => {
                leaves.push(Leaf::Managed(at));
                1
            }
            Lane::Composite { .. } => {
                1 + old_walk(layout, layout.children(*field).unwrap(), leaves)
            }
        })
        .sum()
}
fn prepared_leaves(operations: &[Operation]) -> Vec<Leaf> {
    operations
        .iter()
        .flat_map(|operation| match operation {
            Operation::Scalars(words) => {
                assert_eq!(words.start % 2, 0);
                assert_eq!(words.end % 2, 0);
                (words.start / 2..words.end / 2)
                    .map(Leaf::Scalar)
                    .collect::<Vec<_>>()
            }
            Operation::Managed(at) => vec![Leaf::Managed(*at)],
        })
        .collect()
}
fn artifacts(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let original = super::super::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn prepared_frame_retirement_matches_recursive_order_and_composite_work() {
    let flat = "+> sequence\nwalk(a<[u32]>, b<[u32]>, n<u64>, flag<bool>) => <u64> | (a,b,n,flag) => ((n > 0u64)? | true => walk(a,b,n - 1u64,flag == false) | false => sequence/length(a) + sequence/length(b)).\nwalk(data<[u32]>, data, n<u64>, flag<bool>)\n";
    let managed = "+> sequence\nr := {left: data<[u32]>, right: data}\nsequence/length(r.left) + sequence/length(r.right)\n";
    let mut saw_nested = false;
    let mut saw_coalesced = false;
    let mut saw_managed = false;
    let mut saw_managed_only = false;
    for source in ["seed<u64> + 1u64\n", managed, flat, NESTED] {
        for artifact in artifacts(source) {
            let plan = mech_engine::native_artifact::lower_native_program_artifact(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let layout = plan.facts().buffer_frames();
            let recipes = FrameRetirements::derive(&layout, plan.values().len()).unwrap();
            for value in 0..plan.values().len() {
                let Some(fields) = layout.fields_at(value) else {
                    assert!(recipes.get(value).is_none());
                    continue;
                };
                let mut expected = Vec::new();
                let count = old_walk(&layout, fields, &mut expected);
                let (work, operations) = recipes.get(value).unwrap();
                assert_eq!(work, 32 + count as u64 * 32);
                assert_eq!(prepared_leaves(operations), expected);
                saw_nested |= count > expected.len();
                saw_managed |= expected.iter().any(|leaf| matches!(leaf, Leaf::Managed(_)));
                saw_managed_only |= !expected.is_empty()
                    && expected.iter().all(|leaf| matches!(leaf, Leaf::Managed(_)));
                saw_coalesced |= operations
                    .iter()
                    .any(|op| matches!(op, Operation::Scalars(words) if words.len() > 2));
            }
            assert!(recipes.get(usize::MAX).is_none());
        }
    }
    assert!(saw_nested && saw_coalesced && saw_managed && saw_managed_only);
}

fn value(
    schema: mech_core::SchemaId,
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
) -> Value {
    let definition = artifact.schemas().get(schema).unwrap();
    let mech_core::SchemaBody::Matrix {
        element,
        dimensions,
    } = definition.body()
    else {
        panic!("fixture must have only U32 managed leaves")
    };
    assert_eq!(
        element.as_ref(),
        &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32)
    );
    let elements: usize = dimensions
        .iter()
        .map(|d| match d {
            mech_core::DimensionExpr::Constant(n) => *n as usize,
            mech_core::DimensionExpr::Parameter(_) => 1,
            _ => panic!("unexpected fixture geometry"),
        })
        .product();
    let raw = ValueDraft {
        schema,
        shape_values: vec![1; definition.dimension_parameters().len()].into(),
        data: ValueDataDraft::Matrix(vec![ValueDataDraft::U32(7); elements].into()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    raw.into_memory_budget(&mut reservation).unwrap()
}
fn fill_owners(
    layout: &Layout,
    fields: &[FrameField],
    artifact: &mech_engine::ProgramArtifact,
    frames: &mut BufferFrames,
    memory: &ManagedMemoryBudget,
) {
    for field in fields {
        match field.lane() {
            Lane::Managed { at } => {
                frames.inner.as_mut().unwrap().owners.values_mut()[at] =
                    Some(value(field.schema(), artifact, memory))
            }
            Lane::Composite { .. } => fill_owners(
                layout,
                layout.children(*field).unwrap(),
                artifact,
                frames,
                memory,
            ),
            Lane::Scalar { .. } => {}
        }
    }
}
fn current(frames: &BufferFrames, leaves: &[Leaf], present: bool) {
    let inner = frames.inner.as_ref().unwrap();
    for leaf in leaves {
        match leaf {
            Leaf::Scalar(at) => assert_eq!(
                &inner.words.values()[at * 2..at * 2 + 2],
                if present { &[1, 1] } else { &[0, 0] }
            ),
            Leaf::Managed(at) => assert_eq!(inner.owners.values()[*at].is_some(), present),
        }
    }
}

#[test]
fn prepared_frame_retirement_keeps_partial_denial_sibling_aliases_and_retry() {
    for artifact in artifacts(NESTED) {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let layout = plan.facts().buffer_frames();
        let recipes = FrameRetirements::derive(&layout, plan.values().len()).unwrap();
        let candidates = (0..plan.values().len())
            .filter_map(|slot| {
                let (work, ops) = recipes.get(slot)?;
                let leaves = prepared_leaves(ops);
                (leaves.iter().any(|x| matches!(x, Leaf::Scalar(_)))
                    && leaves.iter().any(|x| matches!(x, Leaf::Managed(_))))
                .then_some((slot, work, leaves))
            })
            .collect::<Vec<_>>();
        let a = &candidates[0];
        let b = candidates
            .iter()
            .find(|b| !b.2.iter().any(|leaf| a.2.contains(leaf)))
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut setup = NativeWorkMeter::new(1_000_000_000).unwrap();
        let mut frames = BufferFrames::new(&layout, 32 * 1024 * 1024, &memory, &mut setup).unwrap();
        for _ in 0..2 {
            frames.inner.as_mut().unwrap().words.values_mut().fill(1);
            for slot in [a.0, b.0] {
                fill_owners(
                    &layout,
                    layout.fields_at(slot).unwrap(),
                    &artifact,
                    &mut frames,
                    &memory,
                );
            }
            let aliases = frames.inner.as_ref().unwrap().owners.values().to_vec();
            let published = aliases.iter().flatten().next().unwrap().clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut limited = NativeWorkMeter::new(a.1 + b.1 - 1).unwrap();
            assert!(frames.retire(&recipes, a.0 as u32, &mut limited).unwrap());
            current(&frames, &a.2, false);
            current(&frames, &b.2, true);
            assert!(frames.retire(&recipes, b.0 as u32, &mut limited).is_err());
            assert_eq!(limited.used(), a.1);
            current(&frames, &b.2, true);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(
                aliases
                    .iter()
                    .flatten()
                    .any(|v| v.shares_frozen_storage(&published))
            );
            let mut exact = NativeWorkMeter::new(b.1).unwrap();
            assert!(frames.retire(&recipes, b.0 as u32, &mut exact).unwrap());
            assert_eq!(exact.used(), b.1);
            current(&frames, &b.2, false);
            assert!(!frames.retire(&recipes, u32::MAX, &mut exact).unwrap());
            assert_eq!(exact.used(), b.1);
        }
        drop(frames);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
