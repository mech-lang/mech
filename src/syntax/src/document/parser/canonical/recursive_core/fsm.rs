use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator};
use super::{Attempt, FactAttempt, calls, child_result, patterns};

pub(super) fn parse_fsm_pipe(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FSM_PIPE, |parser| {
        let node = parser.start();
        let child = parse_fsm_instance(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::FsmPipe, child) {
            return result;
        }
        loop {
            let before = parser.offset();
            let stage = stage(parser);
            match stage {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::FsmPipe);
                    return Attempt::Committed;
                }
            }
        }
        node.complete(parser, SyntaxKind::FsmPipe);
        Attempt::Matched
    })
}

pub(super) fn parse_fsm_instance(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FSM_INSTANCE, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::HASHTAG) || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        if parse_fsm_args(parser) == Attempt::Committed {
            node.complete(parser, SyntaxKind::FsmInstance);
            return Attempt::Committed;
        }
        node.complete(parser, SyntaxKind::FsmInstance);
        Attempt::Matched
    })
}

pub(super) fn parse_fsm_args(parser: &mut Parser<'_>) -> Attempt {
    calls::argument_list(parser, rules::FSM_ARGS, SyntaxKind::FsmArguments)
}

pub(super) fn parse_fsm_state_transition(parser: &mut Parser<'_>) -> Attempt {
    transition(
        parser,
        rules::FSM_STATE_TRANSITION,
        rules::TRANSITION_OPERATOR,
        SyntaxKind::FsmStateTransition,
    )
}

pub(super) fn parse_fsm_async_transition(parser: &mut Parser<'_>) -> Attempt {
    transition(
        parser,
        rules::FSM_ASYNC_TRANSITION,
        rules::ASYNC_TRANSITION_OPERATOR,
        SyntaxKind::FsmAsyncTransition,
    )
}

pub(super) fn parse_fsm_output(parser: &mut Parser<'_>) -> Attempt {
    transition(
        parser,
        rules::FSM_OUTPUT,
        rules::OUTPUT_OPERATOR,
        SyntaxKind::FsmOutput,
    )
}

pub(super) fn parse_fsm_value(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FSM_VALUE, |parser| {
        let node = parser.start();
        match patterns::pattern_with_facts(parser) {
            FactAttempt::Matched(facts)
                if !facts.contains_wildcard && !facts.contains_array_spread_or_rest =>
            {
                node.complete(parser, SyntaxKind::FsmValue);
                Attempt::Matched
            }
            FactAttempt::Matched(_) | FactAttempt::NoMatch => {
                node.abandon(parser);
                Attempt::NoMatch
            }
            FactAttempt::Committed => {
                node.complete(parser, SyntaxKind::FsmValue);
                Attempt::Committed
            }
        }
    })
}

fn stage(parser: &mut Parser<'_>) -> Attempt {
    for parse in [
        parse_fsm_state_transition as fn(&mut Parser<'_>) -> Attempt,
        parse_fsm_async_transition,
        parse_fsm_output,
    ] {
        let result = parse(parser);
        if result != Attempt::NoMatch {
            return result;
        }
    }
    Attempt::NoMatch
}

fn transition(
    parser: &mut Parser<'_>,
    rule: crate::document::RuleId,
    operator: crate::document::RuleId,
    kind: SyntaxKind,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, operator) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_fsm_value(parser);
        if let Some(result) = child_result(parser, node, kind, child) {
            return result;
        }
        node.complete(parser, kind);
        Attempt::Matched
    })
}
