//! Canonical context paths retain token choices, prefix phases, and path repetition.

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

const TOKEN_ALTERNATIVES: &[RuleId] = &[
    rules::ALPHA_TOKEN,
    rules::DIGIT_TOKEN,
    rules::DASH,
    rules::SLASH,
    rules::UNDERSCORE,
    rules::PERIOD,
];
pub(crate) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::CONTEXT_ADDRESS_PATH_TOKEN
            | rules::CONTEXT_ADDRESS_PATH
            | rules::PREFIXED_CONTEXT_PATH
    )
}
enum Frame {
    Call(RuleId),
    Exit(ParserCheckpoint),
    Base(base::continuation::Continuation),
    Token(usize),
    Path(Marker, TextSize, bool),
    Prefix(Marker, usize),
    PrefixResult(Marker),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical path owner");
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
                    match rule {
                        rules::CONTEXT_ADDRESS_PATH_TOKEN => {
                            self.frames.push(Frame::Token(0));
                            self.base(TOKEN_ALTERNATIVES[0]);
                        }
                        rules::CONTEXT_ADDRESS_PATH => {
                            self.frames
                                .push(Frame::Path(parser.start(), parser.offset(), false));
                            self.frames
                                .push(Frame::Call(rules::CONTEXT_ADDRESS_PATH_TOKEN));
                        }
                        rules::PREFIXED_CONTEXT_PATH => {
                            self.frames.push(Frame::Prefix(parser.start(), 0));
                            self.base(rules::AT);
                        }
                        _ => unreachable!("canonical path rule"),
                    }
                }
                Frame::Token(index) => {
                    if self.matched {
                        self.result = Attempt::Matched;
                    } else if let Some(rule) = TOKEN_ALTERNATIVES.get(index + 1) {
                        self.frames.push(Frame::Token(index + 1));
                        self.base(*rule);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Path(marker, before, matched_any) => {
                    let accepted = self.result.accepted();
                    if accepted && parser.offset() != before && !parser.is_halted() {
                        self.frames.push(Frame::Path(marker, parser.offset(), true));
                        self.frames
                            .push(Frame::Call(rules::CONTEXT_ADDRESS_PATH_TOKEN));
                    } else if accepted || matched_any {
                        marker.complete(parser, SyntaxKind::ContextAddressPath);
                        self.result = Attempt::Matched;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Prefix(marker, stage) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else {
                        match stage {
                            0 => {
                                self.frames.push(Frame::Prefix(marker, 1));
                                self.base(rules::IDENTIFIER_PATH_SEGMENT);
                            }
                            1 => {
                                self.frames.push(Frame::Prefix(marker, 2));
                                self.base(rules::SLASH);
                            }
                            2 => {
                                self.frames.push(Frame::PrefixResult(marker));
                                self.frames.push(Frame::Call(rules::CONTEXT_ADDRESS_PATH));
                            }
                            _ => unreachable!("context prefix phase"),
                        }
                    }
                }
                Frame::PrefixResult(marker) => {
                    if self.result.accepted() {
                        marker.complete(parser, SyntaxKind::PrefixedContextPath);
                        self.result = Attempt::Matched;
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

/// The path token remains transparent and emits its canonical lexical child.
pub(crate) fn parse_context_address_path_token(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_ADDRESS_PATH_TOKEN)
}
pub(crate) fn parse_context_address_path(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CONTEXT_ADDRESS_PATH)
}
/// Incomplete context prefixes remain losing candidates until final recovery is selected by their owner.
pub(crate) fn parse_prefixed_context_path(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::PREFIXED_CONTEXT_PATH)
}

#[cfg(test)]
mod tests {
    use super::super::continuation_test_support::{assert_partitions, run};
    use super::*;
    use alloc::string::String;
    #[test]
    fn retained_paths_preserve_partitions_and_hard_limits() {
        assert_partitions::<Continuation>(&[
            (rules::CONTEXT_ADDRESS_PATH_TOKEN, ""),
            (rules::CONTEXT_ADDRESS_PATH_TOKEN, "α"),
            (rules::CONTEXT_ADDRESS_PATH_TOKEN, "e\u{301}"),
            (rules::CONTEXT_ADDRESS_PATH_TOKEN, "👩\u{200d}💻"),
            (rules::CONTEXT_ADDRESS_PATH_TOKEN, "."),
            (rules::CONTEXT_ADDRESS_PATH, "alpha/β-1.file_2"),
            (rules::CONTEXT_ADDRESS_PATH, "x\r\ny"),
            (rules::CONTEXT_ADDRESS_PATH, "x\u{301}/y"),
            (rules::PREFIXED_CONTEXT_PATH, "@"),
            (rules::PREFIXED_CONTEXT_PATH, "@ctx"),
            (rules::PREFIXED_CONTEXT_PATH, "@ctx/"),
            (rules::PREFIXED_CONTEXT_PATH, "@ctx/path/β_2"),
            (rules::PREFIXED_CONTEXT_PATH, "@👩\u{200d}💻/path"),
            (rules::PREFIXED_CONTEXT_PATH, "@e\u{301}/x"),
            (rules::PREFIXED_CONTEXT_PATH, "@a.b/c"),
        ]);
    }
    #[test]
    fn retained_paths_grow_linearly() {
        for (rule, prefix, unit) in [
            (rules::CONTEXT_ADDRESS_PATH, "", "a/"),
            (rules::PREFIXED_CONTEXT_PATH, "@ctx/", "a/"),
            (rules::PREFIXED_CONTEXT_PATH, "@", "a"),
        ] {
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
