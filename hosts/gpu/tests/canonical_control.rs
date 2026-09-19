use mech_core::NodeId;
use mech_engine::{CanonicalSourceFrontend, ExecutableNodeBody, ProgramArtifact};
use mech_gpu::{ComputeLowerer, GpuDiagnosticCode, lower_elementwise_compute_program};
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
    VariableDefineSyntax,
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

#[test]
fn compute_targets_ignore_unreachable_control_and_its_private_slots() {
    use mech_syntax::document::{DocumentSyntax, GreenBuilder, IdGenerator, SyntaxKind};
    for result in ["1f32", "1"] {
        let first = format!("unused := flag<bool> ? | * => {result}");
        let last = "~value := 3f32";
        let mut ids = IdGenerator::default();
        let mut builder = GreenBuilder::new(&mut ids);
        builder.start_node(SyntaxKind::Document);
        builder.start_node(SyntaxKind::Body);
        for (index, (source, rule)) in [
            (first.as_str(), rules::VARIABLE_DEFINE),
            (last, rules::VARIABLE_DEFINE),
        ]
        .into_iter()
        .enumerate()
        {
            if index != 0 {
                builder.token(SyntaxKind::Newline, "\n").unwrap();
            }
            let parsed = parse_canonical_phase_2i_rule_for_test(
                TextSnapshot::new(DocumentId(822), Revision(2), source).unwrap(),
                rule,
                ParseConfig::default(),
            )
            .unwrap();
            assert!(parsed.is_strictly_clean());
            assert_eq!(parsed.consumed.end.0 as usize, source.len());
            fn unit(node: SyntaxNode) -> Option<SyntaxNode> {
                if VariableDefineSyntax::cast(node.clone()).is_some()
                    || ExpressionSyntax::cast(node.clone()).is_some()
                {
                    Some(node)
                } else {
                    node.children().find_map(unit)
                }
            }
            builder
                .reuse_node(unit(parsed.syntax()).unwrap().green().clone())
                .unwrap();
        }
        builder.finish_node().unwrap();
        builder.finish_node().unwrap();
        let document = DocumentSyntax::cast(SyntaxNode::new_root(
            builder.finish().unwrap(),
            TextSnapshot::new(DocumentId(822), Revision(2), format!("{first}\n{last}")).unwrap(),
        ))
        .unwrap();
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
