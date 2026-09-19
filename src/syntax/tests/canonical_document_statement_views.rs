use mech_syntax::document::parser::canonical::parse_canonical_document_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, CanonicalOpAssign, DocumentId, OpAssignSyntax, ParseConfig, Revision, SliceStemSyntax,
    SyntaxNode, TextSnapshot, VariableAssignSyntax, parse_canonical_document, reconstruct_source,
};

fn find<T: AstNode>(node: SyntaxNode) -> Option<T> {
    T::cast(node.clone()).or_else(|| node.children().find_map(find))
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x571), Revision(7), text).unwrap()
}

fn text(node: &SyntaxNode) -> String {
    node.source().text(node.range()).unwrap()
}

#[test]
fn operator_assignment_roles_match_direct_and_document_paths() {
    for (operator, semantic) in [
        ("+=", CanonicalOpAssign::Add),
        ("-=", CanonicalOpAssign::Sub),
        ("*=", CanonicalOpAssign::Mul),
        ("/=", CanonicalOpAssign::Div),
        ("^=", CanonicalOpAssign::Exp),
    ] {
        let statement = format!("answer[1] {operator} other + 2");
        let direct = parse_canonical_document_rule_for_test(
            source(&statement),
            rules::OP_ASSIGN,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(direct.is_strictly_clean(), "{statement:?}");
        assert_eq!(direct.consumed, direct.source.full_range());
        let document_text = format!("{statement}\n");
        let document = parse_canonical_document(source(&document_text), ParseConfig::default());
        assert!(document.diagnostics.is_empty());
        assert_eq!(
            reconstruct_source(&document.root, &document.source).unwrap(),
            document_text
        );
        for root in [direct.syntax(), document.syntax()] {
            let assignment = find::<OpAssignSyntax>(root).unwrap();
            let target = assignment.target().unwrap();
            let stem = target.stem().unwrap();
            assert!(matches!(stem, SliceStemSyntax::Identifier(_)));
            assert_eq!(text(stem.syntax()), "answer");
            assert_eq!(text(target.subscripts().unwrap().syntax()), "[1]");
            assert_eq!(
                assignment
                    .operator()
                    .unwrap()
                    .selected()
                    .unwrap()
                    .semantic(),
                Some(semantic)
            );
            assert_eq!(text(assignment.value().unwrap().syntax()), "other + 2");
            assert_eq!(assignment.syntax().source().document(), DocumentId(0x571));
            assert_eq!(assignment.syntax().source().revision(), Revision(7));
        }
    }
}

#[test]
fn plain_assignment_roles_keep_target_separate_from_rhs_identifiers() {
    for statement in ["answer = other + 2", "answer[1] = other + 2"] {
        let parsed = parse_canonical_document_rule_for_test(
            source(statement),
            rules::VARIABLE_ASSIGN,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean());
        assert_eq!(parsed.consumed, parsed.source.full_range());
        let assignment = find::<VariableAssignSyntax>(parsed.syntax()).unwrap();
        let target = assignment.target().unwrap();
        assert_eq!(text(target.stem().unwrap().syntax()), "answer");
        assert_eq!(target.subscripts().is_some(), statement.contains('['));
        assert_eq!(text(assignment.value().unwrap().syntax()), "other + 2");
    }
}
