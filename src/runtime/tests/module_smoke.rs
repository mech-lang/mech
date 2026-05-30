use mech_core::{hash_str, MechSourceCode, Ref, Value};
use mech_runtime::{BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId, ClosureHostFunction, FileSourceResolver, InMemoryDocsProvider, ModuleScopeMetadata, ModuleBuildOptions, ResolvedSource, RuntimeBuilder, RuntimeContextRegistry, SourceContextBase, SourceContextCapability, SourceContextCapabilityScope, SourceContextDeclaration, SourceInterpreterId, SourceKind, SourceRequest, SourceResolver, SourceScope};

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

fn bool_value(value: bool) -> Value {
  Value::Bool(Ref::new(value))
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
  let version = runtime.resolve_and_store_module_source("main.mec", module_options()).unwrap().unwrap();
  let result = runtime.run_module(version).unwrap();
  assert_bool_true(result, "context docs read");
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
