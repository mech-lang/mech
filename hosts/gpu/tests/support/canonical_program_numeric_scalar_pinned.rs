//! Unchanged signed/unsigned scalar-pair functions used by the compressor.
use super::*;

#[test]
fn pinned_scalar_pair_cost_functions_select_and_preserve_exact_values() {
    let unsigned = concat!(
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-header.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-block-choice.mec"),
        "\n"
    );
    let signed = concat!(
        include_str!("../fixtures/native/deflate-matchfinder.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-block-heuristics.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-optimal-costs.mec"),
        "\n"
    );
    // Focused resident agreement uses the original terminal boundaries. Full
    // length/offset/default traversals remain covered by unchanged oracle
    // components and the protected complete compression; its finite resident
    // helper cannot fund the full signed traversal. No allowance is raised.
    let cases = [
        (
            unsigned,
            "block-cost-literals(data<[u32]>, lens<[u8]>, 0u64, 8u64, 8u64, 11u64, 7u64)",
            59,
            199,
            false,
        ),
        (
            unsigned,
            "block-cost-lengths(data<[u32]>, lens<[u8]>, lens, extras<[u8]>, 28u64, 11u64, 7u64)",
            23,
            19,
            false,
        ),
        (
            unsigned,
            "block-cost-offsets(data<[u32]>, lens<[u8]>, extras<[u8]>, 29u64, 11u64, 7u64)",
            23,
            28,
            false,
        ),
        (
            signed,
            "optimal-default-frequencies(data<[u32]>, 258u64, 5i64, 100000i64)",
            8,
            99226,
            true,
        ),
    ];
    for (header, call, a, b, is_signed) in cases {
        let zero = if is_signed { "0i64" } else { "0u64" };
        let source = format!("{header}(a,b) := {call}\n(a + {zero}, b + {zero})\n");
        selected(&source);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix(vec![Data::U32(3); 288].into())),
                    ("lens", Data::Matrix(vec![Data::U8(2); 288].into())),
                    ("extras", Data::Matrix(vec![Data::U8(2); 32].into())),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            let expected = if is_signed {
                vec![Data::I64(a), Data::I64(b)]
            } else {
                vec![Data::U64(a as u64), Data::U64(b as u64)]
            };
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(expected.into())
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((args, aliases, out));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
