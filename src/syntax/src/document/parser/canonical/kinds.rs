//! Canonical primitive kinds retain prefix and repeated-token phases.

use super::super::rule::rules;
use super::super::{Parser, checkpoint::ParserCheckpoint, marker::Marker};
use super::base;
use super::combinator::Attempt;
use crate::document::{RuleId, SyntaxKind, TextSize};
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}

pub(crate) fn supports(rule: RuleId) -> bool {
    matches!(rule, rules::KIND_ANY | rules::KIND_EMPTY | rules::KIND_ATOM)
}
enum Frame {
    Call(RuleId),
    Exit(ParserCheckpoint),
    Base(base::continuation::Continuation),
    Complete(Marker, SyntaxKind),
    EmptyFirst(Marker),
    EmptyNext(Marker, TextSize),
    Atom(Marker),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical primitive kind owner");
        Self {
            frames: alloc::vec![Frame::Call(rule)],
            result: Attempt::NoMatch,
            matched: false,
            work: 0,
        }
    }
    fn base(&mut self, rule: RuleId) {
        self.frames
            .push(Frame::Base(base::continuation::Continuation::new(rule)));
    }

    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty() {
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let frame = self.frames.pop().expect("retained canonical leaf phase");
            if !matches!(frame, Frame::Base(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        // The transaction owns rejected markers, including a wrapper
                        // around a child that finalized the hard-limit remainder.
                        parser.rewind(checkpoint);
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(matched) => self.matched = matched,
                        base::continuation::Progress::NeedsProcessing => {
                            self.frames.push(Frame::Base(continuation));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::NeedInput => {
                            self.frames.push(Frame::Base(continuation));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::Limited => {
                            self.frames.push(Frame::Base(continuation));
                            return Progress::Limited;
                        }
                    }
                }

                Frame::Call(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.frames.push(Frame::Exit(checkpoint));
                    let marker = parser.start();
                    match rule {
                        rules::KIND_ANY => {
                            self.frames
                                .push(Frame::Complete(marker, SyntaxKind::KindAny));
                            self.base(rules::ASTERISK);
                        }
                        rules::KIND_EMPTY => {
                            self.frames.push(Frame::EmptyFirst(marker));
                            self.base(rules::UNDERSCORE);
                        }
                        rules::KIND_ATOM => {
                            self.frames.push(Frame::Atom(marker));
                            self.base(rules::COLON);
                        }
                        _ => unreachable!("canonical primitive kind rule"),
                    }
                }
                Frame::Complete(marker, kind) => {
                    if self.matched {
                        marker.complete(parser, kind);
                        self.result = Attempt::Matched;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::EmptyFirst(marker) => {
                    if self.matched {
                        self.frames.push(Frame::EmptyNext(marker, parser.offset()));
                        self.base(rules::UNDERSCORE);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::EmptyNext(marker, before) => {
                    if self.matched && parser.offset() != before && !parser.is_halted() {
                        self.frames.push(Frame::EmptyNext(marker, parser.offset()));
                        self.base(rules::UNDERSCORE);
                    } else {
                        marker.complete(parser, SyntaxKind::KindEmpty);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Atom(marker) => {
                    if self.matched {
                        self.frames
                            .push(Frame::Complete(marker, SyntaxKind::KindAtom));
                        self.base(rules::IDENTIFIER);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}
fn drive(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final canonical leaf input"),
        }
    }
}

pub(crate) fn parse_kind_any(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::KIND_ANY)
}
pub(crate) fn parse_kind_empty(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::KIND_EMPTY)
}
pub(crate) fn parse_kind_atom(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::KIND_ATOM)
}

#[cfg(test)]
mod tests {
    use super::super::continuation_test_support::{assert_partitions, run};
    use super::*;
    use alloc::string::String;
    #[test]
    fn retained_kinds_preserve_partitions_and_hard_limits() {
        assert_partitions::<Continuation>(&[
            (rules::KIND_ANY, ""),
            (rules::KIND_ANY, "*"),
            (rules::KIND_ANY, "*\u{301}"),
            (rules::KIND_ANY, "**"),
            (rules::KIND_EMPTY, ""),
            (rules::KIND_EMPTY, "_"),
            (rules::KIND_EMPTY, "____"),
            (rules::KIND_EMPTY, "__\u{301}"),
            (rules::KIND_EMPTY, "__\r\n"),
            (rules::KIND_ATOM, ":"),
            (rules::KIND_ATOM, ":status"),
            (rules::KIND_ATOM, ":e\u{301}"),
            (rules::KIND_ATOM, ":👩\u{200d}💻"),
            (rules::KIND_ATOM, ":💡"),
            (rules::KIND_ATOM, ":status/path"),
            (rules::KIND_ATOM, ":\r\n"),
        ]);
    }
    #[test]
    fn retained_kinds_grow_linearly() {
        for (rule, prefix, unit) in [(rules::KIND_EMPTY, "", "_"), (rules::KIND_ATOM, ":", "a")] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(prefix) + &unit.repeat(n);
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, work) = run::<Continuation>(rule, &text, &chunks, u64::MAX, true);
                let (baseline, one_shot_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, baseline);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.stats.diagnostics_emitted, 0);
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(work <= prior_streamed * 3, "append restarted {rule:?}");
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
