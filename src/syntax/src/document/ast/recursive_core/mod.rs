//! Typed, zero-copy red-tree views for the frozen Phase 2I recursive grammar.
//!
//! Each view owns only a cheap [`SyntaxNode`] handle. Accessors retain the
//! parser's physical child order and expose tokens directly, so source tools do
//! not need to construct a parallel tree or copy source text.

use alloc::vec::Vec;

use crate::document::{
    AstNode, MissingSyntax, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TokenFlags,
};

macro_rules! recursive_ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) crate::document::SyntaxNode);

        impl crate::document::AstNode for $name {
            fn can_cast(kind: crate::document::SyntaxKind) -> bool {
                kind == crate::document::SyntaxKind::$kind
            }

            fn cast(syntax: crate::document::SyntaxNode) -> Option<Self> {
                Self::can_cast(syntax.kind()).then_some(Self(syntax))
            }

            fn syntax(&self) -> &crate::document::SyntaxNode {
                &self.0
            }
        }
    };
}

pub mod calls;
pub mod comprehensions;
pub mod expressions;
pub mod fsm;
pub mod kinds;
pub mod literals;
pub mod patterns;
pub mod precedence;
pub mod structures;
pub mod subscripts;
pub mod variables;

pub use calls::*;
pub use comprehensions::*;
pub use expressions::*;
pub use fsm::*;
pub use kinds::*;
pub use literals::*;
pub use patterns::*;
pub use precedence::*;
pub use structures::*;
pub use subscripts::*;
pub use variables::*;

recursive_ast_node!(ErrorSyntax, Error);

pub(super) fn child<N: AstNode>(syntax: &SyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}

pub(super) fn nth_child<N: AstNode>(syntax: &SyntaxNode, index: usize) -> Option<N> {
    syntax.children().filter_map(N::cast).nth(index)
}

pub(super) fn children<N: AstNode>(syntax: &SyntaxNode) -> Vec<N> {
    syntax.children().filter_map(N::cast).collect()
}

pub(super) fn direct_token(
    syntax: &SyntaxNode,
    kind: SyntaxKind,
    index: usize,
) -> Option<SyntaxToken> {
    let mut matched = 0;
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) if token.kind() == kind => {
                if matched == index {
                    return Some(token);
                }
                matched += 1;
            }
            SyntaxElement::Node(node) if node.kind() == SyntaxKind::Missing => {
                for token in node
                    .tokens()
                    .into_iter()
                    .filter(|token| token.kind() == kind)
                {
                    if matched == index {
                        return Some(token);
                    }
                    matched += 1;
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn direct_tokens(syntax: &SyntaxNode) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) => tokens.push(token),
            SyntaxElement::Node(node) if node.kind() == SyntaxKind::Missing => {
                tokens.extend(node.tokens());
            }
            SyntaxElement::Node(_) => {}
        }
    }
    tokens
}

fn collect_nodes<N: AstNode>(syntax: &SyntaxNode, output: &mut Vec<N>) {
    for child in syntax.children() {
        if let Some(node) = N::cast(child.clone()) {
            output.push(node);
        }
        collect_nodes(&child, output);
    }
}

/// Common source-preserving access available on every typed syntax view.
pub trait RecursiveSyntaxNode: AstNode {
    /// Return the first direct child accepted by a typed view.
    fn child<N: AstNode>(&self) -> Option<N> {
        child(self.syntax())
    }

    /// Return direct typed children in physical source order.
    fn children<N: AstNode>(&self) -> Vec<N> {
        children(self.syntax())
    }

    /// Return direct tokens, including trivia and synthetic recovery tokens.
    fn direct_tokens(&self) -> Vec<SyntaxToken> {
        direct_tokens(self.syntax())
    }

    /// Return every physical token below this node in source order.
    fn physical_tokens(&self) -> Vec<SyntaxToken> {
        self.syntax().tokens()
    }

    /// Return trivia tokens without copying their source text.
    fn trivia_tokens(&self) -> Vec<SyntaxToken> {
        self.physical_tokens()
            .into_iter()
            .filter(|token| {
                token.flags().contains(TokenFlags::TRIVIA)
                    || matches!(
                        token.kind(),
                        SyntaxKind::Whitespace
                            | SyntaxKind::Newline
                            | SyntaxKind::Tab
                            | SyntaxKind::CarriageReturn
                    )
            })
            .collect()
    }

    /// Return synthetic missing tokens inserted by recovery.
    fn missing_tokens(&self) -> Vec<SyntaxToken> {
        self.physical_tokens()
            .into_iter()
            .filter(|token| token.flags().contains(TokenFlags::MISSING))
            .collect()
    }

    /// Return missing-production nodes in source order.
    fn missing_nodes(&self) -> Vec<MissingSyntax> {
        let mut nodes = Vec::new();
        collect_nodes(self.syntax(), &mut nodes);
        nodes
    }

    /// Return error nodes in source order.
    fn error_nodes(&self) -> Vec<ErrorSyntax> {
        let mut nodes = Vec::new();
        collect_nodes(self.syntax(), &mut nodes);
        nodes
    }
}

impl<N: AstNode> RecursiveSyntaxNode for N {}

macro_rules! phase_2i_nodes {
    ($(($variant:ident, $view:ty, $kind:ident, $rule:literal)),+ $(,)?) => {
        /// A closed typed view over every node-valued Phase 2I schema row.
        #[derive(Clone, Debug)]
        pub enum RecursiveCoreSyntax {
            $($variant($view)),+
        }

        impl AstNode for RecursiveCoreSyntax {
            fn can_cast(kind: SyntaxKind) -> bool {
                matches!(kind, $(SyntaxKind::$kind)|+)
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                match syntax.kind() {
                    $(SyntaxKind::$kind => <$view>::cast(syntax).map(Self::$variant),)+
                    _ => None,
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                match self {
                    $(Self::$variant(view) => view.syntax(),)+
                }
            }
        }

        /// Resolve a node-valued schema row to its sole physical syntax kind.
        pub fn phase_2i_node_kind(rule_name: &str) -> Option<SyntaxKind> {
            match rule_name {
                $($rule => Some(SyntaxKind::$kind),)+
                _ => None,
            }
        }
    };
}

phase_2i_nodes! {
    (ArgumentList, ArgumentListSyntax, ArgumentList, "argument-list"),
    (RecordBinding, RecordBindingSyntax, RecordBinding, "binding"),
    (BraceSubscript, BraceSubscriptSyntax, BraceSubscript, "brace-subscript"),
    (BracketSubscript, BracketSubscriptSyntax, BracketSubscript, "bracket-subscript"),
    (CallArgument, CallArgumentSyntax, CallArgument, "call-arg"),
    (BoundCallArgument, BoundCallArgumentSyntax, BoundCallArgument, "call-arg-with-binding"),
    (ComprehensionQualifier, ComprehensionQualifierSyntax, ComprehensionQualifier, "comprehension-qualifier"),
    (Expression, crate::document::ExpressionSyntax, Expression, "expression"),
    (Factor, FactorSyntax, Factor, "factor"),
    (FancyTable, FancyTableSyntax, FancyTable, "fancy-table"),
    (FancyTableHeader, FancyTableHeaderSyntax, FancyTableHeader, "fancy-table-header"),
    (TableField, TableFieldSyntax, TableField, "field"),
    (FormulaSubscript, FormulaSubscriptSyntax, FormulaSubscript, "formula-subscript"),
    (FsmArguments, FsmArgumentsSyntax, FsmArguments, "fsm-args"),
    (FsmAsyncTransition, FsmAsyncTransitionSyntax, FsmAsyncTransition, "fsm-async-transition"),
    (FsmInstance, FsmInstanceSyntax, FsmInstance, "fsm-instance"),
    (FsmOutput, FsmOutputSyntax, FsmOutput, "fsm-output"),
    (FsmPipe, FsmPipeSyntax, FsmPipe, "fsm-pipe"),
    (FsmStateTransition, FsmStateTransitionSyntax, FsmStateTransition, "fsm-state-transition"),
    (FsmValue, FsmValueSyntax, FsmValue, "fsm-value"),
    (FunctionCall, FunctionCallSyntax, FunctionCall, "function-call"),
    (Generator, GeneratorSyntax, Generator, "generator"),
    (HeaderField, HeaderFieldSyntax, HeaderField, "header-field"),
    (InlineTable, InlineTableSyntax, InlineTable, "inline-table"),
    (InlineTableHeader, InlineTableHeaderSyntax, InlineTableHeader, "inline-table-header"),
    (InlineTableRow, InlineTableRowSyntax, InlineTableRow, "inline-table-row"),
    (Kind, KindSyntax, Kind, "kind"),
    (KindAnnotation, KindAnnotationSyntax, KindAnnotation, "kind-annotation"),
    (KindKind, KindKindSyntax, KindKind, "kind-kind"),
    (KindMap, KindMapSyntax, KindMap, "kind-map"),
    (KindMatrix, KindMatrixSyntax, KindMatrix, "kind-matrix"),
    (KindRecord, KindRecordSyntax, KindRecord, "kind-record"),
    (KindScalar, KindScalarSyntax, KindScalar, "kind-scalar"),
    (KindSet, KindSetSyntax, KindSet, "kind-set"),
    (TableKind, TableKindSyntax, TableKind, "kind-table"),
    (KindTuple, KindTupleSyntax, KindTuple, "kind-tuple"),
    (KindWithOption, KindWithOptionSyntax, KindWithOption, "kind-with-option"),
    (LogicExpression, LogicExpressionSyntax, LogicExpression, "l1"),
    (ComparisonExpression, ComparisonExpressionSyntax, ComparisonExpression, "l2"),
    (AdditiveExpression, AdditiveExpressionSyntax, AdditiveExpression, "l3"),
    (MultiplicativeExpression, MultiplicativeExpressionSyntax, MultiplicativeExpression, "l4"),
    (PowerExpression, PowerExpressionSyntax, PowerExpression, "l5"),
    (TableExpression, TableExpressionSyntax, TableExpression, "l6"),
    (SetExpression, SetExpressionSyntax, SetExpression, "l7"),
    (Literal, LiteralSyntax, Literal, "literal"),
    (Map, MapSyntax, Map, "map"),
    (MapEntry, MapEntrySyntax, MapEntry, "mapping"),
    (MatchArm, MatchArmSyntax, MatchArm, "match-arm"),
    (Matrix, MatrixSyntax, Matrix, "matrix"),
    (MatrixColumn, MatrixColumnSyntax, MatrixColumn, "matrix-column"),
    (MatrixComprehension, MatrixComprehensionSyntax, MatrixComprehension, "matrix-comprehension"),
    (MatrixRow, MatrixRowSyntax, MatrixRow, "matrix-row"),
    (NegateFactor, NegateFactorSyntax, NegateFactor, "negate-factor"),
    (NotFactor, NotFactorSyntax, NotFactor, "not-factor"),
    (ParentheticalExpression, ParentheticalExpressionSyntax, ParentheticalExpression, "parenthetical-term"),
    (Pattern, PatternSyntax, Pattern, "pattern"),
    (ArrayPattern, ArrayPatternSyntax, ArrayPattern, "pattern-array"),
    (ArrayPatternElement, ArrayPatternElementSyntax, ArrayPatternElement, "pattern-array-token"),
    (AtomStructPattern, AtomStructPatternSyntax, AtomStructPattern, "pattern-atom-struct"),
    (TuplePattern, TuplePatternSyntax, TuplePattern, "pattern-tuple"),
    (TupleStructPattern, TupleStructPatternSyntax, TupleStructPattern, "pattern-tuple-struct"),
    (RangeExpression, RangeExpressionSyntax, RangeExpression, "range-expression"),
    (RangeSubscript, RangeSubscriptSyntax, RangeSubscript, "range-subscript"),
    (Record, RecordSyntax, Record, "record"),
    (RegularTable, RegularTableSyntax, RegularTable, "regular-table"),
    (Set, SetSyntax, Set, "set"),
    (SetComprehension, SetComprehensionSyntax, SetComprehension, "set-comprehension"),
    (Slice, SliceSyntax, Slice, "slice"),
    (Structure, StructureSyntax, Structure, "structure"),
    (SubscriptList, SubscriptListSyntax, SubscriptList, "subscript"),
    (Table, TableSyntax, Table, "table"),
    (TableHeader, TableHeaderSyntax, TableHeader, "table-header"),
    (TableRow, TableRowSyntax, TableRow, "table-row"),
    (FancyTableRow, FancyTableRowSyntax, FancyTableRow, "table-row2"),
    (Tuple, TupleSyntax, Tuple, "tuple"),
    (TupleStruct, TupleStructSyntax, TupleStruct, "tuple-struct"),
    (Variable, VariableSyntax, Variable, "var"),
    (VariableDefine, crate::document::VariableDefineSyntax, VariableDefine, "variable-define"),
}

/// The two Phase 2I productions that intentionally emit no wrapper node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase2iTransparentAlias {
    Formula,
    PatternArrayItem,
}

pub fn phase_2i_transparent_alias(rule_name: &str) -> Option<Phase2iTransparentAlias> {
    match rule_name {
        "formula" => Some(Phase2iTransparentAlias::Formula),
        "pattern-array-item" => Some(Phase2iTransparentAlias::PatternArrayItem),
        _ => None,
    }
}
