//! Required-production recovery shares balanced abandonment and missing-node
//! publication, retaining the owner's operator restart probe across pauses.
use super::super::structure_shell;
use super::precedence::{Continuation, Progress};
use super::*;
use crate::document::parser::ParserCheckpoint;
use crate::document::parser::recovery::{
    AbandonContinuation, AbandonProgress, BoundaryProgress, MissingContinuation, MissingProgress,
};
use crate::document::{ExpectedSyntax, TextRange, TextSize, TokenFlags};
use alloc::{boxed::Box, string::String, vec::Vec};

const RESTART_BOUNDARIES: &[char] = &[
    ')', ']', '}', '>', '⟩', '╯', '┘', '┛', ',', ';', '|', '│', '┃', '?', '\n', '\r',
];
enum ProbePhase {
    Start,
    Prefix(usize, u32),
    Operator,
    OperatorChild(ParserCheckpoint, Box<Continuation>),
    Trivia(ParserCheckpoint, base::continuation::Continuation),
}
struct Probe<'a> {
    boundaries: Vec<char>,
    prefixes: &'a [&'a str],
    operator: Option<usize>,
    previous: Option<(TextSize, bool)>,
    rejected_trivia_end: TextSize,
    phase: ProbePhase,
}
impl<'a> Probe<'a> {
    fn new(boundaries: &[char], prefixes: &'a [&'a str], operator: Option<usize>) -> Self {
        let mut all = Vec::from(RESTART_BOUNDARIES);
        all.extend_from_slice(boundaries);
        Self {
            boundaries: all,
            prefixes,
            operator,
            previous: None,
            rejected_trivia_end: TextSize::ZERO,
            phase: ProbePhase::Start,
        }
    }
    fn complete(&mut self, value: bool) -> BoundaryProgress {
        self.phase = ProbePhase::Start;
        BoundaryProgress::Complete(value)
    }
    fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        character: char,
        final_input: bool,
        allowance: &mut u64,
    ) -> BoundaryProgress {
        loop {
            if *allowance == 0 {
                return BoundaryProgress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, ProbePhase::Start);
            if !matches!(
                phase,
                ProbePhase::OperatorChild(..) | ProbePhase::Trivia(..)
            ) {
                *allowance -= 1;
            }
            match phase {
                ProbePhase::Start => {
                    if self.boundaries.contains(&character) {
                        return self.complete(true);
                    }
                    self.phase = ProbePhase::Prefix(0, 0);
                }
                ProbePhase::Prefix(index, byte) => {
                    if let Some(prefix) = self.prefixes.get(index) {
                        if let Some(expected) = prefix.as_bytes().get(byte as usize) {
                            match parser.cursor().byte_at(byte) {
                                Some(actual) if actual == *expected => {
                                    self.phase = ProbePhase::Prefix(index, byte + 1)
                                }
                                None if !final_input => {
                                    self.phase = ProbePhase::Prefix(index, byte);
                                    return BoundaryProgress::NeedInput;
                                }
                                _ => self.phase = ProbePhase::Prefix(index + 1, 0),
                            }
                        } else {
                            return self.complete(true);
                        }
                    } else {
                        self.phase = ProbePhase::Operator;
                    }
                }
                ProbePhase::Operator => {
                    let Some(index) = self.operator else {
                        return self.complete(false);
                    };
                    let offset = parser.offset();
                    if let Some((previous, result)) = self.previous {
                        if previous == offset {
                            return self.complete(result);
                        }
                    }
                    if offset < self.rejected_trivia_end {
                        return self.complete(false);
                    }
                    self.phase = ProbePhase::OperatorChild(
                        parser.checkpoint(),
                        Box::new(Continuation::operator(index)),
                    );
                }
                ProbePhase::OperatorChild(checkpoint, mut child) => {
                    match child.advance(parser, final_input, allowance) {
                        Progress::Complete(result) => {
                            let at_operator = result.accepted();
                            parser.rewind(checkpoint);
                            if !at_operator && !parser.is_halted() {
                                self.phase = ProbePhase::Trivia(
                                    checkpoint,
                                    base::continuation::Continuation::new(rules::SPACE_TAB0),
                                );
                            } else {
                                self.previous = Some((checkpoint.cursor.offset, at_operator));
                                return self.complete(at_operator);
                            }
                        }
                        Progress::NeedInput => {
                            self.phase = ProbePhase::OperatorChild(checkpoint, child);
                            return BoundaryProgress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.phase = ProbePhase::OperatorChild(checkpoint, child);
                            return BoundaryProgress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.phase = ProbePhase::OperatorChild(checkpoint, child);
                            return BoundaryProgress::Limited;
                        }
                    }
                }
                ProbePhase::Trivia(checkpoint, mut child) => {
                    match child.advance(parser, final_input, allowance) {
                        base::continuation::Progress::Complete(_) => {
                            self.rejected_trivia_end = parser.offset();
                            parser.rewind(checkpoint);
                            self.previous = Some((checkpoint.cursor.offset, false));
                            return self.complete(false);
                        }
                        base::continuation::Progress::NeedInput => {
                            self.phase = ProbePhase::Trivia(checkpoint, child);
                            return BoundaryProgress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.phase = ProbePhase::Trivia(checkpoint, child);
                            return BoundaryProgress::NeedsProcessing;
                        }
                        base::continuation::Progress::Limited => {
                            self.phase = ProbePhase::Trivia(checkpoint, child);
                            return BoundaryProgress::Limited;
                        }
                    }
                }
            }
        }
    }
}
enum Phase<'a> {
    Horizontal,
    HorizontalRun(TextSize),
    Select,
    Probe(char),
    Missing(MissingContinuation<'a>),
    Abandon(AbandonContinuation<'a>),
    Done,
}
pub(super) struct Required<'a> {
    target: RuleId,
    code: &'a str,
    message: &'a str,
    production: &'a str,
    token: Option<(SyntaxKind, &'a str)>,
    probe: Probe<'a>,
    phase: Phase<'a>,
    pub work: u64,
}
impl<'a> Required<'a> {
    pub fn new(
        target: RuleId,
        code: &'a str,
        message: &'a str,
        production: &'a str,
        boundaries: &[char],
        prefixes: &'a [&'a str],
        operator: Option<usize>,
    ) -> Self {
        Self {
            target,
            code,
            message,
            production,
            token: None,
            probe: Probe::new(boundaries, prefixes, operator),
            phase: Phase::Horizontal,
            work: 0,
        }
    }
    pub fn token(
        target: RuleId,
        code: &'a str,
        message: &'a str,
        token: SyntaxKind,
        text: &'a str,
        prefixes: &'a [&'a str],
    ) -> Self {
        let mut owner = Self::new(target, code, message, "", &[], prefixes, None);
        owner.token = Some((token, text));
        owner.probe.boundaries.retain(|ch| *ch != '?');
        owner
    }
    fn missing(&mut self) {
        let (expected, token) = match self.token {
            Some((token, _)) => (ExpectedSyntax::Token(token), Some(token)),
            None => (
                ExpectedSyntax::Production(String::from(self.production)),
                None,
            ),
        };
        self.phase = Phase::Missing(MissingContinuation::new(
            self.code,
            self.message,
            expected,
            token,
        ));
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        loop {
            if matches!(self.phase, Phase::Done) {
                return Progress::Complete(Attempt::Committed);
            }
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Done);
            if !matches!(
                phase,
                Phase::Probe(_) | Phase::Missing(_) | Phase::Abandon(_)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match phase {
                Phase::Horizontal => {
                    if !parser.is_halted() && matches!(parser.cursor().byte(), Some(b' ' | b'\t')) {
                        self.phase = Phase::HorizontalRun(parser.offset());
                    } else if !final_input && parser.is_eof() && !parser.is_halted() {
                        self.phase = Phase::Horizontal;
                        return Progress::NeedInput;
                    } else {
                        self.phase = Phase::Select;
                    }
                }
                Phase::HorizontalRun(start) => {
                    if matches!(parser.cursor().byte(), Some(b' ' | b'\t'))
                        && parser.bump_char_raw().is_some()
                    {
                        self.phase = Phase::HorizontalRun(start);
                    } else if !final_input && parser.is_eof() && !parser.is_halted() {
                        self.phase = Phase::HorizontalRun(start);
                        return Progress::NeedInput;
                    } else {
                        if parser.offset() > start {
                            parser.token_with_flags(
                                SyntaxKind::Whitespace,
                                TextRange::new(start, parser.offset()),
                                TokenFlags::TRIVIA,
                            );
                        }
                        self.phase = Phase::Horizontal;
                    }
                }
                Phase::Select => {
                    if parser.is_eof() {
                        if final_input {
                            self.missing();
                        } else {
                            self.phase = Phase::Select;
                            return Progress::NeedInput;
                        }
                    } else {
                        self.phase =
                            Phase::Probe(parser.cursor().peek_char().expect("recovery source"));
                    }
                }
                Phase::Probe(character) => {
                    let before = *allowance;
                    let result = if self.token.is_some() {
                        *allowance -= 1;
                        BoundaryProgress::Complete(self.probe.boundaries.contains(&character))
                    } else {
                        self.probe
                            .advance(parser, character, final_input, allowance)
                    };
                    self.work += before - *allowance;
                    match result {
                        BoundaryProgress::Complete(true) => self.missing(),
                        BoundaryProgress::Complete(false) => {
                            self.phase = Phase::Abandon(AbandonContinuation::new(
                                self.target,
                                if self.token.is_some() {
                                    "syntax/unexpected-token-source"
                                } else {
                                    "syntax/unexpected-production-source"
                                },
                                if self.token.is_some() {
                                    "unexpected source where a required token was expected"
                                } else {
                                    "unexpected source where a required production was expected"
                                },
                            ));
                        }
                        BoundaryProgress::NeedInput => {
                            self.phase = Phase::Probe(character);
                            return Progress::NeedInput;
                        }
                        BoundaryProgress::NeedsProcessing => {
                            self.phase = Phase::Probe(character);
                            return Progress::NeedsProcessing;
                        }
                        BoundaryProgress::Limited => {
                            self.phase = Phase::Probe(character);
                            return Progress::Limited;
                        }
                    }
                }
                Phase::Missing(mut child) => {
                    let before = *allowance;
                    let result = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match result {
                        MissingProgress::Complete(_) => {
                            if let Some((_, text)) = self.token {
                                combinator::attach_missing_fix(parser, Some(text));
                            }
                        }
                        MissingProgress::NeedInput => {
                            self.phase = Phase::Missing(child);
                            return Progress::NeedInput;
                        }
                        MissingProgress::NeedsProcessing => {
                            self.phase = Phase::Missing(child);
                            return Progress::NeedsProcessing;
                        }
                        MissingProgress::Limited => {
                            self.phase = Phase::Missing(child);
                            return Progress::Limited;
                        }
                    }
                }
                Phase::Abandon(mut child) => {
                    let before = *allowance;
                    let result = child.advance(
                        parser,
                        final_input,
                        allowance,
                        |parser, ch, final_input, allowance| {
                            self.probe.advance(parser, ch, final_input, allowance)
                        },
                    );
                    self.work += before - *allowance;
                    match result {
                        AbandonProgress::Complete(_) => {}
                        AbandonProgress::NeedInput => {
                            self.phase = Phase::Abandon(child);
                            return Progress::NeedInput;
                        }
                        AbandonProgress::NeedsProcessing => {
                            self.phase = Phase::Abandon(child);
                            return Progress::NeedsProcessing;
                        }
                        AbandonProgress::Limited => {
                            self.phase = Phase::Abandon(child);
                            return Progress::Limited;
                        }
                    }
                }
                Phase::Done => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::continuation_test_support::{
        TestContinuation, TestProgress, assert_partitions, run,
    };
    use super::*;
    impl TestContinuation for Required<'static> {
        fn new(rule: RuleId) -> Self {
            let operator = [
                rules::L1,
                rules::L2,
                rules::L3,
                rules::L4,
                rules::L5,
                rules::L6,
                rules::L7,
            ]
            .iter()
            .position(|candidate| *candidate == rule);
            Self::new(
                rule,
                "syntax/test-required",
                "test required production",
                "expression",
                &[':'],
                &["=>", "⇒"],
                operator,
            )
        }
        fn work(&self) -> u64 {
            self.work
        }
        fn advance(
            &mut self,
            parser: &mut Parser<'_>,
            final_input: bool,
            allowance: &mut u64,
        ) -> TestProgress {
            match self.advance(parser, final_input, allowance) {
                Progress::Complete(result) => TestProgress::Complete(result),
                Progress::NeedInput => TestProgress::NeedInput,
                Progress::NeedsProcessing => TestProgress::NeedsProcessing,
                Progress::Limited => TestProgress::Limited,
            }
        }
    }
    #[test]
    fn required_recovery_retains_trivia_boundaries_prefixes_and_operator_lookahead() {
        assert_partitions::<Required<'static>>(&[
            (rules::FACTOR, ""),
            (rules::FACTOR, "   "),
            (rules::FACTOR, "  ;next"),
            (rules::FACTOR, " 	bad;next"),
            (rules::FACTOR, "bad=>next"),
            (rules::FACTOR, "bad⇒next"),
            (rules::FACTOR, "bad=next"),
            (rules::FACTOR, "[bad,source];next"),
            (rules::FACTOR, "\"é;💡\";next"),
            (rules::FACTOR, "<u8>;next"),
            (rules::FACTOR, "<a,b;next"),
            (rules::L1, "bad & next"),
            (rules::L2, "bad < next"),
            (rules::L2, "bad >= next"),
            (rules::L3, "@@   + next"),
            (rules::L3, "@@    x;next"),
            (rules::L4, "@@ ** next"),
            (rules::L5, "@@ ^ next"),
            (rules::L6, "@@;next"),
            (rules::L7, "@@;next"),
            (rules::L3, "+ next"),
            (rules::L3, "\r\nnext"),
            (rules::L3, " \t "),
        ]);
    }
    #[test]
    fn required_operator_recovery_reuses_rejected_long_trivia_and_exports_once() {
        for (prefix, unit, tail) in [
            ("@@", " ", "x;next"),
            ("@@", " ", "+next"),
            ("\"", "💡", "\";next"),
        ] {
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
                    run::<Required<'static>>(rules::L3, &text, &chunks, u64::MAX, true);
                let (expected, sealed_work) =
                    run::<Required<'static>>(rules::L3, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
    }
}

const CLOSER_BOUNDARIES: &[char] = &[
    ')', ']', '}', '>', '⟩', '╯', '┘', '┛', ',', ';', '|', '│', '┃', '\n', '\r',
];
enum CloserPhase<'a> {
    Abandon(AbandonContinuation<'a>),
    Token(base::continuation::Continuation),
    Shell(structure_shell::Continuation),
    Missing(MissingContinuation<'a>),
    Done,
}
pub(super) struct Closer<'a> {
    close_rule: RuleId,
    close_kind: SyntaxKind,
    close_text: &'a str,
    boundaries: &'a [char],
    phase: CloserPhase<'a>,
    pub work: u64,
}
impl<'a> Closer<'a> {
    pub fn set(
        target: RuleId,
        close_rule: RuleId,
        close_kind: SyntaxKind,
        close_text: &'a str,
        boundaries: &'a [char],
    ) -> Self {
        let mut owner = Self::new(target, close_rule, close_kind, close_text);
        owner.boundaries = boundaries;
        owner
    }
    fn token(&mut self) {
        self.phase = if structure_shell::supports(self.close_rule) {
            CloserPhase::Shell(structure_shell::Continuation::new(self.close_rule))
        } else {
            CloserPhase::Token(base::continuation::Continuation::new(self.close_rule))
        };
    }

    pub fn new(
        target: RuleId,
        close_rule: RuleId,
        close_kind: SyntaxKind,
        close_text: &'a str,
    ) -> Self {
        Self {
            close_rule,
            close_kind,
            close_text,
            boundaries: CLOSER_BOUNDARIES,
            phase: CloserPhase::Abandon(AbandonContinuation::new(
                target,
                "syntax/unexpected-delimited-content",
                "unexpected source before the closing delimiter",
            )),
            work: 0,
        }
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        loop {
            if matches!(self.phase, CloserPhase::Done) {
                return Progress::Complete(Attempt::Committed);
            }
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let before = *allowance;
            let phase = core::mem::replace(&mut self.phase, CloserPhase::Done);
            let result = match phase {
                CloserPhase::Abandon(mut child) => {
                    match child.advance(parser, final_input, allowance, |_, ch, _, allowance| {
                        if *allowance == 0 {
                            return BoundaryProgress::NeedsProcessing;
                        }
                        *allowance -= 1;
                        BoundaryProgress::Complete(self.boundaries.contains(&ch))
                    }) {
                        AbandonProgress::Complete(_) => {
                            self.token();
                            None
                        }
                        AbandonProgress::NeedInput => {
                            self.phase = CloserPhase::Abandon(child);
                            Some(Progress::NeedInput)
                        }
                        AbandonProgress::NeedsProcessing => {
                            self.phase = CloserPhase::Abandon(child);
                            Some(Progress::NeedsProcessing)
                        }
                        AbandonProgress::Limited => {
                            self.phase = CloserPhase::Abandon(child);
                            Some(Progress::Limited)
                        }
                    }
                }
                CloserPhase::Token(mut child) => {
                    match child.advance(parser, final_input, allowance) {
                        base::continuation::Progress::Complete(matched) => {
                            if !matched {
                                self.phase = CloserPhase::Missing(MissingContinuation::new(
                                    "syntax/missing-delimiter",
                                    "missing closing delimiter",
                                    ExpectedSyntax::Token(self.close_kind),
                                    Some(self.close_kind),
                                ));
                            }
                            None
                        }
                        base::continuation::Progress::NeedInput => {
                            self.phase = CloserPhase::Token(child);
                            Some(Progress::NeedInput)
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.phase = CloserPhase::Token(child);
                            Some(Progress::NeedsProcessing)
                        }
                        base::continuation::Progress::Limited => {
                            self.phase = CloserPhase::Token(child);
                            Some(Progress::Limited)
                        }
                    }
                }
                CloserPhase::Shell(mut child) => {
                    match child.advance(parser, final_input, allowance) {
                        structure_shell::Progress::Complete(result) => {
                            if result != Attempt::Matched {
                                self.phase = CloserPhase::Missing(MissingContinuation::new(
                                    "syntax/missing-delimiter",
                                    "missing closing delimiter",
                                    ExpectedSyntax::Token(self.close_kind),
                                    Some(self.close_kind),
                                ));
                            }
                            None
                        }
                        structure_shell::Progress::NeedInput => {
                            self.phase = CloserPhase::Shell(child);
                            Some(Progress::NeedInput)
                        }
                        structure_shell::Progress::NeedsProcessing => {
                            self.phase = CloserPhase::Shell(child);
                            Some(Progress::NeedsProcessing)
                        }
                        structure_shell::Progress::Limited => {
                            self.phase = CloserPhase::Shell(child);
                            Some(Progress::Limited)
                        }
                    }
                }
                CloserPhase::Missing(mut child) => {
                    match child.advance(parser, final_input, allowance) {
                        MissingProgress::Complete(_) => {
                            combinator::attach_missing_fix(parser, Some(self.close_text));
                            None
                        }
                        MissingProgress::NeedInput => {
                            self.phase = CloserPhase::Missing(child);
                            Some(Progress::NeedInput)
                        }
                        MissingProgress::NeedsProcessing => {
                            self.phase = CloserPhase::Missing(child);
                            Some(Progress::NeedsProcessing)
                        }
                        MissingProgress::Limited => {
                            self.phase = CloserPhase::Missing(child);
                            Some(Progress::Limited)
                        }
                    }
                }
                CloserPhase::Done => unreachable!(),
            };
            self.work += before - *allowance;
            if let Some(result) = result {
                return result;
            }
        }
    }
}
