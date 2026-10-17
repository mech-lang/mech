use super::RuntimeProgramRoute;
use crate::{ResidentDurabilityPolicy, RuntimeBuilder};

#[test]
fn malformed_bytecode_fails_closed_without_installing_a_route() {
    let mut runtime = RuntimeBuilder::default().build().unwrap();
    let error = runtime
        .load_bytecode_program(b"not bytecode v1", ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert!(error.kind_message().starts_with("InvalidBytecode:"));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn bytecode_loader_rejects_parallel_execution_and_requirement_authorities() {
    use mech_core::{
        ApplicationRequirement, BytecodeInstruction, BytecodeProgram, ConstantStoreBuilder,
        ExecutionHostFunctionRequest, OperationContractTable, SchemaTableBuilder, write_bytecode,
        write_bytecode_with_artifact,
    };
    use mech_engine::{
        ApplicationRequirementTable, ProgramArtifactDraft, encode_program_artifact_sections,
    };
    use std::collections::{BTreeMap, BTreeSet};

    let schemas = SchemaTableBuilder::new().finish().unwrap().into_parts().0;
    let constants = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts()
        .0;
    let artifact = ProgramArtifactDraft {
        schemas,
        constants,
        contracts: OperationContractTable::empty(),
        requirements: ApplicationRequirementTable::empty(),
        inputs: Box::new([]),
        slots: Box::new([]),
        nodes: Box::new([]),
        bindings: Box::new([]),
        outputs: Box::new([]),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    let mut envelope = BytecodeProgram {
        register_count: 0,
        constants: Vec::new(),
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: Vec::new(),
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    };
    let legacy = write_bytecode(&envelope).unwrap();
    envelope.register_count = 1;
    envelope
        .instructions
        .push(BytecodeInstruction::Return { src: 0 });
    let hybrid = write_bytecode_with_artifact(&envelope, &sections).unwrap();
    envelope.register_count = 0;
    envelope.instructions.clear();
    envelope
        .requirements
        .push(ApplicationRequirement::HostFunction(
            ExecutionHostFunctionRequest {
                name: "unreferenced/host".to_owned(),
            },
        ));
    let forged_requirements = write_bytecode_with_artifact(&envelope, &sections).unwrap();
    for (bytes, reason) in [
        (legacy, "MissingArtifactSections"),
        (hybrid, "ParallelLegacyExecutionAuthority"),
        (forged_requirements, "RequirementTableMismatch"),
    ] {
        let mut runtime = RuntimeBuilder::default().build().unwrap();
        let error = runtime
            .load_bytecode_program(&bytes, ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert!(error.kind_message().starts_with("InvalidBytecode:"));
        assert!(error.display_message().contains(reason), "{error:?}");
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
    }
}
