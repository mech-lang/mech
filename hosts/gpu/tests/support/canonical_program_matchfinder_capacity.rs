//! The real binary-tree match finder's three node tables contain 262144 U16s.
use super::super::{artifacts, inputs, limits};
use super::{KERNEL, c_reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn deflate_node_helpers_handle_the_full_binary_tree_table_capacity() {
    const COUNT: usize = 2 * 32768 + 65536 + 2 * 65536;
    let golden = c_reference();
    assert_eq!(golden.nodes.len(), 512);
    assert_eq!(golden.rebased.len(), 512);
    for (call, named, expected) in [
        (
            "initial-nodes(count<u64>)",
            vec![("count", Data::U64(COUNT as u64))],
            vec![Data::U16(0); COUNT],
        ),
        (
            "rebase-nodes(nodes<[u16]>)",
            vec![(
                "nodes",
                Data::Matrix(
                    golden
                        .nodes
                        .iter()
                        .copied()
                        .cycle()
                        .take(COUNT)
                        .map(Data::U16)
                        .collect(),
                ),
            )],
            golden
                .rebased
                .iter()
                .copied()
                .cycle()
                .take(COUNT)
                .map(Data::U16)
                .collect(),
        ),
    ] {
        let source = format!("{KERNEL}\n{call}\n");
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            let args = inputs(&artifact, &named, &memory);
            // This qualification declares the real table extent explicitly;
            // the ordinary host profile keeps its 65536-element output cap.
            let mut policy = limits();
            policy.library.max_output_elements = Some(COUNT as u64);
            policy.execution.max_work = 10_000_000_000;
            policy.library.max_compute_work = Some(10_000_000_000);
            let report = jit
                .evaluate_into(&args, &mut output, policy, &memory)
                .unwrap_or_else(|error| panic!("{call} at {COUNT} entries: {error:?}"));
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.clone().into())
            );
            eprintln!("DEFLATE_TABLE entries={COUNT} operation={call} report={report:?}");
            drop(output);
            drop(args);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
