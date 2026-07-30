//! Typed syntax views for the closed Phase 2E module-import productions.

use crate::document::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use super::paths::IdentifierPathSegmentSyntax;

macro_rules! import_ast_node {
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

import_ast_node!(ModuleImportNameSegmentSyntax, ModuleImportNameSegment);
import_ast_node!(
    ModuleImportIntrinsicSegmentSyntax,
    ModuleImportIntrinsicSegment
);
import_ast_node!(ModuleImportPathSegmentSyntax, ModuleImportPathSegment);
import_ast_node!(ModuleImportPathSyntax, ModuleImportPath);
import_ast_node!(ModuleImportAliasSegmentSyntax, ModuleImportAliasSegment);
import_ast_node!(ModuleImportAliasPathSyntax, ModuleImportAliasPath);
import_ast_node!(ModuleImportValueAliasSyntax, ModuleImportValueAlias);
import_ast_node!(ContextImportAliasSegmentSyntax, ContextImportAliasSegment);
import_ast_node!(ModuleImportContextAliasSyntax, ModuleImportContextAlias);
import_ast_node!(ModuleImportAliasSyntax, ModuleImportAlias);
import_ast_node!(ModuleRootSyntax, ModuleRoot);
import_ast_node!(ImportGroupItemSyntax, ImportGroupItem);
import_ast_node!(ImportGroupItemsSyntax, ImportGroupItems);
import_ast_node!(AliasedItemImportSyntax, AliasedItemImport);
import_ast_node!(ModuleSuffixImportSyntax, ModuleSuffixImport);
import_ast_node!(ModuleOnlyImportSyntax, ModuleOnlyImport);
import_ast_node!(ModuleImportSyntax, ModuleImport);

/// The structural body selected by a complete module import.
#[derive(Clone, Debug)]
pub enum CanonicalModuleImportBodySyntax {
    AliasedItem(AliasedItemImportSyntax),
    Suffix(ModuleSuffixImportSyntax),
    Module(ModuleOnlyImportSyntax),
}

impl CanonicalModuleImportBodySyntax {
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::AliasedItem(syntax) => syntax.syntax(),
            Self::Suffix(syntax) => syntax.syntax(),
            Self::Module(syntax) => syntax.syntax(),
        }
    }
}

/// The structural compatibility kind selected by a module-import body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalModuleImportKind {
    Module,
    Item,
    Glob,
    Group,
}

impl ModuleImportNameSegmentSyntax {
    pub fn identifier(&self) -> Option<IdentifierPathSegmentSyntax> {
        child(&self.0)
    }
}

impl ModuleImportIntrinsicSegmentSyntax {
    pub fn name(&self) -> Option<ModuleImportNameSegmentSyntax> {
        child(&self.0)
    }

    pub fn marker(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .into_iter()
            .find_map(|element| match element {
                SyntaxElement::Token(token) if token.kind() == SyntaxKind::Underscore => {
                    Some(token)
                }
                _ => None,
            })
    }
}

impl ModuleImportPathSegmentSyntax {
    pub fn name(&self) -> Option<ModuleImportNameSegmentSyntax> {
        child(&self.0)
    }

    pub fn intrinsic(&self) -> Option<ModuleImportIntrinsicSegmentSyntax> {
        child(&self.0)
    }
}

impl ModuleImportPathSyntax {
    pub fn segments(&self) -> impl Iterator<Item = ModuleImportPathSegmentSyntax> {
        self.0
            .children()
            .filter_map(ModuleImportPathSegmentSyntax::cast)
    }
}

impl ModuleImportAliasSegmentSyntax {
    pub fn identifier(&self) -> Option<IdentifierPathSegmentSyntax> {
        child(&self.0)
    }
}

impl ModuleImportAliasPathSyntax {
    pub fn segments(&self) -> impl Iterator<Item = ModuleImportAliasSegmentSyntax> {
        self.0
            .children()
            .filter_map(ModuleImportAliasSegmentSyntax::cast)
    }
}

impl ModuleImportValueAliasSyntax {
    pub fn path(&self) -> Option<ModuleImportAliasPathSyntax> {
        child(&self.0)
    }
}

impl ContextImportAliasSegmentSyntax {
    pub fn tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl ModuleImportContextAliasSyntax {
    pub fn name(&self) -> Option<ContextImportAliasSegmentSyntax> {
        child(&self.0)
    }
}

impl ModuleImportAliasSyntax {
    pub fn context(&self) -> Option<ModuleImportContextAliasSyntax> {
        child(&self.0)
    }

    pub fn value(&self) -> Option<ModuleImportValueAliasSyntax> {
        child(&self.0)
    }
}

impl ModuleRootSyntax {
    pub fn identifier(&self) -> Option<IdentifierPathSegmentSyntax> {
        child(&self.0)
    }
}

impl ImportGroupItemSyntax {
    pub fn path(&self) -> Option<ModuleImportPathSyntax> {
        child(&self.0)
    }
}

impl ImportGroupItemsSyntax {
    pub fn items(&self) -> impl Iterator<Item = ImportGroupItemSyntax> {
        self.0.children().filter_map(ImportGroupItemSyntax::cast)
    }
}

impl AliasedItemImportSyntax {
    pub fn alias(&self) -> Option<ModuleImportAliasSyntax> {
        child(&self.0)
    }

    pub fn module(&self) -> Option<ModuleRootSyntax> {
        child(&self.0)
    }

    pub fn item(&self) -> Option<ModuleImportPathSyntax> {
        child(&self.0)
    }
}

impl ModuleSuffixImportSyntax {
    pub fn module(&self) -> Option<ModuleRootSyntax> {
        child(&self.0)
    }

    pub fn item(&self) -> Option<ModuleImportPathSyntax> {
        child(&self.0)
    }

    pub fn group(&self) -> Option<ImportGroupItemsSyntax> {
        child(&self.0)
    }

    pub fn is_glob(&self) -> bool {
        self.0.children_with_tokens().into_iter().any(|element| {
            matches!(element, SyntaxElement::Token(token) if token.kind() == SyntaxKind::Asterisk)
        })
    }
}

impl ModuleOnlyImportSyntax {
    pub fn module(&self) -> Option<ModuleRootSyntax> {
        child(&self.0)
    }
}

impl ModuleImportSyntax {
    pub fn body(&self) -> Option<CanonicalModuleImportBodySyntax> {
        self.0.children().find_map(|child| {
            AliasedItemImportSyntax::cast(child.clone())
                .map(CanonicalModuleImportBodySyntax::AliasedItem)
                .or_else(|| {
                    ModuleSuffixImportSyntax::cast(child.clone())
                        .map(CanonicalModuleImportBodySyntax::Suffix)
                })
                .or_else(|| {
                    ModuleOnlyImportSyntax::cast(child).map(CanonicalModuleImportBodySyntax::Module)
                })
        })
    }

    pub fn import_kind(&self) -> Option<CanonicalModuleImportKind> {
        match self.body()? {
            CanonicalModuleImportBodySyntax::Module(_) => Some(CanonicalModuleImportKind::Module),
            CanonicalModuleImportBodySyntax::AliasedItem(_) => {
                Some(CanonicalModuleImportKind::Item)
            }
            CanonicalModuleImportBodySyntax::Suffix(suffix) if suffix.is_glob() => {
                Some(CanonicalModuleImportKind::Glob)
            }
            CanonicalModuleImportBodySyntax::Suffix(suffix) if suffix.group().is_some() => {
                Some(CanonicalModuleImportKind::Group)
            }
            CanonicalModuleImportBodySyntax::Suffix(suffix) if suffix.item().is_some() => {
                Some(CanonicalModuleImportKind::Item)
            }
            CanonicalModuleImportBodySyntax::Suffix(_) => None,
        }
    }
}

fn child<N: AstNode>(syntax: &SyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}
