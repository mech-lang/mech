use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot, parse_canonical_document};
fn run() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let source = std::fs::read_to_string(&args[1]).unwrap();
    let expected = std::fs::read(&args[2]).unwrap();
    let parsed = parse_canonical_document(TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source.as_str()).unwrap(), ParseConfig::default());
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = CanonicalSourceFrontend.compile_document_with_catalog(&syntax, mech_stdlib::source_catalog()).unwrap();
    let artifact = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    assert_eq!(bytes, expected, "the complete compressor canonical artifact changed");
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(), expected);
    println!("{}", serde_json::json!({"fresh_source_bytes":source.len(),"artifact_bytes":bytes.len(),"fresh_original_matches_measured_artifact":true,"decoded_reencoding_matches":true,"compression_executed":false}));
}
fn main() { std::thread::Builder::new().stack_size(32*1024*1024).spawn(run).unwrap().join().unwrap(); }
