//! Match arms retain guard candidates and required output recovery.
use super::*;
pub(super) enum Phase {
    Enter,
    Open(Marker),
    Finish(Marker),
    Pattern,
    PatternRecovered,
    Guard(bool, ParserCheckpoint),
    GuardSpace(bool, ParserCheckpoint),
    GuardValue(bool, ParserCheckpoint),
    Output(bool),
    RecoveredOutput,
    RetriedOutput,
    Value(bool),
    MissingValue,
    Suffix(bool, ParserCheckpoint),
    Separator(bool, ParserCheckpoint),
}
impl Continuation {
    fn arm(&mut self, phase: Phase) {
        self.push(Frame::MatchArm(Box::new(phase)));
    }
    fn arm_guard(&mut self, parser: &mut Parser<'_>, committed: bool) {
        self.arm(Phase::Guard(committed, parser.checkpoint()));
        self.base(rules::LIST_SEPARATOR);
    }
    fn arm_output(&mut self, committed: bool) {
        self.arm(Phase::Output(committed));
        self.base(rules::OUTPUT_OPERATOR);
    }
    fn arm_result(&mut self, committed: bool) {
        self.result = if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        };
    }
    #[inline(never)]
    pub(super) fn arm_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter => {
                self.transaction(parser, rules::MATCH_ARM);
                let node = parser.start();
                self.arm(Phase::Open(node));
                self.push(Frame::Primitive(Box::new(primitives::Continuation::new(
                    rules::GUARD_OPERATOR,
                ))));
            }
            Phase::Open(node) => {
                if self.result != Attempt::Matched {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                    self.result = Attempt::NoMatch;
                } else {
                    self.arm(Phase::Finish(node));
                    if parser.push_nesting() {
                        self.push(Frame::PopNesting);
                        self.arm(Phase::Pattern);
                        self.push(Frame::Call(rules::PATTERN));
                    } else {
                        self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                    }
                }
            }
            Phase::Finish(node) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    node.complete(parser, SyntaxKind::MatchArm);
                }
            }
            Phase::Pattern => {
                if self.result == Attempt::NoMatch {
                    self.arm(Phase::PatternRecovered);
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::MATCH_ARM,
                        "syntax/missing-match-arm-pattern",
                        "missing pattern after match arm guard",
                        "pattern",
                        &[],
                        &["=>", "⇒"],
                        None,
                    ))));
                } else {
                    self.arm_guard(parser, self.result == Attempt::Committed);
                }
            }
            Phase::PatternRecovered => self.arm_guard(parser, true),
            Phase::Guard(committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.arm(Phase::GuardSpace(committed, checkpoint));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.arm_output(committed);
                }
            }
            Phase::GuardSpace(committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.arm(Phase::GuardValue(committed, checkpoint));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.arm_output(committed);
                }
            }
            Phase::GuardValue(committed, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                }
                self.arm_output(committed || self.result == Attempt::Committed);
            }
            Phase::Output(committed) => {
                if self.result == Attempt::Matched {
                    self.arm(Phase::Value(committed));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.arm(Phase::RecoveredOutput);
                    self.push(Frame::Required(Box::new(Required::token(
                        rules::MATCH_ARM,
                        "syntax/missing-match-arm-output-operator",
                        "missing output operator after match arm pattern",
                        SyntaxKind::OutputOperator,
                        "=>",
                        &["=>", "⇒"],
                    ))));
                }
            }
            Phase::RecoveredOutput => {
                self.arm(Phase::RetriedOutput);
                self.base(rules::OUTPUT_OPERATOR);
            }
            Phase::RetriedOutput => {
                if self.result == Attempt::Matched {
                    self.arm(Phase::Value(true));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.result = Attempt::Committed;
                }
            }
            Phase::Value(committed) => {
                if self.result == Attempt::NoMatch {
                    self.arm(Phase::MissingValue);
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::MATCH_ARM,
                        "syntax/missing-match-arm-value",
                        "missing expression after match arm output operator",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.arm(Phase::Suffix(
                        committed || self.result == Attempt::Committed,
                        parser.checkpoint(),
                    ));
                    self.base(rules::WHITESPACE1);
                }
            }
            Phase::MissingValue => self.result = Attempt::Committed,
            Phase::Suffix(committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.arm_result(committed);
                } else {
                    self.arm(Phase::Separator(committed, checkpoint));
                    self.push(Frame::Primitive(Box::new(primitives::Continuation::new(
                        rules::STATEMENT_SEPARATOR,
                    ))));
                }
            }
            Phase::Separator(committed, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                }
                self.arm_result(committed);
            }
        }
    }
}
