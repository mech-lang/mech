//! Resolve typed module declarations against the configured function catalog.
use super::*;
use mech_syntax::document::{CanonicalModuleImportBodySyntax, ModuleImportSyntax};

impl SemanticBuilder {
    pub(super) fn register_document_imports(
        &mut self,
        units: &[DocumentUnit],
        resolved_source_modules: &BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        for unit in units {
            match unit {
                DocumentUnit::Import(import) => {
                    self.register_module_import(import, resolved_source_modules)?
                }
                DocumentUnit::Fence(_, _, units) => {
                    self.register_document_imports(units, resolved_source_modules)?
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn register_module_import(
        &mut self,
        import: &ModuleImportSyntax,
        resolved_source_modules: &BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        let syntax = import.syntax();
        let body = self.required(import.body(), syntax, "a module import body")?;
        let (module, items) = match body {
            CanonicalModuleImportBodySyntax::Module(body) => {
                let module = self.required(body.module(), syntax, "a module name")?;
                let module = node_text(module.syntax())?;
                let items =
                    self.function_catalog
                        .as_ref()
                        .map(|catalog| {
                            catalog
                                .module_exports(&module)
                                .filter_map(|export| {
                                    export.item.as_ref().map(|item| {
                                        (item.clone(), Some(format!("{module}/{item}")))
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                (module, items)
            }
            CanonicalModuleImportBodySyntax::AliasedItem(body) => {
                let alias = self.required(body.alias(), syntax, "a module item alias")?;
                if alias.context().is_some() {
                    return Ok(());
                }
                let alias = self.required(
                    alias.value().and_then(|alias| alias.path()),
                    syntax,
                    "a value alias",
                )?;
                let module = self.required(body.module(), syntax, "a module name")?;
                let item = self.required(body.item(), syntax, "a module item")?;
                (
                    node_text(module.syntax())?,
                    vec![(node_text(item.syntax())?, Some(node_text(alias.syntax())?))],
                )
            }
            CanonicalModuleImportBodySyntax::Suffix(body) => {
                let module = node_text(
                    self.required(body.module(), syntax, "a module name")?
                        .syntax(),
                )?;
                if body.is_glob() {
                    let items = self
                        .function_catalog
                        .as_ref()
                        .map(|catalog| {
                            catalog
                                .module_exports(&module)
                                .filter_map(|export| {
                                    export.item.as_ref().map(|item| (item.clone(), None))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    (module, items)
                } else if let Some(group) = body.group() {
                    let items = group
                        .items()
                        .map(|item| {
                            let path = self.required(item.path(), syntax, "a module item")?;
                            Ok((node_text(path.syntax())?, None))
                        })
                        .collect::<Result<_, SourceSemanticError>>()?;
                    (module, items)
                } else {
                    let item = self.required(body.item(), syntax, "a module item")?;
                    (module, vec![(node_text(item.syntax())?, None)])
                }
            }
        };
        if resolved_source_modules.contains(&module) {
            return Ok(());
        }
        let Some(catalog) = self.function_catalog.as_ref() else {
            return Ok(());
        };
        // Source-module value exports are bound by the resolver. Only catalog
        // modules contribute function names at this boundary.
        if !catalog.has_module(&module) {
            return Err(SourceSemanticError {
                code: "source-semantics/unknown-function-import",
                message: format!(
                    "module {module} is not available in the configured catalog or resolved source modules"
                ),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        for (item, alias) in items {
            let export =
                catalog
                    .module_export(&module, &item)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unknown-function-import",
                        message: format!("module {module} has no function {item}"),
                        anchor: SourceSemanticAnchor::for_node(syntax),
                    })?;
            let name = alias.unwrap_or_else(|| item.rsplit('/').next().unwrap_or(&item).to_owned());
            let environment = self
                .function_environment
                .as_mut()
                .expect("a configured catalog has a function environment");
            if (self.function_imports.contains(&name)
                && environment.resolve_name(&name)
                    != Some(crate::FunctionBinding::CatalogOperation(export.operation)))
                || self.local_functions.contains_key(&name)
            {
                return Err(SourceSemanticError {
                    code: "source-semantics/conflicting-function-import",
                    message: format!("function import {name} conflicts with another declaration"),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
            environment
                .bind_catalog_export(export, &name)
                .map_err(|error| SourceSemanticError {
                    code: "source-semantics/invalid-function-import",
                    message: error.display_message(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                })?;
            self.function_imports.insert(name);
        }
        Ok(())
    }
}
