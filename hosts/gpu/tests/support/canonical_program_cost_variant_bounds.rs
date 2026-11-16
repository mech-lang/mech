//! Vector fallback, prefix, error and publication checks for the source experiment.
use super::super::{artifacts, inputs, limits};
use super::independent_tables;
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const HEADER: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-costs.mec"),
    "\n"
);

fn vector(count: usize) -> Data {
    Data::Matrix(
        (0..count)
            .map(|i| Data::U32(0xf000_0000 + i as u32))
            .collect(),
    )
}

#[test]
fn independent_cost_vectors_keep_prefix_suffix_errors_aliases_and_recovery() {
    for (lengths, entry) in [
        (
            false,
            "optimal-default-literals(values<[u32]>, cost<u64>, start<u64>)\n",
        ),
        (
            true,
            "optimal-default-lengths(values<[u32]>, cost<u64>, start<u64>, slots<[u8]>, extras<[u8]>)\n",
        ),
    ] {
        let pinned = format!("{HEADER}{entry}");
        let candidate = independent_tables::source(&pinned);
        let normal = if lengths { 259 } else { 256 };
        let begin = if lengths { 3 } else { 0 };
        let mut expected = Vec::new();
        for (source_index, source) in [pinned, candidate].iter().enumerate() {
            for (form, artifact) in artifacts(source).0.into_iter().enumerate() {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None; artifact.outputs().len()];
                let mut observed = Vec::new();
                let mut policy = limits();
                policy.execution.max_work = 1_000_000_000_000;
                policy.library.max_compute_work = Some(1_000_000_000_000);
                policy.library.max_comparison_work = Some(1_000_000_000_000);
                for (count, start, cost, empty_lookups) in [
                    (normal, begin, 109, false),
                    (normal + 1, begin, 109, false),
                    (normal, 255, 7, false),
                    (0, 259, 7, false),
                    (2, 259, 7, false),
                    (normal - 1, begin, 109, false),
                    (normal, begin, u64::MAX, false),
                    (normal, begin, 83, true),
                    (normal, begin, 83, false),
                ] {
                    let args = inputs(
                        &artifact,
                        &[
                            ("values", vector(count)),
                            ("cost", Data::U64(cost)),
                            ("start", Data::U64(start)),
                            (
                                "slots",
                                Data::Matrix(if empty_lookups {
                                    Box::new([])
                                } else {
                                    vec![Data::U8(1); 259].into()
                                }),
                            ),
                            (
                                "extras",
                                Data::Matrix(if empty_lookups {
                                    Box::new([])
                                } else {
                                    vec![Data::U8(0), Data::U8(1)].into()
                                }),
                            ),
                        ],
                        &memory,
                    );
                    let aliases = args.clone();
                    let prior = output.clone();
                    let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = jit.evaluate_into(&args, &mut output, policy, &memory);
                    let observation = match result {
                        Ok(_) => Ok(output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap()),
                        Err(error) => {
                            let message = format!("{error:?}");
                            assert!(
                                !message.contains("ResourceLimit")
                                    && !message.contains("WorkLimit"),
                                "{message}"
                            );
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                before
                            );
                            assert!(output.iter().zip(&prior).all(|(v, p)| {
                                v.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(p.as_ref().unwrap())
                            }));
                            Err(if message.contains("Arithmetic") {
                                "arithmetic"
                            } else {
                                "index"
                            })
                        }
                    };
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(v, a)| v.shares_frozen_storage(a))
                    );
                    observed.push(observation);
                }
                // The first three length entries are deliberately nonzero and
                // must survive both the direct constructor and suffix fallback.
                if lengths {
                    let Ok(Data::Matrix(first)) = &observed[0] else {
                        panic!()
                    };
                    assert_eq!(
                        &first[..3],
                        &[
                            Data::U32(0xf000_0000),
                            Data::U32(0xf000_0001),
                            Data::U32(0xf000_0002)
                        ]
                    );
                    assert_eq!(observed[6], Err("arithmetic"));
                }
                assert_eq!(observed[5], Err("index"));
                if source_index == 0 && form == 0 {
                    expected = observed;
                } else {
                    assert_eq!(observed, expected);
                }
                drop(output);
                assert_eq!(memory.used_bytes(), 0);
                assert_eq!(memory.retained_snapshot_nodes(), 0);
            }
        }
    }
}
