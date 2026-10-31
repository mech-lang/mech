//! Public source-to-Object application example. No large compiler-thread wrapper.
use mech_core::{
    ValueDataDraft as Data,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{
    NativeCompileLimits, generate_linked_program_application_with_limits,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

#[path = "../tests/fixtures/native/deflate_assembly.rs"]
mod assembly;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn matrix(bytes: &[u8]) -> Data {
    Data::Matrix(bytes.iter().copied().map(Data::U8).collect())
}
#[derive(serde::Deserialize)]
struct Case {
    name: String,
    input: Vec<u8>,
    prefix: Vec<u8>,
    expected: Vec<u8>,
}
fn cases(root: &Path, artifact: &ProgramArtifact) {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/native/deflate-compress-reference.json"
    ))
    .unwrap();
    let cases: Vec<Case> = serde_json::from_value(fixture["cases"].clone()).unwrap();
    let schemas = Arc::new(artifact.schemas().clone());
    std::fs::create_dir(root.join("cases")).unwrap();
    let mut receipts = Vec::new();
    for case in cases {
        let drafts = artifact
            .inputs()
            .iter()
            .map(|input| {
                let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                let bytes = match name.as_str() {
                    "input" => &case.input,
                    "prefix" => &case.prefix,
                    _ => panic!("unexpected application input"),
                };
                let shape = mech_core::shape_for_resolved_extents(
                    schemas.get(input.schema).unwrap(),
                    &[1, bytes.len() as u64],
                )
                .unwrap();
                ValueDraft {
                    schema: input.schema,
                    shape_values: shape.parameter_values().into(),
                    data: matrix(bytes),
                }
            })
            .collect::<Vec<_>>();
        let input = format!("cases/{}.json", case.name);
        std::fs::write(root.join(&input), serde_json::to_vec(&drafts).unwrap()).unwrap();
        let output = artifact.outputs().last().unwrap().schema;
        let shape = mech_core::shape_for_resolved_extents(
            schemas.get(output).unwrap(),
            &[1, case.expected.len() as u64],
        )
        .unwrap();
        let expected = ValueDraft {
            schema: output,
            shape_values: shape.parameter_values().into(),
            data: matrix(&case.expected),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let snapshot = expected.canonical_snapshot_bytes(&schemas).unwrap();
        assert_eq!(
            miniz_oxide::inflate::decompress_to_vec(&case.expected[case.prefix.len()..]).unwrap(),
            case.input
        );
        receipts.push(serde_json::json!({"name":case.name,"input":input,"input_bytes":case.input.len(),"output_lines":artifact.outputs().len(),"last_snapshot_hex":snapshot.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"raw":case.expected,"prefix":case.prefix,"plain":case.input}));
    }
    std::fs::write(
        root.join("cases.json"),
        serde_json::to_vec_pretty(&receipts).unwrap(),
    )
    .unwrap();
}
fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        3,
        "expected NEW_APPLICATION_DIRECTORY COMPILER_LIMITS_JSON original|decoded"
    );
    let directory = PathBuf::from(&args[0]);
    let limits: NativeCompileLimits =
        serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let source = assembly::complete_source();
    assert_eq!(source.len(), 160992);
    assert_eq!(
        hash(source.as_bytes()),
        "345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f"
    );
    let started = Instant::now();
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap();
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let artifact = match args[2].to_str().unwrap() {
        "original" => original,
        "decoded" => mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
        _ => panic!("expected original|decoded"),
    };
    eprintln!(
        "{}",
        serde_json::json!({"phase":"public-source-checked","seconds":started.elapsed().as_secs_f64(),"source_sha256":hash(source.as_bytes()),"bytecode_sha256":hash(&bytes),"form":args[2].to_str()})
    );
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let generated = generate_linked_program_application_with_limits(
        workspace,
        &directory,
        "compression-application",
        &artifact,
        &mech_stdlib::source_catalog(),
        source.as_bytes(),
        limits,
    )
    .unwrap();
    cases(generated.root(), &artifact);
    std::fs::write(
        generated.root().join("compiler-policy.json"),
        serde_json::to_vec_pretty(&limits).unwrap(),
    )
    .unwrap();
    eprintln!(
        "{}",
        serde_json::json!({"phase":"public-object-application-generated","seconds":started.elapsed().as_secs_f64(),"path":generated.root(),"metadata":generated.metadata(),"form":args[2].to_str()})
    );
}
