//! A private child is packed, returned, wrapped twice and projected repeatedly.
//! This is the costs/saved transport required by the compression controller.
use super::{artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    lower_native_program_artifact,
};
use mech_gpu::native_artifact::JitProgramArtifact;

const NESTED: &str = "+> sequence\n<costs> := <{literal<[u32]> length<[u32]> offset<[u32]>}>\n<selected> := <{nodes<[u64]> costs<costs> marker<u64>}>\n<planned> := <{selected<selected> saved<costs>}>\nwrap(nodes<[u64]>, values<[u32]>, marker<u64>) => (out<planned>) :=\n  costs := {literal: values, length: values, offset: values}\n  selected := {nodes: nodes, costs: costs, marker: marker}\n  out := {selected: selected, saved: costs}.\nchecked(capacity<u64>, marker<u64>, allowed<bool>, divisor<u64>) => (out<[u64]>) :=\n  returned := wrap(sequence/with-capacity(0u64, capacity), [7u32], marker)\n  current! := returned.selected.marker == marker\n  same! := returned.selected.costs.literal[1u64] == returned.saved.length[1u64]\n  other! := returned.saved.offset[1u64] == 7u32\n  enabled! := allowed\n  word := marker + 12u64 / divisor\n  out := sequence/append(returned.selected.nodes, word).\nchecked(capacity<u64>, marker<u64>, allowed<bool>, divisor<u64>)\n";

#[test]
fn private_nested_pack_return_and_projection_select_the_complete_component() {
    for artifact in artifacts(NESTED).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let frames = plan.facts().buffer_frames();
        let mut packs = 0;
        let mut projections = 0;
        for (site, block) in plan.blocks().iter().enumerate() {
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                continue;
            };
            if matches!(template.primitive(), Primitive::CompositePack(_))
                && matches!(template.output().body(), mech_core::SchemaBody::Record(_))
            {
                assert!(frames.slot(*output).is_some(), "pack site {site}");
                assert!(frames.action(site).is_some());
                packs += 1;
            } else if template.primitive() == Primitive::RecordFieldAccess {
                assert!(frames.slot(operands[0]).is_some(), "read site {site}");
                assert!(frames.action(site).is_some());
                if matches!(template.output().body(), mech_core::SchemaBody::Record(_)) {
                    assert!(frames.slot(*output).is_some(), "child site {site}");
                    projections += 1;
                }
            }
        }
        assert_eq!(packs, 3);
        assert!(projections >= 5);
    }
}

#[test]
fn nested_current_fields_preserve_errors_publication_aliases_and_retry() {
    for source in [
        NESTED.to_owned(),
        NESTED.replace(
            "  enabled! := allowed\n  word := marker + 12u64 / divisor",
            "  word := marker + 12u64 / divisor\n  enabled! := allowed",
        ),
    ] {
        let assertion_first = source == NESTED;
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (capacity, marker, allowed, divisor) in [
                (16, 7u64, true, 3u64),
                (305257, 91, true, 2),
                (16, 13, false, 0),
                (305257, 37, true, 0),
                (16, u64::MAX, true, 1),
                (305257, 19, true, 4),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("capacity", Data::U64(capacity)),
                        ("marker", Data::U64(marker)),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && marker != u64::MAX {
                    result.unwrap();
                    super::equal(&artifact, &output, &resident.unwrap());
                    let value = output.last().unwrap().as_ref().unwrap();
                    assert_eq!(
                        value.canonical_data_draft().unwrap(),
                        Data::Matrix(vec![Data::U64(marker + 12 / divisor)].into())
                    );
                    let ValueData::Matrix(matrix) = value.data() else {
                        panic!()
                    };
                    assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                } else {
                    let native = format!("{:?}", result.unwrap_err());
                    let resident = resident.unwrap_err();
                    let assertion = !allowed && assertion_first;
                    assert_eq!(native.contains("Integrity"), assertion, "{native}");
                    assert_eq!(resident.contains("LexicalIntegrity"), assertion, "{resident}");
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert!(output.iter().zip(&prior).all(|(v, p)| {
                        v.as_ref()
                            .unwrap()
                            .shares_frozen_storage(p.as_ref().unwrap())
                    }));
                }
                assert!(args.iter().zip(&aliases).all(|(v, a)| v.shares_frozen_storage(a)));
            }
            drop(output);
            assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
        }
    }
}

#[test]
fn nested_private_path_and_output_buffers_keep_the_real_reservation() {
    let source = "+> sequence\nwrap(nodes<[u64]>, words<[u32]>) => (out<{selected<{nodes<[u64]> costs<{literal<[u32]>}>}>}>) :=\n  costs := {literal: words}\n  selected := {nodes: nodes, costs: costs}\n  out := {selected: selected}.\nreturned := wrap(sequence/with-capacity(0u64, capacity<u64>), [7u32])\nsequence/append(returned.selected.nodes, 17u64)\n";
    for (width, capacity) in [(64, 305257), (8, 6633832)] {
        let source = if width == 64 {
            source.to_owned()
        } else {
            source
                .replace("[u64]", "[u8]")
                .replace("0u64, capacity", "0u8, capacity")
                .replace("17u64", "17u8")
        };
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let args = inputs(&artifact, &[("capacity", Data::U64(capacity))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            policy.library.max_cloned_bytes = Some(0);
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .unwrap();
            let value = output.last().unwrap().as_ref().unwrap();
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                Data::Matrix(vec![if width == 64 { Data::U64(17) } else { Data::U8(17) }].into())
            );
            let ValueData::Matrix(matrix) = value.data() else { panic!() };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
            drop((args, output));
            assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
        }
    }
}
