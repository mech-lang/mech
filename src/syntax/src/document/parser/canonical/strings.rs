//! Retained canonical string candidates and rule-owned recovery.
use super::combinator::Attempt;
use super::{base, combinator, literals};
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::{Parser, checkpoint::ParserCheckpoint, marker::Marker, rule::rules};
use crate::document::{ExpectedSyntax, RuleId, SyntaxKind, TextRange, TextSize};
use alloc::vec::Vec;

pub(crate) fn supports(rule: RuleId) -> bool {
    matches!(rule, rules::STRING | rules::UTF8_STRING | rules::RAW_STRING)
}

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
struct Body {
    raw: bool,
    recovery: bool,
    marker: Marker,
    opening_start: TextSize,
    opening: TextRange,
}
impl Body {
    fn kind(self) -> SyntaxKind {
        if self.raw {
            SyntaxKind::RawString
        } else {
            SyntaxKind::Utf8String
        }
    }
    fn delimiter(self) -> &'static str {
        if self.raw { "\"\"\"" } else { "\"" }
    }
    fn quote_count(self) -> u8 {
        if self.raw { 3 } else { 1 }
    }
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    RootCandidate(RuleId, Option<Marker>, u8),
    RootProbe(RuleId, Option<Marker>),
    RootRecovery(Option<Marker>),
    Candidate(bool),
    Recovery(bool),
    Open(Body, u8),
    OpenResult(Body, u8),
    Body(Body),
    BodyProbe(Body),
    TextResult(Body, TextSize),
    NewlineResult(Body, TextSize),
    Close(Body, u8),
    CloseResult(Body, u8),
    Recover(Body),
    Base(base::continuation::Continuation),
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
        assert!(supports(rule), "canonical string owner");
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
            .expect("nonempty string delimiter"),
        ));
    }
    fn scope(&mut self, parser: &mut Parser<'_>, rule: RuleId) {
        let checkpoint = parser.checkpoint();
        parser.state.rules.push_canonical(rule);
        self.push(Frame::Exit(checkpoint));
    }
    fn body(&mut self, parser: &mut Parser<'_>, raw: bool, recovery: bool) {
        let opening_start = parser.offset();
        let body = Body {
            raw,
            recovery,
            marker: parser.start(),
            opening_start,
            opening: TextRange::empty(opening_start),
        };
        self.push(Frame::Open(body, 0));
    }
    fn fail_body(&mut self, parser: &mut Parser<'_>, body: Body) {
        self.result = if body.recovery {
            literals::failed_literal(parser, body.marker, body.kind())
        } else {
            body.marker.abandon(parser);
            Attempt::NoMatch
        };
    }
    fn end_body(&mut self, body: Body) {
        self.push(if body.recovery {
            Frame::Recover(body)
        } else {
            Frame::Close(body, 0)
        });
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
            let frame = self.frames.pop().expect("pending string phase");
            if !matches!(frame, Frame::Base(_) | Frame::Probe(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    self.scope(parser, rule);
                    let marker = (rule == rules::STRING).then(|| parser.start());
                    self.push(Frame::RootCandidate(rule, marker, 0));
                    self.push(Frame::Candidate(rule != rules::UTF8_STRING));
                }
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::RootCandidate(rule, marker, index) => {
                    if self.result.accepted() {
                        if let Some(marker) = marker {
                            marker.complete(parser, SyntaxKind::StringLiteral);
                            self.result = Attempt::Matched;
                        }
                    } else if rule == rules::STRING && index == 0 {
                        self.push(Frame::RootCandidate(rule, marker, 1));
                        self.push(Frame::Candidate(false));
                    } else {
                        self.push(Frame::RootProbe(rule, marker));
                        self.probe(
                            parser,
                            if rule == rules::RAW_STRING {
                                "\"\"\""
                            } else {
                                "\""
                            },
                            final_input,
                        );
                    }
                }
                Frame::RootProbe(rule, marker) => {
                    if self.matched {
                        self.push(Frame::RootRecovery(marker));
                        self.push(Frame::Recovery(rule == rules::RAW_STRING));
                    } else {
                        if let Some(marker) = marker {
                            marker.abandon(parser);
                        }
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::RootRecovery(marker) => {
                    if let Some(marker) = marker {
                        marker.complete(parser, SyntaxKind::StringLiteral);
                    }
                }
                Frame::Candidate(raw) => {
                    self.scope(
                        parser,
                        if raw {
                            rules::RAW_STRING
                        } else {
                            rules::UTF8_STRING
                        },
                    );
                    self.body(parser, raw, false);
                }
                Frame::Recovery(raw) => self.body(parser, raw, true),
                Frame::Open(body, count) => {
                    self.push(Frame::OpenResult(body, count));
                    self.base(rules::QUOTE);
                }
                Frame::OpenResult(mut body, count) => {
                    if !self.matched {
                        self.fail_body(parser, body);
                    } else if count + 1 < body.quote_count() {
                        self.push(Frame::Open(body, count + 1));
                    } else {
                        body.opening = TextRange::new(body.opening_start, parser.offset());
                        self.push(Frame::Body(body));
                    }
                }
                Frame::Body(body) => {
                    if parser.is_eof() {
                        if !final_input {
                            self.push(Frame::Body(body));
                            return Progress::NeedInput;
                        }
                        self.end_body(body);
                    } else {
                        self.push(Frame::BodyProbe(body));
                        self.probe(parser, body.delimiter(), final_input);
                    }
                }
                Frame::BodyProbe(body) => {
                    if self.matched {
                        self.end_body(body);
                    } else {
                        self.push(Frame::TextResult(body, parser.offset()));
                        self.base(if body.raw {
                            rules::RAW_TEXT
                        } else {
                            rules::TEXT
                        });
                    }
                }
                Frame::TextResult(body, before) => {
                    if !self.matched {
                        self.push(Frame::NewlineResult(body, before));
                        self.base(rules::NEW_LINE);
                    } else if parser.offset() == before || parser.is_halted() {
                        self.end_body(body);
                    } else {
                        self.push(Frame::Body(body));
                    }
                }
                Frame::NewlineResult(body, before) => {
                    if !self.matched || parser.offset() == before || parser.is_halted() {
                        self.end_body(body);
                    } else {
                        self.push(Frame::Body(body));
                    }
                }
                Frame::Close(body, count) => {
                    self.push(Frame::CloseResult(body, count));
                    self.base(rules::QUOTE);
                }
                Frame::CloseResult(body, count) => {
                    if !self.matched {
                        self.fail_body(parser, body);
                    } else if count + 1 < body.quote_count() {
                        self.push(Frame::Close(body, count + 1));
                    } else {
                        body.marker.complete(parser, body.kind());
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Recover(body) => {
                    if body.raw {
                        literals::insert_missing_raw_closer(parser);
                    } else {
                        combinator::insert_missing(
                            parser,
                            "syntax/unclosed-utf8-string",
                            "expected a closing quote for UTF-8 string",
                            ExpectedSyntax::Token(SyntaxKind::Quote),
                            Some(SyntaxKind::Quote),
                            Some("\""),
                        );
                    }
                    literals::label_opening(
                        parser,
                        body.opening,
                        if body.raw {
                            "raw string starts here"
                        } else {
                            "opening quote is here"
                        },
                    );
                    body.marker.complete(parser, body.kind());
                    self.result = Attempt::Committed;
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(result) => self.matched = result,
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedsProcessing;
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
                        LiteralProgress::NeedInput => {
                            self.push(Frame::Probe(scan));
                            return Progress::NeedInput;
                        }
                        LiteralProgress::NeedsProcessing => {
                            self.push(Frame::Probe(scan));
                            return Progress::NeedsProcessing;
                        }
                        LiteralProgress::InvalidSource => {
                            panic!("string continuation source bounds changed")
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}

pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            Progress::NeedInput | Progress::Limited => unreachable!("final canonical string input"),
        }
    }
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
    fn raw_and_utf8_selection_recovery_and_limits_survive_every_input_cut() {
        let cases = [
            "",
            "x",
            "\"",
            "\"\"",
            "\"\"\"",
            "\"\"\"\"",
            "\"\"\"\"\"",
            "\"\"\"\"\"\"",
            "\"hello\"x",
            "\"hello",
            "\"e\u{301}\r\n👩\u{200d}💻\"!",
            "\"\\u{1f600}\"",
            "\"\\u{invalid}\"",
            "\"\\!\u{301}\"",
            "\"\"\"hello\"\"\"x",
            "\"\"\"hello",
            "\"\"\"raw\\x\r\ntext\"\"\"",
            "\"a\"\u{301}b\"",
            "\"\\",
            "\"a\u{0}b\"",
        ];
        for rule in [rules::STRING, rules::UTF8_STRING, rules::RAW_STRING] {
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
                            "rule {rule:?}, split {split}, fuel {fuel}, source {text:?}"
                        );
                    }
                    let chunks: Vec<_> = boundaries
                        .windows(2)
                        .map(|pair| &text[pair[0]..pair[1]])
                        .collect();
                    let (observed, _) = run(rule, text, &chunks, fuel, true);
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (baseline, _) = run(rule, accepted, &[accepted], fuel, false);
                    assert_eq!(
                        observed, baseline,
                        "scalar chunks rule {rule:?}, fuel {fuel}, source {text:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn unfinished_string_bodies_have_linear_work_including_final_candidate_replay() {
        for raw in [false, true] {
            for closed in [false, true] {
                let mut previous = None;
                for n in [64, 128, 256, 512] {
                    let delimiter = if raw { "\"\"\"" } else { "\"" };
                    let text = String::from(delimiter)
                        + &"a".repeat(n)
                        + if closed { delimiter } else { "" };
                    let chunks: Vec<_> = text
                        .as_bytes()
                        .chunks(1)
                        .map(|byte| core::str::from_utf8(byte).unwrap())
                        .collect();
                    let (observed, work) = run(rules::STRING, &text, &chunks, u64::MAX, true);
                    let (baseline, _) = run(rules::STRING, &text, &[&text], u64::MAX, false);
                    assert_eq!(observed, baseline);
                    assert!(observed.result.accepted());
                    if closed {
                        assert_eq!(observed.end.to_usize(), text.len());
                        assert_eq!(observed.stats.diagnostics_emitted, 0);
                    }
                    if let Some(previous) = previous {
                        assert!(work <= previous * 3, "string candidate restarted on append");
                    }
                    previous = Some(work);
                }
            }
        }
    }
}
