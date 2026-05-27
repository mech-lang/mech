use mech_core::Value;
use mech_runtime::{FileSourceResolver, ModuleBuildOptions, RuntimeBuilder, SourceRequest, SourceResolver};

fn setup_modules(main_source: &str) -> std::path::PathBuf {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-smoke-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), main_source).unwrap();
  root
}

#[test]
fn resolver_metadata() {
  let root = setup_modules("+> ./math.mec\nok := math/tau > 6.0\n");
  let resolver = FileSourceResolver::new(&root);
  let main = resolver.resolve(&SourceRequest::new("main.mec")).unwrap().unwrap();
  assert_eq!(main.imports.len(), 1);
  assert_eq!(main.dependencies.len(), 1);
  assert!(main.dependencies[0].specifier == "./math.mec" || main.dependencies[0].specifier == "./math.mec".trim());
  assert!(main.dependencies[0].referrer.is_some());

  let dep = resolver.resolve(&main.dependencies[0]).unwrap().unwrap();
  assert!(dep.exports.iter().any(|e| e.name == "tau"));
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
#[ignore = "requires module isolation or cleanup of non-imported dependency bindings"]
fn file_import_does_not_imply_wildcard_import() {
  let root = setup_modules("+> ./math.mec\nok := tau > 6.0\n");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_err());
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
