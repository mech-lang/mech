#![cfg(feature = "full_compiler")]

use std::{collections::BTreeMap, process::Command};

use mech_core::{
    AccessMode, AliasPolicy, AllocationPlan, AllocationRole, ArenaPlacement, BoundCall,
    CallMemoryPlanningRequest, CellSlotId, ChangeDetectionPolicy, CurrentMemoryFootprint,
    DeliveryMode, ExecutionTarget, ExternalInteraction, ImplementationMemoryClass, InputPortLayout,
    InputPortPolicy, MemoryArenaId, MemoryFootprintWitness, MemoryLifetime, MemoryObjectId,
    MemoryObjectOwner, MemoryPlanAuditStatus, MemoryPlanError, MemoryPlanObservation,
    MemoryPlanPoint, MemorySpace, NodeId, OperationContractDeclaration, OutputConstruction,
    OutputPortPolicy, RegionAccessPlan, ResolvedOperationDescriptor, ResourceDemand,
    RuntimeFunctionId, ShapeRule, TargetMemoryProfile, TransactionRequirement, TransferDirection,
    TransferPlan, ValueCell, physical_storage_descriptor, plan_call_memory,
};
use mech_engine::ArtifactSource;
use mech_engine::memory_planner::{
    ActivationMemoryFacts, ActivationValueFact, CallSiteMemoryTemplate, PlannedValueClass,
    ProgramMemoryPlan, ProgramMemoryPlanTemplate, TurnMemoryFacts, ValueMemoryPlanTemplate,
    audit_memory_plan, instantiate_program_memory_plan, plan_turn_memory,
};

fn allocation(id: u32, current: u64, capacity: u64) -> AllocationPlan {
    AllocationPlan {
        id: MemoryObjectId::new(id),
        owner: MemoryObjectOwner::NodeScratch {
            node: NodeId::new(0),
            ordinal: id as u16,
        },
        role: AllocationRole::Scratch,
        slot: None,
        space: MemorySpace::ResidentCpu,
        current_bytes: current,
        capacity_bytes: capacity,
        payload_block_capacity: 0,
        alignment: 8,
        lifetime: MemoryLifetime::Turn {
            first: MemoryPlanPoint::new(0),
            last: MemoryPlanPoint::new(1),
        },
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(0),
            offset: 0,
        },
        reuse_group: None,
    }
}

fn plan_with(allocations: Vec<AllocationPlan>) -> ProgramMemoryPlan {
    ProgramMemoryPlan {
        values: Box::new([]),
        call_nodes: Box::new([]),
        calls: Box::new([]),
        allocations: allocations.into_boxed_slice(),
        arenas: Box::new([]),
        transfers: Box::new([]),
        budget_limits: mech_core::MemoryBudgetLimits::default(),
        peak: ResourceDemand::default(),
        budget_violations: Box::new([]),
    }
}

fn deterministic_program_plan(reverse_facts: bool, allocation_noise: usize) -> ProgramMemoryPlan {
    for value in 0..allocation_noise {
        let _ = ValueCell::from_exact(value as u64).unwrap();
    }
    let cell = ValueCell::from_exact(7_u64).unwrap();
    let descriptor = cell.resolved_descriptor().unwrap();
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let storage =
        physical_storage_descriptor(cell.representation(), &target, MemoryLifetime::Activation);
    let witness = MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements: 1,
        fixed_bytes: 8,
        retained_nodes: 1,
        ..CurrentMemoryFootprint::default()
    });
    let definitions = [
        (PlannedValueClass::Constant, None, None, None),
        (
            PlannedValueClass::Input,
            None,
            None,
            Some(CellSlotId::new(0)),
        ),
        (PlannedValueClass::State, None, None, None),
        (
            PlannedValueClass::Scratch,
            Some(NodeId::new(0)),
            Some(NodeId::new(0)),
            None,
        ),
        (
            PlannedValueClass::Scratch,
            Some(NodeId::new(1)),
            Some(NodeId::new(1)),
            None,
        ),
        (
            PlannedValueClass::Scratch,
            Some(NodeId::new(2)),
            Some(NodeId::new(2)),
            None,
        ),
        (
            PlannedValueClass::Scratch,
            Some(NodeId::new(1)),
            Some(NodeId::new(2)),
            None,
        ),
        (PlannedValueClass::State, None, None, None),
        (PlannedValueClass::PublishedOutput, None, None, None),
    ];
    let values = definitions
        .iter()
        .enumerate()
        .map(
            |(index, (class, producer, last_consumer, alias_source))| ValueMemoryPlanTemplate {
                slot: CellSlotId::new(index as u32),
                descriptor: Some(descriptor.clone()),
                class: *class,
                producer: *producer,
                last_consumer: *last_consumer,
                alias_source: *alias_source,
            },
        )
        .collect::<Vec<_>>();
    let mut entries = values
        .iter()
        .map(|value| {
            (
                value.slot,
                ActivationValueFact {
                    descriptor: descriptor.clone(),
                    storage: storage.clone(),
                    witness,
                },
            )
        })
        .collect::<Vec<_>>();
    if reverse_facts {
        entries.reverse();
    }
    let facts = ActivationMemoryFacts {
        values: entries.into_iter().collect::<BTreeMap<_, _>>(),
        classes: BTreeMap::new(),
    };
    let transaction_allocation = |id, first, last, bytes| AllocationPlan {
        id: MemoryObjectId::new(id),
        owner: MemoryObjectOwner::NodeScratch {
            node: NodeId::new(id),
            ordinal: 0,
        },
        role: AllocationRole::TransactionStage,
        slot: None,
        space: MemorySpace::ResidentCpu,
        current_bytes: bytes,
        capacity_bytes: bytes,
        payload_block_capacity: 0,
        alignment: 8,
        lifetime: MemoryLifetime::Transaction {
            first: MemoryPlanPoint::new(first),
            last: MemoryPlanPoint::new(last),
        },
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(0),
            offset: 0,
        },
        reuse_group: None,
    };
    let template = ProgramMemoryPlanTemplate {
        node_positions: (0..3)
            .map(|position| (NodeId::new(position), position))
            .collect(),
        values: values.into_boxed_slice(),
        call_nodes: Box::new([]),
        call_sites: Box::new([]),
        calls: Box::new([]),
        allocations: vec![
            transaction_allocation(100, 0, 2, 8),
            transaction_allocation(101, 2, 4, 16),
        ]
        .into_boxed_slice(),
        transfers: vec![TransferPlan {
            slot: CellSlotId::new(1),
            direction: TransferDirection::Upload,
            source: MemorySpace::Host,
            destination: MemorySpace::Device { region: 0 },
            current_bytes: 8,
            capacity_bytes: 8,
            lifetime: MemoryLifetime::Transfer {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(1),
            },
            consumer: Some(NodeId::new(0)),
            interface_name: Some("input".to_owned()),
        }]
        .into_boxed_slice(),
    };
    instantiate_program_memory_plan(&template, &target, &facts).unwrap()
}

#[test]
fn program_plan_closes_lifetimes_aliases_reuse_transactions_and_transfers() {
    let first = deterministic_program_plan(false, 0);
    let reordered = deterministic_program_plan(true, 32);
    assert_eq!(first, reordered);

    assert_eq!(first.values[0].lifetime, MemoryLifetime::Program);
    assert_eq!(first.values[1].lifetime, MemoryLifetime::Activation);
    assert_eq!(first.values[2].lifetime, MemoryLifetime::Activation);
    assert!(matches!(
        first.values[2].transaction,
        TransactionRequirement::StageAndSwap { .. }
    ));
    assert_eq!(first.values[8].class, PlannedValueClass::PublishedOutput);
    assert_eq!(first.values[8].lifetime, MemoryLifetime::Activation);
    assert!(matches!(
        first.values[8].transaction,
        TransactionRequirement::StageAndSwap { .. }
    ));
    assert!(matches!(
        first.values[3].lifetime,
        MemoryLifetime::Turn { .. }
    ));
    assert_eq!(first.values[0].alias_group, first.values[1].alias_group);
    assert!(first.values[0].alias_group.is_some());

    let sequential = [3_usize, 4, 5].map(|index| first.values[index].reuse_group);
    assert!(sequential[0].is_some());
    assert_eq!(sequential[0], sequential[1]);
    assert_eq!(sequential[1], sequential[2]);
    assert_ne!(first.values[4].reuse_group, first.values[6].reuse_group);
    let placement = |index: usize| {
        first
            .allocations
            .iter()
            .find(|allocation| allocation.id == first.values[index].object)
            .unwrap()
            .placement
    };
    assert_eq!(placement(3), placement(4));
    assert_eq!(placement(4), placement(5));
    assert_ne!(placement(4), placement(6));
    assert_eq!(first.peak.transaction_peak_bytes, 24);
    assert_eq!(first.peak.transfer_bytes, 8);
    assert_eq!(first.transfers.len(), 1);
}

#[test]
fn reuse_placement_reserves_the_whole_group_before_following_slot_order() {
    for prefix_padding in [false, true] {
        let mut cells = Vec::new();
        let mut definitions = Vec::new();
        if prefix_padding {
            cells.push(ValueCell::from_exact(1_u64).unwrap());
            definitions.push((PlannedValueClass::Input, None));
        }
        let later = cells.len();
        cells.push(ValueCell::from_exact(2_u64).unwrap());
        definitions.push((PlannedValueClass::Scratch, Some(NodeId::new(0))));
        cells.push(ValueCell::from_exact(3_u64).unwrap());
        definitions.push((PlannedValueClass::Input, None));
        let earlier = cells.len();
        cells.push(ValueCell::from_exact(4_u128).unwrap());
        definitions.push((PlannedValueClass::Scratch, Some(NodeId::new(1))));
        let target = TargetMemoryProfile::current_direct_host().unwrap();
        let values = definitions
            .iter()
            .enumerate()
            .map(|(index, (class, producer))| ValueMemoryPlanTemplate {
                slot: CellSlotId::new(index as u32),
                descriptor: Some(cells[index].resolved_descriptor().unwrap()),
                class: *class,
                producer: *producer,
                last_consumer: *producer,
                alias_source: None,
            })
            .collect::<Vec<_>>();
        let facts = ActivationMemoryFacts {
            values: cells
                .iter()
                .enumerate()
                .map(|(index, cell)| {
                    (
                        CellSlotId::new(index as u32),
                        ActivationValueFact {
                            descriptor: cell.resolved_descriptor().unwrap(),
                            storage: physical_storage_descriptor(
                                cell.representation(),
                                &target,
                                MemoryLifetime::Activation,
                            ),
                            witness: MemoryFootprintWitness::Known(CurrentMemoryFootprint {
                                logical_elements: 1,
                                retained_nodes: 1,
                                ..CurrentMemoryFootprint::default()
                            }),
                        },
                    )
                })
                .collect(),
            classes: BTreeMap::new(),
        };
        let template = ProgramMemoryPlanTemplate {
            node_positions: BTreeMap::from([(NodeId::new(1), 0), (NodeId::new(0), 1)]),
            values: values.into_boxed_slice(),
            ..ProgramMemoryPlanTemplate::default()
        };
        let plan = instantiate_program_memory_plan(&template, &target, &facts).unwrap();
        let small = &plan.allocations[later];
        let large = &plan.allocations[earlier];
        assert_eq!(small.reuse_group, large.reuse_group);
        assert_eq!(small.placement, large.placement);
        assert_eq!(large.placement.offset % u64::from(large.alignment), 0);
        // Realization validates capacity, alignment and overlaps with every live input.
        mech_engine::memory_runtime::ManagedProgramMemory::realize(&plan).unwrap();
    }
}

#[test]
fn call_transaction_identity_selects_published_and_mutated_state_stages() {
    for initial_payload in [0, 9] {
        let node = NodeId::new(7);
        let input_slot = CellSlotId::new(0);
        let output_slot = CellSlotId::new(1);
        let cell = ValueCell::from_exact(if initial_payload == 0 {
            String::new()
        } else {
            "published".to_owned()
        })
        .unwrap();
        let descriptor = cell.resolved_descriptor().unwrap();
        let target = TargetMemoryProfile::current_direct_host().unwrap();
        let storage =
            physical_storage_descriptor(cell.representation(), &target, MemoryLifetime::Activation);
        let footprint = MemoryFootprintWitness::Known(CurrentMemoryFootprint {
            logical_elements: 1,
            fixed_bytes: target.primitives.string_header.bytes,
            payload_bytes: initial_payload,
            ..CurrentMemoryFootprint::default()
        });
        let operation = ResolvedOperationDescriptor::from_name(
            "test/r5-published-output",
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
            vec![descriptor.clone()].into_boxed_slice(),
            RuntimeFunctionId::from_name("R5PublishedOutput"),
            ExecutionTarget::DirectRuntime,
        )
        .unwrap();
        let call = plan_call_memory(CallMemoryPlanningRequest {
            bound_call: &bound,
            input_storage: &[storage.clone()],
            output_storage: &[storage.clone()],
            input_witnesses: &[footprint],
            output_witnesses: &[footprint],
            published_output_witnesses: &[footprint],
            implementation_memory: ImplementationMemoryClass::NoAdditionalScratch,
            target: &target,
            regions: &[RegionAccessPlan::WholeValue],
        })
        .unwrap();
        let facts = ActivationMemoryFacts {
            values: BTreeMap::from([
                (
                    input_slot,
                    ActivationValueFact {
                        descriptor: descriptor.clone(),
                        storage: storage.clone(),
                        witness: footprint,
                    },
                ),
                (
                    output_slot,
                    ActivationValueFact {
                        descriptor: descriptor.clone(),
                        storage,
                        witness: footprint,
                    },
                ),
            ]),
            classes: BTreeMap::new(),
        };
        let template = ProgramMemoryPlanTemplate {
            node_positions: BTreeMap::from([(node, 0)]),
            values: vec![
                ValueMemoryPlanTemplate {
                    slot: input_slot,
                    descriptor: Some(descriptor.clone()),
                    class: PlannedValueClass::Input,
                    producer: None,
                    last_consumer: Some(node),
                    alias_source: None,
                },
                ValueMemoryPlanTemplate {
                    slot: output_slot,
                    descriptor: Some(descriptor),
                    class: PlannedValueClass::PublishedOutput,
                    producer: Some(node),
                    last_consumer: None,
                    alias_source: None,
                },
            ]
            .into_boxed_slice(),
            call_nodes: vec![node].into_boxed_slice(),
            call_sites: vec![CallSiteMemoryTemplate {
                node,
                input_sources: vec![ArtifactSource::Slot(input_slot)].into_boxed_slice(),
                output_slots: vec![output_slot].into_boxed_slice(),
            }]
            .into_boxed_slice(),
            calls: vec![call].into_boxed_slice(),
            allocations: Box::new([]),
            transfers: Box::new([]),
        };
        let program = instantiate_program_memory_plan(&template, &target, &facts).unwrap();
        assert!(matches!(
            program.values[1].transaction,
            TransactionRequirement::StageAndSwap { .. }
        ));
        assert_eq!(
            program.calls[0].transactions[0],
            program.values[1].transaction
        );
        assert_eq!(
            program
                .allocations
                .iter()
                .filter(
                    |allocation| allocation.role == AllocationRole::TransactionStage
                        && allocation.slot.is_some()
                )
                .count(),
            1,
        );
        let turn_facts = TurnMemoryFacts {
            resolved_footprints: BTreeMap::from([(
                (node, mech_core::PortDirection::Output, 0),
                CurrentMemoryFootprint {
                    logical_elements: 1,
                    payload_bytes: 13,
                    retained_nodes: 2,
                    ..CurrentMemoryFootprint::default()
                },
            )]),
            published_footprints: BTreeMap::from([(
                (node, 0),
                CurrentMemoryFootprint {
                    logical_elements: 1,
                    payload_bytes: initial_payload,
                    ..CurrentMemoryFootprint::default()
                },
            )]),
            ..TurnMemoryFacts::default()
        };
        let turn = plan_turn_memory(&program, node, &turn_facts).unwrap();
        assert_eq!(turn.transactions.len(), 1);
        // The live turn now retains its port storage as well as the stage.
        let stages = turn
            .allocations
            .iter()
            .filter(|a| a.role == AllocationRole::TransactionStage && a.slot.is_some())
            .collect::<Vec<_>>();
        assert_eq!(stages.len(), 1);
        let stage = stages[0];
        assert!(matches!(stage.lifetime, MemoryLifetime::Transaction { .. }));
        assert_eq!(stage.capacity_bytes, target.primitives.string_header.bytes);
        let arena = turn
            .arenas
            .iter()
            .find(|a| a.id == stage.placement.arena)
            .unwrap();
        assert!(stage.placement.offset + stage.capacity_bytes <= arena.capacity_bytes);

        let staged_payload = turn
            .allocations
            .iter()
            .find(|a| {
                a.owner == stage.owner
                    && a.role == AllocationRole::TransactionStage
                    && a.slot.is_none()
            })
            .unwrap();
        assert_eq!(staged_payload.capacity_bytes, 13);
        assert_eq!(staged_payload.payload_block_capacity, 2);
        let published_payload = turn
            .allocations
            .iter()
            .find(|a| a.owner == stage.owner && a.role == AllocationRole::VariablePayload)
            .unwrap();
        assert_eq!(published_payload.current_bytes, initial_payload);
        let (domain, realized) = mech_engine::memory_runtime::realize_turn_memory(&turn).unwrap();
        let key = domain
            .plan_object_key(realized.revision(), staged_payload.id)
            .unwrap();
        let allocator = domain.planned_allocator(&realized, key).unwrap();
        let first = mech_core::ManagedString::try_new(allocator.clone(), "hello, ").unwrap();
        let second = mech_core::ManagedString::try_new(allocator.clone(), "world!").unwrap();
        assert_eq!(first.as_str(), "hello, ");
        assert_eq!(second.as_str(), "world!");
        assert_eq!(allocator.allocated_bytes().unwrap(), 13);
        let scoped_call = turn.call.as_ref().unwrap();
        assert_eq!(scoped_call.allocations, turn.allocations);
        let scoped_domain = mech_core::MemoryDomain::new().unwrap();
        let scoped_realized = scoped_domain.realize_call_memory_plan(scoped_call).unwrap();
        let scoped_key = scoped_domain
            .plan_object_key(scoped_realized.revision(), staged_payload.id)
            .unwrap();
        let scoped_allocator = scoped_domain
            .planned_allocator(&scoped_realized, scoped_key)
            .unwrap();
        let scoped_value =
            mech_core::ManagedString::try_new(scoped_allocator, "hello, world!").unwrap();
        assert_eq!(scoped_value.as_str(), "hello, world!");

        // State cells can be written by several RMW nodes, so the value
        // declaration does not identify every turn producer. The remapped call
        // transaction still selects the shared global stage for this writer.
        let mut state_mutation = program.clone();
        state_mutation.values[1].class = PlannedValueClass::State;
        state_mutation.values[1].producer = None;
        let state_turn = plan_turn_memory(&state_mutation, node, &turn_facts).unwrap();
        assert_eq!(state_turn.transactions, turn.transactions);
        assert_eq!(state_turn.allocations, turn.allocations);

        // An unpublished temporary has no value-owned stage. Its complete
        // call-local stage family must instead survive program remapping.
        let mut temporary_template = template;
        temporary_template.values[1].class = PlannedValueClass::Scratch;
        let temporary =
            instantiate_program_memory_plan(&temporary_template, &target, &facts).unwrap();
        let temporary_turn = plan_turn_memory(&temporary, node, &turn_facts).unwrap();
        let temporary_payload = temporary_turn
            .allocations
            .iter()
            .find(|a| {
                a.owner == MemoryObjectOwner::NodeOutput { node, port: 0 }
                    && a.role == AllocationRole::TransactionStage
                    && a.payload_block_capacity != 0
            })
            .unwrap();
        assert_eq!(temporary_payload.capacity_bytes, 13);
        assert_eq!(temporary_payload.payload_block_capacity, 2);
        let (domain, realized) =
            mech_engine::memory_runtime::realize_turn_memory(&temporary_turn).unwrap();
        let key = domain
            .plan_object_key(realized.revision(), temporary_payload.id)
            .unwrap();
        let allocator = domain.planned_allocator(&realized, key).unwrap();
        let value = mech_core::ManagedString::try_new(allocator, "hello, world!").unwrap();
        assert_eq!(value.as_str(), "hello, world!");
    }
}

#[test]
fn fresh_process_plan_diagnostic_is_byte_identical() {
    const CHILD: &str = "MECH_R5_PLAN_DIAGNOSTIC_CHILD";
    if std::env::var_os(CHILD).is_some() {
        // Keep the plan separate from libtest progress and elapsed time.
        eprint!("{}", deterministic_program_plan(false, 0).diagnostic_text());
        return;
    }
    let executable = std::env::current_exe().unwrap();
    let run = || {
        Command::new(&executable)
            .args([
                "--exact",
                "fresh_process_plan_diagnostic_is_byte_identical",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap()
    };
    let first = run();
    let second = run();
    assert!(first.status.success(), "first child failed: {first:?}");
    assert!(second.status.success(), "second child failed: {second:?}");
    let expected = deterministic_program_plan(false, 0).diagnostic_text();
    assert!(!expected.is_empty());
    assert_eq!(first.stderr, expected.as_bytes());
    assert_eq!(second.stderr, expected.as_bytes());
}

#[test]
fn shadow_audit_distinguishes_exact_and_deferred_capacity() {
    let plan = plan_with(vec![allocation(0, 8, 16)]);
    let exact = audit_memory_plan(
        &plan,
        &[MemoryPlanObservation {
            object: MemoryObjectId::new(0),
            current_bytes: 8,
            capacity_bytes: 16,
            payload_bytes: 0,
            retained_nodes: 0,
            logical_elements: 0,
        }],
    )
    .unwrap();
    assert_eq!(exact.statuses[0].1, MemoryPlanAuditStatus::Exact);
    exact.assert_conformant().unwrap();

    let deferred = audit_memory_plan(
        &plan,
        &[MemoryPlanObservation {
            object: MemoryObjectId::new(0),
            current_bytes: 8,
            capacity_bytes: 8,
            payload_bytes: 0,
            retained_nodes: 0,
            logical_elements: 0,
        }],
    )
    .unwrap();
    assert_eq!(
        deferred.statuses[0].1,
        MemoryPlanAuditStatus::CapacityDeferredToR6
    );
    deferred.assert_conformant().unwrap();
}

#[test]
fn shadow_audit_requires_current_fixed_resident_and_device_storage() {
    for space in [MemorySpace::ResidentCpu, MemorySpace::Device { region: 0 }] {
        for role in [AllocationRole::FixedStorage, AllocationRole::Scratch] {
            let mut fixed = allocation(0, 8, 16);
            fixed.role = role;
            fixed.space = space;
            let plan = plan_with(vec![fixed]);
            let observed = MemoryPlanObservation {
                object: MemoryObjectId::new(0),
                current_bytes: 0,
                capacity_bytes: 8,
                payload_bytes: 0,
                retained_nodes: 0,
                logical_elements: 0,
            };
            let report = audit_memory_plan(&plan, &[observed]).unwrap();
            assert_eq!(report.mismatches.len(), 1);
            assert_eq!(report.mismatches[0].field, "current_bytes");
            assert!(report.assert_conformant().is_err());
        }

        let cell = ValueCell::from_exact(1_f64).unwrap();
        let descriptor = cell.resolved_descriptor().unwrap();
        let target = TargetMemoryProfile::current_resident_cpu().unwrap();
        let mut storage =
            physical_storage_descriptor(cell.representation(), &target, MemoryLifetime::Activation);
        storage.space = space;
        let facts = ActivationMemoryFacts {
            values: BTreeMap::from([(
                CellSlotId::new(0),
                ActivationValueFact {
                    descriptor: descriptor.clone(),
                    storage,
                    witness: MemoryFootprintWitness::Known(CurrentMemoryFootprint::default()),
                },
            )]),
            classes: BTreeMap::new(),
        };
        let template = ProgramMemoryPlanTemplate {
            values: vec![ValueMemoryPlanTemplate {
                slot: CellSlotId::new(0),
                descriptor: Some(descriptor),
                class: PlannedValueClass::Input,
                producer: None,
                last_consumer: None,
                alias_source: None,
            }]
            .into_boxed_slice(),
            ..ProgramMemoryPlanTemplate::default()
        };
        let value_plan = instantiate_program_memory_plan(&template, &target, &facts).unwrap();
        let report = audit_memory_plan(
            &value_plan,
            &[MemoryPlanObservation {
                object: MemoryObjectId::new(0),
                current_bytes: 8,
                capacity_bytes: 8,
                payload_bytes: 0,
                retained_nodes: 0,
                logical_elements: 0,
            }],
        )
        .unwrap();
        assert_eq!(report.mismatches.len(), 1);
        assert_eq!(report.mismatches[0].field, "logical_elements");
        assert!(report.assert_conformant().is_err());
    }
    // Current capacity may still be below a future bound, and direct/variable
    // backings retain their existing within-capacity audit semantics.
    for (space, role) in [
        (MemorySpace::Host, AllocationRole::FixedStorage),
        (MemorySpace::ResidentCpu, AllocationRole::VariablePayload),
    ] {
        let mut backing = allocation(0, 8, 16);
        backing.role = role;
        backing.space = space;
        let report = audit_memory_plan(
            &plan_with(vec![backing]),
            &[MemoryPlanObservation {
                object: MemoryObjectId::new(0),
                current_bytes: 4,
                capacity_bytes: 8,
                payload_bytes: 0,
                retained_nodes: 0,
                logical_elements: 0,
            }],
        )
        .unwrap();
        report.assert_conformant().unwrap();
    }
}

#[test]
fn shadow_audit_rejects_missing_unexpected_and_oversized_observations() {
    let plan = plan_with(vec![allocation(0, 8, 16)]);
    assert_eq!(
        audit_memory_plan(&plan, &[]),
        Err(MemoryPlanError::ObservationMissing {
            object: MemoryObjectId::new(0)
        })
    );
    assert!(matches!(
        audit_memory_plan(
            &plan,
            &[MemoryPlanObservation {
                object: MemoryObjectId::new(1),
                current_bytes: 0,
                capacity_bytes: 0,
                payload_bytes: 0,
                retained_nodes: 0,
                logical_elements: 0,
            }]
        ),
        Err(MemoryPlanError::ObservationUnexpected { .. })
    ));
    let report = audit_memory_plan(
        &plan,
        &[MemoryPlanObservation {
            object: MemoryObjectId::new(0),
            current_bytes: 17,
            capacity_bytes: 17,
            payload_bytes: 0,
            retained_nodes: 0,
            logical_elements: 0,
        }],
    )
    .unwrap();
    assert!(matches!(
        report.assert_conformant(),
        Err(MemoryPlanError::ObservationExceeded { .. })
    ));
}

#[test]
fn turn_plan_is_deterministic_and_accumulates_current_work() {
    let mut plan = plan_with(vec![allocation(0, 8, 8)]);
    plan.budget_limits.max_cloned_bytes = Some(11);
    let facts = TurnMemoryFacts {
        additional_demand: ResourceDemand {
            cloned_bytes: 12,
            retained_nodes: 3,
            ..ResourceDemand::default()
        },
        ..TurnMemoryFacts::default()
    };
    let first = plan_turn_memory(&plan, NodeId::new(0), &facts).unwrap();
    let second = plan_turn_memory(&plan, NodeId::new(0), &facts).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.allocations.len(), 1);
    assert_eq!(first.arenas.len(), 1);
    assert_eq!(first.arenas[0].capacity_bytes, 8);
    assert_eq!(first.demand.cloned_bytes, 12);
    assert_eq!(first.demand.retained_nodes, 3);
    assert!(first.budget_violations.iter().any(|violation| {
        violation.dimension == mech_core::MemoryBudgetDimension::ClonedBytes
            && violation.required == 12
            && violation.limit == 11
    }));
}

#[test]
fn deferred_transaction_payload_is_replaced_inside_a_turn_arena() {
    let input = ValueCell::from_exact("input".to_owned()).unwrap();
    let output = ValueCell::from_exact("output".to_owned()).unwrap();
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r5-turn-placement",
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
        vec![input.resolved_descriptor().unwrap()].into_boxed_slice(),
        vec![output.resolved_descriptor().unwrap()].into_boxed_slice(),
        RuntimeFunctionId::from_name("R5TurnPlacement"),
        ExecutionTarget::ResidentCpu,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_resident_cpu().unwrap();
    let lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(0),
        last: MemoryPlanPoint::new(1),
    };
    let deferred = MemoryFootprintWitness::Deferred(mech_core::MemoryWitnessStage::Turn);
    let mut call = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &bound,
        input_storage: &[physical_storage_descriptor(
            input.representation(),
            &target,
            lifetime,
        )],
        output_storage: &[physical_storage_descriptor(
            output.representation(),
            &target,
            lifetime,
        )],
        input_witnesses: &[deferred],
        output_witnesses: &[deferred],
        published_output_witnesses: &[deferred],
        implementation_memory: ImplementationMemoryClass::NoAdditionalScratch,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    for allocation in &mut call.allocations {
        if matches!(allocation.lifetime, MemoryLifetime::Transaction { .. }) {
            allocation.owner = MemoryObjectOwner::TransactionStage {
                node: NodeId::new(0),
                output: 0,
            };
        }
    }
    let mut plan = plan_with(call.allocations.to_vec());
    plan.call_nodes = vec![NodeId::new(0)].into_boxed_slice();
    plan.calls = vec![call].into_boxed_slice();
    plan.budget_limits = target.limits;
    let facts = TurnMemoryFacts {
        resolved_footprints: BTreeMap::from([
            (
                (NodeId::new(0), mech_core::PortDirection::Input, 0),
                CurrentMemoryFootprint {
                    logical_elements: 1,
                    payload_bytes: 11,
                    ..CurrentMemoryFootprint::default()
                },
            ),
            (
                (NodeId::new(0), mech_core::PortDirection::Output, 0),
                CurrentMemoryFootprint {
                    logical_elements: 1,
                    payload_bytes: 13,
                    retained_nodes: 2,
                    encoded_bytes: 19,
                    schema_bytes: 7,
                    shape_parameter_count: 1,
                    ..CurrentMemoryFootprint::default()
                },
            ),
        ]),
        ..TurnMemoryFacts::default()
    };
    let turn = plan_turn_memory(&plan, NodeId::new(0), &facts).unwrap();
    let staged = turn
        .allocations
        .iter()
        .find(|allocation| allocation.role == AllocationRole::TransactionStage)
        .unwrap();
    let arena = turn
        .arenas
        .iter()
        .find(|arena| arena.id == staged.placement.arena)
        .unwrap();
    assert_eq!(staged.capacity_bytes, target.primitives.string_header.bytes);
    let payload = turn
        .allocations
        .iter()
        .find(|allocation| {
            allocation.owner == staged.owner && allocation.payload_block_capacity != 0
        })
        .unwrap();
    assert_eq!(payload.current_bytes, 13);
    assert_eq!(payload.capacity_bytes, 13);
    assert_eq!(payload.payload_block_capacity, 2);
    let (domain, realized) = mech_engine::memory_runtime::realize_turn_memory(&turn).unwrap();
    let key = domain
        .plan_object_key(realized.revision(), payload.id)
        .unwrap();
    let allocator = domain.planned_allocator(&realized, key).unwrap();
    let first = mech_core::ManagedString::try_new(allocator.clone(), "hello, ").unwrap();
    let second = mech_core::ManagedString::try_new(allocator.clone(), "world!").unwrap();
    assert_eq!(first.as_str(), "hello, ");
    assert_eq!(second.as_str(), "world!");
    assert_eq!(allocator.allocated_bytes().unwrap(), 13);
    assert!(
        staged.placement.offset + staged.capacity_bytes <= arena.capacity_bytes,
        "resolved stage must remain inside its re-placed turn arena"
    );
}

#[test]
fn current_footprints_are_explicit_values_not_allocation_identity() {
    let left = MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements: 3,
        fixed_bytes: 24,
        ..CurrentMemoryFootprint::default()
    });
    let right = left;
    assert_eq!(left, right);
}

fn empty_template() -> ProgramMemoryPlanTemplate {
    ProgramMemoryPlanTemplate {
        node_positions: BTreeMap::new(),
        values: Box::new([]),
        call_nodes: Box::new([]),
        call_sites: Box::new([]),
        calls: Box::new([]),
        allocations: Box::new([]),
        transfers: Box::new([]),
    }
}

#[test]
fn program_budget_checks_overlapping_allocations_not_each_allocation_alone() {
    let mut target = TargetMemoryProfile::current_resident_cpu().unwrap();
    target.limits.max_temporary_bytes = Some(16 * 1024 * 1024);
    let mut template = empty_template();
    template.allocations = vec![
        allocation(0, 9 * 1024 * 1024, 9 * 1024 * 1024),
        allocation(1, 9 * 1024 * 1024, 9 * 1024 * 1024),
    ]
    .into();
    let plan =
        instantiate_program_memory_plan(&template, &target, &ActivationMemoryFacts::default())
            .unwrap();
    assert_eq!(plan.peak.turn_peak_bytes, 18 * 1024 * 1024);
    assert!(plan.budget_violations.iter().any(|v| v.dimension
        == mech_core::MemoryBudgetDimension::TemporaryBytes
        && v.required == 18 * 1024 * 1024));
    template.allocations[1].lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(2),
        last: MemoryPlanPoint::new(3),
    };
    let plan =
        instantiate_program_memory_plan(&template, &target, &ActivationMemoryFacts::default())
            .unwrap();
    assert_eq!(plan.peak.turn_peak_bytes, 9 * 1024 * 1024);
    assert!(plan.budget_violations.is_empty());
}

#[test]
fn transaction_staging_and_scratch_share_the_temporary_peak() {
    let mut target = TargetMemoryProfile::current_resident_cpu().unwrap();
    target.limits.max_temporary_bytes = Some(16 * 1024 * 1024);
    let mut template = empty_template();
    let mut stage = allocation(1, 9 * 1024 * 1024, 9 * 1024 * 1024);
    stage.role = AllocationRole::TransactionStage;
    stage.lifetime = MemoryLifetime::Transaction {
        first: MemoryPlanPoint::new(0),
        last: MemoryPlanPoint::new(1),
    };
    template.allocations = vec![allocation(0, 9 * 1024 * 1024, 9 * 1024 * 1024), stage].into();
    let plan =
        instantiate_program_memory_plan(&template, &target, &ActivationMemoryFacts::default())
            .unwrap();
    assert_eq!(plan.peak.transaction_peak_bytes, 9 * 1024 * 1024);
    assert_eq!(plan.peak.turn_peak_bytes, 18 * 1024 * 1024);
    assert!(
        plan.budget_violations
            .iter()
            .any(|v| v.dimension == mech_core::MemoryBudgetDimension::TemporaryBytes)
    );
}

#[test]
fn aggregate_transfer_limit_counts_every_boundary_in_its_memory_space() {
    let mut target = TargetMemoryProfile::current_direct_host().unwrap();
    target.limits.max_transfer_bytes = Some(100);
    let mut template = empty_template();
    template.transfers = (0..2)
        .map(|i| TransferPlan {
            slot: CellSlotId::new(i),
            direction: TransferDirection::Upload,
            source: MemorySpace::Host,
            destination: MemorySpace::Device { region: 0 },
            current_bytes: 60,
            capacity_bytes: 60,
            lifetime: MemoryLifetime::Transfer {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(1),
            },
            consumer: Some(NodeId::new(i)),
            interface_name: None,
        })
        .collect::<Vec<_>>()
        .into();
    let plan =
        instantiate_program_memory_plan(&template, &target, &ActivationMemoryFacts::default())
            .unwrap();
    assert_eq!(plan.peak.transfer_bytes, 120);
    assert!(plan.budget_violations.iter().any(|v| v.dimension
        == mech_core::MemoryBudgetDimension::TransferBytes
        && v.required == 120));
}
