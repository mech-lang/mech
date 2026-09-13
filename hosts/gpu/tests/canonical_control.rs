use mech_core::NodeId;
use mech_engine::{CanonicalSourceFrontend, ExecutableNodeBody, ProgramArtifact};
use mech_gpu::{ComputeLowerer, GpuDiagnosticCode, lower_elementwise_compute_program};
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
};

fn artifact(source: &str) -> ProgramArtifact {
    fn expression(node: SyntaxNode) -> Option<ExpressionSyntax> {
        ExpressionSyntax::cast(node.clone()).or_else(|| node.children().find_map(expression))
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(822), Revision(1), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean());
    assert_eq!(parsed.consumed.end.0 as usize, source.len());
    CanonicalSourceFrontend
        .compile_expression(&expression(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap()
}

#[test]
fn compute_targets_report_typed_control_without_dropping_or_flattening_arms() {
    for source in [
        "flag<bool> ? | true => 1f32 | false => 2f32",
        "true ? | true => 1f32 | false => 2f32",
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
        for error in [elementwise, batched] {
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
