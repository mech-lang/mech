//! Retained balanced recovery. Owners supply their canonical restart probe;
//! annotation lookahead and source export remain children of this scanner.
use super::*;
use crate::document::TextSize;
use crate::document::parser::canonical::recursive_core;
use crate::document::parser::checkpoint::ParserCheckpoint;
use crate::document::parser::grapheme_scan::ScanSource;
use crate::document::parser::marker::Marker;
use alloc::{boxed::Box, vec::Vec};

pub(crate) enum BoundaryProgress {
    Complete(bool),
    NeedInput,
    NeedsProcessing,
    Limited,
}
impl From<recursive_core::Progress> for BoundaryProgress {
    fn from(progress: recursive_core::Progress) -> Self {
        match progress {
            recursive_core::Progress::Complete(result) => {
                Self::Complete(result == super::super::canonical::combinator::Attempt::Matched)
            }
            recursive_core::Progress::NeedInput => Self::NeedInput,
            recursive_core::Progress::NeedsProcessing => Self::NeedsProcessing,
            recursive_core::Progress::Limited => Self::Limited,
        }
    }
}
pub(crate) enum AbandonProgress {
    Complete(Option<CompletedMarker>),
    NeedInput,
    NeedsProcessing,
    Limited,
}
enum Phase {
    Start,
    Scan,
    Boundary(char),
    Angle(char),
    Annotation(char, ParserCheckpoint, Box<recursive_core::Continuation>),
    Consume(char, bool),
    Triple(u8),
    FinalBoundary,
    FinalProbe(char),
    Finish,
    Export(CompletedMarker, TextRange, TextSize, String),
    Done(Option<CompletedMarker>),
}
pub(crate) struct AbandonContinuation<'a> {
    target: RuleId,
    code: &'a str,
    message: &'a str,
    start: TextSize,
    checkpoint: Option<ParserCheckpoint>,
    marker: Option<Marker>,
    delimiters: Vec<char>,
    quoted: Option<char>,
    raw_triple: bool,
    escaped: bool,
    phase: Phase,
    pub work: u64,
}
impl<'a> AbandonContinuation<'a> {
    pub fn new(target: RuleId, code: &'a str, message: &'a str) -> Self {
        Self {
            target,
            code,
            message,
            start: TextSize::ZERO,
            checkpoint: None,
            marker: None,
            delimiters: Vec::new(),
            quoted: None,
            raw_triple: false,
            escaped: false,
            phase: Phase::Start,
            work: 0,
        }
    }
    fn exhausted(&mut self, parser: &mut Parser<'_>, stopped: bool) {
        if remaining_recovery_bytes(parser) == 0 && !parser.is_eof() && !stopped {
            parser.halt();
        }
        self.phase = Phase::Finish;
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
        mut boundary: impl FnMut(&mut Parser<'_>, char, bool, &mut u64) -> BoundaryProgress,
    ) -> AbandonProgress {
        loop {
            if let Phase::Done(result) = self.phase {
                return AbandonProgress::Complete(result);
            }
            if !final_input && parser.is_halted() {
                return AbandonProgress::Limited;
            }
            if *allowance == 0 {
                return AbandonProgress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Finish);
            if !matches!(
                phase,
                Phase::Boundary(_) | Phase::FinalProbe(_) | Phase::Annotation(..)
            ) {
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
                    self.phase = Phase::Scan;
                }
                Phase::Scan => {
                    if parser.is_eof() && !final_input {
                        self.phase = Phase::Scan;
                        return AbandonProgress::NeedInput;
                    }
                    if parser.is_eof()
                        || parser.is_halted()
                        || remaining_recovery_bytes(parser) == 0
                    {
                        self.phase = Phase::FinalBoundary;
                        continue;
                    }
                    if self.quoted.is_none() && parser.cursor().byte() == Some(b'"') {
                        if !final_input
                            && (parser.cursor().byte_at(1).is_none()
                                || (parser.cursor().byte_at(1) == Some(b'"')
                                    && parser.cursor().byte_at(2).is_none()))
                        {
                            self.phase = Phase::Scan;
                            return AbandonProgress::NeedInput;
                        }
                        if parser.cursor().starts_with("\"\"\"") {
                            if remaining_recovery_bytes(parser) < 3 {
                                parser.halt();
                                self.phase = Phase::FinalBoundary;
                            } else {
                                self.phase = Phase::Triple(3);
                            }
                            continue;
                        }
                    }
                    if let Some(character) = parser.cursor().peek_char() {
                        self.phase = if self.quoted.is_none() && !self.raw_triple {
                            Phase::Boundary(character)
                        } else {
                            Phase::Angle(character)
                        };
                    } else {
                        self.phase = Phase::FinalBoundary;
                    }
                }
                Phase::Boundary(character) | Phase::FinalProbe(character) => {
                    let final_probe = matches!(phase, Phase::FinalProbe(_));
                    let before = *allowance;
                    let progress = boundary(parser, character, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        BoundaryProgress::Complete(stop) => {
                            let stopped = recovery_boundary(character, &self.delimiters, stop);
                            if final_probe {
                                self.exhausted(parser, stopped);
                            } else {
                                self.phase = if stopped {
                                    Phase::FinalBoundary
                                } else {
                                    Phase::Angle(character)
                                };
                            }
                        }
                        BoundaryProgress::NeedInput => {
                            self.phase = if final_probe {
                                Phase::FinalProbe(character)
                            } else {
                                Phase::Boundary(character)
                            };
                            return AbandonProgress::NeedInput;
                        }
                        BoundaryProgress::NeedsProcessing => {
                            self.phase = if final_probe {
                                Phase::FinalProbe(character)
                            } else {
                                Phase::Boundary(character)
                            };
                            return AbandonProgress::NeedsProcessing;
                        }
                        BoundaryProgress::Limited => {
                            self.phase = if final_probe {
                                Phase::FinalProbe(character)
                            } else {
                                Phase::Boundary(character)
                            };
                            return AbandonProgress::Limited;
                        }
                    }
                }
                Phase::Angle(character) => {
                    if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
                        parser.halt();
                        self.phase = Phase::FinalBoundary;
                    } else if matches!(character, '<' | '⟨') {
                        if !final_input && character == '<' && parser.cursor().byte_at(1).is_none()
                        {
                            self.phase = Phase::Angle(character);
                            return AbandonProgress::NeedInput;
                        }
                        if ["<-", "<=", "<+"]
                            .iter()
                            .any(|prefix| parser.cursor().starts_with(prefix))
                        {
                            self.phase = Phase::Consume(character, false);
                        } else {
                            self.phase = Phase::Annotation(
                                character,
                                parser.checkpoint(),
                                Box::new(recursive_core::Continuation::annotation(false)),
                            );
                        }
                    } else {
                        self.phase = Phase::Consume(character, false);
                    }
                }
                Phase::Annotation(character, checkpoint, mut child) => {
                    let before = *allowance;
                    let progress =
                        BoundaryProgress::from(child.advance(parser, final_input, allowance));
                    self.work += before - *allowance;
                    match progress {
                        BoundaryProgress::Complete(matched) => {
                            parser.rewind(checkpoint);
                            self.phase = if parser.is_halted() {
                                Phase::FinalBoundary
                            } else {
                                Phase::Consume(character, matched)
                            };
                        }
                        BoundaryProgress::NeedInput => {
                            self.phase = Phase::Annotation(character, checkpoint, child);
                            return AbandonProgress::NeedInput;
                        }
                        BoundaryProgress::NeedsProcessing => {
                            self.phase = Phase::Annotation(character, checkpoint, child);
                            return AbandonProgress::NeedsProcessing;
                        }
                        BoundaryProgress::Limited => {
                            self.phase = Phase::Annotation(character, checkpoint, child);
                            return AbandonProgress::Limited;
                        }
                    }
                }
                Phase::Consume(character, opens_angle) => {
                    if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
                        parser.halt();
                        self.phase = Phase::FinalBoundary;
                        continue;
                    }
                    let Some((character, range)) = parser.bump_char_raw() else {
                        self.phase = Phase::FinalBoundary;
                        continue;
                    };
                    charge_recovery_bytes(parser, range.len().0);
                    parser.token_with_flags(
                        token_kind_for_char(character),
                        range,
                        TokenFlags::ERROR,
                    );
                    self.phase = Phase::Scan;
                    if self.raw_triple {
                        continue;
                    }
                    if let Some(quote) = self.quoted {
                        if self.escaped {
                            self.escaped = false;
                        } else if character == '\\' {
                            self.escaped = true;
                        } else if character == quote {
                            self.quoted = None;
                        }
                        continue;
                    }
                    match character {
                        '"' => self.quoted = Some(character),
                        '(' | '[' | '{' => self.delimiters.push(character),
                        '<' | '⟨' if opens_angle => self.delimiters.push(character),
                        ')' | ']' | '}' | '>' | '⟩' => {
                            if self
                                .delimiters
                                .last()
                                .is_some_and(|opener| delimiters_match(*opener, character))
                            {
                                self.delimiters.pop();
                            }
                        }
                        _ => {}
                    }
                }
                Phase::Triple(left) => {
                    if let Some((character, range)) = parser.bump_char_raw() {
                        charge_recovery_bytes(parser, range.len().0);
                        parser.token_with_flags(
                            token_kind_for_char(character),
                            range,
                            TokenFlags::ERROR,
                        );
                        if left > 1 {
                            self.phase = Phase::Triple(left - 1);
                            continue;
                        }
                    } else {
                        parser.halt();
                    }
                    self.raw_triple = !self.raw_triple;
                    self.phase = Phase::Scan;
                }
                Phase::FinalBoundary => {
                    if self.quoted.is_none() && !self.raw_triple {
                        if let Some(character) = parser.cursor().peek_char() {
                            self.phase = Phase::FinalProbe(character);
                            continue;
                        }
                    }
                    self.exhausted(parser, false);
                }
                Phase::Finish => {
                    let marker = self.marker.take().expect("started abandonment");
                    if parser.offset() == self.start {
                        if parser.state.resource_finalizing {
                            parser.rewind(self.checkpoint.expect("abandonment checkpoint"));
                        } else {
                            marker.abandon(parser);
                        }
                        self.phase = Phase::Done(None);
                    } else {
                        let error =
                            marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
                        let range = TextRange::new(self.start, parser.offset());
                        self.phase = Phase::Export(error, range, range.start, String::new());
                    }
                }
                Phase::Export(error, range, at, mut text) => {
                    if at < range.end {
                        let character = parser
                            .source()
                            .scalar_at(at.to_usize())
                            .expect("recovery scalar");
                        text.push_str(character);
                        self.phase = Phase::Export(
                            error,
                            range,
                            TextSize(at.0 + character.len() as u32),
                            text,
                        );
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
                            recovery: Some(RecoveryAction::Abandon {
                                rule: self.target,
                                at: parser.offset(),
                            }),
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
                Phase::Done(_) => unreachable!(),
            }
        }
    }
}
