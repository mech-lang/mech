//! Shared error-skip recovery, including retained context lookahead and export.
use super::*;
use crate::document::TextSize;
use crate::document::parser::checkpoint::ParserCheckpoint;
use crate::document::parser::context_probe::{
    FenceProbe, Progress as ProbeProgress, SubtitleProbe,
};
use crate::document::parser::grapheme_scan::ScanSource;
use crate::document::parser::marker::Marker;

pub(crate) enum SkipProgress {
    Complete(Option<CompletedMarker>),
    NeedInput,
    NeedsProcessing,
    Limited,
}
enum Phase {
    Start,
    Boundary,
    Subtitle(SubtitleProbe),
    Fence(FenceProbe),
    Consume,
    Finish,
    Found {
        error: CompletedMarker,
        range: TextRange,
        at: TextSize,
        text: String,
    },
    Done(Option<CompletedMarker>),
}
pub(crate) struct SkipContinuation<'a> {
    class: RecoveryClass,
    code: &'a str,
    message: &'a str,
    start: TextSize,
    marker: Option<Marker>,
    checkpoint: Option<ParserCheckpoint>,
    phase: Phase,
    // A failed fence probe proves all intermediate indentation starts are not
    // fences either. Reusing that fact prevents repeated long-space lookahead.
    non_fence_until: TextSize,
    pub work: u64,
}
impl<'a> SkipContinuation<'a> {
    pub fn new(class: RecoveryClass, code: &'a str, message: &'a str) -> Self {
        Self {
            class,
            code,
            message,
            start: TextSize::ZERO,
            marker: None,
            checkpoint: None,
            phase: Phase::Start,
            non_fence_until: TextSize::ZERO,
            work: 0,
        }
    }
    fn fence_or_consume(&mut self, parser: &Parser<'_>) {
        self.phase = if parser.offset() < self.non_fence_until {
            Phase::Consume
        } else {
            Phase::Fence(FenceProbe::new())
        };
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> SkipProgress {
        loop {
            if let Phase::Done(result) = self.phase {
                return SkipProgress::Complete(result);
            }
            if !final_input && parser.is_halted() {
                return SkipProgress::Limited;
            }
            if *allowance == 0 {
                return SkipProgress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Finish);
            if !matches!(phase, Phase::Subtitle(_) | Phase::Fence(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match phase {
                Phase::Start => {
                    if !parser.consuming_recovery_allowed() {
                        self.phase = Phase::Done(None);
                        continue;
                    }
                    self.start = parser.offset();
                    self.checkpoint = Some(parser.checkpoint());
                    self.marker = Some(parser.start());
                    self.phase = Phase::Boundary;
                }
                Phase::Boundary => {
                    if parser.is_eof() {
                        if !final_input && !parser.is_halted() {
                            self.phase = Phase::Boundary;
                            return SkipProgress::NeedInput;
                        }
                    } else if !parser.is_halted() {
                        if parser.offset() == self.start {
                            self.phase = Phase::Consume;
                        } else {
                            match self.class {
                                RecoveryClass::Fence => self.phase = Phase::Consume,
                                RecoveryClass::MechItem | RecoveryClass::Paragraph
                                    if is_newline_start(parser.cursor()) => {}
                                RecoveryClass::MechItem if parser.cursor().starts_with(";") => {}
                                RecoveryClass::MechItem => {
                                    if parser.cursor().context_view().is_line_start() {
                                        self.phase = Phase::Subtitle(SubtitleProbe::new());
                                    } else {
                                        self.phase = Phase::Consume;
                                    }
                                }
                                RecoveryClass::Paragraph => self.fence_or_consume(parser),
                            }
                        }
                    }
                }
                Phase::Subtitle(mut probe) => {
                    let before = *allowance;
                    let progress =
                        probe.advance(parser.cursor().context_view(), final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        ProbeProgress::Complete(true) => {}
                        ProbeProgress::Complete(false) => self.fence_or_consume(parser),
                        ProbeProgress::NeedInput => {
                            self.phase = Phase::Subtitle(probe);
                            return SkipProgress::NeedInput;
                        }
                        ProbeProgress::NeedsProcessing => {
                            self.phase = Phase::Subtitle(probe);
                            return SkipProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Fence(mut probe) => {
                    let before = *allowance;
                    let progress =
                        probe.advance(parser.cursor().context_view(), final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        ProbeProgress::Complete(Some(_)) => {}
                        ProbeProgress::Complete(None) => {
                            self.non_fence_until =
                                TextSize(parser.offset().0 + probe.indentation_bytes());
                            self.phase = Phase::Consume;
                        }
                        ProbeProgress::NeedInput => {
                            self.phase = Phase::Fence(probe);
                            return SkipProgress::NeedInput;
                        }
                        ProbeProgress::NeedsProcessing => {
                            self.phase = Phase::Fence(probe);
                            return SkipProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Consume => {
                    if let Some(character) = parser.cursor().peek_char() {
                        if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
                            parser.halt();
                        } else if let Some((character, range)) = parser.bump_char_raw() {
                            charge_recovery_bytes(parser, range.len().0);
                            parser.token_with_flags(
                                token_kind_for_char(character),
                                range,
                                TokenFlags::ERROR,
                            );
                            self.phase = Phase::Boundary;
                        }
                    }
                }
                Phase::Finish => {
                    let marker = self.marker.take().expect("started recovery owner");
                    if parser.offset() == self.start {
                        if parser.state.resource_finalizing {
                            parser.rewind(self.checkpoint.expect("recovery checkpoint"));
                        } else {
                            marker.abandon(parser);
                        }
                        self.phase = Phase::Done(None);
                    } else {
                        let error =
                            marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
                        let range = TextRange::new(self.start, parser.offset());
                        self.phase = Phase::Found {
                            error,
                            range,
                            at: range.start,
                            text: String::new(),
                        };
                    }
                }
                Phase::Found {
                    error,
                    range,
                    at,
                    mut text,
                } => {
                    if at < range.end {
                        let scalar = parser
                            .source()
                            .scalar_at(at.to_usize())
                            .expect("retained recovery source scalar");
                        let next = TextSize(at.0 + scalar.len() as u32);
                        assert!(next <= range.end);
                        text.push_str(scalar);
                        self.phase = Phase::Found {
                            error,
                            range,
                            at: next,
                            text,
                        };
                    } else {
                        let diagnostic = Diagnostic {
                            id: parser.next_diagnostic_id(),
                            code: DiagnosticCode::from(self.code),
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
                            message: String::from(self.message),
                        };
                        parser.push_diagnostic(
                            diagnostic,
                            Some(error.position()),
                            TextRange::new(TextSize::ZERO, range.len()),
                        );
                        self.phase = Phase::Done(Some(error));
                    }
                }
                Phase::Done(_) => unreachable!("completed recovery handled before work charge"),
            }
        }
    }
}
