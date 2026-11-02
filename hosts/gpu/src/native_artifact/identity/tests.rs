use crate::native_artifact::identity::*;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn plan(source: &str) -> NativeArtifactPlan {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4e4c4944), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
        .unwrap()
        .compile_artifact()
        .unwrap();
    mech_engine::native_artifact::lower_pure_native_artifact(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap()
}
#[test]
fn identity_is_deterministic_and_binds_real_typed_ir_revision_and_target() {
    let first = plan("left<u64> + right<u64>\n");
    let same = plan("left<u64> + right<u64>\n");
    let other = plan("left<u64> - right<u64>\n");
    assert_eq!(typed_ir_sha256(&first), typed_ir_sha256(&same));
    assert_ne!(typed_ir_sha256(&first), typed_ir_sha256(&other));
    // Only metadata is exercised here; no object or entry executes.
    let descriptor = NativeAbiDescriptor::from_compiled(&first, 32, 1024);
    let identity = NativeObjectIdentity::from_descriptor(&descriptor).unwrap();
    let checked = descriptor_from_identity(&same, identity.bytes()).unwrap();
    assert_eq!(checked.typed_ir_sha256(), descriptor.typed_ir_sha256());
    assert!(descriptor_from_identity(&other, identity.bytes()).is_err());
    for offset in [0, 16, 24, 32, 40, 72, 104, 128, 136, 144] {
        let mut corrupt = *identity.bytes();
        corrupt[offset] ^= 1;
        assert!(
            descriptor_from_identity(&first, &corrupt).is_err(),
            "field {offset}"
        );
    }
    for offset in [112, 120] {
        let mut corrupt = *identity.bytes();
        corrupt[offset..offset + 8].fill(0);
        assert!(descriptor_from_identity(&first, &corrupt).is_err());
    }
}

#[test]
fn cast_identity_rejects_pre_cast_versions_and_equal_width_reverse_direction() {
    let forward = plan("result<i64> := value<u64>\nresult\n");
    let reverse = plan("result<u64> := value<i64>\nresult\n");
    assert_ne!(typed_ir_sha256(&forward), typed_ir_sha256(&reverse));
    let descriptor = NativeAbiDescriptor::from_compiled(&forward, 32, 1024);
    let identity = NativeObjectIdentity::from_descriptor(&descriptor).unwrap();
    assert!(descriptor_from_identity(&forward, identity.bytes()).is_ok());
    assert!(descriptor_from_identity(&reverse, identity.bytes()).is_err());

    // The F64-only v3 identity must not silently acquire cast semantics.
    // This is metadata validation only; no emitted object executes here.
    for version in 0..NATIVE_TYPED_IR_VERSION {
        let mut prior = *identity.bytes();
        prior[16..24].copy_from_slice(&version.to_le_bytes());
        assert!(matches!(
            descriptor_from_identity(&forward, &prior),
            Err("unsupported linked identity/ABI/work-policy version")
        ));
    }
}
