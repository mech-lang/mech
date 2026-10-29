//! Opaque canonical siblings must not discard a private buffer's reservation.
use super::super::{artifacts, equal, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramDescriptor};

const RECORD: &str = "+> sequence\npack(nodes<[u64]>, costs<[u32]>) => (out<{nodes<[u64]> costs<{literal<[u32]>}>}>) :=\n  out := {nodes: nodes, costs: {literal: costs}}.\nreturned := pack(sequence/with-capacity(0u64, capacity<u64>), [7u32])\nsequence/append(returned.nodes, 17u64)\n";
const TUPLE: &str = "+> sequence\npack(nodes<[u64]>, costs<[u32]>) => (out<([u64], ([u32], bool))>) :=\n  out := (nodes, (costs, true)).\nreturned := pack(sequence/with-capacity(0u64, capacity<u64>), [7u32])\nsequence/append(returned.1, 17u64)\n";

fn reservation(source: &str) {
    for (width, real) in [(64, 305257), (8, 6633832)] {
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
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: unchanged image of the same checked build, confined here.
            let restored = unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &image,
                    &mech_stdlib::source_catalog(),
                )
            }
            .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), image);
            for capacity in [16, real] {
                let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                let args = inputs(&artifact, &[("capacity", Data::U64(capacity))], &memory);
                let mut output = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                let value = output.last().unwrap().as_ref().unwrap();
                assert_eq!(
                    value.canonical_data_draft().unwrap(),
                    Data::Matrix(
                        vec![if width == 64 {
                            Data::U64(17)
                        } else {
                            Data::U8(17)
                        }]
                        .into()
                    )
                );
                let ValueData::Matrix(matrix) = value.data() else {
                    panic!()
                };
                assert_eq!(
                    matrix.unsigned_buffer_capacity(),
                    Some(capacity as usize),
                    "width {width}, opaque sibling"
                );
                drop((output, args));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn opaque_record_siblings_preserve_real_path_and_output_reservations() {
    reservation(RECORD);
}

#[test]
fn opaque_tuple_siblings_preserve_real_path_and_output_reservations() {
    reservation(TUPLE);
}

#[test]
fn opaque_scalar_tuple_siblings_keep_live_bits_error_order_publication_and_retry() {
    let source = "+> sequence\npack(nodes<[u64]>, tag<u64>, allowed<bool>) => (out<{nodes<[u64]> meta<(u64, bool)>}>) :=\n  out := {nodes: nodes, meta: (tag, allowed)}.\nchecked(capacity<u64>, tag<u64>, allowed<bool>, divisor<u64>) => (out<[u64]>) :=\n  packet := pack(sequence/with-capacity(0u64, capacity), tag, allowed)\n  enabled! := packet.meta.2\n  word := packet.meta.1 + 12u64 / divisor\n  out := sequence/append(packet.nodes, word).\nchecked(capacity<u64>, tag<u64>, allowed<bool>, divisor<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (tag, allowed, divisor) in [
            (7, true, 3),
            (91, false, 0),
            (6, true, 0),
            (u64::MAX, true, 1),
            (19, true, 2),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("capacity", Data::U64(16)),
                    ("tag", Data::U64(tag)),
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
            if allowed && divisor != 0 && tag != u64::MAX {
                result.unwrap();
                equal(&artifact, &output, &resident.unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(vec![Data::U64(tag + 12 / divisor)].into())
                );
                let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!()
                };
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(16));
            } else {
                let error = format!("{:?}", result.unwrap_err());
                let ordinary = resident.unwrap_err();
                assert_eq!(error.contains("Integrity"), !allowed, "{error}");
                assert_eq!(
                    ordinary.contains("LexicalIntegrity"),
                    !allowed,
                    "{ordinary}"
                );
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            }
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
