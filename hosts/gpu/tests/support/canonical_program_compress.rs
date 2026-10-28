//! Complete level-12 compression, using only Mech for compression.
use super::{allocation_probe, inputs};
use mech_core::ValueDataDraft as Data;
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[path = "canonical_program_control_usage.rs"]
mod control_usage;

const COMPONENTS: &str = concat!(
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
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-tally.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-costs.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-header.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimize.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-literal-choice.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-static-codes.mec"),
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
    include_str!("../fixtures/native/deflate-select-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-plan-block.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-level12-tables.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-compress-controller.mec"),
    "\n",
);

fn matrix(value: &[u8]) -> Data {
    Data::Matrix(value.iter().copied().map(Data::U8).collect())
}

#[derive(serde::Deserialize)]
struct CompleteCase {
    name: String,
    input: Vec<u8>,
    prefix: Vec<u8>,
    expected: Vec<u8>,
    #[serde(default)]
    first_block_kind: Option<u8>,
}

fn complete_artifacts(source: &str) -> [ProgramArtifact; 2] {
    let start = std::time::Instant::now();
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
        .unwrap();
    println!(
        "{}",
        serde_json::json!({"phase":"complete-source-checked","seconds":start.elapsed().as_secs_f64(),"source_bytes":source.len()})
    );
    control_usage::report(product.program());
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    if let Some(path) = std::env::var_os("MECH_DEFLATE_ARTIFACT") {
        let path = std::path::PathBuf::from(path);
        std::fs::write(&path, &bytes).unwrap();
        std::fs::write(path.with_extension("mec"), source).unwrap();
        println!("saved validated compressor artifact: {}", path.display());
    }
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    println!(
        "{}",
        serde_json::json!({"phase":"complete-artifacts-accepted","seconds":start.elapsed().as_secs_f64(),"bytecode_bytes":bytes.len()})
    );
    [original, decoded]
}

#[test]
fn complete_level12_compression_matches_pinned_bytes() {
    with_compiler_stack(|| {
        let source = complete_source();
        qualify_complete(
            ["original", "decoded"]
                .into_iter()
                .zip(complete_artifacts(&source)),
            false,
        );
    });
}

#[test]
#[ignore = "long-running qualification with full-size buffers and boundary inputs"]
fn complete_level12_compression_crosses_block_and_window_boundaries() {
    with_compiler_stack(|| {
        let source = complete_source();
        let cases = [
            CompleteCase {
                name: "16384-distribution-change".into(),
                input: include_bytes!(
                    "../fixtures/native/deflate-boundary-16384-distribution-change.input"
                )
                .to_vec(),
                prefix: vec![],
                expected: include_bytes!(
                    "../fixtures/native/deflate-boundary-16384-distribution-change.deflate"
                )
                .to_vec(),
                first_block_kind: Some(2),
            },
            CompleteCase {
                name: "33027-periodic-window".into(),
                input: include_bytes!(
                    "../fixtures/native/deflate-boundary-33027-periodic-window.input"
                )
                .to_vec(),
                prefix: vec![],
                expected: include_bytes!(
                    "../fixtures/native/deflate-boundary-33027-periodic-window.deflate"
                )
                .to_vec(),
                first_block_kind: Some(2),
            },
        ];
        qualify_cases(
            ["original", "decoded"]
                .into_iter()
                .zip(complete_artifacts(&source)),
            &cases,
        );
    });
}

// The source frontend traverses the entire assembled compressor. Reserve the
// same host-thread stack used by its release qualification instead of relying
// on the test runner's smaller platform default. This is outside evaluation.
fn with_compiler_stack(run: fn()) {
    std::thread::Builder::new()
        .name("complete-compressor".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn complete_source() -> String {
    format!(
        "{COMPONENTS}buffer := sequence/with-capacity(0u8, 6633832u64)\nprefixed := sequence/append-sublist(buffer, prefix<[u8]>, 0u64, sequence/length(prefix))\ndeflate-compress-into(input<[u8]>, prefixed)\n"
    )
}

// An explicitly requested local investigation reuses normal validated bytecode,
// avoiding source compilation. The maintained test above always compiles source.
#[test]
#[ignore = "requires an explicitly supplied complete compressor artifact"]
fn complete_saved_artifact_reality_check() {
    with_compiler_stack(|| {
        let path = std::path::PathBuf::from(std::env::var_os("MECH_DEFLATE_ARTIFACT").unwrap());
        assert_eq!(
            std::fs::read_to_string(path.with_extension("mec")).unwrap(),
            complete_source()
        );
        let artifact =
            mech_engine::decode_program_artifact_bytecode_v1(&std::fs::read(path).unwrap())
                .unwrap();
        qualify_complete([("saved-decoded", artifact)], true);
    });
}

fn qualify_complete(
    artifacts: impl IntoIterator<Item = (&'static str, ProgramArtifact)>,
    allow_full: bool,
) {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-compress-reference.json"
    ))
    .unwrap();
    let mut cases: Vec<CompleteCase> = serde_json::from_value(fixture["cases"].clone()).unwrap();
    if allow_full {
        if let Some(input) = std::env::var_os("MECH_DEFLATE_FULL_INPUT") {
            let expected = std::env::var_os("MECH_DEFLATE_FULL_REFERENCE").unwrap();
            let input = std::path::PathBuf::from(input);
            cases = vec![CompleteCase {
                name: input.file_name().unwrap().to_string_lossy().into_owned(),
                input: std::fs::read(&input).unwrap(),
                prefix: vec![],
                expected: std::fs::read(expected).unwrap(),
                first_block_kind: None,
            }];
        }
    }
    qualify_cases(artifacts, &cases);
}

fn qualify_cases(
    artifacts: impl IntoIterator<Item = (&'static str, ProgramArtifact)>,
    cases: &[CompleteCase],
) {
    let mode = NativeProgramLimitMode::NoLimits;
    for (form, artifact) in artifacts {
        let compile_start = std::time::Instant::now();
        println!(
            "{}",
            serde_json::json!({"phase":"complete-native-start","artifact":form})
        );
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let plan = jit.descriptor().plan();
        let cells = plan.values().len()
            + 2 * plan.collections().len()
            + plan.iterations().len()
            + 2 * plan
                .collections()
                .iter()
                .filter(|c| c.string_span_projection().is_some())
                .count()
            + 2 * plan
                .collections()
                .iter()
                .filter(|c| c.string_element_map_projection().is_some())
                .count();
        let row_bytes =
            cells.div_ceil(64) * 8 + std::mem::size_of::<usize>() + std::mem::size_of::<Vec<u64>>();
        println!(
            "{}",
            serde_json::json!({"phase":"complete-native-accepted","artifact":form,"seconds":compile_start.elapsed().as_secs_f64(),"blocks":plan.blocks().len(),"values":plan.values().len(),"analysis_cells":cells,"estimated_definition_storage_bytes":row_bytes * plan.blocks().len(),"definition_storage_cap":plan.analysis_byte_limit()})
        );
        for case in cases {
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    ("input", matrix(&case.input)),
                    ("prefix", matrix(&case.prefix)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let policy = mode.execution();
            println!(
                "{}",
                serde_json::json!({"phase":"complete-compression-start","limit_mode":"NO_LIMITS","managed_memory_limit":u64::MAX,"artifact":form,"case":case.name,"input_bytes":case.input.len(),"memory_before":memory.used_bytes(),"policy":format!("{policy:?}")})
            );
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"complete-compression","artifact":form,"case":case.name,"input_bytes":case.input.len(),"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"memory_after":memory.used_bytes(),"result":format!("{result:?}")})
            );
            result.unwrap();
            let actual = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            assert_eq!(actual, matrix(&case.expected), "{} {form}", case.name);
            let Data::Matrix(values) = actual else {
                panic!("compressed bytes")
            };
            let raw: Vec<u8> = values
                .iter()
                .map(|value| match value {
                    Data::U8(byte) => *byte,
                    _ => panic!("compressed byte kind"),
                })
                .collect();
            if let Some(kind) = case.first_block_kind {
                assert_eq!(
                    (raw[case.prefix.len()] >> 1) & 3,
                    kind,
                    "{} {form} first block kind",
                    case.name
                );
            }
            let inflated =
                miniz_oxide::inflate::decompress_to_vec(&raw[case.prefix.len()..]).unwrap();
            assert_eq!(inflated, case.input, "{} {form}", case.name);
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((args, aliases, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
            println!(
                "{}",
                serde_json::json!({"phase":"complete-compression-verified","case":case.name,"raw_bytes":raw.len()-case.prefix.len(),"cleanup":true})
            );
        }
    }
}
