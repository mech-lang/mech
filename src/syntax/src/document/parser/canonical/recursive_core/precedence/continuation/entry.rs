//! Binding and mapping candidates retain value boundaries and optional reuse.
use super::super::super::structures::BindingCandidate;
use super::*;
use crate::document::parser::CleanSubtree;
pub(super) struct Entry {
    node: Marker,
    binding: bool,
    committed: bool,
    value_start: Option<ParserCheckpoint>,
    value_end: Option<ParserCheckpoint>,
    cached: Option<CleanSubtree>,
}
impl Entry {
    fn kind(&self) -> SyntaxKind {
        if self.binding {
            SyntaxKind::RecordBinding
        } else {
            SyntaxKind::MapEntry
        }
    }
    fn rule(&self) -> RuleId {
        if self.binding {
            rules::BINDING
        } else {
            rules::MAPPING
        }
    }
}
pub(super) enum Phase {
    Enter(bool, Option<CleanSubtree>),
    Space(Entry),
    Name(Entry),
    Annotation(Entry),
    Key(Entry),
    Colon(Entry, usize),
    Value(Entry),
    Recovered(Entry),
    Suffix(Entry, usize),
}
impl Continuation {
    fn entry(&mut self, phase: Phase) {
        self.push(Frame::Entry(Box::new(phase)));
    }
    fn entry_finish(&mut self, parser: &mut Parser<'_>, entry: Entry, accept: bool) {
        if !accept {
            if !parser.state.resource_finalizing {
                entry.node.abandon(parser);
            }
            self.result = Attempt::NoMatch;
            self.binding_candidate = None;
        } else {
            let kind = entry.kind();
            entry.node.complete(parser, kind);
            self.result = if entry.committed || parser.is_halted() {
                Attempt::Committed
            } else {
                Attempt::Matched
            };
            if entry.binding {
                self.binding_candidate = if self.result == Attempt::Matched {
                    Some(BindingCandidate {
                        value_start: entry.value_start.expect("binding value start"),
                        value_end: entry.value_end.expect("binding value end"),
                    })
                } else {
                    None
                };
            }
        }
    }
    fn entry_suffix(&mut self, parser: &mut Parser<'_>, mut entry: Entry) {
        entry.value_end = Some(parser.checkpoint());
        self.entry(Phase::Suffix(entry, 0));
        self.base(rules::WHITESPACE0);
    }
    #[inline(never)]
    pub(super) fn entry_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter(binding, cached) => {
                self.transaction(
                    parser,
                    if binding {
                        rules::BINDING
                    } else {
                        rules::MAPPING
                    },
                );
                self.binding_candidate = None;
                let entry = Entry {
                    node: parser.start(),
                    binding,
                    committed: false,
                    value_start: None,
                    value_end: None,
                    cached,
                };
                self.entry(Phase::Space(entry));
                self.base(rules::WHITESPACE0);
            }
            Phase::Space(entry) => {
                if self.result != Attempt::Matched {
                    self.entry_finish(parser, entry, false);
                } else if entry.binding {
                    self.entry(Phase::Name(entry));
                    self.base(rules::IDENTIFIER);
                } else {
                    self.entry(Phase::Key(entry));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::Name(entry) => {
                if self.result != Attempt::Matched {
                    self.entry_finish(parser, entry, false);
                } else {
                    self.entry(Phase::Annotation(entry));
                    self.push(Frame::Call(rules::KIND_ANNOTATION));
                }
            }
            Phase::Annotation(mut entry) => {
                entry.committed = self.result == Attempt::Committed;
                if parser.is_halted() {
                    self.entry_finish(parser, entry, true);
                } else {
                    self.entry(Phase::Colon(entry, 0));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::Key(mut entry) => {
                if self.result == Attempt::NoMatch {
                    self.entry_finish(parser, entry, false);
                } else {
                    entry.committed = self.result == Attempt::Committed;
                    self.entry(Phase::Colon(entry, 0));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::Colon(mut entry, index) => {
                if self.result != Attempt::Matched {
                    if entry.binding || (!entry.committed && !parser.is_halted()) {
                        self.entry_finish(parser, entry, false);
                    } else {
                        self.entry_suffix(parser, entry);
                    }
                } else if index < 2 {
                    self.entry(Phase::Colon(entry, index + 1));
                    self.base(if index == 0 {
                        rules::COLON
                    } else {
                        rules::WHITESPACE0
                    });
                } else {
                    entry.value_start = Some(parser.checkpoint());
                    let reused = entry
                        .cached
                        .as_ref()
                        .is_some_and(|cached| parser.reuse_clean_subtree(cached));
                    self.entry(Phase::Value(entry));
                    if reused {
                        self.result = Attempt::Matched;
                    } else if parser.is_halted() {
                        self.result = Attempt::Committed;
                    } else {
                        self.push(Frame::Call(rules::EXPRESSION));
                    }
                }
            }
            Phase::Value(mut entry) => {
                if self.result == Attempt::NoMatch {
                    let rule = entry.rule();
                    let (code, msg) = if entry.binding {
                        (
                            "syntax/missing-binding-value",
                            "missing value after binding colon",
                        )
                    } else {
                        (
                            "syntax/missing-mapping-value",
                            "missing value after mapping colon",
                        )
                    };
                    self.entry(Phase::Recovered(entry));
                    self.push(Frame::Required(Box::new(Required::new(
                        rule,
                        code,
                        msg,
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    entry.committed |= self.result == Attempt::Committed;
                    self.entry_suffix(parser, entry);
                }
            }
            Phase::Recovered(mut entry) => {
                entry.committed = true;
                self.entry_suffix(parser, entry);
            }
            Phase::Suffix(entry, index) => {
                if index != 1 && self.result != Attempt::Matched && !parser.is_halted() {
                    self.entry_finish(parser, entry, false);
                } else if index < 2 {
                    self.entry(Phase::Suffix(entry, index + 1));
                    self.base(if index == 0 {
                        rules::COMMA
                    } else {
                        rules::WHITESPACE0
                    });
                } else {
                    self.entry_finish(parser, entry, true);
                }
            }
        }
    }
}
