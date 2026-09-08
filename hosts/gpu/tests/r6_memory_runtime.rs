use std::collections::BTreeMap;

use mech_compute::{
    ComputeElementType, ComputeKernel, ComputePhysicalPlan, ComputePort, ComputePortId,
    ComputeProgram, ComputeRegionInterface, ElementwiseIr, ElementwiseStoragePlan,
};
use mech_core::{CellSlotId, GpuMemoryLimits, MemoryRuntimeError, SchemaId};
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
fn unregistered_device_buffers_cannot_enter_submission() {
    let plan = planned_execution(4);
    let object = plan.binding_object(0).unwrap();
    let memory = plan.managed_memory().unwrap();
    assert_eq!(memory.allocations().len(), 1);
    assert_eq!(memory.allocations()[0].actual_block_bytes, 0);
    assert!(matches!(
        memory.begin_submission(&[object], &[]),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::UnplannedAllocation { .. }
        ))
    ));
}

#[cfg(feature = "native")]
#[test]
fn registered_device_buffer_couples_storage_loss_and_accounting_lifetimes() {
    let instance = wgpu::Instance::default();
    let Some(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
    else {
        return;
    };
    let Ok((device, _queue)) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Mech registered buffer ownership test"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    )) else {
        return;
    };
    let plan = planned_execution(4);
    let object = plan.binding_object(0).unwrap();
    let mut memory = plan.managed_memory().unwrap();
    let undersized = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Mech undersized registered buffer ownership test"),
        size: 12,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    assert!(matches!(
        memory.register_device_buffer(object, undersized, 0),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::CapacityExceeded { .. }
        ))
    ));
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Mech registered buffer ownership test"),
        size: 16,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let registered = memory.register_device_buffer(object, buffer, 0).unwrap();
    let observation = memory.allocations()[0];
    assert_eq!(registered.object(), object);
    assert_eq!(registered.allocation_handle(), observation.handle);
    assert_eq!(registered.buffer().size(), 16);
    assert_eq!(observation.actual_block_bytes, 16);
    assert_eq!(observation.device_owner_pins, 1);

    memory.mark_device_lost().unwrap();
    assert!(matches!(
        memory.begin_submission(&[object], &[]),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::DeviceLost { .. }
        ))
    ));
    assert!(matches!(
        memory.record_device_write(object, 16),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::DeviceLost { .. }
        ))
    ));

    drop(registered);
    let observation = memory.allocations()[0];
    assert_eq!(observation.actual_block_bytes, 0);
    assert_eq!(observation.device_owner_pins, 0);
    assert!(matches!(
        memory.begin_submission(&[object], &[]),
        Err(GpuMemoryPlanError::Runtime(
            MemoryRuntimeError::DeviceLost { .. }
        ))
    ));
}
