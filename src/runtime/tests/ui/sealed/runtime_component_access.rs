use mech_runtime::MechRuntime;

fn main() {
  let mut runtime = MechRuntime::builder().build().unwrap();
  let _ = runtime.program();
  let _ = runtime.take_program();
  let _ = runtime.capability_kernel_mut();
}
