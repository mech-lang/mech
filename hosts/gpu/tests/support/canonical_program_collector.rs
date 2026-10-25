//! A real-size, source-initialized collector prefix, executed as one program.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
use std::time::Instant;

const SOURCE: &str = concat!(
    include_str!("../fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-binary-tree.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-collector.mec"),
    "\noptimal-collect-prefix(input<[u8]>, 6u64)\n"
);

#[test]
fn real_buffer_collector_prefix_carries_consecutive_search_and_skip_updates() {
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(SOURCE).0) {
        let compile_start = Instant::now();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let compile_seconds = compile_start.elapsed().as_secs_f64();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("input", Data::Matrix(vec![Data::U8(b'a'); 320].into()))],
            &memory,
        );
        let aliases = args.clone();
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        policy.library.max_output_elements = Some(1_500_513);
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        policy.execution.max_work = 10_000_000_000;
        let retained_before = memory.used_bytes();
        let start = Instant::now();
        let (result, allocations, requested_bytes) = allocation_probe::measured_with_bytes(|| {
            jit.evaluate_into(&args, &mut output, policy, &memory)
        });
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "{}",
            serde_json::json!({
                "witness":"real-buffer-collector-prefix", "artifact":form,
                "input_bytes":320, "requested_positions":6,
                "completed_prefix_positions":if result.is_ok() {6} else {0},
                "table_u16_entries":262_144, "cache_u32_words":1_500_513,
                "depth":300, "nice":258, "compile_seconds":compile_seconds,
                "elapsed_seconds":elapsed, "allocation_requests":allocations,
                "requested_allocation_bytes":requested_bytes,
                "retained_before":retained_before,"retained_after":memory.used_bytes(),
                "result":format!("{result:?}"),"success":result.is_ok(),
            })
        );
        assert!(
            args.iter()
                .zip(&aliases)
                .all(|(value, alias)| value.shares_frozen_storage(alias))
        );
        if result.is_err() {
            assert!(output.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), retained_before);
        }
        result.unwrap();
        let Data::Record(fields) = output
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_data_draft()
            .unwrap()
        else {
            panic!("collector must return its complete state")
        };
        let field = |name: &str| {
            &fields
                .iter()
                .find(|field| field.name == name)
                .unwrap()
                .value
        };
        assert_eq!(field("next-position"), &Data::U64(6));
        assert!(matches!(field("tables"), Data::Matrix(values) if values.len() == 262_144));
        assert!(matches!(field("cache"), Data::Matrix(values) if values.len() == 1_500_513));
        drop((output, args, aliases));
        assert_eq!(memory.used_bytes(), 0);
    }
}
