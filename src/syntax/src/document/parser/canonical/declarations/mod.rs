//! Canonical export and context declaration productions for Phase 2F.
//!
//! These rules form a closed declaration island. They do not select a
//! statement, code, or document root.

use crate::document::{RuleId, SyntaxKind, TextRange};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::Attempt;

/// The complete closed declaration set directly ported by Phase 2F.
pub(crate) const PHASE_2F_DECLARATION_RULES: &[RuleId; 8] = &[
    rules::EXPORT_DECLARATION,
    rules::CONTEXT_DECLARATION,
    rules::CONTEXT_BASE_CONTEXT,
    rules::CONTEXT_BASE_RESOURCE_URI,
    rules::CONTEXT_CAPABILITY_DECLARATION,
    rules::CONTEXT_CAPABILITY_PATH_TOKEN,
    rules::CONTEXT_CAPABILITY_PATH,
    rules::CONTEXT_CAPABILITY_SCOPE,
];

/// Whether `rule` belongs to the Phase 2F declaration layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2F_DECLARATION_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2F declaration production.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::EXPORT_DECLARATION => parse_export_declaration(parser),
        rules::CONTEXT_DECLARATION => parse_context_declaration(parser),
        rules::CONTEXT_BASE_CONTEXT => parse_context_base_context(parser),
        rules::CONTEXT_BASE_RESOURCE_URI => parse_context_base_resource_uri(parser),
        rules::CONTEXT_CAPABILITY_DECLARATION => parse_context_capability_declaration(parser),
        rules::CONTEXT_CAPABILITY_PATH_TOKEN => parse_context_capability_path_token(parser),
        rules::CONTEXT_CAPABILITY_PATH => parse_context_capability_path(parser),
        rules::CONTEXT_CAPABILITY_SCOPE => parse_context_capability_scope(parser),
        _ => unreachable!("Phase 2F declaration support guard rejects every other RuleId"),
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
            _ => unreachable!("final declaration input"),
        }
    }
}
pub(crate) fn parse_export_declaration(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::EXPORT_DECLARATION)
}
pub(crate) fn parse_context_base_context(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_BASE_CONTEXT)
}
pub(crate) fn parse_context_base_resource_uri(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_BASE_RESOURCE_URI)
}
pub(crate) fn parse_context_capability_declaration(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_CAPABILITY_DECLARATION)
}
pub(crate) fn parse_context_capability_path_token(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_CAPABILITY_PATH_TOKEN)
}
pub(crate) fn parse_context_capability_path(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_CAPABILITY_PATH)
}
pub(crate) fn parse_context_capability_scope(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_CAPABILITY_SCOPE)
}
pub(crate) fn parse_context_declaration(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_DECLARATION)
}
