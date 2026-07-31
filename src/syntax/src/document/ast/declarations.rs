//! Typed syntax views for the closed Phase 2F declaration productions.

use alloc::vec::Vec;

use crate::document::{
    AstNode, IdentifierSyntax, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken,
};

macro_rules! declaration_ast_node {
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

declaration_ast_node!(ExportDeclarationSyntax, ExportDeclaration);
declaration_ast_node!(ContextDeclarationSyntax, ContextDeclaration);
declaration_ast_node!(ContextBaseContextSyntax, ContextBaseContext);
declaration_ast_node!(ContextBaseResourceUriSyntax, ContextBaseResourceUri);
declaration_ast_node!(
    ContextCapabilityDeclarationSyntax,
    ContextCapabilityDeclaration
);
declaration_ast_node!(ContextCapabilityPathSyntax, ContextCapabilityPath);
declaration_ast_node!(ContextCapabilityScopeSyntax, ContextCapabilityScope);

impl ExportDeclarationSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub enum CanonicalContextBaseSyntax {
    ResourceUri(ContextBaseResourceUriSyntax),
    Context(ContextBaseContextSyntax),
}

impl CanonicalContextBaseSyntax {
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::ResourceUri(syntax) => syntax.syntax(),
            Self::Context(syntax) => syntax.syntax(),
        }
    }
}

impl ContextDeclarationSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }

    pub fn base(&self) -> Option<CanonicalContextBaseSyntax> {
        self.0.children().find_map(|child| {
            ContextBaseResourceUriSyntax::cast(child.clone())
                .map(CanonicalContextBaseSyntax::ResourceUri)
                .or_else(|| {
                    ContextBaseContextSyntax::cast(child).map(CanonicalContextBaseSyntax::Context)
                })
        })
    }

    pub fn capabilities(&self) -> impl Iterator<Item = ContextCapabilityDeclarationSyntax> {
        self.0
            .children()
            .filter_map(ContextCapabilityDeclarationSyntax::cast)
    }
}

impl ContextBaseContextSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
}

impl ContextBaseResourceUriSyntax {
    pub fn physical_tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl ContextCapabilityDeclarationSyntax {
    pub fn operation(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }

    pub fn scope(&self) -> Option<ContextCapabilityScopeSyntax> {
        child(&self.0)
    }
}

impl ContextCapabilityPathSyntax {
    pub fn tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

#[derive(Clone, Debug)]
pub enum CanonicalContextCapabilityScopeSyntax {
    Wildcard(SyntaxToken),
    Path(ContextCapabilityPathSyntax),
}

impl ContextCapabilityScopeSyntax {
    pub fn selected(&self) -> Option<CanonicalContextCapabilityScopeSyntax> {
        self.0
            .children_with_tokens()
            .into_iter()
            .find_map(|element| match element {
                SyntaxElement::Token(token) if token.kind() == SyntaxKind::Asterisk => {
                    Some(CanonicalContextCapabilityScopeSyntax::Wildcard(token))
                }
                SyntaxElement::Node(node) => ContextCapabilityPathSyntax::cast(node)
                    .map(CanonicalContextCapabilityScopeSyntax::Path),
                _ => None,
            })
    }
}

fn child<N: AstNode>(syntax: &SyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}
