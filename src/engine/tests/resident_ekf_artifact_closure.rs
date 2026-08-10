#![cfg(feature = "resident-ekf-artifact")]

use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, BytecodeInstruction, ChangeDetectionPolicy,
    ConstantId, DeliveryMode, ExecutionResourceRequest, ExternalInteraction, MResult,
    ObservationReplayPolicy, OutputConstruction, ParsedProgram, ResolvedOperationContract,
    ResourceDelivery, ResourceIntent, ShapeRule, ValueData, snapshot::SequenceView,
};
use mech_engine::__gate_d::{
    FrozenEkfCompilationServices, compile_frozen_ekf_source, frozen_ekf_compiler_catalog,
};
use mech_engine::{MechProgram, MechProgramConfig};

const SOURCE: &str =
    include_str!("../../../tests/architecture/resident-activation/ekf-source-v1.mec");

#[test]
fn ordinary_source_and_bytecode_close_the_same_frozen_artifact() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;

    assert_eq!(
        compilation.source_artifact.revision(),
        compilation.decoded_artifact.revision()
    );
    assert_eq!(compilation.source_closure, compilation.decoded_closure);
    assert_eq!(compilation.source_artifact.nodes().len(), 21);
    assert!(compilation.source_artifact.inputs().is_empty());
    assert_eq!(compilation.source_closure.resident_kernels.len(), 15);
    assert_eq!(compilation.source_closure.integrity_predicates.len(), 3);
    assert_eq!(compilation.source_closure.state_updates.len(), 2);
    assert_eq!(compilation.source_closure.constraints.len(), 3);
    assert_eq!(compilation.source_closure.input.name, "frame");
    assert_eq!(compilation.source_closure.output.name, "estimate");
    assert!(
        compilation
            .source_artifact
            .contracts()
            .iter()
            .all(|contract| matches!(contract, ResolvedOperationContract::Declared(_)))
    );
    assert_eq!(services.reads.len(), 1);
    assert_eq!(services.live_bindings.len(), 1);
    assert_eq!(
        compilation.resource_request,
        ExecutionResourceRequest {
            base_uri: "gate-d://ekf/frame".to_string(),
            path: "sample".to_string(),
            context_name: "frame".to_string(),
            operation: "read".to_string(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }
    );
    let parsed = ParsedProgram::from_bytes(&compilation.bytecode)?;
    assert_eq!(
        parsed
            .requirements
            .iter()
            .filter(|requirement| matches!(requirement, ApplicationRequirement::Resource(_)))
            .count(),
        1
    );
    assert_eq!(
        compilation
            .source_artifact
            .contracts()
            .iter()
            .filter(|contract| matches!(
                contract,
                ResolvedOperationContract::Declared(contract)
                    if matches!(contract.interaction, ExternalInteraction::Observation(_))
            ))
            .count(),
        1
    );
    let always_changed = compilation
        .source_artifact
        .contracts()
        .iter()
        .filter_map(|contract| match contract {
            ResolvedOperationContract::Declared(contract)
                if contract.outputs.iter().any(|output| {
                    output.change_detection == ChangeDetectionPolicy::AlwaysChanged
                }) =>
            {
                Some(contract)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(always_changed.len(), 1);
    assert!(matches!(
        always_changed[0].interaction,
        ExternalInteraction::Observation(ref observation)
            if observation.replay == ObservationReplayPolicy::CaptureAsInputFact
    ));
    assert!(
        compilation
            .source_artifact
            .contracts()
            .iter()
            .all(|contract| {
                let ResolvedOperationContract::Declared(contract) = contract else {
                    return false;
                };
                !matches!(contract.interaction, ExternalInteraction::Pure)
                    || contract.outputs.iter().all(|output| {
                        output.change_detection != ChangeDetectionPolicy::AlwaysChanged
                    })
            })
    );
    assert!(
        compilation
            .source_artifact
            .contracts()
            .iter()
            .any(|contract| {
                matches!(
                    contract,
                    ResolvedOperationContract::Declared(contract)
                        if matches!(
                            contract.interaction,
                            ExternalInteraction::Observation(ref observation)
                                if observation.replay == ObservationReplayPolicy::CaptureAsInputFact
                        )
                )
            })
    );
    let observed_frame = services.live_bindings[0]
        .target
        .borrow()
        .as_vecf64()
        .expect("frozen observation must be a concrete f64 vector");
    assert!((0..compilation.source_artifact.constants().len()).all(|index| {
        let value = compilation
            .source_artifact
            .constants()
            .get(ConstantId::new(index as u32))
            .expect("constant ids are dense");
        !matches!(
            value.data(),
            ValueData::Matrix(matrix)
                if matches!(
                    matrix.elements(),
                    SequenceView::F64(values)
                        if values.iter().map(|value| value.to_f64()).eq(observed_frame.iter().copied())
                )
        )
    }));
    Ok(())
}

#[test]
fn declaration_markers_stay_in_bytecode_but_not_the_artifact() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let parsed = ParsedProgram::from_bytes(&compilation.bytecode)?;
    let catalog = frozen_ekf_compiler_catalog()?;
    let declaration_instructions = parsed
        .instructions
        .iter()
        .filter_map(BytecodeInstruction::runtime_function)
        .filter_map(|function| catalog.runtime_entry_by_raw(function))
        .filter(|entry| entry.name.starts_with("VariableDefine"))
        .count();
    assert!(declaration_instructions >= 1);
    assert!(
        compilation
            .source_artifact
            .nodes()
            .iter()
            .all(|node| { !node.operation.operation_name.starts_with("VariableDefine") })
    );
    Ok(())
}

#[test]
fn both_state_updates_are_complete_declared_writes() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    for update in &compilation.source_closure.state_updates {
        let node = &compilation.source_artifact.nodes()[update.node.get() as usize];
        let Some(ResolvedOperationContract::Declared(contract)) =
            compilation.source_artifact.contracts().get(node.contract)
        else {
            panic!("state update must use a declared contract");
        };
        assert_eq!(contract.interaction, ExternalInteraction::Pure);
        assert_eq!(contract.inputs.len(), 1);
        assert_eq!(contract.inputs[0].access, AccessMode::Read);
        assert_eq!(contract.inputs[0].delivery, DeliveryMode::Signal);
        assert_eq!(contract.outputs.len(), 1);
        assert_eq!(contract.outputs[0].access, AccessMode::Write);
        assert_eq!(contract.outputs[0].delivery, DeliveryMode::Signal);
        assert_eq!(
            contract.outputs[0].construction,
            OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 }
            }
        );
        assert_eq!(contract.outputs[0].alias, AliasPolicy::NoAlias);
        assert_eq!(
            contract.outputs[0].change_detection,
            ChangeDetectionPolicy::KernelReported
        );
    }
    Ok(())
}

#[test]
fn changed_frozen_constant_is_rejected_by_semantic_admission() {
    let changed = SOURCE.replace("dt := 0.05", "dt := 0.06");
    let mut services = FrozenEkfCompilationServices::default();
    let error = compile_frozen_ekf_source(&changed, &mut services).unwrap_err();
    assert_eq!(error.kind_name(), "FrozenEkfArtifactClosureError");
}

#[test]
fn same_schema_observation_and_planning_payloads_preserve_program_identity() -> MResult<()> {
    let source_observation_a = [1.0, 2.0, 3.0, 4.0];
    let source_observation_b = [101.0, -22.0, 303.5, 0.125];
    let planning_a = [11.25, -0.375, 22.5, 0.125];
    let planning_b = [-500.0, 19.0, 0.25, 88.0];
    let mut source_services_a =
        FrozenEkfCompilationServices::from_frames(source_observation_a, planning_a);
    let mut source_services_b =
        FrozenEkfCompilationServices::from_frames(source_observation_b, planning_b);

    let compilation_a = compile_frozen_ekf_source(SOURCE, &mut source_services_a)?;
    let compilation_b = compile_frozen_ekf_source(SOURCE, &mut source_services_b)?;

    assert!(source_services_a.planned_reads.is_empty());
    assert!(source_services_b.planned_reads.is_empty());
    assert_eq!(source_services_a.reads.len(), 1);
    assert_eq!(source_services_b.reads.len(), 1);
    let parsed_a = ParsedProgram::from_bytes(&compilation_a.bytecode)?;
    let parsed_b = ParsedProgram::from_bytes(&compilation_b.bytecode)?;
    assert_eq!(parsed_a.constants, parsed_b.constants);
    assert_eq!(parsed_a.constant_blob, parsed_b.constant_blob);
    assert_eq!(
        compilation_a.source_artifact.constants().len(),
        compilation_b.source_artifact.constants().len()
    );
    assert_eq!(
        (0..compilation_a.source_artifact.constants().len())
            .map(|index| compilation_a
                .source_artifact
                .constants()
                .entry(ConstantId::new(index as u32))
                .unwrap()
                .hash())
            .collect::<Vec<_>>(),
        (0..compilation_b.source_artifact.constants().len())
            .map(|index| compilation_b
                .source_artifact
                .constants()
                .entry(ConstantId::new(index as u32))
                .unwrap()
                .hash())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        compilation_a.source_artifact.revision(),
        compilation_b.source_artifact.revision()
    );
    assert_eq!(
        compilation_a.decoded_artifact.revision(),
        compilation_b.decoded_artifact.revision()
    );
    assert_eq!(compilation_a.source_closure, compilation_b.source_closure);
    assert_eq!(compilation_a.bytecode, compilation_b.bytecode);

    let catalog = frozen_ekf_compiler_catalog()?;
    let mut runtime_a =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    let mut runtime_b = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);
    let runtime_observation_a = [9.0, 8.0, 7.0, 6.0];
    let runtime_observation_b = [-4.0, 3.0, -2.0, 1.0];
    let mut runtime_services_a =
        FrozenEkfCompilationServices::from_frames(runtime_observation_a, planning_a);
    let mut runtime_services_b =
        FrozenEkfCompilationServices::from_frames(runtime_observation_b, planning_b);

    runtime_a.run_bytecode_with_services(&compilation_a.bytecode, &mut runtime_services_a)?;
    runtime_b.run_bytecode_with_services(&compilation_b.bytecode, &mut runtime_services_b)?;

    assert_eq!(runtime_services_a.planned_reads.len(), 1);
    assert_eq!(runtime_services_b.planned_reads.len(), 1);
    assert_eq!(runtime_services_a.reads.len(), 1);
    assert_eq!(runtime_services_b.reads.len(), 1);
    assert_eq!(runtime_services_a.live_bindings.len(), 1);
    assert_eq!(runtime_services_b.live_bindings.len(), 1);
    assert_eq!(
        runtime_services_a.live_bindings[0]
            .target
            .borrow()
            .as_vecf64()
            .expect("live binding must contain the actual observation"),
        runtime_observation_a
    );
    assert_eq!(
        runtime_services_b.live_bindings[0]
            .target
            .borrow()
            .as_vecf64()
            .expect("live binding must contain the actual observation"),
        runtime_observation_b
    );
    Ok(())
}
