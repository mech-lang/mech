mod continuation;
use super::Marker;
use crate::document::SyntaxKind;
pub(super) use continuation::{KindContinuation, supports as continuation_supports};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, kinds as leaves};
use super::{Attempt, finish_provisional_marker};

#[cfg(test)]
pub(super) fn parse_kind_annotation(parser: &mut Parser<'_>) -> Attempt {
    kind_annotation(parser, true)
}

#[cfg(test)]
fn kind_annotation(parser: &mut Parser<'_>, recover: bool) -> Attempt {
    KindContinuation::annotation(recover).drive(parser)
}

#[derive(Clone, Copy)]
pub(super) enum KindPosition {
    Ordinary,
    MapKey,
    BraceElement,
}

#[cfg(test)]
pub(super) fn parse_kind(parser: &mut Parser<'_>) -> Attempt {
    kind(parser, KindPosition::Ordinary)
}

#[cfg(test)]
fn kind(parser: &mut Parser<'_>, position: KindPosition) -> Attempt {
    KindContinuation::kind(position).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_with_option(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_WITH_OPTION).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_kind(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_KIND).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_table(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_TABLE).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_set(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_SET).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_map(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_MAP).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_record(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_RECORD).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_matrix(parser: &mut Parser<'_>) -> Attempt {
    kind_matrix(parser, KindPosition::Ordinary)
}

#[cfg(test)]
fn kind_matrix(parser: &mut Parser<'_>, position: KindPosition) -> Attempt {
    KindContinuation::matrix(position).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_tuple(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_TUPLE).drive(parser)
}

#[cfg(test)]
pub(super) fn parse_kind_scalar(parser: &mut Parser<'_>) -> Attempt {
    KindContinuation::new(rules::KIND_SCALAR).drive(parser)
}

fn finish(
    node: super::super::super::marker::Marker,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    result: Attempt,
) -> Attempt {
    if parser.is_halted() {
        node.complete(parser, kind);
        return Attempt::Committed;
    }
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
