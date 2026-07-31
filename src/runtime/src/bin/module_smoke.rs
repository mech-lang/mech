use mech_runtime::{FileSourceResolver, ModuleBuildOptions, RuntimeBuilder, module_namespace_for_import};

fn main() {
  let root = std::env::current_dir().unwrap().join("src/runtime/examples/modules");
  println!("root path: {}", root.display());
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().expect("module version");
  println!("main module version: {}", version);
  let main = runtime.store().get_module_version(version).unwrap().unwrap();
  println!("dependency versions: {:?}", main.dependencies);
  println!("import edges:");
  for edge in &main.import_edges {
    println!(
      "  - scope={:?} specifier={} dependency={}",
      edge.scope,
      edge.import.specifier,
      edge.dependency
    );
  }
  println!("flat imports:");
  for import in &main.imports {
    println!(
      "  - specifier={} kind={:?} namespace={:?}",
      import.specifier,
      import.kind,
      module_namespace_for_import(import)
    );
  }
  println!("scoped imports by scope:");
  for scope in &main.scopes {
    println!("  - scope={:?}", scope.scope);
    for import in &scope.imports {
      println!(
        "      specifier={} kind={:?} namespace={:?}",
        import.specifier,
        import.kind,
        module_namespace_for_import(import)
      );
    }
  }
  println!("stored exports:");
  for export in &main.exports {
    println!("  - {}", export.name);
  }
  let result = runtime.run_module(version).expect("module smoke run_module should succeed");
  println!("run result: {:?}", result);
  println!("event list:");
  for event in runtime.store().list_events(None).unwrap() {
    println!("  - seq={} {} {:?}", event.sequence, event.name(), event.kind);
  }
  println!("module smoke passed");
}
