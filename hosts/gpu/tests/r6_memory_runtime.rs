use std::collections::BTreeMap;

use mech_compute::{
    ComputeElementType, ComputeKernel, ComputePhysicalPlan, ComputePort, ComputePortId,
    ComputeProgram, ComputeRegionInterface, ElementwiseIr, ElementwiseStoragePlan,
};
use mech_core::{CellSlotId, GpuMemoryLimits, MemoryRuntimeError, OwnedAllocationState, SchemaId};
use mech_gpu::{ElementwiseKernel, GpuKernelPlanSource, GpuMemoryPlanError, PlannedGpuExecution};

fn limits() -> GpuMemoryLimits {
    GpuMemoryLimits {
        max_buffer_size: 1024,
        max_storage_buffer_binding_size: 1024,
        max_storage_buffers_per_shader_stage: 8,
        max_bindings_per_bind_group: 8,
        max_compute_workgroups_per_dimension: 65_535,
        max_compute_invocations_per_workgroup: 256,
        max_compute_workgroup_size_x: 256,
        min_storage_buffer_offset_alignment: 4,
    }
}

fn planned_execution(elements: u64) -> PlannedGpuExecution {
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
    let kernel = ElementwiseKernel::from_compute_program(&program).unwrap();
    PlannedGpuExecution::build(
        GpuKernelPlanSource::Elementwise(&kernel),
        &BTreeMap::from([("input".to_owned(), vec![0.0; elements as usize])]),
        limits(),
    )
    .unwrap()
}

#[test]
fn device_buffers_require_exact_registration_before_submission() {
    let plan = planned_execution(4);
    let object = plan.binding_object(0).unwrap();
    let mut memory = plan.managed_memory().unwrap();
    assert_eq!(memory.allocations().len(), 1);
    assert_eq!(memory.allocations()[0].actual_block_bytes, 0);
    assert!(matches!(
        memory.begin_submission(&[object], &[]),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::UnplannedAllocation { .. }
        ))
    ));
    assert!(matches!(
        memory.attach_device_allocation(object, 15, 0),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::CapacityExceeded { .. }
        ))
    ));

    memory.attach_device_allocation(object, 16, 16).unwrap();
    assert_eq!(memory.content_version(object), Some(1));
    let observation = memory.allocations()[0];
    assert_eq!(observation.actual_block_bytes, 16);
    assert_eq!(observation.device_owner_pins, 1);
    assert_eq!(observation.state, OwnedAllocationState::Live);
    assert!(matches!(
        memory.attach_device_allocation(object, 16, 16),
        Err(GpuMemoryPlanError::DuplicateDeviceAllocation { .. })
    ));

    let hold = memory.begin_submission(&[object], &[]).unwrap();
    assert_eq!(memory.ledger().in_flight_device_bytes, 16);
    assert_eq!(memory.allocations()[0].submission_pins, 1);
    hold.complete().unwrap();
    assert_eq!(memory.ledger().in_flight_device_bytes, 0);
    assert_eq!(memory.allocations()[0].submission_pins, 0);
    assert_eq!(memory.record_device_write(object, 16).unwrap(), 2);
    assert_eq!(memory.content_version(object), Some(2));
}

#[test]
fn device_loss_is_a_fail_closed_submission_boundary() {
    let plan = planned_execution(1);
    let object = plan.binding_object(0).unwrap();
    let mut memory = plan.managed_memory().unwrap();
    memory.attach_device_allocation(object, 4, 4).unwrap();
    memory.mark_device_lost().unwrap();
    assert!(matches!(
        memory.begin_submission(&[object], &[]),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::DeviceLost { .. }
        ))
    ));
    assert!(matches!(
        memory.record_device_write(object, 4),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::DeviceLost { .. }
        ))
    ));
}
