//! Typed syntax views for the closed Phase 2F source-import productions.

use alloc::vec::Vec;

use crate::document::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

macro_rules! source_import_ast_node {
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

source_import_ast_node!(SourceImportTailSyntax, SourceImportTail);
source_import_ast_node!(SourcePathComponentSyntax, SourcePathComponent);
source_import_ast_node!(SourceMecPathSyntax, SourceMecPath);
source_import_ast_node!(
    RelativeSourceImportSpecifierSyntax,
    RelativeSourceImportSpecifier
);
source_import_ast_node!(
    AbsoluteSourceImportSpecifierSyntax,
    AbsoluteSourceImportSpecifier
);
source_import_ast_node!(BareSourceImportSpecifierSyntax, BareSourceImportSpecifier);
source_import_ast_node!(SourceImportUriSchemeSyntax, SourceImportUriScheme);
source_import_ast_node!(UriSourceImportSpecifierSyntax, UriSourceImportSpecifier);
source_import_ast_node!(SourceImportSpecifierSyntax, SourceImportSpecifier);
source_import_ast_node!(ImportDeclarationSyntax, ImportDeclaration);

impl SourceImportTailSyntax {
    pub fn physical_tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl SourcePathComponentSyntax {
    pub fn tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl SourceMecPathSyntax {
    pub fn components(&self) -> impl Iterator<Item = SourcePathComponentSyntax> {
        self.0
            .children()
            .filter_map(SourcePathComponentSyntax::cast)
    }
}

impl RelativeSourceImportSpecifierSyntax {
    pub fn path(&self) -> Option<SourceMecPathSyntax> {
        child(&self.0)
    }

    pub fn has_wildcard_suffix(&self) -> bool {
        has_wildcard_suffix(&self.0)
    }
}

impl AbsoluteSourceImportSpecifierSyntax {
    pub fn path(&self) -> Option<SourceMecPathSyntax> {
        child(&self.0)
    }

    pub fn has_wildcard_suffix(&self) -> bool {
        has_wildcard_suffix(&self.0)
    }
}

impl BareSourceImportSpecifierSyntax {
    pub fn path(&self) -> Option<SourceMecPathSyntax> {
        child(&self.0)
    }

    pub fn has_wildcard_suffix(&self) -> bool {
        has_wildcard_suffix(&self.0)
    }
}

impl SourceImportUriSchemeSyntax {
    pub fn tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl UriSourceImportSpecifierSyntax {
    pub fn scheme(&self) -> Option<SourceImportUriSchemeSyntax> {
        child(&self.0)
    }

    pub fn tail(&self) -> Option<SourceImportTailSyntax> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub enum CanonicalSourceImportSpecifierSyntax {
    Relative(RelativeSourceImportSpecifierSyntax),
    Absolute(AbsoluteSourceImportSpecifierSyntax),
    Uri(UriSourceImportSpecifierSyntax),
    Bare(BareSourceImportSpecifierSyntax),
}

impl CanonicalSourceImportSpecifierSyntax {
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Relative(syntax) => syntax.syntax(),
            Self::Absolute(syntax) => syntax.syntax(),
            Self::Uri(syntax) => syntax.syntax(),
            Self::Bare(syntax) => syntax.syntax(),
        }
    }
}

impl SourceImportSpecifierSyntax {
    pub fn selected(&self) -> Option<CanonicalSourceImportSpecifierSyntax> {
        self.0.children().find_map(|child| {
            RelativeSourceImportSpecifierSyntax::cast(child.clone())
                .map(CanonicalSourceImportSpecifierSyntax::Relative)
                .or_else(|| {
                    AbsoluteSourceImportSpecifierSyntax::cast(child.clone())
                        .map(CanonicalSourceImportSpecifierSyntax::Absolute)
                })
                .or_else(|| {
                    UriSourceImportSpecifierSyntax::cast(child.clone())
                        .map(CanonicalSourceImportSpecifierSyntax::Uri)
                })
                .or_else(|| {
                    BareSourceImportSpecifierSyntax::cast(child)
                        .map(CanonicalSourceImportSpecifierSyntax::Bare)
                })
        })
    }
}

impl ImportDeclarationSyntax {
    pub fn specifier(&self) -> Option<SourceImportSpecifierSyntax> {
        child(&self.0)
    }
}

fn child<N: AstNode>(syntax: &SyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}

fn has_wildcard_suffix(syntax: &SyntaxNode) -> bool {
    let elements = syntax.children_with_tokens();
    let Some(path_index) = elements.iter().position(|element| {
        matches!(element, SyntaxElement::Node(node) if node.kind() == SyntaxKind::SourceMecPath)
    }) else {
        return false;
    };
    matches!(
        (elements.get(path_index + 1), elements.get(path_index + 2)),
        (
            Some(SyntaxElement::Token(slash)),
            Some(SyntaxElement::Token(asterisk)),
        ) if slash.kind() == SyntaxKind::Slash && asterisk.kind() == SyntaxKind::Asterisk
    )
}
