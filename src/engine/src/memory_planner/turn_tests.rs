use super::derive_turn_call;
use mech_core::{
    AccessMode, AliasPolicy, BoundCall, CallMemoryPlan, CallMemoryPlanningRequest,
    ChangeDetectionPolicy, CurrentMemoryFootprint, DeliveryMode, ExecutionTarget,
    ExternalInteraction, ImplementationMemoryClass, InputPortLayout, InputPortPolicy,
    MemoryBudgetLimits, MemoryFootprintWitness, MemoryLifetime, MemoryPlanError,
    MemoryWitnessStage, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    PortDirection, RegionAccessPlan, ResolvedOperationDescriptor, RuntimeFunctionId, ShapeRule,
    TargetMemoryProfile, ValueCell, physical_storage_descriptor, plan_call_memory,
};
use std::collections::BTreeMap;

fn template() -> CallMemoryPlan {
    let value = ValueCell::from_exact(String::new()).unwrap();
    let descriptor = value.resolved_descriptor().unwrap();
    let operation = ResolvedOperationDescriptor::from_name(
        "test/owned-turn-template",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::SemanticHash,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let bound = BoundCall::syntax_directed(
        operation,
        vec![descriptor.clone()].into_boxed_slice(),
        vec![descriptor].into_boxed_slice(),
        RuntimeFunctionId::from_name("OwnedTurnTemplate"),
        ExecutionTarget::DirectRuntime,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let storage =
        physical_storage_descriptor(value.representation(), &target, MemoryLifetime::Activation);
    let deferred = MemoryFootprintWitness::Deferred(MemoryWitnessStage::Turn);
    let mut call = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &bound,
        input_storage: &[storage.clone()],
        output_storage: &[storage],
        input_witnesses: &[deferred],
        output_witnesses: &[deferred],
        published_output_witnesses: &[deferred],
        implementation_memory: ImplementationMemoryClass::NoAdditionalScratch,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    call.target.limits = MemoryBudgetLimits {
        max_output_elements: Some(0),
        max_output_bytes: Some(0),
        max_temporary_bytes: Some(0),
        max_cloned_bytes: Some(0),
        max_retained_nodes: Some(0),
        max_comparison_work: Some(0),
        max_compute_work: Some(0),
        max_scalar_instructions: Some(0),
        max_transfer_bytes: Some(0),
        max_storage_buffer_bytes: Some(0),
        max_storage_bindings: Some(0),
    };
    call
}

fn footprint(call: &CallMemoryPlan, payload_bytes: u64) -> CurrentMemoryFootprint {
    CurrentMemoryFootprint {
        logical_elements: 1,
        fixed_bytes: call.target.primitives.string_header.bytes,
        payload_bytes,
        encoded_bytes: payload_bytes + 16,
        retained_nodes: 1,
        ..CurrentMemoryFootprint::default()
    }
}

#[test]
fn owned_turn_template_restores_limits_and_uses_each_current_payload() {
    let mut call = template();
    let original = call.clone();
    let published = [footprint(&call, 7)];
    for bytes in [0, 13, 5] {
        let resolved = BTreeMap::from([
            ((PortDirection::Input, 0), footprint(&call, bytes + 1)),
            ((PortDirection::Output, 0), footprint(&call, bytes)),
        ]);
        let result = derive_turn_call(&mut call, &resolved, Some(&published)).unwrap();
        assert_eq!(result.inputs[0].value.payload.current_bytes, bytes + 1);
        assert_eq!(result.outputs[0].value.payload.current_bytes, bytes);
        assert_eq!(result.target, original.target);
        assert_eq!(call, original);
    }
}

#[test]
fn owned_turn_template_restores_limits_after_rejections_and_retries() {
    let mut call = template();
    let original = call.clone();
    let published = [footprint(&call, 7)];
    let resolved = BTreeMap::from([
        ((PortDirection::Input, 0), footprint(&call, 11)),
        ((PortDirection::Output, 0), footprint(&call, 13)),
    ]);
    assert!(derive_turn_call(&mut call, &resolved, Some(&published)).is_ok());
    for (facts, outputs, expected) in [
        (
            BTreeMap::new(),
            &published[..],
            MemoryPlanError::MissingFootprintWitness {
                stage: MemoryWitnessStage::Turn,
            },
        ),
        (
            resolved.clone(),
            &[][..],
            MemoryPlanError::DescriptorArityMismatch,
        ),
    ] {
        assert_eq!(
            derive_turn_call(&mut call, &facts, Some(outputs)),
            Err(expected)
        );
        assert_eq!(call, original);
        let recovered = derive_turn_call(&mut call, &resolved, Some(&published)).unwrap();
        assert_eq!(recovered.outputs[0].value.payload.current_bytes, 13);
        assert_eq!(recovered.target, original.target);
        assert_eq!(call, original);
    }
}
