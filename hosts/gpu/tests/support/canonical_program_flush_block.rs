//! First connected block witness: header, payload, EOB and all retained fields.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-header.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-writer-tables.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-path-writer.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-sequence-writer.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-choice.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-header-writer.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-stored-writer.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-flush-block.mec"),
    "\n",
);
fn matrix(v: &serde_json::Value, width: u32) -> Data {
    Data::Matrix(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| match width {
                8 => Data::U8(n.as_u64().unwrap() as u8),
                32 => Data::U32(n.as_u64().unwrap() as u32),
                _ => panic!(),
            })
            .collect(),
    )
}
#[test]
fn connected_fixed_block_sequence_and_path_agree() {
    qualify_blocks(0);
}
#[test]
fn connected_consecutive_blocks_carry_output_and_pending_bits() {
    qualify_blocks(1);
}
#[test]
fn connected_dynamic_block_sequence_and_path_agree() {
    qualify_blocks(2);
}
fn qualify_blocks(case: u8) {
    let consecutive = case == 1;
    let dynamic = case == 2;
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-flush-dynamic-reference.json"
    ))
    .unwrap();
    let dynamic_fields = &reference["fields"];
    let tables: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-writer-tables-reference.json"
    ))
    .unwrap();
    let headers: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-header-reference.json"
    ))
    .unwrap();
    let huffman: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-huffman-level12-reference.json"
    ))
    .unwrap();
    let paths: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    let find = |name: &str| {
        tables["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
    };
    let fixed = find("codes-fixed");
    let offsets = find("offsets-fixed");
    let source = format!(
        "{COMPONENTS}seqs := [{{litrunlen-and-length: 1u32, offset: 0u16, offset-slot: 0u16}}]\nitems := [142940806578176u64]\nblock-flush(sequence/with-capacity(0u8, 6633832u64), 0u64, 0u64, [65u8], 0u64, 1u64, seqs, freqs<[u32]>, offset-freqs<[u32]>, lens<[u8]>, words<[u32]>, offset-lens<[u8]>, offset-words<[u32]>, lens, words, offset-lens, offset-words, items, use-items<u64>, 1u64, permutation<[u8]>, reversed<[u8]>, extra-precode<[u8]>, length-slots<[u8]>, length-bases<[u32]>, length-extra<[u8]>, offset-slots<[u8]>, offset-bases<[u32]>, offset-extra<[u8]>)\n"
    );
    let source = if consecutive {
        let source = source
            .replace(
                "\nblock-flush(sequence/with-capacity",
                "\nfirst := block-flush(sequence/with-capacity",
            )
            .replace("[65u8], 0u64", "[65u8 65u8], 0u64")
            .replace(
                "items, use-items<u64>, 1u64,",
                "items, use-items<u64>, 0u64,",
            );
        format!(
            "{source}block-flush(first.bytes, first.bitbuf, first.bitcount, [65u8 65u8], 1u64, 1u64, first.seqs, freqs, offset-freqs, first.litlen-lens, first.litlen-codewords, first.offset-lens, first.offset-codewords, first.static-litlen-lens, first.static-litlen-codewords, first.static-offset-lens, first.static-offset-codewords, first.items, use-items, 1u64, permutation, reversed, extra-precode, length-slots, length-bases, length-extra, offset-slots, offset-bases, offset-extra)\n"
        )
    } else {
        source
    };
    let source = if dynamic {
        source.replace("litrunlen-and-length: 1u32", "litrunlen-and-length: 200u32")
            .replace("items := [142940806578176u64]", "items := [142940806578176u64 | i <- 0u64..=200u64, i < 200u64]")
            .replace("[65u8], 0u64, 1u64", "[65u8 | i <- 0u64..=200u64, i < 200u64], 0u64, 200u64")
            .replace("lens<[u8]>, words<[u32]>, offset-lens<[u8]>, offset-words<[u32]>, lens, words, offset-lens, offset-words", "dynamic-lens<[u8]>, dynamic-words<[u32]>, dynamic-offset-lens<[u8]>, dynamic-offset-words<[u32]>, lens<[u8]>, words<[u32]>, offset-lens<[u8]>, offset-words<[u32]>")
    } else {
        source
    };
    let dynamic_bytes = dynamic_fields["out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    let expected_bytes: &[u8] = if dynamic {
        &dynamic_bytes
    } else if consecutive {
        &[114, 4, 204, 17]
    } else {
        &[115, 4]
    };
    let expected_count = if dynamic {
        dynamic_fields["bitcount"][0].as_u64().unwrap()
    } else if consecutive {
        4
    } else {
        2
    };
    let expected_buf = if dynamic {
        dynamic_fields["bitbuf"][0].as_u64().unwrap()
    } else {
        0
    };
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mech_gpu::native_artifact::NativeCompileLimits {
                max_nodes: 16_384,
                ..Default::default()
            },
        )
        .unwrap();
        for use_items in [0, 1] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut freqs = vec![Data::U32(0); 288];
            freqs[65] = Data::U32(if dynamic { 200 } else { 1 });
            freqs[256] = Data::U32(1);
            let args = inputs(
                &artifact,
                &[
                    ("freqs", Data::Matrix(freqs.into())),
                    ("offset-freqs", Data::Matrix(vec![Data::U32(0); 32].into())),
                    ("lens", matrix(&fixed["lens"], 8)),
                    ("words", matrix(&fixed["codewords"], 32)),
                    ("offset-lens", matrix(&offsets["lens"], 8)),
                    ("offset-words", matrix(&offsets["codewords"], 32)),
                    ("use-items", Data::U64(use_items)),
                    ("dynamic-lens", matrix(&dynamic_fields["litlen_lens"], 8)),
                    (
                        "dynamic-words",
                        matrix(&dynamic_fields["litlen_codewords"], 32),
                    ),
                    (
                        "dynamic-offset-lens",
                        matrix(&dynamic_fields["offset_lens"], 8),
                    ),
                    (
                        "dynamic-offset-words",
                        matrix(&dynamic_fields["offset_codewords"], 32),
                    ),
                    (
                        "permutation",
                        matrix(&headers["tables"]["precode_lens_permutation"], 8),
                    ),
                    ("reversed", matrix(&huffman["reversal_table"], 8)),
                    (
                        "extra-precode",
                        matrix(&headers["tables"]["extra_precode_bits"], 8),
                    ),
                    ("length-slots", matrix(&tables["length_slot_tab"], 8)),
                    ("length-bases", matrix(&tables["length_slot_base"], 32)),
                    ("length-extra", matrix(&tables["extra_length_bits"], 8)),
                    ("offset-slots", matrix(&paths["offset_slots"], 8)),
                    ("offset-bases", matrix(&tables["offset_slot_base"], 32)),
                    ("offset-extra", matrix(&tables["extra_offset_bits"], 8)),
                ],
                &memory,
            );
            let mut policy = limits();
            if consecutive {
                // Same transport profile as the maintained path/sequence writers.
                // Two connected flush calls require 1,258,344 bytes; compiler
                // defaults and the original single-block witness stay unchanged.
                policy.execution.max_transport_bytes = 2 * 1024 * 1024;
            }
            policy.execution.max_work = 1_000_000_000_000;
            policy.library.max_compute_work = Some(1_000_000_000_000);
            policy.library.max_comparison_work = Some(1_000_000_000_000);
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, allocated_bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "connected block case={case} path={use_items} seconds={} allocations={requests} allocated_bytes={allocated_bytes} result={result:?}",
                start.elapsed().as_secs_f64()
            );
            result.unwrap();
            let Data::Record(fields) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!()
            };
            let field = |name: &str| &fields.iter().find(|f| f.name == name).unwrap().value;
            assert_eq!(
                field("bytes"),
                &Data::Matrix(expected_bytes.iter().copied().map(Data::U8).collect())
            );
            assert_eq!(field("bitbuf"), &Data::U64(expected_buf));
            assert_eq!(field("bitcount"), &Data::U64(expected_count));
            assert_eq!(
                field("litlen-lens"),
                &matrix(
                    if dynamic {
                        &dynamic_fields["litlen_lens"]
                    } else {
                        &fixed["lens"]
                    },
                    8
                )
            );
            let mut complete = expected_bytes.to_vec();
            complete.push(expected_buf as u8);
            assert_eq!(
                miniz_oxide::inflate::decompress_to_vec(&complete).unwrap(),
                if dynamic {
                    vec![65; 200]
                } else if consecutive {
                    b"AA".to_vec()
                } else {
                    b"A".to_vec()
                }
            );
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
