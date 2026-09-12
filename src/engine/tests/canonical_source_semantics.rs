#![cfg(feature = "source")]

use std::fs;
use std::path::PathBuf;

use mech_engine::{
    CanonicalSourceFrontend, PHASE_2I_SEMANTIC_RULES, Phase2iSemanticDisposition, SourceValue,
    phase_2i_semantic_disposition,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x540), Revision(4), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

fn recovered_expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x540), Revision(4), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(!parsed.is_strictly_clean(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("recovered Expression")
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

#[test]
fn semantic_policy_covers_the_exact_generated_component() {
    let schema = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .unwrap();
    let names = schema
        .lines()
        .skip(1)
        .map(|line| line.split('\t').next().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(PHASE_2I_SEMANTIC_RULES.len(), 80);
    assert_eq!(
        PHASE_2I_SEMANTIC_RULES
            .iter()
            .map(|rule| rule.grammar_name)
            .collect::<Vec<_>>(),
        names
    );
    assert!(
        PHASE_2I_SEMANTIC_RULES
            .iter()
            .any(|rule| rule.disposition == Phase2iSemanticDisposition::Executable)
    );
    assert_eq!(
        phase_2i_semantic_disposition("kind"),
        Some(Phase2iSemanticDisposition::CompileTime)
    );
    assert_eq!(phase_2i_semantic_disposition("unknown"), None);
}

#[test]
fn typed_expression_builds_source_program_and_preserves_anchors() {
    let expression = expression("1 + 2 * 3");
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression)
        .unwrap();

    assert_eq!(compiled.program().inputs.len(), 0);
    assert_eq!(compiled.program().outputs.len(), 1);
    assert_eq!(compiled.constants().len(), 3);
    assert_eq!(compiled.program().nodes.len(), 2);
    assert_eq!(compiled.contracts().len(), 2);
    assert_eq!(compiled.source_map().nodes.len(), 2);
    assert_eq!(compiled.source_map().nodes[0].operation, "math/mul");
    assert_eq!(compiled.source_map().nodes[1].operation, "math/add");
    assert_eq!(compiled.source_map().outputs[0].document, DocumentId(0x540));
    assert_eq!(compiled.source_map().outputs[0].revision, Revision(4));
    assert_eq!(
        compiled.source_map().outputs[0].range,
        expression.syntax().range()
    );
    compiled
        .compile_artifact()
        .expect("typed source graph must be a canonical artifact input");
}

#[test]
fn identifiers_are_resolved_once_and_reused_as_source_inputs() {
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression("signal + signal"))
        .unwrap();
    assert_eq!(compiled.program().inputs.len(), 1);
    assert_eq!(compiled.program().inputs[0].name, "signal");
    assert_eq!(compiled.program().nodes.len(), 1);
    assert_eq!(
        compiled.program().nodes[0].inputs.as_ref(),
        &[SourceValue::Input(0), SourceValue::Input(0)]
    );
}

#[test]
fn structures_calls_comprehensions_and_fsm_enter_one_source_graph() {
    for (source, final_operation) in [
        ("{a: 1, b: 2}", "source/record"),
        ("{1: 2, 3: 4}", "source/map"),
        ("{1, 2}", "set/define"),
        ("(1, 2)", "source/tuple"),
        ("[1 2]", "source/matrix"),
        ("|a<u8>|1|", "source/table"),
        ("f(left: 1, 2)", "source/call"),
        ("x[1].field", "access/index"),
        ("1..10", "range/exclusive"),
        ("x ? | * => 1", "source/match"),
        ("[x | x <- xs]", "matrix/comprehension"),
        ("{x | x <- xs}", "set/comprehension"),
        ("#controller() -> :ready", "source/fsm"),
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        assert_eq!(
            compiled.source_map().nodes.last().unwrap().operation,
            final_operation,
            "{source:?}"
        );
        assert_eq!(compiled.program().nodes.len(), compiled.contracts().len());
    }
}

#[test]
fn recovered_trees_never_construct_partial_semantics() {
    let error = CanonicalSourceFrontend
        .compile_expression(&recovered_expression("1 +"))
        .err()
        .expect("recovered syntax must be rejected");
    assert_eq!(error.code, "source-semantics/recovered-syntax");
    assert_eq!(error.anchor.document, DocumentId(0x540));
    assert_eq!(error.anchor.revision, Revision(4));
}

#[test]
fn the_source_semantic_module_has_no_aggregate_program_boundary() {
    let source =
        fs::read_to_string(repository_root().join("src/engine/src/source_semantics/frontend.rs"))
            .unwrap();
    assert!(!source.contains("mech_core::Program"));
    assert!(!source.contains("document::lower"));
    assert!(!source.contains("parser::parse("));
}
