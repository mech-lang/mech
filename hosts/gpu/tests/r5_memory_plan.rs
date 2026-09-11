use std::collections::BTreeMap;

use mech_compute::{
    ComputeElementType, ComputeKernel, ComputePhysicalPlan, ComputePort, ComputePortId,
    ComputeProgram, ComputeRegionInterface, ElementwiseIr, ElementwiseStoragePlan,
};
use mech_core::{CellSlotId, GpuMemoryLimits, MemoryBudgetDimension, MemoryPlanError, SchemaId};
use mech_gpu::{
    ElementwiseKernel, GpuKernelPlanSource, PlannedGpuExecution, plan_scalar_instruction_expansion,
};

fn limits(max_buffer_size: u64) -> GpuMemoryLimits {
    GpuMemoryLimits {
        max_buffer_size,
        max_storage_buffer_binding_size: max_buffer_size,
        max_storage_buffers_per_shader_stage: 8,
        max_bindings_per_bind_group: 8,
        max_compute_workgroups_per_dimension: 65_535,
        max_compute_invocations_per_workgroup: 256,
        max_compute_workgroup_size_x: 256,
        min_storage_buffer_offset_alignment: 4,
    }
}

fn planned_execution(
    elements: u64,
    limits: GpuMemoryLimits,
) -> Result<PlannedGpuExecution, mech_gpu::GpuMemoryPlanError> {
    let slot = CellSlotId::new(1);
    let program = ComputeProgram::new(
        ComputeRegionInterface {
            inputs: vec![ComputePort {
                id: ComputePortId::new(0),
                name: "input".into(),
                slot,
                schema: SchemaId::new(0),
                element: ComputeElementType::F32,
                dimensions: vec![elements].into_boxed_slice(),
            }]
            .into_boxed_slice(),
            ..ComputeRegionInterface::default()
        },
        ComputePhysicalPlan::default(),
        ComputeKernel::Elementwise(ElementwiseIr::default()),
    )
    .with_elementwise_storage(ElementwiseStoragePlan {
        slot_elements: BTreeMap::from([(slot, elements)]),
        dispatch_elements: elements,
        ..ElementwiseStoragePlan::default()
    });
    let kernel = ElementwiseKernel::from_compute_program(&program)
        .expect("the backend-neutral fixture must lower through the GPU authority");
    PlannedGpuExecution::build(
        GpuKernelPlanSource::Elementwise(&kernel),
        &BTreeMap::from([("input".to_owned(), vec![0.0; elements as usize])]),
        limits,
    )
}

#[test]
fn adapter_limits_are_consumed_before_gpu_binding_creation() {
    let plan = planned_execution(2, limits(8)).unwrap();
    assert_eq!(plan.binding_bytes(0), Some(8));
    assert!(plan.assert_binding_bytes(0, 8).is_ok());

    assert!(matches!(
        planned_execution(2, limits(7)),
        Err(mech_gpu::GpuMemoryPlanError::Plan(
            MemoryPlanError::TargetLimitExceeded { .. }
        ))
    ));
}

#[test]
fn gpu_plan_identity_and_arena_placement_are_deterministic() {
    let first = planned_execution(4, limits(1024)).unwrap();
    let second = planned_execution(4, limits(1024)).unwrap();
    assert_eq!(first.memory, second.memory);
    assert_eq!(
        first.memory.allocations[0].id,
        mech_core::MemoryObjectId::new(0)
    );
    assert_eq!(first.memory.allocations[0].capacity_bytes, 16);
}

#[test]
fn scalar_instruction_expansion_checks_exact_limit_one_over_and_overflow() {
    let exact = plan_scalar_instruction_expansion(16_777_210, 2, 3, 1).unwrap();
    assert_eq!(exact.additional, 6);
    assert_eq!(exact.total, 16_777_216);

    assert!(matches!(
        plan_scalar_instruction_expansion(16_777_211, 2, 3, 1),
        Err(MemoryPlanError::TargetLimitExceeded { violation })
            if violation.dimension == MemoryBudgetDimension::ScalarInstructions
    ));
    assert!(matches!(
        plan_scalar_instruction_expansion(0, usize::MAX, usize::MAX, usize::MAX),
        Err(MemoryPlanError::ArithmeticOverflow { .. })
    ));
}
