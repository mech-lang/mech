//! Canonical source-import productions for the Phase 2F closed island.
//!
//! This module deliberately stops at declarations. It provides no statement or
//! document dispatcher, so direct parser contracts remain independent from the
//! enclosing code grammar.

use alloc::string::String;

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticLabel, DiagnosticPhase, DiagnosticTags,
    NodeFlags, RuleId, Severity, SyntaxKind, TextRange, TextSize,
};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// The complete closed source-import set directly ported by Phase 2F.
pub(crate) const PHASE_2F_SOURCE_IMPORT_RULES: &[RuleId; 13] = &[
    rules::SOURCE_IMPORT_TAIL,
    rules::SOURCE_PATH_COMPONENT_TOKEN,
    rules::SOURCE_PATH_COMPONENT,
    rules::SOURCE_MEC_PATH,
    rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX,
    rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
    rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER,
    rules::BARE_SOURCE_IMPORT_SPECIFIER,
    rules::URI_SCHEME_PART,
    rules::SOURCE_IMPORT_URI_SCHEME,
    rules::URI_SOURCE_IMPORT_SPECIFIER,
    rules::SOURCE_IMPORT_SPECIFIER,
    rules::IMPORT_DECLARATION,
];

/// Whether `rule` belongs to the Phase 2F source-import layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2F_SOURCE_IMPORT_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2F source-import production.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::SOURCE_IMPORT_TAIL => parse_source_import_tail(parser),
        rules::SOURCE_PATH_COMPONENT_TOKEN => parse_source_path_component_token(parser),
        rules::SOURCE_PATH_COMPONENT => parse_source_path_component(parser),
        rules::SOURCE_MEC_PATH => parse_source_mec_path(parser),
        rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX => parse_source_mec_path_wildcard_suffix(parser),
        rules::RELATIVE_SOURCE_IMPORT_SPECIFIER => parse_relative_source_import_specifier(parser),
        rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER => parse_absolute_source_import_specifier(parser),
        rules::BARE_SOURCE_IMPORT_SPECIFIER => parse_bare_source_import_specifier(parser),
        rules::URI_SCHEME_PART => parse_uri_scheme_part(parser),
        rules::SOURCE_IMPORT_URI_SCHEME => parse_source_import_uri_scheme(parser),
        rules::URI_SOURCE_IMPORT_SPECIFIER => parse_uri_source_import_specifier(parser),
        rules::SOURCE_IMPORT_SPECIFIER => parse_source_import_specifier(parser),
        rules::IMPORT_DECLARATION => parse_import_declaration(parser),
        _ => unreachable!("Phase 2F source-import support guard rejects every other RuleId"),
    })
}

/// Parse a nonempty URI tail while retaining every physical grapheme.
pub(crate) fn parse_source_import_tail(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_IMPORT_TAIL, |parser| {
        let tail = parser.start();
        let mut count = 0_u32;

        loop {
            if rule_is_ahead(parser, rules::NEW_LINE) || rule_is_ahead(parser, rules::SEMICOLON) {
                break;
            }
            let before = parser.offset();
            if !base::parse_rule(parser, rules::ANY_TOKEN) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
            count = count.saturating_add(1);
        }

        if count == 0 {
            tail.abandon(parser);
            return Attempt::NoMatch;
        }
        tail.complete(parser, SyntaxKind::SourceImportTail);
        Attempt::Matched
    })
}

/// Parse one direct source-path component token without a wrapper node.
pub(crate) fn parse_source_path_component_token(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_PATH_COMPONENT_TOKEN, |parser| {
        if base::parse_rule(parser, rules::ALPHA_TOKEN)
            || base::parse_rule(parser, rules::DIGIT_TOKEN)
            || base::parse_rule(parser, rules::DASH)
            || base::parse_rule(parser, rules::UNDERSCORE)
            || base::parse_rule(parser, rules::PERIOD)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse one or more source-path component tokens.
pub(crate) fn parse_source_path_component(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_PATH_COMPONENT, |parser| {
        let component = parser.start();
        let mut matched_any = false;
        loop {
            let before = parser.offset();
            if !parse_source_path_component_token(parser).accepted() {
                break;
            }
            matched_any = true;
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }
        if !matched_any {
            component.abandon(parser);
            return Attempt::NoMatch;
        }
        component.complete(parser, SyntaxKind::SourcePathComponent);
        Attempt::Matched
    })
}

/// Parse the maximal `.mec` path candidate with noncommitting validation.
pub(crate) fn parse_source_mec_path(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_MEC_PATH, |parser| {
        let path = parser.start();
        let start = parser.offset();
        if !parse_source_path_component(parser).accepted() {
            path.abandon(parser);
            return Attempt::NoMatch;
        }

        loop {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::SLASH) {
                parser.rewind(pair);
                break;
            }
            if !parse_source_path_component(parser).accepted() {
                parser.rewind(pair);
                break;
            }
            if parser.is_halted() {
                break;
            }
        }

        let candidate = TextRange::new(start, parser.offset());
        let valid = parser
            .source()
            .text(candidate)
            .map(|text| text.ends_with(".mec"))
            .unwrap_or(false);
        if !valid {
            path.abandon(parser);
            return Attempt::NoMatch;
        }

        path.complete(parser, SyntaxKind::SourceMecPath);
        Attempt::Matched
    })
}

/// Parse an optional `/*` suffix. Its absent case is an accepted empty match.
pub(crate) fn parse_source_mec_path_wildcard_suffix(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX, |parser| {
        let suffix = parser.checkpoint();
        if base::parse_rule(parser, rules::SLASH) && base::parse_rule(parser, rules::ASTERISK) {
            return Attempt::Matched;
        }
        parser.rewind(suffix);
        Attempt::Matched
    })
}

/// Parse a `../` or `./` source-import specifier.
pub(crate) fn parse_relative_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RELATIVE_SOURCE_IMPORT_SPECIFIER, |parser| {
        let specifier = parser.start();
        let prefix = parser.checkpoint();
        let parent = base::parse_rule(parser, rules::PERIOD)
            && base::parse_rule(parser, rules::PERIOD)
            && base::parse_rule(parser, rules::SLASH);
        if !parent {
            parser.rewind(prefix);
            let local =
                base::parse_rule(parser, rules::PERIOD) && base::parse_rule(parser, rules::SLASH);
            if !local {
                specifier.abandon(parser);
                return Attempt::NoMatch;
            }
        }
        if !parse_source_mec_path(parser).accepted()
            || !parse_source_mec_path_wildcard_suffix(parser).accepted()
        {
            specifier.abandon(parser);
            return Attempt::NoMatch;
        }
        specifier.complete(parser, SyntaxKind::RelativeSourceImportSpecifier);
        Attempt::Matched
    })
}

/// Parse an absolute source-import specifier.
pub(crate) fn parse_absolute_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER, |parser| {
        let specifier = parser.start();
        if !base::parse_rule(parser, rules::SLASH)
            || !parse_source_mec_path(parser).accepted()
            || !parse_source_mec_path_wildcard_suffix(parser).accepted()
        {
            specifier.abandon(parser);
            return Attempt::NoMatch;
        }
        specifier.complete(parser, SyntaxKind::AbsoluteSourceImportSpecifier);
        Attempt::Matched
    })
}

/// Parse a direct bare source-import specifier.
pub(crate) fn parse_bare_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::BARE_SOURCE_IMPORT_SPECIFIER, |parser| {
        let specifier = parser.start();
        if !parse_source_mec_path(parser).accepted()
            || !parse_source_mec_path_wildcard_suffix(parser).accepted()
        {
            specifier.abandon(parser);
            return Attempt::NoMatch;
        }
        specifier.complete(parser, SyntaxKind::BareSourceImportSpecifier);
        Attempt::Matched
    })
}

/// Parse one URI scheme component token without a wrapper node.
pub(crate) fn parse_uri_scheme_part(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::URI_SCHEME_PART, |parser| {
        if base::parse_rule(parser, rules::ALPHA_TOKEN)
            || base::parse_rule(parser, rules::DIGIT_TOKEN)
            || base::parse_rule(parser, rules::PLUS)
            || base::parse_rule(parser, rules::DASH)
            || base::parse_rule(parser, rules::PERIOD)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse a source-import URI scheme with an alphabetic leading token.
pub(crate) fn parse_source_import_uri_scheme(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_IMPORT_URI_SCHEME, |parser| {
        let scheme = parser.start();
        if !base::parse_rule(parser, rules::ALPHA_TOKEN) {
            scheme.abandon(parser);
            return Attempt::NoMatch;
        }
        loop {
            let before = parser.offset();
            if !parse_uri_scheme_part(parser).accepted() {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }
        scheme.complete(parser, SyntaxKind::SourceImportUriScheme);
        Attempt::Matched
    })
}

/// Parse a URI source-import specifier with a required physical tail.
pub(crate) fn parse_uri_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::URI_SOURCE_IMPORT_SPECIFIER, |parser| {
        let specifier = parser.start();
        if !parse_source_import_uri_scheme(parser).accepted()
            || !base::parse_exact_tag(parser, "://", SyntaxKind::Text)
            || !parse_source_import_tail(parser).accepted()
        {
            specifier.abandon(parser);
            return Attempt::NoMatch;
        }
        specifier.complete(parser, SyntaxKind::UriSourceImportSpecifier);
        Attempt::Matched
    })
}

/// Parse the ordinary first-success source-import specifier alternative.
pub(crate) fn parse_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SOURCE_IMPORT_SPECIFIER, |parser| {
        let specifier = parser.start();
        let result = first_accepted(
            parser,
            &[
                parse_relative_source_import_specifier,
                parse_absolute_source_import_specifier,
                parse_uri_source_import_specifier,
                parse_bare_source_import_specifier,
            ],
        );
        if result == Attempt::NoMatch {
            specifier.abandon(parser);
            return Attempt::NoMatch;
        }
        specifier.complete(parser, SyntaxKind::SourceImportSpecifier);
        result
    })
}

/// Parse a source-import declaration and retain invalid wildcard syntax in an
/// explicit error wrapper for the compatibility lowerer.
pub(crate) fn parse_import_declaration(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::IMPORT_DECLARATION, |parser| {
        let declaration = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::IMPORT_SIGIL)
            || !base::parse_rule(parser, rules::SPACE_TAB1)
        {
            declaration.abandon(parser);
            return Attempt::NoMatch;
        }

        let invalid = parser.start();
        let specifier_start = parser.offset();
        if !parse_source_import_specifier(parser).accepted() {
            invalid.abandon(parser);
            declaration.abandon(parser);
            return Attempt::NoMatch;
        }
        let specifier_range = TextRange::new(specifier_start, parser.offset());
        let wildcard_offsets = invalid_wildcard_offsets(parser, specifier_range);
        if wildcard_offsets.is_empty() {
            invalid.abandon(parser);
            declaration.complete(parser, SyntaxKind::ImportDeclaration);
            return Attempt::Matched;
        }

        let error = invalid.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
        emit_invalid_wildcard_diagnostic(
            parser,
            error.position(),
            specifier_start,
            &wildcard_offsets,
        );
        declaration.complete(parser, SyntaxKind::ImportDeclaration);
        Attempt::Committed
    })
}

fn first_accepted(parser: &mut Parser<'_>, choices: &[fn(&mut Parser<'_>) -> Attempt]) -> Attempt {
    for choice in choices {
        match choice(parser) {
            Attempt::NoMatch => {}
            result => return result,
        }
    }
    Attempt::NoMatch
}

fn rule_is_ahead(parser: &mut Parser<'_>, rule: RuleId) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = base::parse_rule(parser, rule);
    parser.rewind(checkpoint);
    matched
}

fn invalid_wildcard_offsets(parser: &Parser<'_>, range: TextRange) -> alloc::vec::Vec<usize> {
    let Ok(physical) = parser.source().text(range) else {
        return alloc::vec::Vec::new();
    };
    let semantic = physical.trim_end_matches(char::is_whitespace);
    let stars = semantic
        .char_indices()
        .filter_map(|(offset, character)| (character == '*').then_some(offset))
        .collect::<alloc::vec::Vec<_>>();
    let valid = stars.is_empty() || (stars.len() == 1 && semantic.ends_with("/*"));
    (!valid).then_some(stars).unwrap_or_default()
}

fn emit_invalid_wildcard_diagnostic(
    parser: &mut Parser<'_>,
    error_event: usize,
    specifier_start: TextSize,
    wildcard_offsets: &[usize],
) {
    let first = wildcard_offsets[0];
    let first_len = "*".len();
    let first_start = specifier_start + TextSize::from_u32(first as u32);
    let first_range = TextRange::new(
        first_start,
        first_start + TextSize::from_u32(first_len as u32),
    );
    let mut labels = alloc::vec::Vec::new();
    for offset in wildcard_offsets.iter().copied().skip(1) {
        let start = specifier_start + TextSize::from_u32(offset as u32);
        labels.push(DiagnosticLabel {
            anchor: DiagnosticAnchor::Absolute {
                revision: parser.source().revision(),
                range: TextRange::new(start, start + TextSize::from_u32(first_len as u32)),
            },
            message: String::from("additional wildcard"),
        });
    }
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::from("syntax/invalid-source-import-wildcard"),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range: first_range,
        },
        labels,
        expected: alloc::vec![],
        found: None,
        fixes: alloc::vec![],
        related: alloc::vec![],
        recovery: None,
        tags: DiagnosticTags::NONE,
        message: String::from("source-import wildcard must be the sole final `/*` suffix"),
    };
    let _ = error_event;
    parser.push_diagnostic(diagnostic, None, TextRange::empty(TextSize::ZERO));
}
