use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::path::Path;

fn compile(text: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(71), Revision(1), text).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "source must parse without recovery: {:?}",
        parsed.diagnostics
    );
    CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}
fn run() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "generator WORKSPACE DESTINATION");
    let text = "input<u64> + 1u64\n";
    let artifact = compile(text);
    let _generated = mech_gpu::native_artifact::generate_linked_program_application(
        Path::new(&args[0]), Path::new(&args[1]), "native-metadata-smoke", &artifact,
        &mech_stdlib::source_catalog(), text.as_bytes(),
    ).unwrap();
    println!("generated");
}

fn main() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
