//! Retained Mechdown prefixes, physical delimiters, body children, and recovery.
use super::*;
use crate::document::TextSize;
use crate::document::parser::{
    checkpoint::ParserCheckpoint,
    literal_scan::{LiteralProgress, LiteralScan},
    marker::Marker,
};
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
struct Body {
    rule: RuleId,
    marker: Marker,
    opening: TextRange,
    content_start: TextSize,
}
impl Body {
    fn kind(self) -> SyntaxKind {
        match self.rule {
            rules::INLINE_CODE => SyntaxKind::InlineCode,
            rules::INLINE_EQUATION => SyntaxKind::InlineEquation,
            rules::FOOTNOTE_REFERENCE => SyntaxKind::FootnoteReference,
            rules::REFERENCE => SyntaxKind::Reference,
            rules::SECTION_REFERENCE => SyntaxKind::SectionReference,
            rules::RAW_HYPERLINK => SyntaxKind::RawHyperlink,
            rules::EQUATION => SyntaxKind::Equation,
            _ => unreachable!("Mechdown body owner"),
        }
    }
    fn closer(self) -> Option<RuleId> {
        match self.rule {
            rules::INLINE_CODE => Some(rules::GRAVE),
            rules::INLINE_EQUATION => Some(rules::EQUATION_SIGIL),
            rules::FOOTNOTE_REFERENCE | rules::REFERENCE => Some(rules::RIGHT_BRACKET),
            _ => None,
        }
    }
    fn equation(self) -> bool {
        matches!(self.rule, rules::INLINE_EQUATION | rules::EQUATION)
    }
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint, bool),
    Sigil(bool),
    InlineProbe,
    HyperlinkProbe,
    Open(Body),
    Body(Body),
    Delimiter(Body),
    Element(Body, TextSize, u8),
    End(Body),
    Close(Body, bool),
    ReferenceFirst(Body),
    ReferenceClose(Body),
    ThematicFirst(Marker),
    ThematicRepeat(Marker),
    LineSpace(Marker, SyntaxKind),
    LineEnd(Marker, SyntaxKind),
    Base(base::continuation::Continuation),
    Probe(LiteralScan<'static>),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    pub(super) result: Attempt,
    pub(super) delimiter: Option<CodeblockDelimiter>,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical Mechdown owner");
        Self {
            frames: alloc::vec![Frame::Enter(rule)],
            result: Attempt::NoMatch,
            delimiter: None,
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
    fn probe(&mut self, parser: &Parser<'_>, rule: RuleId, final_input: bool) {
        if parser.offset() > parser.cursor().end() {
            self.matched = false;
            return;
        }
        let literal = fixed_terminal_spec(rule)
            .expect("exact Mechdown terminal")
            .literal;
        self.push(Frame::Probe(
            LiteralScan::new(
                literal,
                parser.offset(),
                final_input.then_some(parser.cursor().context_end()),
            )
            .expect("nonempty Mechdown terminal"),
        ));
    }
    fn open(&mut self, parser: &mut Parser<'_>, rule: RuleId, opening: RuleId) {
        let start = parser.offset();
        let body = Body {
            rule,
            marker: parser.start(),
            opening: TextRange::empty(start),
            content_start: start,
        };
        self.push(Frame::Open(body));
        self.base(opening);
    }
    fn element(&mut self, parser: &Parser<'_>, body: Body) {
        self.push(Frame::Element(body, parser.offset(), 0));
        self.base(match body.rule {
            rules::INLINE_EQUATION | rules::EQUATION => rules::BACKSLASH,
            rules::REFERENCE | rules::SECTION_REFERENCE => rules::ALPHANUMERIC,
            _ => rules::TEXT,
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
            let frame = self.frames.pop().expect("retained Mechdown phase");
            if !matches!(frame, Frame::Base(_) | Frame::Probe(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint, rule != rules::CODEBLOCK_SIGIL));
                    match rule {
                        rules::CODEBLOCK_SIGIL => {
                            self.push(Frame::Sigil(false));
                            self.base(rules::GRAVE_CODEBLOCK_SIGIL);
                        }
                        rules::INLINE_CODE => {
                            self.push(Frame::InlineProbe);
                            self.probe(parser, rules::GRAVE_CODEBLOCK_SIGIL, final_input);
                        }
                        rules::RAW_HYPERLINK => {
                            self.push(Frame::HyperlinkProbe);
                            self.probe(parser, rules::HTTP_PREFIX, final_input);
                        }
                        rules::INLINE_EQUATION | rules::EQUATION => {
                            self.open(parser, rule, rules::EQUATION_SIGIL)
                        }
                        rules::FOOTNOTE_REFERENCE => {
                            self.open(parser, rule, rules::FOOTNOTE_PREFIX)
                        }
                        rules::REFERENCE => self.open(parser, rule, rules::LEFT_BRACKET),
                        rules::SECTION_REFERENCE => self.open(parser, rule, rules::SECTION_SIGIL),
                        rules::THEMATIC_BREAK => {
                            self.push(Frame::ThematicFirst(parser.start()));
                            self.base(rules::ASTERISK);
                        }
                        rules::BLANK_LINE => {
                            self.push(Frame::LineSpace(parser.start(), SyntaxKind::BlankLine));
                            self.base(rules::SPACE_TAB0);
                        }
                        _ => unreachable!("supported Mechdown rule"),
                    }
                }
                Frame::Exit(checkpoint, promote_halt) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    // Let the owning transaction discard provisional wrappers even
                    // after a child finalized a nonrefundable resource remainder.
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    if promote_halt && parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::Sigil(tilde) => {
                    if self.matched {
                        self.delimiter = Some(if tilde {
                            CodeblockDelimiter::Tilde
                        } else {
                            CodeblockDelimiter::Grave
                        });
                        self.result = Attempt::Matched;
                    } else if !tilde {
                        self.push(Frame::Sigil(true));
                        self.base(rules::TILDE_CODEBLOCK_SIGIL);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::InlineProbe => {
                    if self.matched {
                        self.result = Attempt::NoMatch;
                    } else {
                        self.open(parser, rules::INLINE_CODE, rules::GRAVE);
                    }
                }
                Frame::HyperlinkProbe => {
                    if self.matched {
                        let start = parser.offset();
                        self.push(Frame::Body(Body {
                            rule: rules::RAW_HYPERLINK,
                            marker: parser.start(),
                            opening: TextRange::empty(start),
                            content_start: start,
                        }));
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Open(mut body) => {
                    if self.matched {
                        body.opening.end = parser.offset();
                        body.content_start = parser.offset();
                        if body.rule == rules::REFERENCE {
                            self.push(Frame::ReferenceFirst(body));
                            self.base(rules::ALPHANUMERIC);
                        } else {
                            self.push(Frame::Body(body));
                        }
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::ReferenceFirst(body) => {
                    if self.matched {
                        self.push(Frame::Body(body));
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Body(body) => {
                    if parser.is_eof() && !final_input {
                        self.push(Frame::Body(body));
                        return Progress::NeedInput;
                    }
                    if matches!(body.rule, rules::REFERENCE | rules::SECTION_REFERENCE) {
                        self.element(parser, body);
                    } else if body.rule == rules::RAW_HYPERLINK {
                        if parser.is_eof() || parser.cursor().byte() == Some(b' ') {
                            self.push(Frame::End(body));
                        } else {
                            self.element(parser, body);
                        }
                    } else if parser.is_eof()
                        || matches!(parser.cursor().byte(), Some(b'\r' | b'\n'))
                    {
                        self.push(Frame::End(body));
                    } else if let Some(closer) = body.closer() {
                        self.push(Frame::Delimiter(body));
                        self.probe(parser, closer, final_input);
                    } else {
                        self.element(parser, body);
                    }
                }
                Frame::Delimiter(body) => {
                    if self.matched {
                        self.push(Frame::End(body));
                    } else {
                        self.element(parser, body);
                    }
                }
                Frame::Element(body, before, alternative) => {
                    if !self.matched
                        && alternative == 0
                        && (body.equation() || body.rule == rules::SECTION_REFERENCE)
                    {
                        self.push(Frame::Element(body, before, 1));
                        self.base(if body.equation() {
                            rules::TEXT
                        } else {
                            rules::PERIOD
                        });
                    } else if self.matched && !parser.is_halted() && parser.offset() != before {
                        self.push(Frame::Body(body));
                    } else {
                        self.push(Frame::End(body));
                    }
                }
                Frame::End(body) => {
                    if body.rule == rules::REFERENCE {
                        self.push(Frame::ReferenceClose(body));
                        self.base(rules::RIGHT_BRACKET);
                    } else {
                        let empty = parser.offset() == body.content_start;
                        if empty && body.rule == rules::RAW_HYPERLINK {
                            self.result = Attempt::NoMatch;
                        } else {
                            let missing = empty && body.rule != rules::INLINE_CODE;
                            if missing {
                                missing_content(parser, body);
                            }
                            if let Some(closer) = body.closer() {
                                self.push(Frame::Close(body, missing));
                                self.base(closer);
                            } else {
                                body.marker.complete(parser, body.kind());
                                self.result = if missing {
                                    Attempt::Committed
                                } else {
                                    Attempt::Matched
                                };
                            }
                        }
                    }
                }
                Frame::Close(body, missing) => {
                    if !self.matched {
                        missing_closer(parser, body);
                    }
                    body.marker.complete(parser, body.kind());
                    self.result = if missing || !self.matched {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
                Frame::ReferenceClose(body) => {
                    if self.matched {
                        body.marker.complete(parser, body.kind());
                        self.result = Attempt::Matched;
                    } else if parser.is_eof()
                        || matches!(parser.cursor().byte(), Some(b'\r' | b'\n'))
                    {
                        if parser.is_eof() && !final_input {
                            self.push(Frame::ReferenceClose(body));
                            return Progress::NeedInput;
                        }
                        missing_closer(parser, body);
                        body.marker.complete(parser, body.kind());
                        self.result = Attempt::Committed;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::ThematicFirst(marker) => {
                    if self.matched {
                        self.push(Frame::ThematicRepeat(marker));
                        self.base(rules::ASTERISK);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::ThematicRepeat(marker) => {
                    if self.matched && !parser.is_halted() {
                        self.push(Frame::ThematicRepeat(marker));
                        self.base(rules::ASTERISK);
                    } else {
                        self.push(Frame::LineSpace(marker, SyntaxKind::ThematicBreak));
                        self.base(rules::SPACE_TAB0);
                    }
                }
                Frame::LineSpace(marker, kind) => {
                    self.push(Frame::LineEnd(marker, kind));
                    self.base(rules::NEW_LINE);
                }
                Frame::LineEnd(marker, kind) => {
                    if self.matched {
                        marker.complete(parser, kind);
                        self.result = Attempt::Matched;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(matched) => self.matched = matched,
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
                            panic!("Mechdown continuation source bounds changed")
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}

fn missing_content(parser: &mut Parser<'_>, body: Body) {
    let (code, message, production, label) = match body.rule {
        rules::INLINE_EQUATION => (
            "syntax/missing-inline-equation-content",
            "expected inline equation content",
            "inline equation content",
            "inline equation starts here",
        ),
        rules::FOOTNOTE_REFERENCE => (
            "syntax/missing-footnote-reference-content",
            "expected footnote reference content",
            "footnote reference content",
            "footnote reference starts here",
        ),
        rules::SECTION_REFERENCE => (
            "syntax/missing-section-reference",
            "expected a section reference after the section sigil",
            "section reference",
            "section reference starts here",
        ),
        rules::EQUATION => (
            "syntax/missing-equation-content",
            "expected block equation content",
            "equation content",
            "equation starts here",
        ),
        _ => unreachable!("Mechdown content recovery owner"),
    };
    combinator::insert_missing(
        parser,
        code,
        message,
        ExpectedSyntax::Production(String::from(production)),
        None,
        None,
    );
    label_opening(parser, body.opening, label);
}
fn missing_closer(parser: &mut Parser<'_>, body: Body) {
    let (code, message, kind, text, label) = match body.rule {
        rules::INLINE_CODE => (
            "syntax/unclosed-inline-code",
            "expected a closing grave for inline code",
            SyntaxKind::Grave,
            "`",
            Some("inline code starts here"),
        ),
        rules::INLINE_EQUATION => (
            "syntax/unclosed-inline-equation",
            "expected a closing equation sigil",
            SyntaxKind::EquationSigil,
            "$$",
            Some("inline equation starts here"),
        ),
        rules::FOOTNOTE_REFERENCE => (
            "syntax/unclosed-footnote-reference",
            "expected a closing bracket for the footnote reference",
            SyntaxKind::RightBracket,
            "]",
            Some("footnote reference starts here"),
        ),
        rules::REFERENCE => (
            "syntax/unclosed-reference",
            "expected a closing bracket for the reference",
            SyntaxKind::RightBracket,
            "]",
            None,
        ),
        _ => unreachable!("Mechdown closing recovery owner"),
    };
    combinator::insert_missing(
        parser,
        code,
        message,
        ExpectedSyntax::Token(kind),
        Some(kind),
        Some(text),
    );
    if let Some(label) = label {
        label_opening(parser, body.opening, label);
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::continuation_test_support::{assert_partitions, run};
    use super::*;
    #[test]
    fn mechdown_delimiters_recovery_and_limits_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::CODEBLOCK_SIGIL, ""),
            (rules::CODEBLOCK_SIGIL, "`"),
            (rules::CODEBLOCK_SIGIL, "``"),
            (rules::CODEBLOCK_SIGIL, "```"),
            (rules::CODEBLOCK_SIGIL, "~~~"),
            (rules::CODEBLOCK_SIGIL, "~~~\u{301}"),
            (rules::INLINE_CODE, "`"),
            (rules::INLINE_CODE, "``"),
            (rules::INLINE_CODE, "```code"),
            (rules::INLINE_CODE, "`hello`"),
            (rules::INLINE_CODE, "`e\u{301}👩\u{200d}💻`"),
            (rules::INLINE_CODE, "`hello\r\nnext"),
            (rules::INLINE_CODE, "`\\u{41}`"),
            (rules::INLINE_EQUATION, "$"),
            (rules::INLINE_EQUATION, "$$"),
            (rules::INLINE_EQUATION, "$$$$"),
            (rules::INLINE_EQUATION, "$$x+1$$"),
            (rules::INLINE_EQUATION, "$$\\alpha$$"),
            (rules::INLINE_EQUATION, "$$x$"),
            (rules::INLINE_EQUATION, "$$x\r\n"),
            (rules::EQUATION, "$$"),
            (rules::EQUATION, "$$x+\\alpha\r\n"),
            (rules::RAW_HYPERLINK, "http://"),
            (rules::RAW_HYPERLINK, "http://host/x?y=1 rest"),
            (rules::RAW_HYPERLINK, "http://e\u{301}/👩\u{200d}💻"),
            (rules::RAW_HYPERLINK, "http://x\tmore\r\n"),
            (rules::FOOTNOTE_REFERENCE, "[^"),
            (rules::FOOTNOTE_REFERENCE, "[^]"),
            (rules::FOOTNOTE_REFERENCE, "[^hello]"),
            (rules::FOOTNOTE_REFERENCE, "[^e\u{301}\r\n"),
            (rules::REFERENCE, "["),
            (rules::REFERENCE, "[]"),
            (rules::REFERENCE, "[abc]"),
            (rules::REFERENCE, "[abc"),
            (rules::REFERENCE, "[abc def]"),
            (rules::REFERENCE, "[a\r\n"),
            (rules::SECTION_REFERENCE, "§"),
            (rules::SECTION_REFERENCE, "§1.2.α"),
            (rules::SECTION_REFERENCE, "§e\u{301}"),
            (rules::THEMATIC_BREAK, "*"),
            (rules::THEMATIC_BREAK, "***\t\r\n"),
            (rules::THEMATIC_BREAK, "*\u{301}\n"),
            (rules::BLANK_LINE, ""),
            (rules::BLANK_LINE, " \t"),
            (rules::BLANK_LINE, " \t\r\n"),
            (rules::BLANK_LINE, "\nnext"),
        ]);
    }
    #[test]
    fn mechdown_unfinished_bodies_and_line_spacing_retain_linear_work() {
        for (rule, prefix, unit, tails) in [
            (rules::INLINE_CODE, "`", "a", ["", "`"]),
            (rules::INLINE_EQUATION, "$$", "a", ["", "$$"]),
            (rules::EQUATION, "$$", "a", ["", "\r\n"]),
            (rules::RAW_HYPERLINK, "http://", "a", ["", " "]),
            (rules::FOOTNOTE_REFERENCE, "[^", "a", ["", "]"]),
            (rules::REFERENCE, "[", "a", ["", "]"]),
            (rules::SECTION_REFERENCE, "§", "a", ["", "\r\n"]),
            (rules::THEMATIC_BREAK, "", "*", ["", "\r\n"]),
            (rules::BLANK_LINE, "", " ", ["", "\r\n"]),
        ] {
            for tail in tails {
                let mut previous = None;
                for n in [64, 128, 256, 512] {
                    let text = String::from(prefix) + &unit.repeat(n) + tail;
                    let boundaries: Vec<_> = text
                        .char_indices()
                        .map(|(at, _)| at)
                        .chain(core::iter::once(text.len()))
                        .collect();
                    let chunks: Vec<_> = boundaries
                        .windows(2)
                        .map(|pair| &text[pair[0]..pair[1]])
                        .collect();
                    let (observed, work) =
                        run::<Continuation>(rule, &text, &chunks, u64::MAX, true);
                    let (baseline, one_shot_work) =
                        run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                    assert_eq!(observed, baseline, "{rule:?}, tail {tail:?}");
                    assert_eq!(observed.stats.source_bytes as usize, text.len());
                    if let Some((prior_streamed, prior_one_shot)) = previous {
                        assert!(work <= prior_streamed * 3, "append restarted {rule:?}");
                        assert!(one_shot_work <= prior_one_shot * 3);
                    }
                    previous = Some((work, one_shot_work));
                }
            }
        }
    }
}
