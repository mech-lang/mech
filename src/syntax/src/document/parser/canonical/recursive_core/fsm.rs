use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator};
use super::{Attempt, FactAttempt, calls, patterns, recover_required_production_with_prefixes};

pub(super) fn parse_fsm_pipe(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FSM_PIPE, |parser| {
        let node = parser.start();
        let child = parse_fsm_instance(parser);
        let mut committed = match child {
            Attempt::Matched => false,
            Attempt::Committed => true,
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
        };
        loop {
            let before = parser.offset();
            let stage = stage(parser);
            match stage {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed if parser.offset() > before => committed = true,
                Attempt::Committed => break,
            }
        }
        node.complete(parser, SyntaxKind::FsmPipe);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

pub(super) fn parse_fsm_instance(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FSM_INSTANCE, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::HASHTAG) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let mut committed = false;
        if !base::parse_rule(parser, rules::IDENTIFIER) {
            recover_required_production_with_prefixes(
                parser,
                rules::FSM_INSTANCE,
                "syntax/missing-fsm-name",
                "missing state-machine name after hash sign",
                "identifier",
                &["(", "->", "~>", "=>", "→", "⇒"],
            );
            committed = true;
        }
        committed |= parse_fsm_args(parser) == Attempt::Committed;
        node.complete(parser, SyntaxKind::FsmInstance);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
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
            FactAttempt::Recovered(facts)
                if !facts.contains_wildcard && !facts.contains_array_spread_or_rest =>
            {
                node.complete(parser, SyntaxKind::FsmValue);
                Attempt::Committed
            }
            FactAttempt::Recovered(_) if parser.is_halted() => {
                node.complete(parser, SyntaxKind::FsmValue);
                Attempt::Committed
            }
            FactAttempt::Matched(_) | FactAttempt::Recovered(_) | FactAttempt::NoMatch => {
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
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production_with_prefixes(
                    parser,
                    rule,
                    "syntax/missing-fsm-transition-value",
                    "missing state-machine value after transition operator",
                    "fsm-value",
                    &["->", "~>", "=>", "→", "⇒"],
                );
                node.complete(parser, kind);
                return Attempt::Committed;
            }
        }
        node.complete(parser, kind);
        Attempt::Matched
    })
}
