use alloc::vec::Vec;

use crate::document::{AstNode, IdentifierSyntax, SyntaxKind, SyntaxNode, SyntaxToken};

use super::{AnyCallArgumentSyntax, PatternSyntax, child, children, direct_token};

recursive_ast_node!(FsmArgumentsSyntax, FsmArguments);
recursive_ast_node!(FsmAsyncTransitionSyntax, FsmAsyncTransition);
recursive_ast_node!(FsmInstanceSyntax, FsmInstance);
recursive_ast_node!(FsmOutputSyntax, FsmOutput);
recursive_ast_node!(FsmPipeSyntax, FsmPipe);
recursive_ast_node!(FsmStateTransitionSyntax, FsmStateTransition);
recursive_ast_node!(FsmValueSyntax, FsmValue);

#[derive(Clone, Debug)]
pub enum FsmStageSyntax {
    State(FsmStateTransitionSyntax),
    Async(FsmAsyncTransitionSyntax),
    Output(FsmOutputSyntax),
}

impl AstNode for FsmStageSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::FsmStateTransition | SyntaxKind::FsmAsyncTransition | SyntaxKind::FsmOutput
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::FsmStateTransition => {
                FsmStateTransitionSyntax::cast(syntax).map(Self::State)
            }
            SyntaxKind::FsmAsyncTransition => {
                FsmAsyncTransitionSyntax::cast(syntax).map(Self::Async)
            }
            SyntaxKind::FsmOutput => FsmOutputSyntax::cast(syntax).map(Self::Output),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::State(value) => value.syntax(),
            Self::Async(value) => value.syntax(),
            Self::Output(value) => value.syntax(),
        }
    }
}

impl FsmPipeSyntax {
    pub fn instance(&self) -> Option<FsmInstanceSyntax> {
        child(&self.0)
    }
    pub fn stages(&self) -> Vec<FsmStageSyntax> {
        children(&self.0)
    }
}

impl FsmInstanceSyntax {
    pub fn hash(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::HashTag, 0)
    }
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
    pub fn arguments(&self) -> Option<FsmArgumentsSyntax> {
        child(&self.0)
    }
}

impl FsmArgumentsSyntax {
    pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftParen, 0)
    }
    pub fn arguments(&self) -> Vec<AnyCallArgumentSyntax> {
        children(&self.0)
    }
    pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightParen, 0)
    }
}

impl FsmValueSyntax {
    pub fn pattern(&self) -> Option<PatternSyntax> {
        child(&self.0)
    }
}

macro_rules! transition_accessors {
    ($name:ident, $operator:ident) => {
        impl $name {
            pub fn operator(&self) -> Option<SyntaxToken> {
                direct_token(&self.0, SyntaxKind::$operator, 0)
            }
            pub fn value(&self) -> Option<FsmValueSyntax> {
                child(&self.0)
            }
        }
    };
}

transition_accessors!(FsmStateTransitionSyntax, TransitionOperator);
transition_accessors!(FsmAsyncTransitionSyntax, AsyncTransitionOperator);
transition_accessors!(FsmOutputSyntax, OutputOperator);
