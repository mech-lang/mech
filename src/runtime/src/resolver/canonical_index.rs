//! Resolver facts projected directly from the canonical retained document.

use super::*;
use crate::resolver::{SourceImportAlias, imports::classified_module_import};
use mech_syntax::document::{
    AstNode, CanonicalContextBaseSyntax, CanonicalContextCapabilityScopeSyntax,
    CanonicalModuleImportBodySyntax, CodeBlockSyntax, CodeFenceScope, ContextDeclarationSyntax,
    DocumentId, DocumentSyntax, ExportDeclarationSyntax, ImportDeclarationSyntax,
    ModuleImportSyntax, NodeFlags, PrefixedContextPathSyntax, Revision, SyntaxKind, SyntaxNode,
    TextRange,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalSourceIndexError {
    pub document: DocumentId,
    pub revision: Revision,
    pub range: TextRange,
    pub message: &'static str,
}

impl std::fmt::Display for CanonicalSourceIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.message, self.range)
    }
}

impl std::error::Error for CanonicalSourceIndexError {}

type Result<T> = std::result::Result<T, CanonicalSourceIndexError>;

fn error(node: &SyntaxNode, message: &'static str) -> CanonicalSourceIndexError {
    CanonicalSourceIndexError {
        document: node.source().document(),
        revision: node.source().revision(),
        range: node.range(),
        message,
    }
}

fn required<T>(value: Option<T>, owner: &SyntaxNode) -> Result<T> {
    value.ok_or_else(|| error(owner, "canonical declaration is missing a required role"))
}

fn text(node: &SyntaxNode) -> Result<String> {
    node.text()
        .map_err(|_| error(node, "canonical source range is unavailable"))
}

impl SourceIndex {
    /// Index executable document scopes without reparsing or constructing a Program.
    ///
    /// Display-only inline code and disabled/inert fences do not create resolver
    /// dependencies. Named fences share the same interpreter scope by name.
    /// Invalid syntax or unsupported scope configuration fails before publication.
    pub fn from_document(document: &DocumentSyntax) -> Result<Self> {
        let root = document.syntax();
        if root.flags().intersects(
            NodeFlags::ERROR
                | NodeFlags::MISSING
                | NodeFlags::CONTAINS_ERROR
                | NodeFlags::CONTAINS_MISSING,
        ) {
            return Err(error(
                root,
                "cannot index a document containing syntax errors",
            ));
        }
        let mut index = Self::default();
        index.push_scope(SourceScope::Program);
        let mut pending = vec![(root.clone(), SourceScope::Program)];
        while let Some((node, scope)) = pending.pop() {
            if node.kind() == SyntaxKind::InlineMechCode {
                continue;
            }
            if node.kind() == SyntaxKind::MikaSection {
                return Err(error(
                    &node,
                    "Mika scope indexing requires its document owner",
                ));
            }
            if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
                let info = required(fence.info(), &node)?;
                let scope = match info.scope {
                    CodeFenceScope::Inert | CodeFenceScope::Disabled => continue,
                    CodeFenceScope::UnsupportedInfo(_) => {
                        return Err(error(&node, "unsupported fence information"));
                    }
                    CodeFenceScope::Root => SourceScope::Program,
                    CodeFenceScope::Named(name) => {
                        let interpreter = SourceInterpreterId {
                            namespace: mech_core::hash_str(&name),
                            namespace_str: name,
                        };
                        if !index.address_target_interpreters.contains(&interpreter) {
                            index.address_target_interpreters.push(interpreter.clone());
                        }
                        let scope = SourceScope::Interpreter(interpreter);
                        index.push_scope(scope.clone());
                        scope
                    }
                };
                if let Some(options) = fence.options() {
                    return Err(error(
                        options.syntax(),
                        "fence options require their document owner",
                    ));
                }
                if let Some(code) = fence.mech_code() {
                    pending.push((code.syntax().clone(), scope));
                }
                continue;
            }
            if let Some(import) = ImportDeclarationSyntax::cast(node.clone()) {
                let specifier = required(import.specifier(), &node)?;
                let declaration = classify_import_specifier(text(specifier.syntax())?);
                index.canonical_import(&node, scope, declaration)?;
                continue;
            }
            if let Some(import) = ModuleImportSyntax::cast(node.clone()) {
                for declaration in module_imports(&import)? {
                    index.canonical_import(&node, scope.clone(), declaration)?;
                }
                continue;
            }
            if let Some(export) = ExportDeclarationSyntax::cast(node.clone()) {
                let name = text(required(export.name(), &node)?.syntax())?;
                index.push_export(
                    scope,
                    index.declarations.len(),
                    Some(range(&node)?),
                    SourceExportDeclaration { name },
                );
                continue;
            }
            if let Some(context) = ContextDeclarationSyntax::cast(node.clone()) {
                let name = text(required(context.name(), &node)?.syntax())?;
                let base = match required(context.base(), &node)? {
                    CanonicalContextBaseSyntax::Context(base) => SourceContextBase::Context(text(
                        required(base.name(), base.syntax())?.syntax(),
                    )?),
                    CanonicalContextBaseSyntax::ResourceUri(base) => {
                        SourceContextBase::ResourceUri(text(base.syntax())?)
                    }
                };
                let mut capabilities = Vec::new();
                for cap in context.capabilities() {
                    let operation = text(required(cap.operation(), cap.syntax())?.syntax())?;
                    let cap_scope = required(cap.scope(), cap.syntax())?;
                    let scope = match required(cap_scope.selected(), cap_scope.syntax())? {
                        CanonicalContextCapabilityScopeSyntax::Wildcard(_) => {
                            SourceContextCapabilityScope::Wildcard
                        }
                        CanonicalContextCapabilityScopeSyntax::Path(path) => {
                            SourceContextCapabilityScope::Path(text(path.syntax())?)
                        }
                    };
                    capabilities.push(SourceContextCapability { operation, scope });
                }
                index.push_context(
                    scope,
                    index.declarations.len(),
                    Some(range(&node)?),
                    SourceContextDeclaration {
                        name,
                        base,
                        capabilities,
                    },
                );
                continue;
            }
            if let Some(path) = PrefixedContextPathSyntax::cast(node.clone()) {
                let target = text(required(path.context(), &node)?.syntax())?;
                let name = text(required(path.address(), &node)?.syntax())?;
                index.push_address_reference(
                    scope,
                    index.declarations.len(),
                    Some(range(&node)?),
                    SourceAddressReference { name, target },
                );
                continue;
            }
            let children: Vec<_> = node.children().collect();
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, scope.clone())),
            );
        }
        Ok(index)
    }

    fn canonical_import(
        &mut self,
        node: &SyntaxNode,
        scope: SourceScope,
        declaration: SourceImportDeclaration,
    ) -> Result<()> {
        self.push_import(
            scope,
            self.declarations.len(),
            Some(range(node)?),
            declaration,
        );
        Ok(())
    }
}

fn module_imports(import: &ModuleImportSyntax) -> Result<Vec<SourceImportDeclaration>> {
    let node = import.syntax();
    Ok(match required(import.body(), node)? {
        CanonicalModuleImportBodySyntax::Module(body) => {
            let module = text(required(body.module(), node)?.syntax())?;
            vec![classified_module_import(&module, None, None)]
        }
        CanonicalModuleImportBodySyntax::AliasedItem(body) => {
            let module = text(required(body.module(), node)?.syntax())?;
            let item = text(required(body.item(), node)?.syntax())?;
            let alias = required(body.alias(), node)?;
            let alias = if let Some(context) = alias.context() {
                SourceImportAlias::Context(text(required(context.name(), node)?.syntax())?)
            } else {
                let value = required(alias.value(), node)?;
                SourceImportAlias::Value(text(required(value.path(), node)?.syntax())?)
            };
            vec![classified_module_import(&module, Some(&item), Some(alias))]
        }
        CanonicalModuleImportBodySyntax::Suffix(body) => {
            let module = text(required(body.module(), node)?.syntax())?;
            if body.is_glob() {
                let mut declaration = classify_import_specifier(format!("{module}/*"));
                declaration.module = Some(module);
                vec![declaration]
            } else if let Some(group) = body.group() {
                group
                    .items()
                    .map(|item| {
                        let item = text(required(item.path(), node)?.syntax())?;
                        Ok(classified_module_import(&module, Some(&item), None))
                    })
                    .collect::<Result<_>>()?
            } else {
                let item = text(required(body.item(), node)?.syntax())?;
                vec![classified_module_import(&module, Some(&item), None)]
            }
        }
    })
}

// SourceIndex's public resolver coordinates are one-based Unicode scalar columns.
// Canonical byte ranges remain on errors; this conversion never reparses syntax.
fn range(node: &SyntaxNode) -> Result<SourceRange> {
    let source = node.source();
    let location = |offset| -> Result<mech_core::SourceLocation> {
        let line = source.line_index().line_of(offset);
        let start = source.line_index().line_start(line).unwrap();
        let prefix = source
            .text(TextRange::new(start, offset))
            .map_err(|_| error(node, "canonical source position is unavailable"))?;
        Ok(mech_core::SourceLocation {
            row: line + 1,
            col: prefix.chars().count() + 1,
        })
    };
    Ok(SourceRange {
        start: location(node.range().start)?,
        end: location(node.range().end)?,
    })
}
