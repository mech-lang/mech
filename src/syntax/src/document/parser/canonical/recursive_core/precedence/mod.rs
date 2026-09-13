mod continuation;
use crate::document::{RuleId, SyntaxKind};
pub(crate) use continuation::{Continuation, Progress, supports};

use super::super::super::marker::Marker;
use super::super::super::rule::rules;
use super::super::super::{Parser, ParserCheckpoint};
use super::super::{base, operators};
use super::{Attempt, expressions, structures};

#[derive(Clone, Copy)]
pub(super) struct FormulaSeed {
    checkpoint: ParserCheckpoint,
    l1: Marker,
    l2: Marker,
    l3: Marker,
    l4: Marker,
    l5: Marker,
    l6: Marker,
    l7: Marker,
    factor: Marker,
}

impl FormulaSeed {
    pub(super) fn start(parser: &mut Parser<'_>) -> Self {
        let checkpoint = parser.checkpoint();
        Self {
            checkpoint,
            l1: parser.start(),
            l2: parser.start(),
            l3: parser.start(),
            l4: parser.start(),
            l5: parser.start(),
            l6: parser.start(),
            l7: parser.start(),
            factor: parser.start(),
        }
    }

    pub(super) fn abandon(self, parser: &mut Parser<'_>) {
        self.factor.abandon(parser);
        self.l7.abandon(parser);
        self.l6.abandon(parser);
        self.l5.abandon(parser);
        self.l4.abandon(parser);
        self.l3.abandon(parser);
        self.l2.abandon(parser);
        self.l1.abandon(parser);
    }

    pub(super) fn commit(self, parser: &mut Parser<'_>) -> Attempt {
        self.factor.complete(parser, SyntaxKind::Factor);
        complete_seeded_outer(self, parser, 7);
        Attempt::Committed
    }
}

pub(super) fn parse_l1(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L1).drive(parser)
}

pub(super) fn parse_l2(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L2).drive(parser)
}

pub(super) fn parse_l3(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L3).drive(parser)
}

pub(super) fn parse_l4(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L4).drive(parser)
}

pub(super) fn parse_l5(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L5).drive(parser)
}

pub(super) fn parse_l6(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L6).drive(parser)
}

pub(super) fn parse_l7(parser: &mut Parser<'_>) -> Attempt {
    continuation::Continuation::new(rules::L7).drive(parser)
}

pub(super) fn parse_factor(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FACTOR).drive(parser)
}

pub(super) fn parse_parenthetical_term(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PARENTHETICAL_TERM).drive(parser)
}

pub(super) fn parse_negate_factor(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::NEGATE_FACTOR).drive(parser)
}

pub(super) fn parse_not_factor(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::NOT_FACTOR).drive(parser)
}

pub(super) fn parse_range_expression(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::RANGE_EXPRESSION).drive(parser)
}

pub(super) fn parse_match_arm(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::MATCH_ARM).drive(parser)
}

fn complete_seeded_outer(seed: FormulaSeed, parser: &mut Parser<'_>, count: usize) {
    if count >= 7 {
        seed.l7.complete(parser, SyntaxKind::SetExpression);
    }
    if count >= 6 {
        seed.l6.complete(parser, SyntaxKind::TableExpression);
    }
    if count >= 5 {
        seed.l5.complete(parser, SyntaxKind::PowerExpression);
    }
    if count >= 4 {
        seed.l4
            .complete(parser, SyntaxKind::MultiplicativeExpression);
    }
    if count >= 3 {
        seed.l3.complete(parser, SyntaxKind::AdditiveExpression);
    }
    if count >= 2 {
        seed.l2.complete(parser, SyntaxKind::ComparisonExpression);
    }
    if count >= 1 {
        seed.l1.complete(parser, SyntaxKind::LogicExpression);
    }
}

fn finish(
    node: super::super::super::marker::Marker,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    result: Attempt,
) -> Attempt {
    match result {
        Attempt::Matched => {
            node.complete(parser, kind);
            Attempt::Matched
        }
        Attempt::NoMatch => {
            node.abandon(parser);
            Attempt::NoMatch
        }
        Attempt::Committed => {
            node.complete(parser, kind);
            Attempt::Committed
        }
    }
}
