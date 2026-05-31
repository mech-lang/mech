use mech_core::{hash_str, MechSourceCode, Ref, Value};
use mech_runtime::{BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId, ClosureHostFunction, FileSourceResolver, InMemoryDocsProvider, ModuleScopeMetadata, ModuleBuildOptions, ResolvedSource, RuntimeBuilder, RuntimeCapabilityGrant, RuntimeCapabilityGrantSpec, RuntimeCapabilityOperation, RuntimeConfigSpec, RuntimeContextRegistry, RuntimeDocsEntrySpec, RuntimeInMemoryDocsResourceSpec, RuntimeResourceConfigSpec, SourceContextBase, SourceContextCapability, SourceContextCapabilityScope, SourceContextDeclaration, SourceInterpreterId, SourceKind, SourceRequest, SourceResolver, SourceScope, RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceRegistry, RuntimeResourceWriteRequest, RuntimeWorkspace, RuntimeWorkspaceChangeKind, RuntimeWorkspaceConfig, RuntimeWorkspaceDiagnosticSeverity};

fn setup_modules(main_source: &str) -> std::path::PathBuf {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-smoke-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\nsecret := 42\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), main_source).unwrap();
  root
}



fn runtime_with_root(root: &std::path::Path) -> mech_runtime::MechRuntime {
  RuntimeBuilder::new().source_resolver(FileSourceResolver::new(root)).build().unwrap()
}

fn docs_provider_with(
  base_uri: &str,
  path: &str,
  value: Value,
) -> InMemoryDocsProvider {
  InMemoryDocsProvider::new()
    .with_value(base_uri, path, value)
    .unwrap()
}

fn read_grant(resource: &str, path: &str) -> RuntimeCapabilityGrantSpec {
  RuntimeCapabilityGrantSpec::new("task://main", resource)
    .with_operation(RuntimeCapabilityOperation::Read)
    .with_path(path)
}

fn runtime_read_grant(resource: &str, path: &str) -> RuntimeCapabilityGrant {
  RuntimeCapabilityGrant {
    subject: "task://main".to_string(),
    resource: resource.to_string(),
    operations: vec![RuntimeCapabilityOperation::Read],
    paths: vec![path.to_string()],
  }
}

fn bool_value(value: bool) -> Value {
  Value::Bool(Ref::new(value))
}

fn docs_entry(path: &str, value: Value) -> RuntimeDocsEntrySpec {
  RuntimeDocsEntrySpec {
    path: path.to_string(),
    value,
  }
}

fn module_options() -> ModuleBuildOptions<'static> {
  ModuleBuildOptions::new("test", "v0.3", "native", &[], &[])
}

fn assert_bool_true(result: Value, label: &str) {
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from {label}, got {:?}", other),
  }
}

fn assert_bool_false(result: Value, label: &str) {
  match result {
    Value::Bool(value) => assert!(!*value.borrow()),
    other => panic!("expected bool result from {label}, got {:?}", other),
  }
}


#[test]
fn in_memory_docs_provider_write_then_read_returns_value() {
  let mut provider = InMemoryDocsProvider::new();
  provider.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), value: bool_value(true) }).unwrap();
  let value = provider.read(RuntimeResourceReadRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string() }).unwrap();
  assert_bool_true(value, "provider write then read");
}

#[test]
fn resource_registry_write_then_read_returns_value() {
  let mut registry = RuntimeResourceRegistry::new();
  registry.register_provider(Box::new(InMemoryDocsProvider::new())).unwrap();
  registry.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), value: bool_value(true) }).unwrap();
  let value = registry.read(RuntimeResourceReadRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string() }).unwrap();
  assert_bool_true(value, "registry write then read");
}

#[test]
fn resource_registry_write_missing_provider_fails() {
  let mut registry = RuntimeResourceRegistry::new();
  let result = registry.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), value: bool_value(true) });
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderNotFound"), "expected missing provider error, got {error}");
  assert!(error.contains("docs"), "expected scheme in error, got {error}");
  assert!(error.contains("docs://manual"), "expected base URI in error, got {error}");
}

#[test]
fn in_memory_docs_write_invalid_scheme_fails() {
  let mut provider = InMemoryDocsProvider::new();
  let result = provider.write(RuntimeResourceWriteRequest { base_uri: "db://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), value: bool_value(true) });
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceInvalidUri"), "expected invalid URI error, got {error}");
  assert!(error.contains("docs"), "expected docs scheme requirement, got {error}");
}

#[test]
fn in_memory_docs_write_empty_path_fails() {
  let mut provider = InMemoryDocsProvider::new();
  let result = provider.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: String::new(), context_name: "manual".to_string(), value: bool_value(true) });
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceInvalidUri"), "expected invalid URI error, got {error}");
  assert!(error.contains("resource path cannot be empty"), "expected empty path error, got {error}");
}

#[derive(Debug)]
struct ReadOnlyDocsProvider;

impl RuntimeResourceProvider for ReadOnlyDocsProvider {
  fn scheme(&self) -> &str { "docs" }
  fn read(&self, _request: RuntimeResourceReadRequest) -> mech_core::MResult<Value> { unreachable!("read is not needed for the default write behavior test") }
}

#[test]
fn provider_default_write_is_unsupported() {
  let mut provider = ReadOnlyDocsProvider;
  let result = provider.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), value: bool_value(true) });
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceWriteUnsupported"), "expected unsupported write error, got {error}");
  assert!(error.contains("docs"), "expected provider scheme in error, got {error}");
  assert!(error.contains("docs://manual"), "expected base URI in error, got {error}");
  assert!(error.contains("intro/title"), "expected path in error, got {error}");
}

#[test]
fn interpreter_context_name_conflict_fails_resolution() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n@foo := docs://foo{:read(ok)}\n");
  let mut runtime = runtime_with_root(&root);
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressTargetNameConflict"), "expected address target conflict, got {error}");
  assert!(error.contains("foo"), "expected conflicting target in error, got {error}");
}

#[test]
fn duplicate_context_address_target_fails_resolution() {
  let root = setup_modules("@foo := docs://a{:read(*)}\n@foo := docs://b{:read(*)}\n");
  let mut runtime = runtime_with_root(&root);
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressTargetNameConflict"), "expected address target conflict, got {error}");
  assert!(error.contains("foo"), "expected conflicting target in error, got {error}");
}

#[test]
fn duplicate_interpreter_address_target_fails_resolution() {
  let root = setup_modules("~~~mech:foo\n~~~\n\n~~~mech:foo\n~~~\n");
  let mut runtime = runtime_with_root(&root);
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressTargetNameConflict"), "expected address target conflict, got {error}");
  assert!(error.contains("foo"), "expected conflicting target in error, got {error}");
}

#[test]
fn resolved_source_scope_address_target_conflict_fails_validation() {
  let foo = SourceInterpreterId {
    namespace: hash_str("foo"),
    namespace_str: "foo".to_string(),
  };
  let resolved = ResolvedSource::new(
    "main.mec",
    "memory:main.mec",
    MechSourceCode::String("ok := true\n".to_string()),
  )
  .with_kind(SourceKind::Mech)
  .with_scopes(vec![
    ModuleScopeMetadata {
      scope: SourceScope::Interpreter(foo),
      imports: Vec::new(),
      exports: Vec::new(),
      contexts: Vec::new(),
      address_references: Vec::new(),
    },
    ModuleScopeMetadata {
      scope: SourceScope::Program,
      imports: Vec::new(),
      exports: Vec::new(),
      contexts: vec![SourceContextDeclaration {
        name: "foo".to_string(),
        base: SourceContextBase::ResourceUri("docs://foo".to_string()),
        capabilities: Vec::new(),
      }],
      address_references: Vec::new(),
    },
  ]);

  let result = resolved.validate();
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressTargetNameConflict"), "expected address target conflict, got {error}");
  assert!(error.contains("foo"), "expected conflicting target in error, got {error}");
}

#[test]
fn distinct_interpreter_and_context_names_pass() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n@manual := docs://foo{:read(ok)}\n\nresult := ok@foo\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "distinct interpreter/context names");
}

#[test]
fn context_docs_read_returns_value() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "context docs read");
}

#[test]
fn config_spec_registers_in_memory_docs_resource() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  ).with_capability_grant(read_grant("docs://manual", "intro/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "config spec docs read");
}

#[test]
fn config_spec_merges_multiple_docs_bases_into_one_provider() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n@guide := docs://guide{:read(start/title)}\n\nmanual-ok := intro/title@manual\nguide-ok := start/title@guide\n\nresult := manual-ok && guide-ok\n");
  let spec = RuntimeConfigSpec::new()
    .with_resource(RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ))
    .with_resource(RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://guide")
        .with_entry("start/title", bool_value(true)),
    ))
    .with_capability_grant(read_grant("docs://manual", "intro/title"))
    .with_capability_grant(read_grant("docs://guide", "start/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "merged config spec docs bases");
}

#[test]
fn config_spec_multiple_entries_same_base() {
  let root = setup_modules("@manual := docs://manual{:read(intro/*)}\n\na := intro/title@manual\nb := intro/subtitle@manual\n\nresult := a && b\n");
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(RuntimeInMemoryDocsResourceSpec {
      base_uri: "docs://manual".to_string(),
      entries: vec![
        docs_entry("intro/title", bool_value(true)),
        docs_entry("intro/subtitle", bool_value(true)),
      ],
    }),
  ).with_capability_grant(read_grant("docs://manual", "intro/*"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "config spec docs entries under one base");
}

#[test]
fn config_spec_later_duplicate_path_overwrites_earlier() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(false))
        .with_entry("intro/title", bool_value(true)),
    ),
  ).with_capability_grant(read_grant("docs://manual", "intro/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "later duplicate config spec docs path");
}

#[test]
fn config_spec_invalid_docs_uri_fails_build() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("db://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  );
  let result = RuntimeBuilder::new().config_spec(spec).build();
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceInvalidUri"), "expected invalid URI error, got {error}");
  assert!(error.contains("docs"), "expected docs scheme requirement, got {error}");
}

#[test]
fn config_spec_invalid_empty_path_fails_build() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("", bool_value(true)),
    ),
  );
  let result = RuntimeBuilder::new().config_spec(spec).build();
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceInvalidUri"), "expected invalid URI error, got {error}");
  assert!(error.contains("resource path cannot be empty"), "expected empty path error, got {error}");
}

#[test]
fn config_spec_and_direct_docs_provider_conflict() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  );
  let result = RuntimeBuilder::new()
    .config_spec(spec)
    .in_memory_docs(InMemoryDocsProvider::new())
    .build();
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderConflict"), "expected provider conflict, got {error}");
  assert!(error.contains("docs"), "expected docs scheme in conflict, got {error}");
}

#[test]
fn direct_provider_registration_still_works() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "direct docs provider registration");
}

#[test]
fn apply_config_spec_after_build_registers_docs() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  ).with_capability_grant(read_grant("docs://manual", "intro/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .build()
    .unwrap();
  runtime.apply_config_spec(spec).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "applied config spec docs read");
}

#[test]
fn apply_config_spec_conflicts_with_existing_docs_provider() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  );
  let mut runtime = RuntimeBuilder::new()
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();
  let result = runtime.apply_config_spec(spec);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderConflict"), "expected provider conflict, got {error}");
  assert!(error.contains("docs"), "expected docs scheme in conflict, got {error}");
}

#[test]
fn unknown_address_target_is_explicit() {
  let root = setup_modules("result := ok@missing\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected unknown address target error, got {error}");
  assert!(error.contains("missing"), "expected missing target in error, got {error}");
}

#[test]
fn interpreter_address_still_works() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := ok@foo\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "interpreter address");
}

#[test]
fn string_and_comment_address_text_are_not_targets() {
  let root = setup_modules("~~~mech:bar\nbroken := missing\n<+ broken\n~~~\n\ntext := \"@bar\"\n-- @bar\n\nok := true\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  let program = main.scopes.iter().find(|scope| scope.scope == SourceScope::Program).unwrap();
  assert!(program.address_references.is_empty(), "expected no program address references, got {:?}", program.address_references);
  let result = runtime.run_module(version);
  assert!(result.is_ok(), "expected string/comment @bar not to execute interpreter, got {result:?}");
}

fn foo_scope(runtime: &mech_runtime::MechRuntime, version: mech_runtime::ModuleVersionId) -> SourceScope {
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  main.scopes.iter().find_map(|metadata| match &metadata.scope {
    SourceScope::Interpreter(interpreter) if interpreter.namespace_str == "foo" => Some(metadata.scope.clone()),
    SourceScope::Interpreter(_) | SourceScope::Program => None,
  }).unwrap()
}

fn interpreter_scope_named(runtime: &mech_runtime::MechRuntime, version: mech_runtime::ModuleVersionId, name: &str) -> SourceScope {
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  main.scopes.iter().find_map(|metadata| match &metadata.scope {
    SourceScope::Interpreter(interpreter) if interpreter.namespace_str == name => Some(metadata.scope.clone()),
    SourceScope::Interpreter(_) | SourceScope::Program => None,
  }).unwrap()
}


#[test]
fn context_docs_read_without_provider_fails() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let mut runtime = runtime_with_root(&root);
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderNotFound"), "expected missing provider error, got {error}");
  assert!(error.contains("docs"), "expected scheme in error, got {error}");
  assert!(error.contains("docs://manual"), "expected base URI in error, got {error}");
}

#[test]
fn context_docs_read_missing_path_fails() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourcePathNotFound"), "expected missing path error, got {error}");
  assert!(error.contains("intro/title"), "expected path in error, got {error}");
  assert!(error.contains("docs://manual"), "expected base URI in error, got {error}");
}

#[test]
fn context_docs_read_denied_by_capability_fails() {
  let root = setup_modules("@manual := docs://manual{:read(other/path)}\n\nresult := intro/title@manual\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceCapabilityDenied"), "expected capability error, got {error}");
  assert!(error.contains("manual"), "expected context name in error, got {error}");
  assert!(error.contains("read"), "expected operation in error, got {error}");
  assert!(error.contains("intro/title"), "expected path in error, got {error}");
}

#[test]
fn context_docs_read_prefix_wildcard_allows_nested_path() {
  let root = setup_modules("@manual := docs://manual{:read(intro/*)}\n\nresult := intro/title@manual\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/*")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "context docs prefix wildcard");
}

#[test]
fn context_docs_read_prefix_wildcard_does_not_match_sibling() {
  let root = setup_modules("@manual := docs://manual{:read(introduction/*)}\n\nresult := intro/title@manual\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceCapabilityDenied"), "expected capability error, got {error}");
}

#[test]
fn program_scope_does_not_see_interpreter_context() {
  let root = setup_modules("~~~mech:foo\n@manual := docs://manual{:read(intro/title)}\nunused := true\n~~~\n\nresult := intro/title@manual\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected unknown address target, got {error}");
  assert!(error.contains("manual"), "expected target in error, got {error}");
  assert!(!error.contains("ContextAddressReadUnsupported"), "program scope should not resolve interpreter context, got {error}");
}

#[test]
fn interpreter_scope_context_docs_read_returns_value() {
  let root = setup_modules("~~~mech:foo\n@manual := docs://manual{:read(intro/title)}\nresult := intro/title@manual\n~~~\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let foo = foo_scope(&runtime, version);
  let result = runtime.run_module_scope(version, foo).unwrap();
  assert_bool_true(result, "interpreter scope context docs read");
}

#[test]
fn interpreter_scope_does_not_resolve_interpreter_target() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n~~~mech:bar\nresult := ok@foo\n~~~\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let bar = interpreter_scope_named(&runtime, version, "bar");
  let result = runtime.run_module_scope(version, bar);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected unknown address target, got {error}");
  assert!(error.contains("foo"), "expected interpreter target in error, got {error}");
}

#[test]
fn interpreter_address_still_works_from_program() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := ok@foo\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "interpreter address from program");
}

#[test]
fn duplicate_context_registry_binding_rejected() {
  let first = SourceContextDeclaration {
    name: "manual".to_string(),
    base: SourceContextBase::ResourceUri("docs://manual".to_string()),
    capabilities: vec![SourceContextCapability {
      operation: "read".to_string(),
      scope: SourceContextCapabilityScope::Path("intro/title".to_string()),
    }],
  };
  let second = SourceContextDeclaration {
    name: "manual".to_string(),
    base: SourceContextBase::ResourceUri("docs://other".to_string()),
    capabilities: vec![SourceContextCapability {
      operation: "read".to_string(),
      scope: SourceContextCapabilityScope::Wildcard,
    }],
  };

  let result = RuntimeContextRegistry::from_declarations(SourceScope::Program, &[first, second]);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeContextDuplicateBinding"), "expected duplicate binding error, got {error}");
  assert!(error.contains("manual"), "expected duplicate context name in error, got {error}");
}

#[test]
fn derived_context_base_is_unsupported() {
  let declaration = SourceContextDeclaration {
    name: "manual".to_string(),
    base: SourceContextBase::Context("base".to_string()),
    capabilities: vec![SourceContextCapability {
      operation: "read".to_string(),
      scope: SourceContextCapabilityScope::Wildcard,
    }],
  };

  let result = RuntimeContextRegistry::from_declarations(SourceScope::Program, &[declaration]);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeContextDerivedBaseUnsupported"), "expected derived base error, got {error}");
  assert!(error.contains("base"), "expected base context name in error, got {error}");
}

#[test]
fn interpreter_scope_imports_work() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-interpreter-imports-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "~~~mech:foo\n+> ./math.mec\nok := math/tau > 6.0\n<+ ok\n~~~\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let scope = foo_scope(&runtime, version);
  let result = runtime.run_module_scope(version, scope).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module scope run, got {:?}", other),
  }
}

#[test]
fn program_cannot_see_fenced_import_but_interpreter_can() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-interpreter-import-privacy-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "~~~mech:foo\n+> ./math.mec\nok := math/tau > 6.0\n<+ ok\n~~~\n\ntop := math/tau > 6.0\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let scope = foo_scope(&runtime, version);

  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("math/tau"));

  let result = runtime.run_module_scope(version, scope).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module scope run, got {:?}", other),
  }
}

#[test]
fn interpreter_scope_exports_only_interpreter_exports() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-interpreter-exports-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("main.mec"), "~~~mech:foo\nfoo-value := true\n<+ foo-value\n~~~\n\nprogram-value := false\n<+ program-value\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let scope = foo_scope(&runtime, version);
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  let foo = main.scopes.iter().find(|metadata| metadata.scope == scope).unwrap();
  assert!(foo.exports.iter().any(|export| export.name == "foo-value"));
  assert!(!foo.exports.iter().any(|export| export.name == "program-value"));

  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(!*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }

  let result = runtime.run_module_scope(version, scope).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module scope run, got {:?}", other),
  }
}

#[test]
fn interpreter_scope_executes_only_matching_import_edges() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-interpreter-edge-filter-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("good.mec"), "value := true\n<+ value\n").unwrap();
  std::fs::write(root.join("bad.mec"), "missing := bad/value\n").unwrap();
  std::fs::write(root.join("main.mec"), "~~~mech:foo\n+> ./good.mec\nok := good/value\n<+ ok\n~~~\n\n~~~mech:bar\n+> ./bad.mec\n~~~\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let scope = foo_scope(&runtime, version);
  let result = runtime.run_module_scope(version, scope).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module scope run, got {:?}", other),
  }
}

#[test]
fn missing_interpreter_scope_returns_error() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-missing-interpreter-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("main.mec"), "~~~mech:foo\nfoo-value := true\n<+ foo-value\n~~~\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let missing = SourceScope::Interpreter(SourceInterpreterId {
    namespace: hash_str("missing"),
    namespace_str: "missing".to_string(),
  });

  let result = runtime.run_module_scope(version, missing);
  assert!(result.is_err());
}


#[test]
fn program_reads_interpreter_export_by_indexed_address() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := ok@foo\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from addressed interpreter export, got {:?}", other),
  }
}

#[test]
fn program_reads_interpreter_export_by_address_with_interpreter_import() {
  let root = setup_modules("~~~mech:foo\n+> ./math.mec\nok := math/tau > 6.0\n<+ ok\n~~~\n\nresult := ok@foo\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from addressed interpreter export with import, got {:?}", other),
  }
}

#[test]
fn program_address_does_not_execute_unreferenced_interpreter_from_string() {
  let root = setup_modules("~~~mech:bar\nbroken := missing\n<+ broken\n~~~\n\ntext := \"@bar\"\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_ok(), "expected string @bar not to execute interpreter, got {result:?}");
}


#[test]
fn program_address_does_not_execute_unreferenced_interpreter_from_comment() {
  let root = setup_modules("~~~mech:bar\nbroken := missing\n<+ broken\n~~~\n\n-- @bar\n\nok := true\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_ok(), "expected comment @bar not to execute interpreter, got {result:?}");
}

#[test]
fn program_reads_only_requested_interpreter_export() {
  let root = setup_modules("~~~mech:foo\nok := true\nother := false\n<+ ok\n<+ other\n~~~\n\nresult := ok@foo\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from requested addressed export, got {:?}", other),
  }
}

#[test]
fn program_rejects_non_exported_interpreter_address() {
  let root = setup_modules("~~~mech:foo\nhidden := true\n~~~\n\nresult := hidden@foo\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeModuleExportNotFound") || error.contains("does not export"), "expected missing export error, got {error}");
  assert!(error.contains("hidden"), "expected addressed symbol in error, got {error}");
}

#[test]
fn program_unknown_interpreter_address_returns_error() {
  let root = setup_modules("result := ok@missing\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected unknown address target error, got {error}");
  assert!(error.contains("missing"), "expected missing target in error, got {error}");
}

#[test]
fn addressed_assignment_is_unsupported() {
  let root = setup_modules("x@foo := 1\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressedAssignmentUnsupported") || error.contains("addressed assignment"), "expected addressed assignment error, got {error}");
}

#[test]
fn addressed_assignment_to_context_is_still_unsupported() {
  let root = setup_modules("@manual := docs://manual{:write(intro/title)}\n\nintro/title@manual := true\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressedAssignmentUnsupported"), "expected addressed assignment error, got {error}");
}

#[test]
fn addressed_assignment_with_docs_provider_is_still_unsupported() {
  let root = setup_modules("@manual := docs://manual{:write(intro/title)}\n\nintro/title@manual := true\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).in_memory_docs(InMemoryDocsProvider::new()).build().unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressedAssignmentUnsupported"), "expected addressed assignment error, got {error}");
}

#[test]
fn resolver_metadata() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let resolver = FileSourceResolver::new(&root);
  let main = resolver.resolve(&SourceRequest::new("main.mec")).unwrap().unwrap();
  assert_eq!(main.imports.len(), 1);
  assert_eq!(main.contexts.len(), 0);
  assert_eq!(main.dependencies.len(), 1);
  assert!(main.dependencies[0].specifier == "./math.mec" || main.dependencies[0].specifier == "./math.mec".trim());
  assert!(main.dependencies[0].referrer.is_some());

  let dep = resolver.resolve(&main.dependencies[0]).unwrap().unwrap();
  assert!(dep.exports.iter().any(|e| e.name == "tau"));
}

#[test]
fn module_records_store_scoped_source_declarations_without_execution_changes() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-scopes-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("foo.mec"), "foo-result := 1\n<+ foo-result\n").unwrap();
  std::fs::write(root.join("bar.mec"), "bar-result := 2\n<+ bar-result\n").unwrap();
  std::fs::write(
    root.join("main.mec"),
    "@program-db := db://program{:read(*)}\n+> ./math.mec\nok := math/tau > 6.0\n<+ ok\n\n~~~mech:foo\n@foo-db := db://foo{:read(*)}\n+> ./foo.mec\n<+ foo-result\n~~~\n\n~~~mech:bar\n@bar-db := db://bar{:read(*)}\n+> ./bar.mec\n<+ bar-result\n~~~\n",
  ).unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();

  assert_eq!(main.imports.len(), 3);
  assert_eq!(main.exports.len(), 3);
  assert_eq!(main.contexts.len(), 3);
  assert_eq!(main.import_edges.len(), 3);
  assert!(main.import_edges.iter().any(|edge| edge.scope == SourceScope::Program && edge.import.specifier == "./math.mec"));
  assert!(main.import_edges.iter().any(|edge| match &edge.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo" && edge.import.specifier == "./foo.mec",
    SourceScope::Program => false,
  }));
  assert!(main.import_edges.iter().any(|edge| match &edge.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "bar" && edge.import.specifier == "./bar.mec",
    SourceScope::Program => false,
  }));

  let program = main.scopes.iter().find(|scope| scope.scope == SourceScope::Program).unwrap();
  assert_eq!(program.imports.len(), 1);
  assert_eq!(program.imports[0].specifier, "./math.mec");
  assert_eq!(program.exports.len(), 1);
  assert_eq!(program.exports[0].name, "ok");
  assert_eq!(program.contexts.len(), 1);
  assert_eq!(program.contexts[0].name, "program-db");

  let foo = main.scopes.iter().find(|scope| match &scope.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo",
    SourceScope::Program => false,
  }).unwrap();
  assert_eq!(foo.imports.len(), 1);
  assert_eq!(foo.imports[0].specifier, "./foo.mec");
  assert_eq!(foo.exports.len(), 1);
  assert_eq!(foo.exports[0].name, "foo-result");
  assert_eq!(foo.contexts.len(), 1);
  assert_eq!(foo.contexts[0].name, "foo-db");

  let bar = main.scopes.iter().find(|scope| match &scope.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "bar",
    SourceScope::Program => false,
  }).unwrap();
  assert_eq!(bar.imports.len(), 1);
  assert_eq!(bar.imports[0].specifier, "./bar.mec");
  assert_eq!(bar.exports.len(), 1);
  assert_eq!(bar.exports[0].name, "bar-result");
  assert_eq!(bar.contexts.len(), 1);
  assert_eq!(bar.contexts[0].name, "bar-db");

  assert!(!foo.imports.iter().any(|import| import.specifier == "./bar.mec"));
  assert!(!foo.exports.iter().any(|export| export.name == "bar-result"));
  assert!(!foo.contexts.iter().any(|context| context.name == "bar-db"));
}

#[test]
fn program_scope_imports_still_work() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  let program = main.scopes.iter().find(|scope| scope.scope == SourceScope::Program).unwrap();
  assert_eq!(program.imports.len(), 1);
  assert_eq!(program.imports[0].specifier, "./math.mec");

  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn fenced_import_does_not_leak_to_program_scope() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-scoped-fenced-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "~~~mech:foo\n+> ./math.mec\n~~~\nok := math/tau > 6.0\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main.imports.len(), 1);
  let program = main.scopes.iter().find(|scope| scope.scope == SourceScope::Program);
  assert!(program.map(|scope| scope.imports.is_empty()).unwrap_or(true));
  assert!(main.scopes.iter().any(|scope| match &scope.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo" && scope.imports.len() == 1 && scope.imports[0].specifier == "./math.mec",
    SourceScope::Program => false,
  }));

  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("math/tau"));
}

#[test]
fn program_and_fenced_imports_do_not_mix() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-scope-mix-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("foo.mec"), "foo-value := 42\n<+ foo-value\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./math.mec\n\n~~~mech:foo\n+> ./foo.mec\n~~~\n\nok := math/tau > 6.0\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  let program = main.scopes.iter().find(|scope| scope.scope == SourceScope::Program).unwrap();
  assert_eq!(program.imports.len(), 1);
  assert_eq!(program.imports[0].specifier, "./math.mec");
  let foo = main.scopes.iter().find(|scope| match &scope.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo",
    SourceScope::Program => false,
  }).unwrap();
  assert_eq!(foo.imports.len(), 1);
  assert_eq!(foo.imports[0].specifier, "./foo.mec");

  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn program_and_fenced_can_import_same_module_without_duplicate_program_binding() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-same-import-scope-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./math.mec\n\n~~~mech:foo\n+> ./math.mec\n~~~\n\nok := math/tau > 6.0\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main.imports.len(), 2);
  assert_eq!(main.import_edges.len(), 2);

  let program_edges = main.import_edges.iter().filter(|edge| edge.scope == SourceScope::Program).collect::<Vec<_>>();
  assert_eq!(program_edges.len(), 1);
  assert_eq!(program_edges[0].import.specifier, "./math.mec");

  let foo_edges = main.import_edges.iter().filter(|edge| match &edge.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo",
    SourceScope::Program => false,
  }).collect::<Vec<_>>();
  assert_eq!(foo_edges.len(), 1);
  assert_eq!(foo_edges[0].import.specifier, "./math.mec");

  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn fenced_import_binding_is_not_visible_to_program_scope() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-scope-negative-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("foo.mec"), "foo-value := 42\n<+ foo-value\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./math.mec\n\n~~~mech:foo\n+> ./foo.mec\n~~~\n\nok := foo/foo-value > 0\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("foo/foo-value"));
}

#[test]
fn module_records_keep_flat_metadata_for_compatibility() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-flat-compat-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("foo.mec"), "foo-value := 42\n<+ foo-value\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./math.mec\n\n~~~mech:foo\n+> ./foo.mec\n~~~\n\nok := math/tau > 6.0\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main.imports.len(), 2);
  assert_eq!(main.import_edges.len(), 2);
  assert!(main.import_edges.iter().any(|edge| edge.scope == SourceScope::Program && edge.import.specifier == "./math.mec"));
  assert!(main.import_edges.iter().any(|edge| match &edge.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo" && edge.import.specifier == "./foo.mec",
    SourceScope::Program => false,
  }));
  assert!(main.imports.iter().any(|import| import.specifier == "./math.mec"));
  assert!(main.imports.iter().any(|import| import.specifier == "./foo.mec"));

  let program = main.scopes.iter().find(|scope| scope.scope == SourceScope::Program).unwrap();
  assert_eq!(program.imports.len(), 1);
  assert_eq!(program.imports[0].specifier, "./math.mec");
  let foo = main.scopes.iter().find(|scope| match &scope.scope {
    SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo",
    SourceScope::Program => false,
  }).unwrap();
  assert_eq!(foo.imports.len(), 1);
  assert_eq!(foo.imports[0].specifier, "./foo.mec");

  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn build_dependency_graph() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main_version = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main_version.dependencies.len(), 1);
  assert_eq!(main_version.imports.len(), 1);
  assert_eq!(main_version.import_edges.len(), 1);
  let dep_version = runtime.store().get_module_version(main_version.dependencies[0]).unwrap().unwrap();
  assert!(dep_version.exports.iter().any(|e| e.name == "tau"));
}

#[test]
fn run_module() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_ok());
  match result.unwrap() {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn file_import_exposes_exports_under_file_stem_namespace() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn file_import_does_not_imply_wildcard_import() {
  let root = setup_modules("+> ./math.mec\nok := tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("tau"));
}

#[test]
fn file_import_does_not_expose_non_exported_binding() {
  let root = setup_modules("+> ./math.mec\nok := math/secret > 0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("math/secret"));
}

#[test]
fn namespace_import_exposes_exports_under_module_namespace() {
  let root = setup_modules("+> math\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn single_import_exposes_export_unqualified() {
  let root = setup_modules("+> math/tau\nok := tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn wildcard_import_exposes_all_exports_unqualified() {
  let root = setup_modules("+> math/*\nok := tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::Bool(value) => assert!(*value.borrow()),
    other => panic!("expected bool result from module run, got {:?}", other),
  }
}

#[test]
fn wildcard_import_does_not_expose_non_exported_binding() {
  let root = setup_modules("+> math/*\nok := secret > 0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("secret"));
}

#[test]
fn import_conflict_fails() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-conflict-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("science.mec"), "tau := 6.2\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> math/tau\n+> science/tau\nok := tau > 0\n").unwrap();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeModuleImportConflict"));
  assert!(error.contains("tau"));
}

#[test]
fn re_export_works() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-reexport-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("bridge.mec"), "+> math/tau\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> bridge/tau\nok := tau > 6.0\n").unwrap();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result { Value::Bool(v) => assert!(*v.borrow()), other => panic!("expected bool got {:?}", other) }
}

#[test]
fn module_version_records_import_edges() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main.imports.len(), 1);
  assert_eq!(main.contexts.len(), 0);
  assert_eq!(main.dependencies.len(), 1);
  assert_eq!(main.import_edges.len(), 1);
  assert_eq!(main.import_edges[0].scope, SourceScope::Program);
  assert_eq!(main.import_edges[0].import.specifier, "./math.mec");
  assert_eq!(main.import_edges[0].dependency, main.dependencies[0]);
}

#[test]
fn module_version_records_multiple_import_edges_in_order() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-edges-order-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("a.mec"), "x := 1\n<+ x\n").unwrap();
  std::fs::write(root.join("b.mec"), "y := 2\n<+ y\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./a.mec\n+> ./b.mec\nok := a/x < b/y\n").unwrap();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main.import_edges.len(), 2);
  assert_eq!(main.import_edges[0].scope, SourceScope::Program);
  assert_eq!(main.import_edges[1].scope, SourceScope::Program);
  assert_eq!(main.import_edges[0].import.specifier, "./a.mec");
  assert_eq!(main.import_edges[1].import.specifier, "./b.mec");
  assert!(main.dependencies.contains(&main.import_edges[0].dependency));
  assert!(main.dependencies.contains(&main.import_edges[1].dependency));
  match runtime.run_module(version).unwrap() { Value::Bool(v) => assert!(*v.borrow()), other => panic!("expected bool got {:?}", other) }
}

#[test]
fn module_version_records_contexts() {
  let root = setup_modules("@main := db://main{:read(users/*), :write(users/name)}\n+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main.contexts.len(), 1);
  assert_eq!(main.contexts[0].name, "main");
  assert_eq!(main.contexts[0].capabilities.len(), 2);
}

#[test]
fn multi_level_re_export_works() {
  re_export_works();
}

#[test]
fn multi_level_private_symbol_does_not_leak() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-privacy-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\nsecret := 42\n<+ tau\n").unwrap();
  std::fs::write(root.join("bridge.mec"), "+> math/tau\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> bridge/*\nok := secret > 0\n").unwrap();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UndefinedVariable"));
  assert!(error.contains("secret"));
}

#[test]
fn duplicate_unqualified_import_conflict_fails() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-dupe-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("a.mec"), "x := 1\n<+ x\n").unwrap();
  std::fs::write(root.join("b.mec"), "x := 2\n<+ x\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> a/*\n+> b/*\nok := x > 0\n").unwrap();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeModuleImportConflict"));
  assert!(error.contains("x"));
}

#[test]
fn duplicate_same_export_import_policy_is_explicit() {
  let root = setup_modules("+> math/tau\n+> math/*\nok := tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeModuleImportConflict"));
  assert!(error.contains("tau"));
}

#[test]
fn module_host_call_works_inside_isolated_execution() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-host-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := demo/value()\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./math.mec\nok := math/tau > 40\n").unwrap();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  runtime.register_mech_host_function(ClosureHostFunction::new("demo/value", |_s, _c, _a| Ok(Value::F64(Ref::new(42.0))))).unwrap();
  runtime.grant_capability(std::sync::Arc::new(BasicCapability::new(
    CapabilityId(1),
    &BasicSubject::new("program:module-host-test"),
    &BasicResource::new("host:demo/value"),
    [BasicOperation::new("call")],
  ))).unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let mut context = runtime.runtime_context().unwrap().with_subject("program:module-host-test");
  let result = runtime.run_module_with_context(&mut context, version).unwrap();
  match result { Value::Bool(v) => assert!(*v.borrow()), other => panic!("expected bool got {:?}", other) }
}

#[test]
fn repeated_run_module_is_not_stale() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let first = runtime.run_module(version).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 5.0\n<+ tau\n").unwrap();
  let second = runtime.run_module(version).unwrap();
  match (first, second) {
    (Value::Bool(a), Value::Bool(b)) => {
      assert!(*a.borrow());
      assert!(*b.borrow());
    }
    other => panic!("expected bool tuple got {:?}", other),
  }
}

#[test]
fn missing_dependency_fails_module_build() {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-missing-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("main.mec"), "+> ./missing.mec\nok := 1\n").unwrap();

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let result = runtime.resolve_and_store_module_source("main.mec", options);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeModuleDependencyMissing"));
}


fn docs_config(path: &str, value: Value) -> RuntimeConfigSpec {
  RuntimeConfigSpec::new().with_resource(RuntimeResourceConfigSpec::InMemoryDocs(
    RuntimeInMemoryDocsResourceSpec::new("docs://manual")
      .with_entry(path, value),
  ))
}

fn run_docs_config_read(spec: RuntimeConfigSpec) -> Result<Value, mech_core::MechError> {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  runtime.run_module(version)
}

#[test]
fn config_spec_docs_read_requires_host_grant() {
  let result = run_docs_config_read(docs_config("intro/title", bool_value(true)));
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"));
  assert!(error.contains("task://main"));
  assert!(error.contains("docs://manual"));
  assert!(error.contains("intro/title"));
}

#[test]
fn config_spec_docs_read_with_matching_grant_returns_value() {
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "intro/title"));
  assert_bool_true(run_docs_config_read(spec).unwrap(), "matching grant");
}

#[test]
fn host_grant_path_prefix_allows_nested_read() {
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "intro/*"));
  assert_bool_true(run_docs_config_read(spec).unwrap(), "prefix grant");
}

#[test]
fn host_grant_path_prefix_does_not_match_sibling() {
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "introduction/*"));
  let error = format!("{:?}", run_docs_config_read(spec).err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"));
}

#[test]
fn host_grant_wrong_operation_denies_read() {
  let grant = RuntimeCapabilityGrantSpec::new("task://main", "docs://manual")
    .with_operation(RuntimeCapabilityOperation::Write)
    .with_path("intro/title");
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(grant);
  let error = format!("{:?}", run_docs_config_read(spec).err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"));
}

#[test]
fn host_grant_wrong_resource_denies_read() {
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://other", "intro/title"));
  let error = format!("{:?}", run_docs_config_read(spec).err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"));
}

#[test]
fn context_denial_still_uses_context_capability_error() {
  let root = setup_modules("@manual := docs://manual{:read(other/path)}\n\nresult := intro/title@manual\n");
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "intro/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let error = format!("{:?}", runtime.run_module(version).err().unwrap());
  assert!(error.contains("RuntimeResourceCapabilityDenied"));
  assert!(!error.contains("RuntimeCapabilityGrantDenied"));
}

#[test]
fn host_grant_wildcard_path_allows_read() {
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "*"));
  assert_bool_true(run_docs_config_read(spec).unwrap(), "wildcard grant");
}

#[test]
fn direct_provider_read_with_runtime_grant_still_works() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_read_grant("docs://manual", "intro/title")).unwrap();
  assert!(runtime.has_capability_grant("task://main", "docs://manual", &RuntimeCapabilityOperation::Read, "intro/title"));
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "runtime grant");
}

#[test]
fn apply_config_spec_after_build_registers_resources_and_grants() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n");
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "intro/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .build()
    .unwrap();
  runtime.apply_config_spec(spec).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "apply spec");
}

#[test]
fn invalid_grant_empty_subject_fails_build() {
  let spec = RuntimeConfigSpec::new().with_capability_grant(
    RuntimeCapabilityGrantSpec::new("", "docs://manual")
      .with_operation(RuntimeCapabilityOperation::Read)
      .with_path("intro/title"),
  );
  let error = format!("{:?}", RuntimeBuilder::new().config_spec(spec).build().err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantInvalid"));
  assert!(error.contains("subject"));
}

#[test]
fn invalid_grant_empty_operation_fails_build() {
  let spec = RuntimeConfigSpec::new().with_capability_grant(
    RuntimeCapabilityGrantSpec::new("task://main", "docs://manual")
      .with_operation(RuntimeCapabilityOperation::Custom(String::new()))
      .with_path("intro/title"),
  );
  let error = format!("{:?}", RuntimeBuilder::new().config_spec(spec).build().err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantInvalid"));
  assert!(error.contains("operation"));
}

#[test]
fn invalid_grant_empty_path_fails_build() {
  let spec = RuntimeConfigSpec::new().with_capability_grant(
    RuntimeCapabilityGrantSpec::new("task://main", "docs://manual")
      .with_operation(RuntimeCapabilityOperation::Read)
      .with_path(""),
  );
  let error = format!("{:?}", RuntimeBuilder::new().config_spec(spec).build().err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantInvalid"));
  assert!(error.contains("path"));
}


#[test]
fn workspace_open_rejects_duplicate_targets() {
  let root = setup_modules("result := true\n");
  let config = RuntimeWorkspaceConfig::new(&root)
    .target("main", "main.mec")
    .target("main", "other.mec");

  let error = format!("{:?}", RuntimeWorkspace::open(config).err().unwrap());
  assert!(error.contains("RuntimeWorkspaceInvalidConfig"));
  assert!(error.contains("duplicate target"));
}

#[test]
fn workspace_open_rejects_empty_target_name() {
  let root = setup_modules("result := true\n");
  let config = RuntimeWorkspaceConfig::new(&root)
    .target("", "main.mec");

  let error = format!("{:?}", RuntimeWorkspace::open(config).err().unwrap());
  assert!(error.contains("RuntimeWorkspaceInvalidConfig"));
  assert!(error.contains("target name"));
}

#[test]
fn workspace_load_target_without_imports() {
  let root = setup_modules("result := true\n");
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert!(snapshot.diagnostics.is_empty());
  let target = snapshot.targets.get("main").unwrap();
  assert!(snapshot.sources.contains_key(&target.canonical_uri));
  assert!(workspace.target("main").is_some());
  assert_bool_true(runtime.run_module(target.module_version).unwrap(), "workspace target");
}

#[test]
fn workspace_load_target_with_local_import() {
  let root = setup_modules("+> ./math.mec\n\nresult := math/value\n");
  std::fs::write(root.join("math.mec"), "value := true\n<+ value\n").unwrap();
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert!(snapshot.diagnostics.is_empty());
  let target = snapshot.targets.get("main").unwrap();
  let main_path = root.join("main.mec").canonicalize().unwrap();
  let math_path = root.join("math.mec").canonicalize().unwrap();

  assert!(
    snapshot.sources.values().any(|source| {
      source.path.as_ref() == Some(&main_path)
    }),
    "workspace snapshot should contain main.mec source; sources: {:?}",
    snapshot.sources,
  );

  assert!(
    snapshot.sources.values().any(|source| {
      source.path.as_ref() == Some(&math_path)
    }),
    "workspace snapshot should contain math.mec source; sources: {:?}",
    snapshot.sources,
  );
}

#[test]
fn workspace_load_relative_target_uses_workspace_root() {
  let root_a = std::env::temp_dir().join(format!(
    "mech-runtime-workspace-root-a-{}",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  let root_b = std::env::temp_dir().join(format!(
    "mech-runtime-workspace-root-b-{}",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  std::fs::create_dir_all(&root_a).unwrap();
  std::fs::create_dir_all(&root_b).unwrap();

  std::fs::write(root_a.join("main.mec"), "result := false\n").unwrap();
  std::fs::write(root_b.join("main.mec"), "result := true\n").unwrap();

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root_b))
    .build()
    .unwrap();
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root_a)
      .target("main", "main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert!(snapshot.diagnostics.is_empty());

  let target = snapshot.targets.get("main").unwrap();
  assert_eq!(target.specifier, "main.mec");
  let result = runtime.run_module(target.module_version).unwrap();
  match result {
    Value::Bool(value) => assert!(!*value.borrow()),
    other => panic!("expected false bool result from workspace root target, got {:?}", other),
  }
}

#[test]
fn workspace_load_target_outside_root_records_diagnostic() {
  let parent = std::env::temp_dir().join(format!(
    "mech-runtime-workspace-outside-root-{}",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  let root = parent.join("workspace");
  let outside = parent.join("outside");
  std::fs::create_dir_all(&root).unwrap();
  std::fs::create_dir_all(&outside).unwrap();
  std::fs::write(outside.join("main.mec"), "result := true\n").unwrap();

  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root)
      .target("outside", "../outside/main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert_eq!(snapshot.diagnostics.len(), 1);
  assert_eq!(snapshot.diagnostics[0].severity, RuntimeWorkspaceDiagnosticSeverity::Error);
  assert_eq!(snapshot.diagnostics[0].target.as_deref(), Some("outside"));
  assert!(snapshot.diagnostics[0].message.contains("outside workspace root"));
  assert!(!snapshot.targets.contains_key("outside"));
}

#[test]
fn workspace_load_missing_target_records_diagnostic() {
  let root = setup_modules("result := true\n");
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("missing", "missing.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert_eq!(snapshot.diagnostics.len(), 1);
  assert_eq!(snapshot.diagnostics[0].severity, RuntimeWorkspaceDiagnosticSeverity::Error);
  assert_eq!(snapshot.diagnostics[0].target.as_deref(), Some("missing"));
  assert!(snapshot.diagnostics[0].message.contains("missing.mec"));
  assert!(!snapshot.targets.contains_key("missing"));
}

#[test]
fn workspace_load_multiple_targets_continues_after_failure() {
  let root = setup_modules("result := true\n");
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root)
      .target("missing", "missing.mec")
      .target("main", "main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert_eq!(snapshot.diagnostics.len(), 1);
  assert_eq!(snapshot.diagnostics[0].target.as_deref(), Some("missing"));
  let target = snapshot.targets.get("main").unwrap();
  assert_bool_true(runtime.run_module(target.module_version).unwrap(), "workspace surviving target");
}


#[test]
fn workspace_refresh_before_load_fails() {
  let root = setup_modules("result := true\n");
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  let error = format!("{:?}", workspace.refresh(&mut runtime, module_options()).err().unwrap());
  assert!(error.contains("RuntimeWorkspaceNotLoaded"));
}

#[test]
fn workspace_refresh_without_changes_is_empty() {
  let root = setup_modules("result := true\n");
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  workspace.load(&mut runtime, module_options()).unwrap();
  let refresh = workspace.refresh(&mut runtime, module_options()).unwrap();
  assert!(refresh.changes.is_empty());
  assert!(refresh.affected_targets.is_empty());
  assert!(refresh.refresh_diagnostics.is_empty());
  assert!(refresh.snapshot.targets.contains_key("main"));
}

#[test]
fn workspace_refresh_modified_dependency_reloads_target() {
  let root = setup_modules("+> ./math.mec\n\nresult := math/value\n");
  std::fs::write(root.join("math.mec"), "value := false\n<+ value\n").unwrap();
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert_bool_false(runtime.run_module(snapshot.targets["main"].module_version).unwrap(), "initial workspace dependency");
  std::fs::write(root.join("math.mec"), "value := true\n<+ value\n").unwrap();

  let refresh = workspace.refresh(&mut runtime, module_options()).unwrap();
  assert_eq!(refresh.changes.len(), 1);
  assert_eq!(refresh.changes[0].kind, RuntimeWorkspaceChangeKind::Modified);
  assert!(refresh.changes[0].canonical_uri.ends_with("/math.mec"));
  assert_eq!(refresh.affected_targets, vec!["main"]);
  assert!(refresh.refresh_diagnostics.is_empty());
  assert_bool_true(runtime.run_module(refresh.snapshot.targets["main"].module_version).unwrap(), "refreshed workspace dependency");
}

#[test]
fn workspace_refresh_removed_dependency_records_diagnostic() {
  let root = setup_modules("+> ./math.mec\n\nresult := math/value\n");
  std::fs::write(root.join("math.mec"), "value := false\n<+ value\n").unwrap();
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  workspace.load(&mut runtime, module_options()).unwrap();
  std::fs::remove_file(root.join("math.mec")).unwrap();

  let refresh = workspace.refresh(&mut runtime, module_options()).unwrap();
  assert_eq!(refresh.changes.len(), 1);
  assert_eq!(refresh.changes[0].kind, RuntimeWorkspaceChangeKind::Removed);
  assert!(refresh.changes[0].canonical_uri.ends_with("/math.mec"));
  assert_eq!(refresh.affected_targets, vec!["main"]);
  assert_eq!(refresh.refresh_diagnostics.len(), 1);
  assert_eq!(refresh.refresh_diagnostics[0].severity, RuntimeWorkspaceDiagnosticSeverity::Error);
  assert_eq!(refresh.refresh_diagnostics[0].target.as_deref(), Some("main"));
  assert!(!refresh.snapshot.targets.contains_key("main"));
}

#[test]
fn workspace_refresh_modified_target_reloads_target() {
  let root = setup_modules("result := false\n");
  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();
  assert_bool_false(runtime.run_module(snapshot.targets["main"].module_version).unwrap(), "initial workspace target");
  std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

  let refresh = workspace.refresh(&mut runtime, module_options()).unwrap();
  assert_eq!(refresh.changes.len(), 1);
  assert_eq!(refresh.changes[0].kind, RuntimeWorkspaceChangeKind::Modified);
  assert!(refresh.changes[0].canonical_uri.ends_with("/main.mec"));
  assert_eq!(refresh.affected_targets, vec!["main"]);
  assert!(refresh.refresh_diagnostics.is_empty());
  assert_bool_true(runtime.run_module(refresh.snapshot.targets["main"].module_version).unwrap(), "refreshed workspace target");
}
