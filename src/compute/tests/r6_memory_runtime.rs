use std::collections::BTreeMap;

use mech_compute::{ComputePhysicalPlan, PlannedComputeArtifact, instantiate_compute_memory};
use mech_core::{
    CellSlotId, CurrentMemoryFootprint, MemoryFootprintWitness, MemoryLifetime,
    TargetMemoryProfile, ValueCell, physical_storage_descriptor,
};
use mech_engine::memory_planner::{
    ActivationMemoryFacts, ActivationValueFact, PlannedValueClass, ProgramMemoryPlanTemplate,
    ValueMemoryPlanTemplate,
};
use mech_engine::memory_runtime::ManagedProgramMemory;

#[test]
fn compute_activation_realizes_its_subordinate_r5_storage() {
    let slot = CellSlotId::new(0);
    let value = ValueCell::from_exact(3.0_f32).unwrap();
    let descriptor = value.resolved_descriptor().unwrap();
    let target = TargetMemoryProfile::current_native_host().unwrap();
    let storage =
        physical_storage_descriptor(value.representation(), &target, MemoryLifetime::Activation);
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
                    retained_nodes: 1,
                    ..CurrentMemoryFootprint::default()
                }),
            },
        )]),
        classes: BTreeMap::new(),
    };
    let plan = instantiate_compute_memory(&artifact, &target, &facts).unwrap();
    let memory = ManagedProgramMemory::realize(&plan).unwrap();
    assert_eq!(memory.realized().bindings().len(), plan.allocations.len());
    let planned_arena_bytes = plan
        .arenas
        .iter()
        .map(|arena| arena.capacity_bytes)
        .sum::<u64>();
    let initialization_metadata_bytes = u64::try_from(plan.allocations.len())
        .unwrap()
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .unwrap();
    assert_eq!(
        memory.domain().ledger().committed_bytes,
        planned_arena_bytes + initialization_metadata_bytes,
        "the runtime ledger must own the realized arenas and their preplanned initialization maps",
    );
    memory.close().unwrap();
}
