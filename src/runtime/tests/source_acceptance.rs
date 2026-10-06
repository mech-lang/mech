//! Public source, artifact, and resident execution contracts.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

#[path = "source_acceptance_cases/activation_facts.rs"]
mod activation_facts;
#[path = "source_acceptance_cases/index_range.rs"]
mod index_range;
#[path = "source_acceptance_cases/numeric.rs"]
mod numeric;

use mech_engine::ProgramArtifact;
use mech_runtime::{RuntimeBuilder, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

fn compile(source: &str) -> ProgramArtifact {
    compile_with_catalog(source, mech_stdlib::source_catalog())
}

fn compile_with_catalog(
    source: &str,
    catalog: std::sync::Arc<mech_core::FunctionCatalog>,
) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "source-acceptance.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "source did not parse cleanly: {:?}",
        document.snapshot().diagnostics.as_slice(),
    );
    RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap()
        .compile_document(&document)
        .unwrap_or_else(|error| panic!("failed to compile:\n{source}\n{error:?}"))
        .artifact()
        .clone()
}

fn roundtrip(artifact: &ProgramArtifact) -> ProgramArtifact {
    mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(artifact).unwrap(),
    )
    .unwrap()
}
