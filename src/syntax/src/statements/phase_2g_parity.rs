//! Direct parity coverage for the closed Phase 2G executable primitives.
//!
//! The complete subscript, pattern, statement, expression, and state-machine
//! parents remain deliberately out of scope. These tests exercise only the
//! fifteen direct leaves through the hidden source-fragment harness.

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{OpAssignOp, Pattern, Subscript};

use crate::document::ast::{
    OpAssignPrimitiveSyntax, SubscriptPrimitiveSyntax, WildcardPatternSyntax,
};
use crate::document::lower::legacy::{
    LegacyControlValue, LegacyPatternPrimitiveValue, LegacySubscriptPrimitiveValue,
    lower_phase_2g_control_value, lower_phase_2g_pattern_value, lower_phase_2g_subscript_value,
};
use crate::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, PHASE_2G_RULES,
    parse_canonical_phase_2g_rule_for_test,
};
use crate::document::parser::rules;
use crate::document::{
    AstNode, DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode,
    TextRange, TextSize, TextSnapshot, reconstruct_source_range, validate_lossless_range,
};
use crate::{ParseResult, ParseString};
use nom::Err;

#[derive(Clone, Debug, Eq, PartialEq)]
enum LegacyValue {
    Subscript(Subscript),
    Pattern(Pattern),
    Assignment(OpAssignOp),
    Transparent,
}

type LegacyParser = for<'source> fn(ParseString<'source>) -> ParseResult<'source, LegacyValue>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Prefix {
    consumed: TextSize,
    remaining: TextSize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyMatch {
    value: LegacyValue,
    prefix: Prefix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegacyOutcome {
    Matched,
    Error,
    Failure,
}

#[derive(Clone, Copy)]
enum CanonicalValue {
    Subscript(SyntaxKind),
    Pattern(SyntaxKind),
    Assignment(SyntaxKind),
    Transparent(SyntaxKind),
}

#[derive(Clone, Copy)]
struct Contract {
    name: &'static str,
    rule: RuleId,
    value: CanonicalValue,
    parser: LegacyParser,
    probes: [&'static str; 5],
}

fn legacy_statement_separator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyValue> {
    let (input, _) = crate::expressions::statement_separator(input)?;
    Ok((input, LegacyValue::Transparent))
}

fn legacy_select_all<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, value) = crate::expressions::select_all(input)?;
    Ok((input, LegacyValue::Subscript(value)))
}

fn legacy_swizzle_subscript<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyValue> {
    let (input, value) = crate::expressions::swizzle_subscript(input)?;
    Ok((input, LegacyValue::Subscript(value)))
}

fn legacy_dot_subscript<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, value) = crate::expressions::dot_subscript(input)?;
    Ok((input, LegacyValue::Subscript(value)))
}

fn legacy_dot_subscript_int<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyValue> {
    let (input, value) = crate::expressions::dot_subscript_int(input)?;
    Ok((input, LegacyValue::Subscript(value)))
}

fn legacy_wildcard<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, value) = crate::patterns::wildcard(input)?;
    Ok((input, LegacyValue::Pattern(value)))
}

fn legacy_spread_operator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyValue> {
    let (input, _) = crate::patterns::spread_operator(input)?;
    Ok((input, LegacyValue::Transparent))
}

fn legacy_op_assign_operator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyValue> {
    let (input, value) = super::op_assign_operator(input)?;
    Ok((input, LegacyValue::Assignment(value)))
}

macro_rules! legacy_assignment_leaf {
    ($name:ident, $parser:ident) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
            let (input, value) = super::$parser(input)?;
            Ok((input, LegacyValue::Assignment(value)))
        }
    };
}

legacy_assignment_leaf!(legacy_add_assign_operator, add_assign_operator);
legacy_assignment_leaf!(legacy_sub_assign_operator, sub_assign_operator);
legacy_assignment_leaf!(legacy_mul_assign_operator, mul_assign_operator);
legacy_assignment_leaf!(legacy_div_assign_operator, div_assign_operator);
legacy_assignment_leaf!(legacy_exp_assign_operator, exp_assign_operator);

fn legacy_send_operator<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, _) = super::send_operator(input)?;
    Ok((input, LegacyValue::Transparent))
}

fn legacy_guard_operator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, LegacyValue> {
    let (input, _) = crate::state_machines::guard_operator(input)?;
    Ok((input, LegacyValue::Transparent))
}

fn phase_2g_contracts() -> [Contract; 15] {
    [
        Contract {
            name: "statement-separator",
            rule: rules::STATEMENT_SEPARATOR,
            value: CanonicalValue::Transparent(SyntaxKind::Semicolon),
            parser: legacy_statement_separator,
            probes: [";", " ; ", "\n;\r", "x", "；"],
        },
        Contract {
            name: "select-all",
            rule: rules::SELECT_ALL,
            value: CanonicalValue::Subscript(SyntaxKind::SelectAllSubscript),
            parser: legacy_select_all,
            probes: [":", ":rest", "", "[", "："],
        },
        Contract {
            name: "swizzle-subscript",
            rule: rules::SWIZZLE_SUBSCRIPT,
            value: CanonicalValue::Subscript(SyntaxKind::SwizzleSubscript),
            parser: legacy_swizzle_subscript,
            probes: [".x,y", ".x,y,z", ".x,y,", ".x", ".x,"],
        },
        Contract {
            name: "dot-subscript",
            rule: rules::DOT_SUBSCRIPT,
            value: CanonicalValue::Subscript(SyntaxKind::DotSubscript),
            parser: legacy_dot_subscript,
            probes: [".x", ".alpha2", ".x,y", ".42", "."],
        },
        Contract {
            name: "dot-subscript-int",
            rule: rules::DOT_SUBSCRIPT_INT,
            value: CanonicalValue::Subscript(SyntaxKind::DotSubscriptInt),
            parser: legacy_dot_subscript_int,
            probes: [".42", ".1u8", ".1e3", ".1x", "."],
        },
        Contract {
            name: "wildcard",
            rule: rules::WILDCARD,
            value: CanonicalValue::Pattern(SyntaxKind::WildcardPattern),
            parser: legacy_wildcard,
            probes: ["*", "**", "*x", "", "…"],
        },
        Contract {
            name: "spread-operator",
            rule: rules::SPREAD_OPERATOR,
            value: CanonicalValue::Transparent(SyntaxKind::SpreadOperator),
            parser: legacy_spread_operator,
            probes: ["...", "…", "\n...\r", "...rest", ".."],
        },
        Contract {
            name: "op-assign-operator",
            rule: rules::OP_ASSIGN_OPERATOR,
            value: CanonicalValue::Assignment(SyntaxKind::OpAssignOperator),
            parser: legacy_op_assign_operator,
            probes: ["+=", "-=", "*=", "/=", "^="],
        },
        Contract {
            name: "add-assign-operator",
            rule: rules::ADD_ASSIGN_OPERATOR,
            value: CanonicalValue::Assignment(SyntaxKind::AddAssignOperation),
            parser: legacy_add_assign_operator,
            probes: ["+=", "\n+=\r", " +=x", "+", "="],
        },
        Contract {
            name: "sub-assign-operator",
            rule: rules::SUB_ASSIGN_OPERATOR,
            value: CanonicalValue::Assignment(SyntaxKind::SubAssignOperation),
            parser: legacy_sub_assign_operator,
            probes: ["-=", "\t-=\n", " -=x", "-", "="],
        },
        Contract {
            name: "mul-assign-operator",
            rule: rules::MUL_ASSIGN_OPERATOR,
            value: CanonicalValue::Assignment(SyntaxKind::MulAssignOperation),
            parser: legacy_mul_assign_operator,
            probes: ["*=", "\n*=\r", " *=x", "*", "="],
        },
        Contract {
            name: "div-assign-operator",
            rule: rules::DIV_ASSIGN_OPERATOR,
            value: CanonicalValue::Assignment(SyntaxKind::DivAssignOperation),
            parser: legacy_div_assign_operator,
            probes: ["/=", "\n/=\r", " /=x", "/", "="],
        },
        Contract {
            name: "exp-assign-operator",
            rule: rules::EXP_ASSIGN_OPERATOR,
            value: CanonicalValue::Assignment(SyntaxKind::ExpAssignOperation),
            parser: legacy_exp_assign_operator,
            probes: ["^=", "\n^=\r", " ^=x", "^", "="],
        },
        Contract {
            name: "send-operator",
            rule: rules::SEND_OPERATOR,
            value: CanonicalValue::Transparent(SyntaxKind::Text),
            parser: legacy_send_operator,
            probes: ["<-", "\n<-\r", " <- x", "<", "←"],
        },
        Contract {
            name: "guard-operator",
            rule: rules::GUARD_OPERATOR,
            value: CanonicalValue::Transparent(SyntaxKind::Bar),
            parser: legacy_guard_operator,
            probes: [" | ", " │ ", " ├ ", " └ ", " x "],
        },
    ]
}

fn legacy_match(input: &str, parser: LegacyParser) -> Result<LegacyMatch, LegacyOutcome> {
    let graphemes = crate::graphemes::init_source(input);
    let input_len = TextSize::from_u32(input.len() as u32);
    let prefix = |cursor: usize| {
        let consumed = TextSize::from_u32(
            graphemes[..cursor]
                .iter()
                .map(|grapheme| grapheme.len() as u32)
                .sum(),
        );
        // The legacy source helper appends a newline sentinel. Phase 2G owns
        // `whitespace0` around several leaves, so a successful direct parse
        // may consume that sentinel after consuming the entire physical probe.
        // The canonical source fragment has no sentinel; compare only the
        // physical extent.
        let consumed = consumed.min(input_len);
        Prefix {
            consumed,
            remaining: input_len - consumed,
        }
    };
    match parser(ParseString::new(&graphemes)) {
        Ok((remaining, value)) => {
            assert!(
                remaining.error_log.is_empty(),
                "legacy parser recorded errors after accepting {input:?}: {:?}",
                remaining.error_log,
            );
            Ok(LegacyMatch {
                value,
                prefix: prefix(remaining.cursor),
            })
        }
        Err(Err::Error(_)) => Err(LegacyOutcome::Error),
        Err(Err::Failure(_)) => Err(LegacyOutcome::Failure),
        Err(Err::Incomplete(_)) => {
            panic!("legacy Phase 2G parser requested more input for {input:?}")
        }
    }
}

fn canonical_snapshot(
    input: &str,
    rule: RuleId,
    config: ParseConfig,
) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(0x2a), Revision(0), input)
        .expect("direct parity probe must form a source snapshot");
    parse_canonical_phase_2g_rule_for_test(source, rule, config)
        .expect("every Phase 2G direct parity rule must be supported")
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn canonical_value(contract: Contract, canonical: &CanonicalSourceRuleSnapshot) -> LegacyValue {
    match contract.value {
        CanonicalValue::Subscript(kind) => {
            let node = find_node(&canonical.syntax(), kind)
                .expect("matched subscript leaf must retain its syntax node");
            match lower_phase_2g_subscript_value(&SubscriptPrimitiveSyntax::cast(node).unwrap())
                .unwrap()
            {
                LegacySubscriptPrimitiveValue::SelectAll(value)
                | LegacySubscriptPrimitiveValue::Swizzle(value)
                | LegacySubscriptPrimitiveValue::Dot(value)
                | LegacySubscriptPrimitiveValue::DotInt(value) => LegacyValue::Subscript(value),
            }
        }
        CanonicalValue::Pattern(kind) => {
            let node = find_node(&canonical.syntax(), kind)
                .expect("matched pattern leaf must retain its syntax node");
            match lower_phase_2g_pattern_value(&WildcardPatternSyntax::cast(node).unwrap()).unwrap()
            {
                LegacyPatternPrimitiveValue::Wildcard(value) => LegacyValue::Pattern(value),
            }
        }
        CanonicalValue::Assignment(kind) => {
            let node = find_node(&canonical.syntax(), kind)
                .expect("matched assignment leaf must retain its syntax node");
            match lower_phase_2g_control_value(&OpAssignPrimitiveSyntax::cast(node).unwrap())
                .unwrap()
            {
                LegacyControlValue::Operator(value)
                | LegacyControlValue::Add(value)
                | LegacyControlValue::Sub(value)
                | LegacyControlValue::Mul(value)
                | LegacyControlValue::Div(value)
                | LegacyControlValue::Exp(value) => LegacyValue::Assignment(value),
            }
        }
        CanonicalValue::Transparent(token_kind) => {
            assert!(
                canonical
                    .syntax()
                    .tokens()
                    .iter()
                    .any(|token| token.kind() == token_kind)
                    || (token_kind == SyntaxKind::Bar
                        && canonical
                            .syntax()
                            .tokens()
                            .iter()
                            .any(|token| token.kind() == SyntaxKind::BoxDrawing)),
                "transparent {} must retain its central physical token",
                contract.name,
            );
            LegacyValue::Transparent
        }
    }
}

fn assert_parity(contract: Contract, input: &str) {
    let canonical = canonical_snapshot(input, contract.rule, ParseConfig::default());
    match legacy_match(input, contract.parser) {
        Ok(legacy) => {
            assert_eq!(canonical.outcome, CanonicalRuleOutcome::Matched);
            assert!(
                canonical.is_strictly_clean(),
                "{} on {input:?}",
                contract.name
            );
            assert_eq!(canonical.consumed.start, TextSize::ZERO);
            assert_eq!(
                canonical.consumed.end, legacy.prefix.consumed,
                "{} consumed a different prefix for {input:?}",
                contract.name,
            );
            assert_eq!(
                canonical.source.byte_len() - canonical.consumed.end,
                legacy.prefix.remaining
            );
            validate_lossless_range(&canonical.root, &canonical.source, canonical.consumed)
                .unwrap();
            assert_eq!(
                reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed)
                    .unwrap(),
                input[..legacy.prefix.consumed.to_usize()],
            );
            assert_eq!(canonical_value(contract, &canonical), legacy.value);
        }
        Err(legacy_outcome) => {
            assert!(matches!(
                legacy_outcome,
                LegacyOutcome::Error | LegacyOutcome::Failure
            ));
            assert_eq!(canonical.outcome, CanonicalRuleOutcome::NoMatch);
            assert!(!canonical.matched);
            assert!(canonical.diagnostics.is_empty());
            assert_eq!(canonical.consumed, TextRange::empty(TextSize::ZERO));
        }
    }
}

#[test]
fn phase_2g_has_exactly_seventy_five_direct_parity_cases() {
    let contracts = phase_2g_contracts();
    assert_eq!(PHASE_2G_RULES.len(), 15);
    assert_eq!(contracts.len(), 15);
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| matches!(contract.value, CanonicalValue::Transparent(_)))
            .count(),
        4,
    );
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| !matches!(contract.value, CanonicalValue::Transparent(_)))
            .count(),
        11,
    );
    for contract in contracts {
        assert_eq!(
            PHASE_2G_RULES
                .iter()
                .filter(|rule| **rule == contract.rule)
                .count(),
            1,
            "{} must appear exactly once in the Phase 2G surface",
            contract.name,
        );
        for probe in contract.probes {
            assert_parity(contract, probe);
        }
    }
}

#[test]
fn phase_2g_regressions_preserve_prefix_and_whitespace_contracts() {
    let contracts = phase_2g_contracts();
    let contract = |rule| {
        contracts
            .iter()
            .copied()
            .find(|contract| contract.rule == rule)
            .unwrap()
    };
    for input in [".x,y", ".x,y,z", ".x,y,", ".x", ".x,"] {
        assert_parity(contract(rules::SWIZZLE_SUBSCRIPT), input);
    }
    for input in [".42", ".1u8", ".1e3"] {
        assert_parity(contract(rules::DOT_SUBSCRIPT_INT), input);
    }
    for input in ["*", "**", "...", "…", "\n...\r"] {
        let rule = if input.contains('.') || input == "…" {
            rules::SPREAD_OPERATOR
        } else {
            rules::WILDCARD
        };
        assert_parity(contract(rule), input);
    }
    for input in ["+", "=", "+=="] {
        assert_parity(contract(rules::ADD_ASSIGN_OPERATOR), input);
    }
    for input in ["\n+=\r", "\t-=\n"] {
        let rule = if input.contains("+=") {
            rules::ADD_ASSIGN_OPERATOR
        } else {
            rules::SUB_ASSIGN_OPERATOR
        };
        assert_parity(contract(rule), input);
    }
    for rule in [
        rules::STATEMENT_SEPARATOR,
        rules::ADD_ASSIGN_OPERATOR,
        rules::SEND_OPERATOR,
        rules::GUARD_OPERATOR,
    ] {
        for input in ["\u{a0}", "\u{2009}"] {
            assert_parity(contract(rule), input);
        }
    }
    for boundary in ["\u{a0}", "\u{2009}"] {
        assert_parity(
            contract(rules::STATEMENT_SEPARATOR),
            &format!("{boundary};"),
        );
        assert_parity(
            contract(rules::ADD_ASSIGN_OPERATOR),
            &format!("{boundary}+="),
        );
        assert_parity(contract(rules::SEND_OPERATOR), &format!("{boundary}<-"));
        assert_parity(contract(rules::GUARD_OPERATOR), &format!("{boundary}|"));
    }
}

#[test]
fn phase_2g_piece_boundaries_preserve_direct_prefixes() {
    let cases: [(&[&str], RuleId); 7] = [
        (&[".", "x", ",", "y"], rules::SWIZZLE_SUBSCRIPT),
        (&[".", "4", "2"], rules::DOT_SUBSCRIPT_INT),
        (&["+", "="], rules::ADD_ASSIGN_OPERATOR),
        (&["<", "-"], rules::SEND_OPERATOR),
        (&[".", ".", "."], rules::SPREAD_OPERATOR),
        (&["├"], rules::GUARD_OPERATOR),
        (&["\r", "\n", ";", "\t"], rules::STATEMENT_SEPARATOR),
    ];
    for (pieces, rule) in cases {
        let input = pieces.concat();
        let contract = phase_2g_contracts()
            .into_iter()
            .find(|contract| contract.rule == rule)
            .unwrap();
        assert_parity(contract, &input);
        let mut source = TextSnapshot::new(DocumentId(0x2a), Revision(0), "").unwrap();
        for piece in pieces {
            source = source.append(*piece).unwrap();
        }
        assert_eq!(source.piece_count(), pieces.len());
        let parsed =
            parse_canonical_phase_2g_rule_for_test(source.clone(), rule, ParseConfig::default())
                .unwrap();
        assert!(parsed.is_strictly_clean(), "piece-backed rule {rule:?}");
        assert_eq!(parsed.consumed, source.full_range());
        validate_lossless_range(&parsed.root, &source, parsed.consumed).unwrap();
    }
}

#[test]
fn phase_2g_fixed_seed_totality_transactionality_and_linear_bounds() {
    let mut state = 0x2a_u64;
    for _ in 0..96 {
        let len = (next(&mut state) % 32) as usize;
        let mut input = String::new();
        for _ in 0..len {
            input.push(random_scalar(&mut state));
        }
        for rule in PHASE_2G_RULES {
            let parsed = canonical_snapshot(&input, *rule, ParseConfig::default());
            assert!(
                parsed.diagnostics.is_empty(),
                "{rule:?} emitted diagnostics for {input:?}"
            );
            if parsed.matched {
                assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
                validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
            } else {
                assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch);
                assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
            }
        }
    }

    let names = core::iter::repeat("x")
        .take(512)
        .collect::<Vec<_>>()
        .join(",");
    let swizzle = format!(".{names}");
    let parsed = canonical_snapshot(&swizzle, rules::SWIZZLE_SUBSCRIPT, ParseConfig::default());
    assert!(parsed.is_strictly_clean());
    assert!(parsed.stats.parser_steps < (swizzle.len() as u64) * 16);

    let whitespace = format!("{};{}", " \t\r\n".repeat(256), "\n\r\t ".repeat(256));
    let parsed = canonical_snapshot(
        &whitespace,
        rules::STATEMENT_SEPARATOR,
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    assert!(parsed.stats.parser_steps < (whitespace.len() as u64) * 12);

    let limits = ParseLimits {
        max_events: 32,
        fuel: 128,
        ..ParseLimits::default()
    };
    let _ = canonical_snapshot(&swizzle, rules::SWIZZLE_SUBSCRIPT, ParseConfig { limits });
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn random_scalar(state: &mut u64) -> char {
    loop {
        let candidate = (next(state) % 0x11_0000) as u32;
        if let Some(character) = char::from_u32(candidate)
            && !(0xd800..=0xdfff).contains(&candidate)
        {
            return character;
        }
    }
}
