//! Typed handoff from canonical document declarations to runtime compilation.

use std::collections::{BTreeMap, HashMap};

use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, SourceSemanticError};
use mech_syntax::document::DocumentSyntax;

use super::{
    CanonicalSourceIndexError, SourceImportAlias, SourceImportDeclaration, SourceImportKind,
    SourceIndex, SourceInterpreterId, SourceScope, import_may_resolve_source_dependency,
    import_requires_source_dependency, module_namespace_for_import,
};
use crate::RuntimeValueSnapshot;

pub struct CanonicalDocumentCompilation {
    pub index: SourceIndex,
    pub scope: SourceScope,
    pub program: CanonicalSourceProgram,
}

#[derive(Clone)]
pub struct CanonicalResolvedImport<T = RuntimeValueSnapshot> {
    pub declaration: SourceImportDeclaration,
    pub canonical_uri: String,
    pub exports: BTreeMap<String, T>,
}

#[derive(Clone)]
pub struct CanonicalDocumentInputBinding {
    pub input: u32,
    pub name: String,
    pub value: RuntimeValueSnapshot,
}

#[derive(Debug)]
pub enum CanonicalDocumentHandoffError {
    Index(CanonicalSourceIndexError),
    AddressTargets(mech_core::MechError),
    Semantics(SourceSemanticError),
    UnknownResolvedImport {
        specifier: String,
    },
    UnresolvedImport {
        specifier: String,
        occurrence: Option<mech_core::SourceRange>,
    },
    MissingExport {
        dependency: String,
        export: String,
        occurrence: Option<mech_core::SourceRange>,
    },
    ImportConflict {
        binding: String,
        first: String,
        second: String,
        occurrence: Option<mech_core::SourceRange>,
    },
    InputIdentityExhausted,
    MissingCompletedExport {
        export: String,
        output: u32,
        range: Option<mech_syntax::document::TextRange>,
    },
}

impl core::fmt::Display for CanonicalDocumentHandoffError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Index(error) => error.fmt(formatter),
            Self::AddressTargets(error) => write!(formatter, "{error:?}"),
            Self::Semantics(error) => error.fmt(formatter),
            Self::UnknownResolvedImport { specifier } => {
                write!(
                    formatter,
                    "resolved import {specifier:?} is not declared by this scope"
                )
            }
            Self::UnresolvedImport {
                specifier,
                occurrence,
            } => write!(
                formatter,
                "declared import {specifier:?} has no resolved dependency at {occurrence:?}"
            ),
            Self::MissingExport {
                dependency,
                export,
                occurrence,
            } => {
                write!(
                    formatter,
                    "dependency {dependency:?} has no export {export:?} at {occurrence:?}"
                )
            }
            Self::ImportConflict {
                binding,
                first,
                second,
                occurrence,
            } => write!(
                formatter,
                "binding {binding:?} is supplied by both {first:?} and {second:?} at {occurrence:?}"
            ),
            Self::InputIdentityExhausted => {
                formatter.write_str("canonical document input count exceeds u32")
            }
            Self::MissingCompletedExport {
                export,
                output,
                range,
            } => write!(
                formatter,
                "document export {export:?} has no completed output {output} at {range:?}"
            ),
        }
    }
}

impl std::error::Error for CanonicalDocumentHandoffError {}

impl From<CanonicalSourceIndexError> for CanonicalDocumentHandoffError {
    fn from(error: CanonicalSourceIndexError) -> Self {
        Self::Index(error)
    }
}

impl From<SourceSemanticError> for CanonicalDocumentHandoffError {
    fn from(error: SourceSemanticError) -> Self {
        Self::Semantics(error)
    }
}

impl CanonicalDocumentCompilation {
    /// Compile one clean canonical owner while retaining its resolver metadata.
    /// No declaration is reparsed or translated through the legacy Program AST.
    pub fn from_document(document: &DocumentSyntax) -> Result<Self, CanonicalDocumentHandoffError> {
        let index = SourceIndex::from_document(document)?;
        let program = CanonicalSourceFrontend.compile_document(document)?;
        Self::from_parts(index, SourceScope::Program, program)
    }

    /// Compile a resolver-owned document using its defining nominal module.
    /// A detached syntax document has no provenance, so enum owners should
    /// enter through this constructor after resolution.
    pub fn from_source_document(
        document: &crate::SourceDocument,
    ) -> Result<Self, CanonicalDocumentHandoffError> {
        let syntax = document.document();
        let index = SourceIndex::from_document(&syntax)?;
        let program = if let Some(origin) = document.nominal_origin() {
            CanonicalSourceFrontend.compile_document_with_nominal_origin(&syntax, origin)?
        } else {
            CanonicalSourceFrontend.compile_document(&syntax)?
        };
        Self::from_parts(index, SourceScope::Program, program)
    }

    /// Compile one named root-document interpreter with its matching resolver scope.
    pub fn from_named_document_scope(
        document: &DocumentSyntax,
        name: &str,
    ) -> Result<Self, CanonicalDocumentHandoffError> {
        let index = SourceIndex::from_document(document)?;
        let program = CanonicalSourceFrontend.compile_named_document_scope(document, name)?;
        Self::from_parts(index, named_scope(name), program)
    }

    /// Compile one Mika-local root scope with its independently owned resolver facts.
    pub fn from_mika_section(
        section: &mech_syntax::document::MikaSectionSyntax,
    ) -> Result<Self, CanonicalDocumentHandoffError> {
        let index = SourceIndex::from_mika_section(section)?;
        let program = CanonicalSourceFrontend.compile_mika_section(section)?;
        Self::from_parts(index, SourceScope::Program, program)
    }

    /// Compile one named interpreter inside a Mika-local owner.
    pub fn from_named_mika_scope(
        section: &mech_syntax::document::MikaSectionSyntax,
        name: &str,
    ) -> Result<Self, CanonicalDocumentHandoffError> {
        let index = SourceIndex::from_mika_section(section)?;
        let program = CanonicalSourceFrontend.compile_named_mika_scope(section, name)?;
        Self::from_parts(index, named_scope(name), program)
    }

    fn from_parts(
        index: SourceIndex,
        scope: SourceScope,
        program: CanonicalSourceProgram,
    ) -> Result<Self, CanonicalDocumentHandoffError> {
        index
            .validate_address_targets()
            .map_err(CanonicalDocumentHandoffError::AddressTargets)?;
        Ok(Self {
            index,
            scope,
            program,
        })
    }

    pub fn declared_imports(&self) -> Vec<SourceImportDeclaration> {
        self.index.imports_for_scope(&self.scope)
    }

    /// Convert completed artifact outputs into the dependency export map
    /// consumed by another canonical document handoff.
    pub fn exports_from_values(
        &self,
        values: &[RuntimeValueSnapshot],
    ) -> Result<BTreeMap<String, RuntimeValueSnapshot>, CanonicalDocumentHandoffError> {
        self.program
            .document_exports()
            .iter()
            .map(|export| {
                values
                    .get(export.output as usize)
                    .cloned()
                    .map(|value| (export.name.clone(), value))
                    .ok_or_else(|| CanonicalDocumentHandoffError::MissingCompletedExport {
                        export: export.name.clone(),
                        output: export.output,
                        range: self
                            .program
                            .source_map()
                            .outputs
                            .get(export.output as usize)
                            .map(|anchor| anchor.range),
                    })
            })
            .collect()
    }

    /// Bind exports from already-resolved dependencies to canonical artifact
    /// inputs using the established source-import namespace and alias rules.
    pub fn bind_resolved_imports(
        &self,
        resolved: &[CanonicalResolvedImport],
    ) -> Result<Vec<CanonicalDocumentInputBinding>, CanonicalDocumentHandoffError> {
        let environment = canonical_import_values(&self.index, &self.scope, resolved)?;
        validate_canonical_import_uses(
            &self.index,
            &self.scope,
            resolved,
            self.program
                .program()
                .inputs
                .iter()
                .map(|input| input.name.as_str()),
        )?;

        self.program
            .program()
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(input, declaration)| {
                environment.get(&declaration.name).cloned().map(|value| {
                    u32::try_from(input)
                        .map(|input| CanonicalDocumentInputBinding {
                            input,
                            name: declaration.name.clone(),
                            value,
                        })
                        .map_err(|_| CanonicalDocumentHandoffError::InputIdentityExhausted)
                })
            })
            .collect()
    }
}

fn named_scope(name: &str) -> SourceScope {
    SourceScope::Interpreter(SourceInterpreterId {
        namespace: mech_core::hash_str(name),
        namespace_str: name.to_owned(),
    })
}

pub(crate) fn canonical_import_values<T: Clone>(
    index: &SourceIndex,
    scope: &SourceScope,
    resolved: &[CanonicalResolvedImport<T>],
) -> Result<BTreeMap<String, T>, CanonicalDocumentHandoffError> {
    let occurrence = |declaration: &SourceImportDeclaration| {
        index
            .imports
            .iter()
            .find(|candidate| {
                &candidate.occurrence.scope == scope && &candidate.declaration == declaration
            })
            .and_then(|candidate| candidate.occurrence.range.clone())
    };
    let declared = index.imports_for_scope(scope);
    for declaration in declared
        .iter()
        .filter(|declaration| import_requires_source_dependency(declaration))
    {
        if !resolved
            .iter()
            .any(|dependency| &dependency.declaration == declaration)
        {
            return Err(CanonicalDocumentHandoffError::UnresolvedImport {
                specifier: declaration.specifier.clone(),
                occurrence: occurrence(declaration),
            });
        }
    }
    let mut environment = BTreeMap::<String, T>::new();
    let mut ownership = HashMap::<String, String>::new();
    for dependency in resolved {
        if !declared.contains(&dependency.declaration)
            || !import_may_resolve_source_dependency(&dependency.declaration)
        {
            return Err(CanonicalDocumentHandoffError::UnknownResolvedImport {
                specifier: dependency.declaration.specifier.clone(),
            });
        }
        let occurrence = occurrence(&dependency.declaration);
        let mut insert = |binding: String, value: T| -> Result<(), CanonicalDocumentHandoffError> {
            if let Some(first) = ownership.insert(binding.clone(), dependency.canonical_uri.clone())
            {
                return Err(CanonicalDocumentHandoffError::ImportConflict {
                    binding,
                    first,
                    second: dependency.canonical_uri.clone(),
                    occurrence: occurrence.clone(),
                });
            }
            environment.insert(binding, value);
            Ok(())
        };
        match &dependency.declaration.kind {
            SourceImportKind::DependencyOnly | SourceImportKind::Namespace => {
                if let Some(namespace) = module_namespace_for_import(&dependency.declaration) {
                    for (name, value) in &dependency.exports {
                        insert(format!("{namespace}/{name}"), value.clone())?;
                    }
                }
            }
            SourceImportKind::Single { name } => {
                if matches!(
                    dependency.declaration.alias,
                    Some(SourceImportAlias::Context(_))
                ) {
                    continue;
                }
                let value = dependency.exports.get(name).cloned().ok_or_else(|| {
                    CanonicalDocumentHandoffError::MissingExport {
                        dependency: dependency.canonical_uri.clone(),
                        export: name.clone(),
                        occurrence: occurrence.clone(),
                    }
                })?;
                let binding = match &dependency.declaration.alias {
                    Some(SourceImportAlias::Value(alias)) => alias.clone(),
                    Some(SourceImportAlias::Context(_)) => unreachable!(),
                    None => name.clone(),
                };
                insert(binding, value)?;
            }
            SourceImportKind::Wildcard => {
                for (name, value) in &dependency.exports {
                    insert(name.clone(), value.clone())?;
                }
            }
        }
    }

    Ok(environment)
}

/// Check namespace uses against the same declared export authority for both
/// detached dependency values and linked root graph bindings.
pub(crate) fn validate_canonical_import_uses<'a, T>(
    index: &SourceIndex,
    scope: &SourceScope,
    resolved: &[CanonicalResolvedImport<T>],
    input_names: impl IntoIterator<Item = &'a str>,
) -> Result<(), CanonicalDocumentHandoffError> {
    let input_names = input_names.into_iter().collect::<Vec<_>>();
    for dependency in resolved {
        if matches!(
            dependency.declaration.kind,
            SourceImportKind::DependencyOnly | SourceImportKind::Namespace
        ) && let Some(namespace) = module_namespace_for_import(&dependency.declaration)
        {
            let prefix = format!("{namespace}/");
            for export in input_names
                .iter()
                .filter_map(|name| name.strip_prefix(&prefix))
            {
                if !dependency.exports.contains_key(export) {
                    return Err(CanonicalDocumentHandoffError::MissingExport {
                        dependency: dependency.canonical_uri.clone(),
                        export: export.to_owned(),
                        occurrence: index
                            .imports
                            .iter()
                            .find(|item| {
                                &item.occurrence.scope == scope
                                    && item.declaration == dependency.declaration
                            })
                            .and_then(|item| item.occurrence.range.clone()),
                    });
                }
            }
        }
    }
    Ok(())
}
