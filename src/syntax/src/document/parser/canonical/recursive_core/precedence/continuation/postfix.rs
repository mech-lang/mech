//! Call and subscript parents share the canonical child owners and recovery.
use super::super::super::child_result;
use super::*;
#[derive(Clone, Copy)]
pub(super) enum Value {
    Call,
    Subscript,
}
#[derive(Clone, Copy)]
pub(super) struct List {
    rule: RuleId,
    kind: SyntaxKind,
    open: RuleId,
    close: RuleId,
    close_kind: SyntaxKind,
    close_text: &'static str,
    value: Value,
    empty: bool,
    code: &'static str,
    first: &'static str,
    next: &'static str,
    expected: &'static str,
}
pub(super) fn argument_list(rule: RuleId, kind: SyntaxKind) -> List {
    List {
        rule,
        kind,
        open: rules::LEFT_PARENTHESIS,
        close: rules::RIGHT_PARENTHESIS,
        close_kind: SyntaxKind::RightParen,
        close_text: ")",
        value: Value::Call,
        empty: true,
        code: "syntax/missing-call-argument",
        first: "missing call argument",
        next: "missing call argument after separator",
        expected: "call-arg",
    }
}
fn subscript_list(rule: RuleId) -> List {
    let bracket = rule == rules::BRACKET_SUBSCRIPT;
    List {
        rule,
        kind: if bracket {
            SyntaxKind::BracketSubscript
        } else {
            SyntaxKind::BraceSubscript
        },
        open: if bracket {
            rules::LEFT_BRACKET
        } else {
            rules::LEFT_BRACE
        },
        close: if bracket {
            rules::RIGHT_BRACKET
        } else {
            rules::RIGHT_BRACE
        },
        close_kind: if bracket {
            SyntaxKind::RightBracket
        } else {
            SyntaxKind::RightBrace
        },
        close_text: if bracket { "]" } else { "}" },
        value: Value::Subscript,
        empty: false,
        code: "syntax/missing-subscript-value",
        first: "missing subscript value",
        next: "missing subscript value after separator",
        expected: "formula-subscript",
    }
}
pub(super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::ARGUMENT_LIST
            | rules::FSM_ARGS
            | rules::FUNCTION_CALL
            | rules::CALL_ARG_WITH_BINDING
            | rules::CALL_ARG
            | rules::SUBSCRIPT
            | rules::SLICE
            | rules::BRACKET_SUBSCRIPT
            | rules::BRACE_SUBSCRIPT
            | rules::FORMULA_SUBSCRIPT
            | rules::RANGE_SUBSCRIPT
    )
}
pub(super) enum Phase {
    Enter(RuleId),
    Node(Marker, SyntaxKind),
    Finish(Marker, SyntaxKind),
    CallStem(Marker),
    BindingPrefix(Marker, usize),
    BindingValue(Marker),
    BindingRecovered(Marker),
    Value(Value, usize),
    ValueNext(Value, usize),
    List(List),
    ListOpen(Marker, List),
    Empty(Marker, List),
    First(Marker, List),
    Recovered(Marker, List),
    Loop(Marker, List, bool),
    Separator(Marker, List, bool, TextSize),
    Next(Marker, List, bool, TextSize),
    Close(List, bool),
    SlicePath(Marker),
    SliceStem(Marker),
    SubscriptFirst(Marker),
    SubscriptLoop(Marker, bool),
    SubscriptNext(Marker, bool, TextSize),
    Item(usize),
    ItemNext(usize),
}
impl Continuation {
    fn postfix(&mut self, phase: Phase) {
        self.push(Frame::Postfix(Box::new(phase)));
    }
    fn value(&mut self, value: Value, index: usize) {
        self.postfix(Phase::ValueNext(value, index));
        match (value, index) {
            (Value::Call, 0) => self.push(Frame::Call(rules::CALL_ARG_WITH_BINDING)),
            (Value::Call, 1) => self.push(Frame::Call(rules::CALL_ARG)),
            (Value::Subscript, 0) => self.push(Frame::Primitive(Box::new(
                primitives::Continuation::new(rules::SELECT_ALL),
            ))),
            (Value::Subscript, 1) => self.push(Frame::Call(rules::RANGE_SUBSCRIPT)),
            (Value::Subscript, 2) => self.push(Frame::Call(rules::FORMULA_SUBSCRIPT)),
            _ => unreachable!("postfix value alternative"),
        }
    }
    fn list_missing(&mut self, node: Marker, spec: List, first: bool) {
        self.postfix(Phase::Recovered(node, spec));
        self.push(Frame::Required(Box::new(Required::new(
            spec.rule,
            spec.code,
            if first { spec.first } else { spec.next },
            spec.expected,
            &[],
            &[],
            None,
        ))));
    }
    #[inline(never)]
    pub(super) fn postfix_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter(rule) => match rule {
                rules::ARGUMENT_LIST => {
                    self.postfix(Phase::List(argument_list(rule, SyntaxKind::ArgumentList)))
                }
                rules::FSM_ARGS => {
                    self.postfix(Phase::List(argument_list(rule, SyntaxKind::FsmArguments)))
                }
                rules::BRACKET_SUBSCRIPT | rules::BRACE_SUBSCRIPT => {
                    self.postfix(Phase::List(subscript_list(rule)))
                }
                rules::FUNCTION_CALL => {
                    self.transaction(parser, rule);
                    let node = parser.start();
                    self.postfix(Phase::CallStem(node));
                    self.base(rules::IDENTIFIER);
                }
                rules::CALL_ARG_WITH_BINDING => {
                    self.transaction(parser, rule);
                    let node = parser.start();
                    self.postfix(Phase::BindingPrefix(node, 0));
                    self.base(rules::IDENTIFIER);
                }
                rules::CALL_ARG | rules::FORMULA_SUBSCRIPT | rules::RANGE_SUBSCRIPT => {
                    self.transaction(parser, rule);
                    let node = parser.start();
                    let kind = match rule {
                        rules::CALL_ARG => SyntaxKind::CallArgument,
                        rules::FORMULA_SUBSCRIPT => SyntaxKind::FormulaSubscript,
                        _ => SyntaxKind::RangeSubscript,
                    };
                    self.postfix(Phase::Node(node, kind));
                    if rule == rules::CALL_ARG {
                        self.push(Frame::Call(rules::EXPRESSION));
                    } else {
                        self.push(Frame::Call(if rule == rules::FORMULA_SUBSCRIPT {
                            rules::FORMULA
                        } else {
                            rules::RANGE_EXPRESSION
                        }));
                    }
                }
                rules::SLICE => {
                    self.transaction(parser, rule);
                    let node = parser.start();
                    self.postfix(Phase::SlicePath(node));
                    self.push(Frame::LeafPath(Box::new(paths::Continuation::new(
                        rules::PREFIXED_CONTEXT_PATH,
                    ))));
                }
                rules::SUBSCRIPT => {
                    self.transaction(parser, rule);
                    let node = parser.start();
                    self.postfix(Phase::SubscriptFirst(node));
                    self.postfix(Phase::Item(0));
                }
                _ => unreachable!("postfix owner"),
            },
            Phase::Node(node, kind) => {
                if let Some(result) = child_result(parser, node, kind, self.result) {
                    self.result = result;
                } else {
                    node.complete(parser, kind);
                    self.result = Attempt::Matched;
                }
            }
            Phase::Finish(node, kind) => self.result = finish(node, parser, kind, self.result),
            Phase::CallStem(node) => {
                if self.result == Attempt::NoMatch {
                    // The transaction drops rejected provisional markers after a
                    // child has finalized the hard-limit remainder.
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.postfix(Phase::Node(node, SyntaxKind::FunctionCall));
                    self.push(Frame::Call(rules::ARGUMENT_LIST));
                }
            }
            Phase::BindingPrefix(node, index) => {
                if self.result == Attempt::NoMatch {
                    // The transaction drops rejected provisional markers after a
                    // child has finalized the hard-limit remainder.
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else if index < 3 {
                    self.postfix(Phase::BindingPrefix(node, index + 1));
                    self.base(
                        [
                            rules::IDENTIFIER,
                            rules::WHITESPACE0,
                            rules::COLON,
                            rules::WHITESPACE0,
                        ][index + 1],
                    );
                } else {
                    self.postfix(Phase::BindingValue(node));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::BindingValue(node) => {
                if self.result == Attempt::NoMatch {
                    self.postfix(Phase::BindingRecovered(node));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::CALL_ARG_WITH_BINDING,
                        "syntax/missing-bound-call-argument-value",
                        "missing value for bound call argument",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    node.complete(parser, SyntaxKind::BoundCallArgument);
                }
            }
            Phase::BindingRecovered(node) => {
                node.complete(parser, SyntaxKind::BoundCallArgument);
                self.result = Attempt::Committed;
            }
            Phase::Value(value, index) => self.value(value, index),
            Phase::ValueNext(value, index) => {
                if self.result == Attempt::NoMatch
                    && index < if matches!(value, Value::Call) { 1 } else { 2 }
                {
                    self.postfix(Phase::Value(value, index + 1));
                }
            }
            Phase::List(spec) => {
                self.transaction(parser, spec.rule);
                let node = parser.start();
                self.postfix(Phase::ListOpen(node, spec));
                self.base(spec.open);
            }
            Phase::ListOpen(node, spec) => {
                if self.result == Attempt::NoMatch {
                    // The transaction drops rejected provisional markers after a
                    // child has finalized the hard-limit remainder.
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.postfix(Phase::Finish(node, spec.kind));
                    if parser.push_nesting() {
                        self.push(Frame::PopNesting);
                        if spec.empty {
                            self.postfix(Phase::Empty(node, spec));
                            self.base(spec.close);
                        } else {
                            self.postfix(Phase::First(node, spec));
                            self.postfix(Phase::Value(spec.value, 0));
                        }
                    } else {
                        self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                    }
                }
            }
            Phase::Empty(node, spec) => {
                if self.result == Attempt::NoMatch {
                    self.postfix(Phase::First(node, spec));
                    self.postfix(Phase::Value(spec.value, 0));
                }
            }
            Phase::First(node, spec) => {
                if self.result == Attempt::NoMatch {
                    self.list_missing(node, spec, true);
                } else {
                    self.postfix(Phase::Loop(node, spec, self.result == Attempt::Committed));
                }
            }
            Phase::Recovered(node, spec) => self.postfix(Phase::Loop(node, spec, true)),
            Phase::Loop(node, spec, committed) => {
                if parser.is_halted() {
                    self.postfix(Phase::Close(spec, committed));
                    self.base(spec.close);
                } else {
                    self.postfix(Phase::Separator(node, spec, committed, parser.offset()));
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::Separator(node, spec, committed, before) => {
                if self.result == Attempt::Matched {
                    self.postfix(Phase::Next(node, spec, committed, before));
                    self.postfix(Phase::Value(spec.value, 0));
                } else {
                    self.postfix(Phase::Close(spec, committed));
                    self.base(spec.close);
                }
            }
            Phase::Next(node, spec, committed, before) => match self.result {
                Attempt::Matched if parser.offset() > before => {
                    self.postfix(Phase::Loop(node, spec, committed))
                }
                Attempt::Committed => self.postfix(Phase::Loop(node, spec, true)),
                _ => self.list_missing(node, spec, false),
            },
            Phase::Close(spec, committed) => {
                if self.result == Attempt::Matched {
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else {
                    self.push(Frame::Closer(Box::new(Closer::new(
                        spec.rule,
                        spec.close,
                        spec.close_kind,
                        spec.close_text,
                    ))));
                }
            }
            Phase::SlicePath(node) => {
                self.postfix(Phase::SliceStem(node));
                if self.result == Attempt::NoMatch {
                    self.base(rules::IDENTIFIER);
                }
            }
            Phase::SliceStem(node) => {
                if self.result == Attempt::NoMatch {
                    // The transaction drops rejected provisional markers after a
                    // child has finalized the hard-limit remainder.
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.postfix(Phase::Node(node, SyntaxKind::Slice));
                    self.push(Frame::Call(rules::SUBSCRIPT));
                }
            }
            Phase::SubscriptFirst(node) => {
                if self.result == Attempt::NoMatch {
                    // The transaction drops rejected provisional markers after a
                    // child has finalized the hard-limit remainder.
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.postfix(Phase::SubscriptLoop(
                        node,
                        self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::SubscriptLoop(node, committed) => {
                if parser.is_halted() {
                    node.complete(parser, SyntaxKind::SubscriptList);
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else {
                    self.postfix(Phase::SubscriptNext(node, committed, parser.offset()));
                    self.postfix(Phase::Item(0));
                }
            }
            Phase::SubscriptNext(node, mut committed, before) => {
                committed |= self.result == Attempt::Committed;
                if self.result == Attempt::NoMatch || parser.offset() <= before {
                    node.complete(parser, SyntaxKind::SubscriptList);
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else {
                    self.postfix(Phase::SubscriptLoop(node, committed));
                }
            }
            Phase::Item(index) => {
                self.postfix(Phase::ItemNext(index));
                let rule = [
                    rules::SWIZZLE_SUBSCRIPT,
                    rules::DOT_SUBSCRIPT,
                    rules::DOT_SUBSCRIPT_INT,
                    rules::BRACKET_SUBSCRIPT,
                    rules::BRACE_SUBSCRIPT,
                ][index];
                if index < 3 {
                    self.push(Frame::Primitive(Box::new(primitives::Continuation::new(
                        rule,
                    ))));
                } else {
                    self.push(Frame::Call(rule));
                }
            }
            Phase::ItemNext(index) => {
                if self.result == Attempt::NoMatch && index < 4 {
                    self.postfix(Phase::Item(index + 1));
                }
            }
        }
    }
}
