//! Parameterized canonical TAG recognition shares the retained literal scanner.
use super::continuation::Progress;
use super::*;
use crate::document::TextSize;
use crate::document::parser::{
    ParserCheckpoint,
    literal_scan::{LiteralProgress, LiteralScan},
};
enum Phase<'a> {
    Start,
    Scan(LiteralScan<'a>),
    Consume(Option<TextSize>),
    Exit,
    Done,
}
pub(crate) struct ExactTag<'a> {
    literal: &'a str,
    kind: SyntaxKind,
    checkpoint: Option<ParserCheckpoint>,
    phase: Phase<'a>,
    matched: bool,
    pub work: u64,
}
impl<'a> ExactTag<'a> {
    pub fn new(literal: &'a str, kind: SyntaxKind) -> Self {
        Self {
            literal,
            kind,
            checkpoint: None,
            phase: Phase::Start,
            matched: false,
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
            if matches!(self.phase, Phase::Done) {
                return Progress::Complete(self.matched);
            }
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let phase = core::mem::replace(&mut self.phase, Phase::Done);
            if !matches!(phase, Phase::Scan(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match phase {
                Phase::Start => {
                    if self.literal.is_empty() || !self.kind.is_token() {
                        continue;
                    }
                    self.checkpoint = Some(parser.checkpoint());
                    parser.state.rules.push_canonical(rules::TAG);
                    if let Some(scan) = LiteralScan::new(
                        self.literal,
                        parser.offset(),
                        final_input.then_some(parser.cursor().context_end()),
                    ) {
                        self.phase = Phase::Scan(scan);
                    } else {
                        self.phase = Phase::Exit;
                    }
                }
                Phase::Scan(mut scan) => {
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
                        LiteralProgress::Complete(end) => self.phase = Phase::Consume(end),
                        LiteralProgress::NeedInput => {
                            self.phase = Phase::Scan(scan);
                            return Progress::NeedInput;
                        }
                        LiteralProgress::NeedsProcessing => {
                            self.phase = Phase::Scan(scan);
                            return Progress::NeedsProcessing;
                        }
                        LiteralProgress::InvalidSource => panic!("canonical TAG source changed"),
                    }
                }
                Phase::Consume(end) => {
                    if let Some(end) = end {
                        self.matched = parser
                            .bump_bytes_token((end - parser.offset()).0, self.kind)
                            .is_some();
                    }
                    self.phase = Phase::Exit;
                }
                Phase::Exit => {
                    let checkpoint = self.checkpoint.expect("TAG transaction");
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if !self.matched {
                        parser.rewind(checkpoint);
                    }
                }
                Phase::Done => unreachable!(),
            }
        }
    }
}
