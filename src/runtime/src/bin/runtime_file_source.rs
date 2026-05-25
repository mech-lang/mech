use std::fmt::Display;
use std::fs;

use mech_core::{MResult, Value};

use mech_runtime::{
  FileSourceResolver,
  RuntimeBuilder,
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

fn print_value(label: &str, value: &Value) {
  println!("{}: {:?}", label, value);
}

fn main() -> MResult<()> {
  let root = std::env::temp_dir().join("mech-runtime-file-source-demo");

  let _ = fs::remove_dir_all(&root);
  fs::create_dir_all(&root)?;

  let main_path = root.join("index.mec");

  fs::write(
    &main_path,
    r#"
      x := 41 + 1
      x
    "#,
  )?;

  println!("root:   {}", root.display());
  println!("source: {}", main_path.display());

  let resolver = FileSourceResolver::new(&root);

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(resolver)
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:runtime-file-source");

  let module_version = runtime
    .build_module_from_request_with_context(
      &mut context,
      SourceRequest::new("index.mec"),
      env!("CARGO_PKG_VERSION"),
      "mech-current",
      &[],
      &[],
    )?
    .expect("expected index.mec to resolve");

  println!("module version: {}", short(module_version));

  let mut run_context = runtime
    .runtime_context()?
    .with_subject("program:runtime-file-source")
    .with_module_version(module_version);

  let value = runtime.run_module_with_context(
    &mut run_context,
    module_version,
  )?;

  print_value("result", &value);

  match value {
    Value::F64(value) => {
      assert_eq!(*value.borrow(), 42.0);
    }
    Value::I64(value) => {
      assert_eq!(*value.borrow(), 42);
    }
    Value::U64(value) => {
      assert_eq!(*value.borrow(), 42);
    }
    other => {
      println!("result was not a simple scalar: {:?}", other);
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