//! Recursive append carries an independently shaped read-only source buffer.
use super::{artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const PATTERN: &str = "+> sequence\ncopy(out<[u8]>, source<[u8]>, at<u64>, end<u64>) => <[u8]>\n  | (out, source, at, end) => ((at < end)? | true => copy(sequence/append(out, source[at + 1u64]), source, at + 1u64, end) | false => out).\n";
const STATEMENT: &str = "+> sequence\ncopy(out<[u8]>, source<[u8]>, at<u64>, end<u64>) => (result<[u8]>) :=\n  more := at < end\n  next := (more? | true => sequence/append(out, source[at + 1u64]) | false => out)\n  result := (more? | true => copy(next, source, at + 1u64, end) | false => out).\n";

#[test]
fn recursive_append_preserves_independent_buffer_axes_and_recovery() {
    for body in [PATTERN, STATEMENT] {
        let source = format!(
            "{body}copy(sequence/with-capacity(0u8, 320u64), source<[u8]>, 0u64, number<u64>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            policy.execution.max_work = 1_000_000_000_000;
            policy.library.max_compute_work = Some(1_000_000_000_000);
            for (bytes, number, reject) in [
                (vec![], 0, false),
                (vec![2, 5, 7], 3, false),
                (vec![11, 13, 17], 4, true),
                (vec![23, 29, 31], 2, false),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "source",
                            Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
                        ),
                        ("number", Data::U64(number)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let old = output.clone();
                let retained = memory.used_bytes();
                let nodes = memory.retained_snapshot_nodes();
                let resident = reference_result(&artifact, &args);
                let result = jit.evaluate_into(&args, &mut output, policy, &memory);
                if reject {
                    assert!(resident.is_err());
                    assert!(result.is_err());
                    assert!(old.iter().zip(&output).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(memory.used_bytes(), retained);
                    assert_eq!(memory.retained_snapshot_nodes(), nodes);
                } else {
                    result.unwrap();
                    let expected = Data::Matrix(
                        bytes
                            .into_iter()
                            .take(number as usize)
                            .map(Data::U8)
                            .collect(),
                    );
                    let value = output.last().unwrap().as_ref().unwrap();
                    assert_eq!(value.canonical_data_draft().unwrap(), expected);
                    assert_eq!(
                        resident
                            .unwrap()
                            .last()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected
                    );
                    let ValueData::Matrix(matrix) = value.data() else {
                        panic!()
                    };
                    assert_eq!(matrix.unsigned_buffer_capacity(), Some(320));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}
