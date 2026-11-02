#![cfg(feature = "canonical-native-program")]
//! Generated guest code and flat descriptor evidence for the consolidation.
use mech_engine::CanonicalSourceFrontend;
use mech_gpu::native_artifact::emit_program_object;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[test]
fn original_and_decoded_guest_code_and_descriptor_snapshots() {
    let cases = [
        ("scalar", "a := 7u64\nb := a + 11u64\n"),
        (
            "aggregate",
            "pair := (7u64, 11u64)\nvalue := (pair? | (left, right) => left + right)\n",
        ),
        (
            "owner",
            include_str!("fixtures/native/map-owner-binding-state.mec"),
        ),
        (
            "owner-words",
            include_str!("fixtures/native/map-owner-word-state.mec"),
        ),
    ];
    for (name, source) in cases {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x434f4e53), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{name}: {:?}",
            parsed.diagnostics
        );
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytecode).unwrap();
        let mut prior = None;
        for (route, artifact) in [("source", artifact), ("decoded", decoded)] {
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let image = object.descriptor().encode_build_owned_image().unwrap();
            if let Some(prior) = &prior {
                assert_eq!(prior, &image, "{name}: source/decoded metadata");
            }
            prior = Some(image.clone());
            if let Some(directory) = std::env::var_os("MECH_NATIVE_ARCHITECTURE_SNAPSHOT") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(directory.join(format!("{name}-{route}.o")), object.bytes())
                    .unwrap();
                std::fs::write(
                    directory.join(format!("{name}-{route}.descriptor.json")),
                    image,
                )
                .unwrap();
                std::fs::write(
                    directory.join(format!("{name}-{route}.metadata.json")),
                    serde_json::to_vec_pretty(&object.metadata()).unwrap(),
                )
                .unwrap();
                std::fs::write(directory.join(format!("{name}-{route}.mecb")), &bytecode).unwrap();
            }
        }
    }
}
