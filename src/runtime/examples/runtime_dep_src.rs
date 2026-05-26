use std::fmt::Display;
use std::fs;

use mech_core::{MResult, MechSourceCode};

use mech_runtime::{
  FileSourceResolver,
  ResolvedSource,
  RuntimeBuilder,
  ModuleBuildOptions,
  SourceRequest,
};

fn short_text(text: &str) -> String {
  if text.len() <= 18 {
    return text.to_string();
  }

  format!("{}…{}", &text[..8], &text[text.len() - 8..])
}

fn short(id: impl Display) -> String {
  short_text(&id.to_string())
}

fn runtime_target() -> String {
  format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn main() -> MResult<()> {
  let root = std::env::temp_dir().join("mech-runtime-dependency-source-demo");

  let _ = fs::remove_dir_all(&root);
  fs::create_dir_all(&root)?;

  let dep_path = root.join("dep.mec");

  fs::write(
    &dep_path,
    r#"
      y := 20 + 22
      y
    "#,
  )?;

  println!("root: {}", root.display());
  println!("dep:  {}", dep_path.display());

  let resolver = FileSourceResolver::new(&root);

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(resolver)
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:runtime-dependency-source");

  let mut resolved = ResolvedSource::new(
    "index",
    "memory://runtime-dependency-source-demo/index.mec",
    MechSourceCode::String(
      r#"
        x := 41 + 1
        x
      "#
      .to_string(),
    ),
  );

  resolved.dependencies.push(SourceRequest::new("dep.mec"));

  let target = runtime_target();

  let options = ModuleBuildOptions::new(
    env!("CARGO_PKG_VERSION"),
    "mech-current",
    &target,
    &[],
    &[],
  );

  let module_version = runtime.build_module_from_resolved_source_with_context(
    &mut context,
    resolved,
    options,
  )?;

  println!("module version: {}", short(module_version));

  let module_record = runtime
    .store()
    .get_module_version(module_version)?
    .expect("expected parent module version to exist");

  println!(
    "dependency count: {}",
    module_record.dependencies.len(),
  );

  assert_eq!(
    module_record.dependencies.len(),
    1,
    "expected parent module version to record one dependency",
  );

  let dependency_version = module_record.dependencies[0];

  let dependency_record = runtime
    .store()
    .get_module_version(dependency_version)?
    .expect("expected dependency module version to exist");

  println!("dependency version: {}", short(dependency_version));
  println!("dependency module:  {}", short(dependency_record.module));

  runtime.shutdown()?;

  println!();
  println!("events:");

  for event in runtime.list_events(None)? {
    println!(
      "  #{:03} {:24} {:?}",
      event.sequence,
      event.name(),
      event.kind,
    );
  }

  Ok(())
}