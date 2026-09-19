//! Retained module-import choices, separators, commitment, and shared recovery.
use super::*;
use crate::document::TextSize;
use crate::document::parser::checkpoint::ParserCheckpoint;
use crate::document::parser::marker::Marker;
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
enum Op {
    Rule(RuleId),
    Ignore(RuleId),
    Any(&'static [RuleId]),
    Sequence(&'static [Op]),
    Repeat(&'static Op),
}
#[derive(Clone, Copy)]
struct Node {
    marker: Marker,
    kind: SyntaxKind,
}
impl Node {
    fn complete(self, parser: &mut Parser<'_>) {
        self.marker.complete(parser, self.kind);
    }
}
#[derive(Clone, Copy)]
struct List {
    child: RuleId,
    separator: RuleId,
    trailing: bool,
    stop_committed: bool,
}
#[derive(Clone, Copy)]
enum Missing {
    Intrinsic,
    Target,
    Slash,
    Item,
    Suffix,
    GroupItem,
    GroupAfterSeparator,
    GroupClose,
    ContextAlias,
    AliasOperator,
    AliasEqual,
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    Accept,
    Ignore,
    SetResult,
    Finish(Node),
    Aggregate(Node),
    Complete(Node, Attempt),
    Operation(Op),
    Any(&'static [RuleId], usize),
    Sequence(&'static [Op], usize),
    Repeat(&'static Op, TextSize),
    Intrinsic(Node, bool),
    ContextAlias(Node),
    RejectSlash(Node, Attempt, ParserCheckpoint),
    ListStart(Node, List),
    ListFirst(Node, List),
    ListSeparator(Node, List, Attempt, ParserCheckpoint),
    ListItem(Node, List, Attempt, ParserCheckpoint),
    ListFinish(Node, Attempt),
    AliasPrefix(Node, u8),
    Target(Node, bool, u8, bool),
    Suffix(Node, u8, bool),
    SuffixFinish(Node, bool),
    GroupItems,
    GroupSeparator(Attempt, ParserCheckpoint),
    GroupClose(Attempt),
    Module(Node, u8),
    ModuleProbe(Node, ParserCheckpoint),
    ModuleAlias(Node),
    RecoverStart,
    RecoverSegment([Node; 3]),
    RecoverProbe([Node; 3], ParserCheckpoint),
    RecoverFinish([Node; 3]),
    RecoverOperator(Node, u8),
    Missing(Missing),
    MissingOwner(recovery::MissingContinuation<'static>, Option<&'static str>),
    Skip(recovery::SkipContinuation<'static>),
    Base(base::continuation::Continuation),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical module-import owner");
        Self {
            frames: alloc::vec![Frame::Enter(rule)],
            result: Attempt::NoMatch,
            matched: false,
            work: 0,
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
    }
    fn child(&mut self, rule: RuleId) {
        if supports(rule) {
            self.push(Frame::Accept);
            self.push(Frame::Enter(rule));
        } else {
            self.base(rule);
        }
    }
    fn node(parser: &mut Parser<'_>, kind: SyntaxKind) -> Node {
        Node {
            marker: parser.start(),
            kind,
        }
    }
    fn sequence(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind, ops: &'static [Op]) {
        self.push(Frame::Finish(Self::node(parser, kind)));
        self.push(Frame::Operation(Op::Sequence(ops)));
    }
    fn aggregate(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind, choices: &'static [RuleId]) {
        self.push(Frame::Aggregate(Self::node(parser, kind)));
        self.push(Frame::Operation(Op::Any(choices)));
    }
    fn list_pair(&mut self, parser: &Parser<'_>, node: Node, list: List, result: Attempt) {
        self.push(Frame::ListSeparator(
            node,
            list,
            result,
            parser.checkpoint(),
        ));
        self.child(list.separator);
    }
    fn finish_list(&mut self, parser: &mut Parser<'_>, node: Node, list: List, result: Attempt) {
        if list.trailing {
            self.push(Frame::ListFinish(node, result));
            self.base(rules::WHITESPACE0);
        } else {
            node.complete(parser);
            self.result = result;
        }
    }
    fn fail_required(&mut self, node: Node, missing: Missing) {
        self.push(Frame::Complete(node, Attempt::Committed));
        self.push(Frame::Missing(missing));
    }
    fn target(&mut self, node: Node, force_committed: bool) {
        self.push(Frame::Target(node, force_committed, 0, false));
        self.child(rules::MODULE_ROOT);
    }
    fn close_group(&mut self, result: Attempt) {
        self.push(Frame::GroupClose(result));
        self.base(rules::RIGHT_BRACE);
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty() {
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let frame = self.frames.pop().expect("retained module-import phase");
            if !matches!(
                frame,
                Frame::Base(_) | Frame::Skip(_) | Frame::MissingOwner(..)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint));
                    match rule {
                        rules::MODULE_IMPORT_NAME_SEGMENT => self.sequence(
                            parser,
                            SyntaxKind::ModuleImportNameSegment,
                            &[Op::Rule(rules::IDENTIFIER_PATH_SEGMENT)],
                        ),
                        rules::MODULE_IMPORT_ALIAS_SEGMENT => self.sequence(
                            parser,
                            SyntaxKind::ModuleImportAliasSegment,
                            &[Op::Rule(rules::IDENTIFIER_PATH_SEGMENT)],
                        ),
                        rules::MODULE_ROOT => self.sequence(
                            parser,
                            SyntaxKind::ModuleRoot,
                            &[Op::Rule(rules::IDENTIFIER_PATH_SEGMENT)],
                        ),
                        rules::MODULE_IMPORT_INTRINSIC_SEGMENT => {
                            self.push(Frame::Intrinsic(
                                Self::node(parser, SyntaxKind::ModuleImportIntrinsicSegment),
                                false,
                            ));
                            self.base(rules::UNDERSCORE);
                        }
                        rules::MODULE_IMPORT_PATH_SEGMENT => self.aggregate(
                            parser,
                            SyntaxKind::ModuleImportPathSegment,
                            &[
                                rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
                                rules::MODULE_IMPORT_NAME_SEGMENT,
                            ],
                        ),
                        rules::MODULE_IMPORT_VALUE_ALIAS => self.aggregate(
                            parser,
                            SyntaxKind::ModuleImportValueAlias,
                            &[rules::MODULE_IMPORT_ALIAS_PATH],
                        ),
                        rules::MODULE_IMPORT_ALIAS => self.aggregate(
                            parser,
                            SyntaxKind::ModuleImportAlias,
                            &[
                                rules::MODULE_IMPORT_CONTEXT_ALIAS,
                                rules::MODULE_IMPORT_VALUE_ALIAS,
                            ],
                        ),
                        rules::IMPORT_GROUP_ITEM => self.aggregate(
                            parser,
                            SyntaxKind::ImportGroupItem,
                            &[rules::MODULE_IMPORT_PATH],
                        ),
                        rules::MODULE_IMPORT_PATH
                        | rules::MODULE_IMPORT_ALIAS_PATH
                        | rules::IMPORT_GROUP_ITEMS => {
                            let (kind, list) = match rule {
                                rules::MODULE_IMPORT_PATH => (
                                    SyntaxKind::ModuleImportPath,
                                    List {
                                        child: rules::MODULE_IMPORT_PATH_SEGMENT,
                                        separator: rules::SLASH,
                                        trailing: false,
                                        stop_committed: true,
                                    },
                                ),
                                rules::MODULE_IMPORT_ALIAS_PATH => (
                                    SyntaxKind::ModuleImportAliasPath,
                                    List {
                                        child: rules::MODULE_IMPORT_ALIAS_SEGMENT,
                                        separator: rules::SLASH,
                                        trailing: false,
                                        stop_committed: false,
                                    },
                                ),
                                _ => (
                                    SyntaxKind::ImportGroupItems,
                                    List {
                                        child: rules::IMPORT_GROUP_ITEM,
                                        separator: rules::IMPORT_GROUP_SEPARATOR,
                                        trailing: true,
                                        stop_committed: true,
                                    },
                                ),
                            };
                            let node = Self::node(parser, kind);
                            if list.trailing {
                                self.push(Frame::ListStart(node, list));
                                self.base(rules::WHITESPACE0);
                            } else {
                                self.push(Frame::ListFirst(node, list));
                                self.child(list.child);
                            }
                        }
                        rules::CONTEXT_IMPORT_ALIAS_SEGMENT => self.sequence(
                            parser,
                            SyntaxKind::ContextImportAliasSegment,
                            &[
                                Op::Rule(rules::ALPHA_TOKEN),
                                Op::Repeat(&Op::Any(&[
                                    rules::ALPHA_TOKEN,
                                    rules::DIGIT_TOKEN,
                                    rules::DASH,
                                ])),
                            ],
                        ),
                        rules::MODULE_IMPORT_CONTEXT_ALIAS => {
                            self.push(Frame::ContextAlias(Self::node(
                                parser,
                                SyntaxKind::ModuleImportContextAlias,
                            )));
                            self.push(Frame::Operation(Op::Sequence(&[
                                Op::Rule(rules::AT),
                                Op::Rule(rules::CONTEXT_IMPORT_ALIAS_SEGMENT),
                            ])));
                        }
                        rules::IMPORT_ALIAS_OPERATOR => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Sequence(&[
                                Op::Rule(rules::SPACE_TAB0),
                                Op::Rule(rules::COLON),
                                Op::Rule(rules::EQUAL),
                                Op::Rule(rules::SPACE_TAB0),
                            ])));
                        }
                        rules::IMPORT_GROUP_SEPARATOR => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Any(&[
                                rules::LIST_SEPARATOR,
                                rules::WHITESPACE1,
                            ])));
                        }
                        rules::ALIASED_ITEM_IMPORT => {
                            self.push(Frame::AliasPrefix(
                                Self::node(parser, SyntaxKind::AliasedItemImport),
                                0,
                            ));
                            self.child(rules::MODULE_IMPORT_ALIAS);
                        }
                        rules::MODULE_SUFFIX_IMPORT => {
                            self.push(Frame::Suffix(
                                Self::node(parser, SyntaxKind::ModuleSuffixImport),
                                0,
                                false,
                            ));
                            self.child(rules::MODULE_ROOT);
                        }
                        rules::MODULE_ONLY_IMPORT => {
                            self.push(Frame::ContextAlias(Self::node(
                                parser,
                                SyntaxKind::ModuleOnlyImport,
                            )));
                            self.child(rules::MODULE_ROOT);
                        }
                        rules::MODULE_IMPORT => {
                            self.push(Frame::Module(
                                Self::node(parser, SyntaxKind::ModuleImport),
                                0,
                            ));
                            self.base(rules::WHITESPACE0);
                        }
                        _ => unreachable!("supported module-import rule"),
                    }
                }
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::Accept => self.matched = self.result.accepted(),
                Frame::Ignore => self.matched = true,
                Frame::SetResult => {
                    self.result = if self.matched {
                        Attempt::Matched
                    } else {
                        Attempt::NoMatch
                    }
                }
                Frame::Finish(node) => {
                    self.result = if self.matched {
                        node.complete(parser);
                        Attempt::Matched
                    } else {
                        Attempt::NoMatch
                    }
                }
                Frame::Aggregate(node) => {
                    if self.result.accepted() {
                        node.complete(parser);
                    }
                }
                Frame::Complete(node, result) => {
                    node.complete(parser);
                    self.result = result;
                }
                Frame::Operation(op) => match op {
                    Op::Rule(rule) => self.child(rule),
                    Op::Ignore(rule) => {
                        self.push(Frame::Ignore);
                        self.child(rule);
                    }
                    Op::Any(rules) => {
                        self.push(Frame::Any(rules, 0));
                        self.child(rules[0]);
                    }
                    Op::Sequence(ops) => {
                        self.push(Frame::Sequence(ops, 0));
                        self.push(Frame::Operation(ops[0]));
                    }
                    Op::Repeat(op) => {
                        self.push(Frame::Repeat(op, parser.offset()));
                        self.push(Frame::Operation(*op));
                    }
                },
                Frame::Any(rules, index) => {
                    if !self.matched
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Any(rules, index + 1));
                        self.child(*rule);
                    }
                }
                Frame::Sequence(ops, index) => {
                    if self.matched
                        && let Some(op) = ops.get(index + 1)
                    {
                        self.push(Frame::Sequence(ops, index + 1));
                        self.push(Frame::Operation(*op));
                    }
                }
                Frame::Repeat(op, before) => {
                    if self.matched && parser.offset() != before && !parser.is_halted() {
                        self.push(Frame::Repeat(op, parser.offset()));
                        self.push(Frame::Operation(*op));
                    } else {
                        self.matched = true;
                    }
                }
                Frame::Intrinsic(node, name) => {
                    if !self.matched {
                        if name {
                            self.fail_required(node, Missing::Intrinsic);
                        } else {
                            self.result = Attempt::NoMatch;
                        }
                    } else if !name {
                        self.push(Frame::Intrinsic(node, true));
                        self.child(rules::MODULE_IMPORT_NAME_SEGMENT);
                    } else {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::ContextAlias(node) => {
                    if self.matched {
                        let result = if node.kind == SyntaxKind::ModuleOnlyImport {
                            self.result
                        } else {
                            Attempt::Matched
                        };
                        self.push(Frame::RejectSlash(node, result, parser.checkpoint()));
                        self.base(rules::SLASH);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::RejectSlash(node, result, checkpoint) => {
                    parser.rewind(checkpoint);
                    self.result = if self.matched {
                        Attempt::NoMatch
                    } else {
                        node.complete(parser);
                        result
                    };
                }
                Frame::ListStart(node, list) => {
                    self.push(Frame::ListFirst(node, list));
                    self.child(list.child);
                }
                Frame::ListFirst(node, list) => {
                    if self.result == Attempt::NoMatch {
                    } else if list.stop_committed && self.result == Attempt::Committed {
                        node.complete(parser);
                    } else {
                        self.list_pair(parser, node, list, self.result);
                    }
                }
                Frame::ListSeparator(node, list, result, checkpoint) => {
                    if !self.matched {
                        parser.rewind(checkpoint);
                        self.finish_list(parser, node, list, result);
                    } else {
                        self.push(Frame::ListItem(node, list, result, checkpoint));
                        self.child(list.child);
                    }
                }
                Frame::ListItem(node, list, previous, checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                        self.finish_list(parser, node, list, previous);
                    } else if list.stop_committed && self.result == Attempt::Committed {
                        node.complete(parser);
                    } else {
                        let result = if self.result == Attempt::Committed {
                            self.result
                        } else {
                            previous
                        };
                        if parser.is_halted() {
                            self.finish_list(parser, node, list, result);
                        } else {
                            self.list_pair(parser, node, list, result);
                        }
                    }
                }
                Frame::ListFinish(node, result) => {
                    node.complete(parser);
                    self.result = result;
                }
                Frame::AliasPrefix(node, stage) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else if stage == 0 {
                        self.push(Frame::AliasPrefix(node, 1));
                        self.child(rules::IMPORT_ALIAS_OPERATOR);
                    } else {
                        self.target(node, false);
                    }
                }
                Frame::Target(node, force, stage, module_committed) => {
                    if !self.matched {
                        self.fail_required(
                            node,
                            [Missing::Target, Missing::Slash, Missing::Item][stage as usize],
                        );
                    } else if stage == 0 {
                        self.push(Frame::Target(
                            node,
                            force,
                            1,
                            self.result == Attempt::Committed,
                        ));
                        self.base(rules::SLASH);
                    } else if stage == 1 {
                        self.push(Frame::Target(node, force, 2, module_committed));
                        self.child(rules::MODULE_IMPORT_PATH);
                    } else {
                        node.complete(parser);
                        if force || module_committed {
                            self.result = Attempt::Committed;
                        }
                    }
                }
                Frame::Suffix(node, stage, module_committed) => match stage {
                    0 => {
                        if self.matched {
                            self.push(Frame::Suffix(node, 1, self.result == Attempt::Committed));
                            self.base(rules::SLASH);
                        } else {
                            self.result = Attempt::NoMatch;
                        }
                    }
                    1 => {
                        if self.matched {
                            self.push(Frame::Suffix(node, 2, module_committed));
                            self.base(rules::ASTERISK);
                        } else {
                            self.result = Attempt::NoMatch;
                        }
                    }
                    2 => {
                        if self.matched {
                            node.complete(parser);
                            self.result = if module_committed {
                                Attempt::Committed
                            } else {
                                Attempt::Matched
                            };
                        } else {
                            self.push(Frame::Suffix(node, 3, module_committed));
                            self.base(rules::LEFT_BRACE);
                        }
                    }
                    3 => {
                        if self.matched {
                            self.push(Frame::SuffixFinish(node, module_committed));
                            self.push(Frame::GroupItems);
                            self.child(rules::IMPORT_GROUP_ITEMS);
                        } else {
                            self.push(Frame::Suffix(node, 4, module_committed));
                            self.child(rules::MODULE_IMPORT_PATH);
                        }
                    }
                    _ => {
                        if !self.matched {
                            self.fail_required(node, Missing::Suffix);
                        } else {
                            node.complete(parser);
                            if module_committed {
                                self.result = Attempt::Committed;
                            }
                        }
                    }
                },
                Frame::SuffixFinish(node, module_committed) => {
                    node.complete(parser);
                    if module_committed {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::GroupItems => match self.result {
                    Attempt::NoMatch => {
                        self.close_group(Attempt::Committed);
                        self.push(Frame::Missing(Missing::GroupItem));
                    }
                    Attempt::Matched => {
                        self.push(Frame::GroupSeparator(self.result, parser.checkpoint()));
                        self.base(rules::LIST_SEPARATOR);
                    }
                    Attempt::Committed => self.close_group(self.result),
                },
                Frame::GroupSeparator(result, checkpoint) => {
                    parser.rewind(checkpoint);
                    if self.matched {
                        self.close_group(Attempt::Committed);
                        self.push(Frame::Missing(Missing::GroupAfterSeparator));
                    } else {
                        self.close_group(result);
                    }
                }
                Frame::GroupClose(result) => {
                    self.result = if self.matched {
                        result
                    } else {
                        self.push(Frame::Missing(Missing::GroupClose));
                        Attempt::Committed
                    };
                }
                Frame::Module(node, stage) => match stage {
                    0 => {
                        self.push(Frame::Module(node, 1));
                        self.base(rules::IMPORT_SIGIL);
                    }
                    1 if !self.matched => self.result = Attempt::NoMatch,
                    1 => {
                        self.push(Frame::Module(node, 2));
                        self.base(rules::SPACE_TAB0);
                    }
                    _ => {
                        self.push(Frame::ModuleProbe(node, parser.checkpoint()));
                        self.base(rules::AT);
                    }
                },
                Frame::ModuleProbe(node, checkpoint) => {
                    parser.rewind(checkpoint);
                    if self.matched {
                        self.push(Frame::ModuleAlias(node));
                        self.child(rules::ALIASED_ITEM_IMPORT);
                    } else {
                        self.push(Frame::Aggregate(node));
                        self.push(Frame::Operation(Op::Any(&[
                            rules::ALIASED_ITEM_IMPORT,
                            rules::MODULE_SUFFIX_IMPORT,
                            rules::MODULE_ONLY_IMPORT,
                        ])));
                    }
                }
                Frame::ModuleAlias(node) => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::Aggregate(node));
                        self.push(Frame::RecoverStart);
                    } else {
                        node.complete(parser);
                    }
                }
                Frame::RecoverStart => {
                    let nodes = [
                        Self::node(parser, SyntaxKind::AliasedItemImport),
                        Self::node(parser, SyntaxKind::ModuleImportAlias),
                        Self::node(parser, SyntaxKind::ModuleImportContextAlias),
                    ];
                    self.push(Frame::RecoverSegment(nodes));
                    self.push(Frame::Operation(Op::Sequence(&[
                        Op::Ignore(rules::AT),
                        Op::Rule(rules::CONTEXT_IMPORT_ALIAS_SEGMENT),
                    ])));
                }
                Frame::RecoverSegment(nodes) => {
                    if !self.matched {
                        self.push(Frame::RecoverFinish(nodes));
                        self.push(Frame::Missing(Missing::ContextAlias));
                    } else {
                        self.push(Frame::RecoverProbe(nodes, parser.checkpoint()));
                        self.base(rules::SLASH);
                    }
                }
                Frame::RecoverProbe(nodes, checkpoint) => {
                    parser.rewind(checkpoint);
                    if self.matched {
                        self.push(Frame::RecoverFinish(nodes));
                        self.push(Frame::Skip(recovery::SkipContinuation::new(
                            RecoveryClass::MechItem,
                            "syntax/invalid-module-import-context-alias",
                            "context import aliases cannot continue with `/`",
                        )));
                    } else {
                        nodes[2].complete(parser);
                        nodes[1].complete(parser);
                        self.push(Frame::RecoverOperator(nodes[0], 0));
                        self.child(rules::IMPORT_ALIAS_OPERATOR);
                    }
                }
                Frame::RecoverFinish(nodes) => {
                    for node in nodes.into_iter().rev() {
                        node.complete(parser);
                    }
                    self.result = Attempt::Committed;
                }
                Frame::RecoverOperator(node, stage) => match stage {
                    0 if self.matched => self.target(node, true),
                    0 => {
                        self.push(Frame::RecoverOperator(node, 1));
                        self.base(rules::SPACE_TAB0);
                    }
                    1 => {
                        self.push(Frame::RecoverOperator(node, 2));
                        self.base(rules::COLON);
                    }
                    2 if !self.matched => self.fail_required(node, Missing::AliasOperator),
                    2 => {
                        self.push(Frame::RecoverOperator(node, 3));
                        self.base(rules::EQUAL);
                    }
                    3 if !self.matched => self.fail_required(node, Missing::AliasEqual),
                    3 => {
                        self.push(Frame::RecoverOperator(node, 4));
                        self.base(rules::SPACE_TAB0);
                    }
                    _ => self.target(node, true),
                },
                Frame::Missing(missing) => {
                    let (code, message, expected, token, fix_text) = match missing {
                        Missing::Intrinsic => (
                            "syntax/missing-module-import-intrinsic-name",
                            "expected a module-import name after `_`",
                            ExpectedSyntax::Production(String::from("module-import-name-segment")),
                            None,
                            None,
                        ),
                        Missing::Target => (
                            "syntax/missing-module-import-alias-target",
                            "expected a module root after the import alias operator",
                            ExpectedSyntax::Production(String::from("module-root")),
                            None,
                            None,
                        ),
                        Missing::Slash => (
                            "syntax/missing-module-import-aliased-item-separator",
                            "expected `/` between the imported module and item",
                            ExpectedSyntax::Token(SyntaxKind::Slash),
                            Some(SyntaxKind::Slash),
                            Some("/"),
                        ),
                        Missing::Item => (
                            "syntax/missing-module-import-aliased-item",
                            "expected an imported item path after `/`",
                            ExpectedSyntax::Production(String::from("module-import-path")),
                            None,
                            None,
                        ),
                        Missing::Suffix => (
                            "syntax/missing-module-import-suffix",
                            "expected a module import suffix after `/`",
                            ExpectedSyntax::Production(String::from("module-import-path")),
                            None,
                            None,
                        ),
                        Missing::GroupItem => (
                            "syntax/missing-module-import-group-item",
                            "expected an item in the module import group",
                            ExpectedSyntax::Production(String::from("import-group-item")),
                            None,
                            None,
                        ),
                        Missing::GroupAfterSeparator => (
                            "syntax/missing-module-import-group-item",
                            "expected an item after the module import group separator",
                            ExpectedSyntax::Production(String::from("import-group-item")),
                            None,
                            None,
                        ),
                        Missing::GroupClose => (
                            "syntax/unclosed-module-import-group",
                            "expected `}` to close the module import group",
                            ExpectedSyntax::Token(SyntaxKind::RightBrace),
                            Some(SyntaxKind::RightBrace),
                            Some("}"),
                        ),
                        Missing::ContextAlias => (
                            "syntax/missing-module-import-context-alias",
                            "expected a context import alias after `@`",
                            ExpectedSyntax::Production(String::from(
                                "context-import-alias-segment",
                            )),
                            None,
                            None,
                        ),
                        Missing::AliasOperator => (
                            "syntax/missing-module-import-alias-operator",
                            "expected `:=` after the context import alias",
                            ExpectedSyntax::Production(String::from("import-alias-operator")),
                            None,
                            None,
                        ),
                        Missing::AliasEqual => (
                            "syntax/missing-module-import-alias-equal",
                            "expected `=` after `:` in the import alias operator",
                            ExpectedSyntax::Token(SyntaxKind::Equal),
                            Some(SyntaxKind::Equal),
                            Some("="),
                        ),
                    };
                    self.push(Frame::MissingOwner(
                        recovery::MissingContinuation::new(code, message, expected, token),
                        fix_text,
                    ));
                }
                Frame::MissingOwner(mut continuation, fix_text) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        recovery::MissingProgress::Complete(_) => {
                            combinator::attach_missing_fix(parser, fix_text)
                        }
                        recovery::MissingProgress::NeedInput => {
                            self.push(Frame::MissingOwner(continuation, fix_text));
                            return Progress::NeedInput;
                        }
                        recovery::MissingProgress::NeedsProcessing => {
                            self.push(Frame::MissingOwner(continuation, fix_text));
                            return Progress::NeedsProcessing;
                        }
                        recovery::MissingProgress::Limited => {
                            self.push(Frame::MissingOwner(continuation, fix_text));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Skip(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        recovery::SkipProgress::Complete(_) => {}
                        recovery::SkipProgress::NeedInput => {
                            self.push(Frame::Skip(continuation));
                            return Progress::NeedInput;
                        }
                        recovery::SkipProgress::NeedsProcessing => {
                            self.push(Frame::Skip(continuation));
                            return Progress::NeedsProcessing;
                        }
                        recovery::SkipProgress::Limited => {
                            self.push(Frame::Skip(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(matched) => self.matched = matched,
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::Base(continuation));
                            return Progress::Limited;
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::continuation_test_support::{assert_partitions, run};
    use super::*;

    #[test]
    fn missing_name_recovery_can_complete_before_final_eof() {
        use crate::document::parser::LexicalMode;
        use crate::document::{DocumentId, IdGenerator, ParseConfig, Revision, TextSnapshot};
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "_! x").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let wrapper = parser.start();
        let mut continuation = Continuation::new(rules::MODULE_IMPORT_INTRINSIC_SEGMENT);
        loop {
            let mut allowance = 1;
            match continuation.advance(&mut parser, false, &mut allowance) {
                Progress::Complete(result) => {
                    assert_eq!(result, Attempt::Committed);
                    break;
                }
                Progress::NeedsProcessing => {}
                _ => panic!("the complete non-name and lookahead fix the missing-name recovery"),
            }
        }
        assert_eq!(parser.offset(), TextSize(1));
        wrapper.complete(&mut parser, SyntaxKind::Document);
        let output = parser.finish();
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(
            output.diagnostics[0].diagnostic.code.as_str(),
            "syntax/missing-module-import-intrinsic-name"
        );
    }

    #[test]
    fn module_import_choices_commitment_and_recovery_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::MODULE_IMPORT_NAME_SEGMENT, "math"),
            (rules::MODULE_IMPORT_NAME_SEGMENT, "💡\u{301}"),
            (rules::MODULE_IMPORT_INTRINSIC_SEGMENT, "_math"),
            (rules::MODULE_IMPORT_INTRINSIC_SEGMENT, "_"),
            (rules::MODULE_IMPORT_INTRINSIC_SEGMENT, "_!"),
            (rules::MODULE_IMPORT_PATH_SEGMENT, "_math"),
            (rules::MODULE_IMPORT_PATH_SEGMENT, "math"),
            (rules::MODULE_IMPORT_PATH, "math/trig/_sin"),
            (rules::MODULE_IMPORT_PATH, "math/trig/_"),
            (rules::MODULE_IMPORT_PATH, "math/"),
            (rules::MODULE_IMPORT_ALIAS_SEGMENT, "alias"),
            (rules::MODULE_IMPORT_ALIAS_PATH, "alias/path"),
            (rules::MODULE_IMPORT_ALIAS_PATH, "alias/"),
            (rules::MODULE_IMPORT_VALUE_ALIAS, "alias/path"),
            (rules::CONTEXT_IMPORT_ALIAS_SEGMENT, "ctx-2"),
            (rules::CONTEXT_IMPORT_ALIAS_SEGMENT, "é\u{301}"),
            (rules::MODULE_IMPORT_CONTEXT_ALIAS, "@ctx"),
            (rules::MODULE_IMPORT_CONTEXT_ALIAS, "@ctx/path"),
            (rules::MODULE_IMPORT_CONTEXT_ALIAS, "@"),
            (rules::MODULE_IMPORT_ALIAS, "@ctx"),
            (rules::MODULE_IMPORT_ALIAS, "alias/path"),
            (rules::MODULE_ROOT, "math"),
            (rules::IMPORT_ALIAS_OPERATOR, " :=\t"),
            (rules::IMPORT_ALIAS_OPERATOR, ":="),
            (rules::IMPORT_ALIAS_OPERATOR, " :"),
            (rules::IMPORT_ALIAS_OPERATOR, "\n:="),
            (rules::IMPORT_GROUP_SEPARATOR, ","),
            (rules::IMPORT_GROUP_SEPARATOR, "\r\n"),
            (rules::IMPORT_GROUP_ITEM, "trig/sin"),
            (rules::IMPORT_GROUP_ITEMS, " sin, cos\t"),
            (rules::IMPORT_GROUP_ITEMS, "sin,"),
            (rules::IMPORT_GROUP_ITEMS, "sin _"),
            (rules::ALIASED_ITEM_IMPORT, "alias := math/sin"),
            (rules::ALIASED_ITEM_IMPORT, "alias"),
            (rules::ALIASED_ITEM_IMPORT, "alias :="),
            (rules::ALIASED_ITEM_IMPORT, "alias := math"),
            (rules::ALIASED_ITEM_IMPORT, "alias := math/"),
            (rules::MODULE_SUFFIX_IMPORT, "math/*"),
            (rules::MODULE_SUFFIX_IMPORT, "math/sin"),
            (rules::MODULE_SUFFIX_IMPORT, "math/"),
            (rules::MODULE_SUFFIX_IMPORT, "math/{sin,cos}"),
            (rules::MODULE_SUFFIX_IMPORT, "math/{sin,}"),
            (rules::MODULE_SUFFIX_IMPORT, "math/{sin\r\ncos}"),
            (rules::MODULE_SUFFIX_IMPORT, "math/{}"),
            (rules::MODULE_SUFFIX_IMPORT, "math/{"),
            (rules::MODULE_SUFFIX_IMPORT, "math/{_}"),
            (rules::MODULE_ONLY_IMPORT, "math"),
            (rules::MODULE_ONLY_IMPORT, "math/"),
            (rules::MODULE_IMPORT, "+> math"),
            (rules::MODULE_IMPORT, "+>alias := math/sin"),
            (rules::MODULE_IMPORT, "+> @ctx := math/sin"),
            (rules::MODULE_IMPORT, "+> @ctx/path;next"),
            (rules::MODULE_IMPORT, "+> @ctx/💡\u{301}\r\nnext"),
            (rules::MODULE_IMPORT, "+> @ctx"),
            (rules::MODULE_IMPORT, "+> @ctx :"),
            (rules::MODULE_IMPORT, "+> @ctx :="),
            (rules::MODULE_IMPORT, "+> @"),
            (rules::MODULE_IMPORT, "+> math/{sin,}"),
            (rules::MODULE_IMPORT, "+> ./dep.mec"),
        ]);
    }
    #[test]
    fn module_import_lists_aliases_and_shared_skip_retain_linear_work() {
        for (rule, prefix, unit, tail, result, diagnostics, remainder) in [
            (
                rules::MODULE_IMPORT_PATH,
                "",
                "name/",
                "_sin",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::MODULE_IMPORT_PATH,
                "",
                "name/",
                "",
                Attempt::Matched,
                0,
                1,
            ),
            (
                rules::MODULE_IMPORT_ALIAS_PATH,
                "",
                "name/",
                "alias",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
                "a",
                "-a2",
                "",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::IMPORT_ALIAS_OPERATOR,
                "",
                " ",
                ":=",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::IMPORT_GROUP_ITEMS,
                "",
                "sin,",
                "cos",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::MODULE_IMPORT,
                "+> math/{",
                "sin,",
                "cos}",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::MODULE_IMPORT,
                "+> math/{",
                "sin ",
                "_}",
                Attempt::Committed,
                1,
                0,
            ),
            (
                rules::MODULE_IMPORT,
                "+> ",
                "a",
                " := math/sin",
                Attempt::Matched,
                0,
                0,
            ),
            (
                rules::MODULE_IMPORT,
                "+> @ctx/",
                "a",
                ";next",
                Attempt::Committed,
                1,
                5,
            ),
            (
                rules::MODULE_IMPORT,
                "+> @ctx/",
                " ",
                "x",
                Attempt::Committed,
                1,
                0,
            ),
        ] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(prefix) + &unit.repeat(n) + tail;
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, work) = run::<Continuation>(rule, &text, &chunks, u64::MAX, true);
                let (expected, one_shot_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected, "{rule:?}, {tail:?}");
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.stats.diagnostics_emitted, diagnostics);
                assert_eq!(observed.result, result);
                assert_eq!(observed.end.to_usize(), text.len() - remainder);
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(
                        work <= prior_streamed * 3,
                        "module import restarted {rule:?}"
                    );
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
