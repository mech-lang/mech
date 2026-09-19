//! Retained paragraph text and definition lookahead. The lookahead's proven
//! whitespace range is reused while the canonical TEXT children consume it.
use super::{
    base,
    combinator::Attempt,
    terminal_spec::{TerminalSpacing, fixed_terminal_spec},
};
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::{Parser, checkpoint::ParserCheckpoint, marker::Marker, rule::rules};
use crate::document::{RuleId, SyntaxKind, TextRange, TextSize};
use alloc::vec::Vec;

const EXACT_EXCLUSIONS: &[RuleId] = &[
    rules::SECTION_SIGIL,
    rules::FOOTNOTE_PREFIX,
    rules::HIGHLIGHT_SIGIL,
    rules::EQUATION_SIGIL,
    rules::IMG_PREFIX,
    rules::HTTP_PREFIX,
    rules::LEFT_BRACE,
    rules::LEFT_BRACKET,
    rules::LEFT_ANGLE1,
    rules::LEFT_ANGLE2,
    rules::RIGHT_BRACKET,
    rules::TILDE,
    rules::ASTERISK,
    rules::UNDERSCORE,
    rules::GRAVE,
    rules::BAR,
    rules::MIKA_SECTION_OPEN,
    rules::MIKA_SECTION_CLOSE,
];
pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
enum Frame {
    Enter,
    Exit(ParserCheckpoint),
    Exclusion(usize),
    ExclusionResult(usize),
    Definition,
    DefinitionScan(TextSize, TextSize),
    DefinitionResult(TextRange),
    Probe(LiteralScan<'static>),
    Text,
    TextResult(TextSize),
    Base(base::continuation::Continuation),
    Finish,
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    marker: Option<Marker>,
    start: TextSize,
    result: Attempt,
    matched: bool,
    definition: Option<(TextRange, bool)>,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert_eq!(rule, rules::PARAGRAPH_TEXT);
        Self {
            frames: alloc::vec![Frame::Enter],
            marker: None,
            start: TextSize::ZERO,
            result: Attempt::NoMatch,
            matched: false,
            definition: None,
            work: 0,
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn probe(
        &mut self,
        parser: &Parser<'_>,
        start: TextSize,
        literal: &'static str,
        final_input: bool,
    ) {
        if start > parser.cursor().end() {
            self.matched = false;
            return;
        }
        self.push(Frame::Probe(
            LiteralScan::new(
                literal,
                start,
                final_input.then_some(parser.cursor().context_end()),
            )
            .expect("nonempty prose exclusion"),
        ));
    }
    fn after_definition(&mut self, matched: bool) {
        self.push(if matched { Frame::Finish } else { Frame::Text });
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
            let frame = self.frames.pop().expect("prose phase");
            if !matches!(frame, Frame::Base(_) | Frame::Probe(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rules::PARAGRAPH_TEXT);
                    self.push(Frame::Exit(checkpoint));
                    self.marker = Some(parser.start());
                    self.start = parser.offset();
                    self.push(Frame::Exclusion(0));
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
                Frame::Exclusion(index) => {
                    if let Some(spec) = EXACT_EXCLUSIONS
                        .get(index)
                        .and_then(|rule| fixed_terminal_spec(*rule))
                    {
                        if spec.spacing == TerminalSpacing::Exact {
                            self.push(Frame::ExclusionResult(index));
                            self.probe(parser, parser.offset(), spec.literal, final_input);
                        } else {
                            self.push(Frame::Exclusion(index + 1));
                        }
                    } else if index < EXACT_EXCLUSIONS.len() {
                        self.push(Frame::Exclusion(index + 1));
                    } else {
                        self.push(Frame::Definition);
                    }
                }
                Frame::ExclusionResult(index) => {
                    self.push(if self.matched {
                        Frame::Finish
                    } else {
                        Frame::Exclusion(index + 1)
                    });
                }
                Frame::Definition => {
                    if let Some((_, matched)) = self.definition.filter(|(range, _)| {
                        range.start <= parser.offset() && parser.offset() <= range.end
                    }) {
                        self.after_definition(matched);
                    } else {
                        self.push(Frame::DefinitionScan(parser.offset(), parser.offset()));
                    }
                }
                Frame::DefinitionScan(start, at) => {
                    if at < parser.cursor().end()
                        && matches!(
                            parser.source().byte_at(at),
                            Some(b' ' | b'\t' | b'\r' | b'\n')
                        )
                    {
                        // All four spelling bytes are ASCII scalars. Advancing CR
                        // and LF separately yields the same physical lookahead
                        // frontier without treating a split CR as final input.
                        self.push(Frame::DefinitionScan(start, at + TextSize(1)));
                    } else if at == parser.cursor().end() && !final_input {
                        self.push(Frame::DefinitionScan(start, at));
                        return Progress::NeedInput;
                    } else {
                        self.push(Frame::DefinitionResult(TextRange::new(start, at)));
                        self.probe(parser, at, ":=", final_input);
                    }
                }
                Frame::DefinitionResult(range) => {
                    self.definition = Some((range, self.matched));
                    self.after_definition(self.matched);
                }
                Frame::Text => {
                    self.push(Frame::TextResult(parser.offset()));
                    self.push(Frame::Base(base::continuation::Continuation::new(
                        rules::TEXT,
                    )));
                }
                Frame::TextResult(before) => {
                    self.push(
                        if !self.matched || parser.offset() == before || parser.is_halted() {
                            Frame::Finish
                        } else {
                            Frame::Exclusion(0)
                        },
                    );
                }
                Frame::Finish => {
                    let marker = self.marker.take().expect("paragraph owner");
                    if parser.offset() == self.start {
                        marker.abandon(parser);
                        self.result = Attempt::NoMatch;
                    } else {
                        marker.complete(parser, SyntaxKind::ParagraphText);
                        self.result = Attempt::Matched;
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
                            panic!("prose continuation source bounds changed")
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
            _ => unreachable!("final prose input"),
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
    fn paragraph_exclusions_definition_lookahead_and_limits_survive_all_input_cuts() {
        let cases = [
            "",
            "hello",
            "hello world!",
            "  :=x",
            "foo   :=x",
            "foo   :x",
            "foo   :",
            "foo \r\n\t:=x",
            "foo\r\nbar",
            "e\u{301}👩\u{200d}💻 text",
            "plain [link]",
            "plain http://x",
            "plain [[ref]]",
            "plain **bold",
            "plain ╭◉╮",
            "plain $$math",
            "plain \\!\u{301}",
            "x \u{301}:=y",
            "   ",
        ];
        for text in cases {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            for fuel in [0, 1, 2, 4, 8, 16, 64, u64::MAX] {
                for split in &boundaries {
                    let (observed, _) = run(
                        rules::PARAGRAPH_TEXT,
                        text,
                        &[&text[..*split], &text[*split..]],
                        fuel,
                        true,
                    );
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (baseline, _) =
                        run(rules::PARAGRAPH_TEXT, accepted, &[accepted], fuel, false);
                    assert_eq!(
                        observed, baseline,
                        "split {split}, fuel {fuel}, source {text:?}"
                    );
                }
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, _) = run(rules::PARAGRAPH_TEXT, text, &chunks, fuel, true);
                let accepted = &text[..observed.stats.source_bytes as usize];
                let (baseline, _) = run(rules::PARAGRAPH_TEXT, accepted, &[accepted], fuel, false);
                assert_eq!(observed, baseline);
            }
        }
    }

    #[test]
    fn paragraph_scanning_and_shared_whitespace_lookahead_grow_linearly() {
        for tail in ["x", ":=x", ""] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from("word") + &" ".repeat(n) + tail;
                let chunks: Vec<_> = text
                    .as_bytes()
                    .chunks(1)
                    .map(|byte| core::str::from_utf8(byte).unwrap())
                    .collect();
                let (observed, work) = run(rules::PARAGRAPH_TEXT, &text, &chunks, u64::MAX, true);
                let (baseline, one_shot_work) =
                    run(rules::PARAGRAPH_TEXT, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, baseline);
                assert_eq!(observed.result, Attempt::Matched);
                assert_eq!(
                    observed.end.to_usize(),
                    if tail == ":=x" { 4 } else { text.len() }
                );
                assert_eq!(observed.stats.diagnostics_emitted, 0);
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(
                        work <= prior_streamed * 3,
                        "append restarted prose lookahead"
                    );
                    assert!(
                        one_shot_work <= prior_one_shot * 3,
                        "settled whitespace was scanned repeatedly"
                    );
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
