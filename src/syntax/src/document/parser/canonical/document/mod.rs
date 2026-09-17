//! Canonical document grammar interpreter.

mod continuation;

use crate::document::{ExpectedSyntax, NodeFlags, RuleId, SyntaxKind};

use super::super::Parser;
use super::super::recovery;
use super::super::rule::rules;
use super::combinator::Attempt;
use super::document_grammar::{
    DOCUMENT_RULE_COUNT, DOCUMENT_RULES, DocumentRule, GrammarExpression,
};
use super::{
    base, declarations, imports, kinds, literals, mechdown, operators, paths, primitives, prose,
    recursive_core, source_imports, statements, strings, structure_shell,
};

#[derive(Clone, Copy, Debug, Default)]
struct GrammarState {
    codeblock_delimiter: Option<RuleId>,
}

pub(crate) fn parse_document_root(parser: &mut Parser<'_>) {
    let mut continuation = continuation::Continuation::document_root();
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            continuation::Progress::Complete(_) => return,
            continuation::Progress::NeedsProcessing => {}
            _ => unreachable!("finite canonical document"),
        }
    }
}

pub(crate) fn supports(rule: RuleId) -> bool {
    debug_assert_eq!(DOCUMENT_RULES.len(), DOCUMENT_RULE_COUNT);
    DOCUMENT_RULES
        .iter()
        .any(|candidate| candidate.rule == rule)
}

pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let Some(specification) = DOCUMENT_RULES.iter().find(|spec| spec.rule == rule) else {
        return Attempt::NoMatch;
    };
    let mut continuation = continuation::Continuation::for_rule(specification);
    let mut allowance = u64::MAX;
    loop {
        match continuation.advance(parser, true, &mut allowance) {
            continuation::Progress::Complete(result) => return result,
            continuation::Progress::NeedsProcessing => allowance = u64::MAX,
            continuation::Progress::NeedInput | continuation::Progress::Limited => {
                unreachable!("finite document input")
            }
        }
    }
}

fn document_rule_enabled(specification: &DocumentRule) -> bool {
    match specification.feature {
        None => true,
        Some("mika") => cfg!(feature = "mika"),
        Some("invariant_define") => cfg!(feature = "invariant_define"),
        Some(feature) => unreachable!("unknown generated document feature {feature}"),
    }
}

fn required_sequence_item(
    item: &GrammarExpression,
) -> Option<recovery::MissingContinuation<'static>> {
    let GrammarExpression::Rule(rule) = item else {
        return None;
    };
    if *rule == rules::MECH_CODE_ALT {
        return Some(recovery::MissingContinuation::new(
            "syntax/missing-inline-mech-body",
            "expected a body after the inline Mech opener",
            ExpectedSyntax::Production(alloc::string::String::from("inline Mech body")),
            None,
        ));
    }
    let (code, message, token) = if *rule == rules::NEW_LINE {
        (
            "syntax/missing-codeblock-header-newline",
            "expected a newline after the code-block header",
            SyntaxKind::Newline,
        )
    } else if *rule == rules::RIGHT_BRACE {
        (
            "syntax/missing-inline-mech-closer",
            "expected a closing brace for inline Mech code",
            SyntaxKind::RightBrace,
        )
    } else if *rule == rules::MIKA_SECTION_CLOSE {
        (
            "syntax/missing-mika-section-closer",
            "expected a closing Mika section delimiter",
            SyntaxKind::MikaSectionClose,
        )
    } else {
        return None;
    };
    Some(recovery::MissingContinuation::new(
        code,
        message,
        ExpectedSyntax::Token(token),
        Some(token),
    ))
}
