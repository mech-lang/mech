//! Product compilation must retain semantic byte identity and presentation range.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]
use mech_engine::{CanonicalSourceFrontend, SourceSemanticError};
use mech_runtime::{RuntimeBuilder, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

#[test]
fn product_semantic_error_preserves_exact_identity_and_projected_range() {
    for source in ["10⟨u8:1..10⟩\n", "café := 1\n10⟨u8:1..10⟩\n"] {
        let document = SourceDocument::parse_resolved(
            "audit:semantic-diagnostic",
            Revision(17),
            source,
            ParseConfig::default(),
        )
        .unwrap();
        let catalog = mech_stdlib::source_catalog();
        let direct = CanonicalSourceFrontend
            .compile_document_with_catalog(&document.document(), catalog.clone())
            .err()
            .unwrap();
        let error = RuntimeBuilder::new()
            .function_catalog(catalog)
            .build_compiler()
            .unwrap()
            .compile_document(&document)
            .unwrap_err();
        let retained = error
            .kind_as::<SourceSemanticError>()
            .expect("typed semantic error must cross the public compiler boundary");
        assert_eq!(retained, &direct);
        assert_eq!(error.kind_name(), direct.code);
        let range = error
            .primary_range()
            .expect("source boundary projects a presentation range");
        assert_eq!(
            range.start,
            document
                .source()
                .source_location(direct.anchor.range.start)
                .unwrap()
        );
        assert_eq!(
            range.end,
            document
                .source()
                .source_location(direct.anchor.range.end)
                .unwrap()
        );
    }
}
