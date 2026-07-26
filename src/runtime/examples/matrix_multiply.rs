use std::fmt::Display;
use std::sync::Arc;

use mech_core::{MResult, MechMatrix as Matrix, ToMatrix, Value};

use mech_runtime::{
  host_arg_matrix_f64,
  host_call0,
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

fn matrix_f64(
  elements: Vec<f64>,
  rows: usize,
  cols: usize,
) -> Value {
  Value::MatrixF64(<f64 as ToMatrix>::to_matrix(
    elements,
    rows,
    cols,
  ))
}

fn matrix_scalar_f64(matrix: &Matrix<f64>) -> Option<f64> {
  let shape = matrix.shape();

  if shape != vec![1, 1] {
    return None;
  }

  Some(matrix.index2d(1, 1))
}

fn main() -> MResult<()> {
  let v1 = Arc::new(vec![1.0_f64, 2.0, 3.0]);
  let v2 = Arc::new(vec![4.0_f64, 5.0, 6.0]);

  let expected = v1
    .iter()
    .zip(v2.iter())
    .map(|(a, b)| a * b)
    .sum::<f64>();

  let mut runtime = RuntimeBuilder::new()
    .capability_kernel(BasicCapabilityKernel::new())
    .build()?;

  println!("runtime: {}", short(runtime.id()));
  println!("rust v1: {:?}", v1);
  println!("rust v2: {:?}", v2);
  println!("expected v1 ** v2': {}", expected);

  {
    let v1 = v1.clone();

    runtime.register_mech_host_function(ClosureHostFunction::new_pure(
      "demo/matrix/v1",
      move |_services, _context, args| {
        host_call0("demo/matrix/v1", &args, || {
          matrix_f64((*v1).clone(), 1, 3)
        })
      },
    ))?;
  }

  {
    let v2 = v2.clone();

    runtime.register_mech_host_function(ClosureHostFunction::new_pure(
      "demo/matrix/v2",
      move |_services, _context, args| {
        host_call0("demo/matrix/v2", &args, || {
          matrix_f64((*v2).clone(), 1, 3)
        })
      },
    ))?;
  }

  let subject = BasicSubject::new("program:matrix-multiply");

  for (id, name) in [
    (1, "demo/matrix/v1"),
    (2, "demo/matrix/v2"),
  ] {
    runtime.grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(id),
      &subject,
      &BasicResource::new(format!("host:{}", name)),
      [BasicOperation::new("call")],
    )))?;
  }

  let source = r#"
    v1 := demo/matrix/v1()
    v2 := demo/matrix/v2()
    result := v1 ** v2'
    result
  "#;

  println!();
  println!("mech source:");
  println!("{}", source.trim());

  let mut context = runtime
    .runtime_context()?
    .with_subject("program:matrix-multiply");

  let value = runtime.run_string_with_context(
    &mut context,
    source,
  )?;

  println!();
  println!("program result: {:?}", value);

  let result = host_arg_matrix_f64(
    "matrix-multiply result",
    &[value],
    0,
  )?;
  let stored = matrix_scalar_f64(&result)
    .expect("Mech program did not return a 1x1 matrix");

  assert!(
    (stored - expected).abs() < f64::EPSILON,
    "expected {}, got {}",
    expected,
    stored,
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
