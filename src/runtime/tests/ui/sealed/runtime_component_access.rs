use mech_runtime::MechRuntime;

fn main() {
  let mut runtime = MechRuntime::builder().build().unwrap();
  let _ = runtime.program();
  let _ = runtime.take_program();
  let _ = runtime.store_mut();
  let _ = runtime.capability_kernel_mut();
  let _ = runtime.source_resolver_mut();
  let _ = runtime.host_registry_mut();
  let _ = runtime.host_policy_mut();
  let _ = runtime.scheduler_mut();
  let _ = runtime.scheduler_policy_mut();
}
