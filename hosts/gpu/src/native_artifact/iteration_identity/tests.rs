use crate::native_artifact::iteration_identity::*;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn plan(count: u64) -> NativeIterationArtifactPlan {
    let source =
        format!("identity(x<u64>) => (next<u64>) := next := x.\nseed<u64> |{count}> identity\n");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x49544944), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let catalog = mech_stdlib::source_catalog();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    mech_engine::native_artifact::lower_scalar_iteration_artifact(&artifact, &catalog).unwrap()
}
#[test]
fn iteration_identity_binds_count_scope_graph_target_and_all_counts() {
    let other = plan(26);
    let plan = plan(25);
    assert_ne!(typed_ir_sha256(&plan), typed_ir_sha256(&other));
    let descriptor = crate::native_artifact::iteration_abi::NativeIterationDescriptor::from_plan(
        &plan, 32, 1024,
    )
    .unwrap();
    let identity = NativeIterationObjectIdentity::from_descriptor(&descriptor).unwrap();
    let restored = descriptor_from_identity(&plan, identity.bytes()).unwrap();
    assert_eq!(restored.typed_ir_sha256(), descriptor.typed_ir_sha256());
    assert!(descriptor_from_identity(&other, identity.bytes()).is_err());
    for offset in [0, 16, 24, 32, 40, 72, 104, 128, 136, 144, 152, 160, 168] {
        let mut wrong = *identity.bytes();
        wrong[offset] ^= 1;
        assert!(
            descriptor_from_identity(&plan, &wrong).is_err(),
            "field {offset}"
        );
    }
    for offset in [112, 120] {
        let mut wrong = *identity.bytes();
        wrong[offset..offset + 8].fill(0);
        assert!(descriptor_from_identity(&plan, &wrong).is_err());
    }
}
