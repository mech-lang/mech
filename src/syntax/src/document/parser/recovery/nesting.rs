//! Retained nesting-limit recovery and strong document-boundary lookahead.
use super::*;
use crate::document::TextSize;
use crate::document::parser::{
    ParserCheckpoint,
    context_probe::{FenceProbe, Progress as ProbeProgress, SubtitleProbe},
    grapheme_scan::ScanSource,
    marker::Marker,
};
use alloc::boxed::Box;
pub(crate) enum NestingProgress {
    Complete,
    NeedInput,
    NeedsProcessing,
    Limited,
}
enum Phase {
    Start,
    Scan,
    Boundary(bool),
    Subtitle(bool, SubtitleProbe),
    Fence(bool, FenceProbe),
    Consume,
    Finish,
    Missing(Box<MissingContinuation<'static>>),
    Export(CompletedMarker, TextRange, TextSize, String),
    Done,
}
pub(crate) struct NestingContinuation {
    start: TextSize,
    checkpoint: Option<ParserCheckpoint>,
    marker: Option<Marker>,
    nested: u32,
    phase: Phase,
    pub work: u64,
}
impl NestingContinuation {
    pub fn new() -> Self {
        Self {
            start: TextSize::ZERO,
            checkpoint: None,
            marker: None,
            nested: 0,
            phase: Phase::Start,
            work: 0,
        }
    }
    fn boundary(&mut self, parser: &mut Parser<'_>, final_check: bool, stop: bool) {
        if final_check {
            if !stop {
                parser.halt();
            }
            self.phase = Phase::Finish;
        } else {
            self.phase = if stop { Phase::Finish } else { Phase::Consume };
        }
    }
    fn exhaustion(&mut self, parser: &Parser<'_>) {
        self.phase = if remaining_recovery_bytes(parser) == 0 && !parser.is_eof() {
            Phase::Boundary(true)
        } else {
            Phase::Finish
        };
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> NestingProgress {
        loop {
            if matches!(self.phase, Phase::Done) {
                return NestingProgress::Complete;
            }
            if !final_input && parser.is_halted() {
                return NestingProgress::Limited;
            }
            if *allowance == 0 {
                return NestingProgress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Done);
            if !matches!(
                phase,
                Phase::Subtitle(..) | Phase::Fence(..) | Phase::Missing(_)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match phase {
                Phase::Start => {
                    if !parser.consuming_recovery_allowed() {
                        self.phase = Phase::Done;
                        continue;
                    }
                    self.start = parser.offset();
                    self.checkpoint = Some(parser.checkpoint());
                    self.marker = Some(parser.start());
                    self.phase = Phase::Scan;
                }
                Phase::Scan => {
                    if parser.is_eof() && !final_input {
                        self.phase = Phase::Scan;
                        return NestingProgress::NeedInput;
                    }
                    if parser.is_eof() || remaining_recovery_bytes(parser) == 0 {
                        self.exhaustion(parser);
                    } else {
                        self.phase = Phase::Boundary(false);
                    }
                }
                Phase::Boundary(final_check) => {
                    if self.nested != 0 {
                        self.boundary(parser, final_check, false);
                        continue;
                    }
                    let byte = parser.cursor().byte();
                    if matches!(byte, Some(b')' | b';' | b'\r' | b'\n')) {
                        self.boundary(parser, final_check, true);
                    } else if matches!(byte, Some(b'-' | b'/'))
                        && parser.cursor().byte_at(1).is_none()
                        && !final_input
                    {
                        self.phase = Phase::Boundary(final_check);
                        return NestingProgress::NeedInput;
                    } else if parser.cursor().starts_with("--") || parser.cursor().starts_with("//")
                    {
                        self.boundary(parser, final_check, true);
                    } else if parser.cursor().context_view().is_line_start() {
                        self.phase = Phase::Subtitle(final_check, SubtitleProbe::new());
                    } else {
                        self.boundary(parser, final_check, false);
                    }
                }
                Phase::Subtitle(final_check, mut child) => {
                    let before = *allowance;
                    let progress =
                        child.advance(parser.cursor().context_view(), final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        ProbeProgress::Complete(true) => self.boundary(parser, final_check, true),
                        ProbeProgress::Complete(false) => {
                            self.phase = Phase::Fence(final_check, FenceProbe::new())
                        }
                        ProbeProgress::NeedInput => {
                            self.phase = Phase::Subtitle(final_check, child);
                            return NestingProgress::NeedInput;
                        }
                        ProbeProgress::NeedsProcessing => {
                            self.phase = Phase::Subtitle(final_check, child);
                            return NestingProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Fence(final_check, mut child) => {
                    let before = *allowance;
                    let progress =
                        child.advance(parser.cursor().context_view(), final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        ProbeProgress::Complete(found) => {
                            self.boundary(parser, final_check, found.is_some())
                        }
                        ProbeProgress::NeedInput => {
                            self.phase = Phase::Fence(final_check, child);
                            return NestingProgress::NeedInput;
                        }
                        ProbeProgress::NeedsProcessing => {
                            self.phase = Phase::Fence(final_check, child);
                            return NestingProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Consume => {
                    let Some(character) = parser.cursor().peek_char() else {
                        self.exhaustion(parser);
                        continue;
                    };
                    if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
                        parser.halt();
                        self.exhaustion(parser);
                        continue;
                    }
                    let Some((character, range)) = parser.bump_char_raw() else {
                        self.exhaustion(parser);
                        continue;
                    };
                    if character == '(' {
                        self.nested = self.nested.saturating_add(1);
                    } else if character == ')' {
                        self.nested = self.nested.saturating_sub(1);
                    }
                    charge_recovery_bytes(parser, range.len().0);
                    parser.token_with_flags(
                        token_kind_for_char(character),
                        range,
                        TokenFlags::ERROR,
                    );
                    self.phase = Phase::Scan;
                }
                Phase::Finish => {
                    let marker = self.marker.take().expect("nesting recovery marker");
                    if self.start == parser.offset() {
                        if parser.state.resource_finalizing {
                            parser.rewind(self.checkpoint.expect("nesting recovery checkpoint"));
                        } else {
                            marker.abandon(parser);
                        }
                        self.phase = Phase::Missing(Box::new(MissingContinuation::new(
                            "syntax/nesting-limit",
                            "syntax nesting limit reached",
                            ExpectedSyntax::Production(String::from("expression")),
                            None,
                        )));
                    } else {
                        let error =
                            marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
                        let range = TextRange::new(self.start, parser.offset());
                        self.phase = Phase::Export(error, range, range.start, String::new());
                    }
                }
                Phase::Missing(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        MissingProgress::Complete(_) => {}
                        MissingProgress::NeedInput => {
                            self.phase = Phase::Missing(child);
                            return NestingProgress::NeedInput;
                        }
                        MissingProgress::NeedsProcessing => {
                            self.phase = Phase::Missing(child);
                            return NestingProgress::NeedsProcessing;
                        }
                        MissingProgress::Limited => {
                            self.phase = Phase::Missing(child);
                            return NestingProgress::Limited;
                        }
                    }
                }
                Phase::Export(error, range, at, mut text) => {
                    if at < range.end {
                        let scalar = parser
                            .source()
                            .scalar_at(at.to_usize())
                            .expect("nesting recovery scalar");
                        text.push_str(scalar);
                        self.phase =
                            Phase::Export(error, range, TextSize(at.0 + scalar.len() as u32), text);
                    } else {
                        let diagnostic = Diagnostic {
                            id: parser.next_diagnostic_id(),
                            code: DiagnosticCode::syntax("nesting-limit"),
                            phase: DiagnosticPhase::Syntax,
                            severity: Severity::Error,
                            rule: parser.current_rule(),
                            context: parser.current_context(),
                            primary: DiagnosticAnchor::Absolute {
                                revision: parser.source().revision(),
                                range,
                            },
                            labels: alloc::vec![],
                            expected: alloc::vec![],
                            found: Some(FoundSyntax {
                                kind: Some(SyntaxKind::Unknown),
                                text: Some(text),
                            }),
                            fixes: alloc::vec![],
                            related: alloc::vec![],
                            recovery: Some(RecoveryAction::Skip { range }),
                            tags: DiagnosticTags::NONE,
                            message: String::from("syntax nesting limit reached"),
                        };
                        parser.push_diagnostic(
                            diagnostic,
                            Some(error.position()),
                            TextRange::new(TextSize::ZERO, range.len()),
                        );
                    }
                }
                Phase::Done => unreachable!(),
            }
        }
    }
}
