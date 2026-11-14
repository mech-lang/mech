//! The actual skip collector retains its real tree/cache and terminal owners.
use super::*;
use mech_core::{ValueData, snapshot::SequenceView};

const COMPONENTS: &str = concat!(
    include_str!("../../../../../tests/fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../../../../../tests/fixtures/native/deflate-binary-tree.mec"),
    "\n",
    include_str!("../../../../../tests/fixtures/native/deflate-collector.mec"),
    "\n",
    include_str!("../../../../../tests/fixtures/native/deflate-block-collector.mec"),
    "\n",
    include_str!("../../../../../tests/fixtures/native/deflate-block-heuristics.mec"),
    "\n",
    include_str!("../../../../../tests/fixtures/native/deflate-block-loop.mec"),
    "\n",
);

fn tree(data: &ValueData) -> &[u16] {
    let ValueData::Matrix(matrix) = data else {
        panic!("tree matrix")
    };
    let SequenceView::U16(words) = matrix.elements() else {
        panic!("packed U16 tree")
    };
    words
}
fn cache(data: &ValueData) -> &[u32] {
    let ValueData::Matrix(matrix) = data else {
        panic!("cache matrix")
    };
    let SequenceView::U32(words) = matrix.elements() else {
        panic!("packed U32 cache")
    };
    words
}

#[test]
fn numeric_interval_real_skip_forwards_current_buffers_and_keeps_cold_rebase() {
    let source = format!(
        "{COMPONENTS}\ninspect-skip(tables<[u16]>, cache<[u32]>, input<[u8]>, skip<u64>, slide<u64>) => (out<([u16],[u32],u64,u64,u64)>) :=\n  next := optimal-skip-run(tables, cache, 0u64, 0u64, input, 0u64, 0u64, slide, 258u64, 258u64, skip, 0u64)\n  out := (next.tables, next.cache, next.next-position, next.cache-ptr, next.skip).\nanswer := inspect-skip(tables<[u16]>, cache<[u32]>, input<[u8]>, skip<u64>, slide<u64>)\nanswer\n"
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let root = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| region.tail.parent().is_none())
            .expect("the actual skip root has a native interval");
        let proof = root.numeric.as_ref().unwrap();
        assert!(proof.phase.as_ref().unwrap().checked_children);
        assert!(proof.exits.iter().any(|exit| matches!(
            exit.kind,
            crate::native_artifact::program::prepare::numeric_plan::ExitKind::Unsupported
        )));
        assert!(proof.result.frame().is_some());
        assert!(proof.arrays.len() >= 2);
        let memory = mode.memory_budget();
        for (skip, slide) in [(0, u64::MAX), (2, u64::MAX), (2, 0)] {
            let args = inputs(
                &artifact,
                &[
                    ("tables", Data::Matrix(vec![Data::U16(0); 262_144].into())),
                    ("cache", Data::Matrix(vec![Data::U32(0); 1_500_513].into())),
                    ("input", Data::Matrix(b"abcdefgh".map(Data::U8).into())),
                    ("skip", Data::U64(skip)),
                    ("slide", Data::U64(slide)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let named = |name| {
                let at = artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == name
                    })
                    .unwrap();
                &args[at]
            };
            let old_tree = tree(named("tables").data());
            let old_cache = cache(named("cache").data());
            for restored in [false, true] {
                let mut out = vec![None; artifact.outputs().len()];
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut out, mode.execution(), &memory)
                };
                result.unwrap();
                let answer = out.last().unwrap().as_ref().unwrap();
                let fields = answer.tuple_view().unwrap().elements();
                let next_tree = tree(&fields[0]);
                let next_cache = cache(&fields[1]);
                assert_eq!(next_tree.len(), 262_144);
                assert_eq!(next_cache.len(), 1_500_513);
                for (field, expected) in fields[2..].iter().zip([skip, skip, 0]) {
                    assert!(matches!(field, ValueData::U64(value) if *value == expected));
                }
                if skip == 0 {
                    // Inspect the internal native return boundary. The final
                    // public tuple has the ordinary canonical representation.
                    for address in [old_tree.as_ptr() as usize, old_cache.as_ptr() as usize] {
                        assert!(
                            counts.terminal_backings.contains(&(address, address)),
                            "{counts:?}"
                        );
                    }
                    assert!(next_tree.iter().all(|&word| word == 0));
                    assert!(next_cache.iter().all(|&word| word == 0));
                    assert_eq!(counts.phases, 1);
                    assert!(counts.first_change_backings.is_empty());
                    assert!(counts.cold_private_arrays.is_empty());
                } else {
                    assert_ne!(next_tree.as_ptr(), old_tree.as_ptr());
                    assert_ne!(next_cache.as_ptr(), old_cache.as_ptr());
                    assert!(next_tree.iter().any(|&word| word != 0));
                    assert_eq!(&next_cache[..2], &[97 << 16, 98 << 16]);
                    assert!(next_cache[2..].iter().all(|&word| word == 0));
                    if slide == 0 {
                        assert!(!counts.cold_private_arrays.is_empty());
                        assert!(!counts.prepaid_source_visits.is_empty(), "{counts:?}");
                        assert!(
                            counts
                                .prepaid_segment_charges
                                .iter()
                                .all(|(actual, expected)| actual == expected),
                            "paid source work must be credited while the entire current entry cost remains charged: {counts:?}"
                        );
                    }
                }
                assert!(old_tree.iter().all(|&word| word == 0));
                assert!(old_cache.iter().all(|&word| word == 0));
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop((args, aliases));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
