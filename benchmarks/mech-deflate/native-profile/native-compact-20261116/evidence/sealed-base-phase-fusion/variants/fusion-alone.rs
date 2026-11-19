//! Canonical base rules own their token, alternative, and repetition phases.
use super::*;
use crate::document::parser::grapheme_scan::{GraphemeScan, ScanProgress};
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::{checkpoint::ParserCheckpoint, marker::Marker};
use crate::document::{TextRange, TextSize};
use alloc::{boxed::Box, vec::Vec};

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
    Literal(Option<SyntaxKind>),
    LiteralResult(Option<TextSize>, Option<SyntaxKind>),
    Classified(SyntaxKind, fn(char) -> bool),
    Raw(Option<SyntaxKind>),
    Grapheme(Option<SyntaxKind>),
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

#[derive(Default)]
pub(crate) struct FrameStorage {
    frames: Vec<Frame>,
    scanner: Option<Box<Scanner>>,
    pub(super) prediction: Option<Box<super::recognition::Storage>>,
}
enum Scanner {
    Literal(LiteralScan<'static>),
    Grapheme(GraphemeScan),
}

pub(crate) struct Continuation {
    initial: Option<RuleId>,
    storage: FrameStorage,
    active: bool,
    result: bool,
    raw_range: Option<TextRange>,
    pub work: u64,
    #[cfg(test)]
    disable_sealed_fusion: bool,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        Self {
            initial: Some(rule),
            storage: FrameStorage::default(),
            active: false,
            result: false,
            raw_range: None,
            work: 0,
            #[cfg(test)]
            disable_sealed_fusion: false,
        }
    }
    fn scanner(&mut self, scanner: Scanner) {
        if let Some(slot) = &mut self.storage.scanner {
            **slot = scanner;
        } else {
            self.storage.scanner = Some(Box::new(scanner));
        }
    }
    fn push(&mut self, frame: Frame) {
        self.storage.frames.push(frame);
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
        self.scanner(Scanner::Literal(
            LiteralScan::new(
                literal,
                parser.offset(),
                final_input.then_some(parser.cursor().context_end()),
            )
            .expect("nonempty canonical literal"),
        ));
        self.push(Frame::Literal(kind));
    }
    fn unless(&mut self, parser: &Parser<'_>, excluded: &'static [RuleId], rule: RuleId) {
        self.push(Frame::Unless(parser.checkpoint(), rule));
        self.choice(excluded);
    }
    fn call(
        &mut self,
        parser: &mut Parser<'_>,
        rule: RuleId,
        final_input: bool,
        allowance: &mut u64,
    ) -> Option<Progress> {
        let fixed = fixed_terminal_spec(rule);
        if fixed.is_none() && !non_fixed_rule(rule) {
            self.result = false;
            return None;
        }
        if final_input
            && let Some(spec) = fixed
            && spec.spacing == TerminalSpacing::Exact
            && let Some((end, work)) = {
                let start = parser.offset();
                let consume = parser.cursor().end();
                let context = parser.cursor().context_end();
                parser
                    .state
                    .literal_cache
                    .replay(spec.literal, start, consume, context, *allowance)
                    .or_else(|| {
                        if parser.is_halted() || parser.state.resource_finalizing {
                            return None;
                        }
                        parser.state.literal_cache.sealed_ascii_miss(
                            parser.source,
                            spec.literal,
                            start,
                            consume,
                            context,
                            *allowance,
                        )
                    })
            }
        {
            *allowance -= work + 3;
            self.work += work + 3;
            if let Some(end) = end {
                let checkpoint = parser.checkpoint();
                parser.state.rules.push_canonical(rule);
                self.result = parser
                    .bump_bytes_token((end - parser.offset()).0, spec.kind)
                    .is_some();
                parser.state.rules.truncate(checkpoint.rule_depth);
                if !self.result {
                    parser.rewind(checkpoint);
                }
            } else {
                // Nothing consumed input, created markers, or emitted a
                // diagnostic. Record the transient rule depth and the empty
                // rewind without saving and restoring the parser state.
                let depth = parser.state.rules.len() + 1;
                parser.state.rules.note_peak(depth);
                if !parser.state.resource_finalizing {
                    parser.state.rewinds += 1;
                }
                self.result = false;
            }
            return None;
        }
        let checkpoint = parser.checkpoint();
        parser.state.rules.push_canonical(rule);
        self.push(Frame::Exit(checkpoint));
        if let Some(spec) = fixed {
            self.push(Frame::Fixed(spec));
            return None;
        }
        match specification(rule).expect("supported non-fixed base rule") {
            Spec::Alias(rule) => self.push(Frame::Call(rule)),
            Spec::Choice(rules) => self.choice(rules),
            Spec::Sequence(rules) => self.sequence(rules),
            Spec::Repeat { required, choices } => {
                if required {
                    self.one_or_more(choices)
                } else {
                    self.repeat(parser, choices)
                }
            }
            Spec::RepeatedNode {
                kind,
                first,
                rest,
                single_first,
            } => {
                if final_input && !parser.is_halted() {
                    let state = self
                        .storage
                        .prediction
                        .get_or_insert_with(|| Box::new(super::recognition::Storage::default()));
                    let cache = &mut state.memo;
                    if cache.capture.is_none() && cache.kind(rule, kind, first, rest).is_some() {
                        if let Some((work, raw)) =
                            cache.replay(parser, rule, checkpoint.rule_depth, *allowance + 1)
                        {
                            let exit = self
                                .storage
                                .frames
                                .pop()
                                .expect("pending repeated-node exit");
                            debug_assert!(matches!(exit, Frame::Exit(_)));
                            parser.state.rules.truncate(checkpoint.rule_depth);
                            *allowance -= work - 1;
                            self.work += work - 1;
                            self.raw_range = Some(raw);
                            self.result = true;
                            return None;
                        }
                        cache.capture = Some(super::recognition::Capture::new(
                            parser,
                            rule,
                            kind,
                            self.storage.frames.len() - 1,
                            checkpoint,
                            self.work - 1,
                        ));
                    }
                }
                self.node(parser, kind);
                self.push(Frame::FirstRepeat(rest));
                if single_first {
                    self.push(Frame::Call(first[0]));
                } else {
                    self.choice(first);
                }
            }
            Spec::Classified(kind, classify) => {
                if self.charge_sealed_followup(final_input, allowance) {
                    return self.classified(parser, kind, classify, final_input, allowance);
                }
                self.push(Frame::Classified(kind, classify));
            }
            Spec::Raw(kind) => self.push(Frame::Raw(kind)),
            Spec::Unless { excluded, rule } => self.unless(parser, excluded, rule),
            Spec::NewlineIndent => {
                self.push(Frame::FirstRepeat(&[rules::SPACE_TAB]));
                self.push(Frame::Call(rules::NEW_LINE));
            }
            Spec::Escape => {
                self.node(parser, SyntaxKind::EscapedCharacter);
                self.push(Frame::EscapeAfterSlash);
                self.push(Frame::Call(rules::BACKSLASH));
            }
            Spec::Never => self.result = false,
        }
        None
    }

    /// Execute the next deterministic phase without a stack round trip only
    /// when this sealed poll can pay that phase's original logical work.
    #[inline]
    fn charge_sealed_followup(&mut self, final_input: bool, allowance: &mut u64) -> bool {
        #[cfg(test)]
        if self.disable_sealed_fusion {
            return false;
        }
        if !final_input || *allowance == 0 {
            return false;
        }
        *allowance -= 1;
        self.work += 1;
        true
    }

    fn classified(
        &mut self,
        parser: &mut Parser<'_>,
        kind: SyntaxKind,
        classify: fn(char) -> bool,
        final_input: bool,
        allowance: &mut u64,
    ) -> Option<Progress> {
        if let Some(first) = parser.cursor().peek_char() {
            if classify(first) {
                if self.charge_sealed_followup(final_input, allowance) {
                    return self.raw(parser, Some(kind), final_input, allowance);
                }
                self.push(Frame::Raw(Some(kind)));
            } else {
                self.result = false;
            }
        } else if !final_input && parser.offset() == parser.cursor().end() {
            self.push(Frame::Classified(kind, classify));
            return Some(Progress::NeedInput);
        } else {
            self.result = false;
        }
        None
    }

    fn raw(
        &mut self,
        parser: &mut Parser<'_>,
        kind: Option<SyntaxKind>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Option<Progress> {
        if let Some(capture) = self
            .storage
            .prediction
            .as_mut()
            .and_then(|state| state.memo.capture.as_mut())
        {
            capture.raw(parser);
        }
        if parser.charge() {
            let end = final_input.then_some(parser.cursor().context_end());
            parser.observe_definition_raw_origin(end);
            let scan = parser
                .state
                .grapheme_tail
                .take()
                .filter(|scan| scan.resumes_at(parser.offset(), end))
                .unwrap_or_else(|| GraphemeScan::new(parser.offset(), end));
            if final_input && *allowance != 0 {
                if let Some(pending) = self.finish_sealed_grapheme(parser, kind, allowance, scan) {
                    return Some(pending);
                }
            } else {
                self.scanner(Scanner::Grapheme(scan));
                self.push(Frame::Grapheme(kind));
            }
        } else {
            self.raw_range = None;
            self.result = false;
        }
        None
    }

    fn accept_grapheme(
        &mut self,
        parser: &mut Parser<'_>,
        range: Option<TextRange>,
        kind: Option<SyntaxKind>,
    ) {
        self.raw_range = range.and_then(|range| parser.cursor.bump_bytes(range.len().0));
        self.result = self.raw_range.is_some();
        if let (Some(range), Some(kind)) = (self.raw_range, kind) {
            parser.token(kind, range);
        }
    }

    fn finish_sealed_grapheme(
        &mut self,
        parser: &mut Parser<'_>,
        kind: Option<SyntaxKind>,
        allowance: &mut u64,
        mut scan: GraphemeScan,
    ) -> Option<Progress> {
        let before = *allowance;
        let progress = scan.advance(
            parser.source(),
            parser.cursor().context_end(),
            true,
            allowance,
        );
        self.work += before - *allowance;
        let range = match progress {
            ScanProgress::Grapheme(range) => {
                parser.state.grapheme_tail = Some(scan);
                (range.end <= parser.cursor().end()).then_some(range)
            }
            ScanProgress::End => None,
            ScanProgress::NeedInput | ScanProgress::NeedsProcessing => {
                self.scanner(Scanner::Grapheme(scan));
                self.push(Frame::Grapheme(kind));
                return Some(if progress == ScanProgress::NeedInput {
                    Progress::NeedInput
                } else {
                    Progress::NeedsProcessing
                });
            }
            ScanProgress::InvalidSource => panic!("base continuation source bounds changed"),
        };
        // A completed scan can yield before GraphemeResult. Leave that
        // frame pending at the exact same allowance boundary.
        if *allowance == 0 {
            self.push(Frame::GraphemeResult(range, kind));
            return Some(Progress::NeedsProcessing);
        }
        *allowance -= 1;
        self.work += 1;
        self.accept_grapheme(parser, range, kind);
        None
    }

    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        if let Some(rule) = self.initial.take() {
            self.storage = core::mem::take(&mut parser.state.base_storage);
            self.active = true;
            debug_assert!(self.storage.frames.is_empty());
            self.push(Frame::Call(rule));
        }
        while !self.storage.frames.is_empty() {
            // Stop before unwinding markers can publish a resource remainder
            // against temporary EOF. The stream owner seals accepted input and
            // drains these same phases to construct its terminal limited view.
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            let frame = self.storage.frames.pop().expect("pending base phase");
            if *allowance == 0 {
                self.push(frame);
                return Progress::NeedsProcessing;
            }
            // Predict before charging this call frame: summaries include its
            // original work, including every failed nested alternative. The
            // cache belongs to this active continuation until storage returns.
            if let Frame::Call(rule) = &frame
                && final_input
                && let Some(index) = non_fixed_index(*rule)
                && let Some((result, work)) = super::super::prediction::predict_base(
                    parser,
                    *rule,
                    index,
                    *allowance,
                    &mut self.storage.prediction,
                )
            {
                *allowance -= work;
                self.work += work;
                self.result = result;
                continue;
            }
            if !matches!(frame, Frame::Literal(..) | Frame::Grapheme(..)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Call(rule) => {
                    if let Some(progress) = self.call(parser, rule, final_input, allowance) {
                        return progress;
                    }
                }
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if !self.result {
                        parser.rewind(checkpoint);
                    }
                    if self
                        .storage
                        .prediction
                        .as_ref()
                        .and_then(|state| state.memo.capture.as_ref())
                        .is_some_and(|capture| capture.depth == self.storage.frames.len())
                    {
                        let cache = &mut self
                            .storage
                            .prediction
                            .as_mut()
                            .expect("lexical cache owner")
                            .memo;
                        let capture = cache.capture.take().expect("completed lexical capture");
                        capture.complete(cache, parser, self.result, self.work, self.raw_range);
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
                Frame::Literal(kind) => {
                    let Some(Scanner::Literal(scan)) = self.storage.scanner.as_deref_mut() else {
                        unreachable!("pending literal scanner")
                    };
                    let before = *allowance;
                    let progress = scan.advance_cached(
                        parser.source,
                        parser.cursor().end(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                        &mut parser.state.literal_cache,
                    );
                    self.work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(end) => {
                            self.push(Frame::LiteralResult(end, kind))
                        }
                        LiteralProgress::NeedsProcessing | LiteralProgress::NeedInput => {
                            self.push(Frame::Literal(kind));
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
                    if let Some(progress) =
                        self.classified(parser, kind, classify, final_input, allowance)
                    {
                        return progress;
                    }
                }
                Frame::Raw(kind) => {
                    if let Some(progress) = self.raw(parser, kind, final_input, allowance) {
                        return progress;
                    }
                }
                Frame::Grapheme(kind) => {
                    let Some(Scanner::Grapheme(scan)) = self.storage.scanner.as_deref_mut() else {
                        unreachable!("pending grapheme scanner")
                    };
                    let before = *allowance;
                    let progress = scan.advance(
                        parser.source(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        ScanProgress::Grapheme(range) => {
                            parser.state.grapheme_tail = Some(scan.clone());
                            self.push(Frame::GraphemeResult(
                                (range.end <= parser.cursor().end()).then_some(range),
                                kind,
                            ));
                        }
                        ScanProgress::End => self.push(Frame::GraphemeResult(None, kind)),
                        ScanProgress::NeedInput | ScanProgress::NeedsProcessing => {
                            self.push(Frame::Grapheme(kind));
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
                    self.accept_grapheme(parser, range, kind);
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
        if self.active {
            parser.state.base_storage = core::mem::take(&mut self.storage);
            self.active = false;
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
        run_with_prediction(
            rule,
            text,
            chunks,
            allowance_size,
            fuel,
            false,
            allowance_size,
        )
    }
    fn run_with_prediction(
        rule: RuleId,
        text: &str,
        chunks: &[&str],
        allowance_size: u64,
        fuel: u64,
        reference: bool,
        final_allowance: u64,
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
        parser.state.disable_rejections_for_test = reference;
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
            let mut allowance = final_allowance;
            let progress = continuation.advance(&mut parser, true, &mut allowance);
            assert!(continuation.work - before <= final_allowance);
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

    #[test]
    fn nested_predictions_survive_input_sealing_and_suspended_storage() {
        for (rule, text) in [
            (rules::IDENTIFIER, "alphaβe\u{301}123+tail!"),
            (rules::IDENTIFIER, "alphabet123__more+tail!"),
            (rules::TEXT, "letters 12_34,!? é👩\u{200d}💻"),
            (rules::DIGIT_SEQUENCE, "12_34_56x"),
            (rules::WHITESPACE0, " \r\n\t x"),
            (rules::ESCAPED_CHAR, "\\u{0001f600}x"),
        ] {
            for split in text.char_indices().map(|(at, _)| at).chain([text.len()]) {
                let chunks = [&text[..split], &text[split..]];
                for fuel in [0, 1, 4, 16, u64::MAX] {
                    assert_eq!(
                        run_with_prediction(rule, text, &chunks, 1, fuel, false, 8192),
                        run_with_prediction(rule, text, &chunks, 1, fuel, true, 8192),
                        "rule {rule:?}, text {text:?}, split {split}, fuel {fuel}"
                    );
                }
            }
        }
    }

    #[test]
    fn prediction_cache_returns_from_nested_calls_to_the_same_parser() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "alpha123 ").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let _wrapper = parser.start();
        let mut continuation = Continuation::new(rules::IDENTIFIER);
        let mut allowance = u64::MAX;
        assert!(matches!(
            continuation.advance(&mut parser, true, &mut allowance),
            Progress::Complete(true)
        ));
        assert!(!continuation.active);
        assert!(continuation.storage.prediction.is_none());
        let cache = parser.state.base_storage.prediction.as_ref().unwrap();
        for rule in [rules::IDENTIFIER, rules::ALPHA, rules::DIGIT] {
            assert!(
                cache[non_fixed_index(rule).unwrap()].is_some(),
                "uncached nested rule {rule:?}"
            );
        }
    }
}

#[cfg(test)]
mod fused_tests;
