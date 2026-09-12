use crate::document::{RuleId, SyntaxKind};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator, pattern_primitives};
use super::{Attempt, FactAttempt, PatternFacts, expressions, nesting_limit, transactional_fact};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrayToken {
    Spread,
    Rest,
    Item(PatternFacts),
}

pub(super) fn parse_pattern(parser: &mut Parser<'_>) -> Attempt {
    pattern_with_facts(parser).attempt()
}

pub(super) fn parse_pattern_tuple_struct(parser: &mut Parser<'_>) -> Attempt {
    tuple_struct_with_facts(
        parser,
        rules::PATTERN_TUPLE_STRUCT,
        rules::GRAVE,
        SyntaxKind::TupleStructPattern,
    )
    .attempt()
}

pub(super) fn parse_pattern_array_item(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::PATTERN_ARRAY_ITEM, parse_pattern)
}

pub(super) fn parse_pattern_array_token(parser: &mut Parser<'_>) -> Attempt {
    array_token(parser).attempt()
}

pub(super) fn parse_pattern_array(parser: &mut Parser<'_>) -> Attempt {
    array_with_facts(parser).attempt()
}

pub(super) fn parse_pattern_atom_struct(parser: &mut Parser<'_>) -> Attempt {
    tuple_struct_with_facts(
        parser,
        rules::PATTERN_ATOM_STRUCT,
        rules::COLON,
        SyntaxKind::AtomStructPattern,
    )
    .attempt()
}

pub(super) fn parse_pattern_tuple(parser: &mut Parser<'_>) -> Attempt {
    tuple_with_facts(parser).attempt()
}

pub(super) fn pattern_with_facts(parser: &mut Parser<'_>) -> FactAttempt<PatternFacts> {
    transactional_fact(parser, rules::PATTERN, |parser| {
        let node = parser.start();
        let mut selected = tuple_struct_with_facts(
            parser,
            rules::PATTERN_ATOM_STRUCT,
            rules::COLON,
            SyntaxKind::AtomStructPattern,
        );
        if matches!(selected, FactAttempt::NoMatch) {
            selected = tuple_struct_with_facts(
                parser,
                rules::PATTERN_TUPLE_STRUCT,
                rules::GRAVE,
                SyntaxKind::TupleStructPattern,
            );
        }
        if matches!(selected, FactAttempt::NoMatch)
            && pattern_primitives::parse_wildcard(parser) == Attempt::Matched
        {
            selected = FactAttempt::Matched(PatternFacts {
                contains_wildcard: true,
                contains_array_spread_or_rest: false,
            });
        }
        if matches!(selected, FactAttempt::NoMatch) {
            selected = array_with_facts(parser);
        }
        if matches!(selected, FactAttempt::NoMatch) {
            selected = tuple_with_facts(parser);
        }
        if matches!(selected, FactAttempt::NoMatch) {
            selected = match expressions::parse_expression(parser) {
                Attempt::Matched => FactAttempt::Matched(PatternFacts::default()),
                Attempt::Committed => FactAttempt::Committed,
                Attempt::NoMatch => FactAttempt::NoMatch,
            };
        }
        match selected {
            FactAttempt::Matched(facts) => {
                node.complete(parser, SyntaxKind::Pattern);
                FactAttempt::Matched(facts)
            }
            FactAttempt::Committed => {
                node.complete(parser, SyntaxKind::Pattern);
                FactAttempt::Committed
            }
            FactAttempt::NoMatch => {
                node.abandon(parser);
                FactAttempt::NoMatch
            }
        }
    })
}

fn tuple_struct_with_facts(
    parser: &mut Parser<'_>,
    rule: RuleId,
    prefix: RuleId,
    kind: SyntaxKind,
) -> FactAttempt<PatternFacts> {
    transactional_fact(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, prefix)
            || !base::parse_rule(parser, rules::IDENTIFIER)
            || !base::parse_rule(parser, rules::LEFT_PARENTHESIS)
        {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| pattern_list(parser, false)) else {
            nesting_limit(parser);
            node.complete(parser, kind);
            return FactAttempt::Committed;
        };
        finish_facts(node, parser, kind, interior)
    })
}

fn tuple_with_facts(parser: &mut Parser<'_>) -> FactAttempt<PatternFacts> {
    transactional_fact(parser, rules::PATTERN_TUPLE, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_PARENTHESIS) {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| pattern_list(parser, false)) else {
            nesting_limit(parser);
            node.complete(parser, SyntaxKind::TuplePattern);
            return FactAttempt::Committed;
        };
        finish_facts(node, parser, SyntaxKind::TuplePattern, interior)
    })
}

fn pattern_list(parser: &mut Parser<'_>, allow_empty: bool) -> FactAttempt<PatternFacts> {
    if !base::parse_rule(parser, rules::WHITESPACE0) {
        return FactAttempt::NoMatch;
    }
    if allow_empty && base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
        return FactAttempt::Matched(PatternFacts::default());
    }
    let mut facts = match pattern_with_facts(parser) {
        FactAttempt::Matched(facts) => facts,
        FactAttempt::NoMatch => return FactAttempt::NoMatch,
        FactAttempt::Committed => return FactAttempt::Committed,
    };
    loop {
        let pair = parser.checkpoint();
        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
            break;
        }
        match pattern_with_facts(parser) {
            FactAttempt::Matched(item) => facts.merge(item),
            FactAttempt::NoMatch => {
                parser.rewind(pair);
                break;
            }
            FactAttempt::Committed => return FactAttempt::Committed,
        }
    }
    if !base::parse_rule(parser, rules::WHITESPACE0)
        || !base::parse_rule(parser, rules::RIGHT_PARENTHESIS)
    {
        FactAttempt::NoMatch
    } else {
        FactAttempt::Matched(facts)
    }
}

fn array_with_facts(parser: &mut Parser<'_>) -> FactAttempt<PatternFacts> {
    transactional_fact(parser, rules::PATTERN_ARRAY, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACKET) {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return FactAttempt::NoMatch;
            }
            let mut tokens = alloc::vec::Vec::new();
            loop {
                if base::parse_rule(parser, rules::RIGHT_BRACKET) {
                    break;
                }
                let before = parser.offset();
                match array_token(parser) {
                    FactAttempt::Matched(token) if parser.offset() > before => tokens.push(token),
                    FactAttempt::Matched(_) | FactAttempt::NoMatch => return FactAttempt::NoMatch,
                    FactAttempt::Committed => return FactAttempt::Committed,
                }
                if !base::parse_rule(parser, rules::WHITESPACE0) {
                    return FactAttempt::NoMatch;
                }
                let _ = base::parse_rule(parser, rules::LIST_SEPARATOR);
            }
            validate_array_tokens(&tokens)
        }) else {
            nesting_limit(parser);
            node.complete(parser, SyntaxKind::ArrayPattern);
            return FactAttempt::Committed;
        };
        finish_facts(node, parser, SyntaxKind::ArrayPattern, interior)
    })
}

fn array_token(parser: &mut Parser<'_>) -> FactAttempt<ArrayToken> {
    transactional_fact(parser, rules::PATTERN_ARRAY_TOKEN, |parser| {
        let node = parser.start();
        let token = if pattern_primitives::parse_spread_operator(parser) == Attempt::Matched {
            ArrayToken::Spread
        } else if base::parse_rule(parser, rules::ENUM_SEPARATOR) {
            ArrayToken::Rest
        } else {
            match pattern_with_facts(parser) {
                FactAttempt::Matched(facts) => ArrayToken::Item(facts),
                FactAttempt::NoMatch => {
                    node.abandon(parser);
                    return FactAttempt::NoMatch;
                }
                FactAttempt::Committed => {
                    node.complete(parser, SyntaxKind::ArrayPatternElement);
                    return FactAttempt::Committed;
                }
            }
        };
        node.complete(parser, SyntaxKind::ArrayPatternElement);
        FactAttempt::Matched(token)
    })
}

fn validate_array_tokens(tokens: &[ArrayToken]) -> FactAttempt<PatternFacts> {
    let spread = tokens
        .iter()
        .filter(|token| matches!(token, ArrayToken::Spread))
        .count();
    let rest = tokens
        .iter()
        .filter(|token| matches!(token, ArrayToken::Rest))
        .count();
    if spread > 1 || rest > 1 || (spread > 0 && rest > 0) {
        return FactAttempt::NoMatch;
    }
    if let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, ArrayToken::Rest))
    {
        if tokens[..index]
            .iter()
            .any(|token| !matches!(token, ArrayToken::Item(_)))
            || tokens
                .get(index + 1..)
                .is_none_or(|tail| !matches!(tail, [ArrayToken::Item(_)]))
        {
            return FactAttempt::NoMatch;
        }
    }
    let mut facts = PatternFacts {
        contains_wildcard: false,
        contains_array_spread_or_rest: spread > 0 || rest > 0,
    };
    for token in tokens {
        if let ArrayToken::Item(item) = token {
            facts.merge(*item);
        }
    }
    FactAttempt::Matched(facts)
}

fn finish_facts(
    node: super::super::super::marker::Marker,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    result: FactAttempt<PatternFacts>,
) -> FactAttempt<PatternFacts> {
    match result {
        FactAttempt::Matched(facts) => {
            node.complete(parser, kind);
            FactAttempt::Matched(facts)
        }
        FactAttempt::NoMatch => {
            node.abandon(parser);
            FactAttempt::NoMatch
        }
        FactAttempt::Committed => {
            node.complete(parser, kind);
            FactAttempt::Committed
        }
    }
}
