use mech_runtime::{FileSourceResolver, ModuleBuildOptions, RuntimeBuilder};

fn main() {
  let root = std::env::current_dir().unwrap().join("src/runtime/examples/modules");
  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&root)).build().unwrap();
  let options = ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]);
  let version = runtime.resolve_and_store_module_source("main.mec", options).unwrap().expect("module version");
  runtime.run_module(version).unwrap();
  println!("module smoke passed");
}
