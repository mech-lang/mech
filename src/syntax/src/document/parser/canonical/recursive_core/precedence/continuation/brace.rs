//! Shared brace selection retains record, map, set, and comprehension candidates.
use super::super::super::{finish_provisional_marker, structures::BindingCandidate};
use super::*;
use crate::document::parser::CleanSubtree;
use alloc::string::String;
#[derive(Clone, Copy)]
pub(super) struct Owner {
    structure: Marker,
    map: Marker,
    set: Marker,
    comprehension: Marker,
    record: Marker,
    expression: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Head {
    owner: Owner,
    entry: Marker,
    committed: bool,
    after: ParserCheckpoint,
}
pub(super) struct Record {
    owner: Owner,
    interior: ParserCheckpoint,
    bindings: Vec<BindingCandidate>,
    committed: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Map {
    owner: Owner,
    committed: bool,
    strict: bool,
}
pub(super) enum Phase {
    Enter(bool),
    Choice(bool, usize),
    Chosen(bool, usize, Marker, ParserCheckpoint),
    General(bool),
    Exit(ParserCheckpoint),
    Open(Owner),
    Limited(Owner),
    Space(Owner),
    Binding(Owner, ParserCheckpoint),
    Head(Owner, Marker),
    MissingHead(Owner, Marker),
    BarSpace(Head),
    Bar(Head),
    Qualifiers(Head),
    ColonStart(Head),
    Colon(Head, usize),
    Value(Head),
    ValueRecovered(Head),
    ValueSuffix(Head, usize),
    SetStart(Head),
    SetLoop(Owner, bool),
    SetSeparator(Owner, bool, ParserCheckpoint),
    SetSpaceSeparator(Owner, bool, ParserCheckpoint),
    SetItem(Owner, bool, ParserCheckpoint),
    SetTrailing(Owner, bool),
    SetClose(Owner, bool),
    SetRecovered(Owner),
    RecordStart(Record),
    RecordLoop(Record),
    RecordBinding(Record, TextSize),
    RecordSeparator(Record, TextSize),
    RecordTrailing(Record, ParserCheckpoint),
    RecordClose(Record, ParserCheckpoint),
    RecordRejected(Record, ParserCheckpoint),
    RecordProbe(Record),
    RecordRecovered(Owner),
    Cache(Record, usize, Vec<CleanSubtree>),
    Replay(Owner, alloc::vec::IntoIter<CleanSubtree>, bool),
    ReplayItem(Owner, alloc::vec::IntoIter<CleanSubtree>, bool),
    ReplayUncached(Owner),
    MapTail(Map),
    MapTrailing(Map),
    MapClose(Map),
    MapRecovered(Map),
}
impl Continuation {
    fn brace(&mut self, phase: Phase) {
        self.push(Frame::Brace(Box::new(phase)));
    }
    fn brace_formula(&mut self, committed: bool) {
        self.expression_result(if committed {
            FactAttempt::Committed
        } else {
            FactAttempt::Matched(ExpressionForm::Formula)
        });
    }
    fn brace_map_finish(&mut self, parser: &mut Parser<'_>, map: Map) {
        map.owner.map.complete(parser, SyntaxKind::Map);
        map.owner.structure.complete(parser, SyntaxKind::Structure);
        self.brace_formula(map.committed);
    }
    fn brace_set_finish(&mut self, parser: &mut Parser<'_>, owner: Owner, committed: bool) {
        owner.set.complete(parser, SyntaxKind::Set);
        finish_provisional_marker(parser, owner.map, SyntaxKind::Map);
        owner.structure.complete(parser, SyntaxKind::Structure);
        self.brace_formula(committed);
    }
    fn brace_record_finish(&mut self, parser: &mut Parser<'_>, owner: Owner, committed: bool) {
        owner.record.complete(parser, SyntaxKind::Record);
        finish_provisional_marker(parser, owner.comprehension, SyntaxKind::SetComprehension);
        finish_provisional_marker(parser, owner.set, SyntaxKind::Set);
        finish_provisional_marker(parser, owner.map, SyntaxKind::Map);
        owner.structure.complete(parser, SyntaxKind::Structure);
        self.brace_formula(committed);
    }
    fn brace_record_recover(&mut self, owner: Owner) {
        self.brace(Phase::RecordRecovered(owner));
        self.push(Frame::Closer(Box::new(Closer::new(
            rules::RECORD,
            rules::RIGHT_BRACE,
            SyntaxKind::RightBrace,
            "}",
        ))));
    }
    fn brace_head(
        &mut self,
        parser: &mut Parser<'_>,
        owner: Owner,
        entry: Marker,
        committed: bool,
    ) {
        if parser.is_halted() {
            entry.complete(parser, SyntaxKind::MapEntry);
            owner
                .comprehension
                .complete(parser, SyntaxKind::SetComprehension);
            owner.set.complete(parser, SyntaxKind::Set);
            owner.map.complete(parser, SyntaxKind::Map);
            owner.structure.complete(parser, SyntaxKind::Structure);
            self.expression_result(FactAttempt::Committed);
        } else {
            self.brace(Phase::BarSpace(Head {
                owner,
                entry,
                committed,
                after: parser.checkpoint(),
            }));
            self.base(rules::SPACE_TAB0);
        }
    }
    fn brace_replay_start(
        &mut self,
        parser: &mut Parser<'_>,
        record: Record,
        caches: Option<Vec<CleanSubtree>>,
    ) {
        parser.rewind(record.interior);
        finish_provisional_marker(parser, record.owner.record, SyntaxKind::Record);
        finish_provisional_marker(
            parser,
            record.owner.comprehension,
            SyntaxKind::SetComprehension,
        );
        finish_provisional_marker(parser, record.owner.set, SyntaxKind::Set);
        if let Some(caches) = caches {
            self.brace(Phase::Replay(record.owner, caches.into_iter(), false));
        } else {
            self.brace(Phase::ReplayUncached(record.owner));
            self.push(Frame::Call(rules::MAPPING));
        }
    }
    #[inline(never)]
    pub(super) fn brace_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        match *phase {
            Phase::Enter(expression) => self.brace(Phase::Choice(expression, 0)),
            Phase::Choice(expression, index) => {
                let checkpoint = parser.checkpoint();
                let node = parser.start();
                self.brace(Phase::Chosen(expression, index, node, checkpoint));
                if index < 2 {
                    self.push(Frame::Shell(Box::new(
                        super::super::super::super::structure_shell::Continuation::new(
                            if index == 0 {
                                rules::EMPTY_SET
                            } else {
                                rules::EMPTY_MAP
                            },
                        ),
                    )));
                } else {
                    self.push(Frame::Call(rules::TABLE));
                }
            }
            Phase::Chosen(expression, index, node, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    if index < 2 {
                        self.brace(Phase::Choice(expression, index + 1));
                    } else {
                        self.brace(Phase::General(expression));
                    }
                } else {
                    node.complete(parser, SyntaxKind::Structure);
                    self.brace_formula(self.result == Attempt::Committed);
                }
            }
            Phase::General(expression) => {
                let checkpoint = parser.checkpoint();
                let owner = Owner {
                    structure: parser.start(),
                    map: parser.start(),
                    set: parser.start(),
                    comprehension: parser.start(),
                    record: parser.start(),
                    expression,
                };
                self.brace(Phase::Exit(checkpoint));
                self.brace(Phase::Open(owner));
                self.base(rules::LEFT_BRACE);
            }
            Phase::Exit(checkpoint) => {
                if self.facts == FactAttempt::NoMatch {
                    parser.rewind(checkpoint);
                }
                self.result = self.facts.attempt();
            }
            Phase::Open(owner) => {
                if self.result != Attempt::Matched {
                    self.expression_result(FactAttempt::NoMatch);
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.brace(Phase::Space(owner));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.brace(Phase::Limited(owner));
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Limited(owner) => {
                owner.record.complete(parser, SyntaxKind::Record);
                finish_provisional_marker(
                    parser,
                    owner.comprehension,
                    SyntaxKind::SetComprehension,
                );
                owner.set.complete(parser, SyntaxKind::Set);
                finish_provisional_marker(parser, owner.map, SyntaxKind::Map);
                owner.structure.complete(parser, SyntaxKind::Structure);
                self.expression_result(FactAttempt::Committed);
            }
            Phase::Space(owner) => {
                if self.result != Attempt::Matched {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    self.brace(Phase::Binding(owner, parser.checkpoint()));
                    self.push(Frame::Call(rules::BINDING));
                }
            }
            Phase::Binding(owner, interior) => {
                if self.result == Attempt::NoMatch {
                    finish_provisional_marker(parser, owner.record, SyntaxKind::Record);
                    let entry = parser.start();
                    self.brace(Phase::Head(owner, entry));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    let mut bindings = Vec::new();
                    if self.result == Attempt::Matched {
                        bindings.push(self.binding_candidate.expect("clean binding checkpoints"));
                    }
                    self.brace(Phase::RecordStart(Record {
                        owner,
                        interior,
                        bindings,
                        committed: self.result == Attempt::Committed,
                    }));
                }
            }
            Phase::Head(owner, entry) => {
                if self.result == Attempt::NoMatch {
                    if !parser.cursor().starts_with(",") {
                        self.expression_result(FactAttempt::NoMatch);
                    } else {
                        self.brace(Phase::MissingHead(owner, entry));
                        self.push(Frame::Missing(Box::new(
                            crate::document::parser::recovery::MissingContinuation::new(
                                "syntax/missing-set-item",
                                "missing set item",
                                crate::document::ExpectedSyntax::Production(String::from(
                                    "expression",
                                )),
                                None,
                            ),
                        )));
                    }
                } else {
                    self.brace_head(parser, owner, entry, self.result == Attempt::Committed);
                }
            }
            Phase::MissingHead(owner, entry) => self.brace_head(parser, owner, entry, true),
            Phase::BarSpace(head) => {
                if self.result == Attempt::Matched {
                    self.brace(Phase::Bar(head));
                    self.base(rules::BAR);
                } else {
                    self.brace(Phase::ColonStart(head));
                }
            }
            Phase::Bar(head) => {
                if self.result == Attempt::Matched {
                    if !head.owner.expression {
                        self.expression_result(FactAttempt::NoMatch);
                    } else {
                        finish_provisional_marker(parser, head.entry, SyntaxKind::MapEntry);
                        self.brace(Phase::Qualifiers(head));
                        self.comprehension_tail(rules::RIGHT_BRACE, false);
                    }
                } else {
                    self.brace(Phase::ColonStart(head));
                }
            }
            Phase::Qualifiers(head) => {
                if self.result == Attempt::NoMatch {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    head.owner
                        .comprehension
                        .complete(parser, SyntaxKind::SetComprehension);
                    finish_provisional_marker(parser, head.owner.set, SyntaxKind::Set);
                    finish_provisional_marker(parser, head.owner.map, SyntaxKind::Map);
                    finish_provisional_marker(parser, head.owner.structure, SyntaxKind::Structure);
                    self.expression_result(
                        if head.committed || self.result == Attempt::Committed {
                            FactAttempt::Recovered(ExpressionForm::SetComprehension)
                        } else {
                            FactAttempt::Matched(ExpressionForm::SetComprehension)
                        },
                    );
                }
            }
            Phase::ColonStart(head) => {
                parser.rewind(head.after);
                self.brace(Phase::Colon(head, 0));
                self.base(rules::WHITESPACE0);
            }
            Phase::Colon(head, index) => {
                if self.result != Attempt::Matched {
                    self.brace(Phase::SetStart(head));
                } else if index < 2 {
                    self.brace(Phase::Colon(head, index + 1));
                    self.base(if index == 0 {
                        rules::COLON
                    } else {
                        rules::WHITESPACE0
                    });
                } else {
                    self.brace(Phase::Value(head));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::Value(mut head) => {
                if self.result == Attempt::NoMatch {
                    self.brace(Phase::ValueRecovered(head));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::MAP,
                        "syntax/missing-mapping-value",
                        "missing value after mapping colon",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    head.committed |= self.result == Attempt::Committed;
                    self.brace(Phase::ValueSuffix(head, 0));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::ValueRecovered(mut head) => {
                head.committed = true;
                self.brace(Phase::ValueSuffix(head, 0));
                self.base(rules::WHITESPACE0);
            }
            Phase::ValueSuffix(head, index) => {
                if index != 1 && self.result != Attempt::Matched {
                    self.expression_result(FactAttempt::NoMatch);
                } else if index < 2 {
                    self.brace(Phase::ValueSuffix(head, index + 1));
                    self.base(if index == 0 {
                        rules::COMMA
                    } else {
                        rules::WHITESPACE0
                    });
                } else {
                    head.entry.complete(parser, SyntaxKind::MapEntry);
                    finish_provisional_marker(
                        parser,
                        head.owner.comprehension,
                        SyntaxKind::SetComprehension,
                    );
                    finish_provisional_marker(parser, head.owner.set, SyntaxKind::Set);
                    self.brace(Phase::MapTail(Map {
                        owner: head.owner,
                        committed: head.committed,
                        strict: false,
                    }));
                    self.push(Frame::Map(Box::new(map::Phase::Loop(true, false))));
                }
            }
            Phase::SetStart(head) => {
                parser.rewind(head.after);
                finish_provisional_marker(parser, head.entry, SyntaxKind::MapEntry);
                finish_provisional_marker(
                    parser,
                    head.owner.comprehension,
                    SyntaxKind::SetComprehension,
                );
                self.brace(Phase::SetLoop(head.owner, head.committed));
            }
            Phase::SetLoop(owner, committed) => {
                if parser.is_halted() {
                    self.brace(Phase::SetTrailing(owner, committed));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.brace(Phase::SetSeparator(owner, committed, parser.checkpoint()));
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::SetSeparator(owner, committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.brace(Phase::SetItem(owner, committed, checkpoint));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.brace(Phase::SetSpaceSeparator(owner, committed, checkpoint));
                    self.base(rules::WHITESPACE1);
                }
            }
            Phase::SetSpaceSeparator(owner, committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.brace(Phase::SetItem(owner, committed, checkpoint));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.brace(Phase::SetTrailing(owner, committed));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::SetItem(owner, committed, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    self.brace(Phase::SetTrailing(owner, committed));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.brace(Phase::SetLoop(
                        owner,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::SetTrailing(owner, committed) => {
                self.brace(Phase::SetClose(owner, committed));
                self.base(rules::RIGHT_BRACE);
            }
            Phase::SetClose(owner, committed) => {
                if self.result == Attempt::Matched {
                    self.brace_set_finish(parser, owner, committed);
                } else {
                    self.brace(Phase::SetRecovered(owner));
                    self.push(Frame::Closer(Box::new(Closer::new(
                        rules::SET,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        "}",
                    ))));
                }
            }
            Phase::SetRecovered(owner) => self.brace_set_finish(parser, owner, true),
            Phase::RecordStart(record) => {
                let committed = record.committed;
                self.brace(Phase::RecordLoop(record));
                if committed {
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::RecordLoop(record) => {
                if parser.is_halted() {
                    self.brace(Phase::RecordTrailing(record, parser.checkpoint()));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.brace(Phase::RecordBinding(record, parser.offset()));
                    self.push(Frame::Call(rules::BINDING));
                }
            }
            Phase::RecordBinding(mut record, before) => match self.result {
                Attempt::Matched => {
                    record
                        .bindings
                        .push(self.binding_candidate.expect("clean binding checkpoints"));
                    self.brace(Phase::RecordLoop(record));
                }
                Attempt::Committed => {
                    record.committed = true;
                    self.brace(Phase::RecordSeparator(record, before));
                    self.base(rules::LIST_SEPARATOR);
                }
                Attempt::NoMatch => {
                    self.brace(Phase::RecordTrailing(record, parser.checkpoint()));
                    self.base(rules::WHITESPACE0);
                }
            },
            Phase::RecordSeparator(record, before) => {
                if parser.offset() == before {
                    self.brace(Phase::RecordTrailing(record, parser.checkpoint()));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.brace(Phase::RecordLoop(record));
                }
            }
            Phase::RecordTrailing(record, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.brace(Phase::RecordClose(record, checkpoint));
                    self.base(rules::RIGHT_BRACE);
                } else {
                    self.brace(Phase::RecordRejected(record, checkpoint));
                }
            }
            Phase::RecordClose(record, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.brace_record_finish(parser, record.owner, record.committed);
                } else {
                    self.brace(Phase::RecordRejected(record, checkpoint));
                }
            }
            Phase::RecordRejected(record, checkpoint) => {
                parser.rewind(checkpoint);
                if record.committed {
                    self.brace_record_recover(record.owner);
                } else {
                    self.brace(Phase::RecordProbe(record));
                    self.push(Frame::MappingProbe(Box::new(mapping_probe::Phase::Enter)));
                }
            }
            Phase::RecordProbe(record) => {
                if self.result == Attempt::Matched {
                    self.brace(Phase::Cache(record, 0, Vec::new()));
                } else {
                    self.brace_record_recover(record.owner);
                }
            }
            Phase::RecordRecovered(owner) => self.brace_record_finish(parser, owner, true),
            Phase::Cache(record, index, mut caches) => {
                if let Some(binding) = record.bindings.get(index) {
                    if let Some(cache) =
                        parser.cache_clean_subtree(binding.value_start, binding.value_end)
                    {
                        caches.push(cache);
                        self.brace(Phase::Cache(record, index + 1, caches));
                    } else {
                        self.brace_replay_start(parser, record, None);
                    }
                } else {
                    self.brace_replay_start(parser, record, Some(caches));
                }
            }
            Phase::Replay(owner, mut caches, committed) => {
                if let Some(cache) = caches.next() {
                    self.brace(Phase::ReplayItem(owner, caches, committed));
                    self.push(Frame::Entry(Box::new(entry::Phase::Enter(
                        false,
                        Some(cache),
                    ))));
                } else {
                    self.brace(Phase::MapTail(Map {
                        owner,
                        committed,
                        strict: true,
                    }));
                    self.push(Frame::Map(Box::new(map::Phase::Loop(true, false))));
                }
            }
            Phase::ReplayItem(owner, caches, committed) => {
                if self.result == Attempt::NoMatch {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    self.brace(Phase::Replay(
                        owner,
                        caches,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::ReplayUncached(owner) => {
                if self.result == Attempt::NoMatch {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    self.brace(Phase::MapTail(Map {
                        owner,
                        committed: self.result == Attempt::Committed,
                        strict: true,
                    }));
                    self.push(Frame::Map(Box::new(map::Phase::Loop(true, false))));
                }
            }
            Phase::MapTail(mut map) => {
                map.committed |= self.result == Attempt::Committed;
                self.brace(Phase::MapTrailing(map));
                self.base(rules::WHITESPACE0);
            }
            Phase::MapTrailing(map) => {
                if map.strict && self.result != Attempt::Matched {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    self.brace(Phase::MapClose(map));
                    self.base(rules::RIGHT_BRACE);
                }
            }
            Phase::MapClose(mut map) => {
                if self.result == Attempt::Matched {
                    self.brace_map_finish(parser, map);
                } else if map.strict && !map.committed {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    if !map.strict {
                        map.committed = true;
                    }
                    self.brace(Phase::MapRecovered(map));
                    self.push(Frame::Closer(Box::new(Closer::new(
                        rules::MAP,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        "}",
                    ))));
                }
            }
            Phase::MapRecovered(map) => self.brace_map_finish(parser, map),
        }
    }
}
