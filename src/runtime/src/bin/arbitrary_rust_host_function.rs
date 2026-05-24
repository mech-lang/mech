use std::fmt::Display;
use std::sync::Arc;

use mech_core::{MResult, Ref, Value};

use mech_runtime::{
  BasicCapability,
  BasicCapabilityKernel,
  BasicOperation,
  BasicResource,
  BasicSubject,
  CapabilityId,
  ClosureHostFunction,
  InMemorySourceResolver,
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

fn main() -> MResult<()> {
  let mut source_resolver = InMemorySourceResolver::new();

  source_resolver.insert_string(
    "main",
    r#"
      demo/echo("hello from rust")
    "#,
  )?;

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(source_resolver)
    .capability_kernel(BasicCapabilityKernel::new())
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/echo",
    |_services, _context, args| {
      Ok(args
        .into_iter()
        .next()
        .unwrap_or(Value::Empty))
    },
  ))?;

  runtime.grant_capability(Arc::new(BasicCapability::new(
    CapabilityId(1),
    &BasicSubject::new("program:arbitrary-rust-host"),
    &BasicResource::new("host:demo/echo"),
    [BasicOperation::new("call")],
  )))?;

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:arbitrary-rust-host");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/echo("hello from rust")"#,
  )?;
  
  println!("result: {:?}", value);

  match value {
    Value::String(text) => {
      assert_eq!(&*text.borrow(), "hello from rust");
    }
    other => {
      panic!("expected string result, got {:?}", other);
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