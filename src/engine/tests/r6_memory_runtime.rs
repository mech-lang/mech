#![cfg(feature = "full_compiler")]

use mech_core::{
    AccessMode, AliasPolicy, AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement,
    ArenaPlan, BoundCall, CallMemoryPlanningRequest, CellSlotId, ChangeDetectionPolicy,
    CurrentMemoryFootprint, DeliveryMode, ExecutionTarget, ExternalInteraction,
    ImplementationMemoryClass, InputPortLayout, InputPortPolicy, MemoryArenaId, MemoryBudgetLimits,
    MemoryFootprintWitness, MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemorySpace, NodeId,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, Ref, RegionAccessPlan,
    ResolvedOperationDescriptor, ResourceDemand, RuntimeFunctionId, ShapeRule, TargetMemoryProfile,
    ValueCell, physical_storage_descriptor, plan_call_memory,
};
use mech_engine::ArtifactSource;
use mech_engine::memory_planner::{
    ActivationMemoryFacts, ActivationValueFact, CallSiteMemoryTemplate, PlannedValueClass,
    ProgramMemoryPlan, ProgramMemoryPlanTemplate, ValueMemoryPlanTemplate,
    instantiate_program_memory_plan,
};
use mech_engine::memory_runtime::{ManagedProgramMemory, realize_resident_memory};
use nalgebra::DMatrix;

fn plan() -> ProgramMemoryPlan {
    ProgramMemoryPlan {
        values: Box::new([]),
        call_nodes: Box::new([]),
        calls: Box::new([]),
        allocations: vec![AllocationPlan {
            id: MemoryObjectId::new(0),
            owner: MemoryObjectOwner::NodeScratch {
                node: NodeId::new(0),
                ordinal: 0,
            },
            role: AllocationRole::Scratch,
            slot: None,
            space: MemorySpace::ResidentCpu,
            current_bytes: 16,
            capacity_bytes: 16,
            payload_block_capacity: 0,
            alignment: 8,
            lifetime: MemoryLifetime::Activation,
            placement: ArenaPlacement {
                arena: MemoryArenaId::new(0),
                offset: 0,
            },
            reuse_group: None,
        }]
        .into_boxed_slice(),
        arenas: vec![ArenaPlan {
            id: MemoryArenaId::new(0),
            space: MemorySpace::ResidentCpu,
            backing: ArenaBackingKind::ContiguousBytes,
            capacity_bytes: 16,
            alignment: 8,
            members: vec![MemoryObjectId::new(0)].into_boxed_slice(),
        }]
        .into_boxed_slice(),
        transfers: Box::new([]),
        budget_limits: MemoryBudgetLimits::default(),
        peak: ResourceDemand::default(),
        budget_violations: Box::new([]),
    }
}

fn assert_realized(memory: &ManagedProgramMemory) {
    assert_eq!(memory.realized().bindings().len(), 1);
    // The live ledger includes the 16-byte arena plus its preallocated
    // eight-byte initialization map. Initialization authority is owned and
    // reclaimed with the arena; it is not free side metadata.
    assert_eq!(memory.domain().ledger().committed_bytes, 24);
    let observations = memory.domain().allocation_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].actual_block_bytes, 16);
}

#[test]
fn direct_and_resident_adapters_share_the_r5_realization_boundary() {
    let direct = ManagedProgramMemory::realize(&plan()).unwrap();
    assert_realized(&direct);
    direct.close().unwrap();

    let resident = realize_resident_memory(&plan()).unwrap();
    assert_realized(&resident);
    resident.close().unwrap();
}

#[test]
fn repeated_activation_and_close_reclaims_all_instance_storage() {
    for _ in 0..32 {
        let memory = ManagedProgramMemory::realize(&plan()).unwrap();
        assert_eq!(memory.domain().ledger().committed_bytes, 24);
        memory.close().unwrap();
    }
}

fn solve_program(call_count: u32) -> ProgramMemoryPlan {
    let mut target = TargetMemoryProfile::current_resident_cpu().unwrap();
    target.limits.max_output_bytes = Some(2_000);
    let dimension = 205;
    let coefficients = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::identity(dimension, dimension)),
        dimension,
        dimension,
    )
    .unwrap();
    let rhs = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::from_element(dimension, 1, 1.0)),
        dimension,
        1,
    )
    .unwrap();
    let descriptors = [
        coefficients.resolved_descriptor().unwrap(),
        rhs.resolved_descriptor().unwrap(),
    ];
    let lifetime = MemoryLifetime::Turn {
        first: mech_core::MemoryPlanPoint::new(0),
        last: mech_core::MemoryPlanPoint::new(1),
    };
    let storages = [
        physical_storage_descriptor(coefficients.representation(), &target, lifetime),
        physical_storage_descriptor(rhs.representation(), &target, lifetime),
    ];
    let witnesses = [
        MemoryFootprintWitness::Known(CurrentMemoryFootprint {
            logical_elements: u64::try_from(dimension * dimension).unwrap(),
            ..CurrentMemoryFootprint::default()
        }),
        MemoryFootprintWitness::Known(CurrentMemoryFootprint {
            logical_elements: u64::try_from(dimension).unwrap(),
            ..CurrentMemoryFootprint::default()
        }),
    ];
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r6-program-budget-scope",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    };
                    2
                ]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::AlwaysChanged,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let bound = BoundCall::syntax_directed(
        operation,
        descriptors.to_vec().into_boxed_slice(),
        vec![descriptors[1].clone()].into_boxed_slice(),
        RuntimeFunctionId::from_name("R6ProgramBudgetScope"),
        ExecutionTarget::ResidentCpu,
    )
    .unwrap();
    let call = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &bound,
        input_storage: &storages,
        output_storage: &[storages[1].clone()],
        input_witnesses: &witnesses,
        output_witnesses: &[witnesses[1]],
        published_output_witnesses: &[witnesses[1]],
        implementation_memory: ImplementationMemoryClass::MatrixSolve,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    assert!(call.demand.work.compute < target.limits.max_compute_work.unwrap());
    assert!(
        call.outputs[0].value.current_address_span_bytes < target.limits.max_output_bytes.unwrap()
    );

    let mut template = ProgramMemoryPlanTemplate::default();
    let mut facts = ActivationMemoryFacts::default();
    for raw in 0..call_count {
        let node = NodeId::new(raw);
        let slot = CellSlotId::new(raw);
        template.node_positions.insert(node, raw);
        template.values = template
            .values
            .iter()
            .cloned()
            .chain([ValueMemoryPlanTemplate {
                slot,
                descriptor: Some(descriptors[1].clone()),
                class: PlannedValueClass::PublishedOutput,
                producer: Some(node),
                last_consumer: None,
                alias_source: None,
            }])
            .collect();
        template.call_nodes = template.call_nodes.iter().copied().chain([node]).collect();
        template.call_sites = template
            .call_sites
            .iter()
            .cloned()
            .chain([CallSiteMemoryTemplate {
                node,
                input_sources: vec![
                    ArtifactSource::Constant(mech_core::ConstantId::new(0)),
                    ArtifactSource::Constant(mech_core::ConstantId::new(1)),
                ]
                .into_boxed_slice(),
                output_slots: vec![slot].into_boxed_slice(),
            }])
            .collect();
        template.calls = template
            .calls
            .iter()
            .cloned()
            .chain([call.clone()])
            .collect();
        facts.values.insert(
            slot,
            ActivationValueFact {
                descriptor: descriptors[1].clone(),
                storage: storages[1].clone(),
                witness: witnesses[1],
            },
        );
    }
    instantiate_program_memory_plan(&template, &target, &facts).unwrap()
}

#[test]
fn realization_preserves_per_call_work_and_output_budget_scopes() {
    let single = solve_program(1);
    assert!(
        single.budget_violations.is_empty(),
        "single call exceeded a supposedly per-call limit: {:?}",
        single.budget_violations
    );
    ManagedProgramMemory::realize(&single)
        .unwrap()
        .close()
        .unwrap();

    let multiple = solve_program(2);
    assert!(
        multiple.budget_violations.is_empty(),
        "program planner changed a per-call limit into a program quota: {:?}",
        multiple.budget_violations
    );
    assert!(multiple.peak.work.compute > multiple.budget_limits.max_compute_work.unwrap());
    let output_bytes = multiple
        .values
        .iter()
        .filter(|value| value.class == PlannedValueClass::PublishedOutput)
        .map(|value| value.layout.current_address_span_bytes + value.layout.payload.current_bytes)
        .sum::<u64>();
    assert!(output_bytes > multiple.budget_limits.max_output_bytes.unwrap());
    ManagedProgramMemory::realize(&multiple)
        .unwrap()
        .close()
        .unwrap();
}

#[test]
fn ordinary_source_literals_enter_one_interpreter_memory_session() {
    use mech_core::{FunctionCatalogBuilder, NoMechExecutionServices};
    use mech_engine::{CompilerPlanningConfig, CompilerPlanningProgram};
    use std::sync::Arc;

    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut catalog).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut catalog).unwrap();
    mech_engine::install_intrinsic_source(&mut catalog).unwrap();
    let mut program = CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        Arc::new(catalog.build().unwrap()),
    );
    let tree = mech_syntax::parser::parse(
        "number := 1.0\ntext := \"managed\"\nmatrix := [1.0 2.0; 3.0 4.0]\nnumber",
    )
    .unwrap();
    let mut services = NoMechExecutionServices;
    program
        .plan_tree_with_services(&tree, &mut services)
        .unwrap();

    let values = program
        .compiler_root_symbol_cells(&["number", "text", "matrix"])
        .unwrap();
    let owner = values[0].1.memory_domain().unwrap().id();
    assert!(
        values
            .iter()
            .all(|(_, value)| value.memory_domain().unwrap().id() == owner),
        "source cells escaped the interpreter session: {:?}",
        values
            .iter()
            .map(|(name, value)| (name, value.memory_domain().unwrap().id()))
            .collect::<Vec<_>>()
    );
}

mod managed_index_conversion {
    use mech_core::*;

    fn bind(source: ValueCell, output: ValueCell) -> SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let operation = OperationId::from_name("access/index");
        let runtime = RuntimeFunctionId::from_name("access/index");
        let entry = catalog.runtime_entry(runtime).unwrap();
        let contract = entry.operation_contract(operation).unwrap().clone();
        let parts = entry
            .bind_resolved_invocation(
                operation,
                ExecutionTarget::DirectRuntime,
                FunctionInvocation::unary(output, source),
            )
            .unwrap();
        SpecializedFunction::syntax_directed(
            parts,
            ResolvedOperationDescriptor::from_name("access/index", contract).unwrap(),
            runtime,
            ExecutionTarget::DirectRuntime,
            entry.implementation_memory_class(),
        )
        .unwrap()
    }

    fn indices(output: &ValueCell) -> Vec<u64> {
        let value = output.snapshot().unwrap();
        match value.data() {
            ValueData::Index(value) => vec![*value],
            ValueData::Matrix(matrix) => {
                let snapshot::SequenceView::Index(values) = matrix.elements() else {
                    panic!("expected Index matrix")
                };
                values.to_vec()
            }
            _ => panic!("expected Index value"),
        }
    }

    macro_rules! selector_family {
        ($name:ident, $type:ty, $first:expr, $second:expr) => {
            #[test]
            fn $name() {
                let session = MemoryDomain::new().unwrap();
                let source = ValueCell::from_exact_in(&session, $first as $type).unwrap();
                let output = ValueCell::from_exact_in(&session, 1_usize).unwrap();
                let function = bind(source.clone(), output.clone());
                function.instance().solve_result().unwrap();
                assert_eq!(indices(&output), vec![1]);
                source
                    .replace(
                        &ValueCell::from_exact($second as $type)
                            .unwrap()
                            .snapshot()
                            .unwrap(),
                    )
                    .unwrap();
                function.instance().solve_result().unwrap();
                assert_eq!(indices(&output), vec![2]);

                let source = ValueCell::from_exact_in(
                    &session,
                    nalgebra::DMatrix::from_row_slice(
                        2,
                        3,
                        &[
                            1 as $type, 6 as $type, 2 as $type, 5 as $type, 3 as $type, 4 as $type,
                        ],
                    ),
                )
                .unwrap();
                let output = ValueCell::from_exact_in(
                    &session,
                    nalgebra::DMatrix::from_element(6, 1, 1_usize),
                )
                .unwrap();
                let function = bind(source.clone(), output.clone());
                let output_alias = output.clone();
                function.instance().solve_result().unwrap();
                assert_eq!(indices(&output_alias), vec![1, 6, 2, 5, 3, 4]);
                for position in [0, 2, 5] {
                    let mut values = [7 as $type; 6];
                    values[position] = 0 as $type;
                    let version = output.published_version();
                    let replacement =
                        ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(2, 3, &values));
                    if std::any::TypeId::of::<$type>() == std::any::TypeId::of::<usize>() {
                        // Index itself has no zero value. Canonical ingress must reject it
                        // before a kernel can observe the invalid logical selector.
                        assert_eq!(
                            replacement.unwrap_err().kind_name(),
                            "ValueCellSnapshotFailure"
                        );
                    } else {
                        source
                            .replace(&replacement.unwrap().snapshot().unwrap())
                            .unwrap();
                        assert_eq!(
                            function.instance().solve_result().unwrap_err().kind_name(),
                            "CannotConvertToType"
                        );
                    }
                    assert_eq!(indices(&output_alias), vec![1, 6, 2, 5, 3, 4]);
                    assert_eq!(output.published_version(), version);
                }
            }
        };
    }

    selector_family!(index_ports_are_managed, usize, 1, 2);
    selector_family!(u8_ports_are_managed, u8, 1, 2);
    selector_family!(u16_ports_are_managed, u16, 1, 2);
    selector_family!(u32_ports_are_managed, u32, 1, 2);
    selector_family!(u64_ports_are_managed, u64, 1, 2);
    selector_family!(u128_ports_are_managed, u128, 1, 2);
    selector_family!(i8_ports_are_managed, i8, 1, 2);
    selector_family!(i16_ports_are_managed, i16, 1, 2);
    selector_family!(i32_ports_are_managed, i32, 1, 2);
    selector_family!(i64_ports_are_managed, i64, 1, 2);
    selector_family!(i128_ports_are_managed, i128, 1, 2);
    selector_family!(f32_fractional_ports_are_managed, f32, 1.9, 2.5);
    selector_family!(f64_fractional_ports_are_managed, f64, 1.9, 2.5);
}
