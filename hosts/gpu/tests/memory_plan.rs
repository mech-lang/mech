use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use mech_compute::{
    ComputeElementType, ComputeKernel, ComputePhysicalPlan, ComputePort, ComputePortId,
    ComputeProgram, ComputeRegionInterface, ElementwiseIr, ElementwiseStoragePlan,
};
use mech_core::{CellSlotId, GpuMemoryLimits, MemoryBudgetDimension, MemoryPlanError, SchemaId};
use mech_gpu::{
    ComputeLowerer, ElementwiseKernel, FixedShapeKernel, GpuExecutionBindingRole, GpuExecutionPlan,
    GpuKernelPlanSource, PlannedGpuExecution, plan_scalar_instruction_expansion,
};
use mech_runtime::RuntimeBuilder;

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

#[test]
fn served_ekf_shader_bindings_fit_ten_without_reducing_physical_storage() {
    let full_source = include_str!("../../../examples/ekf/localization.mec");
    let region = full_source
        .split_once("5. ekf-batch @compute\n")
        .unwrap()
        .1
        .split_once("6. Live Tracking Field\n")
        .unwrap()
        .0;
    // Preserve the served example's actual kernel and retained-output contract,
    // while omitting the unrelated scene and timer coordinator from this plan test.
    let source = format!(
        "+> math/*\n\n@filters := compute://filters/kernel {{:read(sample/result.0), :read(sample/result.2), :write(input/control), :write(input/camera), :write(input/measurement), :write(turn)}}\n@filters/input/control <- [0.05<f32>; 1f32; 0f32]\n@filters/input/camera <- [1f32; 1f32]\n@filters/input/measurement <- [1f32; 0f32; 0f32]\n@filters/turn <- 1\n\n5. ekf-batch @compute\n{region}"
    );
    let document = mech_runtime::SourceDocument::parse_resolved(
        "test://served-ekf-bindings",
        mech_syntax::document::Revision(0),
        Arc::<str>::from(source),
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap();
    let mixed = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_mixed_document(&document)
        .unwrap();
    let inputs = BTreeMap::from([
        ("control".to_owned(), [0.05, 1.0, 0.0].repeat(1000)),
        ("camera".to_owned(), [1.0, 1.0].repeat(1000)),
        ("measurement".to_owned(), [1.0, 0.0, 0.0].repeat(1000)),
    ]);
    let kernel = ComputeLowerer
        .compile_broadcast(&mixed.compute.artifact, &inputs)
        .unwrap();
    let execution =
        GpuExecutionPlan::build(GpuKernelPlanSource::FixedShape(&kernel), &inputs).unwrap();
    assert_eq!(execution.dispatch_elements, 1000);
    assert_eq!(execution.bindings.len(), 10);
    assert_eq!(execution.wgsl.matches("@binding(").count(), 10);
    assert_eq!(
        execution
            .bindings
            .iter()
            .map(|binding| binding.binding)
            .collect::<Vec<_>>(),
        (0..10).collect::<Vec<_>>(),
    );
    assert_eq!(execution.states.len(), 4);
    assert_eq!(
        execution
            .states
            .iter()
            .filter(|state| state.recurrence)
            .count(),
        2
    );
    assert_eq!(execution.physical_outputs.len(), 2);
    for state in execution.states.iter().filter(|state| !state.recurrence) {
        assert!(
            !execution
                .wgsl
                .contains(&format!("state_read_{}", state.slot))
        );
        assert!(
            execution
                .wgsl
                .contains(&format!("state_write_{}", state.slot))
        );
    }
    for binding in &execution.bindings {
        eprintln!(
            "EKF binding={} name={} role={:?} access={:?} slot={} reads={} writes={}",
            binding.binding,
            binding.name,
            binding.role,
            binding.access,
            binding.slot,
            matches!(
                binding.role,
                GpuExecutionBindingRole::Input
                    | GpuExecutionBindingRole::StateRead
                    | GpuExecutionBindingRole::IntegrityFault
            ),
            matches!(
                binding.role,
                GpuExecutionBindingRole::StateWrite | GpuExecutionBindingRole::IntegrityFault
            ),
        );
    }
    let limits = GpuMemoryLimits {
        max_buffer_size: 1 << 20,
        max_storage_buffer_binding_size: 1 << 20,
        max_storage_buffers_per_shader_stage: 10,
        max_bindings_per_bind_group: 10,
        max_compute_workgroups_per_dimension: 65535,
        max_compute_invocations_per_workgroup: 256,
        max_compute_workgroup_size_x: 256,
        min_storage_buffer_offset_alignment: 256,
    };
    let planned = PlannedGpuExecution::from_execution(execution.clone(), limits).unwrap();
    assert_eq!(planned.memory.demand.storage_bindings, 10);
    let mut state_objects = BTreeSet::new();
    for state in &execution.states {
        let [current, next] = planned
            .state_objects(mech_core::CellSlotId::new(state.slot))
            .unwrap();
        assert_ne!(current, next);
        assert!(state_objects.insert(current));
        assert!(state_objects.insert(next));
    }
    assert_eq!(
        state_objects.len(),
        8,
        "all recurrence/publication double buffers must remain allocated"
    );
    assert_eq!(planned.writable_state_objects(0).unwrap().len(), 4);
    assert_eq!(planned.writable_state_objects(1).unwrap().len(), 4);
    let mut insufficient_limits = limits;
    insufficient_limits.max_storage_buffers_per_shader_stage = 9;
    assert!(PlannedGpuExecution::from_execution(execution.clone(), insufficient_limits).is_err());
    let decoded: GpuExecutionPlan =
        serde_json::from_str(&serde_json::to_string(&execution).unwrap()).unwrap();
    assert_eq!(decoded, execution);
    decoded.validate().unwrap();
    let restored = FixedShapeKernel::from_compute_program(kernel.compute_program()).unwrap();
    assert_eq!(
        GpuExecutionPlan::build(GpuKernelPlanSource::FixedShape(&restored), &inputs).unwrap(),
        execution,
    );
}
