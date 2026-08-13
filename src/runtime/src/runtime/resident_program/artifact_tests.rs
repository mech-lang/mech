use super::{RuntimeProgramLoadOptions, RuntimeProgramRoute};
use crate::{ResidentDurabilityPolicy, ResidentRoutingPolicy, RuntimeBuilder};

#[test]
fn artifact_load_options_default_to_prefer_resident_and_volatile() {
    let options = RuntimeProgramLoadOptions::default();
    assert_eq!(options.routing, ResidentRoutingPolicy::PreferResident);
    assert_eq!(options.durability, ResidentDurabilityPolicy::Volatile);
}

#[test]
fn malformed_bytecode_fails_closed_without_installing_a_route() {
    let mut runtime = RuntimeBuilder::default().build().unwrap();
    let error = runtime
        .load_bytecode_program(b"not bytecode v1", RuntimeProgramLoadOptions::default())
        .unwrap_err();
    assert!(error.kind_message().starts_with("InvalidBytecode:"));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
    assert_eq!(runtime.program_execution_info().legacy_turns, 0);
}
