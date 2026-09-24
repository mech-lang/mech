//! Resolver facts projected directly from the canonical retained document.

use super::*;
use crate::resolver::{
    SourceImportAlias, classify_import_specifier, imports::classified_module_import,
};
use mech_syntax::document::{
    AstNode, CanonicalContextBaseSyntax, CanonicalContextCapabilityScopeSyntax,
    CanonicalModuleImportBodySyntax, CodeBlockSyntax, CodeFenceScope, ContextDeclarationSyntax,
    ContextSendSyntax, DocumentId, DocumentSyntax, ExportDeclarationSyntax,
    ImportDeclarationSyntax, ModuleImportSyntax, NodeFlags, OpAssignSyntax,
    PrefixedContextPathSyntax, Revision, SliceStemSyntax, SliceSyntax, SyntaxKind, SyntaxNode,
    TextRange, TextSize, VariableAssignSyntax, VariableDefineSyntax, VariableStemSyntax,
    VariableSyntax,
};
use std::collections::HashMap;

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
        Self::from_local_owner(document.syntax())
    }

    /// Index only this Mika's lexical body. Nested Mika owners have separate indexes.
    pub fn from_mika_section(section: &mech_syntax::document::MikaSectionSyntax) -> Result<Self> {
        validate_mika_section(section)?;
        let body = required(section.body(), section.syntax())?;
        Self::from_local_owner(body.syntax())
    }

    fn from_local_owner(root: &SyntaxNode) -> Result<Self> {
        validate_local_owner(root)?;
        let locations = SourceLocationProjector::new(root)?;
        Self::from_local_owner_with_locations(root, &locations)
    }

    fn from_local_owner_with_locations(
        root: &SyntaxNode,
        locations: &SourceLocationProjector,
    ) -> Result<Self> {
        validate_local_owner(root)?;
        let mut index = Self::default();
        index.push_scope(SourceScope::Program);
        let mut pending = vec![(root.clone(), SourceScope::Program, true)];
        while let Some((node, scope, publish_declarations)) = pending.pop() {
            if node.kind() == SyntaxKind::InlineMechCode {
                continue;
            }
            if node.kind() == SyntaxKind::MikaSection {
                continue;
            }
            if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
                let info = required(fence.info(), &node)?;
                let scope = match info.scope {
                    CodeFenceScope::Inert | CodeFenceScope::Disabled => continue,
                    CodeFenceScope::Root => scope.clone(),
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
                required(fence.presentation(), &node)?;
                if let Some(code) = fence.mech_code() {
                    pending.push((code.syntax().clone(), scope, publish_declarations));
                }
                continue;
            }
            // Assignment and send destination bases are writes, not addressed
            // reads. Assignment subscripts and right-hand values are reads.
            if let Some(send) = ContextSendSyntax::cast(node.clone()) {
                let expression = required(send.expression(), &node)?;
                pending.push((expression.syntax().clone(), scope, publish_declarations));
                continue;
            }
            if let Some(definition) = VariableDefineSyntax::cast(node.clone()) {
                let expression = required(definition.value(), &node)?;
                pending.push((expression.syntax().clone(), scope, publish_declarations));
                continue;
            }
            if let Some(assign) = VariableAssignSyntax::cast(node.clone()) {
                let target = required(assign.target(), &node)?;
                let expression = required(assign.value(), &node)?;
                pending.push((
                    expression.syntax().clone(),
                    scope.clone(),
                    publish_declarations,
                ));
                if let Some(subscripts) = target.subscripts() {
                    pending.push((subscripts.syntax().clone(), scope, publish_declarations));
                }
                continue;
            }
            if let Some(assign) = OpAssignSyntax::cast(node.clone()) {
                let target = required(assign.target(), &node)?;
                let expression = required(assign.value(), &node)?;
                pending.push((
                    expression.syntax().clone(),
                    scope.clone(),
                    publish_declarations,
                ));
                if let Some(subscripts) = target.subscripts() {
                    pending.push((subscripts.syntax().clone(), scope, publish_declarations));
                }
                continue;
            }
            // FSM formal inputs are declarations, not reads. Specifications
            // have no executable read roles at all; implementations begin
            // indexing at their start value and arms, so omit only the direct
            // Variable children that occupy the formal input list.
            if node.kind() == SyntaxKind::FsmSpecification {
                continue;
            }
            if node.kind() == SyntaxKind::FsmImplementation {
                let children = node
                    .children()
                    .filter(|child| child.kind() != SyntaxKind::Variable)
                    .collect::<Vec<_>>();
                pending.extend(
                    children
                        .into_iter()
                        .rev()
                        .map(|child| (child, scope.clone(), false)),
                );
                continue;
            }
            if let Some(import) = ImportDeclarationSyntax::cast(node.clone()) {
                if publish_declarations {
                    let specifier = required(import.specifier(), &node)?;
                    let declaration = classify_import_specifier(text(specifier.syntax())?);
                    index.canonical_import(&locations, specifier.syntax(), scope, declaration)?;
                }
                continue;
            }
            if let Some(import) = ModuleImportSyntax::cast(node.clone()) {
                if publish_declarations {
                    let occurrence = module_import_range(&locations, &import)?;
                    for declaration in module_imports(&import)? {
                        index.push_import(
                            scope.clone(),
                            index.declarations.len(),
                            Some(occurrence.clone()),
                            declaration,
                        );
                    }
                }
                continue;
            }
            if let Some(export) = ExportDeclarationSyntax::cast(node.clone()) {
                if publish_declarations {
                    let name = required(export.name(), &node)?;
                    let occurrence = range(&locations, name.syntax())?;
                    let name = text(name.syntax())?;
                    index.push_export(
                        scope,
                        index.declarations.len(),
                        Some(occurrence),
                        SourceExportDeclaration { name },
                    );
                }
                continue;
            }
            if let Some(context) = ContextDeclarationSyntax::cast(node.clone()) {
                if !publish_declarations {
                    continue;
                }
                let name = required(context.name(), &node)?;
                let base = required(context.base(), &node)?;
                // Resolver occurrences span semantic roles, excluding the leading
                // @/whitespace and trailing capability delimiters or trivia.
                let mut occurrence = SourceRange {
                    start: range(&locations, name.syntax())?.start,
                    end: range(&locations, base.syntax())?.end,
                };
                let name = text(name.syntax())?;
                let base = match base {
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
                    occurrence.end = range(&locations, cap_scope.syntax())?.end;
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
                    Some(occurrence),
                    SourceContextDeclaration {
                        name,
                        base,
                        capabilities,
                    },
                );
                continue;
            }
            if let Some(variable) = VariableSyntax::cast(node.clone())
                && let Some(VariableStemSyntax::Context(path)) = variable.stem()
            {
                index.canonical_address_reference(&locations, &path, variable.syntax(), scope)?;
                continue;
            }
            if let Some(slice) = SliceSyntax::cast(node.clone())
                && let Some(SliceStemSyntax::Context(path)) = slice.stem()
            {
                index.canonical_address_reference(
                    &locations,
                    &path,
                    slice.syntax(),
                    scope.clone(),
                )?;
                if let Some(subscripts) = slice.subscripts() {
                    pending.push((subscripts.syntax().clone(), scope, publish_declarations));
                }
                continue;
            }
            if let Some(path) = PrefixedContextPathSyntax::cast(node.clone()) {
                index.canonical_address_reference(&locations, &path, &node, scope)?;
                continue;
            }
            let child_declarations = publish_declarations && !declaration_boundary(node.kind());
            let children: Vec<_> = node.children().collect();
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, scope.clone(), child_declarations)),
            );
        }
        Ok(index)
    }

    fn canonical_import(
        &mut self,
        locations: &SourceLocationProjector,
        node: &SyntaxNode,
        scope: SourceScope,
        declaration: SourceImportDeclaration,
    ) -> Result<()> {
        self.push_import(
            scope,
            self.declarations.len(),
            Some(range(locations, node)?),
            declaration,
        );
        Ok(())
    }

    fn canonical_address_reference(
        &mut self,
        locations: &SourceLocationProjector,
        path: &PrefixedContextPathSyntax,
        occurrence: &SyntaxNode,
        scope: SourceScope,
    ) -> Result<()> {
        let target = text(required(path.context(), path.syntax())?.syntax())?;
        let name = text(required(path.address(), path.syntax())?.syntax())?;
        self.push_address_reference(
            scope,
            self.declarations.len(),
            Some(range(locations, occurrence)?),
            SourceAddressReference { name, target },
        );
        Ok(())
    }
}

fn validate_local_owner(root: &SyntaxNode) -> Result<()> {
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
    Ok(())
}

fn validate_mika_section(section: &mech_syntax::document::MikaSectionSyntax) -> Result<()> {
    if section.syntax().flags().intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING,
    ) {
        return Err(error(
            section.syntax(),
            "cannot index a Mika section containing syntax errors",
        ));
    }
    Ok(())
}

fn module_import_range(
    locations: &SourceLocationProjector,
    import: &ModuleImportSyntax,
) -> Result<SourceRange> {
    let owner = import.syntax();
    let mut roles = Vec::new();
    match required(import.body(), owner)? {
        CanonicalModuleImportBodySyntax::Module(body) => {
            roles.push(required(body.module(), owner)?.syntax().clone());
        }
        CanonicalModuleImportBodySyntax::AliasedItem(body) => {
            roles.push(required(body.module(), owner)?.syntax().clone());
            roles.push(required(body.item(), owner)?.syntax().clone());
            let alias = required(body.alias(), owner)?;
            if let Some(context) = alias.context() {
                roles.push(required(context.name(), owner)?.syntax().clone());
            } else {
                let value = required(alias.value(), owner)?;
                roles.push(required(value.path(), owner)?.syntax().clone());
            }
        }
        CanonicalModuleImportBodySyntax::Suffix(body) => {
            roles.push(required(body.module(), owner)?.syntax().clone());
            if let Some(group) = body.group() {
                for item in group.items() {
                    roles.push(required(item.path(), owner)?.syntax().clone());
                }
            } else if let Some(item) = body.item() {
                roles.push(item.syntax().clone());
            }
        }
    }
    let start = roles.iter().min_by_key(|node| node.range().start).unwrap();
    let end = roles.iter().max_by_key(|node| node.range().end).unwrap();
    Ok(SourceRange {
        start: range(locations, start)?.start,
        end: range(locations, end)?.end,
    })
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

// Resolver coordinates use one-based extended grapheme columns. Retained byte
// ranges stay canonical; the snapshot owns this consumer-coordinate projection.
fn range(locations: &SourceLocationProjector, node: &SyntaxNode) -> Result<SourceRange> {
    Ok(SourceRange {
        start: locations.location(node, node.range().start)?,
        end: locations.location(node, node.range().end)?,
    })
}

fn declaration_boundary(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::FunctionDefine
            | SyntaxKind::FsmImplementation
            | SyntaxKind::FsmTransition
            | SyntaxKind::FsmGuard
            | SyntaxKind::FsmStatementTransition
            | SyntaxKind::FsmBlockTransition
    )
}

struct SourceLocationProjector {
    locations: HashMap<TextSize, mech_core::SourceLocation>,
}

impl SourceLocationProjector {
    fn new(root: &SyntaxNode) -> Result<Self> {
        #[cfg(test)]
        SOURCE_LOCATION_PROJECTIONS.with(|count| count.set(count.get() + 1));
        let mut offsets = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(node) = pending.pop() {
            offsets.push(node.range().start);
            offsets.push(node.range().end);
            pending.extend(node.children());
        }
        offsets.sort_unstable();
        offsets.dedup();
        let projected = root
            .source()
            .source_locations(&offsets)
            .ok_or_else(|| error(root, "canonical source position is unavailable"))?;
        Ok(Self {
            locations: offsets.into_iter().zip(projected).collect(),
        })
    }

    fn location(&self, node: &SyntaxNode, offset: TextSize) -> Result<mech_core::SourceLocation> {
        self.locations
            .get(&offset)
            .cloned()
            .ok_or_else(|| error(node, "canonical source position is unavailable"))
    }
}

/// Complete resolver projection for a document and its lexical Mika owners.
/// Each local owner has an independent root/named scope namespace.
#[derive(Clone, Debug)]
pub struct CanonicalDocumentIndex {
    pub owner: mech_syntax::document::DocumentScopeId,
    pub root: SourceIndex,
    pub mika: Vec<CanonicalMikaIndex>,
}

#[derive(Clone, Debug)]
pub struct CanonicalMikaIndex {
    pub owner: mech_syntax::document::MikaDocumentScope,
    pub index: SourceIndex,
}

impl CanonicalDocumentIndex {
    pub fn from_document(document: &DocumentSyntax) -> Result<Self> {
        let locations = SourceLocationProjector::new(document.syntax())?;
        let root = SourceIndex::from_local_owner_with_locations(document.syntax(), &locations)?;
        let mika = document
            .mika_scopes()
            .into_iter()
            .map(|owner| {
                validate_mika_section(&owner.section)?;
                let body = required(owner.section.body(), owner.section.syntax())?;
                let index =
                    SourceIndex::from_local_owner_with_locations(body.syntax(), &locations)?;
                Ok(CanonicalMikaIndex { owner, index })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            owner: document.scope_id(),
            root,
            mika,
        })
    }
}

#[cfg(test)]
std::thread_local! {
    static SOURCE_LOCATION_PROJECTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_syntax::document::parser::canonical::{
        parse_canonical_document_rule_for_test, parse_canonical_phase_2f_rule_for_test,
        parse_canonical_phase_2i_rule_for_test,
    };
    use mech_syntax::document::parser::canonical_rule_id;
    use mech_syntax::document::{
        DocumentId, DocumentSyntax, GreenBuilder, IdGenerator, ParseConfig, TextSnapshot,
        parse_canonical_document,
    };

    fn direct_fragment(
        rule: &str,
        source: &str,
    ) -> std::sync::Arc<mech_syntax::document::GreenNode> {
        let source = TextSnapshot::new(DocumentId(77), Revision(1), source).unwrap();
        let rule_id = canonical_rule_id(rule).unwrap();
        let snapshot = if rule == "variable-define" {
            parse_canonical_phase_2i_rule_for_test(source, rule_id, ParseConfig::default())
        } else if matches!(
            rule,
            "import-declaration" | "export-declaration" | "context-declaration"
        ) {
            parse_canonical_phase_2f_rule_for_test(source, rule_id, ParseConfig::default())
        } else {
            parse_canonical_document_rule_for_test(source, rule_id, ParseConfig::default())
        }
        .unwrap();
        assert!(
            snapshot.is_strictly_clean(),
            "{rule}: outcome={:?}, consumed={:?}, full={:?}, flags={:?}, diagnostics={:#?}",
            snapshot.outcome,
            snapshot.consumed,
            snapshot.source.full_range(),
            snapshot.root.flags,
            snapshot.diagnostics
        );
        assert_eq!(snapshot.consumed, snapshot.source.full_range());
        snapshot.root
    }

    fn nested_index(kind: SyntaxKind) -> SourceIndex {
        let fragments = [
            ("import-declaration", "+> ./hidden.mec"),
            ("export-declaration", "<+ hidden"),
            ("context-declaration", "@local := @env"),
            ("variable-assign", "value = @live/VALUE"),
        ];
        let mut source = String::new();
        let mut roots = Vec::new();
        for (rule, text) in fragments {
            source.push_str(text);
            roots.push(direct_fragment(rule, text));
        }

        let mut ids = IdGenerator::with_next(10_000, 10_000, 10_000);
        let mut builder = GreenBuilder::new(&mut ids);
        builder.start_node(kind);
        for root in roots {
            builder.reuse_node(root).unwrap();
        }
        builder.finish_node().unwrap();
        let root = SyntaxNode::new_root(
            builder.finish().unwrap(),
            TextSnapshot::new(DocumentId(78), Revision(1), source).unwrap(),
        );
        SourceIndex::from_local_owner(&root).unwrap()
    }

    #[test]
    fn nested_owner_declarations_are_not_published_but_reads_are_indexed() {
        let function = nested_index(SyntaxKind::FunctionDefine);
        let transition = nested_index(SyntaxKind::FsmTransition);

        for index in [&function, &transition] {
            assert!(index.imports.is_empty());
            assert!(index.exports.is_empty());
            assert!(index.contexts.is_empty());
        }
        assert_eq!(function.address_references.len(), 1);
        assert_eq!(function.address_references[0].reference.target, "live");
        assert_eq!(transition.address_references.len(), 1);
        assert_eq!(transition.address_references[0].reference.target, "live");
    }

    #[test]
    fn variable_definitions_traverse_only_their_values() {
        let source = "answer := @env/HOME";
        let root = SyntaxNode::new_root(
            direct_fragment("variable-define", source),
            TextSnapshot::new(DocumentId(80), Revision(1), source).unwrap(),
        );
        let index = SourceIndex::from_local_owner(&root).unwrap();
        assert_eq!(index.address_references.len(), 1);
        assert_eq!(index.address_references[0].reference.target, "env");
        assert_eq!(index.address_references[0].reference.name, "HOME");
    }

    #[test]
    fn complete_document_indexes_all_mika_owners_with_one_coordinate_projection() {
        let source = "x := @root/VALUE\n\n~∘~⸢y := @child/VALUE\n\n╭◉╮⸢z := @nested/VALUE\n⸥\n⸥\n";
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(79), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        SOURCE_LOCATION_PROJECTIONS.with(|count| count.set(0));
        let index = CanonicalDocumentIndex::from_document(&document).unwrap();
        assert_eq!(index.root.address_references.len(), 1);
        assert_eq!(index.mika.len(), 2);
        assert_eq!(index.mika[0].index.address_references.len(), 1);
        assert_eq!(index.mika[1].index.address_references.len(), 1);
        SOURCE_LOCATION_PROJECTIONS.with(|count| assert_eq!(count.get(), 1));
    }
}
