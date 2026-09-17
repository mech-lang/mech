//! Direct parity coverage for the closed Phase 2H structure shell.
//!
//! Complete matrix, table, map, set, structure, expression, and document
//! parents remain deliberately absent. These checks exercise only the ten
//! direct leaves through the hidden source-fragment harness.

use super::*;

use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{SourceLocation, SourceRange, Token as LegacyToken, TokenKind};
use unicode_segmentation::UnicodeSegmentation;

use crate::document::ast::{
    EmptyMapSyntax, EmptySetSyntax, MatrixDelimiterStyle, TableRowSeparatorSyntax,
};
use crate::document::lower::legacy::{
    LegacyStructureShellValue, lower_legacy_empty_map, lower_legacy_empty_set,
    lower_legacy_table_row_separator, lower_phase_2h_structure_shell_value,
};
use crate::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, PHASE_2H_RULES,
    parse_canonical_phase_2h_rule_for_test,
};
use crate::document::parser::rules;
use crate::document::{
    AstNode, DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode,
    SyntaxToken, TextRange, TextSize, TextSnapshot, TokenFlags, reconstruct_source_range,
    validate_lossless_range,
};
use crate::{ParseResult, ParseString};
use nom::Err;

#[derive(Clone, Debug, Eq, PartialEq)]
enum LegacyValue {
    Token(LegacyToken),
    Unit,
    Structure(LegacyStructureShellValue),
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
    Error,
    Failure,
}

#[derive(Clone, Copy)]
enum CanonicalValue {
    Token,
    Unit,
    Structure(SyntaxKind),
}

#[derive(Clone, Copy)]
struct Contract {
    name: &'static str,
    rule: RuleId,
    value: CanonicalValue,
    parser: LegacyParser,
    probes: [&'static str; 5],
}

macro_rules! legacy_token_parser {
    ($name:ident, $parser:ident) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
            let (input, value) = super::$parser(input)?;
            Ok((input, LegacyValue::Token(value)))
        }
    };
}

legacy_token_parser!(legacy_matrix_start, matrix_start);
legacy_token_parser!(legacy_matrix_end, matrix_end);
legacy_token_parser!(legacy_table_start, table_start);
legacy_token_parser!(legacy_table_end, table_end);
legacy_token_parser!(legacy_table_separator, table_separator);
legacy_token_parser!(legacy_table_horz, table_horz);

fn legacy_table_top<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, _) = super::table_top(input)?;
    Ok((input, LegacyValue::Unit))
}

fn legacy_row_separator<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, value) = super::row_separator(input)?;
    Ok((
        input,
        LegacyValue::Structure(LegacyStructureShellValue::TableRow(value)),
    ))
}

fn legacy_empty_map<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, value) = super::empty_map(input)?;
    Ok((
        input,
        LegacyValue::Structure(LegacyStructureShellValue::EmptyMap(value)),
    ))
}

fn legacy_empty_set<'source>(input: ParseString<'source>) -> ParseResult<'source, LegacyValue> {
    let (input, value) = super::empty_set(input)?;
    Ok((
        input,
        LegacyValue::Structure(LegacyStructureShellValue::EmptySet(value)),
    ))
}

fn phase_2h_contracts() -> [Contract; 10] {
    [
        Contract {
            name: "matrix-start",
            rule: rules::MATRIX_START,
            value: CanonicalValue::Token,
            parser: legacy_matrix_start,
            probes: ["[", "╭", "┌", "┏", "]"],
        },
        Contract {
            name: "matrix-end",
            rule: rules::MATRIX_END,
            value: CanonicalValue::Token,
            parser: legacy_matrix_end,
            probes: ["]", "╯", "┘", "┛", "["],
        },
        Contract {
            name: "table-start",
            rule: rules::TABLE_START,
            value: CanonicalValue::Token,
            parser: legacy_table_start,
            probes: ["╭", "┌", "┏", "{", " | "],
        },
        Contract {
            name: "table-end",
            rule: rules::TABLE_END,
            value: CanonicalValue::Token,
            parser: legacy_table_end,
            probes: ["╯", "┘", "┛", "}", "\t│\t"],
        },
        Contract {
            name: "table-separator",
            rule: rules::TABLE_SEPARATOR,
            value: CanonicalValue::Token,
            parser: legacy_table_separator,
            probes: ["|", " | ", "\t│\t", "\u{00a0}┃\u{2009}", "\n|\n"],
        },
        Contract {
            name: "table-horz",
            rule: rules::TABLE_HORZ,
            value: CanonicalValue::Token,
            parser: legacy_table_horz,
            probes: ["-", "─", "--", "x", ""],
        },
        Contract {
            name: "table-top",
            rule: rules::TABLE_TOP,
            value: CanonicalValue::Unit,
            parser: legacy_table_top,
            probes: ["╭───\n", "┌───\r", "┏───\r\n", "{───\n", "|───\n"],
        },
        Contract {
            name: "row-separator",
            rule: rules::ROW_SEPARATOR,
            value: CanonicalValue::Structure(SyntaxKind::TableRowSeparator),
            parser: legacy_row_separator,
            probes: ["─", "┘", "┼──┼", "|", "  ───  "],
        },
        Contract {
            name: "empty-map",
            rule: rules::EMPTY_MAP,
            value: CanonicalValue::Structure(SyntaxKind::EmptyMap),
            parser: legacy_empty_map,
            probes: ["{:}", "{ : }", "{\n:\n}", "{", "{a}"],
        },
        Contract {
            name: "empty-set",
            rule: rules::EMPTY_SET,
            value: CanonicalValue::Structure(SyntaxKind::EmptySet),
            parser: legacy_empty_set,
            probes: ["{}", "{_}", "{___}", "{ _ }", "{a}"],
        },
    ]
}

fn legacy_match(input: &str, parser: LegacyParser) -> Result<LegacyMatch, LegacyOutcome> {
    let graphemes = crate::graphemes::init_tag(input);
    let input_len = TextSize::from_u32(input.len() as u32);
    let prefix = |cursor: usize| {
        let consumed = TextSize::from_u32(
            graphemes[..cursor]
                .iter()
                .map(|grapheme| grapheme.len() as u32)
                .sum(),
        );
        assert!(
            consumed <= input_len,
            "legacy Phase 2H parser consumed beyond physical input"
        );
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
            panic!("legacy Phase 2H parser requested more input for {input:?}")
        }
    }
}

fn canonical_snapshot(
    input: &str,
    rule: RuleId,
    config: ParseConfig,
) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(0x2b), Revision(0), input)
        .expect("direct parity probe must form a source snapshot");
    parse_canonical_phase_2h_rule_for_test(source, rule, config)
        .expect("every Phase 2H direct parity rule must be supported")
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn canonical_value(contract: Contract, canonical: &CanonicalSourceRuleSnapshot) -> LegacyValue {
    match contract.value {
        CanonicalValue::Token => {
            let token = semantic_token(contract.rule, &canonical.syntax());
            LegacyValue::Token(lower_direct_token(&canonical.syntax(), &token))
        }
        CanonicalValue::Unit => LegacyValue::Unit,
        CanonicalValue::Structure(kind) => {
            let node = find_node(&canonical.syntax(), kind)
                .expect("matched structure leaf must retain its syntax node");
            LegacyValue::Structure(lower_phase_2h_structure_shell_value(&node).unwrap())
        }
    }
}

fn semantic_token(rule: RuleId, syntax: &SyntaxNode) -> SyntaxToken {
    let tokens = syntax.tokens();
    let token = tokens
        .into_iter()
        .find(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace
                    | SyntaxKind::Tab
                    | SyntaxKind::Newline
                    | SyntaxKind::CarriageReturn
            )
        })
        .unwrap_or_else(|| panic!("{rule:?} must retain a central physical token"));
    match rule {
        rules::MATRIX_START
        | rules::MATRIX_END
        | rules::TABLE_START
        | rules::TABLE_END
        | rules::TABLE_SEPARATOR
        | rules::TABLE_HORZ => token,
        _ => unreachable!("only token-valued Phase 2H rules select a semantic token"),
    }
}

fn lower_direct_token(syntax: &SyntaxNode, token: &SyntaxToken) -> LegacyToken {
    let kind = match token.kind() {
        SyntaxKind::LeftBracket => TokenKind::LeftBracket,
        SyntaxKind::RightBracket => TokenKind::RightBracket,
        SyntaxKind::LeftBrace => TokenKind::LeftBrace,
        SyntaxKind::RightBrace => TokenKind::RightBrace,
        SyntaxKind::BoxDrawing => TokenKind::BoxDrawing,
        SyntaxKind::Bar => TokenKind::Bar,
        SyntaxKind::Dash => TokenKind::Dash,
        other => panic!("unsupported direct Phase 2H token kind {other:?}"),
    };
    LegacyToken {
        kind,
        chars: token.text().unwrap().chars().collect(),
        src_range: source_range(syntax.source(), token.range()),
    }
}

fn source_range(source: &TextSnapshot, range: TextRange) -> SourceRange {
    SourceRange {
        start: source_location(source, range.start),
        end: source_location(source, range.end),
    }
}

fn source_location(source: &TextSnapshot, offset: TextSize) -> SourceLocation {
    let line = source.line_index().line_of(offset);
    let line_start = source
        .line_index()
        .line_start(line)
        .expect("source line index must resolve every valid token offset");
    let text = source
        .text(TextRange::new(line_start, offset))
        .expect("token offset must delimit valid source text");
    SourceLocation {
        row: line + 1,
        col: text.graphemes(true).count() + 1,
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
        Err(LegacyOutcome::Error | LegacyOutcome::Failure) => {
            assert_eq!(canonical.outcome, CanonicalRuleOutcome::NoMatch);
            assert!(!canonical.matched);
            assert!(canonical.diagnostics.is_empty());
            assert_eq!(canonical.consumed, TextRange::empty(TextSize::ZERO));
        }
    }
}

#[test]
fn phase_2h_has_exactly_fifty_direct_parity_cases() {
    let contracts = phase_2h_contracts();
    assert_eq!(PHASE_2H_RULES.len(), 10);
    assert_eq!(contracts.len(), 10);
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| matches!(contract.value, CanonicalValue::Token))
            .count(),
        6,
    );
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| matches!(contract.value, CanonicalValue::Unit))
            .count(),
        1,
    );
    for contract in contracts {
        assert_eq!(
            PHASE_2H_RULES
                .iter()
                .filter(|rule| **rule == contract.rule)
                .count(),
            1,
            "{} must appear exactly once in the Phase 2H surface",
            contract.name,
        );
        for probe in contract.probes {
            assert_parity(contract, probe);
        }
    }
}

#[test]
fn phase_2h_table_top_requires_a_physical_newline() {
    for input in ["╭───\n", "╭───\r", "╭───\r\n", "┏───\r\n"] {
        let parsed = canonical_snapshot(input, rules::TABLE_TOP, ParseConfig::default());
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{input:?}");
        assert!(parsed.diagnostics.is_empty(), "{input:?}");
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::new(TextSize::ZERO, parsed.source.byte_len()),
            "{input:?}",
        );
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            input,
            "{input:?}",
        );
        assert!(
            parsed
                .syntax()
                .tokens()
                .into_iter()
                .all(|token| !token.flags().contains(TokenFlags::SYNTHETIC)),
            "{input:?}",
        );
    }

    for input in ["╭───", "╭", "┏━━━\r\n"] {
        let parsed = canonical_snapshot(input, rules::TABLE_TOP, ParseConfig::default());
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{input:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{input:?}"
        );
        assert!(parsed.diagnostics.is_empty(), "{input:?}");
    }
}

#[test]
fn phase_2h_delimiter_spellings_and_views_are_exact() {
    for (input, expected) in [
        ("[", MatrixDelimiterStyle::Bracket),
        ("╭", MatrixDelimiterStyle::RoundedBox),
        ("┌", MatrixDelimiterStyle::LightBox),
        ("┏", MatrixDelimiterStyle::BoldBox),
    ] {
        assert_parity(
            phase_2h_contracts()
                .into_iter()
                .find(|contract| contract.rule == rules::MATRIX_START)
                .unwrap(),
            input,
        );
        let parsed = canonical_snapshot(input, rules::MATRIX_START, ParseConfig::default());
        let token = semantic_token(rules::MATRIX_START, &parsed.syntax());
        assert_eq!(
            MatrixDelimiterStyle::from_opening_token(&token),
            Some(expected)
        );
        assert_eq!(expected.opening_text(), input);
    }
    for (input, expected) in [
        ("]", MatrixDelimiterStyle::Bracket),
        ("╯", MatrixDelimiterStyle::RoundedBox),
        ("┘", MatrixDelimiterStyle::LightBox),
        ("┛", MatrixDelimiterStyle::BoldBox),
    ] {
        let parsed = canonical_snapshot(input, rules::MATRIX_END, ParseConfig::default());
        let token = semantic_token(rules::MATRIX_END, &parsed.syntax());
        assert_eq!(
            MatrixDelimiterStyle::from_closing_token(&token),
            Some(expected)
        );
        assert_eq!(expected.closing_text(), input);
    }

    let opening = canonical_snapshot("[", rules::MATRIX_START, ParseConfig::default());
    let closing = canonical_snapshot("╯", rules::MATRIX_END, ParseConfig::default());
    assert_eq!(
        MatrixDelimiterStyle::from_opening_token(&semantic_token(
            rules::MATRIX_START,
            &opening.syntax(),
        )),
        Some(MatrixDelimiterStyle::Bracket),
    );
    assert_eq!(
        MatrixDelimiterStyle::from_closing_token(&semantic_token(
            rules::MATRIX_END,
            &closing.syntax(),
        )),
        Some(MatrixDelimiterStyle::RoundedBox),
    );

    for input in ["╭", "┌", "┏", "{", "|", "│", "┃", "╯", "┘", "┛", "}"] {
        let rule = if matches!(input, "╭" | "┌" | "┏" | "{") {
            rules::TABLE_START
        } else {
            rules::TABLE_END
        };
        assert_parity(
            phase_2h_contracts()
                .into_iter()
                .find(|contract| contract.rule == rule)
                .unwrap(),
            input,
        );
    }
}

#[test]
fn phase_2h_structure_views_preserve_physical_distinctions() {
    for (input, marker) in [("{}", false), ("{_}", true), ("{___}", true)] {
        let parsed = canonical_snapshot(input, rules::EMPTY_SET, ParseConfig::default());
        let node = find_node(&parsed.syntax(), SyntaxKind::EmptySet).unwrap();
        let syntax = EmptySetSyntax::cast(node).unwrap();
        assert_eq!(syntax.uses_explicit_empty_marker(), marker, "{input:?}");
        assert_eq!(syntax.marker().is_some(), marker, "{input:?}");
        assert_eq!(lower_legacy_empty_set(&syntax).unwrap().elements.len(), 0);
    }

    let map = canonical_snapshot("{\n:\n}", rules::EMPTY_MAP, ParseConfig::default());
    let map =
        EmptyMapSyntax::cast(find_node(&map.syntax(), SyntaxKind::EmptyMap).unwrap()).unwrap();
    assert!(lower_legacy_empty_map(&map).unwrap().elements.is_empty());

    let separator = canonical_snapshot("  ───  ", rules::ROW_SEPARATOR, ParseConfig::default());
    let separator = TableRowSeparatorSyntax::cast(
        find_node(&separator.syntax(), SyntaxKind::TableRowSeparator).unwrap(),
    )
    .unwrap();
    assert_eq!(
        separator
            .physical_tokens()
            .into_iter()
            .map(|token| token.text().unwrap())
            .collect::<String>(),
        "  ───  ",
    );
    assert!(
        lower_legacy_table_row_separator(&separator)
            .unwrap()
            .columns
            .is_empty()
    );
}

#[test]
fn phase_2h_regressions_preserve_closed_prefixes_and_horizontal_trivia() {
    let contracts = phase_2h_contracts();
    let contract = |rule| {
        contracts
            .iter()
            .copied()
            .find(|contract| contract.rule == rule)
            .unwrap()
    };
    for input in [" | ", "\t│\t", "\u{00a0}┃\u{2009}", "\n|", "|\n"] {
        assert_parity(contract(rules::TABLE_SEPARATOR), input);
    }
    for input in [
        "─",
        "───",
        "┼──┼",
        "┘",
        "┛",
        "╯",
        "|",
        "│",
        "┃",
        "  ───  ",
        "─   ",
        "─\t\t",
        "─\u{00a0}\u{2009}",
        "─   x",
        "─   |   ",
        "─\t┃\t",
        "┘   x",
    ] {
        assert_parity(contract(rules::ROW_SEPARATOR), input);
    }
    for input in ["{", "{:", "{_", "{: value}", "{a}"] {
        assert_parity(contract(rules::EMPTY_MAP), input);
        assert_parity(contract(rules::EMPTY_SET), input);
    }
    let parsed = canonical_snapshot("─x", rules::ROW_SEPARATOR, ParseConfig::default());
    assert_eq!(parsed.consumed.end, TextSize::from_u32("─".len() as u32));
}

#[test]
fn phase_2h_limits_remain_hard_for_direct_shell_rules() {
    let config = ParseConfig {
        limits: ParseLimits {
            fuel: 64,
            max_events: 16,
            ..ParseLimits::default()
        },
    };
    for (rule, input) in [
        (rules::TABLE_TOP, format!("╭{}\n", "─".repeat(8_192))),
        (rules::ROW_SEPARATOR, format!("{}", "─".repeat(8_192))),
        (
            rules::TABLE_SEPARATOR,
            format!(
                "{}|{}",
                " \t\u{00a0}\u{2009}".repeat(8_192),
                " \t".repeat(8_192)
            ),
        ),
        (
            rules::EMPTY_MAP,
            format!("{{{}:{} }}", "\n ".repeat(8_192), "\t\r".repeat(8_192)),
        ),
    ] {
        let parsed = canonical_snapshot(&input, rule, config);
        assert!(parsed.stats.parser_steps <= config.limits.fuel, "{rule:?}");
        assert!(
            parsed.stats.events_emitted <= config.limits.max_events as u64,
            "{rule:?}"
        );
    }
}
