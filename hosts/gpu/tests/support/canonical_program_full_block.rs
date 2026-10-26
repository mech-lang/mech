//! Full first-block collection keeps the original physical tree/cache extents.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits};
use serde_json::Value as Json;
use std::time::Instant;

const SOURCE: &str = concat!(
    include_str!("../fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-binary-tree.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-collector.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-collector.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-heuristics.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-loop.mec"),
    "\n",
);

#[test]
fn real_buffer_first_block_collects_all_eight_literals() {
    collect_cases(&["eight-literals"], 10_000_000_000, 4916);
}

// These diagnostic runs deliberately have a separate finite work allowance.
// They establish completion cost without changing any component qualification.
#[test]
#[ignore = "explicit real-buffer completion diagnostic with a larger work allowance"]
fn real_buffer_complete_eight_byte_blocks_diagnostic() {
    collect_cases(
        &["eight-literals", "eight-overlap"],
        1_000_000_000_000,
        16_384,
    );
}

#[test]
#[ignore = "explicit 320-position completion diagnostic with a larger work allowance"]
fn real_buffer_complete_320_byte_block_diagnostic() {
    collect_cases(&["320-overlap"], 1_000_000_000_000, 16_384);
}

fn assert_reference(actual: &Data, case: &Json) {
    let Data::Record(fields) = actual else {
        panic!("complete block state")
    };
    let scalars = case["scalars"].as_object().unwrap();
    let shapes = case["shapes"].as_object().unwrap();
    assert_eq!(fields.len(), scalars.len() + shapes.len());
    let field = |name: &str| {
        &fields
            .iter()
            .find(|field| field.name == name)
            .unwrap()
            .value
    };
    for (name, expected) in scalars {
        assert_eq!(
            field(name),
            &Data::U64(expected.as_u64().unwrap()),
            "{name}"
        );
    }
    for (name, length) in shapes {
        let Data::Matrix(words) = field(name) else {
            panic!("{name} must retain its complete buffer")
        };
        assert_eq!(words.len() as u64, length.as_u64().unwrap(), "{name}");
        // The reference producer scans every cell and lists all nonzero words.
        // Compare every cell, including the original zero-filled remainder.
        let mut nonzero = case[name].as_array().unwrap().iter().peekable();
        for (index, word) in words.iter().enumerate() {
            let expected = if nonzero
                .peek()
                .is_some_and(|entry| entry[0].as_u64().unwrap() == index as u64)
            {
                nonzero.next().unwrap()[1].as_u64().unwrap()
            } else {
                0
            };
            let actual = match (name.as_str(), word) {
                ("tables", Data::U16(value)) => u64::from(*value),
                (_, Data::U32(value)) if name != "tables" => u64::from(*value),
                _ => panic!("{name}[{index}] has the wrong word kind"),
            };
            assert_eq!(actual, expected, "{name}[{index}]");
        }
        assert!(nonzero.next().is_none(), "reference entry beyond {name}");
    }
}

fn collect_cases(names: &[&str], work_limit: u64, max_blocks: u64) {
    let reference: Json = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-block-reference.json"
    ))
    .unwrap();
    let source = format!("{SOURCE}\noptimal-first-block(input<[u8]>)\n");
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let compile_start = Instant::now();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeCompileLimits {
                max_nodes: max_blocks,
                ..NativeCompileLimits::default()
            },
        )
        .unwrap();
        let compile_seconds = compile_start.elapsed().as_secs_f64();
        for name in names {
            let case = reference["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["name"] == *name)
                .unwrap();
            let input = case["input"].as_array().unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[(
                    "input",
                    Data::Matrix(
                        input
                            .iter()
                            .map(|byte| Data::U8(u8::try_from(byte.as_u64().unwrap()).unwrap()))
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                )],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            policy.library.max_output_elements = Some(1_500_513);
            policy.library.max_compute_work = Some(work_limit);
            policy.library.max_comparison_work = Some(work_limit);
            policy.execution.max_work = work_limit;
            let before = memory.used_bytes();
            let start = Instant::now();
            let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"first-complete-block","case":name,"artifact":form,"input_bytes":input.len(),"work_limit":work_limit,"compile_block_limit":max_blocks,"lowered_blocks":jit.descriptor().plan().blocks().len(),"compile_seconds":compile_seconds,"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":allocations,"requested_allocation_bytes":bytes,"retained_before":before,"retained_after":memory.used_bytes(),"result":format!("{result:?}")})
            );
            if result.is_err() {
                assert!(output.iter().all(Option::is_none));
                assert_eq!(memory.used_bytes(), before);
            }
            result.unwrap();
            let actual = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            assert_reference(&actual, case);
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
            drop((output, args, aliases));
            assert_eq!(memory.used_bytes(), 0);
            println!(
                "reference agreement: {name} {form}, all scalar fields and all six complete buffers"
            );
        }
    }
}
