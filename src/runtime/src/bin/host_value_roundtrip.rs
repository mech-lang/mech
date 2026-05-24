use std::fmt::Display;
use std::sync::Arc;

use mech_core::{MResult, Value};

use mech_runtime::{
  host_arg_cloned,
  host_arg_string,
  value_string,
  BasicCapability,
  BasicCapabilityKernel,
  BasicOperation,
  BasicResource,
  BasicSubject,
  CapabilityId,
  ClosureHostFunction,
  RuntimeBuilder,
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

fn display_value(value: &Value) -> String {
  match value {
    Value::String(text) => {
      format!("String({:?})", text.borrow())
    }
    Value::F64(value) => {
      format!("F64({})", *value.borrow())
    }
    Value::I64(value) => {
      format!("I64({})", *value.borrow())
    }
    Value::Bool(value) => {
      format!("Bool({})", *value.borrow())
    }
    other => {
      format!("{:?}", other)
    }
  }
}

fn assert_string(value: Value, expected: &str) {
  match value {
    Value::String(text) => {
      assert_eq!(&*text.borrow(), expected);
    }
    other => {
      panic!("expected string `{}`, got {:?}", expected, other);
    }
  }
}

fn main() -> MResult<()> {
  let mut runtime = RuntimeBuilder::new()
    .capability_kernel(BasicCapabilityKernel::new())
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/value/wrap",
    |_services, _context, args| {
      let input = host_arg_string("demo/value/wrap", &args, 0)?;

      let output = format!("rust-wrap({})", input);

      println!("rust demo/value/wrap:");
      println!("  input:  {:?}", input);
      println!("  output: {:?}", output);

      Ok(value_string(output))
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/value/append",
    |_services, _context, args| {
      let input = host_arg_string("demo/value/append", &args, 0)?;
      let suffix = host_arg_string("demo/value/append", &args, 1)?;

      let output = format!("{}{}", input, suffix);

      println!("rust demo/value/append:");
      println!("  input:  {:?}", input);
      println!("  suffix: {:?}", suffix);
      println!("  output: {:?}", output);

      Ok(value_string(output))
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/value/inspect",
    |_services, _context, args| {
      let value = host_arg_cloned("demo/value/inspect", &args, 0)?;

      println!("rust demo/value/inspect:");
      println!("  value: {}", display_value(&value));

      // Return the value unchanged so the Mech program's final result is the
      // value Rust inspected.
      Ok(value)
    },
  ))?;

  let subject = BasicSubject::new("program:host-value-roundtrip");

  for (id, name) in [
    (1, "demo/value/wrap"),
    (2, "demo/value/append"),
    (3, "demo/value/inspect"),
  ] {
    runtime.grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(id),
      &subject,
      &BasicResource::new(format!("host:{}", name)),
      [BasicOperation::new("call")],
    )))?;
  }

  let source = r#"
    base := "mech"
    wrapped := demo/value/wrap(base)
    combined := demo/value/append(wrapped, " runtime")
    demo/value/inspect(combined)
  "#;

  println!();
  println!("mech source:");
  println!("{}", source.trim());

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-value-roundtrip");

  let value = runtime.run_string_with_context(
    &mut context,
    source,
  )?;

  println!();
  println!("program result: {}", display_value(&value));

  assert_string(value, "rust-wrap(mech) runtime");

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