use super::RuntimeProgramRoute;
use crate::{ResidentDurabilityPolicy, RuntimeBuilder};

#[test]
fn malformed_bytecode_fails_closed_without_installing_a_route() {
    let mut runtime = RuntimeBuilder::default().build().unwrap();
    let error = runtime
        .load_production_bytecode_program(b"not bytecode v1", ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert!(error.kind_message().starts_with("InvalidBytecode:"));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
    assert_eq!(runtime.program_execution_info().legacy_turns, 0);
}
