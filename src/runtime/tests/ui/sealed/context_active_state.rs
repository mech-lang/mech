use mech_runtime::MechRuntime;

fn main() {
  let runtime = MechRuntime::builder().build().unwrap();
  let mut context = runtime.runtime_context().unwrap();
  context.budget.max_steps = Some(u64::MAX);
  context.events.clear();
  context.access.reads.clear();
}
