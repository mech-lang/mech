//! Measure complete level-12 source compilation separately from building the compiler.
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn run() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() == 4,
        "expected benchmark|fixture|SOURCE OUTPUT_DIRECTORY compact|managed|frontend"
    );
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).unwrap();
    let source = std::fs::read_to_string(&args[1]).unwrap();
    std::fs::write(out.join("source.mec"), &source).unwrap();
    let total = Instant::now();
    let mut phases = Vec::new();
    let mut stamp = |name: &str, start: Instant| {
        let row = serde_json::json!({"phase":name,"seconds":start.elapsed().as_secs_f64(),"total_seconds":total.elapsed().as_secs_f64()});
        eprintln!("{row}");
        phases.push(row);
    };
    eprintln!("compile-profile: pid={} phase=parse", std::process::id());
    let start = Instant::now();
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    stamp("parse", start);
    let start = Instant::now();
    let catalog = mech_stdlib::source_catalog();
    stamp("catalog", start);
    eprintln!("compile-profile: phase=frontend");
    let start = Instant::now();
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, catalog.clone())
        .unwrap();
    stamp("frontend", start);
    let start = Instant::now();
    let artifact = product.compile_artifact().unwrap();
    stamp("artifact", start);
    let start = Instant::now();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    stamp("encode", start);
    std::fs::write(out.join("program.mecb"), &bytes).unwrap();
    eprintln!("compile-profile: phase=native mode={}", args[3]);
    let start = Instant::now();
    assert_eq!(args[3], "frontend");
    let (object, metadata) = (Vec::<u8>::new(), serde_json::json!(null));
    stamp("native", start);
    let total_seconds = total.elapsed().as_secs_f64();
    std::fs::write(out.join("program.o"), &object).unwrap();
    std::fs::write(
        out.join("metadata.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let receipt = serde_json::json!({"mode":args[3],"phases":phases,"total_seconds":total_seconds,"source_bytes":source.len(),"source_sha256":hash(source.as_bytes()),"bytecode_bytes":bytes.len(),"bytecode_sha256":hash(&bytes),"object_bytes":object.len(),"object_sha256":hash(&object)});
    std::fs::write(
        out.join("compile.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    println!("{receipt}");
}
fn main() {
    std::thread::Builder::new()
        .name("compile-profile".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
