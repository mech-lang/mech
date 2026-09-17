#[cfg(feature = "source")]
#[path = "canonical_index.rs"]
mod canonical;
#[cfg(feature = "source")]
pub use canonical::{CanonicalDocumentIndex, CanonicalMikaIndex, CanonicalSourceIndexError};

use mech_core::{MResult, MechError, SourceRange};

use super::{
    AddressTargetNameConflict, SourceAddressReference, SourceContextBase, SourceContextCapability,
    SourceContextCapabilityScope, SourceContextDeclaration, SourceExportDeclaration,
    SourceImportDeclaration, classify_import_specifier,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceScope {
    Program,
    Interpreter(SourceInterpreterId),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceInterpreterId {
    pub namespace: u64,
    pub namespace_str: String,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOccurrence {
    pub scope: SourceScope,
    pub order: usize,
    pub range: Option<SourceRange>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceImportDeclaration {
    pub occurrence: SourceOccurrence,
    pub declaration: SourceImportDeclaration,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceExportDeclaration {
    pub occurrence: SourceOccurrence,
    pub declaration: SourceExportDeclaration,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceContextDeclaration {
    pub occurrence: SourceOccurrence,
    pub declaration: SourceContextDeclaration,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceAddressReference {
    pub occurrence: SourceOccurrence,
    pub reference: SourceAddressReference,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceDeclaration {
    Import(ScopedSourceImportDeclaration),
    Export(ScopedSourceExportDeclaration),
    Context(ScopedSourceContextDeclaration),
    AddressReference(ScopedSourceAddressReference),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleScopeMetadata {
    pub scope: SourceScope,
    pub imports: Vec<SourceImportDeclaration>,
    pub exports: Vec<SourceExportDeclaration>,
    pub contexts: Vec<SourceContextDeclaration>,
    pub address_references: Vec<SourceAddressReference>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceIndex {
    pub declarations: Vec<SourceDeclaration>,
    pub imports: Vec<ScopedSourceImportDeclaration>,
    pub exports: Vec<ScopedSourceExportDeclaration>,
    pub contexts: Vec<ScopedSourceContextDeclaration>,
    pub address_references: Vec<ScopedSourceAddressReference>,
    pub scopes: Vec<SourceScope>,
    pub address_target_interpreters: Vec<SourceInterpreterId>,
}

impl SourceIndex {
    pub fn validate_address_targets(&self) -> MResult<()> {
        let mut targets: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for interpreter in &self.address_target_interpreters {
            if let Some(first_kind) =
                targets.insert(interpreter.namespace_str.clone(), "interpreter".to_string())
            {
                return Err(MechError::new(
                    AddressTargetNameConflict {
                        name: interpreter.namespace_str.clone(),
                        first_kind,
                        second_kind: "interpreter".to_string(),
                    },
                    None,
                ));
            }
        }

        for context in &self.contexts {
            if let Some(first_kind) =
                targets.insert(context.declaration.name.clone(), "context".to_string())
            {
                return Err(MechError::new(
                    AddressTargetNameConflict {
                        name: context.declaration.name.clone(),
                        first_kind,
                        second_kind: "context".to_string(),
                    },
                    None,
                ));
            }
        }

        Ok(())
    }

    pub fn module_scopes(&self) -> Vec<ModuleScopeMetadata> {
        let mut scopes: Vec<SourceScope> = self.scopes.clone();

        for declaration in &self.declarations {
            let scope = match declaration {
                SourceDeclaration::Import(import) => &import.occurrence.scope,
                SourceDeclaration::Export(export) => &export.occurrence.scope,
                SourceDeclaration::Context(context) => &context.occurrence.scope,
                SourceDeclaration::AddressReference(reference) => &reference.occurrence.scope,
            };
            if !scopes.contains(scope) {
                scopes.push(scope.clone());
            }
        }

        scopes
            .into_iter()
            .map(|scope| ModuleScopeMetadata {
                imports: self.imports_for_scope(&scope),
                exports: self.exports_for_scope(&scope),
                contexts: self.contexts_for_scope(&scope),
                address_references: self.address_references_for_scope(&scope),
                scope,
            })
            .collect()
    }

    pub fn all_imports(&self) -> Vec<SourceImportDeclaration> {
        self.imports.iter().map(|x| x.declaration.clone()).collect()
    }
    pub fn all_exports(&self) -> Vec<SourceExportDeclaration> {
        self.exports.iter().map(|x| x.declaration.clone()).collect()
    }
    pub fn all_contexts(&self) -> Vec<SourceContextDeclaration> {
        self.contexts
            .iter()
            .map(|x| x.declaration.clone())
            .collect()
    }
    pub fn all_address_references(&self) -> Vec<SourceAddressReference> {
        self.address_references
            .iter()
            .map(|x| x.reference.clone())
            .collect()
    }

    pub fn program_imports(&self) -> Vec<SourceImportDeclaration> {
        self.imports_for_scope(&SourceScope::Program)
    }
    pub fn program_exports(&self) -> Vec<SourceExportDeclaration> {
        self.exports_for_scope(&SourceScope::Program)
    }
    pub fn program_contexts(&self) -> Vec<SourceContextDeclaration> {
        self.contexts_for_scope(&SourceScope::Program)
    }
    pub fn program_address_references(&self) -> Vec<SourceAddressReference> {
        self.address_references_for_scope(&SourceScope::Program)
    }

    pub fn imports_for_scope(&self, scope: &SourceScope) -> Vec<SourceImportDeclaration> {
        self.imports
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.declaration.clone())
            .collect()
    }

    pub fn exports_for_scope(&self, scope: &SourceScope) -> Vec<SourceExportDeclaration> {
        self.exports
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.declaration.clone())
            .collect()
    }

    pub fn contexts_for_scope(&self, scope: &SourceScope) -> Vec<SourceContextDeclaration> {
        self.contexts
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.declaration.clone())
            .collect()
    }

    pub fn address_references_for_scope(&self, scope: &SourceScope) -> Vec<SourceAddressReference> {
        self.address_references
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.reference.clone())
            .collect()
    }

    pub fn interpreter_scopes(&self) -> Vec<SourceInterpreterId> {
        let mut scopes = Vec::new();
        for scope in &self.scopes {
            if let SourceScope::Interpreter(interpreter) = scope {
                if !scopes.contains(interpreter) {
                    scopes.push(interpreter.clone());
                }
            }
        }
        for declaration in &self.declarations {
            let scope = match declaration {
                SourceDeclaration::Import(import) => &import.occurrence.scope,
                SourceDeclaration::Export(export) => &export.occurrence.scope,
                SourceDeclaration::Context(context) => &context.occurrence.scope,
                SourceDeclaration::AddressReference(reference) => &reference.occurrence.scope,
            };
            if let SourceScope::Interpreter(interpreter) = scope {
                if !scopes.contains(interpreter) {
                    scopes.push(interpreter.clone());
                }
            }
        }
        scopes
    }

    fn push_scope(&mut self, scope: SourceScope) {
        if !self.scopes.contains(&scope) {
            self.scopes.push(scope);
        }
    }

    fn push_import(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceImportDeclaration,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceImportDeclaration {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            declaration,
        };
        self.declarations
            .push(SourceDeclaration::Import(scoped.clone()));
        self.imports.push(scoped);
    }

    fn push_export(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceExportDeclaration,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceExportDeclaration {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            declaration,
        };
        self.declarations
            .push(SourceDeclaration::Export(scoped.clone()));
        self.exports.push(scoped);
    }

    fn push_context(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceContextDeclaration,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceContextDeclaration {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            declaration,
        };
        self.declarations
            .push(SourceDeclaration::Context(scoped.clone()));
        self.contexts.push(scoped);
    }

    fn push_address_reference(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        reference: SourceAddressReference,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceAddressReference {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            reference,
        };
        self.declarations
            .push(SourceDeclaration::AddressReference(scoped.clone()));
        self.address_references.push(scoped);
    }
}
