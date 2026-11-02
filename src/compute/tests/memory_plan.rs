use std::collections::BTreeMap;

use mech_compute::{
    ComputeKernel, ComputePhysicalPlan, ComputeProgram, ComputeRegionInterface,
    PlannedComputeArtifact, instantiate_compute_memory,
};
use mech_core::{
    CellSlotId, CurrentMemoryFootprint, GpuMemoryLimits, MemoryFootprintWitness, MemoryLifetime,
    MemoryPlanError, TargetMemoryProfile, ValueCell, physical_storage_descriptor,
};
use mech_engine::memory_planner::{
    ActivationMemoryFacts, ActivationValueFact, PlannedValueClass, ProgramMemoryPlanTemplate,
    ValueMemoryPlanTemplate,
};

fn fixture() -> (
    PlannedComputeArtifact,
    ActivationMemoryFacts,
    TargetMemoryProfile,
) {
    let slot = CellSlotId::new(0);
    let cell = ValueCell::from_exact(1.0_f32).unwrap();
    let descriptor = cell.resolved_descriptor().unwrap();
    let target = TargetMemoryProfile::current_native_host().unwrap();
    let storage =
        physical_storage_descriptor(cell.representation(), &target, MemoryLifetime::Activation);
    let artifact = PlannedComputeArtifact {
        placement: ComputePhysicalPlan::default(),
        memory: ProgramMemoryPlanTemplate {
            values: vec![ValueMemoryPlanTemplate {
                slot,
                descriptor: Some(descriptor.clone()),
                class: PlannedValueClass::Input,
                producer: None,
                last_consumer: None,
                alias_source: None,
            }]
            .into_boxed_slice(),
            ..ProgramMemoryPlanTemplate::default()
        },
    };
    let facts = ActivationMemoryFacts {
        values: BTreeMap::from([(
            slot,
            ActivationValueFact {
                descriptor,
                storage,
                witness: MemoryFootprintWitness::Known(CurrentMemoryFootprint {
                    logical_elements: 1,
                    fixed_bytes: 4,
                    ..CurrentMemoryFootprint::default()
                }),
            },
        )]),
        classes: BTreeMap::new(),
    };
    (artifact, facts, target)
}

#[test]
fn compute_activation_plan_is_deterministic_and_target_local() {
    let (artifact, facts, target) = fixture();
    let first = instantiate_compute_memory(&artifact, &target, &facts).unwrap();
    let second = instantiate_compute_memory(&artifact, &target, &facts).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.values.len(), 1);
    assert_eq!(first.values[0].layout.slot, target.primitives.f32_slot);
    assert_eq!(first.allocations[0].capacity_bytes, 4);
}

#[test]
fn compute_activation_cannot_default_a_missing_footprint() {
    let (artifact, _, target) = fixture();
    assert_eq!(
        instantiate_compute_memory(&artifact, &target, &ActivationMemoryFacts::default()),
        Err(MemoryPlanError::MissingFootprintWitness {
            stage: mech_core::MemoryWitnessStage::Activation
        })
    );
}

#[test]
fn mixed_compute_uses_each_memory_spaces_own_target_layout() {
    let host_slot = CellSlotId::new(0);
    let device_slot = CellSlotId::new(1);
    let cell = ValueCell::from_exact(1.0_f32).unwrap();
    let descriptor = cell.resolved_descriptor().unwrap();
    let host_target = TargetMemoryProfile::current_native_host().unwrap();
    // Use a deliberately wider but valid device ABI slot so this test proves
    // that Host and Device values are projected by different authorities.
    let mut device_target = TargetMemoryProfile::gpu(GpuMemoryLimits {
        max_buffer_size: 1 << 20,
        max_storage_buffer_binding_size: 1 << 20,
        max_storage_buffers_per_shader_stage: 8,
        max_bindings_per_bind_group: 8,
        max_compute_workgroups_per_dimension: 65_535,
        max_compute_invocations_per_workgroup: 256,
        max_compute_workgroup_size_x: 256,
        min_storage_buffer_offset_alignment: 4,
    })
    .unwrap();
    device_target.primitives.f32_slot.bytes = 8;
    device_target.primitives.f32_slot.alignment = 8;
    let values = [host_slot, device_slot]
        .into_iter()
        .map(|slot| ValueMemoryPlanTemplate {
            slot,
            descriptor: Some(descriptor.clone()),
            class: PlannedValueClass::Input,
            producer: None,
            last_consumer: None,
            alias_source: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let artifact = PlannedComputeArtifact {
        placement: ComputePhysicalPlan::default(),
        memory: ProgramMemoryPlanTemplate {
            values,
            ..ProgramMemoryPlanTemplate::default()
        },
    };
    let facts = ActivationMemoryFacts {
        values: BTreeMap::from([
            (
                host_slot,
                ActivationValueFact {
                    descriptor: descriptor.clone(),
                    storage: physical_storage_descriptor(
                        cell.representation(),
                        &host_target,
                        MemoryLifetime::Activation,
                    ),
                    witness: MemoryFootprintWitness::Known(CurrentMemoryFootprint {
                        logical_elements: 1,
                        fixed_bytes: 4,
                        ..CurrentMemoryFootprint::default()
                    }),
                },
            ),
            (
                device_slot,
                ActivationValueFact {
                    descriptor,
                    storage: physical_storage_descriptor(
                        cell.representation(),
                        &device_target,
                        MemoryLifetime::Activation,
                    ),
                    witness: MemoryFootprintWitness::Known(CurrentMemoryFootprint {
                        logical_elements: 1,
                        fixed_bytes: 8,
                        ..CurrentMemoryFootprint::default()
                    }),
                },
            ),
        ]),
        classes: BTreeMap::new(),
    };
    let plan = instantiate_compute_memory(&artifact, &device_target, &facts).unwrap();
    assert_eq!(plan.values[0].layout.slot, host_target.primitives.f32_slot);
    assert_eq!(
        plan.values[1].layout.slot,
        device_target.primitives.f32_slot
    );
    assert_eq!(plan.allocations[0].capacity_bytes, 4);
    assert_eq!(plan.allocations[1].capacity_bytes, 8);
}

#[test]
fn backend_neutral_compute_program_remains_separate_from_target_memory_authority() {
    let program = ComputeProgram::new(
        ComputeRegionInterface::default(),
        ComputePhysicalPlan::default(),
        ComputeKernel::Elementwise(Default::default()),
    );
    assert!(program.interface().inputs.is_empty());
    assert!(program.plan().slots.is_empty());
}
