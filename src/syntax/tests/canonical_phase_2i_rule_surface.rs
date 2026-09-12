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

// Structural hashes captured from the canonical Phase 2I-C clean recognizer.
// S2 recovery paths must never change these clean trees.
const PHASE_2I_C_CLEAN_HASHES: &[u64; 80] = &[
    4690694857777605883,
    13905645448042576103,
    10870448413493509317,
    3020044792086417727,
    16488282065910742516,
    14281317787117039531,
    2769745888203706395,
    386169036022407112,
    13485355853238889961,
    2950799521910062956,
    5781077196392771268,
    11186595071878088510,
    8906010932451903890,
    4021004977156526808,
    16829516963740763292,
    11531028313062290691,
    14229619072495360548,
    15955007560155675859,
    1474182565632622366,
    6071720404677301713,
    4896411008235002978,
    14822387269900549885,
    12483771098825470956,
    14425534328365033674,
    15085676178738391771,
    5314978172347722699,
    16957770984042310547,
    17402586977283105061,
    13096106965844893654,
    15299103449794643293,
    14185569560039540382,
    4633517801149530814,
    8666045603091957262,
    3710294141652703806,
    4646339690299799936,
    7364264300447854822,
    17617743857904627426,
    15367931155919598467,
    16462568352945589092,
    9702843422604825095,
    8906010932451903890,
    14642993577772595476,
    3110090191883221768,
    5141929758315618324,
    12020738732408630277,
    5914926822494316490,
    1317429301566980389,
    6337713287772140858,
    7707487066569956823,
    17525468374060031790,
    5781543885781755312,
    6735033807355356576,
    4834957888592439121,
    5671002928360723118,
    5056544452338799626,
    3739378678532252297,
    7172780464351664167,
    1537065674291990702,
    14556742428805822168,
    14638571144539794630,
    10868118958275826861,
    12757472422431810587,
    1216509841501496325,
    4685379702415065137,
    226662167499618394,
    12182555315924508784,
    2001573980976137318,
    7955882449780973395,
    15635711094224418627,
    13216532378308639288,
    11183257828521482799,
    18246268259994793420,
    4336346069398682744,
    10999501563816869536,
    7238909464233703798,
    6342872406303678999,
    18365713193954839706,
    17269789349614934,
    7045606203036859219,
    14800384589674269329,
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
    for (index, (rule, text)) in SMOKE_CASES.iter().enumerate() {
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
        assert_eq!(
            parsed.root.structural_hash, PHASE_2I_C_CLEAN_HASHES[index],
            "{rule:?} on {text:?} changed its Phase 2I-C clean tree"
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
