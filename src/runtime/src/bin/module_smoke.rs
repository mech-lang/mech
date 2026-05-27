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
  println!("stored imports:");
  for import in &main.imports {
    println!(
      "  - specifier={} kind={:?} namespace={:?}",
      import.specifier,
      import.kind,
      module_namespace_for_import(import)
    );
  }
  println!("stored exports:");
  for export in &main.exports {
    println!("  - {}", export.name);
  }
  let result = runtime.run_module(version);
  println!("run result: {:?}", result);
  result.unwrap();
  println!("module smoke passed");
}
