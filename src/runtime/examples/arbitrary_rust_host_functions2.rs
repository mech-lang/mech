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
  ClosureHostFunction,
  RuntimeBuilder,
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

fn print_value(label: &str, value: &Value) {
  println!("{}: {:?}", label, value);
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

fn assert_f64(value: Value, expected: f64) {
  let actual = *value
    .as_f64()
    .expect("expected f64-compatible value")
    .borrow();

  assert!(
    (actual - expected).abs() < f64::EPSILON,
    "expected {}, got {}",
    expected,
    actual,
  );
}

fn assert_bool(value: Value, expected: bool) {
  match value {
    Value::Bool(actual) => {
      assert_eq!(*actual.borrow(), expected);
    }
    other => {
      panic!("expected bool `{}`, got {:?}", expected, other);
    }
  }
}

fn main() -> MResult<()> {
  let mut runtime = RuntimeBuilder::new()
    .capability_kernel(BasicCapabilityKernel::new())
    .build()?;

  println!("runtime: {}", short(runtime.id()));

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/text/shout",
    |_services, _context, args| {
      host_call1("demo/text/shout", &args, |text: String| {
        text.to_uppercase()
      })
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/text/join",
    |_services, _context, args| {
      host_call2("demo/text/join", &args, |left: String, right: String| {
        format!("{} {}", left, right)
      })
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/math/add",
    |_services, _context, args| {
      host_call2("demo/math/add", &args, |left: f64, right: f64| {
        left + right
      })
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/math/affine",
    |_services, _context, args| {
      host_call3(
        "demo/math/affine",
        &args,
        |x: f64, scale: f64, offset: f64| {
          (x * scale) + offset
        },
      )
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/bool/not",
    |_services, _context, args| {
      host_call1("demo/bool/not", &args, |value: bool| {
        !value
      })
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/optional/greet",
    |_services, _context, args| {
      let name = host_arg_optional_string(
        "demo/optional/greet",
        &args,
        0,
      )?
      .unwrap_or_else(|| "world".to_string());

      Ok(value_string(format!("hello {}", name)))
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/value/echo",
    |_services, _context, args| {
      Ok(host_arg_cloned("demo/value/echo", &args, 0)?)
    },
  ))?;

  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/result/checked-reciprocal",
    |_services, _context, args| {
      host_call_result1(
        "demo/result/checked-reciprocal",
        &args,
        |x: f64| -> MResult<f64> {
          if x == 0.0 {
            return Err(mech_core::MechError::new(
              DemoDivideByZeroError,
              None,
            ));
          }

          Ok(1.0 / x)
        },
      )
    },
  ))?;

  let subject = BasicSubject::new("program:host-args-showcase");

  for (id, name) in [
    (1, "demo/text/shout"),
    (2, "demo/text/join"),
    (3, "demo/math/add"),
    (4, "demo/math/affine"),
    (5, "demo/bool/not"),
    (6, "demo/optional/greet"),
    (7, "demo/value/echo"),
    (8, "demo/result/checked-reciprocal"),
  ] {
    runtime.grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(id),
      &subject,
      &BasicResource::new(format!("host:{}", name)),
      [BasicOperation::new("call")],
    )))?;
  }

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/text/shout("hello runtime")"#,
  )?;

  print_value("shout", &value);
  assert_string(value, "HELLO RUNTIME");

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/text/join("hello", "mech")"#,
  )?;

  print_value("join", &value);
  assert_string(value, "hello mech");

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/math/add(20, 22)"#,
  )?;

  print_value("add", &value);
  assert_f64(value, 42.0);

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/math/affine(10, 4, 2)"#,
  )?;

  print_value("affine", &value);
  assert_f64(value, 42.0);

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/bool/not(false)"#,
  )?;

  print_value("not", &value);
  assert_bool(value, true);

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/optional/greet()"#,
  )?;

  print_value("optional default", &value);
  assert_string(value, "hello world");

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/optional/greet("mech")"#,
  )?;

  print_value("optional provided", &value);
  assert_string(value, "hello mech");

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/value/echo("raw value")"#,
  )?;

  print_value("raw echo", &value);
  assert_string(value, "raw value");

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:host-args-showcase");

  let value = runtime.run_string_with_context(
    &mut context,
    r#"demo/result/checked-reciprocal(4)"#,
  )?;

  print_value("checked reciprocal", &value);
  assert_f64(value, 0.25);

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

#[derive(Debug, Clone)]
struct DemoDivideByZeroError;

impl mech_core::MechErrorKind for DemoDivideByZeroError {
  fn name(&self) -> &str {
    "DemoDivideByZero"
  }

  fn message(&self) -> String {
    "division by zero in demo/result/checked-reciprocal".to_string()
  }
}