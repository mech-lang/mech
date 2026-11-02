use crate::native_artifact::control_identity::*;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn plan(source: &str) -> NativeMatchArtifactPlan {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d4c4944), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    mech_engine::native_artifact::lower_scalar_match_artifact(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap()
}
#[test]
fn match_link_identity_binds_exact_cfg_revision_target_entry_and_counts() {
    let first = plan("(flag<bool>? | true => value<u64> + 1u64 | false => value<u64>)\n");
    let same = plan("(flag<bool>? | true => value<u64> + 1u64 | false => value<u64>)\n");
    let other = plan("(flag<bool>? | true => value<u64> + 2u64 | false => value<u64>)\n");
    assert_eq!(typed_ir_sha256(&first), typed_ir_sha256(&same));
    assert_ne!(typed_ir_sha256(&first), typed_ir_sha256(&other));
    let descriptor =
        crate::native_artifact::control_abi::NativeMatchDescriptor::from_plan(&first, 32, 1024)
            .unwrap();
    let identity = NativeMatchObjectIdentity::from_descriptor(&descriptor).unwrap();
    let restored = descriptor_from_identity(&same, identity.bytes()).unwrap();
    assert_eq!(restored.typed_ir_sha256(), descriptor.typed_ir_sha256());
    assert_eq!(restored.entry(), descriptor.entry());
    assert_eq!(restored.visits().len(), descriptor.visits().len());
    assert!(descriptor_from_identity(&other, identity.bytes()).is_err());
    for version in [0u64, 1] {
        let mut old = *identity.bytes();
        old[16..24].copy_from_slice(&version.to_le_bytes());
        assert!(descriptor_from_identity(&first, &old).is_err());
    }
    for offset in [0, 16, 24, 32, 40, 72, 104, 128, 136, 144, 152, 160] {
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
