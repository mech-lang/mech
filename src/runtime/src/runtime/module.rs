// ---------------------------------------------------------------------------
// Module methods
// ---------------------------------------------------------------------------

use super::*;

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

      let record = self.module_builder.build_resolved_source(
        resolved,
        options.compiler_version.to_string(),
        options.language_edition.to_string(),
        options.target.to_string(),
        &feature_flags_owned,
        &dependency_versions,
        &concrete_capability_requirements,
      )?;

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

}