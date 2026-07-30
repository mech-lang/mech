use alloc::vec::Vec;

use crate::document::red::{AstNode, IdentifierSyntax, SyntaxNode, SyntaxToken};
use crate::document::SyntaxKind;

macro_rules! primitive_kind_ast_node {
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

primitive_kind_ast_node!(KindAnySyntax, KindAny);
primitive_kind_ast_node!(KindEmptySyntax, KindEmpty);
primitive_kind_ast_node!(KindAtomSyntax, KindAtom);

impl KindAnySyntax {
    pub fn asterisk(&self) -> Option<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Asterisk)
    }
}

impl KindEmptySyntax {
    pub fn underscores(&self) -> Vec<SyntaxToken> {
        self.0
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::Underscore)
            .collect()
    }
}

impl KindAtomSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        self.0.children().find_map(IdentifierSyntax::cast)
    }
}
