use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mech_core::{hash_str, MechSourceCode, ModuleManifestConfig, ModuleManifestExportConfig, ModuleManifestExportKind, Ref, Value};
use mech_program::{MechProgram, MechProgramConfig};
use mech_host_cli::{CliBackend, CliResourceProvider};
use mech_runtime::*;

fn setup_modules(main_source: &str) -> std::path::PathBuf {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-smoke-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\nsecret := 42\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), main_source).unwrap();
  root
}



fn browser_dom_manifest() -> ModuleManifestConfig {
  ModuleManifestConfig {
    name: "browser".to_string(),
    exports: vec![ModuleManifestExportConfig {
      name: "dom".to_string(),
      kind: ModuleManifestExportKind::Context,
      base_uri: "browser://dom".to_string(),
      operations: vec!["read".to_string(), "write".to_string()],
    }],
  }
}

fn runtime_with_browser_manifest() -> mech_runtime::MechRuntime {
  RuntimeBuilder::new()
    .module_manifest(browser_dom_manifest())
    .unwrap()
    .build()
    .unwrap()
}

fn runtime_with_root(root: &std::path::Path) -> mech_runtime::MechRuntime {
  RuntimeBuilder::new().source_resolver(FileSourceResolver::new(root)).build().unwrap()
}

fn runtime_with_root_and_browser_manifest(root: &std::path::Path) -> mech_runtime::MechRuntime {
  RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(root))
    .module_manifest(browser_dom_manifest())
    .unwrap()
    .build()
    .unwrap()
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

fn runtime_write_grant_for(subject: &str, resource: &str, path: &str) -> RuntimeCapabilityGrant {
  RuntimeCapabilityGrant {
    subject: subject.to_string(),
    resource: resource.to_string(),
    operations: vec![RuntimeCapabilityOperation::Write],
    paths: vec![path.to_string()],
  }
}

fn runtime_read_grant_for(subject: &str, resource: &str, path: &str) -> RuntimeCapabilityGrant {
  RuntimeCapabilityGrant {
    subject: subject.to_string(),
    resource: resource.to_string(),
    operations: vec![RuntimeCapabilityOperation::Read],
    paths: vec![path.to_string()],
  }
}

fn runtime_context_read_grant(runtime: &MechRuntime, resource: &str, path: &str) -> RuntimeCapabilityGrant {
  let subject = runtime.runtime_context().unwrap().subject;
  runtime_read_grant_for(&subject, resource, path)
}

fn runtime_context_write_grant(runtime: &MechRuntime, resource: &str, path: &str) -> RuntimeCapabilityGrant {
  let subject = runtime.runtime_context().unwrap().subject;
  runtime_write_grant_for(&subject, resource, path)
}

#[derive(Clone, Debug, Default)]
struct RecordingCliState {
  env: HashMap<String, String>,
  stdout: Vec<String>,
  stderr: Vec<String>,
}

#[derive(Clone, Debug)]
struct RecordingCliBackend {
  state: Arc<Mutex<RecordingCliState>>,
}

impl CliBackend for RecordingCliBackend {
  fn env_var(&self, name: &str) -> mech_core::MResult<Option<String>> {
    Ok(self.state.lock().unwrap().env.get(name).cloned())
  }

  fn write_stdout(&mut self, text: &str) -> mech_core::MResult<()> {
    self.state.lock().unwrap().stdout.push(text.to_string());
    Ok(())
  }

  fn write_stderr(&mut self, text: &str) -> mech_core::MResult<()> {
    self.state.lock().unwrap().stderr.push(text.to_string());
    Ok(())
  }
}

#[derive(Debug)]
struct RecordingCliHostFactory {
  manifest: HostManifestConfig,
  state: Arc<Mutex<RecordingCliState>>,
}

impl RuntimeHostFactory for RecordingCliHostFactory {
  fn provider_name(&self) -> &str { "cli" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, _settings: &ConfigValue) -> mech_core::MResult<()> { Ok(()) }
  fn instantiate(&self, instance_name: &str, _settings: &ConfigValue) -> mech_core::MResult<RuntimeHostInstallation> {
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      resource_providers: vec![Box::new(CliResourceProvider::for_instance(instance_name, RecordingCliBackend { state: self.state.clone() }))],
    })
  }
}

fn runtime_with_recording_cli() -> (MechRuntime, Arc<Mutex<RecordingCliState>>) {
  let state = Arc::new(Mutex::new(RecordingCliState::default()));
  let runtime = RuntimeBuilder::new()
    .host_factory(Box::new(RecordingCliHostFactory { manifest: mech_host_cli::cli_host_manifest().unwrap(), state: state.clone() })).unwrap()
    .host_instance(HostInstanceConfig { name: "cli".to_string(), provider: "cli".to_string(), settings: ConfigValue::Map(Default::default()) })
    .build()
    .unwrap();
  (runtime, state)
}

fn grant_runtime_stdout_line(runtime: &mut MechRuntime) {
  let subject = runtime.runtime_context().unwrap().subject;
  for resource in ["cli://cli/stdout", "cli://stdout"] {
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject: subject.clone(),
      resource: resource.to_string(),
      operations: vec![RuntimeCapabilityOperation::Write],
      paths: vec!["line".to_string()],
    }).unwrap();
  }
}

fn run_module_as_main(runtime: &mut MechRuntime, version: ModuleVersionId) -> mech_core::MResult<Value> {
  let mut context = runtime.runtime_context()?.with_subject("task://main");
  runtime.run_module_with_context(&mut context, version)
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
fn direct_run_string_preserves_normal_imports() {
  let root = setup_modules("+> ./math.mec
ok := math/tau > 6.0
");
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .build()
    .unwrap();
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  let result = runtime.run_module(version).unwrap();

  assert_bool_true(result, "direct run_string normal import");
}

#[test]
fn in_memory_docs_provider_write_then_read_returns_value() {
  let mut provider = InMemoryDocsProvider::new();
  provider.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), operation: RuntimeCapabilityOperation::Write, value: bool_value(true), intent: RuntimeResourceWriteIntent::Assign }).unwrap();
  let value = provider.read(RuntimeResourceReadRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string() }).unwrap();
  assert_bool_true(value, "provider write then read");
}

#[test]
fn resource_registry_write_then_read_returns_value() {
  let mut registry = RuntimeResourceRegistry::new();
  registry.register_provider(Box::new(InMemoryDocsProvider::new())).unwrap();
  registry.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), operation: RuntimeCapabilityOperation::Write, value: bool_value(true), intent: RuntimeResourceWriteIntent::Assign }).unwrap();
  let value = registry.read(RuntimeResourceReadRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string() }).unwrap();
  assert_bool_true(value, "registry write then read");
}

#[test]
fn resource_registry_write_missing_provider_fails() {
  let mut registry = RuntimeResourceRegistry::new();
  let result = registry.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), operation: RuntimeCapabilityOperation::Write, value: bool_value(true), intent: RuntimeResourceWriteIntent::Assign });
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderNotFound"), "expected missing provider error, got {error}");
  assert!(error.contains("docs"), "expected scheme in error, got {error}");
  assert!(error.contains("docs://manual"), "expected base URI in error, got {error}");
}

#[test]
fn in_memory_docs_write_invalid_scheme_fails() {
  let mut provider = InMemoryDocsProvider::new();
  let result = provider.write(RuntimeResourceWriteRequest { base_uri: "db://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), operation: RuntimeCapabilityOperation::Write, value: bool_value(true), intent: RuntimeResourceWriteIntent::Assign });
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceInvalidUri"), "expected invalid URI error, got {error}");
  assert!(error.contains("docs"), "expected docs scheme requirement, got {error}");
}

#[test]
fn in_memory_docs_write_empty_path_fails() {
  let mut provider = InMemoryDocsProvider::new();
  let result = provider.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: String::new(), context_name: "manual".to_string(), operation: RuntimeCapabilityOperation::Write, value: bool_value(true), intent: RuntimeResourceWriteIntent::Assign });
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
  let result = provider.write(RuntimeResourceWriteRequest { base_uri: "docs://manual".to_string(), path: "intro/title".to_string(), context_name: "manual".to_string(), operation: RuntimeCapabilityOperation::Write, value: bool_value(true), intent: RuntimeResourceWriteIntent::Assign });
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
fn repeated_interpreter_fences_merge_before_resolution_and_execution() {
  let root = setup_modules("~~~mech:foo\nx := 1\n~~~\n\nprose stays out\n\n~~~mech:foo\ny := x + 1\n<+ y\n~~~\n\nresult := @foo/y\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  match result {
    Value::F64(value) => assert_eq!(*value.borrow(), 2.0),
    other => panic!("expected merged interpreter result, got {:?}", other),
  }
}

#[test]
fn faux_tilde_interpreter_fence_inside_markdown_backtick_block_is_ignored() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n```text\n~~~mech:foo\nbroken := missing\n~~~\n```\n\nresult := @foo/ok\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "interpreter ignores faux tilde fence");
}

#[test]
fn faux_backtick_interpreter_fence_inside_markdown_tilde_block_is_ignored() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n~~~text\n```mech:foo\nbroken := missing\n```\n~~~\n\nresult := @foo/ok\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "interpreter ignores faux backtick fence");
}

#[test]
fn duplicate_resolved_interpreter_address_target_still_fails_validation() {
  let foo = SourceInterpreterId {
    namespace: hash_str("foo"),
    namespace_str: "foo".to_string(),
  };
  let scope = ModuleScopeMetadata {
    scope: SourceScope::Interpreter(foo),
    imports: Vec::new(),
    exports: Vec::new(),
    contexts: Vec::new(),
    address_references: Vec::new(),
  };
  let resolved = ResolvedSource::new(
    "main.mec",
    "memory:main.mec",
    MechSourceCode::String("ok := true\n".to_string()),
  )
  .with_kind(SourceKind::Mech)
  .with_scopes(vec![scope.clone(), scope]);

  let result = resolved.validate();
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
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n@manual := docs://foo{:read(ok)}\n\nresult := @foo/ok\n");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "distinct interpreter/context names");
}

#[test]
fn context_docs_read_returns_value() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "context docs read");
}

#[test]
fn context_docs_read_uses_provider_base_for_full_requested_uri() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let root_provider = docs_provider_with("docs://manual", "intro/title", bool_value(false));
  let intro_provider = docs_provider_with("docs://manual/intro", "title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(root_provider)
    .in_memory_docs(intro_provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual/intro", "title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "context docs read from most specific provider");
}

#[test]
fn config_spec_registers_in_memory_docs_resource() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
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
  assert_bool_true(run_module_as_main(&mut runtime, version).unwrap(), "config spec docs read");
}

#[test]
fn config_spec_merges_multiple_docs_bases_into_one_provider() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n@guide := docs://guide{:read(start/title)}\n\nmanual-ok := @manual/intro/title\nguide-ok := @guide/start/title\n\nresult := manual-ok && guide-ok\n");
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
  assert_bool_true(run_module_as_main(&mut runtime, version).unwrap(), "merged config spec docs bases");
}

#[test]
fn config_spec_multiple_entries_same_base() {
  let root = setup_modules("@manual := docs://manual{:read(intro/*)}\n\na := @manual/intro/title\nb := @manual/intro/subtitle\n\nresult := a && b\n");
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
  assert_bool_true(run_module_as_main(&mut runtime, version).unwrap(), "config spec docs entries under one base");
}

#[test]
fn config_spec_later_duplicate_path_overwrites_earlier() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
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
  assert_bool_true(run_module_as_main(&mut runtime, version).unwrap(), "later duplicate config spec docs path");
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
fn config_spec_and_direct_docs_fallback_can_coexist() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  );
  RuntimeBuilder::new()
    .config_spec(spec)
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();
}

#[test]
fn direct_provider_registration_still_works() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "direct docs provider registration");
}

#[test]
fn apply_config_spec_after_build_registers_docs() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
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
  assert_bool_true(run_module_as_main(&mut runtime, version).unwrap(), "applied config spec docs read");
}

#[test]
fn apply_config_spec_after_docs_fallback_can_coexist() {
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
  runtime.apply_config_spec(spec).unwrap();
}

#[test]
fn config_spec_conflicts_with_existing_same_docs_base() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  );
  let result = RuntimeBuilder::new()
    .in_memory_docs(docs_provider_with("docs://manual", "other/title", bool_value(true)))
    .config_spec(spec)
    .build();
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderConflict"), "expected provider conflict, got {error}");
}

#[test]
fn apply_config_spec_conflicts_with_existing_same_docs_base() {
  let spec = RuntimeConfigSpec::new().with_resource(
    RuntimeResourceConfigSpec::InMemoryDocs(
      RuntimeInMemoryDocsResourceSpec::new("docs://manual")
        .with_entry("intro/title", bool_value(true)),
    ),
  );
  let mut runtime = RuntimeBuilder::new()
    .in_memory_docs(docs_provider_with("docs://manual", "other/title", bool_value(true)))
    .build()
    .unwrap();
  let result = runtime.apply_config_spec(spec);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderConflict"), "expected provider conflict, got {error}");
}

#[test]
fn two_fallback_docs_providers_conflict() {
  let result = RuntimeBuilder::new()
    .in_memory_docs(InMemoryDocsProvider::new())
    .in_memory_docs(InMemoryDocsProvider::new())
    .build();
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderConflict"), "expected provider conflict, got {error}");
}

#[test]
fn registered_module_manifest_context_import_still_resolves() {
  let root = setup_modules("+> @manual := docs/page\nresult := @manual/title\n<+ result\nresult\n");
  let manifest = ModuleManifestConfig {
    name: "docs".to_string(),
    exports: vec![ModuleManifestExportConfig {
      name: "page".to_string(),
      kind: ModuleManifestExportKind::Context,
      base_uri: "docs://manual".to_string(),
      operations: vec!["read".to_string()],
    }],
  };
  let provider = docs_provider_with("docs://manual", "title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .module_manifest(manifest)
    .unwrap()
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime
    .grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "title"))
    .unwrap();
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();
  let result = match runtime.run_module(version).unwrap() {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_true(result, "registered module manifest context import");
}

#[test]
fn host_instance_unknown_context_does_not_fallback_to_module_manifest() {
  let root = setup_modules("+> @bad := cli/missing\nresult := true\n");
  let manifest = ModuleManifestConfig {
    name: "cli".to_string(),
    exports: vec![ModuleManifestExportConfig {
      name: "missing".to_string(),
      kind: ModuleManifestExportKind::Context,
      base_uri: "docs://wrong".to_string(),
      operations: vec!["read".to_string()],
    }],
  };
  let backend = FakeCliBackend::default();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root))
      .module_manifest(manifest)
      .unwrap(),
    backend,
  )
  .build()
  .unwrap();
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("HostInterfaceUnknownContext"), "expected host unknown context, got {error}");
}

#[test]
fn unknown_address_target_is_explicit() {
  let root = setup_modules("result := @missing/ok\n");
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
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := @foo/ok\n");
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
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let mut runtime = runtime_with_root(&root);
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
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
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
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
  let root = setup_modules("@manual := docs://manual{:read(other/path)}\n\nresult := @manual/intro/title\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
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
  let root = setup_modules("@manual := docs://manual{:read(intro/*)}\n\nresult := @manual/intro/title\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/*")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "context docs prefix wildcard");
}

#[test]
fn context_docs_read_prefix_wildcard_does_not_match_sibling() {
  let root = setup_modules("@manual := docs://manual{:read(introduction/*)}\n\nresult := @manual/intro/title\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceCapabilityDenied"), "expected capability error, got {error}");
}

#[test]
fn program_scope_does_not_see_interpreter_context() {
  let root = setup_modules("~~~mech:foo\n@manual := docs://manual{:read(intro/title)}\nunused := true\n~~~\n\nresult := @manual/intro/title\n");
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
  let root = setup_modules("~~~mech:foo\n@manual := docs://manual{:read(intro/title)}\nresult := @manual/intro/title\n~~~\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let foo = foo_scope(&runtime, version);
  let result = runtime.run_module_scope(version, foo).unwrap();
  assert_bool_true(result, "interpreter scope context docs read");
}

#[test]
fn interpreter_scope_does_not_resolve_interpreter_target() {
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\n~~~mech:bar\nresult := @foo/ok\n~~~\n");
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
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := @foo/ok\n");
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
fn derived_context_unknown_base_is_rejected() {
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
  assert!(error.contains("RuntimeContextInvalidBinding"), "expected invalid binding error, got {error}");
  assert!(error.contains("base"), "expected base context name in error, got {error}");
}

#[test]
fn direct_runtime_derived_context_without_host_read_preflights() {
  let mut runtime = runtime_with_browser_manifest();
  runtime.run_string("@base := docs://manual{:read(*)}\n@docs := @base\nx := 1\n").unwrap();
}

#[test]
fn direct_runtime_derived_context_read_uses_alias() {
  let mut runtime = RuntimeBuilder::new()
    .resource_provider(Box::new(docs_provider_with("docs://manual", "intro/title", Value::String(Ref::new("Hello".to_string())))))
    .build()
    .unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  let result = runtime.run_string("@base := docs://manual{:read(intro/title)}\n@docs := @base\nresult := @docs/intro/title\n").unwrap();
  match result {
    Value::String(value) => assert_eq!(&*value.borrow(), "Hello"),
    other => panic!("expected docs title string, got {other:?}"),
  }
}

#[test]
fn derived_context_cannot_expand_capabilities() {
  let mut runtime = runtime_with_browser_manifest();
  let result = runtime.run_string("@base := docs://manual{:read(intro/title)}\n@docs := @base{:write(intro/title)}\n");
  assert!(result.is_err(), "derived write should not be allowed to expand read-only base");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeContextInvalidBinding"), "expected invalid binding error, got {error}");
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
  let root = setup_modules("~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := @foo/ok\n");

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
  let root = setup_modules("~~~mech:foo\n+> ./math.mec\nok := math/tau > 6.0\n<+ ok\n~~~\n\nresult := @foo/ok\n");

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
  let root = setup_modules("~~~mech:foo\nok := true\nother := false\n<+ ok\n<+ other\n~~~\n\nresult := @foo/ok\n");

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
  let root = setup_modules("~~~mech:foo\nhidden := true\n~~~\n\nresult := @foo/hidden\n");

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
  let root = setup_modules("result := @missing/ok\n");

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
  let root = setup_modules("@foo/x := 1\nresult := @foo/x\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let result = runtime.resolve_and_store_module_source("main.mec", options);
  assert!(result.is_err(), "prefix addressed define should be rejected while parsing/building");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("ParserErrorContext"), "expected parser error for addressed define, got {error}");
}

#[test]
fn addressed_assignment_to_context_is_still_unsupported() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n@manual/intro/title := true\nresult := @manual/intro/title\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err(), "prefix addressed define should be rejected while parsing/building");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("ParserErrorContext"), "expected parser error for addressed define, got {error}");
}

#[test]
fn addressed_assignment_with_docs_provider_is_still_unsupported() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n@manual/intro/title := true\nresult := @manual/intro/title\n");

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).in_memory_docs(InMemoryDocsProvider::new()).build().unwrap();
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err(), "prefix addressed define should be rejected while parsing/building");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("ParserErrorContext"), "expected parser error for addressed define, got {error}");
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
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .config_spec(spec)
    .build()
    .unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  run_module_as_main(&mut runtime, version)
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
  let root = setup_modules("@manual := docs://manual{:read(other/path)}\n\nresult := @manual/intro/title\n");
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
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let provider = docs_provider_with("docs://manual", "intro/title", bool_value(true));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .in_memory_docs(provider)
    .build()
    .unwrap();
  let subject = runtime.runtime_context().unwrap().subject;
  runtime.grant_capability(runtime_read_grant_for(&subject, "docs://manual", "intro/title")).unwrap();
  assert!(runtime.has_capability_grant(&subject, "docs://manual", &RuntimeCapabilityOperation::Read, "intro/title"));
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "runtime grant");
}

#[test]
fn apply_config_spec_after_build_registers_resources_and_grants() {
  let root = setup_modules("@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n");
  let spec = docs_config("intro/title", bool_value(true))
    .with_capability_grant(read_grant("docs://manual", "intro/title"));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .build()
    .unwrap();
  runtime.apply_config_spec(spec).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(run_module_as_main(&mut runtime, version).unwrap(), "apply spec");
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

  assert!(snapshot.import_edges.iter().any(|edge| {
    edge.specifier == "./math.mec"
  }));

  assert_bool_true(
    runtime.run_module(target.module_version).unwrap(),
    "workspace imported target",
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
  assert_eq!(
    refresh.refresh_diagnostics[0].severity,
    RuntimeWorkspaceDiagnosticSeverity::Error,
  );
  assert_eq!(
    refresh.refresh_diagnostics[0].target.as_deref(),
    Some("main"),
  );

  assert_eq!(refresh.snapshot.diagnostics.len(), 1);
  assert_eq!(
    refresh.snapshot.diagnostics[0].target.as_deref(),
    Some("main"),
  );
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

#[test]
fn workspace_refresh_preserves_unaffected_diagnostics() {
  let root = setup_modules("result := false\n");

  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root)
      .target("missing", "missing.mec")
      .target("main", "main.mec"),
  ).unwrap();

  let initial = workspace.load(&mut runtime, module_options()).unwrap();

  assert_eq!(initial.diagnostics.len(), 1);
  assert_eq!(initial.diagnostics[0].target.as_deref(), Some("missing"));
  assert!(initial.targets.contains_key("main"));

  std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

  let refresh = workspace.refresh(&mut runtime, module_options()).unwrap();

  assert_eq!(refresh.affected_targets, vec!["main"]);
  assert!(refresh.refresh_diagnostics.is_empty());

  assert_eq!(refresh.snapshot.diagnostics.len(), 1);
  assert_eq!(
    refresh.snapshot.diagnostics[0].target.as_deref(),
    Some("missing"),
  );

  let target = refresh.snapshot.targets.get("main").unwrap();
  assert_bool_true(
    runtime.run_module(target.module_version).unwrap(),
    "refreshed workspace target with retained diagnostic",
  );
}

#[test]
fn workspace_load_folder_discovers_non_target_sources() {
  let root = setup_modules("result := true\n");
  std::fs::create_dir_all(root.join("src/nested")).unwrap();
  std::fs::write(root.join("src/nested/lib.mec"), "value := true\n").unwrap();

  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root)
      .target("main", "main.mec")
      .folder("src"),
  ).unwrap();

  let snapshot = workspace.load(&mut runtime, module_options()).unwrap();

  assert!(snapshot.diagnostics.is_empty());
  assert!(snapshot.targets.contains_key("main"));
  assert_eq!(snapshot.targets.len(), 1);

  let discovered_path = root
    .join("src/nested/lib.mec")
    .canonicalize()
    .unwrap();

  assert!(
    snapshot.sources.values().any(|source| {
      source.path.as_ref() == Some(&discovered_path)
    }),
    "workspace snapshot should contain discovered source; sources: {:?}",
    snapshot.sources,
  );
}

#[test]
fn workspace_refresh_preserves_folder_discovered_sources() {
  let root = setup_modules("result := true\n");
  std::fs::create_dir_all(root.join("src")).unwrap();
  std::fs::write(root.join("src/lib.mec"), "value := false\n").unwrap();

  let mut runtime = runtime_with_root(&root);
  let mut workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root)
      .target("main", "main.mec")
      .folder("src"),
  ).unwrap();

  let initial = workspace.load(&mut runtime, module_options()).unwrap();
  assert!(initial.diagnostics.is_empty());

  let discovered_path = root.join("src/lib.mec").canonicalize().unwrap();
  assert!(initial.sources.values().any(|source| {
    source.path.as_ref() == Some(&discovered_path)
  }));

  std::fs::write(root.join("main.mec"), "result := false\n").unwrap();

  let refresh = workspace.refresh(&mut runtime, module_options()).unwrap();
  assert!(refresh.refresh_diagnostics.is_empty());

  assert!(refresh.snapshot.sources.values().any(|source| {
    source.path.as_ref() == Some(&discovered_path)
  }));
}

#[test]
fn workspace_watcher_watches_configured_folder() {
  let root = setup_modules("result := true\n");
  std::fs::create_dir_all(root.join("src")).unwrap();

  let workspace = RuntimeWorkspace::open(
    RuntimeWorkspaceConfig::new(&root)
      .target("main", "main.mec")
      .folder("src"),
  ).unwrap();

  let mut runtime = runtime_with_browser_manifest();
  let watcher = RuntimeWorkspaceWatcher::open(&workspace, &mut runtime).unwrap();
  let watched_paths = watcher.watched_paths();

  assert!(watched_paths.contains(&root.join("src").canonicalize().unwrap()));
}

#[test]
fn interpreter_scope_ignores_faux_tilde_fence_inside_backtick_code_block() {
  let root = setup_modules(
    "~~~mech:foo\n\
ok := true\n\
<+ ok\n\
~~~\n\
\n\
```text\n\
~~~mech:foo\n\
broken := missing\n\
~~~\n\
```\n\
\n\
result := @foo/ok\n",
  );

  let mut runtime = runtime_with_root(&root);
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  let result = runtime.run_module(version).unwrap();

  match result {
    Value::Bool(value) => assert_eq!(*value.borrow(), true),
    other => panic!("expected faux tilde fence to be ignored, got {:?}", other),
  }
}

#[test]
fn interpreter_scope_ignores_faux_backtick_fence_inside_tilde_code_block() {
  let root = setup_modules(
    "~~~mech:foo\n\
ok := true\n\
<+ ok\n\
~~~\n\
\n\
~~~text\n\
```mech:foo\n\
broken := missing\n\
```\n\
~~~\n\
\n\
result := @foo/ok\n",
  );

  let mut runtime = runtime_with_root(&root);
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  let result = runtime.run_module(version).unwrap();

  match result {
    Value::Bool(value) => assert_eq!(*value.borrow(), true),
    other => panic!("expected faux backtick fence to be ignored, got {:?}", other),
  }
}

#[test]
fn manifest_context_import_materializes_without_source_dependency() {
  let root = setup_modules("+> @ui := browser/dom\nx := 1\n");
  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let record = runtime.store().get_module_version(version).unwrap().unwrap();

  assert_eq!(record.imports.len(), 1);
  assert_eq!(record.import_edges.len(), 0);
  assert_eq!(record.dependencies.len(), 0);
  let program_scope = record.scopes.iter().find(|scope| scope.scope == SourceScope::Program).unwrap();
  assert!(program_scope.contexts.iter().any(|context| context.name == "ui"));
  assert!(record.contexts.iter().any(|context| context.name == "ui"));
}

#[test]
fn manifest_context_import_is_visible_to_scoped_address_resolution() {
  let root = setup_modules("+> @ui := browser/dom\ntitle := @ui/counter/_text\n");
  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err(), "expected resource/provider failure without browser provider");
  let error = format!("{:?}", result.err().unwrap());
  assert!(!error.contains("UnknownAddressTarget"), "context import should be visible to address resolution, got {error}");
}

#[test]
fn manifest_context_import_conflicts_with_interpreter_address_target() {
  let root = setup_modules("+> @foo := browser/dom\n\n~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := @foo/counter/_text\n");
  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_err(), "manifest context alias should conflict with interpreter target");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("AddressTargetNameConflict"), "expected address target conflict, got {error}");
  assert!(error.contains("foo"), "expected conflicting target name in error, got {error}");
}

#[test]
fn context_import_alias_is_not_bound_as_value_import() {
  let root = setup_modules("+> @ui := browser/dom\n+> ./math.mec\nresult := ui\n");
  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let record = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(record.imports.len(), 2);
  assert_eq!(record.import_edges.len(), 1);
  assert!(record.import_edges.iter().all(|edge| !matches!(edge.import.alias, Some(SourceImportAlias::Context(_)))));

  let result = runtime.run_module(version);
  assert!(result.is_err(), "context alias should not be available as a value binding");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("Undefined") || error.contains("Variable"), "expected missing value binding error, got {error}");
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn direct_runtime_normal_import_is_not_dropped() {
  let mut runtime = runtime_with_browser_manifest();
  let result = runtime.run_string("+> math/sin\nresult := sin(0)\n").unwrap();

  match result {
    Value::F64(value) => assert_eq!(*value.borrow(), 0.0),
    other => panic!("expected sin(0) to return 0.0, got {other:?}"),
  }
}

#[test]
fn manifest_context_import_ordering_supports_derived_alias() {
  let mut runtime = runtime_with_browser_manifest();
  let result = runtime.run_string("+> @ui := browser/dom\n@dom := @ui\n");
  assert!(result.is_ok(), "manifest context import should be available to later derived alias: {result:?}");
}

#[test]
fn stored_module_manifest_context_import_ordering_supports_derived_alias() {
  let root = setup_modules(
    "+> @ui := browser/dom\n\n@dom := @ui\n\nresult := @dom/counter/_text\n",
  );

  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  runtime
    .grant_capability(runtime_context_read_grant(&runtime, "browser://dom", "counter/_text"))
    .unwrap();

  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  let record = runtime.store().get_module_version(version).unwrap().unwrap();
  let program_scope = record
    .scopes
    .iter()
    .find(|scope| scope.scope == SourceScope::Program)
    .unwrap();

  let names = program_scope
    .contexts
    .iter()
    .map(|context| context.name.as_str())
    .collect::<Vec<_>>();

  assert_eq!(names, vec!["ui", "dom"]);

  let result = runtime.run_module(version);
  assert!(result.is_err(), "without browser provider this should fail at provider lookup");

  let error = format!("{:?}", result.err().unwrap());
  assert!(
    !error.contains("RuntimeContextInvalidBinding")
      && !error.contains("RuntimeContextDerivedBaseUnsupported")
      && !error.contains("references unknown context `@ui`"),
    "derived manifest alias should be materialized before @dom, got {error}",
  );
  assert!(
    error.contains("RuntimeResourceProviderNotFound")
      || error.contains("BrowserResourceProvider")
      || error.contains("provider"),
    "expected to reach provider layer after successful context materialization, got {error}",
  );
}

#[test]
fn stored_module_derived_context_before_manifest_base_is_rejected() {
  let root = setup_modules(
    "@dom := @ui\n\n+> @ui := browser/dom\n\nx := 1\n",
  );

  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  let result = runtime.run_module(version);
  assert!(result.is_err());

  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("RuntimeContextInvalidBinding")
      && error.contains("unknown context `@ui`"),
    "derived alias before base should still fail, got {error}",
  );
}

#[test]
fn direct_runtime_manifest_context_import_read_uses_browser_binding_before_lowering() {
  let mut runtime = runtime_with_browser_manifest();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "browser://dom", "counter/_text")).unwrap();
  let result = runtime.run_string("+> @ui := browser/dom\ntitle := @ui/counter/_text\n");
  assert!(result.is_err(), "expected browser provider failure without a browser provider");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderNotFound"), "expected browser provider lookup after recognizing @ui, got {error}");
  assert!(!error.contains("UnknownAddressTarget"), "@ui should not be treated as an unknown address target: {error}");
}

#[test]
fn direct_runtime_manifest_context_import_write_uses_provider_lookup() {
  let mut runtime = runtime_with_browser_manifest();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "browser://dom", "counter/_text")).unwrap();
  let result = runtime.run_string("+> @ui := browser/dom
@ui/counter/_text = \"hello\"
");
  assert!(result.is_err(), "browser write should reach provider lookup without a browser provider");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeResourceProviderNotFound"), "expected provider lookup, got {error}");
  assert!(!error.contains("UnknownAddressTarget"), "@ui should not be treated as an unknown address target: {error}");
}

#[test]
fn repeated_manifest_context_alias_in_separate_fenced_scopes_is_legal() {
  let root = setup_modules("~~~mech:foo\n+> @ui := browser/dom\na := @ui/counter/_text\n~~~\n\n~~~mech:bar\n+> @ui := browser/dom\nb := @ui/counter/_text\n~~~\n");
  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let result = runtime.resolve_and_store_module_source("main.mec", module_options());
  assert!(result.is_ok(), "same context alias in separate fenced scopes should be legal: {result:?}");
}

#[test]
fn manifest_context_import_in_fenced_scope_does_not_leak_to_program_scope() {
  let root = setup_modules("~~~mech:foo\n+> @ui := browser/dom\na := @ui/counter/_text\n~~~\n\nresult := @ui/counter/_text\n");
  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err(), "program scope should not see fenced @ui context import");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected scoped address-target failure, got {error}");
}

#[test]
fn resolving_module_with_fenced_context_import_does_not_pollute_direct_runtime_bindings() {
  let root = setup_modules(
    "~~~mech:foo\n+> @ui := browser/dom\na := @ui/counter/_text\n~~~\n"
  );

  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();

  let result = runtime.run_string("x := @ui/counter/_text\n");
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());

  assert!(
    error.contains("UnknownAddressTarget") || error.contains("Undefined") || error.contains("direct_context_target"),
    "expected @ui not to exist in direct runtime scope, got {error}"
  );
  assert!(
    !error.contains("RuntimeResourceProviderNotFound"),
    "module resolution leaked @ui into global browser bindings: {error}"
  );
}

#[test]
fn fenced_context_alias_can_match_unrelated_interpreter_name_without_resolving_as_interpreter() {
  let root = setup_modules(
    "~~~mech:foo\nok := true\n<+ ok\n~~~\n\n~~~mech:bar\n+> @foo := browser/dom\nx := @foo/counter/_text\n~~~\n"
  );

  let mut runtime = runtime_with_root_and_browser_manifest(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();

  let result = runtime.run_module_scope(
    version,
    SourceScope::Interpreter(SourceInterpreterId {
      namespace: hash_str("bar"),
      namespace_str: "bar".to_string(),
    }),
  );

  assert!(result.is_err(), "expected browser provider failure without browser provider");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("RuntimeResourceProviderNotFound")
      || error.contains("RuntimeCapabilityGrantDenied")
      || error.contains("RuntimeResourceCapabilityDenied"),
    "expected @foo in bar to resolve as context, not interpreter, got {error}"
  );
  assert!(
    !error.contains("RuntimeModuleExportNotFound"),
    "@foo in bar resolved as interpreter foo instead of local context: {error}"
  );
}
#[test]
fn direct_runtime_context_addressed_slice_is_rejected_explicitly() {
  let mut runtime = runtime_with_browser_manifest();
  let result = runtime.run_string("+> @ui := browser/dom\nx := @ui/counter[0]\n");
  assert!(result.is_err(), "context-addressed browser slices should be explicit until semantics are defined");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("context-addressed slices are not supported"), "expected explicit slice error, got {error}");
}

#[test]
fn module_manifest_catalog_validates_builder_manifests() {
  let invalids = [
    ModuleManifestConfig { name: "".to_string(), exports: vec![] },
    ModuleManifestConfig { name: "bad".to_string(), exports: vec![ModuleManifestExportConfig { name: "".to_string(), kind: ModuleManifestExportKind::Context, base_uri: "browser://dom".to_string(), operations: vec!["read".to_string()] }] },
    ModuleManifestConfig { name: "bad".to_string(), exports: vec![ModuleManifestExportConfig { name: "dom".to_string(), kind: ModuleManifestExportKind::Context, base_uri: "browser://dom".to_string(), operations: vec!["read".to_string()] }, ModuleManifestExportConfig { name: "dom".to_string(), kind: ModuleManifestExportKind::Context, base_uri: "browser://dom".to_string(), operations: vec!["write".to_string()] }] },
    ModuleManifestConfig { name: "bad".to_string(), exports: vec![ModuleManifestExportConfig { name: "dom".to_string(), kind: ModuleManifestExportKind::Context, base_uri: "browser-dom".to_string(), operations: vec!["read".to_string()] }] },
    ModuleManifestConfig { name: "bad".to_string(), exports: vec![ModuleManifestExportConfig { name: "dom".to_string(), kind: ModuleManifestExportKind::Context, base_uri: "browser://dom".to_string(), operations: vec![] }] },
    ModuleManifestConfig { name: "bad".to_string(), exports: vec![ModuleManifestExportConfig { name: "dom".to_string(), kind: ModuleManifestExportKind::Context, base_uri: "browser://dom".to_string(), operations: vec!["list".to_string()] }] },
  ];

  for manifest in invalids {
    let result = RuntimeBuilder::new().module_manifest(manifest);
    assert!(result.is_err(), "invalid manifest should be rejected by builder/catalog");
  }
}


#[derive(Debug, Default)]
struct RuntimeFakeCliState {
  env: HashMap<String, String>,
  stdout: Vec<String>,
  stderr: Vec<String>,
  env_reads: usize,
  stdout_writes: usize,
  stderr_writes: usize,
}

#[derive(Debug, Clone)]
struct RuntimeFakeCliBackend {
  state: Arc<Mutex<RuntimeFakeCliState>>,
}

impl RuntimeFakeCliBackend {
  fn new(state: Arc<Mutex<RuntimeFakeCliState>>) -> Self {
    Self { state }
  }
}

impl CliBackend for RuntimeFakeCliBackend {
  fn env_var(&self, name: &str) -> mech_core::MResult<Option<String>> {
    let mut state = self.state.lock().unwrap();
    state.env_reads += 1;
    Ok(state.env.get(name).cloned())
  }

  fn write_stdout(&mut self, text: &str) -> mech_core::MResult<()> {
    let mut state = self.state.lock().unwrap();
    state.stdout_writes += 1;
    state.stdout.push(text.to_string());
    Ok(())
  }

  fn write_stderr(&mut self, text: &str) -> mech_core::MResult<()> {
    let mut state = self.state.lock().unwrap();
    state.stderr_writes += 1;
    state.stderr.push(text.to_string());
    Ok(())
  }
}

fn runtime_with_fake_cli(state: Arc<Mutex<RuntimeFakeCliState>>) -> MechRuntime {
  with_test_cli(RuntimeBuilder::new(), RuntimeFakeCliBackend::new(state))
    .build()
    .unwrap()
}

#[derive(Debug)]
struct TestCliFactory<B: CliBackend + Clone + 'static> {
  backend: B,
  manifest: HostManifestConfig,
}

impl<B: CliBackend + Clone + 'static> TestCliFactory<B> {
  fn new(backend: B) -> Self {
    Self { backend, manifest: mech_host_cli::cli_host_manifest().unwrap() }
  }
}

impl<B: CliBackend + Clone + 'static> RuntimeHostFactory for TestCliFactory<B> {
  fn provider_name(&self) -> &str { "cli" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, _settings: &ConfigValue) -> mech_core::MResult<()> { Ok(()) }
  fn instantiate(&self, instance_name: &str, _settings: &ConfigValue) -> mech_core::MResult<RuntimeHostInstallation> {
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      resource_providers: vec![Box::new(CliResourceProvider::for_instance(instance_name, self.backend.clone()))],
    })
  }
}

fn with_test_cli<B: CliBackend + Clone + 'static>(builder: RuntimeBuilder, backend: B) -> RuntimeBuilder {
  builder
    .host_factory(Box::new(TestCliFactory::new(backend))).unwrap()
    .host_instance(HostInstanceConfig { name: "cli".to_string(), provider: "cli".to_string(), settings: ConfigValue::Map(Default::default()) })
}

#[test]
fn with_test_cli_registers_cli_provider_for_instance_bases() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/test-home");

  let mut runtime = with_test_cli(
    RuntimeBuilder::new(),
    backend,
  )
  .build()
  .unwrap();

  runtime
    .grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME"))
    .unwrap();

  let result = runtime
    .run_string("+> @env := cli/env\nhome := @env/HOME\nhome\n")
    .unwrap();

  assert_string_value(result, "/tmp/test-home");
}

#[test]
fn runtime_bind_context_export_resolves_host_interface() {
  let backend = FakeCliBackend::default();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();

  runtime.bind_context_export("out", "cli", "stdout").unwrap();

  let binding = runtime.resource_binding("out").unwrap();
  assert_eq!(binding.base_uri, "cli://cli/stdout");
  assert_eq!(binding.root_path, "");
}

#[test]
fn runtime_bind_context_export_falls_back_to_module_manifest() {
  let manifest = ModuleManifestConfig {
    name: "docs".to_string(),
    exports: vec![ModuleManifestExportConfig {
      name: "page".to_string(),
      kind: ModuleManifestExportKind::Context,
      base_uri: "docs://manual".to_string(),
      operations: vec!["read".to_string()],
    }],
  };
  let mut runtime = RuntimeBuilder::new()
    .module_manifest(manifest)
    .unwrap()
    .build()
    .unwrap();

  runtime.bind_context_export("doc", "docs", "page").unwrap();

  let binding = runtime.resource_binding("doc").unwrap();
  assert_eq!(binding.base_uri, "docs://manual");
  assert_eq!(binding.root_path, "");
}

#[test]
fn runtime_bind_context_export_host_unknown_context_does_not_fallback() {
  let manifest = ModuleManifestConfig {
    name: "cli".to_string(),
    exports: vec![ModuleManifestExportConfig {
      name: "missing".to_string(),
      kind: ModuleManifestExportKind::Context,
      base_uri: "docs://wrong".to_string(),
      operations: vec!["read".to_string()],
    }],
  };
  let backend = FakeCliBackend::default();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new().module_manifest(manifest).unwrap(),
    backend,
  )
  .build()
  .unwrap();

  let result = runtime.bind_context_export("bad", "cli", "missing");
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("HostInterfaceUnknownContext"), "got {error}");
}

#[derive(Debug, Clone, Default)]
struct FakeCliBackend {
  env: std::collections::HashMap<String, String>,
  stdout: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
  stderr: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
  calls: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl FakeCliBackend {
  fn with_env(mut self, name: &str, value: &str) -> Self {
    self.env.insert(name.to_string(), value.to_string());
    self
  }
}

impl CliBackend for FakeCliBackend {
  fn env_var(&self, name: &str) -> mech_core::MResult<Option<String>> {
    self.calls.lock().unwrap().push(format!("env:{name}"));
    Ok(self.env.get(name).cloned())
  }

  fn write_stdout(&mut self, text: &str) -> mech_core::MResult<()> {
    self.calls.lock().unwrap().push(format!("stdout:{text}"));
    self.stdout.lock().unwrap().push(text.to_string());
    Ok(())
  }

  fn write_stderr(&mut self, text: &str) -> mech_core::MResult<()> {
    self.calls.lock().unwrap().push(format!("stderr:{text}"));
    self.stderr.lock().unwrap().push(text.to_string());
    Ok(())
  }
}


#[test]
fn cli_manifest_env_import_reads_through_runtime() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  state.lock().unwrap().env.insert("HOME".to_string(), "/tmp/home".to_string());

  let mut runtime = runtime_with_fake_cli(state.clone());
  runtime
    .grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME"))
    .unwrap();

  let result = runtime
    .run_string("+> @env := cli/env
@env/HOME
")
    .unwrap();

  let result = match result {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  match result {
    Value::String(value) => assert_eq!(&*value.borrow(), "/tmp/home"),
    other => panic!("expected HOME string from cli env, got {:?}", other),
  }

  assert_eq!(state.lock().unwrap().env_reads, 1);
}

#[test]
fn cli_manifest_stdout_send_line_writes_through_runtime() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  let mut runtime = runtime_with_fake_cli(state.clone());

  runtime
    .grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line"))
    .unwrap();

  runtime
    .run_string("+> @out := cli/stdout
@out/line <- \"hello\"
")
    .unwrap();

  let state = state.lock().unwrap();
  assert_eq!(state.stdout, vec!["hello\n".to_string()]);
  assert_eq!(state.stdout_writes, 1);
}

#[test]
fn cli_manifest_stderr_send_text_writes_through_runtime() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  let mut runtime = runtime_with_fake_cli(state.clone());

  runtime
    .grant_capability(runtime_context_write_grant(&runtime, "cli://stderr", "text"))
    .unwrap();

  runtime
    .run_string("+> @err := cli/stderr
@err/text <- \"warning\"
")
    .unwrap();

  let state = state.lock().unwrap();
  assert_eq!(state.stderr, vec!["warning".to_string()]);
  assert_eq!(state.stderr_writes, 1);
}

#[test]
fn cli_stdout_assignment_errors_and_writes_nothing() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  let mut runtime = runtime_with_fake_cli(state.clone());

  runtime
    .grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line"))
    .unwrap();

  let result = runtime.run_string("+> @out := cli/stdout
@out/line = \"hello\"
");

  assert!(result.is_err());
  let state = state.lock().unwrap();
  assert!(state.stdout.is_empty());
  assert_eq!(state.stdout_writes, 0);
}

#[test]
fn cli_stdout_define_errors_and_writes_nothing() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  let mut runtime = runtime_with_fake_cli(state.clone());

  runtime
    .grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line"))
    .unwrap();

  let result = runtime.run_string("+> @out := cli/stdout
@out/line := \"hello\"
");

  assert!(result.is_err());
  let state = state.lock().unwrap();
  assert!(state.stdout.is_empty());
  assert_eq!(state.stdout_writes, 0);
}

#[test]
fn cli_env_send_errors() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  let mut runtime = runtime_with_fake_cli(state.clone());

  runtime
    .grant_capability(runtime_context_write_grant(&runtime, "cli://cli/env", "HOME"))
    .unwrap();

  let result = runtime.run_string("+> @env := cli/env
@env/HOME <- \"x\"
");

  assert!(result.is_err());
  let state = state.lock().unwrap();
  assert_eq!(state.env_reads, 0);
  assert_eq!(state.stdout_writes, 0);
  assert_eq!(state.stderr_writes, 0);
}

#[test]
fn cli_stdout_missing_write_grant_fails_before_backend() {
  let state = Arc::new(Mutex::new(RuntimeFakeCliState::default()));
  let mut runtime = runtime_with_fake_cli(state.clone());

  let result = runtime.run_string("+> @out := cli/stdout
@out/line <- \"hello\"
");

  assert!(result.is_err());
  let state = state.lock().unwrap();
  assert!(state.stdout.is_empty());
  assert_eq!(state.stdout_writes, 0);
}

#[test]
fn cli_host_env_manifest_import_reads_with_runtime_grant() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/mech-home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();
  runtime.run_string("+> @env := cli/env\nhome := @env/HOME\n").unwrap();
  let id = hash_str("home");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  match value {
    Value::String(value) => assert_eq!(&*value.borrow(), "/tmp/mech-home"),
    other => panic!("expected string home, got {other:?}"),
  }
}

#[test]
fn cli_host_stdout_send_writes_line_with_runtime_grant() {
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  runtime.run_string("+> @out := cli/stdout\n@out/line <- \"hello\"\n").unwrap();
  assert_eq!(stdout.lock().unwrap().as_slice(), &["hello\n".to_string()]);
}

#[test]
fn cli_host_stderr_send_writes_text_with_runtime_grant() {
  let backend = FakeCliBackend::default();
  let stderr = backend.stderr.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stderr", "text")).unwrap();
  runtime.run_string("+> @err := cli/stderr\n@err/text <- \"warning\"\n").unwrap();
  assert_eq!(stderr.lock().unwrap().as_slice(), &["warning".to_string()]);
}

#[test]
fn cli_host_stdout_assignment_errors_and_writes_nothing() {
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let result = runtime.run_string("+> @out := cli/stdout\n@out/line = \"hello\"\n");
  assert!(result.is_err(), "stdout assignment should error");
  assert!(stdout.lock().unwrap().is_empty(), "stdout assignment should not write");
}

#[test]
fn cli_host_stdout_definition_errors_and_writes_nothing() {
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let result = runtime.run_string("+> @out := cli/stdout\n@out/line := \"hello\"\n");
  assert!(result.is_err(), "stdout definition should error");
  assert!(stdout.lock().unwrap().is_empty(), "stdout definition should not write");
}

#[test]
fn cli_host_env_send_errors() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://cli/env", "HOME")).unwrap();
  let result = runtime.run_string("+> @env := cli/env\n@env/HOME <- \"x\"\n");
  assert!(result.is_err(), "env send should error");
}

#[test]
fn cli_host_missing_env_read_grant_fails_before_backend_call() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let calls = backend.calls.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  let result = runtime.run_string("+> @env := cli/env\nhome := @env/HOME\n");
  assert!(result.is_err(), "env read without runtime grant should fail");
  assert!(calls.lock().unwrap().is_empty(), "backend should not be called without read grant");
}

#[test]
fn cli_host_missing_stdout_write_grant_fails_before_backend_call() {
  let backend = FakeCliBackend::default();
  let calls = backend.calls.clone();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  let result = runtime.run_string("+> @out := cli/stdout\n@out/line <- \"hello\"\n");
  assert!(result.is_err(), "stdout send without runtime grant should fail");
  assert!(calls.lock().unwrap().is_empty(), "backend should not be called without write grant");
  assert!(stdout.lock().unwrap().is_empty(), "stdout should not be written without grant");
}

#[test]
fn cli_host_missing_stderr_write_grant_fails_before_backend_call() {
  let backend = FakeCliBackend::default();
  let calls = backend.calls.clone();
  let stderr = backend.stderr.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  let result = runtime.run_string("+> @err := cli/stderr\n@err/text <- \"warning\"\n");
  assert!(result.is_err(), "stderr send without runtime grant should fail");
  assert!(calls.lock().unwrap().is_empty(), "backend should not be called without write grant");
  assert!(stderr.lock().unwrap().is_empty(), "stderr should not be written without grant");
}

#[test]
fn default_cli_stdout_grant_allows_send() {
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://cli/stdout", "text")).unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  runtime.run_string("+> @out := cli/stdout\n@out/line <- \"hello\"\n").unwrap();
  assert_eq!(stdout.lock().unwrap().as_slice(), &["hello\n".to_string()]);
}

#[test]
fn narrow_env_grant_permits_path_but_denies_home() {
  let backend = FakeCliBackend::default().with_env("PATH", "/bin").with_env("HOME", "/tmp/home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://cli/env", "PATH")).unwrap();
  runtime.run_string("+> @env := cli/env\npath := @env/PATH\n").unwrap();
  let result = runtime.run_string("+> @env := cli/env\nhome := @env/HOME\n");
  assert!(result.is_err());
  assert!(format!("{:?}", result.err().unwrap()).contains("RuntimeCapabilityGrantDenied"));
}

#[test]
fn narrow_stdout_grant_permits_line_but_denies_text() {
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  runtime.run_string("+> @out := cli/stdout\n@out/line <- \"hello\"\n").unwrap();
  let result = runtime.run_string("+> @out := cli/stdout\n@out/text <- \"bad\"\n");
  assert!(result.is_err());
  assert!(format!("{:?}", result.err().unwrap()).contains("RuntimeCapabilityGrantDenied"));
  assert_eq!(stdout.lock().unwrap().as_slice(), &["hello\n".to_string()]);
}



#[test]
fn direct_run_string_preserves_named_fence_interpreter() {
  let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();

  runtime.run_string(
    "~~~mech:foo
ok := true
<+ ok
~~~
",
  ).unwrap();

  assert!(runtime.has_interpreter(hash_str("foo")));
  let values = runtime
    .symbol_values_for_interpreter(hash_str("foo"), &["ok".to_string()])
    .expect("named fence interpreter should expose symbols");
  assert_eq!(values.len(), 1, "named fence export should remain observable after direct run");
}

#[test]
fn function_body_context_read_is_rejected_before_stdout_send() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    r#"+> @out := cli/stdout
+> @env := cli/env
@out/line <- "must-not-write"
uses-env(root<string>) => <string>
  | @env/HOME.
"#,
  );

  assert!(result.is_err(), "function body context read should be rejected");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("direct_context_read_placement") || error.contains("function definitions"),
    "expected function definition context-read placement error, got {error}",
  );
  assert!(state.lock().unwrap().stdout.is_empty());
}

#[test]
fn assignment_subscript_context_read_is_preflighted_before_stdout_send() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    r#"+> @out := cli/stdout
+> @env := cli/env
@out/line <- "must-not-write"
xs := [0 0]
xs[@env/HOME] = 1
"#,
  );

  assert!(result.is_err(), "assignment target subscript context read should be preflighted");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"), "got {error}");
  assert!(state.lock().unwrap().stdout.is_empty());
}

#[test]
fn op_assignment_subscript_context_read_is_preflighted_before_stdout_send() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    r#"+> @out := cli/stdout
+> @env := cli/env
@out/line <- "must-not-write"
xs := [0 0]
xs[@env/HOME] += 1
"#,
  );

  assert!(result.is_err(), "op-assignment target subscript context read should be preflighted");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("RuntimeCapabilityGrantDenied") || error.contains("parse") || error.contains("Parse"),
    "got {error}",
  );
  assert!(state.lock().unwrap().stdout.is_empty());
}

#[test]
fn nested_env_read_denial_preflights_before_stdout_write() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let result = runtime.run_string("+> @env := cli/env\n+> @out := cli/stdout\n@out/line <- \"must-not-write\"\nx := [@env/HOME]\n");
  assert!(result.is_err());
  assert!(format!("{:?}", result.err().unwrap()).contains("RuntimeCapabilityGrantDenied"));
  assert!(stdout.lock().unwrap().is_empty());
}

#[test]
fn function_define_env_read_denial_preflights_before_stdout_write() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let result = runtime.run_string("+> @out := cli/stdout\n+> @env := cli/env\n@out/line <- \"must-not-write\"\nuses-env(root<string>) => <string>\n  | @env/HOME.\n");
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("direct_context_read_placement") || error.contains("function definitions"),
    "got {error}",
  );
  assert!(stdout.lock().unwrap().is_empty());
  // FSM traversal is covered structurally by preflight_fsm_implementation_context_capabilities;
  // add a parser-level FSM fixture when the compact syntax is less brittle for this suite.
}

#[test]
fn match_arm_pattern_context_read_fails_preflight_before_stdout_write() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}
@env := cli://env{:read(HOME)}
@out/line <- \"must-not-write\"
result := \"x\"?
  | @env/HOME => true
  | * => false.
",
  );

  assert!(result.is_err(), "match arm context read should fail in preflight");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("RuntimeCapabilityGrantDenied") || error.contains("context_read"),
    "expected context read preflight error, got {error}",
  );
  assert!(
    state.lock().unwrap().stdout.is_empty(),
    "preflight failed after stdout write: {:?}",
    state.lock().unwrap().stdout,
  );
}

#[test]
fn fsm_pipe_transition_context_read_fails_preflight_before_stdout_write() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}
@env := cli://env{:read(HOME)}
@out/line <- \"must-not-write\"
machine := #Machine() -> @env/HOME
",
  );

  assert!(result.is_err(), "FSM pipe transition context read should fail in preflight");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("RuntimeCapabilityGrantDenied") || error.contains("context_read"),
    "expected context read preflight error, got {error}",
  );
  assert!(
    state.lock().unwrap().stdout.is_empty(),
    "preflight failed after stdout write: {:?}",
    state.lock().unwrap().stdout,
  );
}

#[test]
fn fsm_declare_context_read_fails_preflight_before_stdout_write() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}
@env := cli://env{:read(HOME)}
@out/line <- \"must-not-write\"
#run := #Machine(@env/HOME)
",
  );

  assert!(result.is_err(), "FSM declaration context read should fail in preflight");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("RuntimeCapabilityGrantDenied") || error.contains("context_read"),
    "expected context read preflight error, got {error}",
  );
  assert!(
    state.lock().unwrap().stdout.is_empty(),
    "preflight failed after stdout write: {:?}",
    state.lock().unwrap().stdout,
  );
}

#[test]
fn match_arm_context_read_pattern_compares_value_when_allowed() {
  let (mut runtime, state) = runtime_with_recording_cli();
  state.lock().unwrap().env.insert(
    "MECH_MATCH_PATTERN".to_string(),
    "expected".to_string(),
  );

  let subject = runtime.runtime_context().unwrap().subject;
  runtime.grant_capability(RuntimeCapabilityGrant {
    subject,
    resource: "cli://cli/env".to_string(),
    operations: vec![RuntimeCapabilityOperation::Read],
    paths: vec!["MECH_MATCH_PATTERN".to_string()],
  }).unwrap();

  let matching = runtime.run_string(
    "@env := cli://env{:read(MECH_MATCH_PATTERN)}
result := \"expected\"?
  | @env/MECH_MATCH_PATTERN => true
  | * => false.
result
",
  ).unwrap();

  let matching = match matching {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_true(matching, "match arm context read pattern should match equal value");

  let mismatching = runtime.run_string(
    "@env := cli://env{:read(MECH_MATCH_PATTERN)}
mismatch := \"other\"?
  | @env/MECH_MATCH_PATTERN => true
  | * => false.
mismatch
",
  ).unwrap();

  let mismatching = match mismatching {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_false(mismatching, "match arm context read pattern should not bind mismatched value");

  let wrapped_matching = runtime.run_string(
    "@env := cli://env{:read(MECH_MATCH_PATTERN)}
wrapped := \"expected\"?
  | (@env/MECH_MATCH_PATTERN) => true
  | * => false.
wrapped
",
  ).unwrap();

  let wrapped_matching = match wrapped_matching {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_true(
    wrapped_matching,
    "wrapped match arm context read pattern should match equal value",
  );

  let wrapped_mismatching = runtime.run_string(
    "@env := cli://env{:read(MECH_MATCH_PATTERN)}
wrappedMismatch := \"other\"?
  | (@env/MECH_MATCH_PATTERN) => true
  | * => false.
wrappedMismatch
",
  ).unwrap();

  let wrapped_mismatching = match wrapped_mismatching {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_false(
    wrapped_mismatching,
    "wrapped match arm context read pattern should not bind mismatched value",
  );

  let mut bool_runtime = RuntimeBuilder::new()
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();

  bool_runtime.grant_capability(runtime_context_write_grant(
    &bool_runtime,
    "docs://manual",
    "flag",
  )).unwrap();
  bool_runtime.grant_capability(runtime_context_read_grant(
    &bool_runtime,
    "docs://manual",
    "flag",
  )).unwrap();

  let bool_matching = bool_runtime.run_string(
    "@manual := docs://manual{:read(flag), :write(flag)}
@manual/flag = false
matched := false?
  | @manual/flag => true
  | * => false.
matched
",
  ).unwrap();

  let bool_matching = match bool_matching {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_true(
    bool_matching,
    "boolean context match pattern should compare false to false",
  );

  let bool_mismatching = bool_runtime.run_string(
    "@manual := docs://manual{:read(flag), :write(flag)}
@manual/flag = true
mismatched := false?
  | @manual/flag => true
  | * => false.
mismatched
",
  ).unwrap();

  let bool_mismatching = match bool_mismatching {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  assert_bool_false(
    bool_mismatching,
    "boolean context match pattern should not match false to true",
  );
}

#[test]
fn run_tree_with_context_preflight_failure_emits_failure_and_profile_events() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let mut config = RuntimeConfig::default();
  config.diagnostics.profile_enabled = true;
  let mut runtime = with_test_cli(RuntimeBuilder::new().config(config), backend).build().unwrap();
  let mut context = runtime.runtime_context().unwrap();
  let tree = mech_syntax::parser::parse("+> @env := cli/env\nhome := @env/HOME\n").unwrap();
  let result = runtime.run_tree_with_context(&mut context, &tree);
  assert!(result.is_err());
  assert!(context.events.iter().any(|event| matches!(event.kind, RuntimeEventKind::ProgramStarted { .. })));
  assert!(context.events.iter().any(|event| matches!(event.kind, RuntimeEventKind::ProgramFailed { .. })));
  assert!(context.events.iter().any(|event| matches!(event.kind, RuntimeEventKind::ProgramProfiled { .. })));
}



#[test]
fn module_preflight_denial_emits_program_failed_event() {
  let root = setup_modules(
    "+> @out := cli/stdout\n+> @env := cli/env\n@out/line <- \"must-not-write\"\nhome := @env/HOME\n",
  );
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root)),
    backend,
  )
  .build()
  .unwrap();
  runtime
    .grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line"))
    .unwrap();
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();
  let mut context = runtime.runtime_context().unwrap();

  let result = runtime.run_module_with_context(&mut context, version);

  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"), "got {error}");
  assert!(
    stdout.lock().unwrap().is_empty(),
    "stdout should not be written before denied env read is reported"
  );
  assert!(
    !context
      .events
      .iter()
      .any(|event| matches!(event.kind, RuntimeEventKind::ProgramStarted { .. })),
    "graph preflight should fail before module program execution starts"
  );
}

#[test]
fn cli_host_direct_env_declaration_reads_with_runtime_grant() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/direct-home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();
  runtime.run_string("@env := cli://env{:read(HOME)}\nhome := @env/HOME\n").unwrap();
  let id = hash_str("home");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  assert_string_value(value, "/tmp/direct-home");
}

#[test]
fn cli_host_direct_stdout_declaration_sends_with_runtime_grant() {
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  runtime.run_string("@out := cli://stdout{:write(line)}\n@out/line <- \"hello\"\n").unwrap();
  assert_eq!(stdout.lock().unwrap().as_slice(), &["hello\n".to_string()]);
}

#[test]
fn cli_host_direct_stderr_declaration_sends_with_runtime_grant() {
  let backend = FakeCliBackend::default();
  let stderr = backend.stderr.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stderr", "text")).unwrap();
  runtime.run_string("@err := cli://stderr{:write(text)}\n@err/text <- \"warning\"\n").unwrap();
  assert_eq!(stderr.lock().unwrap().as_slice(), &["warning".to_string()]);
}

#[test]
fn cli_host_env_assignment_errors() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://cli/env", "HOME")).unwrap();
  let result = runtime.run_string("+> @env := cli/env\n@env/HOME = \"x\"\n");
  assert!(result.is_err(), "env assignment should error");
}

#[test]
fn cli_host_stdout_read_errors() {
  let backend = FakeCliBackend::default();
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://cli/stdout", "line")).unwrap();
  let result = runtime.run_string("+> @out := cli/stdout\nx := @out/line\n");
  assert!(result.is_err(), "stdout read should error");
}

#[test]
fn cli_context_module_read_exports_value() {
  let root = setup_modules("+> @env := cli/env\nhome := @env/HOME\n<+ home\nhome\n");
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/module-home");
  let mut runtime = with_test_cli(RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  let result = match result {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  match result {
    Value::String(value) => assert_eq!(&*value.borrow(), "/tmp/module-home"),
    other => panic!("expected string home, got {other:?}"),
  }
}

#[test]
fn cli_context_module_send_is_not_stripped() {
  let root = setup_modules("+> @out := cli/stdout\n@out/line <- \"hello\"\n");
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)), backend).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  runtime.run_module(version).unwrap();
  assert_eq!(stdout.lock().unwrap().as_slice(), &["hello\n".to_string()]);
}

#[derive(Debug, Clone)]
struct RecordingResourceProvider {
  scheme: &'static str,
  bases: Vec<String>,
  values: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Value>>>,
  writes: std::sync::Arc<std::sync::Mutex<Vec<(String, String, Value)>>>,
}

impl RecordingResourceProvider {
  fn new(scheme: &'static str, bases: &[&str]) -> Self {
    Self {
      scheme,
      bases: bases.iter().map(|base| base.to_string()).collect(),
      values: Default::default(),
      writes: Default::default(),
    }
  }

  fn with_value(self, base: &str, path: &str, value: Value) -> Self {
    self.values.lock().unwrap().insert(format!("{base}|{path}"), value);
    self
  }
}

impl RuntimeResourceProvider for RecordingResourceProvider {
  fn scheme(&self) -> &str { self.scheme }

  fn base_uris(&self) -> Vec<String> { self.bases.clone() }

  fn preflight_write(&self, _request: RuntimeResourceWritePreflightRequest) -> mech_core::MResult<()> {
    Ok(())
  }

  fn read(&self, request: RuntimeResourceReadRequest) -> mech_core::MResult<Value> {
    self.values
      .lock()
      .unwrap()
      .get(&format!("{}|{}", request.base_uri, request.path))
      .cloned()
      .ok_or_else(|| mech_core::MechError::new(RuntimeResourcePathNotFound { base_uri: request.base_uri, path: request.path }, None))
  }

  fn write(&mut self, request: RuntimeResourceWriteRequest) -> mech_core::MResult<()> {
    self.writes.lock().unwrap().push((request.base_uri.clone(), request.path.clone(), request.value.clone()));
    self.values.lock().unwrap().insert(format!("{}|{}", request.base_uri, request.path), request.value);
    Ok(())
  }
}

fn assert_string_value(value: Value, expected: &str) {
  let value = match value {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  };
  match value {
    Value::String(value) => assert_eq!(&*value.borrow(), expected),
    other => panic!("expected string value `{expected}`, got {other:?}"),
  }
}

#[test]
fn cli_context_direct_read_resolves_inside_formula_expression() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();
  runtime.run_string("+> @env := cli/env\nmsg := \"HOME=\" + @env/HOME\n").unwrap();
  let id = hash_str("msg");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  assert_string_value(value, "HOME=/tmp/home");
}

#[test]
fn cli_context_standalone_expression_returns_env_value() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();
  let value = runtime.run_string("+> @env := cli/env\n@env/HOME\n").unwrap();
  assert_string_value(value, "/tmp/home");
}


#[test]
fn docs_context_send_errors_and_does_not_write() {
  let mut runtime = RuntimeBuilder::new().in_memory_docs(InMemoryDocsProvider::new()).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();

  let result = runtime.run_string("@manual := docs://manual{:read(intro/title), :write(intro/title)}\n@manual/intro/title <- \"hello\"\n");
  assert!(result.is_err(), "docs context send should error");

  let read_result = runtime.run_string("@manual := docs://manual{:read(intro/title)}\nresult := @manual/intro/title\n");
  assert!(read_result.is_err(), "docs send should not write a readable value");
  let error = format!("{:?}", read_result.err().unwrap());
  assert!(error.contains("RuntimeResourcePathNotFound"), "expected missing docs path after failed send, got {error}");
}

#[test]
fn docs_context_write_requires_runtime_grant_before_provider_write() {
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"]);
  let writes = provider.writes.clone();
  let mut runtime = RuntimeBuilder::new().resource_provider(Box::new(provider)).build().unwrap();
  let result = runtime.run_string("@manual := docs://manual{:write(intro/title)}\n@manual/intro/title = \"hello\"\n");
  assert!(result.is_err(), "write without host grant should fail");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"), "expected host grant denial, got {error}");
  assert!(writes.lock().unwrap().is_empty(), "provider write should not be called without grant");
}

#[test]
fn docs_context_write_with_runtime_grant_reaches_provider() {
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"]);
  let writes = provider.writes.clone();
  let mut runtime = RuntimeBuilder::new().resource_provider(Box::new(provider)).build().unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  runtime.run_string("@manual := docs://manual{:write(intro/title)}\n@manual/intro/title = \"hello\"\n").unwrap();
  let writes = writes.lock().unwrap();
  assert_eq!(writes.len(), 1);
  assert_eq!(writes[0].0, "docs://manual");
  assert_eq!(writes[0].1, "intro/title");
  assert_string_value(writes[0].2.clone(), "hello");
}

#[test]
fn browser_context_subroot_normalizes_to_provider_base_path() {
  let provider = RecordingResourceProvider::new("browser", &["browser://dom"])
    .with_value("browser://dom", "counter/text", Value::String(Ref::new("count".to_string())));
  let mut runtime = RuntimeBuilder::new().resource_provider(Box::new(provider)).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "browser://dom", "counter/text")).unwrap();
  runtime.run_string("@ui := browser://dom/counter{:read(text)}\ntitle := @ui/text\n").unwrap();
  let id = hash_str("title");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  assert_string_value(value, "count");
}

#[test]
fn docs_context_subroot_normalizes_to_provider_base_path() {
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"])
    .with_value("docs://manual", "intro/title", Value::String(Ref::new("Manual".to_string())));
  let mut runtime = RuntimeBuilder::new().resource_provider(Box::new(provider)).build().unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  runtime.run_string("@manual := docs://manual/intro{:read(title)}\ntitle := @manual/title\n").unwrap();
  let id = hash_str("title");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  assert_string_value(value, "Manual");
}


#[test]
fn context_write_uses_active_runtime_context_subject() {
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"]);
  let writes = provider.writes.clone();
  let mut runtime = RuntimeBuilder::new().resource_provider(Box::new(provider)).build().unwrap();
  let mut context = runtime.runtime_context().unwrap().with_subject("task://custom");

  runtime.grant_capability(runtime_write_grant_for("task://custom", "docs://manual", "intro/title")).unwrap();

  runtime.run_string_with_context(
    &mut context,
    "@manual := docs://manual{:write(intro/title)}\n@manual/intro/title = \"hello\"\n",
  ).unwrap();

  assert_eq!(writes.lock().unwrap().len(), 1);
}

#[test]
fn context_write_does_not_accept_grant_for_default_subject_when_context_subject_differs() {
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"]);
  let writes = provider.writes.clone();
  let mut runtime = RuntimeBuilder::new().resource_provider(Box::new(provider)).build().unwrap();
  runtime.grant_capability(runtime_write_grant_for("task://main", "docs://manual", "intro/title")).unwrap();
  let mut context = runtime.runtime_context().unwrap().with_subject("task://custom");

  let result = runtime.run_string_with_context(
    &mut context,
    "@manual := docs://manual{:write(intro/title)}\n@manual/intro/title = \"hello\"\n",
  );

  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("RuntimeCapabilityGrantDenied"));
  assert!(writes.lock().unwrap().is_empty());
}

#[test]
fn unqualified_fenced_context_import_is_available_to_program_execution() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/fenced-home");
  let mut runtime = with_test_cli(RuntimeBuilder::new(), backend).build().unwrap();
  let mut context = runtime.runtime_context().unwrap().with_subject("task://fenced");
  runtime.grant_capability(RuntimeCapabilityGrant {
    subject: "task://fenced".to_string(),
    resource: "cli://cli/env".to_string(),
    operations: vec![RuntimeCapabilityOperation::Read],
    paths: vec!["HOME".to_string()],
  }).unwrap();

  runtime.run_string_with_context(
    &mut context,
    "```mech
+> @env := cli/env
home := @env/HOME
```
",
  ).unwrap();

  let id = hash_str("home");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  assert_string_value(value, "/tmp/fenced-home");
}

#[test]
fn named_fenced_context_import_write_uses_context_registry() {
  let root = setup_modules("~~~mech:bar\n+> @out := cli/stdout\n@out/line <- \"hello\"\n~~~\n");
  let backend = FakeCliBackend::default();
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root)),
    backend,
  )
  .build()
  .unwrap();

  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();

  runtime.run_module_scope(
    version,
    SourceScope::Interpreter(SourceInterpreterId {
      namespace: hash_str("bar"),
      namespace_str: "bar".to_string(),
    }),
  ).unwrap();

  assert_eq!(stdout.lock().unwrap().as_slice(), &["hello\n".to_string()]);
}

#[test]
fn named_fenced_context_import_read_exports_value() {
  let root = setup_modules("~~~mech:bar\n+> @env := cli/env\nhome := @env/HOME\n<+ home\nhome\n~~~\n");
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/named-fence-home");
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root)),
    backend,
  )
  .build()
  .unwrap();

  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();

  let result = runtime.run_module_scope(
    version,
    SourceScope::Interpreter(SourceInterpreterId {
      namespace: hash_str("bar"),
      namespace_str: "bar".to_string(),
    }),
  ).unwrap();

  assert_string_value(result, "/tmp/named-fence-home");
}

#[test]
fn module_context_read_after_context_write_uses_execution_order() {
  let root = setup_modules(
    "@manual := docs://manual{:read(intro/title), :write(intro/title)}\n@manual/intro/title = \"hello\"\nresult := @manual/intro/title\n<+ result\nresult\n",
  );
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"]);
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .resource_provider(Box::new(provider))
    .build()
    .unwrap();

  runtime.grant_capability(runtime_context_write_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();

  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();

  assert_string_value(result, "hello");
}

#[test]
fn module_context_read_after_context_write_ignores_stale_provider_value() {
  let root = setup_modules(
    "@manual := docs://manual{:read(intro/title), :write(intro/title)}\n@manual/intro/title = \"new\"\nresult := @manual/intro/title\n<+ result\nresult\n",
  );
  let provider = RecordingResourceProvider::new("docs", &["docs://manual"])
    .with_value("docs://manual", "intro/title", Value::String(Ref::new("old".to_string())));
  let mut runtime = RuntimeBuilder::new()
    .source_resolver(FileSourceResolver::new(&root))
    .resource_provider(Box::new(provider))
    .build()
    .unwrap();

  runtime.grant_capability(runtime_context_write_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();

  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();

  assert_string_value(result, "new");
}

#[test]
fn direct_context_read_resolves_inside_op_assign() {
  let provider = RecordingResourceProvider::new("docs", &["docs://numbers"])
    .with_value("docs://numbers", "increment", Value::F64(Ref::new(2.0)));
  let mut runtime = RuntimeBuilder::new()
    .resource_provider(Box::new(provider))
    .build()
    .unwrap();

  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://numbers", "increment")).unwrap();

  runtime.run_string("@numbers := docs://numbers{:read(increment)}\n~total := 1.0\ntotal += @numbers/increment\n").unwrap();

  let id = hash_str("total");
  let value = runtime.program().interpreter().symbols().borrow().get(id).unwrap().borrow().clone();
  match value {
    Value::F64(value) => assert_eq!(*value.borrow(), 3.0),
    other => panic!("expected f64 total, got {other:?}"),
  }
}


#[test]
fn cli_context_source_scope_denial_preflights_before_stdout_write() {
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new(),
    backend,
  )
  .build()
  .unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "cli://env", "HOME")).unwrap();

  let result = runtime.run_string(r#"+> @out := cli/stdout
@env := cli://env{:read(PATH)}

@out/line <- "must-not-write"
home := @env/HOME
"#);

  let error = format!("{:?}", result.err().expect("source-level env denial should fail"));
  assert!(error.contains("RuntimeResourceCapabilityDenied"), "expected source-level capability error, got {error}");
  assert!(stdout.lock().unwrap().is_empty(), "stdout write should be preflight-blocked");
}

#[test]
fn module_graph_preflight_blocks_dependency_stdout_before_main_env_denial() {
  let root = setup_modules("+> ./dep.mec\n+> @env := cli/env\nhome := @env/HOME\n");
  std::fs::write(
    root.join("dep.mec"),
    "+> @out := cli/stdout\n@out/line <- \"must-not-write\"\n",
  )
  .unwrap();
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root)),
    backend,
  )
  .build()
  .unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let mut context = runtime.runtime_context().unwrap();

  let result = runtime.run_module_with_context(&mut context, version);

  let error = format!("{:?}", result.err().expect("main env grant denial should fail"));
  assert!(error.contains("RuntimeCapabilityGrantDenied"), "expected runtime grant denial, got {error}");
  assert!(stdout.lock().unwrap().is_empty(), "dependency stdout write should be preflight-blocked");
}

#[test]
fn module_graph_preflight_blocks_current_stdout_before_dependency_denial() {
  let root = setup_modules("+> ./dep.mec\n+> @out := cli/stdout\n@out/line <- \"must-not-write\"\n");
  std::fs::write(root.join("dep.mec"), "+> @env := cli/env\nhome := @env/HOME\n").unwrap();
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root)),
    backend,
  )
  .build()
  .unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let mut context = runtime.runtime_context().unwrap();

  let result = runtime.run_module_with_context(&mut context, version);

  let error = format!("{:?}", result.err().expect("dependency env grant denial should fail"));
  assert!(error.contains("RuntimeCapabilityGrantDenied"), "expected runtime grant denial, got {error}");
  assert!(stdout.lock().unwrap().is_empty(), "current module stdout write should be preflight-blocked");
}

#[test]
fn module_graph_preflight_blocks_addressed_interpreter_import_write_before_denial() {
  let root = setup_modules(
    r#"~~~mech:foo
+> ./dep.mec
+> @env := cli/env
home := @env/HOME
<+ home
~~~

value := @foo/home
"#,
  );
  std::fs::write(
    root.join("dep.mec"),
    "+> @out := cli/stdout\n@out/line <- \"must-not-write\"\n",
  )
  .unwrap();
  let backend = FakeCliBackend::default().with_env("HOME", "/tmp/home");
  let stdout = backend.stdout.clone();
  let mut runtime = with_test_cli(
    RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root)),
    backend,
  )
  .build()
  .unwrap();
  runtime.grant_capability(runtime_context_write_grant(&runtime, "cli://stdout", "line")).unwrap();
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let mut context = runtime.runtime_context().unwrap();

  let result = runtime.run_module_with_context(&mut context, version);

  let error = format!("{:?}", result.err().expect("addressed interpreter env grant denial should fail"));
  assert!(error.contains("RuntimeCapabilityGrantDenied"), "expected runtime grant denial, got {error}");
  assert!(stdout.lock().unwrap().is_empty(), "addressed interpreter dependency write should be preflight-blocked");
}

#[test]
fn top_level_context_send_writes_stdout() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}\n@out/line <- \"top-level-send-ok\"\n\"done\"\n",
  );

  assert!(result.is_ok(), "top-level send failed: {result:?}");
  let stdout = state.lock().unwrap().stdout.clone();
  assert_eq!(stdout, vec!["top-level-send-ok\n".to_string()]);
}
#[test]
fn unknown_context_send_target_fails_preflight_before_writes() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}\n@out/line <- \"must-not-write\"\n@missing/line <- \"boom\"\n",
  );

  assert!(result.is_err(), "missing context send should fail");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("missing"), "expected missing context in error, got {error}");
  assert!(
    state.lock().unwrap().stdout.is_empty(),
    "preflight failed after stdout write: {:?}",
    state.lock().unwrap().stdout,
  );
}
#[test]
fn context_assignment_to_send_only_cli_stream_fails_preflight_before_writes() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}
@out/line <- \"must-not-write\"
@out/line = \"done\"
",
  );

  assert!(result.is_err(), "stdout assignment should fail in preflight");
  let error = format!("{:?}", result.err().unwrap());
  assert!(
    error.contains("send-only") || error.contains("use <-"),
    "expected send-only assignment error, got {error}",
  );
  assert!(
    state.lock().unwrap().stdout.is_empty(),
    "preflight failed after stdout write: {:?}",
    state.lock().unwrap().stdout,
  );
}

#[test]
fn context_send_inside_function_body_fails_runtime_preflight() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}\nemit() = result<string> := @out/line <- \"must-not-write\".\n",
  );

  assert!(result.is_err(), "nested function send should fail");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("function body"), "expected function body placement error, got {error}");
  assert!(state.lock().unwrap().stdout.is_empty());
}
#[test]
fn context_send_inside_fsm_transition_fails_runtime_preflight() {
  let (mut runtime, state) = runtime_with_recording_cli();
  grant_runtime_stdout_line(&mut runtime);

  let result = runtime.run_string(
    "@out := cli://stdout{:write(line)}\n#machine(x) -> :start\n:start -> @out/line <- \"must-not-write\"\n.\n",
  );

  assert!(result.is_err(), "FSM send should fail");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("FSM transition"), "expected FSM transition placement error, got {error}");
  assert!(state.lock().unwrap().stdout.is_empty());
}
#[test]
fn context_assignment_inside_function_body_fails_runtime_preflight() {
  let mut runtime = RuntimeBuilder::new()
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();

  runtime.grant_capability(runtime_context_write_grant(&runtime, "docs://manual", "intro/title")).unwrap();

  let result = runtime.run_string(
    "@manual := docs://manual{:write(intro/title)}\nemit() = result<bool> := @manual/intro/title = true.\n",
  );

  assert!(result.is_err(), "nested function context assignment should fail");
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("function body"), "expected function body placement error, got {error}");
}
#[test]
fn top_level_context_assignment_still_writes_and_reads() {
  let mut runtime = RuntimeBuilder::new()
    .in_memory_docs(InMemoryDocsProvider::new())
    .build()
    .unwrap();

  runtime.grant_capability(runtime_context_write_grant(&runtime, "docs://manual", "intro/title")).unwrap();
  runtime.grant_capability(runtime_context_read_grant(&runtime, "docs://manual", "intro/title")).unwrap();

  let result = runtime.run_string(
    "@manual := docs://manual{:write(intro/title), :read(intro/title)}\n@manual/intro/title = true\nresult := @manual/intro/title\n",
  ).unwrap();

  assert_bool_true(result, "top-level context assignment");
}

#[test]
fn module_interpreter_address_preflight_allows_non_context_target() {
  let root = setup_modules("~~~mech:foo
ok := true
<+ ok
~~~

result := @foo/ok
");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  assert_bool_true(runtime.run_module(version).unwrap(), "interpreter address from program");
}

#[test]
fn module_unknown_address_target_still_fails_before_execution() {
  let root = setup_modules("result := @missing/HOME
");
  let mut runtime = runtime_with_root(&root);
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected unknown address target, got {error}");
  assert!(error.contains("missing"), "expected missing target in error, got {error}");
}

#[test]
fn module_function_unknown_address_target_is_preflighted_before_send() {
  fn token(kind: mech_core::TokenKind, text: &str) -> mech_core::Token {
    mech_core::Token::new(kind, mech_core::SourceRange::default(), text.chars().collect())
  }

  fn ident(name: &str) -> mech_core::Identifier {
    mech_core::Identifier { name: token(mech_core::TokenKind::Identifier, name) }
  }

  fn addressed_var(target: &str, name: &str) -> mech_core::Expression {
    mech_core::Expression::Var(mech_core::Var { name: ident(name), context: Some(ident(target)), kind: None })
  }

  let tree = mech_core::Program {
    title: None,
    body: mech_core::Body {
      sections: vec![mech_core::Section {
        subtitle: None,
        elements: vec![mech_core::SectionElement::MechCode(vec![
          (mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(mech_core::ContextDeclaration {
            name: ident("out"),
            base: mech_core::ContextBase::ResourceUri(token(mech_core::TokenKind::String, "cli://cli/stdout")),
            capabilities: vec![mech_core::ContextCapabilityDeclaration {
              operation: ident("write"),
              scope: mech_core::ContextCapabilityScope::Path(ident("line")),
            }],
          })), None),
          (mech_core::MechCode::Statement(mech_core::Statement::ContextSend(mech_core::ContextSend {
            target: mech_core::Var { name: ident("line"), context: Some(ident("out")), kind: None },
            expression: mech_core::Expression::Literal(mech_core::Literal::String(mech_core::MechString {
              text: token(mech_core::TokenKind::String, "must-not-write"),
            })),
          })), None),
          (mech_core::MechCode::FunctionDefine(mech_core::FunctionDefine {
            name: ident("lookup"),
            input: vec![],
            output: vec![],
            statements: vec![mech_core::Statement::VariableDefine(mech_core::VariableDefine {
              mutable: false,
              var: mech_core::Var { name: ident("value"), context: None, kind: None },
              expression: addressed_var("missing", "HOME"),
            })],
            match_arms: vec![],
          }), None),
          (mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(mech_core::VariableDefine {
            mutable: false,
            var: mech_core::Var { name: ident("result"), context: None, kind: None },
            expression: mech_core::Expression::Literal(mech_core::Literal::Boolean(token(mech_core::TokenKind::True, "true"))),
          })), None),
        ])],
      }],
    },
  };

  let state = Arc::new(Mutex::new(RecordingCliState::default()));
  let mut runtime = RuntimeBuilder::new()
    .host_factory(Box::new(RecordingCliHostFactory {
      manifest: mech_host_cli::cli_host_manifest().unwrap(),
      state: state.clone(),
    }))
    .unwrap()
    .host_instance(HostInstanceConfig {
      name: "cli".to_string(),
      provider: "cli".to_string(),
      settings: ConfigValue::Map(Default::default()),
    })
    .build()
    .unwrap();

  grant_runtime_stdout_line(&mut runtime);

  let index = SourceIndex::from_program(&tree);
  let version = runtime
    .store_resolved_module_source(
      ResolvedSource::new("main.mec", "memory://main.mec", MechSourceCode::Tree(tree))
        .with_imports(index.all_imports())
        .with_exports(index.all_exports())
        .with_contexts(index.all_contexts())
        .with_address_references(index.all_address_references())
        .with_scopes(index.module_scopes()),
      module_options(),
    )
    .unwrap();

  let result = runtime.run_module(version);
  assert!(result.is_err());

  let error = format!("{:?}", result.err().unwrap());
  assert!(error.contains("UnknownAddressTarget"), "expected unknown address target, got {error}");
  assert!(state.lock().unwrap().stdout.is_empty(), "stdout write leaked before module preflight failed");
}

#[test]
fn module_function_pattern_interpreter_address_is_literal_not_capture() {
  let root = setup_modules(r#"~~~mech:cfg
STATE := "secret"
<+ STATE
~~~

pick(x<string>) => <string>
  | @cfg/STATE => "matched"
  | * => "missed".

result := pick("not-secret") == "missed"
"#);

  let mut runtime = runtime_with_root(&root);
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  assert_bool_true(
    runtime.run_module(version).unwrap(),
    "module interpreter address pattern should compare, not capture",
  );
}

#[test]
fn module_function_pattern_interpreter_address_matches_export_value() {
  let root = setup_modules(r#"~~~mech:cfg
STATE := "secret"
<+ STATE
~~~

pick(x<string>) => <string>
  | @cfg/STATE => "matched"
  | * => "missed".

result := pick("secret") == "matched"
"#);

  let mut runtime = runtime_with_root(&root);
  let version = runtime
    .resolve_and_store_module_source("main.mec", module_options())
    .unwrap()
    .unwrap();

  assert_bool_true(
    runtime.run_module(version).unwrap(),
    "module interpreter address pattern should match exported value",
  );
}



#[test]
fn run_bytecode_does_not_leave_symbol_state_for_next_source() {
  let mut compiler_program = MechProgram::new(MechProgramConfig::default());
  compiler_program.run_string("x := 2").unwrap();
  let bytecode = compiler_program.compile_bytecode().unwrap();

  let mut runtime = RuntimeBuilder::new().build().unwrap();
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_source_with_context(&mut context, &MechSourceCode::ByteCode(bytecode)).unwrap();

  let result = runtime
    .run_source_with_context(&mut context, &MechSourceCode::String("x := 3".to_string()))
    .unwrap();

  assert_eq!(result, Value::F64(Ref::new(3.0)));
}

#[test]
fn run_bytecode_error_restores_previous_program_state() {
  let mut compiler_program = MechProgram::new(MechProgramConfig::default());
  compiler_program.run_string("x := 2").unwrap();
  let mut bytecode = compiler_program.compile_bytecode().unwrap();
  bytecode.truncate(bytecode.len().saturating_sub(1));

  let mut runtime = RuntimeBuilder::new().build().unwrap();
  let mut context = runtime.runtime_context().unwrap();
  runtime
    .run_source_with_context(&mut context, &MechSourceCode::String("y := 1".to_string()))
    .unwrap();
  assert!(runtime.run_source_with_context(&mut context, &MechSourceCode::ByteCode(bytecode)).is_err());

  let result = runtime
    .run_source_with_context(&mut context, &MechSourceCode::String("z := y + 1".to_string()))
    .unwrap();

  assert_eq!(result, Value::F64(Ref::new(2.0)));
}

#[test]
fn run_source_with_context_bytecode_emits_completion_and_profile_events() {
  let mut compiler_program = MechProgram::new(MechProgramConfig::default());
  compiler_program.run_string("x := 1 + 2").unwrap();
  let bytecode = compiler_program.compile_bytecode().unwrap();

  let mut config = RuntimeConfig::default();
  config.diagnostics.profile_enabled = true;
  let mut runtime = RuntimeBuilder::new().config(config).build().unwrap();
  let mut context = runtime.runtime_context().unwrap();
  let result = runtime.run_source_with_context(&mut context, &MechSourceCode::ByteCode(bytecode)).unwrap();

  assert_eq!(result, Value::F64(Ref::new(3.0)));
  assert!(context.events.iter().any(|event| matches!(event.kind, RuntimeEventKind::ProgramStarted { .. })));
  assert!(context.events.iter().any(|event| matches!(event.kind, RuntimeEventKind::ProgramCompleted { .. })));
  assert!(context.events.iter().any(|event| matches!(event.kind, RuntimeEventKind::ProgramProfiled { .. })));
}
