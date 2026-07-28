use mech_runtime::{
  ClosureHostFunction, HostFunctionTransactionMode,
};

fn main() {
  let _ = HostFunctionTransactionMode::ImmediateOnly;
  let _ = std::mem::size_of::<ClosureHostFunction>();
}
