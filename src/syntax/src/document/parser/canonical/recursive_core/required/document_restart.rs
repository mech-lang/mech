//! Canonical document and statement prefixes that delimited recovery leaves
//! available to the enclosing unfenced Mech/document parser.
use super::*;
use crate::document::parser::context_probe::{
    FenceProbe, Progress as ProbeProgress, SubtitleProbe,
};
use crate::document::parser::terminal::is_horizontal_space;

enum Part {
    Tilde,
    Space,
    Define,
    Assign,
}
enum Phase {
    Start,
    Attachment(TextSize),
    Subtitle(SubtitleProbe),
    Fence(FenceProbe),
    Assignment(ParserCheckpoint),
    Document(
        Box<crate::document::parser::canonical::document::continuation::Continuation<'static>>,
        ParserCheckpoint,
    ),
    Variable(Box<Continuation>, ParserCheckpoint),
    Base(Part, base::continuation::Continuation, ParserCheckpoint),
    Done(bool),
}
pub(in super::super) struct DocumentRestart {
    phase: Phase,
    line_start: TextSize,
}
impl DocumentRestart {
    pub fn new() -> Self {
        Self {
            phase: Phase::Start,
            line_start: TextSize::ZERO,
        }
    }
    pub fn is_finished(&self) -> bool {
        matches!(self.phase, Phase::Done(_))
    }
    fn base(&mut self, part: Part, rule: RuleId, checkpoint: ParserCheckpoint) {
        self.phase = Phase::Base(
            part,
            base::continuation::Continuation::new(rule),
            checkpoint,
        );
    }
    fn done(&mut self, parser: &mut Parser<'_>, checkpoint: ParserCheckpoint, found: bool) {
        parser.rewind(checkpoint);
        self.phase = Phase::Done(found);
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> BoundaryProgress {
        loop {
            let phase = core::mem::replace(&mut self.phase, Phase::Done(false));
            match phase {
                Phase::Done(result) => {
                    self.phase = Phase::Done(result);
                    return BoundaryProgress::Complete(result);
                }
                Phase::Start => {
                    if !parser.state.document_mech
                        || parser.state.fenced_mech
                        || parser.cursor().peek_char().is_some_and(is_horizontal_space)
                    {
                        return BoundaryProgress::Complete(false);
                    }
                    self.phase = Phase::Attachment(parser.offset());
                }
                Phase::Attachment(at) => {
                    if *allowance == 0 {
                        self.phase = Phase::Attachment(at);
                        return BoundaryProgress::NeedsProcessing;
                    }
                    *allowance -= 1;
                    let prior = parser
                        .source()
                        .chunk_before(at)
                        .and_then(|chunk| chunk.text.chars().next_back());
                    match prior {
                        Some(ch) if is_horizontal_space(ch) => {
                            self.phase = Phase::Attachment(TextSize(at.0 - ch.len_utf8() as u32));
                        }
                        None | Some('\n' | '\r') => {
                            self.line_start = at;
                            self.phase = Phase::Subtitle(SubtitleProbe::new());
                        }
                        Some(';') => self.base(Part::Tilde, rules::TILDE, parser.checkpoint()),
                        _ => self.phase = Phase::Done(false),
                    }
                }
                Phase::Subtitle(mut probe) => {
                    let view = crate::document::parser::Cursor::for_range_with_context(
                        parser.source(),
                        TextRange::new(self.line_start, parser.cursor().end()),
                        parser.cursor().context_end(),
                    );
                    match probe.advance(view.context_view(), final_input, allowance) {
                        ProbeProgress::Complete(true) => self.phase = Phase::Done(true),
                        ProbeProgress::Complete(false) => {
                            self.phase = Phase::Fence(FenceProbe::new())
                        }
                        ProbeProgress::NeedInput => {
                            self.phase = Phase::Subtitle(probe);
                            return BoundaryProgress::NeedInput;
                        }
                        ProbeProgress::NeedsProcessing => {
                            self.phase = Phase::Subtitle(probe);
                            return BoundaryProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Fence(mut probe) => {
                    let view = crate::document::parser::Cursor::for_range_with_context(
                        parser.source(),
                        TextRange::new(self.line_start, parser.cursor().end()),
                        parser.cursor().context_end(),
                    );
                    match probe.advance(view.context_view(), final_input, allowance) {
                        ProbeProgress::Complete(Some(_)) => self.phase = Phase::Done(true),
                        ProbeProgress::Complete(None) => {
                            let specification = crate::document::parser::canonical::document_grammar::DOCUMENT_RULES
                                .iter().find(|spec| spec.rule == rules::NOT_MECH_CODE)
                                .expect("canonical Mech/document boundary");
                            self.phase = Phase::Document(Box::new(
                                crate::document::parser::canonical::document::continuation::Continuation::for_rule(specification)
                            ), parser.checkpoint());
                        }
                        ProbeProgress::NeedInput => {
                            self.phase = Phase::Fence(probe);
                            return BoundaryProgress::NeedInput;
                        }
                        ProbeProgress::NeedsProcessing => {
                            self.phase = Phase::Fence(probe);
                            return BoundaryProgress::NeedsProcessing;
                        }
                    }
                }
                Phase::Variable(mut child, checkpoint) => {
                    match child.advance(parser, final_input, allowance) {
                        Progress::Complete(Attempt::Matched) => {
                            self.base(Part::Space, rules::SPACE_TAB0, checkpoint)
                        }
                        Progress::Complete(_) => self.done(parser, checkpoint, false),
                        Progress::NeedInput => {
                            self.phase = Phase::Variable(child, checkpoint);
                            return BoundaryProgress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.phase = Phase::Variable(child, checkpoint);
                            return BoundaryProgress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.done(parser, checkpoint, false);
                            return BoundaryProgress::Limited;
                        }
                    }
                }
                Phase::Document(mut child, checkpoint) => {
                    // Reuse the enclosing document grammar's attachment rule.
                    // Disable delimited recovery while this rule is lookahead.
                    let outer_mech = parser.state.document_mech;
                    parser.state.document_mech = false;
                    let outer_recovery = parser.replace_consuming_recovery(false);
                    let progress = child.advance(parser, final_input, allowance);
                    parser.replace_consuming_recovery(outer_recovery);
                    parser.state.document_mech = outer_mech;
                    use crate::document::parser::canonical::document::continuation::Progress as DocumentProgress;
                    match progress {
                        DocumentProgress::Complete(Attempt::Matched) => {
                            self.done(parser, checkpoint, true)
                        }
                        DocumentProgress::Complete(_) => {
                            parser.rewind(checkpoint);
                            self.base(Part::Tilde, rules::TILDE, checkpoint);
                        }
                        DocumentProgress::NeedInput => {
                            self.phase = Phase::Document(child, checkpoint);
                            return BoundaryProgress::NeedInput;
                        }
                        DocumentProgress::NeedsProcessing => {
                            self.phase = Phase::Document(child, checkpoint);
                            return BoundaryProgress::NeedsProcessing;
                        }
                        DocumentProgress::Limited => {
                            self.done(parser, checkpoint, false);
                            return BoundaryProgress::Limited;
                        }
                    }
                }
                Phase::Assignment(checkpoint) => {
                    if *allowance == 0 {
                        self.phase = Phase::Assignment(checkpoint);
                        return BoundaryProgress::NeedsProcessing;
                    }
                    *allowance -= 1;
                    if parser.cursor().byte() == Some(b'=') {
                        match parser.cursor().byte_at(1) {
                            None if !final_input => {
                                self.phase = Phase::Assignment(checkpoint);
                                return BoundaryProgress::NeedInput;
                            }
                            Some(b'=') => {
                                self.done(parser, checkpoint, false);
                                continue;
                            }
                            _ => {}
                        }
                    }
                    self.base(Part::Assign, rules::ASSIGN_OPERATOR, checkpoint);
                }
                Phase::Base(part, mut child, checkpoint) => {
                    match child.advance(parser, final_input, allowance) {
                        base::continuation::Progress::Complete(matched) => match part {
                            Part::Tilde => {
                                self.phase = Phase::Variable(
                                    Box::new(Continuation::new(rules::VAR)),
                                    checkpoint,
                                )
                            }
                            Part::Space => {
                                self.base(Part::Define, rules::DEFINE_OPERATOR, checkpoint)
                            }
                            Part::Define if matched => self.done(parser, checkpoint, true),
                            Part::Define => self.phase = Phase::Assignment(checkpoint),
                            Part::Assign => self.done(parser, checkpoint, matched),
                        },
                        base::continuation::Progress::NeedInput => {
                            self.phase = Phase::Base(part, child, checkpoint);
                            return BoundaryProgress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.phase = Phase::Base(part, child, checkpoint);
                            return BoundaryProgress::NeedsProcessing;
                        }
                        base::continuation::Progress::Limited => {
                            self.done(parser, checkpoint, false);
                            return BoundaryProgress::Limited;
                        }
                    }
                }
            }
        }
    }
}
