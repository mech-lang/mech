use super::LibraryCallRoute as Route;
use crate::native_artifact::{JitProgramArtifact, NativeProgramDescriptor};
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[test]
fn prepared_selected_routes_are_rederived_from_actual_factories_on_image_restore() {
    for (source, expected) in [
        (
            "+> sequence\nsequence/clear(input<[u8]>)\n",
            Route::UnsignedBuffer,
        ),
        (
            "+> sequence\nsequence/length(input<[u32]>)\n",
            Route::UnsignedMatrixLength,
        ),
        (
            "xs := input<[u16]>\nxs[at<u64>]\n",
            Route::UnsignedMatrixElement,
        ),
        (
            "replace(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=\n  ~next := xs\n  next[at] = word\n  out := next.\nreplace(input<[u32]>, at<u64>, word<u32>)\n",
            Route::UnsignedMatrixReplace,
        ),
        (
            "record := entry<{word<u32>}>\nrecord.word\n",
            Route::RecordScalarField,
        ),
        (
            "+> sequence\nwalk(words<[u32]>, left<u64>) => <u64>\n  | (words, left) => ((left > 0u64)? | true => walk(words, left - 1u64) | false => sequence/length(words)).\nwalk(input<[u32]>, steps<u64>)\n",
            Route::BufferFrame,
        ),
        (
            "+> text\ntext/join(input<[string]>, \",\")\n",
            Route::Canonical,
        ),
    ] {
        let catalog = mech_stdlib::source_catalog();
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x524f5554), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        let original = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let descriptor = jit.descriptor();
            assert!(
                descriptor.metadata().plan.blocks().iter().enumerate().any(
                    |(site, block)| matches!(
                        block.action(),
                        mech_engine::native_artifact::NativeIterationAction::LibraryCall { .. }
                    ) && descriptor.metadata().call_routes[site] == expected
                ),
                "{expected:?}: {source}"
            );
            let image = descriptor.encode_build_owned_image().unwrap();
            // SAFETY: this checked JIT build produced the unmodified image.
            // Restored metadata is inspected only and never executes a pointer.
            let restored =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            assert_eq!(
                descriptor.metadata().call_routes,
                restored.metadata().call_routes
            );
            assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        }
    }
}
