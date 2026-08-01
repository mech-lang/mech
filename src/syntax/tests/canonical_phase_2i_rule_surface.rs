use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::{canonical_rule_name, rules};
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxNode, TextRange, TextSize, TextSnapshot,
    reconstruct_source_range, validate_lossless_range,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const SMOKE_CASES: &[(RuleId, &str)] = &[
    (rules::ARGUMENT_LIST, "(1, x: 2)"),
    (rules::BINDING, "a: 1"),
    (rules::BRACE_SUBSCRIPT, "{1}"),
    (rules::BRACKET_SUBSCRIPT, "[1]"),
    (rules::CALL_ARG, "1"),
    (rules::CALL_ARG_WITH_BINDING, "x: 1"),
    (rules::COMPREHENSION_QUALIFIER, "x <- xs"),
    (rules::EXPRESSION, "1 + 2"),
    (rules::FACTOR, "1"),
    (rules::FANCY_TABLE, "╭─\n│a│\n│1│"),
    (rules::FANCY_TABLE_HEADER, "a│"),
    (rules::FIELD, "a"),
    (rules::FORMULA, "1 + 2"),
    (rules::FORMULA_SUBSCRIPT, "1 + 2"),
    (rules::FSM_ARGS, "(1, x: 2)"),
    (rules::FSM_ASYNC_TRANSITION, "~> :next"),
    (rules::FSM_INSTANCE, "#machine(1)"),
    (rules::FSM_OUTPUT, "=> :value"),
    (rules::FSM_PIPE, "#machine -> :next => :value"),
    (rules::FSM_STATE_TRANSITION, "-> :next"),
    (rules::FSM_VALUE, ":value"),
    (rules::FUNCTION_CALL, "foo(1)"),
    (rules::GENERATOR, "x <- xs"),
    (rules::HEADER_FIELD, "a<u8>"),
    (rules::INLINE_TABLE, "|a<u8>|1|"),
    (rules::INLINE_TABLE_HEADER, "a<u8>|"),
    (rules::INLINE_TABLE_ROW, "1|"),
    (rules::KIND, "u8"),
    (rules::KIND_ANNOTATION, "<u8>"),
    (rules::KIND_KIND, "<u8>"),
    (rules::KIND_MAP, "{u8:f64}"),
    (rules::KIND_MATRIX, "[u8]"),
    (rules::KIND_RECORD, "{a<u8>}"),
    (rules::KIND_SCALAR, "u8:1..10"),
    (rules::KIND_SET, "{u8}:10"),
    (rules::KIND_TABLE, "|a<u8>|:10"),
    (rules::KIND_TUPLE, "(u8,f64)"),
    (rules::KIND_WITH_OPTION, "u8?"),
    (rules::L1, "true && false"),
    (rules::L2, "1 == 2"),
    (rules::L3, "1 + 2"),
    (rules::L4, "2 * 3"),
    (rules::L5, "2 ^ 3"),
    (rules::L6, "a ⋈ b"),
    (rules::L7, "{1} ∪ {2}"),
    (rules::LITERAL, "1"),
    (rules::MAP, "{1:2}"),
    (rules::MAPPING, "1:2"),
    (rules::MATCH_ARM, "| * => 1"),
    (rules::MATRIX, "[1 2]"),
    (rules::MATRIX_COLUMN, "1,"),
    (rules::MATRIX_COMPREHENSION, "[x | x <- xs]"),
    (rules::MATRIX_ROW, "1 2;"),
    (rules::NEGATE_FACTOR, "-1"),
    (rules::NOT_FACTOR, "!true"),
    (rules::PARENTHETICAL_TERM, "(1 + 2)"),
    (rules::PATTERN, "*"),
    (rules::PATTERN_ARRAY, "[head, ..., tail]"),
    (rules::PATTERN_ARRAY_ITEM, "x"),
    (rules::PATTERN_ARRAY_TOKEN, "..."),
    (rules::PATTERN_ATOM_STRUCT, ":some(x)"),
    (rules::PATTERN_TUPLE, "(x,y)"),
    (rules::PATTERN_TUPLE_STRUCT, "`some(x)"),
    (rules::RANGE_EXPRESSION, "1..10"),
    (rules::RANGE_SUBSCRIPT, "1..10"),
    (rules::RECORD, "{a:1}"),
    (rules::REGULAR_TABLE, "|a<u8>|\n|1|"),
    (rules::SET, "{1,2}"),
    (rules::SET_COMPREHENSION, "{x | x <- xs}"),
    (rules::SLICE, "x[1]"),
    (rules::STRUCTURE, "[1]"),
    (rules::SUBSCRIPT, "[1].field"),
    (rules::TABLE, "|a<u8>|1|"),
    (rules::TABLE_HEADER, "a<u8>|"),
    (rules::TABLE_ROW, "|1 2|"),
    (rules::TABLE_ROW2, "|1|2|"),
    (rules::TUPLE, "(1,2)"),
    (rules::TUPLE_STRUCT, ":some(1)"),
    (rules::VAR, "x<u8>"),
    (rules::VARIABLE_DEFINE, "x := 1"),
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c), Revision(0), text).unwrap()
}

#[derive(Debug)]
struct SchemaRow {
    policy: String,
    kind: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema() -> BTreeMap<String, SchemaRow> {
    let source = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .expect("read phase-2i-syntax-schema.tsv");
    source
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 6);
            (
                fields[0].to_owned(),
                SchemaRow {
                    policy: fields[2].to_owned(),
                    kind: fields[3].to_owned(),
                },
            )
        })
        .collect()
}

fn count_named_kind(node: &SyntaxNode, expected: &str) -> usize {
    usize::from(format!("{:?}", node.kind()) == expected)
        + node
            .children()
            .map(|child| count_named_kind(&child, expected))
            .sum::<usize>()
}

fn direct_named_kind_count(node: &SyntaxNode, expected: &str) -> usize {
    node.children()
        .filter(|child| format!("{:?}", child.kind()) == expected)
        .count()
}

fn alias_kind_name(rule_name: &str) -> String {
    rule_name
        .split('-')
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

#[test]
fn every_phase_2i_rule_has_a_clean_direct_smoke_case() {
    assert_eq!(SMOKE_CASES.len(), 80);
    let schema = schema();
    for (rule, text) in SMOKE_CASES {
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source(text), *rule, ParseConfig::default())
                .unwrap_or_else(|| panic!("missing Phase 2I dispatcher arm for {rule:?}"));
        assert_eq!(
            parsed.outcome,
            CanonicalRuleOutcome::Matched,
            "{rule:?} on {text:?}"
        );
        assert!(parsed.is_strictly_clean(), "{rule:?} on {text:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::new(TextSize::ZERO, TextSize(text.len() as u32)),
            "{rule:?} on {text:?}"
        );
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
            .unwrap_or_else(|error| panic!("{rule:?} was not lossless: {error:?}"));
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            *text
        );

        let name = canonical_rule_name(*rule).expect("canonical rule name");
        let row = schema
            .get(name)
            .unwrap_or_else(|| panic!("schema row for {name}"));
        let syntax = parsed.syntax();
        match row.policy.as_str() {
            "node" => assert_eq!(
                direct_named_kind_count(&syntax, &row.kind),
                1,
                "{name} must emit its configured direct node"
            ),
            "conditional-node" => assert_eq!(
                count_named_kind(&syntax, &row.kind),
                1,
                "{name} operator case must emit exactly one chain node"
            ),
            "transparent" => {
                let alias = alias_kind_name(name);
                assert_eq!(
                    count_named_kind(&syntax, &alias),
                    0,
                    "{name} must not emit an alias wrapper"
                );
                let expected_child = match name {
                    "formula" => "AdditiveExpression",
                    "pattern-array-item" => "Pattern",
                    _ => unreachable!("closed transparent schema"),
                };
                assert_eq!(
                    direct_named_kind_count(&syntax, expected_child),
                    1,
                    "{name}"
                );
            }
            other => panic!("unknown schema policy {other:?}"),
        }
    }
}

#[test]
fn every_conditional_precedence_rule_omits_its_chain_node_without_an_operator() {
    let schema = schema();
    for (rule, _) in SMOKE_CASES {
        let name = canonical_rule_name(*rule).unwrap();
        let row = &schema[name];
        if row.policy != "conditional-node" {
            continue;
        }
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source("1"), *rule, ParseConfig::default())
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{name}");
        assert!(parsed.is_strictly_clean(), "{name}");
        assert_eq!(count_named_kind(&parsed.syntax(), &row.kind), 0, "{name}");
    }
}

#[test]
fn every_phase_2i_rule_rejects_empty_source_transactionally() {
    for (rule, _) in SMOKE_CASES {
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source(""), *rule, ParseConfig::default())
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{rule:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{rule:?}"
        );
        assert!(parsed.diagnostics.is_empty(), "{rule:?}");
        assert!(!parsed.root.flags.intersects(
            mech_syntax::document::NodeFlags::ERROR
                | mech_syntax::document::NodeFlags::MISSING
                | mech_syntax::document::NodeFlags::CONTAINS_ERROR
                | mech_syntax::document::NodeFlags::CONTAINS_MISSING
        ));
    }
}
