use mech_runtime::{
  ClosureHostFunction, HostFunctionTransactionMode,
};

fn main() {
  drop(HostFunctionTransactionMode::ImmediateOnly);
  assert_ne!(std::mem::size_of::<ClosureHostFunction>(), 0);
}
