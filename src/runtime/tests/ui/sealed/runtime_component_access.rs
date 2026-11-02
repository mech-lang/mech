use mech_runtime::MechRuntime;

fn main() {
  let mut runtime = MechRuntime::builder().build().unwrap();
  drop(runtime.program());
  drop(runtime.take_program());
  drop(runtime.capability_kernel_mut());
}
