use std::fmt::Display;
use std::sync::Arc;

use mech_core::{MResult, Value};

use mech_runtime::{
  BasicCapability,
  BasicCapabilityKernel,
  BasicOperation,
  BasicResource,
  BasicSubject,
  CapabilityId,
  DeterministicHostFunction,
  RuntimeBuilder,
  TaskRecord,
};

use mech_runtime::host::*;

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
    .host_function(DeterministicHostFunction::new(
      "demo/value/wrap",
      |_context, _args| Ok(value_string(String::new())),
      |_context, args| {
        let input = host_arg_string("demo/value/wrap", &args, 0)?;
        Ok(value_string(format!("rust-wrap({})", input)))
      },
    ))?
    .host_function(DeterministicHostFunction::new(
      "demo/value/append",
      |_context, _args| Ok(value_string(String::new())),
      |_context, args| {
        let input = host_arg_string("demo/value/append", &args, 0)?;
        let suffix = host_arg_string("demo/value/append", &args, 1)?;
        Ok(value_string(format!("{}{}", input, suffix)))
      },
    ))?
    .host_function(DeterministicHostFunction::new(
      "demo/value/inspect",
      |_context, args| host_arg_cloned("demo/value/inspect", &args, 0),
      |_context, args| {
        host_arg_cloned("demo/value/inspect", &args, 0)
      },
    ))?
    .build()?;

  println!("runtime: {}", short(runtime.id()));

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

  let task = TaskRecord::new(
    runtime.next_task_id(),
    "program:host-value-roundtrip",
  )
    .with_capabilities(vec![
      CapabilityId(1),
      CapabilityId(2),
      CapabilityId(3),
    ]);
  let mut context = runtime.context_for_task(&task)?;

  let value = runtime.run_string_with_context(
    &mut context,
    source,
  )?;

  println!();
  println!("program result: {}", display_value(value.as_value()));

  assert_string(value.into_value(), "rust-wrap(mech) runtime");

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
