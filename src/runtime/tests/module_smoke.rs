use mech_core::{Ref, Value};
use mech_runtime::{BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId, ClosureHostFunction, FileSourceResolver, ModuleBuildOptions, RuntimeBuilder, SourceRequest, SourceResolver, SourceScope};

fn setup_modules(main_source: &str) -> std::path::PathBuf {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-smoke-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\nsecret := 42\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), main_source).unwrap();
  root
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
