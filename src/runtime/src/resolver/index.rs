use mech_core::{
    ContextBase, ContextCapabilityScope, MechCode, Program, SectionElement, SourceRange, Statement,
    Token,
};

use super::{
    classify_import_specifier, SourceContextBase, SourceContextCapability,
    SourceContextCapabilityScope, SourceContextDeclaration, SourceExportDeclaration,
    SourceImportDeclaration,
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
pub enum SourceDeclaration {
    Import(ScopedSourceImportDeclaration),
    Export(ScopedSourceExportDeclaration),
    Context(ScopedSourceContextDeclaration),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceIndex {
    pub declarations: Vec<SourceDeclaration>,
    pub imports: Vec<ScopedSourceImportDeclaration>,
    pub exports: Vec<ScopedSourceExportDeclaration>,
    pub contexts: Vec<ScopedSourceContextDeclaration>,
}

impl SourceIndex {
    pub fn from_program(program: &Program) -> Self {
        let mut index = Self::default();
        let mut order = 0usize;

        for section in &program.body.sections {
            for element in &section.elements {
                match element {
                    SectionElement::MechCode(code_items) => {
                        for (code, _) in code_items {
                            if let MechCode::Statement(statement) = code {
                                match statement {
                                    Statement::ImportDeclaration(import) => {
                                        index.push_import(
                                            SourceScope::Program,
                                            order,
                                            declaration_range(import.tokens()),
                                            classify_import_specifier(import.specifier.to_string()),
                                        );
                                        order += 1;
                                    }
                                    Statement::ExportDeclaration(export) => {
                                        index.push_export(
                                            SourceScope::Program,
                                            order,
                                            declaration_range(export.tokens()),
                                            SourceExportDeclaration {
                                                name: export.name.to_string(),
                                            },
                                        );
                                        order += 1;
                                    }
                                    Statement::ContextDeclaration(context) => {
                                        index.push_context(
                                            SourceScope::Program,
                                            order,
                                            declaration_range(context.tokens()),
                                            source_context_declaration(context),
                                        );
                                        order += 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    SectionElement::FencedMechCode(fenced) => {
                        let scope = SourceScope::Interpreter(SourceInterpreterId {
                            namespace: fenced.config.namespace,
                            namespace_str: fenced.config.namespace_str.clone(),
                        });

                        // TODO: Interleaving imports/exports/statements exactly as written in fenced code
                        // requires parser ordering data; currently imports and exports preserve local vector order.
                        for import in &fenced.imports {
                            index.push_import(
                                scope.clone(),
                                order,
                                declaration_range(import.tokens()),
                                classify_import_specifier(import.specifier.to_string()),
                            );
                            order += 1;
                        }

                        for export in &fenced.exports {
                            index.push_export(
                                scope.clone(),
                                order,
                                declaration_range(export.tokens()),
                                SourceExportDeclaration {
                                    name: export.name.to_string(),
                                },
                            );
                            order += 1;
                        }

                        for (code, _) in &fenced.code {
                            if let MechCode::Statement(Statement::ContextDeclaration(context)) =
                                code
                            {
                                index.push_context(
                                    scope.clone(),
                                    order,
                                    declaration_range(context.tokens()),
                                    source_context_declaration(context),
                                );
                                order += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // TODO: index addressed paths from statements/expressions in a future PR.
        index
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

    pub fn program_imports(&self) -> Vec<SourceImportDeclaration> {
        self.imports_for_scope(&SourceScope::Program)
    }
    pub fn program_exports(&self) -> Vec<SourceExportDeclaration> {
        self.exports_for_scope(&SourceScope::Program)
    }
    pub fn program_contexts(&self) -> Vec<SourceContextDeclaration> {
        self.contexts_for_scope(&SourceScope::Program)
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

    pub fn interpreter_scopes(&self) -> Vec<SourceInterpreterId> {
        let mut scopes = Vec::new();
        for declaration in &self.declarations {
            let scope = match declaration {
                SourceDeclaration::Import(import) => &import.occurrence.scope,
                SourceDeclaration::Export(export) => &export.occurrence.scope,
                SourceDeclaration::Context(context) => &context.occurrence.scope,
            };
            if let SourceScope::Interpreter(interpreter) = scope {
                if !scopes.contains(interpreter) {
                    scopes.push(interpreter.clone());
                }
            }
        }
        scopes
    }

    fn push_import(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceImportDeclaration,
    ) {
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
}

fn source_context_declaration(context: &mech_core::ContextDeclaration) -> SourceContextDeclaration {
    let base = match &context.base {
        ContextBase::ResourceUri(uri) => SourceContextBase::ResourceUri(uri.to_string()),
        ContextBase::Context(name) => SourceContextBase::Context(name.to_string()),
    };
    let capabilities = context
        .capabilities
        .iter()
        .map(|capability| {
            let scope = match &capability.scope {
                ContextCapabilityScope::Path(path) => {
                    SourceContextCapabilityScope::Path(path.to_string())
                }
                ContextCapabilityScope::Wildcard(_) => SourceContextCapabilityScope::Wildcard,
            };
            SourceContextCapability {
                operation: capability.operation.to_string(),
                scope,
            }
        })
        .collect();
    SourceContextDeclaration {
        name: context.name.to_string(),
        base,
        capabilities,
    }
}

fn declaration_range(mut tokens: Vec<Token>) -> Option<SourceRange> {
    Token::merge_tokens(&mut tokens).map(|token| token.src_range)
}
