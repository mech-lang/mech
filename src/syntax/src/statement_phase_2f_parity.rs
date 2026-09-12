//! Crate-internal parity coverage for the closed Phase 2F declarations.
//!
//! These checks keep the private legacy parsers as the value oracle while
//! exercising canonical fragments directly. Every accepted node, token, and
//! transparent production compares both its retained prefix and its exact
//! compatibility value.

use super::*;

use alloc::vec::Vec;

use mech_core::{SourceLocation, SourceRange, Token as LegacyToken, TokenKind};
use unicode_segmentation::UnicodeSegmentation;

use crate::document::lower::legacy::{
    LegacyDeclarationValue, LegacySourceImportValue, lower_phase_2f_declaration_value,
    lower_phase_2f_source_import_value,
};
use crate::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2f_rule_for_test,
};
use crate::document::parser::rules;
use crate::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, SyntaxToken, TextRange,
    TextSize, TextSnapshot, reconstruct_source_range, validate_lossless_range,
};
use crate::{ParseResult, ParseString};
use nom::Err;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParityValue {
    SourceNode(LegacySourceImportValue),
    DeclarationNode(LegacyDeclarationValue),
    Token(LegacyToken),
    Transparent(Vec<LegacyToken>),
}

type LegacyParser = for<'source> fn(ParseString<'source>) -> ParseResult<'source, ParityValue>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Prefix {
    consumed: TextSize,
    remaining: TextSize,
}

#[derive(Debug, Eq, PartialEq)]
struct LegacyMatch {
    value: ParityValue,
    prefix: Prefix,
}

#[derive(Debug, Eq, PartialEq)]
enum LegacyOutcome {
    Matched(LegacyMatch),
    Error { reached: TextSize },
    Failure { reached: TextSize },
}

#[derive(Clone, Copy)]
enum CanonicalValue {
    SourceNode(SyntaxKind),
    DeclarationNode(SyntaxKind),
    Token,
    Transparent,
}

#[derive(Clone, Copy)]
struct Contract {
    name: &'static str,
    rule: RuleId,
    value: CanonicalValue,
    parser: LegacyParser,
    probes: [&'static str; 5],
}

macro_rules! legacy_source_node_parser {
    ($name:ident, $parser:path, $variant:ident) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, ParityValue> {
            let (input, value) = $parser(input)?;
            Ok((
                input,
                ParityValue::SourceNode(LegacySourceImportValue::$variant(value)),
            ))
        }
    };
}

macro_rules! legacy_declaration_node_parser {
    ($name:ident, $parser:path, $variant:ident) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, ParityValue> {
            let (input, value) = $parser(input)?;
            Ok((
                input,
                ParityValue::DeclarationNode(LegacyDeclarationValue::$variant(value)),
            ))
        }
    };
}

macro_rules! legacy_token_parser {
    ($name:ident, $parser:path) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, ParityValue> {
            let (input, value) = $parser(input)?;
            Ok((input, ParityValue::Token(value)))
        }
    };
}

legacy_source_node_parser!(legacy_source_import_tail, super::source_import_tail, Tail);
legacy_token_parser!(
    legacy_source_path_component_token,
    super::source_path_component_token
);
legacy_source_node_parser!(
    legacy_source_path_component,
    super::source_path_component,
    Component
);
legacy_source_node_parser!(legacy_source_mec_path, super::source_mec_path, MecPath);
legacy_source_node_parser!(
    legacy_relative_source_import_specifier,
    super::relative_source_import_specifier,
    Relative
);
legacy_source_node_parser!(
    legacy_absolute_source_import_specifier,
    super::absolute_source_import_specifier,
    Absolute
);
legacy_source_node_parser!(
    legacy_bare_source_import_specifier,
    super::bare_source_import_specifier,
    Bare
);
legacy_token_parser!(legacy_uri_scheme_part, super::uri_scheme_part);
legacy_source_node_parser!(
    legacy_source_import_uri_scheme,
    super::source_import_uri_scheme,
    UriScheme
);
legacy_source_node_parser!(
    legacy_uri_source_import_specifier,
    super::uri_source_import_specifier,
    Uri
);
legacy_source_node_parser!(
    legacy_source_import_specifier,
    super::source_import_specifier,
    Specifier
);
legacy_source_node_parser!(
    legacy_import_declaration,
    super::import_declaration,
    Declaration
);
legacy_declaration_node_parser!(legacy_export_declaration, super::export_declaration, Export);
legacy_declaration_node_parser!(
    legacy_context_declaration,
    super::context_declaration,
    ContextDeclaration
);
legacy_declaration_node_parser!(
    legacy_context_base_context,
    super::context_base_context,
    ContextBaseContext
);
legacy_declaration_node_parser!(
    legacy_context_base_resource_uri,
    super::context_base_resource_uri,
    ContextBaseResourceUri
);
legacy_declaration_node_parser!(
    legacy_context_capability_declaration,
    super::context_capability_declaration,
    CapabilityDeclaration
);
legacy_token_parser!(
    legacy_context_capability_path_token,
    super::context_capability_path_token
);
legacy_declaration_node_parser!(
    legacy_context_capability_path,
    super::context_capability_path,
    CapabilityPath
);
legacy_declaration_node_parser!(
    legacy_context_capability_scope,
    super::context_capability_scope,
    CapabilityScope
);

fn legacy_source_mec_path_wildcard_suffix<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, ParityValue> {
    let (input, value) = super::source_mec_path_wildcard_suffix(input)?;
    Ok((input, ParityValue::Transparent(value)))
}

fn phase_2f_contracts() -> [Contract; 21] {
    [
        Contract {
            name: "source-import-tail",
            rule: rules::SOURCE_IMPORT_TAIL,
            value: CanonicalValue::SourceNode(SyntaxKind::SourceImportTail),
            parser: legacy_source_import_tail,
            probes: ["dep", "dep   ", "dep;rest", "\n", "💡/*\u{2009}"],
        },
        Contract {
            name: "source-path-component-token",
            rule: rules::SOURCE_PATH_COMPONENT_TOKEN,
            value: CanonicalValue::Token,
            parser: legacy_source_path_component_token,
            probes: ["a", "1", "-", "_", "."],
        },
        Contract {
            name: "source-path-component",
            rule: rules::SOURCE_PATH_COMPONENT,
            value: CanonicalValue::SourceNode(SyntaxKind::SourcePathComponent),
            parser: legacy_source_path_component,
            probes: ["foo", "foo-1_bar.mec", "foo/bar", "/", "."],
        },
        Contract {
            name: "source-mec-path",
            rule: rules::SOURCE_MEC_PATH,
            value: CanonicalValue::SourceNode(SyntaxKind::SourceMecPath),
            parser: legacy_source_mec_path,
            probes: [
                "foo.mec",
                "path/to/foo.mec",
                "foo.mec/*",
                "foo.MEC",
                "foo.mec/bar",
            ],
        },
        Contract {
            name: "source-mec-path-wildcard-suffix",
            rule: rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX,
            value: CanonicalValue::Transparent,
            parser: legacy_source_mec_path_wildcard_suffix,
            probes: ["", "/*", "/", "/x", "/*/x"],
        },
        Contract {
            name: "relative-source-import-specifier",
            rule: rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
            value: CanonicalValue::SourceNode(SyntaxKind::RelativeSourceImportSpecifier),
            parser: legacy_relative_source_import_specifier,
            probes: [
                "./foo.mec",
                "../lib/foo.mec",
                "../foo.mec/*",
                "foo.mec",
                "./foo.MEC",
            ],
        },
        Contract {
            name: "absolute-source-import-specifier",
            rule: rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER,
            value: CanonicalValue::SourceNode(SyntaxKind::AbsoluteSourceImportSpecifier),
            parser: legacy_absolute_source_import_specifier,
            probes: [
                "/foo.mec",
                "/lib/foo.mec",
                "/foo.mec/*",
                "foo.mec",
                "/foo.MEC",
            ],
        },
        Contract {
            name: "bare-source-import-specifier",
            rule: rules::BARE_SOURCE_IMPORT_SPECIFIER,
            value: CanonicalValue::SourceNode(SyntaxKind::BareSourceImportSpecifier),
            parser: legacy_bare_source_import_specifier,
            probes: [
                "foo.mec",
                "lib/foo.mec",
                "foo.mec/*",
                "./foo.mec",
                "foo.MEC",
            ],
        },
        Contract {
            name: "uri-scheme-part",
            rule: rules::URI_SCHEME_PART,
            value: CanonicalValue::Token,
            parser: legacy_uri_scheme_part,
            probes: ["a", "1", "+", "-", "."],
        },
        Contract {
            name: "source-import-uri-scheme",
            rule: rules::SOURCE_IMPORT_URI_SCHEME,
            value: CanonicalValue::SourceNode(SyntaxKind::SourceImportUriScheme),
            parser: legacy_source_import_uri_scheme,
            probes: ["https", "git+ssh", "a1.-", "1http", "http://x"],
        },
        Contract {
            name: "uri-source-import-specifier",
            rule: rules::URI_SOURCE_IMPORT_SPECIFIER,
            value: CanonicalValue::SourceNode(SyntaxKind::UriSourceImportSpecifier),
            parser: legacy_uri_source_import_specifier,
            probes: [
                "https://example.com/dep.mec",
                "memory://scratch/dep",
                "x://   ",
                "x://",
                "https:/x",
            ],
        },
        Contract {
            name: "source-import-specifier",
            rule: rules::SOURCE_IMPORT_SPECIFIER,
            value: CanonicalValue::SourceNode(SyntaxKind::SourceImportSpecifier),
            parser: legacy_source_import_specifier,
            probes: [
                "./dep.mec",
                "/dep.mec",
                "foo.mec://bar",
                "dep.mec",
                "dep.MEC",
            ],
        },
        Contract {
            name: "import-declaration",
            rule: rules::IMPORT_DECLARATION,
            value: CanonicalValue::SourceNode(SyntaxKind::ImportDeclaration),
            parser: legacy_import_declaration,
            probes: [
                "+> dep.mec",
                "+>\u{2009}dep.mec/*",
                "+> https://x/dep   ",
                "+>dep.mec",
                "+> https://x/a*b",
            ],
        },
        Contract {
            name: "export-declaration",
            rule: rules::EXPORT_DECLARATION,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ExportDeclaration),
            parser: legacy_export_declaration,
            probes: [
                "<+ value",
                "<+\tvalue",
                "<+\nvalue",
                "<+value",
                "<+\u{00a0}value",
            ],
        },
        Contract {
            name: "context-declaration",
            rule: rules::CONTEXT_DECLARATION,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ContextDeclaration),
            parser: legacy_context_declaration,
            probes: [
                "@ui := fs://workspace",
                "@users := @main{:read(users/*), :write(*)}",
                "@users := @main{:read(*),}",
                "@users := @main{}",
                "@users := @main {:read(*)}",
            ],
        },
        Contract {
            name: "context-base-context",
            rule: rules::CONTEXT_BASE_CONTEXT,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ContextBaseContext),
            parser: legacy_context_base_context,
            probes: ["@main", "@main/sub", "@💡", "main", "@_"],
        },
        Contract {
            name: "context-base-resource-uri",
            rule: rules::CONTEXT_BASE_RESOURCE_URI,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ContextBaseResourceUri),
            parser: legacy_context_base_resource_uri,
            probes: [
                "fs://workspace",
                "1.0://a_b/path",
                "-://x",
                "fs://",
                "fs://a:b",
            ],
        },
        Contract {
            name: "context-capability-declaration",
            rule: rules::CONTEXT_CAPABILITY_DECLARATION,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ContextCapabilityDeclaration),
            parser: legacy_context_capability_declaration,
            probes: [
                ":read(*)",
                ":write(users/*)",
                ":read/write(foo)",
                ":read()",
                "read(*)",
            ],
        },
        Contract {
            name: "context-capability-path-token",
            rule: rules::CONTEXT_CAPABILITY_PATH_TOKEN,
            value: CanonicalValue::Token,
            parser: legacy_context_capability_path_token,
            probes: ["a", "1", "/", "_", "*"],
        },
        Contract {
            name: "context-capability-path",
            rule: rules::CONTEXT_CAPABILITY_PATH,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ContextCapabilityPath),
            parser: legacy_context_capability_path,
            probes: ["users", "users/read", "users/*", "*", "foo/*/bar"],
        },
        Contract {
            name: "context-capability-scope",
            rule: rules::CONTEXT_CAPABILITY_SCOPE,
            value: CanonicalValue::DeclarationNode(SyntaxKind::ContextCapabilityScope),
            parser: legacy_context_capability_scope,
            probes: ["*", "users", "users/*", "**", "foo*"],
        },
    ]
}

fn legacy_outcome(input: &str, parser: LegacyParser) -> LegacyOutcome {
    let graphemes = crate::graphemes::init_source(input);
    let input_len = TextSize::from_u32(input.len() as u32);
    let to_offset = |cursor: usize| {
        assert!(
            cursor <= graphemes.len(),
            "legacy parser cursor {cursor} exceeds {} graphemes for {input:?}",
            graphemes.len(),
        );
        TextSize::from_u32(
            graphemes[..cursor]
                .iter()
                .map(|grapheme| grapheme.len() as u32)
                .sum(),
        )
    };
    let prefix = |cursor: usize| {
        let consumed = to_offset(cursor);
        assert!(
            consumed <= input_len,
            "legacy parser consumed its artificial source sentinel for {input:?}"
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
            LegacyOutcome::Matched(LegacyMatch {
                value,
                prefix: prefix(remaining.cursor),
            })
        }
        Err(Err::Error(error)) => LegacyOutcome::Error {
            reached: to_offset(error.remaining_input.cursor),
        },
        Err(Err::Failure(error)) => LegacyOutcome::Failure {
            reached: to_offset(error.remaining_input.cursor),
        },
        Err(Err::Incomplete(_)) => {
            panic!("legacy Phase 2F production requested more input for {input:?}")
        }
    }
}

fn canonical_snapshot(input: &str, rule: RuleId) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(0x2f), Revision(0), input)
        .expect("short direct parity probe must form a source snapshot");
    parse_canonical_phase_2f_rule_for_test(source, rule, ParseConfig::default())
        .expect("every direct Phase 2F parity rule must be supported")
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn canonical_value(contract: Contract, canonical: &CanonicalSourceRuleSnapshot) -> ParityValue {
    match contract.value {
        CanonicalValue::SourceNode(kind) => {
            let node = find_node(&canonical.syntax(), kind).unwrap_or_else(|| {
                panic!(
                    "{} should emit {kind:?} for an accepted direct probe",
                    contract.name,
                )
            });
            ParityValue::SourceNode(lower_phase_2f_source_import_value(&node).unwrap_or_else(
                |diagnostics| {
                    panic!(
                        "{} should lower canonical {kind:?}: {diagnostics:?}",
                        contract.name,
                    )
                },
            ))
        }
        CanonicalValue::DeclarationNode(kind) => {
            let node = find_node(&canonical.syntax(), kind).unwrap_or_else(|| {
                panic!(
                    "{} should emit {kind:?} for an accepted direct probe",
                    contract.name,
                )
            });
            ParityValue::DeclarationNode(lower_phase_2f_declaration_value(&node).unwrap_or_else(
                |diagnostics| {
                    panic!(
                        "{} should lower canonical {kind:?}: {diagnostics:?}",
                        contract.name,
                    )
                },
            ))
        }
        CanonicalValue::Token => {
            let tokens = canonical.syntax().tokens();
            let [token] = tokens.as_slice() else {
                panic!(
                    "{} should retain exactly one canonical token for a direct probe",
                    contract.name,
                );
            };
            ParityValue::Token(lower_direct_token(&canonical.syntax(), token))
        }
        CanonicalValue::Transparent => ParityValue::Transparent(
            canonical
                .syntax()
                .tokens()
                .iter()
                .map(|token| lower_direct_token(&canonical.syntax(), token))
                .collect(),
        ),
    }
}

fn lower_direct_token(syntax: &SyntaxNode, token: &SyntaxToken) -> LegacyToken {
    let kind = match token.kind() {
        SyntaxKind::Alpha => TokenKind::Alpha,
        SyntaxKind::Digit => TokenKind::Digit,
        SyntaxKind::Dash => TokenKind::Dash,
        SyntaxKind::Underscore => TokenKind::Underscore,
        SyntaxKind::Period => TokenKind::Period,
        SyntaxKind::Plus => TokenKind::Plus,
        SyntaxKind::Slash => TokenKind::Slash,
        SyntaxKind::Asterisk => TokenKind::Asterisk,
        other => panic!("unsupported direct Phase 2F token kind {other:?}"),
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

fn assert_matched(
    contract: Contract,
    input: &str,
    legacy: LegacyMatch,
    canonical: CanonicalSourceRuleSnapshot,
) {
    assert_eq!(
        canonical.outcome,
        CanonicalRuleOutcome::Matched,
        "{} should cleanly match {input:?}; canonical diagnostics: {:?}",
        contract.name,
        canonical.diagnostics,
    );
    assert!(
        canonical.is_strictly_clean(),
        "{} should have a clean canonical result for {input:?}: {:?}",
        contract.name,
        canonical.diagnostics,
    );
    assert_eq!(
        canonical.consumed.start,
        TextSize::ZERO,
        "{} should begin at the probe start for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.end, legacy.prefix.consumed,
        "{} consumed a different prefix for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.source.byte_len() - canonical.consumed.end,
        legacy.prefix.remaining,
        "{} left a different suffix for {input:?}",
        contract.name,
    );
    validate_lossless_range(&canonical.root, &canonical.source, canonical.consumed).unwrap_or_else(
        |error| {
            panic!(
                "{} produced a non-lossless fragment for {input:?}: {error:?}",
                contract.name,
            )
        },
    );
    assert_eq!(
        reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed)
            .expect("validated canonical fragment must reconstruct"),
        input[..legacy.prefix.consumed.to_usize()],
        "{} reconstructed a different accepted prefix for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical_value(contract, &canonical),
        legacy.value,
        "{} lowered a different legacy value for {input:?}",
        contract.name,
    );
}

fn assert_error(
    contract: Contract,
    input: &str,
    reached: TextSize,
    canonical: CanonicalSourceRuleSnapshot,
) {
    assert_eq!(
        canonical.outcome,
        CanonicalRuleOutcome::NoMatch,
        "{} should remain noncommitting for rejected probe {input:?}",
        contract.name,
    );
    assert!(
        !canonical.matched,
        "{} unexpectedly matched {input:?}",
        contract.name
    );
    assert!(
        canonical.diagnostics.is_empty(),
        "{} emitted diagnostics for a noncommitting probe {input:?}: {:?}",
        contract.name,
        canonical.diagnostics,
    );
    assert_eq!(
        canonical.consumed,
        TextRange::empty(TextSize::ZERO),
        "{} retained source after noncommitting probe {input:?}",
        contract.name,
    );
    assert!(
        reached <= TextSize::from_u32(input.len() as u32),
        "legacy {} failure point exceeded the probe for {input:?}",
        contract.name,
    );
}

fn assert_failure(
    contract: Contract,
    input: &str,
    reached: TextSize,
    canonical: CanonicalSourceRuleSnapshot,
) {
    assert_eq!(
        canonical.outcome,
        CanonicalRuleOutcome::Committed,
        "{} should retain its local commitment for {input:?}",
        contract.name,
    );
    assert!(
        canonical.matched,
        "{} should retain {input:?}",
        contract.name
    );
    assert!(
        !canonical.diagnostics.is_empty(),
        "{} should report its retained malformed prefix for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.start,
        TextSize::ZERO,
        "{} should retain from the probe start for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.end, reached,
        "{} retained a different malformed prefix for {input:?}",
        contract.name,
    );
    validate_lossless_range(&canonical.root, &canonical.source, canonical.consumed).unwrap_or_else(
        |error| {
            panic!(
                "{} produced a non-lossless retained fragment for {input:?}: {error:?}",
                contract.name,
            )
        },
    );
    assert_eq!(
        reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed)
            .expect("validated canonical fragment must reconstruct"),
        input[..reached.to_usize()],
        "{} reconstructed a different retained prefix for {input:?}",
        contract.name,
    );
}

fn assert_direct_parity_probe(contract: Contract, input: &str) {
    let legacy = legacy_outcome(input, contract.parser);
    let canonical = canonical_snapshot(input, contract.rule);
    match legacy {
        LegacyOutcome::Matched(legacy) => assert_matched(contract, input, legacy, canonical),
        LegacyOutcome::Error { reached } => assert_error(contract, input, reached, canonical),
        LegacyOutcome::Failure { reached } => assert_failure(contract, input, reached, canonical),
    }
}

#[test]
fn phase_2f_has_exactly_105_direct_legacy_parity_cases() {
    let contracts = phase_2f_contracts();
    assert_eq!(contracts.len(), 21);
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| matches!(
                contract.value,
                CanonicalValue::SourceNode(_) | CanonicalValue::DeclarationNode(_)
            ))
            .count(),
        17,
    );
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| matches!(contract.value, CanonicalValue::Token))
            .count(),
        3,
    );
    assert_eq!(
        contracts
            .iter()
            .filter(|contract| matches!(contract.value, CanonicalValue::Transparent))
            .count(),
        1,
    );
    for contract in contracts {
        assert_eq!(
            phase_2f_contracts()
                .iter()
                .filter(|candidate| candidate.rule == contract.rule)
                .count(),
            1,
            "each Phase 2F rule needs exactly one parity contract: {}",
            contract.name,
        );
        for input in contract.probes {
            assert_direct_parity_probe(contract, input);
        }
    }
}

#[test]
fn import_declaration_leading_whitespace_regressions_match_legacy_values() {
    let contract = phase_2f_contracts()
        .into_iter()
        .find(|contract| contract.rule == rules::IMPORT_DECLARATION)
        .expect("Phase 2F import declaration contract");
    for input in [
        "\n+> dep.mec",
        "\r+> dep.mec",
        "\r\n+> dep.mec",
        " \n\t+> dep.mec",
    ] {
        assert_direct_parity_probe(contract, input);
    }
}
