#![cfg(feature = "source")]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use mech_engine::{CanonicalSourceFrontend, PHASE_2I_SEMANTIC_RULES, Phase2iSemanticDisposition};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSize,
    TextSnapshot,
};

struct SemanticCase {
    rule: &'static str,
    source: &'static str,
    final_operation: Option<&'static str>,
}

const SEMANTIC_CASES: &[SemanticCase] = &[
    case("expression", "1 + 2", Some("math/add")),
    case("factor", "1", None),
    case("fancy-table", "╭─\n│a│\n│1│", Some("source/table")),
    case("formula", "1 + 2", Some("math/add")),
    case(
        "fsm-pipe",
        "#machine -> :next => :value",
        Some("source/fsm"),
    ),
    case("function-call", "foo(1)", Some("source/call")),
    case("inline-table", "|a<u8>|1|", Some("source/table")),
    case("l1", "true && false", Some("logic/and")),
    case("l2", "1 == 2", Some("compare/eq")),
    case("l3", "1 + 2", Some("math/add")),
    case("l4", "2 * 3", Some("math/mul")),
    case("l5", "2 ^ 3", Some("math/pow")),
    case("l6", "a ⋈ b", Some("table/join")),
    case("l7", "{1} ∪ {2}", Some("set/union")),
    case("literal", "1", None),
    case("map", "{1:2}", Some("source/map")),
    case("matrix", "[1 2]", Some("source/matrix")),
    case(
        "matrix-comprehension",
        "[x | x <- xs]",
        Some("matrix/comprehension"),
    ),
    case("negate-factor", "-1", Some("math/neg")),
    case("not-factor", "!true", Some("logic/not")),
    case("parenthetical-term", "(1 + 2)", Some("math/add")),
    case("range-expression", "1..10", Some("range/exclusive")),
    case("record", "{a:1}", Some("source/record")),
    case("regular-table", "|a<u8>|\n|1|", Some("source/table")),
    case("set", "{1,2}", Some("set/define")),
    case(
        "set-comprehension",
        "{x | x <- xs}",
        Some("set/comprehension"),
    ),
    case("slice", "x[1]", Some("access/index")),
    case("structure", "[1]", Some("source/matrix")),
    case("table", "|a<u8>|1|", Some("source/table")),
    case("tuple", "(1,2)", Some("source/tuple")),
    case("tuple-struct", ":some(1)", Some("source/tuple-struct")),
    case("var", "x<u8>", None),
];

const fn case(
    rule: &'static str,
    source: &'static str,
    final_operation: Option<&'static str>,
) -> SemanticCase {
    SemanticCase {
        rule,
        source,
        final_operation,
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x549), Revision(7), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    assert_eq!(
        parsed.consumed.end,
        TextSize(source.len() as u32),
        "{source:?}"
    );
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

fn certification_dispositions() -> BTreeMap<String, String> {
    let table = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-certification.tsv"),
    )
    .unwrap();
    table
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 12);
            (fields[0].to_owned(), fields[8].to_owned())
        })
        .collect()
}

#[test]
fn every_executable_rule_has_specification_derived_program_evidence() {
    let dispositions = certification_dispositions();
    let cases = SEMANTIC_CASES
        .iter()
        .map(|case| (case.rule, case))
        .collect::<BTreeMap<_, _>>();
    let executable = PHASE_2I_SEMANTIC_RULES
        .iter()
        .filter(|rule| rule.disposition == Phase2iSemanticDisposition::Executable)
        .collect::<Vec<_>>();
    assert_eq!(executable.len(), 32);
    assert_eq!(cases.len(), executable.len());

    for rule in PHASE_2I_SEMANTIC_RULES {
        let expected = match rule.disposition {
            Phase2iSemanticDisposition::Executable => "executable",
            Phase2iSemanticDisposition::Structural => "structural",
            Phase2iSemanticDisposition::CompileTime => "compile-time",
        };
        assert_eq!(
            dispositions.get(rule.grammar_name).map(String::as_str),
            Some(expected)
        );
    }

    for rule in executable {
        let case = cases
            .get(rule.grammar_name)
            .unwrap_or_else(|| panic!("missing semantic case for {}", rule.grammar_name));
        let syntax = expression(case.source);
        let compiled = CanonicalSourceFrontend
            .compile_expression(&syntax)
            .unwrap_or_else(|error| panic!("{} on {:?}: {error}", case.rule, case.source));
        assert_eq!(compiled.program().outputs.len(), 1, "{}", case.rule);
        assert_eq!(
            compiled.program().nodes.len(),
            compiled.contracts().len(),
            "{}",
            case.rule
        );
        assert_eq!(
            compiled
                .source_map()
                .nodes
                .last()
                .map(|node| node.operation.as_str()),
            case.final_operation,
            "{} on {:?}",
            case.rule,
            case.source
        );
        assert_eq!(
            compiled.source_map().outputs[0].range,
            syntax.syntax().range(),
            "{}",
            case.rule
        );
        compiled
            .compile_artifact()
            .unwrap_or_else(|error| panic!("{} artifact: {error:?}", case.rule));
    }
}
