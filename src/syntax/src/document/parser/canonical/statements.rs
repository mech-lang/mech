//! Canonical comment owners retain sigil probes and physical text across input.
use super::super::literal_scan::{LiteralProgress, LiteralScan};
use super::super::rule::rules;
use super::super::{Parser, checkpoint::ParserCheckpoint, marker::Marker};
use super::base;
use super::combinator::Attempt;
use crate::document::{RuleId, SyntaxKind, TextSize};
use alloc::{boxed::Box, vec::Vec};

pub(crate) fn supports(rule: RuleId) -> bool {
    matches!(rule, rules::COMMENT | rules::COMMENT_SIGIL)
}
pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint, bool),
    Leading(Marker),
    LeadingResult(Marker),
    AfterSigil(Marker),
    Body(Marker),
    BodyResult(Marker, TextSize),
    SigilProbe(bool),
    SigilToken(RuleId, bool),
    Base(base::continuation::Continuation),
    Paragraph(Box<super::document::continuation::Continuation<'static>>),
    Probe(LiteralScan<'static>),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical comment owner");
        Self {
            frames: alloc::vec![Frame::Enter(rule)],
            result: Attempt::NoMatch,
            matched: false,
            work: 0,
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
    }
    fn probe(&mut self, parser: &Parser<'_>, literal: &'static str, final_input: bool) {
        if parser.offset() > parser.cursor().end() {
            self.matched = false;
            return;
        }
        self.push(Frame::Probe(
            LiteralScan::new(
                literal,
                parser.offset(),
                final_input.then_some(parser.cursor().context_end()),
            )
            .expect("comment sigil"),
        ));
    }
    fn finish_comment(&mut self, parser: &mut Parser<'_>, marker: Marker) {
        marker.complete(parser, SyntaxKind::Comment);
        self.result = Attempt::Matched;
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
            let frame = self.frames.pop().expect("comment phase");
            if !matches!(
                frame,
                Frame::Base(_) | Frame::Probe(_) | Frame::Paragraph(_)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint, rule == rules::COMMENT));
                    if rule == rules::COMMENT {
                        self.push(Frame::Leading(parser.start()));
                    } else {
                        self.push(Frame::SigilProbe(false));
                        self.probe(parser, "--", final_input);
                    }
                }
                Frame::Exit(checkpoint, promote_halt) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    if promote_halt && parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::Leading(marker) => {
                    self.push(Frame::LeadingResult(marker));
                    self.base(rules::SPACE_TAB);
                }
                Frame::LeadingResult(marker) => {
                    if self.matched && !parser.is_halted() {
                        self.push(Frame::Leading(marker));
                    } else {
                        self.push(Frame::AfterSigil(marker));
                        self.push(Frame::Enter(rules::COMMENT_SIGIL));
                    }
                }
                Frame::AfterSigil(marker) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::Body(marker));
                    } else {
                        marker.abandon(parser);
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Body(marker) => {
                    if matches!(parser.cursor().byte(), Some(b'\r' | b'\n')) {
                        self.finish_comment(parser, marker);
                    } else if parser.is_eof() {
                        if !final_input {
                            self.push(Frame::Body(marker));
                            return Progress::NeedInput;
                        }
                        self.finish_comment(parser, marker);
                    } else {
                        self.push(Frame::BodyResult(marker, parser.offset()));
                        let rule = super::document_grammar::DOCUMENT_RULES
                            .iter()
                            .find(|rule| rule.rule == rules::PARAGRAPH_ELEMENT)
                            .expect("canonical paragraph element");
                        self.push(Frame::Paragraph(Box::new(
                            super::document::continuation::Continuation::for_rule(rule),
                        )));
                    }
                }
                Frame::BodyResult(marker, before) => {
                    if !self.matched || parser.offset() == before || parser.is_halted() {
                        self.finish_comment(parser, marker);
                    } else {
                        self.push(Frame::Body(marker));
                    }
                }
                Frame::SigilProbe(slash) => {
                    if self.matched {
                        let rule = if slash { rules::SLASH } else { rules::DASH };
                        self.push(Frame::SigilToken(rule, false));
                        self.base(rule);
                    } else if !slash {
                        self.push(Frame::SigilProbe(true));
                        self.probe(parser, "//", final_input);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::SigilToken(rule, second) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else if second {
                        self.result = Attempt::Matched;
                    } else {
                        self.push(Frame::SigilToken(rule, true));
                        self.base(rule);
                    }
                }
                Frame::Paragraph(mut continuation) => {
                    use super::document::continuation::Progress as ParagraphProgress;
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        ParagraphProgress::Complete(result) => {
                            self.matched = result != Attempt::NoMatch;
                        }
                        ParagraphProgress::NeedsProcessing => {
                            self.push(Frame::Paragraph(continuation));
                            return Progress::NeedsProcessing;
                        }
                        ParagraphProgress::NeedInput => {
                            self.push(Frame::Paragraph(continuation));
                            return Progress::NeedInput;
                        }
                        ParagraphProgress::Limited => {
                            self.push(Frame::Paragraph(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(result) => self.matched = result,
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::Base(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Probe(mut scan) => {
                    let before = *allowance;
                    let progress = scan.advance(
                        parser.source(),
                        parser.cursor().end(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(end) => self.matched = end.is_some(),
                        LiteralProgress::NeedsProcessing => {
                            self.push(Frame::Probe(scan));
                            return Progress::NeedsProcessing;
                        }
                        LiteralProgress::NeedInput => {
                            self.push(Frame::Probe(scan));
                            return Progress::NeedInput;
                        }
                        LiteralProgress::InvalidSource => {
                            panic!("comment continuation source bounds changed")
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}
fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final comment input"),
        }
    }
}
/// Parse the transparent comment-sigil owner with its exact boolean contract.
pub(crate) fn parse_comment_sigil(parser: &mut Parser<'_>) -> bool {
    parse_rule(parser, rules::COMMENT_SIGIL) == Attempt::Matched
}
/// Comment bodies share the retained rich paragraph-element grammar.
pub(crate) fn parse_comment(parser: &mut Parser<'_>) -> Attempt {
    parse_rule(parser, rules::COMMENT)
}

#[cfg(test)]
mod tests {
    use super::super::continuation_test_support::{self, Output};
    use super::*;
    use alloc::string::String;
    fn run(rule: RuleId, text: &str, chunks: &[&str], fuel: u64, step: bool) -> (Output, u64) {
        continuation_test_support::run::<Continuation>(rule, text, chunks, fuel, step)
    }

    #[test]
    fn comment_sigils_and_bodies_survive_every_scalar_cut_and_fuel_boundary() {
        let cases = [
            "",
            "-",
            "--",
            "/",
            "//",
            "--hello",
            "//hello\r\nnext",
            " \t--e\u{301}👩\u{200d}💻",
            "--\r\nnext",
            "-\u{301}-x",
            "/\u{301}/x",
            "\u{a0}--text\n",
            " //raw\\text\u{0}x",
            "--🇦🇧🇨\n",
            "-- **bold** [link](https://mech-lang.org) {ans}\nnext",
            "// __under__ `code` {{1 + 2}} {ans + 1}\r\nnext",
        ];
        for rule in [rules::COMMENT, rules::COMMENT_SIGIL] {
            for text in cases {
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                for fuel in [0, 1, 2, 4, 8, 16, 64, u64::MAX] {
                    for split in &boundaries {
                        let (observed, _) =
                            run(rule, text, &[&text[..*split], &text[*split..]], fuel, true);
                        let accepted = &text[..observed.stats.source_bytes as usize];
                        let (baseline, _) = run(rule, accepted, &[accepted], fuel, false);
                        assert_eq!(
                            observed, baseline,
                            "rule {rule:?}, split {split}, fuel {fuel}, text {text:?}"
                        );
                    }
                    let chunks: Vec<_> = boundaries
                        .windows(2)
                        .map(|pair| &text[pair[0]..pair[1]])
                        .collect();
                    let (observed, _) = run(rule, text, &chunks, fuel, true);
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (baseline, _) = run(rule, accepted, &[accepted], fuel, false);
                    assert_eq!(observed, baseline);
                }
            }
        }
    }

    #[test]
    fn rich_comment_body_uses_paragraph_nodes_and_stops_at_newline() {
        let source = "-- **bold** [link](https://mech-lang.org) `code` {ans}\nnext";
        let (parsed, _) = run(rules::COMMENT, source, &[source], u64::MAX, false);
        assert_eq!(parsed.result, Attempt::Matched);
        assert_eq!(parsed.end.to_usize(), source.find('\n').unwrap());
        for kind in ["Strong", "Hyperlink", "InlineCode", "EvalInlineMechCode"] {
            assert!(
                parsed.events.contains(kind),
                "missing {kind}: {}",
                parsed.events
            );
        }
    }

    #[test]
    fn growing_comment_text_and_leading_spacing_keep_linear_work() {
        for leading in [false, true] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = if leading {
                    " ".repeat(n) + "--x"
                } else {
                    String::from("--") + &"a".repeat(n)
                };
                let chunks: Vec<_> = text
                    .as_bytes()
                    .chunks(1)
                    .map(|byte| core::str::from_utf8(byte).unwrap())
                    .collect();
                let (observed, work) = run(rules::COMMENT, &text, &chunks, u64::MAX, true);
                let (baseline, _) = run(rules::COMMENT, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, baseline);
                assert_eq!(observed.result, Attempt::Matched);
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.stats.diagnostics_emitted, 0);
                if let Some(previous) = previous {
                    assert!(work <= previous * 3, "comment restarted on append");
                }
                previous = Some(work);
            }
        }
    }
}
