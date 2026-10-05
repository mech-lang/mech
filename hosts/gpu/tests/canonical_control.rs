use mech_core::NodeId;
use mech_engine::{CanonicalSourceFrontend, ExecutableNodeBody, ProgramArtifact};
use mech_gpu::{ComputeLowerer, GpuDiagnosticCode, lower_elementwise_compute_program};

use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
};

fn artifact(source: &str) -> ProgramArtifact {
    fn expression(node: SyntaxNode) -> Option<ExpressionSyntax> {
        ExpressionSyntax::cast(node.clone()).or_else(|| node.children().find_map(expression))
    }
    let parsed = mech_syntax::document::parse_canonical_document(
        TextSnapshot::new(DocumentId(822), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let syntax = expression(parsed.syntax()).unwrap();
    assert_eq!(syntax.syntax().range(), parsed.source.full_range());
    CanonicalSourceFrontend
        .compile_expression(&syntax)
        .unwrap()
        .compile_artifact()
        .unwrap()
}

#[test]
fn compute_targets_report_typed_control_without_dropping_or_flattening_arms() {
    for source in [
        "flag<bool> ? | true => 1f32 | false => 2f32",
        "true ? | true => 1f32 | false => 2f32",
        "signal<f32> ? | 0f32 => 1f32 | * => 2f32",
    ] {
        let artifact = artifact(source);
        let control: NodeId = artifact
            .nodes()
            .iter()
            .find(|node| matches!(node.body, ExecutableNodeBody::Match(_)))
            .unwrap()
            .node;
        let placement = ComputeLowerer.plan(&artifact);
        assert!(!placement.fully_accelerated);
        assert_eq!(
            placement
                .nodes
                .iter()
                .find(|node| node.node == control)
                .unwrap()
                .target,
            mech_compute::ComputeExecutionTarget::Cpu
        );
        let elementwise = lower_elementwise_compute_program(&artifact).unwrap_err();
        let batched = ComputeLowerer.compile_batched(&artifact, 1).unwrap_err();
        for (error, input_schema_checked_first) in [(elementwise, false), (batched, true)] {
            if input_schema_checked_first && source.starts_with("flag<bool>") {
                assert!(
                    error.diagnostics().iter().any(|diagnostic| {
                        diagnostic.code == GpuDiagnosticCode::SchemaUnsupported
                            && diagnostic.detail.contains("port `flag`")
                    }),
                    "{error}"
                );
                continue;
            }
            assert!(
                error
                    .diagnostics()
                    .iter()
                    .any(|diagnostic| diagnostic.node == Some(control)
                        && diagnostic.code == GpuDiagnosticCode::OperationUnsupported
                        && diagnostic.detail.contains("Typed match control")),
                "{error}"
            );
        }
    }
}

#[test]
fn compute_targets_ignore_unreachable_control_and_its_private_slots() {
    use mech_syntax::document::DocumentSyntax;
    for result in ["1f32", "1"] {
        let source = format!("unused := flag<bool> ? | * => {result}.\n~value := 3f32");
        let parsed = mech_syntax::document::parse_canonical_document(
            TextSnapshot::new(DocumentId(822), Revision(2), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        let artifact = CanonicalSourceFrontend
            .compile_document(&document)
            .unwrap()
            .compile_artifact()
            .unwrap();
        assert!(
            artifact
                .nodes()
                .iter()
                .any(|node| matches!(node.body, ExecutableNodeBody::Match(_)))
        );
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        lower_elementwise_compute_program(&artifact).unwrap();
        ComputeLowerer.compile(&artifact).unwrap();
        ComputeLowerer.compile_batched(&artifact, 1).unwrap();
    }
}
