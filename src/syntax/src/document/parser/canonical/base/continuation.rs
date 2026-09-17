//! Canonical base rules own their token, alternative, and repetition phases.
use super::*;
use crate::document::parser::grapheme_scan::{GraphemeScan, ScanProgress};
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::{checkpoint::ParserCheckpoint, marker::Marker};
use crate::document::{TextRange, TextSize};
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(bool),
    NeedInput,
    NeedsProcessing,
    Limited,
}

#[derive(Clone, Copy)]
struct UnicodeEscape {
    checkpoint: ParserCheckpoint,
    start: TextSize,
    scalar: Option<u32>,
    digits: usize,
}

enum Frame {
    Call(RuleId),
    Exit(ParserCheckpoint),
    Sequence(&'static [RuleId], usize),
    Choice(&'static [RuleId], usize),
    Repeat(&'static [RuleId], TextSize),
    FirstRepeat(&'static [RuleId]),
    CompleteNode(Marker, SyntaxKind),
    Unless(ParserCheckpoint, RuleId),
    Fixed(&'static FixedTerminalSpec),
    FixedAfterLeading(&'static FixedTerminalSpec),
    FixedAfterToken,
    Literal(LiteralScan<'static>, Option<SyntaxKind>),
    LiteralResult(Option<TextSize>, Option<SyntaxKind>),
    Classified(SyntaxKind, fn(char) -> bool),
    Raw(Option<SyntaxKind>),
    Grapheme(GraphemeScan, Option<SyntaxKind>),
    GraphemeResult(Option<TextRange>, Option<SyntaxKind>),
    EscapeAfterSlash,
    EscapeZero,
    EscapeUnicode,
    EscapeFallback,
    EscapeSymbol(usize),
    EscapeSymbolResult(usize),
    UnicodePrefix(UnicodeEscape, u8),
    UnicodeDigit(UnicodeEscape),
    UnicodeDigitResult(UnicodeEscape),
    UnicodeCloser(UnicodeEscape),
    UnicodeComplete(UnicodeEscape),
}

pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: bool,
    raw_range: Option<TextRange>,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        Self {
            frames: alloc::vec![Frame::Call(rule)],
            result: false,
            raw_range: None,
            work: 0,
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn choice(&mut self, rules: &'static [RuleId]) {
        self.result = false;
        if let Some(rule) = rules.first() {
            self.push(Frame::Choice(rules, 0));
            self.push(Frame::Call(*rule));
        }
    }
    fn sequence(&mut self, rules: &'static [RuleId]) {
        self.result = true;
        if let Some(rule) = rules.first() {
            self.push(Frame::Sequence(rules, 0));
            self.push(Frame::Call(*rule));
        }
    }
    fn repeat(&mut self, parser: &Parser<'_>, rules: &'static [RuleId]) {
        self.push(Frame::Repeat(rules, parser.offset()));
        self.choice(rules);
    }
    fn one_or_more(&mut self, rules: &'static [RuleId]) {
        self.push(Frame::FirstRepeat(rules));
        self.choice(rules);
    }
    fn node(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind) {
        self.push(Frame::CompleteNode(parser.start(), kind));
    }
    fn literal(
        &mut self,
        parser: &Parser<'_>,
        literal: &'static str,
        kind: Option<SyntaxKind>,
        final_input: bool,
    ) {
        // Resource finalization owns the whole accepted document remainder,
        // including bytes past an embedded fence's local consume bound. Failed
        // speculative terminals during unwind must not construct an out-of-range
        // Unicode cursor at that document-wide frontier.
        if parser.offset() > parser.cursor().end() {
            self.result = false;
            return;
        }
        self.push(Frame::Literal(
            LiteralScan::new(
                literal,
                parser.offset(),
                final_input.then_some(parser.cursor().context_end()),
            )
            .expect("nonempty canonical literal"),
            kind,
        ));
    }
    fn unless(&mut self, parser: &Parser<'_>, excluded: &'static [RuleId], rule: RuleId) {
        self.push(Frame::Unless(parser.checkpoint(), rule));
        self.choice(excluded);
    }
    fn call(&mut self, parser: &mut Parser<'_>, rule: RuleId) {
        if !supports(rule) {
            self.result = false;
            return;
        }
        let checkpoint = parser.checkpoint();
        parser.state.rules.push_canonical(rule);
        self.push(Frame::Exit(checkpoint));
        if let Some(spec) = fixed_terminal_spec(rule) {
            self.push(Frame::Fixed(spec));
            return;
        }
        match rule {
            rules::TRANSITION_OPERATOR => {
                self.choice(&[rules::TRANSITION_OPERATOR_A, rules::TRANSITION_OPERATOR_U])
            }
            rules::OUTPUT_OPERATOR => {
                self.choice(&[rules::OUTPUT_OPERATOR_A, rules::OUTPUT_OPERATOR_U])
            }
            rules::EMOJI_GRAPHEME => self.push(Frame::Classified(SyntaxKind::Emoji, is_emoji)),
            rules::ALPHA => self.push(Frame::Classified(SyntaxKind::Alpha, char::is_alphabetic)),
            rules::DIGIT => self.push(Frame::Classified(SyntaxKind::Digit, char::is_numeric)),
            rules::ANY => self.push(Frame::Raw(Some(SyntaxKind::Any))),
            rules::ANY_TOKEN => self.push(Frame::Call(rules::ANY)),
            rules::ALPHA_TOKEN => self.push(Frame::Call(rules::ALPHA)),
            rules::DIGIT_TOKEN => self.push(Frame::Call(rules::DIGIT)),
            rules::FORBIDDEN_EMOJI => self.choice(FORBIDDEN_EMOJI_RULES),
            rules::EMOJI => self.unless(parser, &[rules::FORBIDDEN_EMOJI], rules::EMOJI_GRAPHEME),
            rules::ALPHANUMERIC => self.choice(&[rules::ALPHA_TOKEN, rules::DIGIT_TOKEN]),
            rules::UNDERSCORE_DIGIT => self.sequence(&[rules::UNDERSCORE, rules::DIGIT_TOKEN]),
            rules::DIGIT_SEQUENCE => {
                self.node(parser, SyntaxKind::DigitSequence);
                self.push(Frame::FirstRepeat(&[
                    rules::UNDERSCORE_DIGIT,
                    rules::DIGIT_TOKEN,
                ]));
                self.push(Frame::Call(rules::DIGIT_TOKEN));
            }
            rules::GROUPING_SYMBOL => self.choice(GROUPING_SYMBOL_RULES),
            rules::PUNCTUATION => self.choice(PUNCTUATION_RULES),
            rules::ESCAPED_CHAR => {
                self.node(parser, SyntaxKind::EscapedCharacter);
                self.push(Frame::EscapeAfterSlash);
                self.push(Frame::Call(rules::BACKSLASH));
            }
            rules::SYMBOL => self.choice(SYMBOL_RULES),
            rules::IDENTIFIER_SYMBOL => self.choice(IDENTIFIER_SYMBOL_RULES),
            rules::TEXT => self.choice(TEXT_RULES),
            rules::RAW_TEXT => self.choice(RAW_TEXT_RULES),
            rules::NEW_LINE => self.choice(&[
                rules::CARRIAGE_RETURN_NEW_LINE,
                rules::NEW_LINE_CHAR,
                rules::CARRIAGE_RETURN,
            ]),
            rules::WHITESPACE => self.choice(&[rules::SPACE, rules::TAB, rules::NEW_LINE]),
            rules::WHITESPACE0 => self.repeat(parser, &[rules::WHITESPACE]),
            rules::WHITESPACE1 => self.one_or_more(&[rules::WHITESPACE]),
            rules::NEWLINE_INDENT => {
                self.push(Frame::FirstRepeat(&[rules::SPACE_TAB]));
                self.push(Frame::Call(rules::NEW_LINE));
            }
            rules::WS1E | rules::SPACE_TAB1 => self.one_or_more(&[rules::SPACE_TAB]),
            rules::WS0E | rules::SPACE_TAB0 => self.repeat(parser, &[rules::SPACE_TAB]),
            rules::SPACE_TAB => {
                self.choice(&[rules::SPACE, rules::TAB, rules::NBSP, rules::THIN_SPACE])
            }
            rules::LIST_SEPARATOR => {
                self.sequence(&[rules::WHITESPACE0, rules::COMMA, rules::WHITESPACE0])
            }
            rules::ENUM_SEPARATOR => {
                self.sequence(&[rules::WHITESPACE0, rules::BAR, rules::WHITESPACE0])
            }
            rules::IDENTIFIER => {
                self.node(parser, SyntaxKind::Identifier);
                self.push(Frame::FirstRepeat(&[
                    rules::ALPHA_TOKEN,
                    rules::DIGIT_TOKEN,
                    rules::IDENTIFIER_SYMBOL,
                    rules::EMOJI,
                ]));
                self.choice(&[rules::ALPHA_TOKEN, rules::EMOJI]);
            }
            rules::IDENTIFIER_PATH_SEGMENT_EMOJI => {
                self.unless(parser, PATH_EMOJI_EXCLUSIONS, rules::EMOJI)
            }
            rules::IDENTIFIER_PATH_SEGMENT => {
                self.node(parser, SyntaxKind::IdentifierPathSegment);
                self.push(Frame::FirstRepeat(&[
                    rules::ALPHA_TOKEN,
                    rules::DIGIT_TOKEN,
                    rules::DASH,
                    rules::IDENTIFIER_PATH_SEGMENT_EMOJI,
                ]));
                self.choice(&[rules::ALPHA_TOKEN, rules::IDENTIFIER_PATH_SEGMENT_EMOJI]);
            }
            rules::LEFT_ANGLE => self.choice(&[rules::LEFT_ANGLE1, rules::LEFT_ANGLE2]),
            rules::RIGHT_ANGLE => self.choice(&[rules::RIGHT_ANGLE1, rules::RIGHT_ANGLE2]),
            rules::BOX_DRAWING_CHAR => self.choice(BOX_DRAWING_CHAR_RULES),
            rules::BOX_DRAWING_EMOJI => self.choice(BOX_DRAWING_EMOJI_RULES),
            rules::TAG => self.result = false,
            _ => unreachable!("supported base rule"),
        }
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty() {
            // Stop before unwinding markers can publish a resource remainder
            // against temporary EOF. The stream owner seals accepted input and
            // drains these same phases to construct its terminal limited view.
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            let frame = self.frames.pop().expect("pending base phase");
            if *allowance == 0 {
                self.push(frame);
                return Progress::NeedsProcessing;
            }
            if !matches!(frame, Frame::Literal(..) | Frame::Grapheme(..)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Call(rule) => self.call(parser, rule),
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if !self.result {
                        parser.rewind(checkpoint);
                    }
                }
                Frame::Sequence(rules, index) => {
                    if self.result
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Sequence(rules, index + 1));
                        self.push(Frame::Call(*rule));
                    }
                }
                Frame::Choice(rules, index) => {
                    if !self.result
                        && !parser.is_halted()
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Choice(rules, index + 1));
                        self.push(Frame::Call(*rule));
                    }
                }
                Frame::FirstRepeat(rules) => {
                    if self.result {
                        self.repeat(parser, rules);
                    }
                }
                Frame::Repeat(rules, before) => {
                    if self.result && parser.offset() != before && !parser.is_halted() {
                        self.repeat(parser, rules);
                    } else {
                        self.result = true;
                    }
                }
                Frame::CompleteNode(marker, kind) => {
                    if self.result {
                        marker.complete(parser, kind);
                    }
                }
                Frame::Unless(checkpoint, rule) => {
                    parser.rewind(checkpoint);
                    if self.result {
                        self.result = false;
                    } else {
                        self.push(Frame::Call(rule));
                    }
                }
                Frame::Fixed(spec) => {
                    if spec.spacing == TerminalSpacing::Whitespace0Both {
                        self.push(Frame::FixedAfterLeading(spec));
                        self.push(Frame::Call(rules::WHITESPACE0));
                    } else {
                        self.literal(parser, spec.literal, Some(spec.kind), final_input);
                    }
                }
                Frame::FixedAfterLeading(spec) => {
                    if self.result {
                        self.push(Frame::FixedAfterToken);
                        self.literal(parser, spec.literal, Some(spec.kind), final_input);
                    }
                }
                Frame::FixedAfterToken => {
                    if self.result {
                        self.push(Frame::Call(rules::WHITESPACE0));
                    }
                }
                Frame::Literal(mut scan, kind) => {
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
                        LiteralProgress::Complete(end) => {
                            self.push(Frame::LiteralResult(end, kind))
                        }
                        LiteralProgress::NeedsProcessing | LiteralProgress::NeedInput => {
                            self.push(Frame::Literal(scan, kind));
                            return if progress == LiteralProgress::NeedInput {
                                Progress::NeedInput
                            } else {
                                Progress::NeedsProcessing
                            };
                        }
                        LiteralProgress::InvalidSource => {
                            panic!("base continuation source bounds changed")
                        }
                    }
                }
                Frame::LiteralResult(end, kind) => {
                    self.result = match (end, kind) {
                        (Some(end), Some(kind)) => parser
                            .bump_bytes_token((end - parser.offset()).0, kind)
                            .is_some(),
                        (Some(_), None) => true,
                        (None, _) => false,
                    };
                }
                Frame::Classified(kind, classify) => {
                    if let Some(first) = parser.cursor().peek_char() {
                        if classify(first) {
                            self.push(Frame::Raw(Some(kind)));
                        } else {
                            self.result = false;
                        }
                    } else if !final_input && parser.offset() == parser.cursor().end() {
                        self.push(Frame::Classified(kind, classify));
                        return Progress::NeedInput;
                    } else {
                        self.result = false;
                    }
                }
                Frame::Raw(kind) => {
                    if parser.charge() {
                        self.push(Frame::Grapheme(
                            GraphemeScan::new(
                                parser.offset(),
                                final_input.then_some(parser.cursor().context_end()),
                            ),
                            kind,
                        ));
                    } else {
                        self.raw_range = None;
                        self.result = false;
                    }
                }
                Frame::Grapheme(mut scan, kind) => {
                    let before = *allowance;
                    let progress = scan.advance(
                        parser.source(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        ScanProgress::Grapheme(range) => self.push(Frame::GraphemeResult(
                            (range.end <= parser.cursor().end()).then_some(range),
                            kind,
                        )),
                        ScanProgress::End => self.push(Frame::GraphemeResult(None, kind)),
                        ScanProgress::NeedInput | ScanProgress::NeedsProcessing => {
                            self.push(Frame::Grapheme(scan, kind));
                            return if progress == ScanProgress::NeedInput {
                                Progress::NeedInput
                            } else {
                                Progress::NeedsProcessing
                            };
                        }
                        ScanProgress::InvalidSource => {
                            panic!("base continuation source bounds changed")
                        }
                    }
                }
                Frame::GraphemeResult(range, kind) => {
                    self.raw_range =
                        range.and_then(|range| parser.cursor.bump_bytes(range.len().0));
                    self.result = self.raw_range.is_some();
                    if let (Some(range), Some(kind)) = (self.raw_range, kind) {
                        parser.token(kind, range);
                    }
                }
                Frame::EscapeAfterSlash => {
                    if self.result {
                        self.push(Frame::EscapeZero);
                        self.literal(parser, "0", None, final_input);
                    }
                }
                Frame::EscapeZero => {
                    if self.result {
                        self.push(Frame::LiteralResult(
                            Some(parser.offset() + TextSize(1)),
                            Some(SyntaxKind::EscapedChar),
                        ));
                    } else {
                        self.push(Frame::EscapeUnicode);
                        self.literal(parser, "u{", None, final_input);
                    }
                }
                Frame::EscapeUnicode => {
                    if self.result {
                        self.push(Frame::UnicodePrefix(
                            UnicodeEscape {
                                checkpoint: parser.checkpoint(),
                                start: parser.offset(),
                                scalar: Some(0),
                                digits: 0,
                            },
                            0,
                        ));
                    } else {
                        self.push(Frame::EscapeFallback);
                    }
                }
                Frame::UnicodePrefix(state, count) => {
                    if count > 0 && !self.result {
                        continue;
                    }
                    if count == 2 {
                        self.push(Frame::UnicodeDigit(state));
                    } else {
                        self.push(Frame::UnicodePrefix(state, count + 1));
                        self.push(Frame::Raw(None));
                    }
                }
                Frame::UnicodeDigit(state) => match parser.cursor().peek_char() {
                    Some(ch) if ch.is_ascii_hexdigit() => {
                        self.push(Frame::UnicodeDigitResult(state));
                        self.push(Frame::Raw(None));
                    }
                    None if !final_input && parser.offset() == parser.cursor().end() => {
                        self.push(Frame::UnicodeDigit(state));
                        return Progress::NeedInput;
                    }
                    _ if state.digits > 0 && state.scalar.and_then(char::from_u32).is_some() => {
                        self.push(Frame::UnicodeCloser(state));
                        self.literal(parser, "}", None, final_input);
                    }
                    _ => {
                        parser.rewind(state.checkpoint);
                        self.push(Frame::EscapeFallback);
                    }
                },
                Frame::UnicodeDigitResult(mut state) => {
                    if !self.result {
                        continue;
                    }
                    let range = self.raw_range.expect("matched raw digit");
                    state.digits += 1;
                    state.scalar = if range.len() == TextSize(1) {
                        state
                            .scalar
                            .and_then(|value| value.checked_mul(16))
                            .and_then(|value| {
                                value.checked_add(
                                    (parser.source().byte_at(range.start).unwrap() as char)
                                        .to_digit(16)
                                        .unwrap(),
                                )
                            })
                    } else {
                        None
                    };
                    self.push(Frame::UnicodeDigit(state));
                }
                Frame::UnicodeCloser(state) => {
                    if self.result {
                        self.push(Frame::UnicodeComplete(state));
                        self.push(Frame::Raw(None));
                    } else {
                        parser.rewind(state.checkpoint);
                        self.push(Frame::EscapeFallback);
                    }
                }
                Frame::UnicodeComplete(state) => {
                    if self.result {
                        parser.token(
                            SyntaxKind::EscapedChar,
                            TextRange::new(state.start, parser.offset()),
                        );
                    }
                }
                Frame::EscapeFallback => match parser.cursor().peek_char() {
                    Some(ch) if ch.is_alphabetic() => {
                        self.push(Frame::Raw(Some(SyntaxKind::EscapedChar)))
                    }
                    Some(_) => self.push(Frame::EscapeSymbol(0)),
                    None if !final_input && parser.offset() == parser.cursor().end() => {
                        self.push(Frame::EscapeFallback);
                        return Progress::NeedInput;
                    }
                    None => self.result = false,
                },
                Frame::EscapeSymbol(index) => {
                    if let Some(rule) = SYMBOL_RULES.iter().chain(PUNCTUATION_RULES).nth(index) {
                        let spec = fixed_terminal_spec(*rule).expect("escape punctuation terminal");
                        self.push(Frame::EscapeSymbolResult(index));
                        self.literal(parser, spec.literal, None, final_input);
                    } else {
                        self.result = false;
                    }
                }
                Frame::EscapeSymbolResult(index) => {
                    if self.result {
                        self.push(Frame::Raw(Some(SyntaxKind::EscapedChar)));
                    } else {
                        self.push(Frame::EscapeSymbol(index + 1));
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parser::LexicalMode;
    use crate::document::{DocumentId, IdGenerator, ParseConfig, Revision, TextSnapshot};
    use alloc::string::String;

    fn run(
        rule: RuleId,
        text: &str,
        chunks: &[&str],
        allowance_size: u64,
        fuel: u64,
    ) -> (bool, TextSize, String, crate::document::ParseStats, u64) {
        let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut config = ParseConfig::default();
        config.limits.fuel = fuel;
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            config,
            &mut ids,
        );
        let wrapper = parser.start();
        let mut state = parser.suspend();
        let mut continuation = Continuation::new(rule);
        'input: for chunk in chunks {
            source = source.append(*chunk).unwrap();
            let mut parser = Parser::resume(&source, state, &mut ids);
            loop {
                let before = continuation.work;
                let mut allowance = allowance_size;
                let progress = continuation.advance(&mut parser, false, &mut allowance);
                assert!(continuation.work - before <= allowance_size);
                let suspended = parser.suspend();
                parser = Parser::resume(&source, suspended, &mut ids);
                if matches!(progress, Progress::Limited) {
                    state = parser.suspend();
                    break 'input;
                }
                if !matches!(progress, Progress::NeedsProcessing) {
                    break;
                }
                assert_eq!(allowance, 0);
                assert!(continuation.work < 10_000_000);
            }
            state = parser.suspend();
        }
        assert_eq!(chunks.concat(), text);
        assert!(source.byte_len().to_usize() <= text.len());
        let mut parser = Parser::resume(&source, state, &mut ids);
        let result = loop {
            let before = continuation.work;
            let mut allowance = allowance_size;
            let progress = continuation.advance(&mut parser, true, &mut allowance);
            assert!(continuation.work - before <= allowance_size);
            match progress {
                Progress::Complete(result) => break result,
                Progress::NeedInput | Progress::Limited => panic!("final input failed to drain"),
                Progress::NeedsProcessing => {
                    let suspended = parser.suspend();
                    parser = Parser::resume(&source, suspended, &mut ids);
                    assert!(continuation.work < 10_000_000);
                }
            }
        };
        assert_eq!(parser.state.rules.len(), 0);
        let consumed = parser.offset();
        wrapper.complete(&mut parser, SyntaxKind::Document);
        let output = parser.finish();
        (
            result,
            consumed,
            alloc::format!("{:?}", output.events),
            output.stats,
            continuation.work,
        )
    }

    #[test]
    fn base_rules_keep_exact_output_and_fuel_across_input_and_work_partitions() {
        let cases = [
            (rules::COLON, ":="),
            (rules::TRANSITION_OPERATOR, "~>x"),
            (rules::OUTPUT_OPERATOR, "->x"),
            (rules::WHITESPACE0, " \r\n\t x"),
            (rules::NEWLINE_INDENT, "\r\n\u{a0}\u{2009} x"),
            (rules::IDENTIFIER, "alphaβe\u{301}123+tail!"),
            (rules::IDENTIFIER_PATH_SEGMENT, "hello-world/x"),
            (rules::DIGIT_SEQUENCE, "12_34_56x"),
            (rules::DIGIT_SEQUENCE, "1_"),
            (rules::EMOJI, "👩\u{200d}💻!"),
            (rules::EMOJI, "╭◉╮"),
            (rules::EMOJI_GRAPHEME, "🇦🇧🇨"),
            (rules::ESCAPED_CHAR, "\\u{0001f600}x"),
            (rules::ESCAPED_CHAR, "\\u{110000}x"),
            (rules::ESCAPED_CHAR, "\\u{123456789abcdef}"),
            (rules::ESCAPED_CHAR, "\\u{1\u{301}}"),
            (rules::ESCAPED_CHAR, "\\u{}"),
            (rules::ESCAPED_CHAR, "\\u{"),
            (rules::ESCAPED_CHAR, "\\u{+1}"),
            (rules::ESCAPED_CHAR, "\\0x"),
            (rules::ESCAPED_CHAR, "\\e\u{301}x"),
            (rules::ESCAPED_CHAR, "\\!\u{301}x"),
            (rules::ESCAPED_CHAR, "\\!x"),
            (rules::ESCAPED_CHAR, "\\"),
            (rules::TEXT, "e\u{301}x"),
            (rules::RAW_TEXT, "\\u{1}"),
            (rules::LIST_SEPARATOR, " \r\n,\t x"),
            (rules::ENUM_SEPARATOR, " | x"),
        ];
        for (rule, text) in cases {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            for fuel in [0, 1, 2, 4, 8, 16, 64, u64::MAX] {
                let baseline = run(rule, text, &[text], u64::MAX, fuel);
                for split in &boundaries {
                    let observed = run(rule, text, &[&text[..*split], &text[*split..]], 1, fuel);
                    let accepted = &text[..observed.3.source_bytes as usize];
                    let baseline = if accepted.len() == text.len() {
                        baseline.clone()
                    } else {
                        run(rule, accepted, &[accepted], u64::MAX, fuel)
                    };
                    assert_eq!(
                        (&observed.0, &observed.1, &observed.2, &observed.3),
                        (&baseline.0, &baseline.1, &baseline.2, &baseline.3),
                        "rule {rule:?}, split {split}, fuel {fuel}, text {text:?}"
                    );
                }
                let scalar_chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let observed = run(rule, text, &scalar_chunks, 1, fuel);
                let accepted = &text[..observed.3.source_bytes as usize];
                let baseline = if accepted.len() == text.len() {
                    baseline.clone()
                } else {
                    run(rule, accepted, &[accepted], u64::MAX, fuel)
                };
                assert_eq!(
                    (&observed.0, &observed.1, &observed.2, &observed.3),
                    (&baseline.0, &baseline.1, &baseline.2, &baseline.3),
                    "scalar input rule {rule:?}, fuel {fuel}, text {text:?}"
                );
            }
        }
    }

    #[test]
    fn hard_limit_preserves_open_owners_until_the_accepted_prefix_is_sealed() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "alpha").unwrap();
        let mut ids = IdGenerator::new();
        let mut config = ParseConfig::default();
        config.limits.fuel = 1;
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            config,
            &mut ids,
        );
        let wrapper = parser.start();
        let mut continuation = Continuation::new(rules::IDENTIFIER);
        loop {
            match continuation.advance(&mut parser, false, &mut 1) {
                Progress::NeedsProcessing => {}
                Progress::Limited => break,
                _ => panic!("fuel exhaustion must be an explicit terminal outcome"),
            }
        }
        assert!(parser.is_halted());
        assert!(!parser.state.resource_finalizing);
        assert_eq!(parser.state.fuel, 0);
        assert!(parser.state.open_markers.len() >= 2);
        let events = parser.state.events.len();
        let work = continuation.work;
        assert!(matches!(
            continuation.advance(&mut parser, false, &mut 100),
            Progress::Limited
        ));
        assert_eq!(continuation.work, work);
        assert_eq!(parser.state.events.len(), events);
        let state = parser.suspend();
        let mut parser = Parser::resume(&source, state, &mut ids);
        loop {
            match continuation.advance(&mut parser, true, &mut 1) {
                Progress::NeedsProcessing => {}
                Progress::Complete(_) => break,
                _ => panic!("sealed limited source must drain"),
            }
        }
        wrapper.complete(&mut parser, SyntaxKind::Document);
        let output = parser.finish();
        assert_eq!(output.stats.parser_steps, 1);
        assert_eq!(output.stats.source_bytes, 5);
        assert_eq!(output.diagnostics.len(), 1);
        assert!(alloc::format!("{:?}", output.events).contains("end: TextSize(5)"));
    }

    #[test]
    fn unfinished_identifiers_whitespace_and_unicode_escapes_keep_linear_work() {
        for rule in [rules::IDENTIFIER, rules::WHITESPACE0, rules::ESCAPED_CHAR] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let (text, expected_end) = match rule {
                    rules::IDENTIFIER => {
                        let text = "a".repeat(n);
                        (text, n)
                    }
                    rules::WHITESPACE0 => {
                        let text = " ".repeat(n);
                        (text, n)
                    }
                    _ => {
                        let text = String::from("\\u{") + &"0".repeat(n) + "61}";
                        let end = text.len();
                        (text, end)
                    }
                };
                let chunks: Vec<_> = text
                    .as_bytes()
                    .chunks(1)
                    .map(|byte| core::str::from_utf8(byte).unwrap())
                    .collect();
                let observed = run(rule, &text, &chunks, 1, u64::MAX);
                assert!(observed.0);
                assert_eq!(observed.1.to_usize(), expected_end);
                if let Some(previous) = previous {
                    assert!(
                        observed.4 <= previous * 3,
                        "rule {rule:?} restarted on append"
                    );
                }
                previous = Some(observed.4);
            }
        }
    }
}
