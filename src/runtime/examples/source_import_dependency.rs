use std::fs;

use mech_core::MResult;
use mech_runtime::{
  FileSourceResolver, ModuleBuildOptions, RuntimeBuilder, SourceRequest,
};

fn runtime_target() -> String {
  format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn main() -> MResult<()> {
  let root = std::env::temp_dir().join(format!(
    "mech-source-import-dependency-{}",
    std::process::id()
  ));
  if root.exists() {
    fs::remove_dir_all(&root)?;
  }
  fs::create_dir_all(&root)?;

  fs::write(
    root.join("index.mec"),
    "++ ./dep.mec\n\nx := 42\nx\n",
  )?;
  fs::write(root.join("dep.mec"), "y := 1\ny\n")?;

  let resolver = FileSourceResolver::new(&root);
  let mut runtime = RuntimeBuilder::new().source_resolver(resolver).build()?;
  let target = runtime_target();
  let options = ModuleBuildOptions::new(
    env!("CARGO_PKG_VERSION"),
    "mech-current",
    &target,
    &[],
    &[],
  );

  let version = runtime
    .resolve_and_store_module_source(SourceRequest::new("index.mec"), options)?
    .expect("expected index.mec to resolve");

  let record = runtime
    .store()
    .get_module_version(version)?
    .expect("expected module version");

  assert_eq!(record.dependencies.len(), 1);

  runtime.run_module(version)?;

  fs::remove_dir_all(&root)?;

  Ok(())
}
