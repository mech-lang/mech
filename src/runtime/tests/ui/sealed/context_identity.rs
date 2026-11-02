use mech_runtime::{
  RuntimeId, TransactionId, MechRuntime,
};

fn main() {
  let runtime = MechRuntime::builder().build().unwrap();
  let mut context = runtime.runtime_context().unwrap();
  context.runtime = RuntimeId(2);
  context.subject = "forged".to_string();
  context.transaction = Some(TransactionId(3));
}
