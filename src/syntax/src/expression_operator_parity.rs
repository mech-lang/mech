//! Crate-internal parity coverage for Phase 2D operator productions.
//!
//! This module is deliberately nested beneath `expressions` so the tests can
//! exercise the six private legacy table-operator parsers without changing
//! their visibility.  It compares direct canonical productions with the
//! legacy functions' values and prefix extents.

use super::*;

use alloc::vec::Vec;

use mech_core::nodes::{
    AddSubOp, ComparisonOp, FormulaOperator, LogicOp, MulDivOp, PowerOp, RangeOp, SetOp, TableOp,
    VecOp,
};

use crate::document::ast::operators::{
    AddSubOperatorSyntax, ComparisonOperatorSyntax, LogicOperatorSyntax, MatrixOperatorSyntax,
    MulDivOperatorSyntax, OperatorSyntax, PowerOperatorSyntax, RangeOperatorSyntax,
    SetOperatorSyntax, TableOperatorSyntax,
};
use crate::document::lower::legacy::{
    LegacyOperatorValue, lower_legacy_add_sub_operator, lower_legacy_comparison_operator,
    lower_legacy_logic_operator, lower_legacy_matrix_operator, lower_legacy_mul_div_operator,
    lower_legacy_power_operator, lower_legacy_range_operator, lower_legacy_set_operator,
    lower_legacy_table_operator, lower_phase_2d_operator_value,
};
use crate::document::parser::canonical::operators::PHASE_2D_OPERATOR_RULES;
use crate::document::parser::canonical::{
    CanonicalSourceRuleSnapshot, parse_canonical_phase_2d_rule_for_test,
};
use crate::document::parser::rules;
use crate::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange,
    TextSize, TextSnapshot, reconstruct_source_range,
};
use crate::{ParseResult, ParseString};

type LegacyLeafParser =
    for<'source> fn(ParseString<'source>) -> ParseResult<'source, LegacyOperatorValue>;
type LegacyNonLeafParser =
    for<'source> fn(ParseString<'source>) -> ParseResult<'source, LegacyNonLeafValue>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LegacyPrefix {
    consumed: TextSize,
    remaining: TextSize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyMatch {
    value: LegacyOperatorValue,
    prefix: LegacyPrefix,
}

struct LeafContract {
    name: &'static str,
    rule: RuleId,
    kind: SyntaxKind,
    parser: LegacyLeafParser,
    expected: LegacyOperatorValue,
    // Every glyph spelling admitted by this direct leaf.  The one whitespace
    // constrained rule uses complete accepted source fragments instead.
    spellings: &'static [&'static str],
    // Minimal success, representative success, valid prefix, boundary, and a
    // shared-prefix/ambiguous probe, respectively.
    probes: [&'static str; 5],
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LegacyNonLeafValue {
    Formula(FormulaOperator),
    Range(RangeOp),
    Transparent,
}

struct NonLeafContract {
    name: &'static str,
    rule: RuleId,
    kind: Option<SyntaxKind>,
    parser: LegacyNonLeafParser,
    expected: LegacyNonLeafValue,
    probes: [&'static str; 5],
}

enum DirectContract {
    Leaf(LeafContract),
    NonLeaf(NonLeafContract),
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(926), Revision(0), text).unwrap()
}

fn parse(text: &str, rule: RuleId) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2d_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2D direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn legacy_match(input: &str, parser: LegacyLeafParser) -> Option<LegacyMatch> {
    let graphemes = crate::graphemes::init_tag(input);
    parser(ParseString::new(&graphemes))
        .ok()
        .map(|(remaining, value)| {
            assert!(
                remaining.error_log.is_empty(),
                "legacy parser diagnostics on {input:?}"
            );
            let consumed = graphemes[..remaining.cursor]
                .iter()
                .map(|grapheme| grapheme.len())
                .sum::<usize>();
            let remaining_bytes = graphemes[remaining.cursor..]
                .iter()
                .map(|grapheme| grapheme.len())
                .sum::<usize>();
            LegacyMatch {
                value,
                prefix: LegacyPrefix {
                    consumed: TextSize(consumed as u32),
                    remaining: TextSize(remaining_bytes as u32),
                },
            }
        })
}

fn assert_matched_parity(contract: &LeafContract, input: &str, case_kind: &str) {
    let canonical = parse(input, contract.rule);
    let legacy = legacy_match(input, contract.parser).unwrap_or_else(|| {
        panic!(
            "legacy {} {case_kind} unexpectedly rejected {input:?}",
            contract.name
        )
    });

    assert!(
        canonical.matched,
        "canonical {} {case_kind} unexpectedly rejected {input:?}",
        contract.name
    );
    assert!(
        canonical.is_strictly_clean(),
        "canonical {} {case_kind} emitted diagnostics for {input:?}",
        contract.name
    );
    assert_eq!(
        canonical.consumed.start,
        TextSize::ZERO,
        "{} {case_kind} start extent on {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.end, legacy.prefix.consumed,
        "{} {case_kind} consumed extent on {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.source.byte_len().0 - canonical.consumed.end.0,
        legacy.prefix.remaining.0,
        "{} {case_kind} remaining extent on {input:?}",
        contract.name,
    );
    assert_eq!(
        reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed).unwrap(),
        &input[..legacy.prefix.consumed.to_usize()],
        "{} {case_kind} must retain exactly the legacy prefix for {input:?}",
        contract.name,
    );

    let node = find_node(&canonical.syntax(), contract.kind).unwrap_or_else(|| {
        panic!(
            "{} {case_kind} did not produce {:?} for {input:?}",
            contract.name, contract.kind
        )
    });
    let lowered = lower_phase_2d_operator_value(&OperatorSyntax::cast(node).unwrap()).unwrap();
    assert_eq!(
        legacy.value, contract.expected,
        "legacy {} value on {input:?}",
        contract.name,
    );
    assert_eq!(
        lowered, legacy.value,
        "canonical {} lowering on {input:?}",
        contract.name,
    );
    assert_eq!(
        lowered, contract.expected,
        "canonical {} expected lowering on {input:?}",
        contract.name,
    );
}

fn assert_probe_parity(contract: &LeafContract, input: &str, case_kind: &str) {
    if legacy_match(input, contract.parser).is_some() {
        assert_matched_parity(contract, input, case_kind);
        return;
    }

    let canonical = parse(input, contract.rule);
    assert!(
        !canonical.matched,
        "canonical {} {case_kind} unexpectedly accepted {input:?}",
        contract.name
    );
    assert!(
        canonical.diagnostics.is_empty(),
        "canonical {} {case_kind} emitted diagnostics for {input:?}",
        contract.name
    );
    assert_eq!(
        canonical.consumed,
        TextRange::empty(TextSize::ZERO),
        "canonical {} {case_kind} must rewind on {input:?}",
        contract.name,
    );
}

fn assert_nonleaf_matched_parity(contract: &NonLeafContract, input: &str, case_kind: &str) {
    let canonical = parse(input, contract.rule);
    let legacy = legacy_nonleaf_match(input, contract.parser).unwrap_or_else(|| {
        panic!(
            "legacy {} {case_kind} unexpectedly rejected {input:?}",
            contract.name
        )
    });

    assert!(
        canonical.matched,
        "canonical {} {case_kind} unexpectedly rejected {input:?}",
        contract.name
    );
    assert!(
        canonical.is_strictly_clean(),
        "canonical {} {case_kind} emitted diagnostics for {input:?}",
        contract.name
    );
    assert_eq!(
        canonical.consumed.start,
        TextSize::ZERO,
        "{} {case_kind} start extent on {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.end, legacy.prefix.consumed,
        "{} {case_kind} consumed extent on {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.source.byte_len().0 - canonical.consumed.end.0,
        legacy.prefix.remaining.0,
        "{} {case_kind} remaining extent on {input:?}",
        contract.name,
    );
    assert_eq!(
        reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed).unwrap(),
        &input[..legacy.prefix.consumed.to_usize()],
        "{} {case_kind} must retain exactly the legacy prefix for {input:?}",
        contract.name,
    );

    let lowered = match contract.kind {
        Some(kind) => lower_nonleaf_value(&canonical.syntax(), kind),
        None => LegacyNonLeafValue::Transparent,
    };
    assert_eq!(
        legacy.value, contract.expected,
        "legacy {} value on {input:?}",
        contract.name,
    );
    assert_eq!(
        lowered, legacy.value,
        "canonical {} lowering on {input:?}",
        contract.name,
    );
    assert_eq!(
        lowered, contract.expected,
        "canonical {} expected lowering on {input:?}",
        contract.name,
    );
}

fn assert_nonleaf_probe_parity(contract: &NonLeafContract, input: &str, case_kind: &str) {
    if legacy_nonleaf_match(input, contract.parser).is_some() {
        assert_nonleaf_matched_parity(contract, input, case_kind);
        return;
    }

    let canonical = parse(input, contract.rule);
    assert!(
        !canonical.matched,
        "canonical {} {case_kind} unexpectedly accepted {input:?}",
        contract.name
    );
    assert!(
        canonical.diagnostics.is_empty(),
        "canonical {} {case_kind} emitted diagnostics for {input:?}",
        contract.name
    );
    assert_eq!(
        canonical.consumed,
        TextRange::empty(TextSize::ZERO),
        "canonical {} {case_kind} must rewind on {input:?}",
        contract.name,
    );
}

fn legacy_nonleaf_match(input: &str, parser: LegacyNonLeafParser) -> Option<LegacyNonLeafMatch> {
    let graphemes = crate::graphemes::init_tag(input);
    parser(ParseString::new(&graphemes))
        .ok()
        .map(|(remaining, value)| {
            assert!(
                remaining.error_log.is_empty(),
                "legacy parser diagnostics on {input:?}"
            );
            let consumed = graphemes[..remaining.cursor]
                .iter()
                .map(|grapheme| grapheme.len())
                .sum::<usize>();
            let remaining_bytes = graphemes[remaining.cursor..]
                .iter()
                .map(|grapheme| grapheme.len())
                .sum::<usize>();
            LegacyNonLeafMatch {
                value,
                prefix: LegacyPrefix {
                    consumed: TextSize(consumed as u32),
                    remaining: TextSize(remaining_bytes as u32),
                },
            }
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyNonLeafMatch {
    value: LegacyNonLeafValue,
    prefix: LegacyPrefix,
}

fn lower_nonleaf_value(root: &SyntaxNode, kind: SyntaxKind) -> LegacyNonLeafValue {
    let node = find_node(root, kind)
        .unwrap_or_else(|| panic!("direct canonical aggregate did not produce {kind:?}"));
    match kind {
        SyntaxKind::AddSubOperator => LegacyNonLeafValue::Formula(
            lower_legacy_add_sub_operator(&AddSubOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::MulDivOperator => LegacyNonLeafValue::Formula(
            lower_legacy_mul_div_operator(&MulDivOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::PowerOperator => LegacyNonLeafValue::Formula(
            lower_legacy_power_operator(&PowerOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::MatrixOperator => LegacyNonLeafValue::Formula(
            lower_legacy_matrix_operator(&MatrixOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::RangeOperator => LegacyNonLeafValue::Range(
            lower_legacy_range_operator(&RangeOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::ComparisonOperator => LegacyNonLeafValue::Formula(
            lower_legacy_comparison_operator(&ComparisonOperatorSyntax::cast(node).unwrap())
                .unwrap(),
        ),
        SyntaxKind::LogicOperator => LegacyNonLeafValue::Formula(
            lower_legacy_logic_operator(&LogicOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::TableOperator => LegacyNonLeafValue::Formula(
            lower_legacy_table_operator(&TableOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        SyntaxKind::SetOperator => LegacyNonLeafValue::Formula(
            lower_legacy_set_operator(&SetOperatorSyntax::cast(node).unwrap()).unwrap(),
        ),
        _ => panic!("unexpected non-leaf Phase 2D syntax kind {kind:?}"),
    }
}

macro_rules! legacy_operator_parser {
    ($name:ident, $parser:path, $family:ident) => {
        fn $name<'source>(
            input: ParseString<'source>,
        ) -> ParseResult<'source, LegacyOperatorValue> {
            let (input, value) = $parser(input)?;
            Ok((input, LegacyOperatorValue::$family(value)))
        }
    };
}

legacy_operator_parser!(legacy_add, super::add, AddSub);
legacy_operator_parser!(legacy_subtract, super::subtract, AddSub);
legacy_operator_parser!(legacy_raw_subtract, super::raw_subtract, AddSub);
legacy_operator_parser!(legacy_spaced_subtract, super::spaced_subtract, AddSub);
legacy_operator_parser!(legacy_multiply, super::multiply, MulDiv);
legacy_operator_parser!(legacy_divide, super::divide, MulDiv);
legacy_operator_parser!(legacy_modulus, super::modulus, MulDiv);
legacy_operator_parser!(legacy_power, super::power, Power);
legacy_operator_parser!(legacy_matrix_multiply, super::matrix_multiply, Matrix);
legacy_operator_parser!(legacy_matrix_solve, super::matrix_solve, Matrix);
legacy_operator_parser!(legacy_dot_product, super::dot_product, Matrix);
legacy_operator_parser!(legacy_cross_product, super::cross_product, Matrix);
legacy_operator_parser!(legacy_range_inclusive, super::range_inclusive, Range);
legacy_operator_parser!(legacy_range_exclusive, super::range_exclusive, Range);
legacy_operator_parser!(legacy_not_equal, super::not_equal, Comparison);
legacy_operator_parser!(legacy_equal_to, super::equal_to, Comparison);
legacy_operator_parser!(legacy_strict_not_equal, super::strict_not_equal, Comparison);
legacy_operator_parser!(legacy_strict_equal, super::strict_equal, Comparison);
legacy_operator_parser!(legacy_greater_than, super::greater_than, Comparison);
legacy_operator_parser!(legacy_less_than, super::less_than, Comparison);
legacy_operator_parser!(
    legacy_greater_than_equal,
    super::greater_than_equal,
    Comparison
);
legacy_operator_parser!(legacy_less_than_equal, super::less_than_equal, Comparison);
legacy_operator_parser!(legacy_or, super::or, Logic);
legacy_operator_parser!(legacy_and, super::and, Logic);
legacy_operator_parser!(legacy_not, super::not, Logic);
legacy_operator_parser!(legacy_xor, super::xor, Logic);
legacy_operator_parser!(legacy_join, super::join, Table);
legacy_operator_parser!(legacy_left_join, super::left_join, Table);
legacy_operator_parser!(legacy_right_join, super::right_join, Table);
legacy_operator_parser!(legacy_full_join, super::full_join, Table);
legacy_operator_parser!(legacy_left_semi_join, super::left_semi_join, Table);
legacy_operator_parser!(legacy_left_anti_join, super::left_anti_join, Table);
legacy_operator_parser!(legacy_union_op, super::union_op, Set);
legacy_operator_parser!(legacy_intersection, super::intersection, Set);
legacy_operator_parser!(legacy_difference, super::difference, Set);
legacy_operator_parser!(legacy_complement, super::complement, Set);
legacy_operator_parser!(legacy_subset, super::subset, Set);
legacy_operator_parser!(legacy_superset, super::superset, Set);
legacy_operator_parser!(legacy_proper_subset, super::proper_subset, Set);
legacy_operator_parser!(legacy_proper_superset, super::proper_superset, Set);
legacy_operator_parser!(legacy_element_of, super::element_of, Set);
legacy_operator_parser!(legacy_not_element_of, super::not_element_of, Set);
legacy_operator_parser!(
    legacy_symmetric_difference,
    super::symmetric_difference,
    Set
);

macro_rules! legacy_formula_operator_parser {
    ($name:ident, $parser:path) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyNonLeafValue> {
            let (input, value) = $parser(input)?;
            Ok((input, LegacyNonLeafValue::Formula(value)))
        }
    };
}

legacy_formula_operator_parser!(legacy_add_sub_operator, super::add_sub_operator);
legacy_formula_operator_parser!(legacy_mul_div_operator, super::mul_div_operator);
legacy_formula_operator_parser!(legacy_power_operator, super::power_operator);
legacy_formula_operator_parser!(legacy_matrix_operator, super::matrix_operator);
legacy_formula_operator_parser!(legacy_comparison_operator, super::comparison_operator);
legacy_formula_operator_parser!(legacy_logic_operator, super::logic_operator);
legacy_formula_operator_parser!(legacy_table_operator, super::table_operator);
legacy_formula_operator_parser!(legacy_set_operator, super::set_operator);

fn legacy_range_operator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyNonLeafValue> {
    let (input, value) = super::range_operator(input)?;
    Ok((input, LegacyNonLeafValue::Range(value)))
}

fn legacy_transpose<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyNonLeafValue> {
    let (input, ()) = super::transpose(input)?;
    Ok((input, LegacyNonLeafValue::Transparent))
}

macro_rules! leaf_contract {
  (
    $name:literal,
    $rule:ident,
    $kind:ident,
    $parser:ident,
    $expected:expr,
    [$($spelling:expr),+ $(,)?],
    [$($probe:expr),+ $(,)?]
  ) => {
    LeafContract {
      name: $name,
      rule: rules::$rule,
      kind: SyntaxKind::$kind,
      parser: $parser,
      expected: $expected,
      spellings: &[$($spelling),+],
      probes: [$($probe),+],
    }
  };
}

fn phase_2d_leaf_contracts() -> [LeafContract; 43] {
    [
        leaf_contract!(
            "add",
            ADD,
            AddOperation,
            legacy_add,
            LegacyOperatorValue::AddSub(AddSubOp::Add),
            ["+"],
            ["+", "\t+\u{2009}", "+tail", "-", "++"]
        ),
        leaf_contract!(
            "subtract",
            SUBTRACT,
            SubtractOperation,
            legacy_subtract,
            LegacyOperatorValue::AddSub(AddSubOp::Sub),
            ["-", " - "],
            ["-", " - ", "-tail", "--", " - -"]
        ),
        leaf_contract!(
            "raw-subtract",
            RAW_SUBTRACT,
            RawSubtractOperation,
            legacy_raw_subtract,
            LegacyOperatorValue::AddSub(AddSubOp::Sub),
            ["-"],
            ["-", "-tail", "-value", "--", " -"]
        ),
        leaf_contract!(
            "spaced-subtract",
            SPACED_SUBTRACT,
            SpacedSubtractOperation,
            legacy_spaced_subtract,
            LegacyOperatorValue::AddSub(AddSubOp::Sub),
            [" - "],
            [" - ", "\t-\u{2009}", " - tail", "-", " -- "]
        ),
        leaf_contract!(
            "multiply",
            MULTIPLY,
            MultiplyOperation,
            legacy_multiply,
            LegacyOperatorValue::MulDiv(MulDivOp::Mul),
            ["*", "×"],
            ["*", "\t×\u{2009}", "*tail", "?", "**"]
        ),
        leaf_contract!(
            "divide",
            DIVIDE,
            DivideOperation,
            legacy_divide,
            LegacyOperatorValue::MulDiv(MulDivOp::Div),
            ["/", "÷"],
            ["/", "\u{00A0}÷\u{2009}", "/tail", "?", "//"]
        ),
        leaf_contract!(
            "modulus",
            MODULUS,
            ModulusOperation,
            legacy_modulus,
            LegacyOperatorValue::MulDiv(MulDivOp::Mod),
            ["%"],
            ["%", " \t%\u{00A0}", "%tail", "?", "%%"]
        ),
        leaf_contract!(
            "power",
            POWER,
            PowerOperation,
            legacy_power,
            LegacyOperatorValue::Power(PowerOp::Pow),
            ["^"],
            ["^", "\t^\u{2009}", "^tail", "?", "^^"]
        ),
        leaf_contract!(
            "matrix-multiply",
            MATRIX_MULTIPLY,
            MatrixMultiplyOperation,
            legacy_matrix_multiply,
            LegacyOperatorValue::Matrix(VecOp::MatMul),
            ["**"],
            ["**", " \t**\u{2009}", "**tail", "*", "***"]
        ),
        leaf_contract!(
            "matrix-solve",
            MATRIX_SOLVE,
            MatrixSolveOperation,
            legacy_matrix_solve,
            LegacyOperatorValue::Matrix(VecOp::Solve),
            ["\\"],
            ["\\", " \\\t", "\\tail", "?", "\\\\"]
        ),
        leaf_contract!(
            "dot-product",
            DOT_PRODUCT,
            DotProductOperation,
            legacy_dot_product,
            LegacyOperatorValue::Matrix(VecOp::Dot),
            ["·", "•"],
            ["·", "\t•\u{2009}", "·tail", "?", "··"]
        ),
        leaf_contract!(
            "cross-product",
            CROSS_PRODUCT,
            CrossProductOperation,
            legacy_cross_product,
            LegacyOperatorValue::Matrix(VecOp::Cross),
            ["⨯"],
            ["⨯", "\t⨯\u{2009}", "⨯tail", "?", "⨯⨯"]
        ),
        leaf_contract!(
            "range-inclusive",
            RANGE_INCLUSIVE,
            RangeInclusiveOperation,
            legacy_range_inclusive,
            LegacyOperatorValue::Range(RangeOp::Inclusive),
            ["..="],
            ["..=", "..=\u{2009}", "..=tail", "..", "...="]
        ),
        leaf_contract!(
            "range-exclusive",
            RANGE_EXCLUSIVE,
            RangeExclusiveOperation,
            legacy_range_exclusive,
            LegacyOperatorValue::Range(RangeOp::Exclusive),
            [".."],
            ["..", "..\u{2009}", "..tail", ".", "..="]
        ),
        leaf_contract!(
            "not-equal",
            NOT_EQUAL,
            NotEqualOperation,
            legacy_not_equal,
            LegacyOperatorValue::Comparison(ComparisonOp::NotEqual),
            ["!=", "¬=", "≠"],
            ["!=", "\t≠\u{2009}", "!=tail", "!", "!=="]
        ),
        leaf_contract!(
            "equal-to",
            EQUAL_TO,
            EqualToOperation,
            legacy_equal_to,
            LegacyOperatorValue::Comparison(ComparisonOp::Equal),
            ["==", "⩵"],
            ["==", " ⩵ ", "==tail", "=", "==="]
        ),
        leaf_contract!(
            "strict-not-equal",
            STRICT_NOT_EQUAL,
            StrictNotEqualOperation,
            legacy_strict_not_equal,
            LegacyOperatorValue::Comparison(ComparisonOp::StrictNotEqual),
            ["!==", "!≡", "¬≡", "¬=="],
            ["!==", " ¬== ", "!==tail", "!=", "!===="]
        ),
        leaf_contract!(
            "strict-equal",
            STRICT_EQUAL,
            StrictEqualOperation,
            legacy_strict_equal,
            LegacyOperatorValue::Comparison(ComparisonOp::StrictEqual),
            ["===", "≡"],
            ["===", "\t≡\u{2009}", "===tail", "==", "===="]
        ),
        leaf_contract!(
            "greater-than",
            GREATER_THAN,
            GreaterThanOperation,
            legacy_greater_than,
            LegacyOperatorValue::Comparison(ComparisonOp::GreaterThan),
            [">"],
            [">", "\t>\u{2009}", ">tail", "<", ">="]
        ),
        leaf_contract!(
            "less-than",
            LESS_THAN,
            LessThanOperation,
            legacy_less_than,
            LegacyOperatorValue::Comparison(ComparisonOp::LessThan),
            ["<"],
            ["<", "\t<\u{2009}", "<tail", ">", "<-"]
        ),
        leaf_contract!(
            "greater-than-equal",
            GREATER_THAN_EQUAL,
            GreaterThanEqualOperation,
            legacy_greater_than_equal,
            LegacyOperatorValue::Comparison(ComparisonOp::GreaterThanEqual),
            [">=", "≥"],
            [">=", "\t≥\u{2009}", ">=tail", ">", ">=="]
        ),
        leaf_contract!(
            "less-than-equal",
            LESS_THAN_EQUAL,
            LessThanEqualOperation,
            legacy_less_than_equal,
            LegacyOperatorValue::Comparison(ComparisonOp::LessThanEqual),
            ["<=", "≤"],
            ["<=", "\t≤\u{2009}", "<=tail", "<", "<=="]
        ),
        leaf_contract!(
            "or",
            OR,
            OrOperation,
            legacy_or,
            LegacyOperatorValue::Logic(LogicOp::Or),
            ["||", "∨", "⋁"],
            ["||", "\t∨\u{2009}", "||tail", "|", "|||"]
        ),
        leaf_contract!(
            "and",
            AND,
            AndOperation,
            legacy_and,
            LegacyOperatorValue::Logic(LogicOp::And),
            ["&&", "∧", "⋀"],
            ["&&", "\t∧\u{2009}", "&&tail", "&", "&&&"]
        ),
        leaf_contract!(
            "not",
            NOT,
            NotOperation,
            legacy_not,
            LegacyOperatorValue::Logic(LogicOp::Not),
            ["!", "¬"],
            ["!", "¬", "!tail", "?", "!!"]
        ),
        leaf_contract!(
            "xor",
            XOR,
            XorOperation,
            legacy_xor,
            LegacyOperatorValue::Logic(LogicOp::Xor),
            ["^^", "⊕", "⊻"],
            ["^^", "\t⊕\u{2009}", "^^tail", "^", "^^^"]
        ),
        leaf_contract!(
            "join",
            JOIN,
            JoinOperation,
            legacy_join,
            LegacyOperatorValue::Table(TableOp::InnerJoin),
            ["⋈"],
            ["⋈", "\t⋈\u{2009}", "⋈tail", "?", "⋈⋈"]
        ),
        leaf_contract!(
            "left-join",
            LEFT_JOIN,
            LeftJoinOperation,
            legacy_left_join,
            LegacyOperatorValue::Table(TableOp::LeftOuterJoin),
            ["⟕"],
            ["⟕", "\t⟕\u{2009}", "⟕tail", "?", "⟕⟕"]
        ),
        leaf_contract!(
            "right-join",
            RIGHT_JOIN,
            RightJoinOperation,
            legacy_right_join,
            LegacyOperatorValue::Table(TableOp::RightOuterJoin),
            ["⟖"],
            ["⟖", "\t⟖\u{2009}", "⟖tail", "?", "⟖⟖"]
        ),
        leaf_contract!(
            "full-join",
            FULL_JOIN,
            FullJoinOperation,
            legacy_full_join,
            LegacyOperatorValue::Table(TableOp::FullOuterJoin),
            ["⟗"],
            ["⟗", "\t⟗\u{2009}", "⟗tail", "?", "⟗⟗"]
        ),
        leaf_contract!(
            "left-semi-join",
            LEFT_SEMI_JOIN,
            LeftSemiJoinOperation,
            legacy_left_semi_join,
            LegacyOperatorValue::Table(TableOp::LeftSemiJoin),
            ["⋉"],
            ["⋉", "\t⋉\u{2009}", "⋉tail", "?", "⋉⋉"]
        ),
        leaf_contract!(
            "left-anti-join",
            LEFT_ANTI_JOIN,
            LeftAntiJoinOperation,
            legacy_left_anti_join,
            LegacyOperatorValue::Table(TableOp::LeftAntiJoin),
            ["▷"],
            ["▷", "\t▷\u{2009}", "▷tail", "?", "▷▷"]
        ),
        leaf_contract!(
            "union-op",
            UNION_OP,
            UnionOperation,
            legacy_union_op,
            LegacyOperatorValue::Set(SetOp::Union),
            ["∪"],
            ["∪", "\t∪\u{2009}", "∪tail", "?", "∪∪"]
        ),
        leaf_contract!(
            "intersection",
            INTERSECTION,
            IntersectionOperation,
            legacy_intersection,
            LegacyOperatorValue::Set(SetOp::Intersection),
            ["∩"],
            ["∩", "\t∩\u{2009}", "∩tail", "?", "∩∩"]
        ),
        leaf_contract!(
            "difference",
            DIFFERENCE,
            DifferenceOperation,
            legacy_difference,
            LegacyOperatorValue::Set(SetOp::Difference),
            ["∖"],
            ["∖", "\t∖\u{2009}", "∖tail", "?", "∖∖"]
        ),
        leaf_contract!(
            "complement",
            COMPLEMENT,
            ComplementOperation,
            legacy_complement,
            LegacyOperatorValue::Set(SetOp::Complement),
            ["∁"],
            ["∁", "\t∁\u{2009}", "∁tail", "?", "∁∁"]
        ),
        leaf_contract!(
            "subset",
            SUBSET,
            SubsetOperation,
            legacy_subset,
            LegacyOperatorValue::Set(SetOp::Subset),
            ["⊆"],
            ["⊆", "\t⊆\u{2009}", "⊆tail", "?", "⊆⊆"]
        ),
        leaf_contract!(
            "superset",
            SUPERSET,
            SupersetOperation,
            legacy_superset,
            LegacyOperatorValue::Set(SetOp::Superset),
            ["⊇"],
            ["⊇", "\t⊇\u{2009}", "⊇tail", "?", "⊇⊇"]
        ),
        leaf_contract!(
            "proper-subset",
            PROPER_SUBSET,
            ProperSubsetOperation,
            legacy_proper_subset,
            LegacyOperatorValue::Set(SetOp::ProperSubset),
            ["⊊", "⊂"],
            ["⊊", "\t⊂\u{2009}", "⊊tail", "?", "⊊⊊"]
        ),
        leaf_contract!(
            "proper-superset",
            PROPER_SUPERSET,
            ProperSupersetOperation,
            legacy_proper_superset,
            LegacyOperatorValue::Set(SetOp::ProperSuperset),
            ["⊋", "⊃"],
            ["⊋", "\t⊃\u{2009}", "⊋tail", "?", "⊋⊋"]
        ),
        leaf_contract!(
            "element-of",
            ELEMENT_OF,
            ElementOfOperation,
            legacy_element_of,
            LegacyOperatorValue::Set(SetOp::ElementOf),
            ["∈"],
            ["∈", "\t∈\u{2009}", "∈tail", "?", "∈∈"]
        ),
        leaf_contract!(
            "not-element-of",
            NOT_ELEMENT_OF,
            NotElementOfOperation,
            legacy_not_element_of,
            LegacyOperatorValue::Set(SetOp::NotElementOf),
            ["∉"],
            ["∉", "\t∉\u{2009}", "∉tail", "?", "∉∉"]
        ),
        leaf_contract!(
            "symmetric-difference",
            SYMMETRIC_DIFFERENCE,
            SymmetricDifferenceOperation,
            legacy_symmetric_difference,
            LegacyOperatorValue::Set(SetOp::SymmetricDifference),
            [" Δ ", "\tΔ\u{2009}"],
            [" Δ ", "\tΔ\u{2009}", " Δ tail", "Δ", " Δ Δ "]
        ),
    ]
}

fn phase_2d_nonleaf_contracts() -> [NonLeafContract; 10] {
    [
        NonLeafContract {
            name: "add-sub-operator",
            rule: rules::ADD_SUB_OPERATOR,
            kind: Some(SyntaxKind::AddSubOperator),
            parser: legacy_add_sub_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::AddSub(AddSubOp::Add)),
            probes: ["+", " \t+\u{2009}", "+tail", "?", "++"],
        },
        NonLeafContract {
            name: "mul-div-operator",
            rule: rules::MUL_DIV_OPERATOR,
            kind: Some(SyntaxKind::MulDivOperator),
            parser: legacy_mul_div_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::MulDiv(MulDivOp::Mul)),
            probes: ["*", " \t*\u{2009}", "*tail", "?", "**"],
        },
        NonLeafContract {
            name: "power-operator",
            rule: rules::POWER_OPERATOR,
            kind: Some(SyntaxKind::PowerOperator),
            parser: legacy_power_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::Power(PowerOp::Pow)),
            probes: ["^", " \t^\u{2009}", "^tail", "?", "^^"],
        },
        NonLeafContract {
            name: "matrix-operator",
            rule: rules::MATRIX_OPERATOR,
            kind: Some(SyntaxKind::MatrixOperator),
            parser: legacy_matrix_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::Vec(VecOp::MatMul)),
            probes: ["**", " \t**\u{2009}", "**tail", "*", "***"],
        },
        NonLeafContract {
            name: "range-operator",
            rule: rules::RANGE_OPERATOR,
            kind: Some(SyntaxKind::RangeOperator),
            parser: legacy_range_operator,
            expected: LegacyNonLeafValue::Range(RangeOp::Inclusive),
            probes: ["..=", "..=\u{2009}", "..=tail", ".", "..=="],
        },
        NonLeafContract {
            name: "comparison-operator",
            rule: rules::COMPARISON_OPERATOR,
            kind: Some(SyntaxKind::ComparisonOperator),
            parser: legacy_comparison_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::Comparison(
                ComparisonOp::StrictNotEqual,
            )),
            probes: ["!==", " \t!==\u{2009}", "!==tail", "!", "!===="],
        },
        NonLeafContract {
            name: "logic-operator",
            rule: rules::LOGIC_OPERATOR,
            kind: Some(SyntaxKind::LogicOperator),
            parser: legacy_logic_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::Logic(LogicOp::And)),
            probes: ["&&", " \t&&\u{2009}", "&&tail", "!", "&&&"],
        },
        NonLeafContract {
            name: "table-operator",
            rule: rules::TABLE_OPERATOR,
            kind: Some(SyntaxKind::TableOperator),
            parser: legacy_table_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::Table(TableOp::InnerJoin)),
            probes: ["⋈", " \t⋈\u{2009}", "⋈tail", "?", "⋈⋈"],
        },
        NonLeafContract {
            name: "set-operator",
            rule: rules::SET_OPERATOR,
            kind: Some(SyntaxKind::SetOperator),
            parser: legacy_set_operator,
            expected: LegacyNonLeafValue::Formula(FormulaOperator::Set(SetOp::Union)),
            probes: ["∪", " \t∪\u{2009}", "∪tail", "?", "∪∪"],
        },
        NonLeafContract {
            name: "transpose",
            rule: rules::TRANSPOSE,
            kind: None,
            parser: legacy_transpose,
            expected: LegacyNonLeafValue::Transparent,
            probes: ["'", "'\u{2009}", "'tail", "?", "''"],
        },
    ]
}

fn phase_2d_direct_contracts() -> Vec<DirectContract> {
    let mut contracts = phase_2d_leaf_contracts()
        .into_iter()
        .map(DirectContract::Leaf)
        .collect::<Vec<_>>();
    contracts.extend(
        phase_2d_nonleaf_contracts()
            .into_iter()
            .map(DirectContract::NonLeaf),
    );
    contracts
}

fn direct_contract_rule(contract: &DirectContract) -> RuleId {
    match contract {
        DirectContract::Leaf(contract) => contract.rule,
        DirectContract::NonLeaf(contract) => contract.rule,
    }
}

const PROBE_KINDS: [&str; 5] = [
    "minimal success",
    "representative success",
    "valid prefix with remainder",
    "boundary rejection",
    "overlapping or ambiguous spelling",
];

#[test]
fn all_phase_2d_direct_rules_match_legacy_across_265_probes() {
    let contracts = phase_2d_direct_contracts();
    assert_eq!(contracts.len(), 53);
    assert_eq!(PHASE_2D_OPERATOR_RULES.len(), 53);
    for rule in PHASE_2D_OPERATOR_RULES {
        assert_eq!(
            contracts
                .iter()
                .filter(|contract| direct_contract_rule(contract) == *rule)
                .count(),
            1,
            "direct differential table must cover {rule:?} exactly once",
        );
    }
    let mut direct_cases = 0;
    for contract in &contracts {
        match contract {
            DirectContract::Leaf(contract) => {
                for (case_kind, input) in PROBE_KINDS.into_iter().zip(contract.probes) {
                    assert_probe_parity(contract, input, case_kind);
                    direct_cases += 1;
                }
            }
            DirectContract::NonLeaf(contract) => {
                for (case_kind, input) in PROBE_KINDS.into_iter().zip(contract.probes) {
                    assert_nonleaf_probe_parity(contract, input, case_kind);
                    direct_cases += 1;
                }
            }
        }
    }
    assert_eq!(direct_cases, 265);
}

#[test]
fn all_phase_2d_node_valued_direct_rules_have_exact_lowering_contracts() {
    let contracts = phase_2d_leaf_contracts();
    assert_eq!(contracts.len(), 43);
    for contract in &contracts {
        for spelling in contract.spellings {
            assert_matched_parity(contract, spelling, "node-valued lowering spelling");
        }
    }
}

#[test]
fn every_accepted_phase_2d_operator_spelling_has_exact_value_and_extent() {
    let contracts = phase_2d_leaf_contracts();
    assert_eq!(contracts.len(), 43);
    for contract in &contracts {
        for spelling in contract.spellings {
            assert_matched_parity(contract, spelling, "accepted spelling");
        }
    }
}

#[test]
fn private_table_operator_functions_match_canonical_values_and_extents() {
    let contracts = phase_2d_leaf_contracts();
    let table_names = [
        "join",
        "left-join",
        "right-join",
        "full-join",
        "left-semi-join",
        "left-anti-join",
    ];
    let table_contracts = contracts
        .iter()
        .filter(|contract| table_names.contains(&contract.name))
        .collect::<Vec<_>>();
    assert_eq!(table_contracts.len(), 6);
    for contract in table_contracts {
        for spelling in contract.spellings {
            assert_matched_parity(contract, spelling, "private table spelling");
        }
        for (case_kind, input) in PROBE_KINDS.into_iter().zip(contract.probes) {
            assert_probe_parity(contract, input, case_kind);
        }
    }
}
