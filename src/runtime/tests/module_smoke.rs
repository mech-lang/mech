use mech_runtime::{FileSourceResolver, ModuleBuildOptions, RuntimeBuilder, SourceRequest, SourceResolver};

fn setup_modules() -> std::path::PathBuf {
  let root = std::env::temp_dir().join(format!("mech-runtime-module-smoke-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("math.mec"), "tau := 6.28318\n<+ tau\n").unwrap();
  std::fs::write(root.join("main.mec"), "+> ./math.mec\nok := tau > 6.0\n").unwrap();
  root
}

#[test]
fn resolver_metadata() {
  let root = setup_modules();
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
  let root = setup_modules();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let main_version = runtime.store().get_module_version(version).unwrap().unwrap();
  assert_eq!(main_version.dependencies.len(), 1);
  let dep_version = runtime.store().get_module_version(main_version.dependencies[0]).unwrap().unwrap();
  assert!(dep_version.exports.iter().any(|e| e.name == "tau"));
}

#[test]
fn run_module() {
  let root = setup_modules();
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().unwrap();
  let result = runtime.run_module(version);
  assert!(result.is_ok());
}
