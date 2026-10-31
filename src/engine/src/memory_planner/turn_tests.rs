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

fn scratch_template() -> CallMemoryPlan {
    let original = template();
    let mut target = original.target.clone();
    target.limits = MemoryBudgetLimits::default();
    let mut call = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &original.bound_call,
        input_storage: &original.input_storage,
        output_storage: &original.output_storage,
        input_witnesses: &original.input_witnesses,
        output_witnesses: &original.output_witnesses,
        published_output_witnesses: &original.output_witnesses,
        implementation_memory: ImplementationMemoryClass::CanonicalFinalize,
        target: &target,
        regions: &original.output_regions,
    })
    .unwrap();
    call.target.limits = original.target.limits;
    assert!(
        call.allocations
            .iter()
            .any(|a| matches!(a.owner, mech_core::MemoryObjectOwner::NodeScratch { .. }))
    );
    call
}

// Preserve the exact former native adapter as an independent input-projection
// oracle. Both projections use the maintained general turn planner, not a
// duplicate output expectation or a detached substitute plan.
fn owned_single_call_projection(
    original: &CallMemoryPlan,
    node: mech_core::NodeId,
    facts: &super::TurnMemoryFacts,
) -> Result<super::TurnMemoryPlan, MemoryPlanError> {
    let mut call = original.clone();
    for allocation in &mut call.allocations {
        if let mech_core::MemoryObjectOwner::NodeScratch { node: owner, .. } = &mut allocation.owner
        {
            *owner = node;
        }
    }
    super::plan_turn_memory(
        &super::ProgramMemoryPlan {
            call_nodes: Box::new([node]),
            allocations: call.allocations.clone(),
            budget_limits: call.target.limits,
            calls: Box::new([call]),
            ..Default::default()
        },
        node,
        facts,
    )
}

#[test]
fn borrowed_single_call_projection_keeps_current_facts_scratch_and_finite_violations() {
    for node in [mech_core::NodeId(0), mech_core::NodeId(37)] {
        let call = scratch_template();
        let original = call.clone();
        for bytes in [0, 13, 511] {
            let facts = super::TurnMemoryFacts {
                resolved_footprints: BTreeMap::from([
                    ((node, PortDirection::Input, 0), footprint(&call, bytes + 1)),
                    ((node, PortDirection::Output, 0), footprint(&call, bytes)),
                    (
                        (mech_core::NodeId(99), PortDirection::Input, 0),
                        footprint(&call, 2000),
                    ),
                ]),
                published_footprints: BTreeMap::from([((node, 0), footprint(&call, 7))]),
                ..Default::default()
            };
            let owned = owned_single_call_projection(&call, node, &facts).unwrap();
            let borrowed = super::plan_single_call_turn_memory(&call, node, &facts).unwrap();
            assert_eq!(borrowed, owned);
            assert!(
                !borrowed.budget_violations.is_empty(),
                "zero finite limits stay effective"
            );
            assert_eq!(
                borrowed.call.as_ref().unwrap().inputs[0]
                    .value
                    .payload
                    .current_bytes,
                bytes + 1
            );
            assert_eq!(
                borrowed.call.as_ref().unwrap().outputs[0]
                    .value
                    .payload
                    .current_bytes,
                bytes
            );
            assert!(
                borrowed
                    .allocations
                    .iter()
                    .filter_map(|a| match a.owner {
                        mech_core::MemoryObjectOwner::NodeScratch { node, .. } => Some(node),
                        _ => None,
                    })
                    .all(|owner| owner == node)
            );
            assert_eq!(
                call, original,
                "borrowing cannot retag or replace the prepared call"
            );
        }
    }
}

#[test]
fn borrowed_single_call_projection_keeps_error_precedence_immutable_input_and_retry() {
    let node = mech_core::NodeId(37);
    let call = scratch_template();
    let original = call.clone();
    let valid = super::TurnMemoryFacts {
        resolved_footprints: BTreeMap::from([
            ((node, PortDirection::Input, 0), footprint(&call, 11)),
            ((node, PortDirection::Output, 0), footprint(&call, 13)),
        ]),
        published_footprints: BTreeMap::from([((node, 0), footprint(&call, 7))]),
        ..Default::default()
    };
    let mut missing_and_overflow = valid.clone();
    missing_and_overflow
        .resolved_footprints
        .remove(&(node, PortDirection::Input, 0));
    missing_and_overflow.additional_demand.turn_peak_bytes = u64::MAX;
    let mut overflow = valid.clone();
    overflow.additional_demand.turn_peak_bytes = u64::MAX;
    for (index, facts) in [missing_and_overflow, overflow].into_iter().enumerate() {
        let expected = owned_single_call_projection(&call, node, &facts);
        assert!(expected.is_err());
        if index == 0 {
            assert_eq!(
                expected,
                Err(MemoryPlanError::MissingFootprintWitness {
                    stage: MemoryWitnessStage::Turn
                })
            );
        }
        assert_eq!(
            super::plan_single_call_turn_memory(&call, node, &facts),
            expected
        );
        assert_eq!(call, original);
        let recovered = super::plan_single_call_turn_memory(&call, node, &valid).unwrap();
        assert_eq!(
            recovered,
            owned_single_call_projection(&call, node, &valid).unwrap()
        );
        assert_eq!(call, original);
    }
}

#[cfg(feature = "resident-artifact")]
fn observed_plan() -> (super::TurnMemoryPlan, CallMemoryPlan) {
    let node = mech_core::NodeId(37);
    let call = scratch_template();
    let facts = super::TurnMemoryFacts {
        resolved_footprints: BTreeMap::from([
            ((node, PortDirection::Input, 0), footprint(&call, 11)),
            ((node, PortDirection::Output, 0), footprint(&call, 2)),
        ]),
        published_footprints: BTreeMap::from([((node, 0), footprint(&call, 7))]),
        ..Default::default()
    };
    (
        super::plan_single_call_turn_memory(&call, node, &facts).unwrap(),
        call,
    )
}

#[cfg(feature = "resident-artifact")]
#[test]
fn observed_owned_call_refines_current_outputs_and_keeps_published_finite_facts() {
    let (plan, template) = observed_plan();
    let retained_template = template.clone();
    for bytes in [0, 13, 511] {
        let output = footprint(&template, bytes);
        let result = super::apply_observed_turn_demand(
            plan.clone(),
            mech_core::ResourceDemand {
                output_elements: 1,
                persistent_bytes: output.fixed_bytes + output.payload_bytes,
                retained_nodes: 1,
                ..Default::default()
            },
            Some(output),
        )
        .unwrap();
        let call = result.call.as_ref().unwrap();
        assert_eq!(call.inputs[0].value.payload.current_bytes, 11);
        assert_eq!(call.outputs[0].value.payload.current_bytes, bytes);
        assert_eq!(
            result.facts.published_footprints,
            plan.facts.published_footprints
        );
        assert_eq!(
            result.facts.resolved_footprints[&(plan.node, PortDirection::Input, 0)],
            plan.facts.resolved_footprints[&(plan.node, PortDirection::Input, 0)]
        );
        assert_eq!(result.node, plan.node);
        assert_eq!(result.budget_limits, plan.budget_limits);
        assert_eq!(call.target.limits, plan.budget_limits);
        assert!(
            !result.budget_violations.is_empty(),
            "finite zero limits remain effective"
        );
        assert_eq!(template, retained_template);
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn observed_owned_call_keeps_the_call_when_refinement_is_not_selected() {
    let (plan, _) = observed_plan();
    let original = plan.clone();
    let result = super::apply_observed_turn_demand(plan, Default::default(), None).unwrap();
    assert_eq!(result.call, original.call);
    assert_eq!(result.facts, original.facts);
    assert_eq!(result.transactions, original.transactions);
    assert_eq!(result.node, original.node);
    assert_eq!(result.budget_limits, original.budget_limits);

    let mut no_call = original;
    no_call.call = None;
    let result = super::apply_observed_turn_demand(no_call, Default::default(), None).unwrap();
    assert!(result.call.is_none());
}

#[cfg(feature = "resident-artifact")]
#[test]
fn observed_owned_call_preserves_overflow_before_resolution_and_allows_retry() {
    let (plan, template) = observed_plan();
    let retained_template = template.clone();
    let mut missing = plan.clone();
    // Initial derivation resolves the call's witnesses. Use the actual
    // deferred template to test missing-fact rejection during refinement.
    missing.call = Some(template.clone());
    missing
        .facts
        .resolved_footprints
        .remove(&(plan.node, PortDirection::Input, 0));
    let overflow = CurrentMemoryFootprint {
        fixed_bytes: u64::MAX,
        payload_bytes: 1,
        ..footprint(&template, 0)
    };
    assert_eq!(
        super::apply_observed_turn_demand(missing.clone(), Default::default(), Some(overflow)),
        Err(MemoryPlanError::TargetAddressOverflow)
    );
    assert_eq!(
        super::apply_observed_turn_demand(
            missing,
            Default::default(),
            Some(footprint(&template, 13))
        ),
        Err(MemoryPlanError::MissingFootprintWitness {
            stage: MemoryWitnessStage::Turn
        })
    );
    let recovered =
        super::apply_observed_turn_demand(plan, Default::default(), Some(footprint(&template, 13)))
            .unwrap();
    assert_eq!(
        recovered.call.unwrap().outputs[0]
            .value
            .payload
            .current_bytes,
        13
    );
    assert_eq!(template, retained_template);
}
