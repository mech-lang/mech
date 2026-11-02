//! FSM instance, pipe, and transition owners share call lists and recovery.
use super::*;
pub(super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::FSM_PIPE
            | rules::FSM_INSTANCE
            | rules::FSM_STATE_TRANSITION
            | rules::FSM_ASYNC_TRANSITION
            | rules::FSM_OUTPUT
    )
}
#[derive(Clone, Copy)]
pub(super) struct Transition {
    rule: RuleId,
    kind: SyntaxKind,
}
pub(super) enum Phase {
    Enter(RuleId),
    PipeFirst(Marker),
    PipeLoop(Marker, bool),
    PipeNext(Marker, bool, TextSize),
    Stage(usize),
    StageNext(usize),
    InstanceOpen(Marker),
    InstanceName(Marker),
    InstanceRecovered(Marker),
    InstanceArgs(Marker, bool),
    TransitionOpen(Marker, Transition),
    TransitionValue(Marker, Transition),
    TransitionRecovered(Marker, Transition),
}
impl Continuation {
    fn fsm(&mut self, phase: Phase) {
        self.push(Frame::Fsm(Box::new(phase)));
    }
    #[inline(never)]
    pub(super) fn fsm_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter(rule) => {
                self.transaction(parser, rule);
                let node = parser.start();
                match rule {
                    rules::FSM_PIPE => {
                        self.fsm(Phase::PipeFirst(node));
                        self.push(Frame::Call(rules::FSM_INSTANCE));
                    }
                    rules::FSM_INSTANCE => {
                        self.fsm(Phase::InstanceOpen(node));
                        self.base(rules::HASHTAG);
                    }
                    _ => {
                        let (operator, kind) = match rule {
                            rules::FSM_STATE_TRANSITION => {
                                (rules::TRANSITION_OPERATOR, SyntaxKind::FsmStateTransition)
                            }
                            rules::FSM_ASYNC_TRANSITION => (
                                rules::ASYNC_TRANSITION_OPERATOR,
                                SyntaxKind::FsmAsyncTransition,
                            ),
                            rules::FSM_OUTPUT => (rules::OUTPUT_OPERATOR, SyntaxKind::FsmOutput),
                            _ => unreachable!("canonical FSM transition"),
                        };
                        self.fsm(Phase::TransitionOpen(node, Transition { rule, kind }));
                        self.base(operator);
                    }
                }
            }
            Phase::PipeFirst(node) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.fsm(Phase::PipeLoop(node, self.result == Attempt::Committed));
                }
            }
            Phase::PipeLoop(node, committed) => {
                self.fsm(Phase::PipeNext(node, committed, parser.offset()));
                self.fsm(Phase::Stage(0));
            }
            Phase::PipeNext(node, committed, before) => {
                if self.result != Attempt::NoMatch && parser.offset() > before {
                    self.fsm(Phase::PipeLoop(
                        node,
                        committed || self.result == Attempt::Committed,
                    ));
                } else {
                    node.complete(parser, SyntaxKind::FsmPipe);
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
            }
            Phase::Stage(index) => {
                self.fsm(Phase::StageNext(index));
                self.push(Frame::Call(
                    [
                        rules::FSM_STATE_TRANSITION,
                        rules::FSM_ASYNC_TRANSITION,
                        rules::FSM_OUTPUT,
                    ][index],
                ));
            }
            Phase::StageNext(index) => {
                if self.result == Attempt::NoMatch && index < 2 {
                    self.fsm(Phase::Stage(index + 1));
                }
            }
            Phase::InstanceOpen(node) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.fsm(Phase::InstanceName(node));
                    self.base(rules::IDENTIFIER);
                }
            }
            Phase::InstanceName(node) => {
                if self.result == Attempt::NoMatch {
                    self.fsm(Phase::InstanceRecovered(node));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::FSM_INSTANCE,
                        "syntax/missing-fsm-name",
                        "missing state-machine name after hash sign",
                        "identifier",
                        &[],
                        &["(", "->", "~>", "=>", "→", "⇒"],
                        None,
                    ))));
                } else {
                    self.fsm(Phase::InstanceArgs(node, false));
                    self.push(Frame::Call(rules::FSM_ARGS));
                }
            }
            Phase::InstanceRecovered(node) => {
                self.fsm(Phase::InstanceArgs(node, true));
                self.push(Frame::Call(rules::FSM_ARGS));
            }
            Phase::InstanceArgs(node, committed) => {
                node.complete(parser, SyntaxKind::FsmInstance);
                self.result = if committed || self.result == Attempt::Committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                };
            }
            Phase::TransitionOpen(node, spec) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.fsm(Phase::TransitionValue(node, spec));
                    if parser.push_nesting() {
                        self.push(Frame::PopNesting);
                        self.push(Frame::Call(rules::FSM_VALUE));
                    } else {
                        self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                    }
                }
            }
            Phase::TransitionValue(node, spec) => {
                if self.result == Attempt::NoMatch {
                    self.fsm(Phase::TransitionRecovered(node, spec));
                    self.push(Frame::Required(Box::new(Required::new(
                        spec.rule,
                        "syntax/missing-fsm-transition-value",
                        "missing state-machine value after transition operator",
                        "fsm-value",
                        &[],
                        &["->", "~>", "=>", "→", "⇒"],
                        None,
                    ))));
                } else {
                    node.complete(parser, spec.kind);
                }
            }
            Phase::TransitionRecovered(node, spec) => {
                node.complete(parser, spec.kind);
                self.result = Attempt::Committed;
            }
        }
    }
}
