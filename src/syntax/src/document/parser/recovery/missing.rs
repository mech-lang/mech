//! Missing-node recovery with retained physical found-syntax classification.
use super::*;
use crate::document::TextSize;
use crate::document::parser::LexicalMode;
use crate::document::parser::canonical::found::{
    FilteredContinuation, SourceContinuation, SourceProgress,
};

enum Phase {
    Start,
    Classify,
    Source(SourceContinuation),
    Filtered(FilteredContinuation),
    Publish(FoundSyntax),
    Done,
}
pub(crate) enum MissingProgress {
    Complete(CompletedMarker),
    NeedInput,
    NeedsProcessing,
    Limited,
}
pub(crate) struct MissingContinuation<'a> {
    code: &'a str,
    message: &'a str,
    expected: Option<ExpectedSyntax>,
    token: Option<SyntaxKind>,
    at: TextSize,
    marker: Option<CompletedMarker>,
    phase: Phase,
    pub work: u64,
}
impl<'a> MissingContinuation<'a> {
    pub fn new(
        code: &'a str,
        message: &'a str,
        expected: ExpectedSyntax,
        token: Option<SyntaxKind>,
    ) -> Self {
        Self {
            code,
            message,
            expected: Some(expected),
            token,
            at: TextSize::ZERO,
            marker: None,
            phase: Phase::Start,
            work: 0,
        }
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> MissingProgress {
        loop {
            if matches!(self.phase, Phase::Done) {
                return MissingProgress::Complete(self.marker.expect("completed missing node"));
            }
            if !final_input && parser.is_halted() {
                return MissingProgress::Limited;
            }
            if *allowance == 0 {
                return MissingProgress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Done);
            if !matches!(phase, Phase::Source(_) | Phase::Filtered(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match phase {
                Phase::Start => {
                    self.at = parser.offset();
                    let marker = parser.start();
                    if let Some(token) = self.token {
                        parser.missing_token(token);
                    }
                    self.marker = Some(marker.complete_with_flags(
                        parser,
                        SyntaxKind::Missing,
                        NodeFlags::MISSING,
                    ));
                    self.phase = Phase::Classify;
                }
                Phase::Classify => match parser.state.lexical_mode {
                    LexicalMode::CanonicalSourceFragment => {
                        self.phase = Phase::Source(SourceContinuation::new(parser.offset()))
                    }
                    LexicalMode::PrototypeDocument => {
                        if !final_input && parser.offset() >= parser.cursor().context_end() {
                            self.phase = Phase::Classify;
                            return MissingProgress::NeedInput;
                        }
                        self.phase = Phase::Publish(parser.found_syntax());
                    }
                    LexicalMode::CanonicalGrammar => {
                        self.phase = Phase::Filtered(FilteredContinuation::new(parser.offset()));
                    }
                },
                Phase::Source(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(
                        parser.source(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        SourceProgress::Complete(found) => self.phase = Phase::Publish(found),
                        SourceProgress::NeedInput => {
                            self.phase = Phase::Source(continuation);
                            return MissingProgress::NeedInput;
                        }
                        SourceProgress::NeedsProcessing => {
                            self.phase = Phase::Source(continuation);
                            return MissingProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Filtered(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(
                        parser.source(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        SourceProgress::Complete(found) => self.phase = Phase::Publish(found),
                        SourceProgress::NeedInput => {
                            self.phase = Phase::Filtered(continuation);
                            return MissingProgress::NeedInput;
                        }
                        SourceProgress::NeedsProcessing => {
                            self.phase = Phase::Filtered(continuation);
                            return MissingProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Publish(found) => {
                    let expected = self.expected.take().expect("one missing diagnostic");
                    let diagnostic = Diagnostic {
                        id: parser.next_diagnostic_id(),
                        code: DiagnosticCode::from(self.code),
                        phase: DiagnosticPhase::Syntax,
                        severity: Severity::Error,
                        rule: parser.current_rule(),
                        context: parser.current_context(),
                        primary: DiagnosticAnchor::Absolute {
                            revision: parser.source().revision(),
                            range: TextRange::empty(self.at),
                        },
                        labels: alloc::vec![],
                        expected: alloc::vec![expected.clone()],
                        found: Some(found),
                        fixes: alloc::vec![],
                        related: alloc::vec![],
                        recovery: Some(RecoveryAction::Insert {
                            syntax: expected,
                            at: self.at,
                        }),
                        tags: DiagnosticTags::NONE,
                        message: String::from(self.message),
                    };
                    parser.push_diagnostic(
                        diagnostic,
                        Some(self.marker.expect("missing node").position()),
                        TextRange::empty(TextSize::ZERO),
                    );
                }
                Phase::Done => unreachable!("missing recovery completion handled before work"),
            }
        }
    }
}
