use alloc::vec::Vec;

use crate::document::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken, TokenFlags};

fn child<N: AstNode>(syntax: &SyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}

fn children<N: AstNode>(syntax: &SyntaxNode) -> Vec<N> {
    syntax.children().filter_map(N::cast).collect()
}

fn nth_child<N: AstNode>(syntax: &SyntaxNode, index: usize) -> Option<N> {
    syntax.children().filter_map(N::cast).nth(index)
}

macro_rules! grammar_ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                Self::can_cast(syntax.kind()).then_some(Self(syntax))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

grammar_ast_node!(GrammarDocumentSyntax, GrammarDocument);
grammar_ast_node!(GrammarSyntax, Grammar);
grammar_ast_node!(GrammarRuleSyntax, GrammarRule);
grammar_ast_node!(GrammarIdentifierSyntax, GrammarIdentifier);
grammar_ast_node!(GrammarExpressionSyntax, GrammarExpression);
grammar_ast_node!(GrammarTermSyntax, GrammarTerm);
grammar_ast_node!(GrammarFactorSyntax, GrammarFactor);
grammar_ast_node!(GrammarDefinitionSyntax, GrammarDefinition);
grammar_ast_node!(GrammarRepeat0Syntax, GrammarRepeat0);
grammar_ast_node!(GrammarRepeat1Syntax, GrammarRepeat1);
grammar_ast_node!(GrammarOptionalSyntax, GrammarOptional);
grammar_ast_node!(GrammarPeekSyntax, GrammarPeek);
grammar_ast_node!(GrammarNotSyntax, GrammarNot);
grammar_ast_node!(GrammarListSyntax, GrammarList);
grammar_ast_node!(GrammarRangeSyntax, GrammarRange);
grammar_ast_node!(GrammarGroupSyntax, GrammarGroup);
grammar_ast_node!(GrammarTerminalSyntax, GrammarTerminal);
grammar_ast_node!(GrammarTerminalTokenSyntax, GrammarTerminalToken);

impl GrammarDocumentSyntax {
    pub fn grammar(&self) -> Option<GrammarSyntax> {
        child(&self.0)
    }
}

impl GrammarSyntax {
    pub fn first_rule(&self) -> Option<GrammarRuleSyntax> {
        child(&self.0)
    }

    pub fn rules(&self) -> Vec<GrammarRuleSyntax> {
        children(&self.0)
    }
}

impl GrammarRuleSyntax {
    pub fn name(&self) -> Option<GrammarIdentifierSyntax> {
        child(&self.0)
    }

    pub fn expression(&self) -> Option<GrammarExpressionSyntax> {
        child(&self.0)
    }
}

impl GrammarIdentifierSyntax {
    pub fn name_token(&self) -> Option<SyntaxToken> {
        self.0.tokens().into_iter().find(|token| {
            !token
                .flags()
                .intersects(TokenFlags::TRIVIA | TokenFlags::SYNTHETIC)
        })
    }

    pub fn name_tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl GrammarExpressionSyntax {
    pub fn first_term(&self) -> Option<GrammarTermSyntax> {
        child(&self.0)
    }

    pub fn terms(&self) -> Vec<GrammarTermSyntax> {
        children(&self.0)
    }
}

impl GrammarTermSyntax {
    pub fn first_factor(&self) -> Option<GrammarFactorSyntax> {
        child(&self.0)
    }

    pub fn factors(&self) -> Vec<GrammarFactorSyntax> {
        children(&self.0)
    }
}

impl GrammarFactorSyntax {
    pub fn definition(&self) -> Option<GrammarDefinitionSyntax> {
        child(&self.0)
    }

    pub fn repeat0(&self) -> Option<GrammarRepeat0Syntax> {
        child(&self.0)
    }

    pub fn repeat1(&self) -> Option<GrammarRepeat1Syntax> {
        child(&self.0)
    }

    pub fn optional(&self) -> Option<GrammarOptionalSyntax> {
        child(&self.0)
    }

    pub fn peek(&self) -> Option<GrammarPeekSyntax> {
        child(&self.0)
    }

    pub fn not(&self) -> Option<GrammarNotSyntax> {
        child(&self.0)
    }

    pub fn list(&self) -> Option<GrammarListSyntax> {
        child(&self.0)
    }

    pub fn range(&self) -> Option<GrammarRangeSyntax> {
        child(&self.0)
    }

    pub fn group(&self) -> Option<GrammarGroupSyntax> {
        child(&self.0)
    }

    pub fn terminal(&self) -> Option<GrammarTerminalSyntax> {
        child(&self.0)
    }
}

macro_rules! unary_factor_view {
    ($name:ident) => {
        impl $name {
            pub fn factor(&self) -> Option<GrammarFactorSyntax> {
                child(&self.0)
            }
        }
    };
}

impl GrammarDefinitionSyntax {
    pub fn identifier(&self) -> Option<GrammarIdentifierSyntax> {
        child(&self.0)
    }
}

unary_factor_view!(GrammarRepeat0Syntax);
unary_factor_view!(GrammarRepeat1Syntax);
unary_factor_view!(GrammarOptionalSyntax);
unary_factor_view!(GrammarPeekSyntax);
unary_factor_view!(GrammarNotSyntax);

impl GrammarListSyntax {
    pub fn first_factor(&self) -> Option<GrammarFactorSyntax> {
        nth_child(&self.0, 0)
    }

    pub fn second_factor(&self) -> Option<GrammarFactorSyntax> {
        nth_child(&self.0, 1)
    }

    pub fn factors(&self) -> Vec<GrammarFactorSyntax> {
        children(&self.0)
    }
}

impl GrammarRangeSyntax {
    pub fn start(&self) -> Option<GrammarTerminalTokenSyntax> {
        nth_child(&self.0, 0)
    }

    pub fn end(&self) -> Option<GrammarTerminalTokenSyntax> {
        nth_child(&self.0, 1)
    }

    pub fn terminal_tokens(&self) -> Vec<GrammarTerminalTokenSyntax> {
        children(&self.0)
    }
}

impl GrammarGroupSyntax {
    pub fn expression(&self) -> Option<GrammarExpressionSyntax> {
        child(&self.0)
    }
}

impl GrammarTerminalSyntax {
    pub fn terminal_token(&self) -> Option<GrammarTerminalTokenSyntax> {
        child(&self.0)
    }
}

impl GrammarTerminalTokenSyntax {
    pub fn opening_quote(&self) -> Option<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Quote)
    }

    pub fn closing_quote(&self) -> Option<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::Quote)
            .nth(1)
    }

    pub fn content_tokens(&self) -> Vec<SyntaxToken> {
        let tokens = self.0.tokens();
        let Some(opening) = tokens
            .iter()
            .position(|token| token.kind() == SyntaxKind::Quote)
        else {
            return Vec::new();
        };
        let Some(closing) = tokens
            .iter()
            .rposition(|token| token.kind() == SyntaxKind::Quote)
        else {
            return Vec::new();
        };
        if opening >= closing {
            return Vec::new();
        }
        tokens[opening + 1..closing].to_vec()
    }
}
