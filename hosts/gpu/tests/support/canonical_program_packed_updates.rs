//! Real-size indexed updates preserve immutable inputs and failure recovery.
use super::{allocation_probe, artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn single_cell_matrix_updates_keep_aliases_publication_and_finite_denials() {
    let source = "<buffer> := <[u32]:2,3>\nreplace(xs<buffer>, at<u64>, incoming<[u32]>) => (out<buffer>) :=\n  ~next := xs\n  next[at] = incoming\n  out := next.\nreplace(input<buffer>, at<u64>, incoming<[u32]>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (at, incoming, denied_copy) in [
            (4, vec![Data::U32(17)], false),
            (6, vec![Data::U32(19), Data::U32(23)], false),
            (1, vec![Data::U32(31)], true),
            (6, vec![Data::U32(127)], false),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::Matrix((1..=6).map(Data::U32).collect())),
                    ("at", Data::U64(at)),
                    ("incoming", Data::Matrix(incoming.clone().into())),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut policy = limits();
            if denied_copy {
                policy.library.max_cloned_bytes = Some(0);
            }
            let result = jit.evaluate_into(&args, &mut output, policy, &memory);
            if incoming.len() != 1 || denied_copy {
                assert!(result.is_err());
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
                assert!(output.iter().zip(&prior).all(|(now, old)| {
                    now.as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
                let mut expected = (1..=6).map(Data::U32).collect::<Vec<_>>();
                expected[[0, 3, 1, 4, 2, 5][at as usize - 1]] = incoming[0].clone();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into())
                );
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(arg, alias)| arg.shares_frozen_storage(alias))
            );
            assert_eq!(
                args.iter()
                    .find_map(|arg| match arg.canonical_data_draft().unwrap() {
                        Data::Matrix(words) if words.len() == 6 => Some(words),
                        _ => None,
                    })
                    .unwrap(),
                (1..=6)
                    .map(Data::U32)
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn unsigned_updates_preserve_linear_indexing_of_rectangular_matrices() {
    for width in [8, 16, 32, 64] {
        let word = |value: u64| match width {
            8 => Data::U8(value as u8),
            16 => Data::U16(value as u16),
            32 => Data::U32(value as u32),
            64 => Data::U64(value),
            _ => unreachable!(),
        };
        let source = format!(
            "<buffer> := <[u{width}]:2,3>\nreplace(xs<buffer>, at<u64>, word<u{width}>) => (out<buffer>) :=\n  ~next := xs\n  next[at] = word\n  out := next.\nreplace(input<buffer>, at<u64>, word<u{width}>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            // Mech linear indices visit columns; canonical Matrix cells are
            // stored by rows. Pin each destination independently of the kernel.
            for (at, cell) in (1..=6).zip([0, 3, 1, 4, 2, 5]) {
                let original = Data::Matrix((1..=6).map(word).collect());
                let args = inputs(
                    &artifact,
                    &[
                        ("input", original),
                        ("at", Data::U64(at)),
                        ("word", word(127)),
                    ],
                    &memory,
                );
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
                let mut expected = (1..=6).map(word).collect::<Vec<_>>();
                expected[cell] = word(127);
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into())
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn real_u32_buffer_update_preserves_other_words_aliases_and_retries() {
    let source = "replace(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=\n  ~next := xs\n  next[at] = word\n  out := next.\nreplace(input<[u32]>, at<u64>, word<u32>)\n";
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut policy = limits();
        policy.library.max_output_elements = Some(1_500_513);
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        policy.execution.max_work = 10_000_000_000;
        let mut output = vec![None; artifact.outputs().len()];
        for (count, at) in [
            (3, 2),
            (1_500_513, 1),
            (1_500_513, 1_500_514),
            (1_500_513, 1_500_513),
            (0, 1),
            (3, 1),
        ] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "input",
                        Data::Matrix(vec![Data::U32(0x81234567); count].into()),
                    ),
                    ("at", Data::U64(at)),
                    ("word", Data::U32(0xf0123456)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let start = std::time::Instant::now();
            let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"packed-u32-update",
                "artifact":form,"count":count,"at":at,
                "elapsed_seconds":start.elapsed().as_secs_f64(),
                "allocation_requests":allocations,"requested_allocation_bytes":bytes,
                "result":format!("{result:?}")})
            );
            if at == 0 || at > count as u64 {
                assert!(result.is_err(), "{result:?}");
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                if count <= 3 {
                    equal(&artifact, &output, &reference(&artifact, &args));
                }
                let mech_core::ValueData::Matrix(matrix) =
                    output.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!()
                };
                let mech_core::snapshot::SequenceView::U32(words) = matrix.elements() else {
                    panic!()
                };
                assert_eq!(words.len(), count);
                assert!(words.iter().enumerate().all(|(i, word)| *word
                    == if i as u64 + 1 == at {
                        0xf0123456
                    } else {
                        0x81234567
                    }));
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
            for value in &args {
                if let mech_core::ValueData::Matrix(matrix) = value.data() {
                    let mech_core::snapshot::SequenceView::U32(words) = matrix.elements() else {
                        panic!()
                    };
                    assert_eq!(words.len(), count);
                    assert!(words.iter().all(|word| *word == 0x81234567));
                }
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
