use std::collections::HashMap;
use std::fmt::Display;

use mech_core::{MResult, MechSourceCode};

use mech_runtime::{
  ResolvedSource,
  RuntimeBuilder,
  SourceRequest,
  SourceResolver,
};

#[derive(Debug)]
struct CycleSourceResolver {
  sources: HashMap<String, ResolvedSource>,
}

impl CycleSourceResolver {
  fn new() -> Self {
    let mut a = ResolvedSource::new(
      "a",
      "memory://cycle/a.mec",
      MechSourceCode::String("a := 1\na".to_string()),
    );

    let mut b = ResolvedSource::new(
      "b",
      "memory://cycle/b.mec",
      MechSourceCode::String("b := 2\nb".to_string()),
    );

    a.dependencies.push(SourceRequest::new("b.mec"));
    b.dependencies.push(SourceRequest::new("a.mec"));

    let mut sources = HashMap::new();
    sources.insert("a.mec".to_string(), a);
    sources.insert("b.mec".to_string(), b);

    Self { sources }
  }
}

impl SourceResolver for CycleSourceResolver {
  fn resolve(
    &self,
    request: &SourceRequest,
  ) -> MResult<Option<ResolvedSource>> {
    Ok(self.sources.get(&request.specifier).cloned())
  }
}

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
  let resolver = CycleSourceResolver::new();

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(resolver)
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:runtime-dependency-cycle");

  let target = runtime_target();

  let result = runtime.build_module_from_request_with_context(
    &mut context,
    SourceRequest::new("a.mec"),
    env!("CARGO_PKG_VERSION"),
    "mech-current",
    &target,
    &[],
    &[],
  );

  match result {
    Ok(version) => {
      panic!(
        "expected dependency cycle error, got module version {:?}",
        version,
      );
    }
    Err(error) => {
    println!("error: {:?}", error);

    let error_text = format!("{:?}", error);

    assert!(
        error_text.contains("RuntimeModuleDependencyCycle"),
        "expected RuntimeModuleDependencyCycle error, got {:?}",
        error,
    );
    }
  }

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