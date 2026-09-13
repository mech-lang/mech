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
use super::combinator::Attempt;

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

mod continuation;
pub(crate) use continuation::{Continuation, Progress};
fn drive(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final source-import input"),
        }
    }
}
pub(crate) fn parse_source_import_tail(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_IMPORT_TAIL)
}
pub(crate) fn parse_source_path_component_token(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_PATH_COMPONENT_TOKEN)
}
pub(crate) fn parse_source_path_component(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_PATH_COMPONENT)
}
pub(crate) fn parse_source_mec_path(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_MEC_PATH)
}
pub(crate) fn parse_source_mec_path_wildcard_suffix(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX)
}
pub(crate) fn parse_relative_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RELATIVE_SOURCE_IMPORT_SPECIFIER)
}
pub(crate) fn parse_absolute_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER)
}
pub(crate) fn parse_bare_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::BARE_SOURCE_IMPORT_SPECIFIER)
}
pub(crate) fn parse_uri_scheme_part(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::URI_SCHEME_PART)
}
pub(crate) fn parse_source_import_uri_scheme(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_IMPORT_URI_SCHEME)
}
pub(crate) fn parse_uri_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::URI_SOURCE_IMPORT_SPECIFIER)
}
pub(crate) fn parse_source_import_specifier(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SOURCE_IMPORT_SPECIFIER)
}
pub(crate) fn parse_import_declaration(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::IMPORT_DECLARATION)
}
