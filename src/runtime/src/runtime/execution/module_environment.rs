use crate::context::{RuntimeContextBinding, RuntimeContextRegistry};
use crate::resolver::{
    SourceExportDeclaration, SourceImportAlias, SourceImportDeclaration, SourceImportKind,
    SourceScope,
};
use crate::runtime::{RuntimeInvalidOperationError, RuntimeModuleImportConflict};
use crate::store::ModuleVersionRecord;
use mech_core::{MResult, MechError, MechSourceCode, ValRef, hash_str};
use mech_program::MechProgram;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RuntimeAddressTarget {
    Interpreter(SourceScope),
    Context(RuntimeContextBinding),
    Unknown,
}

pub(super) struct PreparedModuleScopeExecution {
    pub(super) record: ModuleVersionRecord,
    pub(super) source: MechSourceCode,
    pub(super) environment: HashMap<String, ValRef>,
}

pub(super) struct ProgramEnvironmentOverlay;

pub(super) fn imports_for_scope<'a>(
    record: &'a ModuleVersionRecord,
    scope: &SourceScope,
) -> &'a [SourceImportDeclaration] {
    if let Some(metadata) = record
        .scopes
        .iter()
        .find(|metadata| &metadata.scope == scope)
    {
        return metadata.imports.as_slice();
    }

    if matches!(scope, SourceScope::Program) {
        return record.imports.as_slice();
    }

    &[]
}

pub(super) fn import_has_source_edge(
    record: &ModuleVersionRecord,
    scope: &SourceScope,
    import: &SourceImportDeclaration,
) -> bool {
    record
        .import_edges
        .iter()
        .any(|edge| &edge.scope == scope && &edge.import == import)
}

pub(super) fn materialize_function_imports_for_scope(
    program: &mut MechProgram,
    record: &ModuleVersionRecord,
    scope: &SourceScope,
) -> MResult<()> {
    for import in imports_for_scope(record, scope) {
        if matches!(import.alias, Some(SourceImportAlias::Context(_))) {
            continue;
        }

        if import_has_source_edge(record, scope, import) {
            continue;
        }

        let module = import.module.as_deref().ok_or_else(|| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "materialize_function_import",
                    reason: format!(
                        "non-source import `{}` is missing module metadata",
                        import.specifier
                    ),
                },
                None,
            )
        })?;

        match &import.kind {
            SourceImportKind::Namespace => program.load_function_module(module)?,
            SourceImportKind::Single { .. } => {
                let item = import.item.as_deref().ok_or_else(|| {
                    MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "materialize_function_import",
                            reason: format!(
                                "item import `{}` is missing item metadata",
                                import.specifier
                            ),
                        },
                        None,
                    )
                })?;

                match &import.alias {
                    Some(SourceImportAlias::Value(alias)) => {
                        program.import_function_module_item_as(module, item, alias.as_str())?;
                    }
                    Some(SourceImportAlias::Context(_)) => {
                        unreachable!("context imports were filtered above")
                    }
                    None => program.import_function_module_item(module, item)?,
                }
            }
            SourceImportKind::Wildcard => program.import_function_module_glob(module)?,
            SourceImportKind::DependencyOnly => {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "materialize_function_import",
                        reason: format!(
                            "required source dependency `{}` has no module import edge",
                            import.specifier
                        ),
                    },
                    None,
                ));
            }
        }
    }

    Ok(())
}

impl ProgramEnvironmentOverlay {
    pub(super) fn install(
        program: &mut MechProgram,
        environment: &HashMap<String, ValRef>,
    ) -> MResult<()> {
        let mut ids = HashMap::new();
        for name in environment.keys() {
            let id = hash_str(name);
            if let Some(first) = ids.insert(id, name.clone()) {
                if first != *name {
                    return Err(MechError::new(
                        RuntimeModuleImportConflict {
                            binding: name.clone(),
                            first_import: first,
                            second_import: name.clone(),
                        },
                        None,
                    ));
                }
            }
        }

        let symbols = program.interpreter_mut().symbols();
        let mut symbols_brrw = symbols.borrow_mut();
        for (name, value_ref) in environment {
            let id = hash_str(name);
            let previous_dictionary = symbols_brrw.dictionary.borrow().get(&id).cloned();
            if let Some(previous_name) = &previous_dictionary {
                if previous_name != name && !environment.contains_key(previous_name) {
                    return Err(MechError::new(
                        RuntimeModuleImportConflict {
                            binding: name.clone(),
                            first_import: previous_name.clone(),
                            second_import: name.clone(),
                        },
                        None,
                    ));
                }
            }
            symbols_brrw.symbols.insert(id, value_ref.clone());
            symbols_brrw
                .dictionary
                .borrow_mut()
                .insert(id, name.clone());
        }

        Ok(())
    }
}

pub(super) fn execution_scope_for_extracted_module_source(scope: &SourceScope) -> SourceScope {
    match scope {
        SourceScope::Program => SourceScope::Program,
        SourceScope::Interpreter(_) => SourceScope::Program,
    }
}

pub(super) fn resolve_runtime_address_target(
    record: &ModuleVersionRecord,
    scope: &SourceScope,
    context_registry: &RuntimeContextRegistry,
    target: &str,
) -> RuntimeAddressTarget {
    if !matches!(scope, SourceScope::Program) {
        if let Some(binding) = context_registry.get(target) {
            return RuntimeAddressTarget::Context(binding.clone());
        }
    }

    for metadata in &record.scopes {
        if let SourceScope::Interpreter(interpreter) = &metadata.scope {
            if interpreter.namespace_str == target {
                return RuntimeAddressTarget::Interpreter(metadata.scope.clone());
            }
        }
    }

    if let Some(binding) = context_registry.get(target) {
        return RuntimeAddressTarget::Context(binding.clone());
    }

    RuntimeAddressTarget::Unknown
}

pub(super) fn context_registry_for_scope(
    record: &ModuleVersionRecord,
    scope: &SourceScope,
) -> MResult<RuntimeContextRegistry> {
    let declarations = record
        .scopes
        .iter()
        .find(|metadata| &metadata.scope == scope)
        .map(|metadata| metadata.contexts.as_slice())
        .unwrap_or(&[]);
    RuntimeContextRegistry::from_declarations(scope.clone(), declarations)
}

pub(super) fn exports_for_scope<'a>(
    record: &'a ModuleVersionRecord,
    scope: &SourceScope,
) -> &'a [SourceExportDeclaration] {
    record
        .scopes
        .iter()
        .find(|metadata| &metadata.scope == scope)
        .map(|metadata| metadata.exports.as_slice())
        .unwrap_or(&[])
}

pub(super) fn merge_module_environment(
    target: &mut HashMap<String, ValRef>,
    source: HashMap<String, ValRef>,
    first_import: &str,
    second_import: &str,
) -> MResult<()> {
    for (binding, value) in source {
        if target.contains_key(&binding) {
            return Err(MechError::new(
                RuntimeModuleImportConflict {
                    binding,
                    first_import: first_import.to_string(),
                    second_import: second_import.to_string(),
                },
                None,
            ));
        }
        target.insert(binding, value);
    }
    Ok(())
}
