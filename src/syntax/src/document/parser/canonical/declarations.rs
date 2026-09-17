//! Canonical export and context declaration productions for Phase 2F.
//!
//! These rules form a closed declaration island. They do not select a
//! statement, code, or document root.

use crate::document::{RuleId, SyntaxKind, TextRange};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

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

/// Parse an export declaration with its canonical `whitespace1` boundary.
pub(crate) fn parse_export_declaration(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::EXPORT_DECLARATION, |parser| {
        let declaration = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::MODULE_EXPORT_SIGIL)
            || !base::parse_rule(parser, rules::WHITESPACE1)
            || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            if parser.is_halted() {
                declaration.complete(parser, SyntaxKind::ExportDeclaration);
                return Attempt::Committed;
            }
            declaration.abandon(parser);
            return Attempt::NoMatch;
        }
        declaration.complete(parser, SyntaxKind::ExportDeclaration);
        Attempt::Matched
    })
}

/// Parse an `@context` base reference.
pub(crate) fn parse_context_base_context(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_BASE_CONTEXT, |parser| {
        let base_context = parser.start();
        if !base::parse_rule(parser, rules::AT) || !base::parse_rule(parser, rules::IDENTIFIER) {
            if parser.is_halted() {
                base_context.complete(parser, SyntaxKind::ContextBaseContext);
                return Attempt::Committed;
            }
            base_context.abandon(parser);
            return Attempt::NoMatch;
        }
        base_context.complete(parser, SyntaxKind::ContextBaseContext);
        Attempt::Matched
    })
}

/// Parse the closed resource URI form used by context declarations.
pub(crate) fn parse_context_base_resource_uri(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_BASE_RESOURCE_URI, |parser| {
        let resource = parser.start();
        if !parse_one_or_more(parser, parse_context_resource_scheme_token)
            || !base::parse_exact_tag(parser, "://", SyntaxKind::Text)
            || !parse_one_or_more(parser, parse_context_resource_tail_token)
        {
            if parser.is_halted() {
                resource.complete(parser, SyntaxKind::ContextBaseResourceUri);
                return Attempt::Committed;
            }
            resource.abandon(parser);
            return Attempt::NoMatch;
        }
        resource.complete(parser, SyntaxKind::ContextBaseResourceUri);
        Attempt::Matched
    })
}

/// Parse one capability declaration without implicit whitespace or recovery.
pub(crate) fn parse_context_capability_declaration(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_CAPABILITY_DECLARATION, |parser| {
        let declaration = parser.start();
        if !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::IDENTIFIER)
            || !base::parse_rule(parser, rules::LEFT_PARENTHESIS)
            || !parse_context_capability_scope(parser).accepted()
            || !base::parse_rule(parser, rules::RIGHT_PARENTHESIS)
        {
            if parser.is_halted() {
                declaration.complete(parser, SyntaxKind::ContextCapabilityDeclaration);
                return Attempt::Committed;
            }
            declaration.abandon(parser);
            return Attempt::NoMatch;
        }
        declaration.complete(parser, SyntaxKind::ContextCapabilityDeclaration);
        Attempt::Matched
    })
}

/// Parse one direct context-capability path token without a wrapper node.
pub(crate) fn parse_context_capability_path_token(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_CAPABILITY_PATH_TOKEN, |parser| {
        if base::parse_rule(parser, rules::ALPHA_TOKEN)
            || base::parse_rule(parser, rules::DIGIT_TOKEN)
            || base::parse_rule(parser, rules::DASH)
            || base::parse_rule(parser, rules::SLASH)
            || base::parse_rule(parser, rules::UNDERSCORE)
            || base::parse_rule(parser, rules::PERIOD)
            || base::parse_rule(parser, rules::ASTERISK)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse and validate a complete capability path candidate without diagnostics.
pub(crate) fn parse_context_capability_path(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_CAPABILITY_PATH, |parser| {
        let path = parser.start();
        let start = parser.offset();
        if !parse_one_or_more(parser, parse_context_capability_path_token) {
            path.abandon(parser);
            return Attempt::NoMatch;
        }
        let candidate = TextRange::new(start, parser.offset());
        if parser.is_halted() {
            path.complete(parser, SyntaxKind::ContextCapabilityPath);
            return Attempt::Committed;
        }
        let valid = parser
            .source()
            .text(candidate)
            .map(|text| context_capability_path_is_valid(&text))
            .unwrap_or(false);
        if !valid {
            path.abandon(parser);
            return Attempt::NoMatch;
        }
        path.complete(parser, SyntaxKind::ContextCapabilityPath);
        Attempt::Matched
    })
}

/// Parse either a lone wildcard token or a validated capability path.
pub(crate) fn parse_context_capability_scope(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_CAPABILITY_SCOPE, |parser| {
        let scope = parser.start();
        let result = if base::parse_rule(parser, rules::ASTERISK) {
            Attempt::Matched
        } else {
            parse_context_capability_path(parser)
        };
        if result == Attempt::NoMatch {
            scope.abandon(parser);
            return Attempt::NoMatch;
        }
        scope.complete(parser, SyntaxKind::ContextCapabilityScope);
        result
    })
}

/// Parse the declaration plus its fully transactional optional capability group.
pub(crate) fn parse_context_declaration(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_DECLARATION, |parser| {
        let declaration = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::AT)
            || !base::parse_rule(parser, rules::IDENTIFIER)
            || !base::parse_rule(parser, rules::DEFINE_OPERATOR)
            || first_accepted(
                parser,
                &[parse_context_base_resource_uri, parse_context_base_context],
            ) == Attempt::NoMatch
        {
            if parser.is_halted() {
                declaration.complete(parser, SyntaxKind::ContextDeclaration);
                return Attempt::Committed;
            }
            declaration.abandon(parser);
            return Attempt::NoMatch;
        }

        parse_optional_capability_group(parser);
        declaration.complete(parser, SyntaxKind::ContextDeclaration);
        Attempt::Matched
    })
}

fn parse_optional_capability_group(parser: &mut Parser<'_>) {
    let group = parser.checkpoint();
    if !base::parse_rule(parser, rules::LEFT_BRACE)
        || !parse_context_capability_declaration(parser).accepted()
    {
        if parser.is_halted() {
            return;
        }
        parser.rewind(group);
        return;
    }

    loop {
        let separator = parser.checkpoint();
        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
            parser.rewind(separator);
            break;
        }
        if !parse_context_capability_declaration(parser).accepted() {
            if parser.is_halted() {
                return;
            }
            parser.rewind(separator);
            break;
        }
    }

    let trailing = parser.checkpoint();
    if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
        parser.rewind(trailing);
    }
    if !base::parse_rule(parser, rules::RIGHT_BRACE) {
        if parser.is_halted() {
            return;
        }
        parser.rewind(group);
    }
}

fn parse_context_resource_scheme_token(parser: &mut Parser<'_>) -> Attempt {
    if base::parse_rule(parser, rules::ALPHA_TOKEN)
        || base::parse_rule(parser, rules::DIGIT_TOKEN)
        || base::parse_rule(parser, rules::DASH)
        || base::parse_rule(parser, rules::PERIOD)
    {
        Attempt::Matched
    } else {
        Attempt::NoMatch
    }
}

fn parse_context_resource_tail_token(parser: &mut Parser<'_>) -> Attempt {
    if parse_context_resource_scheme_token(parser).accepted()
        || base::parse_rule(parser, rules::SLASH)
        || base::parse_rule(parser, rules::UNDERSCORE)
    {
        Attempt::Matched
    } else {
        Attempt::NoMatch
    }
}

fn parse_one_or_more(parser: &mut Parser<'_>, parse: fn(&mut Parser<'_>) -> Attempt) -> bool {
    let mut matched_any = false;
    loop {
        let before = parser.offset();
        if !parse(parser).accepted() {
            break;
        }
        matched_any = true;
        if parser.offset() == before || parser.is_halted() {
            break;
        }
    }
    matched_any
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

fn context_capability_path_is_valid(path: &str) -> bool {
    let star_count = path.chars().filter(|character| *character == '*').count();
    star_count == 0 || (star_count == 1 && path.ends_with("/*") && path.len() > 2)
}
