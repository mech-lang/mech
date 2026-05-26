use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechSourceCode};

use mech_runtime::{
  ResolvedSource,
  RuntimeBuilder,
  ModuleBuildOptions,
  SourceRequest,
  SourceResolver,
};

#[derive(Debug, Clone)]
struct DiamondSourceResolver {
  counts: Arc<Mutex<HashMap<String, usize>>>,
}

impl DiamondSourceResolver {
  fn new() -> Self {
    Self {
      counts: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  fn counts(&self) -> Arc<Mutex<HashMap<String, usize>>> {
    self.counts.clone()
  }

  fn count_resolution(&self, specifier: &str) -> usize {
    let mut counts = self
      .counts
      .lock()
      .expect("resolver counts mutex should not be poisoned");

    let count = counts.entry(specifier.to_string()).or_insert(0);
    *count += 1;
    *count
  }

  fn root_source() -> ResolvedSource {
    let mut source = ResolvedSource::new(
      "root",
      "memory://diamond/root.mec",
      MechSourceCode::String(
        r#"
          root := 1
          root
        "#
        .to_string(),
      ),
    );

    source.dependencies.push(SourceRequest::new("a.mec"));
    source.dependencies.push(SourceRequest::new("b.mec"));

    source
  }

  fn a_source() -> ResolvedSource {
    let mut source = ResolvedSource::new(
      "a",
      "memory://diamond/a.mec",
      MechSourceCode::String(
        r#"
          a := 10
          a
        "#
        .to_string(),
      ),
    );

    source.dependencies.push(SourceRequest::new("shared.mec"));

    source
  }

  fn b_source() -> ResolvedSource {
    let mut source = ResolvedSource::new(
      "b",
      "memory://diamond/b.mec",
      MechSourceCode::String(
        r#"
          b := 20
          b
        "#
        .to_string(),
      ),
    );

    source.dependencies.push(SourceRequest::new("shared.mec"));

    source
  }

  fn shared_source(resolve_count: usize) -> ResolvedSource {
    let value = if resolve_count == 1 { 42 } else { 99 };

    ResolvedSource::new(
      "shared",
      "memory://diamond/shared.mec",
      MechSourceCode::String(
        format!(
          r#"
            shared := {}
            shared
          "#,
          value,
        ),
      ),
    )
  }
}

impl SourceResolver for DiamondSourceResolver {
  fn resolve(
    &self,
    request: &SourceRequest,
  ) -> MResult<Option<ResolvedSource>> {
    let resolve_count = self.count_resolution(&request.specifier);

    let resolved = match request.specifier.as_str() {
      "root.mec" => Some(Self::root_source()),
      "a.mec" => Some(Self::a_source()),
      "b.mec" => Some(Self::b_source()),
      "shared.mec" => Some(Self::shared_source(resolve_count)),
      _ => None,
    };

    Ok(resolved)
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
  let resolver = DiamondSourceResolver::new();
  let counts = resolver.counts();

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(resolver)
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:runtime-dependency-diamond");

  let target = runtime_target();
  let options = ModuleBuildOptions::new(
    env!("CARGO_PKG_VERSION"),
    "mech-current",
    &target,
    &[],
    &[],
  );

  let root_version = runtime
    .build_module_from_request_with_context(
      &mut context,
      SourceRequest::new("root.mec"),
      options,
    )?
    .expect("expected root.mec to resolve");

  println!("root version: {}", short(root_version));

  let root_record = runtime
    .store()
    .get_module_version(root_version)?
    .expect("expected root module version to exist");

  assert_eq!(
    root_record.dependencies.len(),
    2,
    "root should depend on a and b",
  );

  let a_version = root_record.dependencies[0];
  let b_version = root_record.dependencies[1];

  let a_record = runtime
    .store()
    .get_module_version(a_version)?
    .expect("expected a module version to exist");

  let b_record = runtime
    .store()
    .get_module_version(b_version)?
    .expect("expected b module version to exist");

  assert_eq!(
    a_record.dependencies.len(),
    1,
    "a should depend on shared",
  );

  assert_eq!(
    b_record.dependencies.len(),
    1,
    "b should depend on shared",
  );

  let shared_from_a = a_record.dependencies[0];
  let shared_from_b = b_record.dependencies[0];

  println!("a version:      {}", short(a_version));
  println!("b version:      {}", short(b_version));
  println!("shared from a:  {}", short(shared_from_a));
  println!("shared from b:  {}", short(shared_from_b));

  assert_eq!(
    shared_from_a,
    shared_from_b,
    "a and b should record the same shared ModuleVersionId",
  );

  let shared_record = runtime
    .store()
    .get_module_version(shared_from_a)?
    .expect("expected shared module version to exist");

  println!("shared module:  {}", short(shared_record.module));

  let counts = counts
    .lock()
    .expect("resolver counts mutex should not be poisoned");

  let shared_resolve_count = counts
    .get("shared.mec")
    .copied()
    .unwrap_or(0);

  println!("shared resolve count: {}", shared_resolve_count);

  assert_eq!(
    shared_resolve_count,
    2,
    "diamond graph should request shared.mec through both a and b",
  );

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
