//! Typed syntax views for the closed Phase 2C literal and number productions.

use alloc::vec::Vec;

use crate::document::red::IdentifierSyntax;
use crate::document::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

fn child<N: AstNode>(syntax: &SyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}

fn children<N: AstNode>(syntax: &SyntaxNode) -> Vec<N> {
    syntax.children().filter_map(N::cast).collect()
}

fn nth_child<N: AstNode>(syntax: &SyntaxNode, index: usize) -> Option<N> {
    syntax.children().filter_map(N::cast).nth(index)
}

fn numeric_child(syntax: &SyntaxNode, index: usize) -> Option<SyntaxNode> {
    syntax
        .children()
        .filter(|child| {
            matches!(
                child.kind(),
                SyntaxKind::HexadecimalLiteral
                    | SyntaxKind::DecimalLiteral
                    | SyntaxKind::OctalLiteral
                    | SyntaxKind::BinaryLiteral
                    | SyntaxKind::ScientificLiteral
                    | SyntaxKind::RationalLiteral
                    | SyntaxKind::FloatLiteral
                    | SyntaxKind::IntegerLiteral
                    | SyntaxKind::UntypedInteger
            )
        })
        .nth(index)
}

macro_rules! literal_ast_node {
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

literal_ast_node!(EmptyLiteralSyntax, EmptyLiteral);
literal_ast_node!(AtomLiteralSyntax, AtomLiteral);
literal_ast_node!(StringLiteralSyntax, StringLiteral);
literal_ast_node!(Utf8StringSyntax, Utf8String);
literal_ast_node!(RawStringSyntax, RawString);
literal_ast_node!(NumberSyntax, Number);
literal_ast_node!(ComplexNumberSyntax, ComplexNumber);
literal_ast_node!(RealNumberSyntax, RealNumber);
literal_ast_node!(UntypedRealNumberSyntax, UntypedRealNumber);
literal_ast_node!(RationalLiteralSyntax, RationalLiteral);
literal_ast_node!(ScientificLiteralSyntax, ScientificLiteral);
literal_ast_node!(FloatDecimalStartSyntax, FloatDecimalStart);
literal_ast_node!(FloatFullSyntax, FloatFull);
literal_ast_node!(FloatLiteralSyntax, FloatLiteral);
literal_ast_node!(IntegerLiteralSyntax, IntegerLiteral);
literal_ast_node!(TypedIntegerSyntax, TypedInteger);
literal_ast_node!(UntypedIntegerSyntax, UntypedInteger);
literal_ast_node!(DecimalLiteralSyntax, DecimalLiteral);
literal_ast_node!(HexadecimalLiteralSyntax, HexadecimalLiteral);
literal_ast_node!(OctalLiteralSyntax, OctalLiteral);
literal_ast_node!(BinaryLiteralSyntax, BinaryLiteral);

/// `digit-sequence` was introduced by Phase 2A and gains this typed view so
/// literal accessors need not expose untyped child-node searches.
literal_ast_node!(DigitSequenceSyntax, DigitSequence);

impl EmptyLiteralSyntax {
    pub fn underscores(&self) -> Vec<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::Underscore)
            .collect()
    }
}

impl AtomLiteralSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
}

impl StringLiteralSyntax {
    pub fn utf8(&self) -> Option<Utf8StringSyntax> {
        child(&self.0)
    }

    pub fn raw(&self) -> Option<RawStringSyntax> {
        child(&self.0)
    }
}

impl Utf8StringSyntax {
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
}

impl RawStringSyntax {
    pub fn quote_tokens(&self) -> Vec<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::Quote)
            .collect()
    }
}

impl NumberSyntax {
    pub fn complex(&self) -> Option<ComplexNumberSyntax> {
        child(&self.0)
    }

    pub fn real(&self) -> Option<RealNumberSyntax> {
        child(&self.0)
    }
}

impl ComplexNumberSyntax {
    pub fn components(&self) -> Vec<UntypedRealNumberSyntax> {
        children(&self.0)
    }

    pub fn real(&self) -> Option<UntypedRealNumberSyntax> {
        let values = self.components();
        (values.len() == 2).then(|| values[0].clone())
    }

    pub fn imaginary(&self) -> Option<UntypedRealNumberSyntax> {
        let values = self.components();
        match values.as_slice() {
            [imaginary] | [_, imaginary] => Some(imaginary.clone()),
            _ => None,
        }
    }
}

impl RealNumberSyntax {
    pub fn value(&self) -> Option<SyntaxNode> {
        numeric_child(&self.0, 0)
    }

    pub fn is_negated(&self) -> bool {
        matches!(
            self.0.children_with_tokens().first(),
            Some(SyntaxElement::Token(token)) if token.kind() == SyntaxKind::Dash
        )
    }
}

impl UntypedRealNumberSyntax {
    pub fn value(&self) -> Option<SyntaxNode> {
        numeric_child(&self.0, 0)
    }

    pub fn is_negated(&self) -> bool {
        matches!(
            self.0.children_with_tokens().first(),
            Some(SyntaxElement::Token(token)) if token.kind() == SyntaxKind::Dash
        )
    }
}

impl RationalLiteralSyntax {
    pub fn numerator(&self) -> Option<IntegerLiteralSyntax> {
        nth_child(&self.0, 0)
    }

    pub fn denominator(&self) -> Option<IntegerLiteralSyntax> {
        nth_child(&self.0, 1)
    }
}

impl ScientificLiteralSyntax {
    pub fn base(&self) -> Option<SyntaxNode> {
        self.numeric_parts().into_iter().next()
    }

    pub fn exponent(&self) -> Option<SyntaxNode> {
        self.numeric_parts().into_iter().nth(1)
    }

    fn numeric_parts(&self) -> Vec<SyntaxNode> {
        self.0
            .children()
            .filter(|child| {
                matches!(
                    child.kind(),
                    SyntaxKind::FloatLiteral | SyntaxKind::IntegerLiteral
                )
            })
            .collect()
    }
}

impl FloatDecimalStartSyntax {
    pub fn part(&self) -> Option<DigitSequenceSyntax> {
        child(&self.0)
    }
}

impl FloatFullSyntax {
    pub fn whole(&self) -> Option<DigitSequenceSyntax> {
        nth_child(&self.0, 0)
    }

    pub fn part(&self) -> Option<DigitSequenceSyntax> {
        nth_child(&self.0, 1)
    }
}

impl FloatLiteralSyntax {
    pub fn decimal_start(&self) -> Option<FloatDecimalStartSyntax> {
        child(&self.0)
    }

    pub fn full(&self) -> Option<FloatFullSyntax> {
        child(&self.0)
    }
}

impl IntegerLiteralSyntax {
    pub fn typed(&self) -> Option<TypedIntegerSyntax> {
        child(&self.0)
    }

    pub fn untyped(&self) -> Option<UntypedIntegerSyntax> {
        child(&self.0)
    }
}

impl TypedIntegerSyntax {
    pub fn digits(&self) -> Option<DigitSequenceSyntax> {
        child(&self.0)
    }

    pub fn suffix(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
}

impl UntypedIntegerSyntax {
    pub fn digits(&self) -> Option<DigitSequenceSyntax> {
        child(&self.0)
    }
}

macro_rules! based_literal_view {
    ($name:ident) => {
        impl $name {
            pub fn digits(&self) -> Option<DigitSequenceSyntax> {
                child(&self.0)
            }
        }
    };
}

based_literal_view!(DecimalLiteralSyntax);
based_literal_view!(OctalLiteralSyntax);
based_literal_view!(BinaryLiteralSyntax);

impl HexadecimalLiteralSyntax {
    pub fn payload_tokens(&self) -> Vec<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .filter(|token| token.kind() != SyntaxKind::Text)
            .collect()
    }
}

impl DigitSequenceSyntax {
    pub fn tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}
