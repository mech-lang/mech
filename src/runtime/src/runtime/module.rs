// Module methods
// -----------------------------------------------------------------------------

// Modules represent a unit of code that can be compiled and executed within the runtime. They are versioned, allowing for updates and changes over time while maintaining references to specific versions. The module methods handle the resolution, compilation, storage, and activation of modules within the runtime. They also manage the dependencies between modules, ensuring that imports are correctly resolved and that circular dependencies are handled gracefully.

// The included methods are:

// - `ensure_module`: Ensures that a module with the given canonical URI exists in the store, creating it if it doesn't.
// - `resolve_source`: Resolves a source request to a resolved source, which includes the canonical URI and the source code. This method does not emit any events.
// - `resolve_source_evented`: A convenience method that creates a runtime context and resolves a source request, emitting events as needed.
// - `store_resolved_module_source`: Stores a resolved module source in the runtime, compiling it and its dependencies as needed, and returns the ModuleVersionId of the stored module.
// - `resolve_and_store_module_source`: Resolves a source request and stores the resulting module source in the runtime, compiling it and its dependencies as needed, and returns the ModuleVersionId of the stored module.
// - `put_source_module`: A convenience method that takes raw source code, constructs a resolved source, and stores it as a module in the runtime, returning the ModuleVersionId of the stored module.
// - `activate_module_version`: Activates a specific version of a module, making it the active version for that module.
// - `active_module_version`: Retrieves the active version of a module, if any.

use super::*;
use crate::SourceIndex;

fn source_index_for_module_record_source(
  source: &mech_core::MechSourceCode,
) -> MResult<Option<SourceIndex>> {
  match source {
    mech_core::MechSourceCode::Tree(tree) => Ok(Some(SourceIndex::from_program(tree))),
    mech_core::MechSourceCode::String(source) => {
      let tree = mech_syntax::parser::parse(source.trim())?;
      Ok(Some(SourceIndex::from_program(&tree)))
    }
    _ => Ok(None),
  }
}

impl MechRuntime {
  pub fn ensure_module(
    &mut self,
    name: &str,
    canonical_uri: &str,
  ) -> MResult<ModuleId> {
    if let Some(module) = self.store.find_module_by_name(canonical_uri)? {
      return Ok(module.id);
    }

    let id = module_id(canonical_uri);
    let module = ModuleRecord::new(id, canonical_uri)
      .with_description(name.to_string());

    self.store.put_module(module)
  }

  fn materialize_manifest_context_imports(
    &mut self,
    record: &mut crate::RuntimeModuleRecord,
  ) -> MResult<()> {
    let Some(index) = source_index_for_module_record_source(&record.source)? else {
      return self.materialize_manifest_context_imports_legacy(record);
    };

    let mut all_contexts = Vec::new();

    for scope in &mut record.scopes {
      let contexts = self.context_declarations_from_index_scope(&index, &scope.scope)?;
      scope.contexts = contexts;
      all_contexts.extend(scope.contexts.iter().cloned());
    }

    record.contexts = all_contexts;
    Ok(())
  }

  fn materialize_manifest_context_imports_legacy(
    &mut self,
    record: &mut crate::RuntimeModuleRecord,
  ) -> MResult<()> {
    for scope in &mut record.scopes {
      let context_imports = scope
        .imports
        .iter()
        .filter_map(|import| match &import.alias {
          Some(SourceImportAlias::Context(alias)) => Some((import, alias.clone())),
          _ => None,
        })
        .collect::<Vec<_>>();

      for (import, alias) in context_imports {
        if scope.contexts.iter().any(|context| context.name == alias) {
          return Err(MechError::new(RuntimeInvalidOperationError {
            operation: "materialize_manifest_context_imports",
            reason: format!("context import duplicates an existing context binding `{alias}`"),
          }, None));
        }

        let module = import.module.as_deref().ok_or_else(|| {
          MechError::new(RuntimeInvalidOperationError {
            operation: "materialize_manifest_context_imports",
            reason: format!("context import `{}` is missing module metadata", import.specifier),
          }, None)
        })?;
        let item = import.item.as_deref().ok_or_else(|| {
          MechError::new(RuntimeInvalidOperationError {
            operation: "materialize_manifest_context_imports",
            reason: format!("context import `{}` is missing item metadata", import.specifier),
          }, None)
        })?;

        let export = self.module_manifests.context_export(module, item)?;
        let declaration = crate::SourceContextDeclaration {
          name: alias,
          base: crate::SourceContextBase::ResourceUri(export.base_uri.clone()),
          capabilities: export.operations.iter().map(|operation| crate::SourceContextCapability {
            operation: operation.clone(),
            scope: crate::SourceContextCapabilityScope::Wildcard,
          }).collect(),
        };

        scope.contexts.push(declaration.clone());
        record.contexts.push(declaration);
      }
    }

    Ok(())
  }

  fn validate_runtime_module_record_address_targets(
    record: &crate::RuntimeModuleRecord,
  ) -> MResult<()> {
    let mut interpreter_targets: HashMap<String, String> = HashMap::new();

    for metadata in &record.scopes {
      if let SourceScope::Interpreter(interpreter) = &metadata.scope {
        if let Some(first_kind) = interpreter_targets.insert(interpreter.namespace_str.clone(), "interpreter".to_string()) {
          return Err(MechError::new(
            crate::resolver::AddressTargetNameConflict {
              name: interpreter.namespace_str.clone(),
              first_kind,
              second_kind: "interpreter".to_string(),
            },
            None,
          ));
        }
      }
    }

    for metadata in &record.scopes {
      let mut scope_targets = HashMap::new();
      match &metadata.scope {
        SourceScope::Program => {
          for (name, kind) in &interpreter_targets {
            scope_targets.insert(name.clone(), kind.clone());
          }
        }
        SourceScope::Interpreter(interpreter) => {
          scope_targets.insert(interpreter.namespace_str.clone(), "interpreter".to_string());
        }
      }

      for context in &metadata.contexts {
        if let Some(first_kind) = scope_targets.insert(context.name.clone(), "context".to_string()) {
          return Err(MechError::new(
            crate::resolver::AddressTargetNameConflict {
              name: context.name.clone(),
              first_kind,
              second_kind: "context".to_string(),
            },
            None,
          ));
        }
      }
    }

    Ok(())
  }

  pub fn resolve_source(
    &self,
    request: impl Into<SourceRequest>,
  ) -> MResult<Option<ResolvedSource>> {
    let request = request.into();
    request.validate()?;

    self.source_resolver.resolve(&request)
  }

  pub fn resolve_source_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: impl Into<SourceRequest>,
  ) -> MResult<Option<ResolvedSource>> {
    context.validate()?;
    context.charge_step()?;

    let request = request.into();
    request.validate()?;

    let resolved = self.source_resolver.resolve(&request)?;

    if let Some(source) = &resolved {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::SourceResolved {
          canonical_uri: source.canonical_uri.clone(),
        },
      )?;
    }

    Ok(resolved)
  }

  pub fn resolve_source_evented(
    &mut self,
    request: impl Into<SourceRequest>,
  ) -> MResult<Option<ResolvedSource>> {
    let mut context = self.runtime_context()?;
    self.resolve_source_with_context(&mut context, request)
  }

  pub fn store_resolved_module_source(
    &mut self,
    resolved: ResolvedSource,
    options: ModuleBuildOptions<'_>,
  ) -> MResult<ModuleVersionId> {
    let mut context = self.runtime_context()?;

    self.build_module_from_resolved_source_with_context(
      &mut context,
      resolved,
      options,
    )
  }

  pub fn build_module_from_resolved_source_with_context(
    &mut self,
    context: &mut RuntimeContext,
    resolved: ResolvedSource,
    options: ModuleBuildOptions<'_>,
  ) -> MResult<ModuleVersionId> {
    let mut dependency_graph = ModuleDependencyGraph::new();

    self.build_module_from_resolved_source_with_context_and_graph(
      context,
      resolved,
      options,
      &mut dependency_graph,
    )
  }

  fn build_module_from_resolved_source_with_context_and_graph(
    &mut self,
    context: &mut RuntimeContext,
    resolved: ResolvedSource,
    options: ModuleBuildOptions<'_>,
    dependency_graph: &mut ModuleDependencyGraph,
  ) -> MResult<ModuleVersionId> {
    context.validate()?;
    context.charge_step()?;

    let canonical_uri = resolved.canonical_uri.clone();

    if let Some(module_version) = dependency_graph.cached_version(&canonical_uri) {
      return Ok(module_version);
    }

    if let Some(cycle) = dependency_graph.enter(&canonical_uri) {
      return Err(MechError::new(
        RuntimeModuleDependencyCycleError {
          cycle,
        },
        None,
      ));
    }

    let result = (|| -> MResult<ModuleVersionId> {
      let feature_flags_owned = options
        .feature_flags
        .iter()
        .map(|flag| (*flag).to_string())
        .collect::<Vec<_>>();

      let mut resolved = resolved;

      let explicit_capability_requirements = options
        .capability_requirements
        .iter()
        .map(|resource| {
          CapabilityRequest::from_keys(
            context.subject.clone(),
            "use",
            (*resource).to_string(),
          )
        })
        .collect::<Vec<_>>();

      resolved
        .capability_requirements
        .extend(explicit_capability_requirements);

      let concrete_capability_requirements =
        resolved.capability_requirements.clone();

      let mut dependency_versions = Vec::new();
      let mut flat_import_dependencies = Vec::new();

      for import in &resolved.imports {
        if !crate::resolver::import_requires_source_dependency(import) {
          continue;
        }
        let dependency_request = crate::resolver::source_request_for_import(import, Some(&canonical_uri));
        let dependency_version = self
          .build_module_from_request_with_context_and_graph(
            context,
            dependency_request.clone(),
            options,
            dependency_graph,
          )?
          .ok_or_else(|| {
            MechError::new(
              RuntimeModuleDependencyMissingError {
                module: canonical_uri.clone(),
                specifier: dependency_request.specifier.clone(),
                referrer: dependency_request.referrer.clone(),
              },
              None,
            )
          })?;
        dependency_versions.push(dependency_version);
        flat_import_dependencies.push((import.clone(), dependency_version));
      }

      let mut import_edges = Vec::new();
      let mut matched_flat_imports = vec![false; flat_import_dependencies.len()];
      for scope_metadata in &resolved.scopes {
        for import in &scope_metadata.imports {
          if !crate::resolver::import_requires_source_dependency(import) {
            continue;
          }
          let Some((index, (_, dependency_version))) = flat_import_dependencies
            .iter()
            .enumerate()
            .find(|(index, (flat_import, _))| {
              !matched_flat_imports[*index] && flat_import == import
            }) else {
              return Err(MechError::new(RuntimeInvalidOperationError { operation: "resolve_and_store_module_source", reason: format!("scoped import `{}` was not found in flat imports", import.specifier) }, None));
            };
          matched_flat_imports[index] = true;
          import_edges.push(ModuleImportEdge {
            scope: scope_metadata.scope.clone(),
            import: import.clone(),
            dependency: *dependency_version,
          });
        }
      }

      let mut record = self.module_builder.build_resolved_source(
        resolved,
        options.compiler_version.to_string(),
        options.language_edition.to_string(),
        options.target.to_string(),
        &feature_flags_owned,
        &dependency_versions,
        &concrete_capability_requirements,
      )?;

      self.materialize_manifest_context_imports(&mut record)?;
      Self::validate_runtime_module_record_address_targets(&record)?;

      let module = self.ensure_module(
        &record.name,
        &record.canonical_uri,
      )?;

      debug_assert_eq!(
        module,
        record.module_id,
        "ModuleBuilder and runtime module store derived different ModuleId values",
      );

      if self
        .store
        .get_module_version(record.module_version)?
        .is_some()
      {
        dependency_graph.cache_version(&canonical_uri, record.module_version);
        return Ok(record.module_version);
      }

      let version = ModuleVersionRecord::new(
        record.module_version,
        module,
        1,
      )
      .with_source(record.source)
      .with_exports(record.exports)
      .with_imports(record.imports)
      .with_contexts(record.contexts)
      .with_address_references(record.address_references)
      .with_scopes(record.scopes)
      .with_dependencies(record.dependency_versions)
      .with_import_edges(import_edges)
      .with_capability_requirements(record.capability_requirements);
      validate_module_import_edges(&version)?;

      self.store.put_module_version(version)?;

      dependency_graph.cache_version(&canonical_uri, record.module_version);

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ModuleCompiled {
          module_version: record.module_version,
        },
      )?;

      Ok(record.module_version)
    })();

    dependency_graph.leave(&canonical_uri);

    result
  }

  fn build_module_from_request_with_context_and_graph(
    &mut self,
    context: &mut RuntimeContext,
    request: impl Into<SourceRequest>,
    options: ModuleBuildOptions<'_>,
    dependency_graph: &mut ModuleDependencyGraph,
  ) -> MResult<Option<ModuleVersionId>> {
    let Some(resolved) = self.resolve_source_with_context(context, request)? else {
      return Ok(None);
    };

    Ok(Some(
      self.build_module_from_resolved_source_with_context_and_graph(
        context,
        resolved,
        options,
        dependency_graph,
      )?,
    ))
  }

  pub fn resolve_and_store_module_source(
    &mut self,
    request: impl Into<SourceRequest>,
    options: ModuleBuildOptions<'_>,
  ) -> MResult<Option<ModuleVersionId>> {
    let mut context = self.runtime_context()?;

    self.build_module_from_request_with_context(
      &mut context,
      request,
      options,
    )
  }

  pub fn build_module_from_request_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: impl Into<SourceRequest>,
    options: ModuleBuildOptions<'_>,
  ) -> MResult<Option<ModuleVersionId>> {
    let mut dependency_graph = ModuleDependencyGraph::new();

    self.build_module_from_request_with_context_and_graph(
      context,
      request,
      options,
      &mut dependency_graph,
    )
  }

  pub fn put_source_module(
    &mut self,
    name: &str,
    canonical_uri: &str,
    source: &str,
    options: ModuleBuildOptions<'_>,
  ) -> MResult<ModuleVersionId> {
    let mut context = self.runtime_context()?;

    self.put_source_module_with_context(
      &mut context,
      name,
      canonical_uri,
      source,
      options,
    )
  }

  pub fn put_source_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    name: &str,
    canonical_uri: &str,
    source: &str,
    options: ModuleBuildOptions<'_>,
  ) -> MResult<ModuleVersionId> {
    let resolved = ResolvedSource::new(
      name,
      canonical_uri,
      MechSourceCode::String(source.to_string()),
    );

    self.build_module_from_resolved_source_with_context(
      context,
      resolved,
      options,
    )
  }

  pub fn activate_module_version(
    &mut self,
    module: ModuleId,
    version: ModuleVersionId,
  ) -> MResult<()> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.activate_module_version_with_context(&mut context, module, version)
  }

  pub fn activate_module_version_with_context(
    &mut self,
    context: &mut RuntimeContext,
    module: ModuleId,
    version: ModuleVersionId,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;

    self.store.set_active_module_version(module, version)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleActivated {
        module_version: version,
      },
    )?;

    Ok(())
  }

  pub fn active_module_version(&self, module: ModuleId) -> MResult<Option<ModuleVersionId>> {
    self.store.get_active_module_version(module)
  }

  pub(crate) fn workspace_module_records(
    &self,
    version: ModuleVersionId,
  ) -> MResult<Option<(ModuleRecord, ModuleVersionRecord)>> {
    let Some(version_record) = self.store.get_module_version(version)? else {
      return Ok(None);
    };
    let Some(module_record) = self.store.get_module(version_record.module)? else {
      return Ok(None);
    };

    Ok(Some((module_record, version_record)))
  }

}
