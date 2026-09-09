#![cfg(feature = "functions")]

use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, CallAccessRequest,
    CellPublicationCandidate, CellPublicationEvidence, FunctionInvocation, KernelMemoryFrame,
    ManagedCallAccessRequest, ManagedPort, ManagedSequence, ManagedString, MechExecutionServices,
    MechFunctionImpl, MemoryAccessMode, MemoryAccessRegion, MemoryArenaId, MemoryBudgetLimits,
    MemoryBudgetViolation, MemoryDomain, MemoryLifetime, MemoryObjectId, MemoryObjectOwner,
    MemoryPlanPoint, MemoryPlanRevision, MemoryRuntimeError, MemorySpace, NoMechExecutionServices,
    PreparedCellPublication, PreparedCellPublicationBatch, PublicationCandidate,
    ReactiveSolveStatus, ResourceDemand, ReuseGroupId, RuntimeBinding, RuntimePlanView, SchemaBody,
    SchemaDraft, SchemaTableBuilder, ValueCell, ValueDataDraft, ValueDraft,
};

fn runtime_plan_view<'a>(
    revision: MemoryPlanRevision,
    allocations: &'a [AllocationPlan],
    arenas: &'a [ArenaPlan],
    demand: ResourceDemand,
    limits: MemoryBudgetLimits,
    violations: &'a [MemoryBudgetViolation],
) -> RuntimePlanView<'a> {
    RuntimePlanView::new(
        revision,
        allocations,
        arenas,
        demand,
        0,
        limits,
        &[],
        64,
        violations,
    )
}

fn allocation(
    id: u32,
    arena: u32,
    offset: u64,
    current: u64,
    capacity: u64,
    lifetime: MemoryLifetime,
    reuse_group: Option<u32>,
) -> AllocationPlan {
    AllocationPlan {
        id: MemoryObjectId::new(id),
        owner: MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: mech_core::PortDirection::Output,
            port: id as u16,
        },
        role: AllocationRole::FixedStorage,
        slot: Some(mech_core::PlannedSlotKind::FixedScalar(
            mech_core::ScalarMemoryKind::Unsigned(mech_core::IntegerWidth::W64),
        )),
        space: MemorySpace::Host,
        current_bytes: current,
        capacity_bytes: capacity,
        payload_block_capacity: 0,
        alignment: 8,
        lifetime,
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(arena),
            offset,
        },
        reuse_group: reuse_group.map(ReuseGroupId::new),
    }
}

fn f64_allocation(
    id: u32,
    arena: u32,
    offset: u64,
    current: u64,
    capacity: u64,
    lifetime: MemoryLifetime,
) -> AllocationPlan {
    let mut allocation = allocation(id, arena, offset, current, capacity, lifetime, None);
    allocation.slot = Some(mech_core::PlannedSlotKind::FixedScalar(
        mech_core::ScalarMemoryKind::Floating(mech_core::FloatWidth::W64),
    ));
    allocation
}

fn arena(id: u32, backing: ArenaBackingKind, capacity: u64, members: &[u32]) -> ArenaPlan {
    ArenaPlan {
        id: MemoryArenaId::new(id),
        space: MemorySpace::Host,
        backing,
        alignment: 8,
        capacity_bytes: capacity,
        members: members
            .iter()
            .copied()
            .map(MemoryObjectId::new)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

#[test]
fn reservations_are_finite_and_release_unused_authority() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 8, 64, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 64, &[0])];
    {
        let reservation = domain
            .prepare_realization(runtime_plan_view(
                revision,
                &allocations,
                &arenas,
                ResourceDemand::default(),
                MemoryBudgetLimits::default(),
                &[],
            ))
            .unwrap();
        assert_eq!(reservation.reserved_bytes(), 72);
        assert_eq!(domain.ledger().reserved_bytes, 72);
        assert_eq!(domain.ledger().active_reservations, 1);
    }
    assert_eq!(domain.ledger().reserved_bytes, 0);
    assert_eq!(domain.ledger().active_reservations, 0);
}

#[test]
fn realization_recomputes_budget_instead_of_trusting_cached_violations() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let demand = ResourceDemand {
        turn_peak_bytes: 17,
        ..ResourceDemand::default()
    };
    let limits = MemoryBudgetLimits {
        max_temporary_bytes: Some(16),
        ..MemoryBudgetLimits::default()
    };

    assert!(matches!(
        domain.prepare_realization(runtime_plan_view(
            revision,
            &allocations,
            &arenas,
            demand,
            limits,
            &[],
        )),
        Err(MemoryRuntimeError::BudgetExceeded {
            requested: 17,
            limit: 16,
            ..
        })
    ));
    assert_eq!(domain.ledger().reserved_bytes, 0);
    assert_eq!(domain.ledger().active_reservations, 0);
}

#[test]
fn realization_tracks_initialization_and_scoped_access() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 8, 64, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 64, &[0])];
    let reservation = domain
        .prepare_realization(runtime_plan_view(
            revision,
            &allocations,
            &arenas,
            ResourceDemand::default(),
            MemoryBudgetLimits::default(),
            &[],
        ))
        .unwrap();
    let realized = domain.materialize(reservation).unwrap();
    let key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let binding = realized.binding(key).unwrap();
    assert_eq!(binding.required_initialization_bytes(), 8);
    assert_eq!(binding.initialized_bytes(), 0);
    let unreadable = domain.prepare_call(
        &realized,
        &[CallAccessRequest {
            object: key,
            mode: MemoryAccessMode::Read,
            region: MemoryAccessRegion::Contiguous {
                offset_bytes: 0,
                length_bytes: 8,
            },
        }],
    );
    assert!(matches!(
        unreadable,
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));

    let write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: key,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            }],
        )
        .unwrap();
    domain
        .acquire_call(&realized, &write)
        .unwrap()
        .with_object_init_writer::<u8>(key, |writer| writer.copy_from_slice(&42_u64.to_ne_bytes()))
        .unwrap();
    assert_eq!(realized.binding(key).unwrap().initialized_bytes(), 8);

    let read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: key,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let frame = domain.acquire_call(&realized, &read).unwrap();
    let value = frame
        .with_bytes(key, |bytes| u64::from_ne_bytes(bytes.try_into().unwrap()))
        .unwrap();
    assert_eq!(value, 42);

    let observations = domain.allocation_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].capacity_bytes, 64);
    assert_eq!(observations[0].actual_block_bytes, 64);
    assert!(observations[0].actual_block_alignment >= 8);
}

#[test]
fn logical_managed_ports_resolve_only_inside_the_prepared_lease_scope() {
    let cell = ValueCell::from_exact(0_u64).unwrap();
    let clone = cell.clone();
    let invocation = FunctionInvocation::nullary(cell.clone());
    let port = invocation.output().try_managed::<u64>().unwrap();
    assert_eq!(port.logical_cell_id(), cell.reactive_cell_id());

    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let prepared = domain
        .prepare_managed_call(
            &realized,
            &[ManagedCallAccessRequest::new(
                &port,
                object,
                MemoryAccessMode::Write,
                MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            )],
        )
        .unwrap();
    let mut frame = domain.acquire_call(&realized, &prepared).unwrap();
    frame
        .with_port_init_writer(&port, |writer| writer.write_next(17))
        .unwrap();
    drop(frame);

    let prepared = domain
        .prepare_managed_call(
            &realized,
            &[ManagedCallAccessRequest::new(
                &port,
                object,
                MemoryAccessMode::Read,
                MemoryAccessRegion::WholeInitialized,
            )],
        )
        .unwrap();
    let frame = domain.acquire_call(&realized, &prepared).unwrap();
    assert_eq!(
        frame.with_port_slice(&port, |values| values[0]).unwrap(),
        17
    );
    assert!(cell.same_logical_cell(&clone));
}

#[test]
fn value_cell_clones_share_shape_storage_and_publication_version() {
    let cell = ValueCell::dynamic_matrix(
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        vec![1, 2].into_boxed_slice(),
        vec![
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(1.0)),
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(2.0)),
        ]
        .into_boxed_slice(),
    )
    .unwrap();
    let clone = cell.clone();
    let initial_version = cell.published_version();
    let replacement = ValueCell::dynamic_matrix(
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        vec![2, 2].into_boxed_slice(),
        vec![
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(3.0)),
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(4.0)),
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(5.0)),
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(6.0)),
        ]
        .into_boxed_slice(),
    )
    .unwrap()
    .snapshot()
    .unwrap();

    cell.replace(&replacement).unwrap();
    assert!(cell.same_logical_cell(&clone));
    assert_eq!(
        clone.snapshot().unwrap().canonical_data_draft().unwrap(),
        replacement.canonical_data_draft().unwrap(),
    );
    assert_eq!(clone.shape().parameter_values(), &[2, 2]);
    assert!(clone.published_version() > initial_version);
}

#[test]
fn managed_function_entry_executes_only_through_its_complete_frame() {
    struct Double {
        input: ManagedPort<u64>,
        output: ManagedPort<u64>,
    }

    impl MechFunctionImpl for Double {
        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            _: &mut dyn MechExecutionServices,
        ) -> mech_core::MResult<ReactiveSolveStatus> {
            let value = frame.with_port_slice(&self.input, |values| values[0])?;
            frame.with_port_init_writer(&self.output, |writer| writer.write_next(value * 2))?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn to_string(&self) -> String {
            "r6 low-level double kernel".into()
        }
    }

    let input_cell = ValueCell::from_exact(0_u64).unwrap();
    let output_cell = ValueCell::from_exact(0_u64).unwrap();
    let invocation = FunctionInvocation::unary(output_cell, input_cell);
    let input = invocation.input(0).unwrap().try_managed::<u64>().unwrap();
    let output = invocation.output().try_managed::<u64>().unwrap();
    let function = Double {
        input: input.clone(),
        output: output.clone(),
    };

    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None),
        allocation(1, 0, 8, 0, 8, MemoryLifetime::Activation, None),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0, 1])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let input_object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let output_object = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    let initialize = domain
        .prepare_managed_call(
            &realized,
            &[ManagedCallAccessRequest::new(
                &input,
                input_object,
                MemoryAccessMode::Write,
                MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            )],
        )
        .unwrap();
    domain
        .acquire_call(&realized, &initialize)
        .unwrap()
        .with_port_init_writer(&input, |writer| writer.write_next(21))
        .unwrap();
    let prepared = domain
        .prepare_managed_call(
            &realized,
            &[
                ManagedCallAccessRequest::new(
                    &input,
                    input_object,
                    MemoryAccessMode::Read,
                    MemoryAccessRegion::WholeInitialized,
                ),
                ManagedCallAccessRequest::new(
                    &output,
                    output_object,
                    MemoryAccessMode::Write,
                    MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                ),
            ],
        )
        .unwrap();
    let mut frame = domain.acquire_call(&realized, &prepared).unwrap();
    let mut services = NoMechExecutionServices;
    assert_eq!(
        function.solve_managed(&mut frame, &mut services).unwrap(),
        ReactiveSolveStatus::Changed,
    );
    drop(frame);
    let read = domain
        .prepare_managed_call(
            &realized,
            &[ManagedCallAccessRequest::new(
                &output,
                output_object,
                MemoryAccessMode::Read,
                MemoryAccessRegion::WholeInitialized,
            )],
        )
        .unwrap();
    assert_eq!(
        domain
            .acquire_call(&realized, &read)
            .unwrap()
            .with_port_slice(&output, |values| values[0])
            .unwrap(),
        42,
    );
}

#[test]
fn low_level_matrix_object_remapping_preserves_admission_and_reclamation() {
    struct Sum {
        input: ManagedPort<f64>,
        output: ManagedPort<f64>,
    }

    impl MechFunctionImpl for Sum {
        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            _: &mut dyn MechExecutionServices,
        ) -> mech_core::MResult<ReactiveSolveStatus> {
            let sum = frame.with_port_slice(&self.input, |values| values.iter().sum::<f64>())?;
            frame.with_port_init_writer(&self.output, |writer| writer.write_next(sum))?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn to_string(&self) -> String {
            "r6 low-level sum kernel".into()
        }
    }

    let matrix = ValueCell::dynamic_matrix(
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        vec![1, 2].into_boxed_slice(),
        vec![
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(1.0)),
            mech_core::ValueDataDraft::F64(F64Bits::from_f64(2.0)),
        ]
        .into_boxed_slice(),
    )
    .unwrap();
    let matrix_clone = matrix.clone();
    let output_cell = ValueCell::from_exact(0.0_f64).unwrap();
    let invocation = FunctionInvocation::unary(output_cell, matrix.clone());
    let input = invocation
        .input(0)
        .unwrap()
        .try_managed_matrix::<f64>()
        .unwrap();
    let output = invocation.output().try_managed::<f64>().unwrap();
    let function = Sum {
        input: input.clone(),
        output: output.clone(),
    };

    let domain = MemoryDomain::new().unwrap();
    let initial_revision = domain.issue_plan_revision().unwrap();
    let initial_allocations = [
        f64_allocation(0, 0, 0, 16, 16, MemoryLifetime::Activation),
        f64_allocation(1, 0, 16, 0, 8, MemoryLifetime::Activation),
    ];
    let initial_arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 24, &[0, 1])];
    let initial = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    initial_revision,
                    &initial_allocations,
                    &initial_arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let initial_input = domain
        .plan_object_key(initial_revision, MemoryObjectId::new(0))
        .unwrap();
    let initial_output = domain
        .plan_object_key(initial_revision, MemoryObjectId::new(1))
        .unwrap();
    initialize_managed_f64(&domain, &initial, &input, initial_input, &[1.0, 2.0]);
    assert_eq!(
        execute_sum(
            &domain,
            &initial,
            &function,
            &input,
            initial_input,
            &output,
            initial_output,
        ),
        3.0,
    );
    let initial_handle = initial.binding(initial_input).unwrap().handle().unwrap();

    let grown_revision = domain.issue_plan_revision().unwrap();
    let grown_allocations = [
        f64_allocation(0, 1, 0, 32, 32, MemoryLifetime::Activation),
        f64_allocation(1, 1, 32, 0, 8, MemoryLifetime::Activation),
    ];
    let grown_arenas = [arena(1, ArenaBackingKind::ContiguousBytes, 40, &[0, 1])];
    let grown = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    grown_revision,
                    &grown_allocations,
                    &grown_arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    domain.activate_realization(&grown).unwrap();
    let grown_input = domain
        .plan_object_key(grown_revision, MemoryObjectId::new(0))
        .unwrap();
    let grown_output = domain
        .plan_object_key(grown_revision, MemoryObjectId::new(1))
        .unwrap();
    let grown_handle = grown.binding(grown_input).unwrap().handle().unwrap();
    assert_ne!(initial_handle, grown_handle);
    initialize_managed_f64(&domain, &grown, &input, grown_input, &[1.0, 2.0, 3.0, 4.0]);
    let replacement = ValueCell::dynamic_matrix(
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        vec![2, 2].into_boxed_slice(),
        [1.0, 2.0, 3.0, 4.0]
            .into_iter()
            .map(|value| mech_core::ValueDataDraft::F64(F64Bits::from_f64(value)))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
    .unwrap()
    .snapshot()
    .unwrap();
    matrix.replace(&replacement).unwrap();
    assert_eq!(matrix_clone.shape().parameter_values(), &[2, 2]);
    assert_eq!(
        execute_sum(
            &domain,
            &grown,
            &function,
            &input,
            grown_input,
            &output,
            grown_output,
        ),
        10.0,
    );

    domain.retire(initial_handle).unwrap();
    let initial_output_handle = initial.binding(initial_output).unwrap().handle().unwrap();
    if initial_output_handle != initial_handle {
        domain.retire(initial_output_handle).unwrap();
    }
    assert_eq!(domain.collect_retired().unwrap(), 0);
    drop(initial);
    assert_eq!(domain.collect_retired().unwrap(), 1);

    let before_failed_growth = matrix_clone.snapshot().unwrap();
    let rejected_revision = domain.issue_plan_revision().unwrap();
    let rejected_allocations = [allocation(
        0,
        2,
        0,
        80,
        80,
        MemoryLifetime::Activation,
        None,
    )];
    let rejected_arenas = [arena(2, ArenaBackingKind::ContiguousBytes, 80, &[0])];
    let limits = MemoryBudgetLimits {
        max_storage_buffer_bytes: Some(64),
        ..MemoryBudgetLimits::default()
    };
    assert!(matches!(
        domain.prepare_realization(runtime_plan_view(
            rejected_revision,
            &rejected_allocations,
            &rejected_arenas,
            ResourceDemand::default(),
            limits,
            &[],
        )),
        Err(MemoryRuntimeError::BudgetExceeded { .. })
    ));
    assert_eq!(
        matrix_clone
            .snapshot()
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        before_failed_growth.canonical_data_draft().unwrap(),
    );
}

fn initialize_managed_f64(
    domain: &MemoryDomain,
    realized: &mech_core::RealizedMemoryPlan,
    port: &ManagedPort<f64>,
    object: mech_core::PlanObjectKey,
    values: &[f64],
) {
    let bytes = (values.len() * core::mem::size_of::<f64>()) as u64;
    let prepared = domain
        .prepare_managed_call(
            realized,
            &[ManagedCallAccessRequest::new(
                port,
                object,
                MemoryAccessMode::Write,
                MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: bytes,
                },
            )],
        )
        .unwrap();
    domain
        .acquire_call(realized, &prepared)
        .unwrap()
        .with_port_init_writer(port, |writer| writer.copy_from_slice(values))
        .unwrap();
}

fn execute_sum(
    domain: &MemoryDomain,
    realized: &mech_core::RealizedMemoryPlan,
    function: &impl MechFunctionImpl,
    input: &ManagedPort<f64>,
    input_object: mech_core::PlanObjectKey,
    output: &ManagedPort<f64>,
    output_object: mech_core::PlanObjectKey,
) -> f64 {
    let prepared = domain
        .prepare_managed_call(
            realized,
            &[
                ManagedCallAccessRequest::new(
                    input,
                    input_object,
                    MemoryAccessMode::Read,
                    MemoryAccessRegion::WholeInitialized,
                ),
                ManagedCallAccessRequest::new(
                    output,
                    output_object,
                    MemoryAccessMode::Write,
                    MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                ),
            ],
        )
        .unwrap();
    let mut frame = domain.acquire_call(realized, &prepared).unwrap();
    function
        .solve_managed(&mut frame, &mut NoMechExecutionServices)
        .unwrap();
    drop(frame);
    let read = domain
        .prepare_managed_call(
            realized,
            &[ManagedCallAccessRequest::new(
                output,
                output_object,
                MemoryAccessMode::Read,
                MemoryAccessRegion::WholeInitialized,
            )],
        )
        .unwrap();
    domain
        .acquire_call(realized, &read)
        .unwrap()
        .with_port_slice(output, |values| values[0])
        .unwrap()
}

#[test]
fn lease_acquisition_is_atomic_and_region_aware() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None),
        allocation(1, 0, 8, 0, 8, MemoryLifetime::Activation, None),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0, 1])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let left = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let right = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    for key in [left, right] {
        let write = domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object: key,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                }],
            )
            .unwrap();
        domain
            .acquire_call(&realized, &write)
            .unwrap()
            .with_object_init_writer::<u8>(key, |writer| {
                for _ in 0..8 {
                    writer.write_next(key.object().get() as u8)?;
                }
                Ok::<(), MemoryRuntimeError>(())
            })
            .unwrap();
    }

    let left_read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: left,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let reader_one = domain.acquire_call(&realized, &left_read).unwrap();
    let left_read_two = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: left,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let reader_two = domain.acquire_call(&realized, &left_read_two).unwrap();
    let left_write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: left,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    assert!(matches!(
        domain.acquire_call(&realized, &left_write),
        Err(MemoryRuntimeError::BorrowConflict { .. })
    ));
    let right_write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: right,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    assert!(domain.acquire_call(&realized, &right_write).is_ok());
    drop(reader_two);
    drop(reader_one);
    assert!(domain.acquire_call(&realized, &left_write).is_ok());

    let gap = MemoryAccessRegion::Contiguous {
        offset_bytes: 4,
        length_bytes: 4,
    };
    let empty_domain = MemoryDomain::new().unwrap();
    let empty_revision = empty_domain.issue_plan_revision().unwrap();
    let empty_realized = empty_domain
        .materialize(
            empty_domain
                .prepare_realization(runtime_plan_view(
                    empty_revision,
                    &[allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)],
                    &[arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let empty_key = empty_domain
        .plan_object_key(empty_revision, MemoryObjectId::new(0))
        .unwrap();
    let gap_write = empty_domain
        .prepare_call(
            &empty_realized,
            &[CallAccessRequest {
                object: empty_key,
                mode: MemoryAccessMode::Write,
                region: gap,
            }],
        )
        .unwrap();
    let mut frame = empty_domain
        .acquire_call(&empty_realized, &gap_write)
        .unwrap();
    assert!(matches!(
        frame.with_bytes_mut(empty_key, |_| ()),
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));
}

#[test]
fn overlapping_regions_require_a_disjoint_lifetime_reuse_group() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(
            0,
            0,
            0,
            0,
            32,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(1),
            },
            Some(3),
        ),
        allocation(
            1,
            0,
            0,
            0,
            32,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(2),
                last: MemoryPlanPoint::new(3),
            },
            Some(3),
        ),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 32, &[0, 1])];
    assert!(
        domain
            .prepare_realization(runtime_plan_view(
                revision,
                &allocations,
                &arenas,
                ResourceDemand::default(),
                MemoryBudgetLimits::default(),
                &[],
            ))
            .is_ok()
    );

    let invalid_domain = MemoryDomain::new().unwrap();
    let invalid_revision = invalid_domain.issue_plan_revision().unwrap();
    let mut invalid = allocations;
    invalid[1].lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(1),
        last: MemoryPlanPoint::new(2),
    };
    assert!(matches!(
        invalid_domain.prepare_realization(runtime_plan_view(
            invalid_revision,
            &invalid,
            &arenas,
            ResourceDemand::default(),
            MemoryBudgetLimits::default(),
            &[],
        )),
        Err(MemoryRuntimeError::InvalidReuse { .. })
    ));
}

#[test]
fn reused_regions_are_leaseable_only_during_their_declared_plan_interval() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(
            0,
            0,
            0,
            0,
            16,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(1),
            },
            Some(7),
        ),
        allocation(
            1,
            0,
            0,
            0,
            16,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(2),
                last: MemoryPlanPoint::new(3),
            },
            Some(7),
        ),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0, 1])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let first = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let second = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    assert_eq!(
        realized.binding(first).unwrap().handle(),
        realized.binding(second).unwrap().handle()
    );

    let prepare_write = |object| {
        domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 16,
                    },
                }],
            )
            .unwrap()
    };
    let first_write = prepare_write(first);
    let second_write = prepare_write(second);

    assert!(matches!(
        domain.acquire_call(&realized, &first_write),
        Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
    ));
    {
        let scope = domain.enter_plan_point(MemoryPlanPoint::new(0)).unwrap();
        assert_eq!(scope.point(), MemoryPlanPoint::new(0));
        domain
            .acquire_call(&realized, &first_write)
            .unwrap()
            .with_object_init_writer::<u8>(first, |writer| {
                for _ in 0..16 {
                    writer.write_next(11)?;
                }
                Ok::<(), MemoryRuntimeError>(())
            })
            .unwrap();
        assert!(matches!(
            domain.acquire_call(&realized, &second_write),
            Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
        ));
        assert!(matches!(
            domain.enter_plan_point(MemoryPlanPoint::new(1)),
            Err(MemoryRuntimeError::TurnInFlight)
        ));
    }
    {
        let first_read = domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object: first,
                    mode: MemoryAccessMode::Read,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 16,
                    },
                }],
            )
            .unwrap();
        let _scope = domain.enter_plan_point(MemoryPlanPoint::new(1)).unwrap();
        let frame = domain.acquire_call(&realized, &first_read).unwrap();
        frame
            .with_bytes(first, |bytes| assert!(bytes.iter().all(|byte| *byte == 11)))
            .unwrap();
    }
    {
        let _scope = domain.enter_plan_point(MemoryPlanPoint::new(2)).unwrap();
        assert!(matches!(
            domain.acquire_call(&realized, &first_write),
            Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
        ));
        domain
            .acquire_call(&realized, &second_write)
            .unwrap()
            .with_object_init_writer::<u8>(second, |writer| {
                for _ in 0..16 {
                    writer.write_next(22)?;
                }
                Ok::<(), MemoryRuntimeError>(())
            })
            .unwrap();
    }
    {
        let _scope = domain.enter_plan_point(MemoryPlanPoint::new(0)).unwrap();
        assert!(matches!(
            domain.prepare_call(
                &realized,
                &[CallAccessRequest {
                    object: first,
                    mode: MemoryAccessMode::Read,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 16,
                    },
                }],
            ),
            Err(MemoryRuntimeError::UninitializedAccess { .. })
        ));
    }
}

#[test]
fn retired_allocations_wait_for_held_leases_before_reclamation() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &[allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)],
                    &[arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            }],
        )
        .unwrap();
    domain
        .acquire_call(&realized, &write)
        .unwrap()
        .with_object_init_writer::<u8>(object, |writer| {
            writer.copy_from_slice(&23_u64.to_ne_bytes())
        })
        .unwrap();
    let read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let held = domain.acquire_call(&realized, &read).unwrap();
    let handle = realized.binding(object).unwrap().handle().unwrap();
    domain.retire(handle).unwrap();
    assert_eq!(domain.collect_retired().unwrap(), 0);
    let read_after_retire = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    assert!(matches!(
        domain.acquire_call(&realized, &read_after_retire),
        Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
    ));
    assert_eq!(
        held.with_bytes(object, |bytes| u64::from_ne_bytes(
            bytes.try_into().unwrap()
        ))
        .unwrap(),
        23
    );
    drop(held);
    // A live realization is retained storage ownership, independently of
    // whether any access lease is currently installed.
    assert_eq!(domain.collect_retired().unwrap(), 0);
    assert!(matches!(
        domain.acquire_call(&realized, &read),
        Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
    ));
    drop(realized);
    assert_eq!(domain.collect_retired().unwrap(), 1);
    assert!(matches!(
        domain.retire(handle),
        Err(MemoryRuntimeError::StaleAllocationGeneration { .. })
    ));
}

#[test]
fn device_registration_and_submission_holds_follow_planned_ownership() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let mut device = allocation(0, 0, 0, 16, 16, MemoryLifetime::Activation, None);
    device.space = MemorySpace::Device { region: 0 };
    let transfer_lifetime = MemoryLifetime::Transfer {
        first: MemoryPlanPoint::new(0),
        last: MemoryPlanPoint::new(0),
    };
    let mut transfer = allocation(1, 1, 0, 0, 8, transfer_lifetime, None);
    transfer.role = AllocationRole::TransferStage;
    let mut device_arena = arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0]);
    device_arena.space = MemorySpace::Device { region: 0 };
    let transfer_arena = arena(1, ArenaBackingKind::ContiguousBytes, 8, &[1]);
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &[device, transfer],
                    &[device_arena, transfer_arena],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let device_object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let transfer_object = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    assert!(matches!(
        realized.binding(device_object).unwrap(),
        RuntimeBinding::Device { .. }
    ));
    assert_eq!(domain.allocation_observations()[0].actual_block_bytes, 0);

    let owner = domain
        .register_device_allocation(&realized, device_object, 16, 16)
        .unwrap();
    let observation = domain.allocation_observations()[0];
    assert_eq!(observation.actual_block_bytes, 16);
    assert_eq!(observation.device_owner_pins, 1);
    let prepared = domain
        .prepare_device_submission(&realized, &[device_object], &[transfer_object])
        .unwrap();
    let hold = domain
        .begin_prepared_device_submission(&realized, &prepared)
        .unwrap();
    assert_eq!(domain.ledger().in_flight_device_bytes, 16);
    assert_eq!(domain.ledger().in_flight_transfer_bytes, 8);
    assert_eq!(domain.allocation_observations()[0].submission_pins, 1);

    hold.complete().unwrap();
    assert_eq!(domain.ledger().in_flight_device_bytes, 0);
    assert_eq!(domain.ledger().in_flight_transfer_bytes, 0);
    domain
        .begin_prepared_device_submission(&realized, &prepared)
        .unwrap()
        .complete()
        .unwrap();
    domain.retire(owner.handle()).unwrap();
    assert_eq!(domain.collect_retired().unwrap(), 0);
    drop(owner);
    assert_eq!(domain.collect_retired().unwrap(), 0);
    drop(realized);
    assert_eq!(domain.collect_retired().unwrap(), 2);
}

#[test]
fn device_loss_prevents_new_submissions() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let mut allocation = allocation(0, 0, 0, 4, 4, MemoryLifetime::Activation, None);
    allocation.space = MemorySpace::Device { region: 2 };
    allocation.alignment = 4;
    let mut arena = arena(0, ArenaBackingKind::ContiguousBytes, 4, &[0]);
    arena.space = MemorySpace::Device { region: 2 };
    arena.alignment = 4;
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &[allocation],
                    &[arena],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let owner = domain
        .register_device_allocation(&realized, object, 4, 4)
        .unwrap();
    owner.mark_lost().unwrap();
    assert!(matches!(
        domain.begin_device_submission(&realized, &[object], &[]),
        Err(MemoryRuntimeError::DeviceLost { .. })
    ));
    assert!(domain.allocation_observations()[0].device_lost);
}

#[test]
fn handles_are_domain_scoped_and_stale_generations_are_rejected() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let first = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let old = first.binding(key).unwrap().handle().unwrap();
    let other = MemoryDomain::new().unwrap();
    assert!(matches!(
        other.retire(old),
        Err(MemoryRuntimeError::WrongMemoryDomain { .. })
    ));
    domain.retire(old).unwrap();
    assert_eq!(domain.collect_retired().unwrap(), 0);
    drop(first);
    assert_eq!(domain.collect_retired().unwrap(), 1);

    let next_revision = domain.issue_plan_revision().unwrap();
    let second = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    next_revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let next_key = domain
        .plan_object_key(next_revision, MemoryObjectId::new(0))
        .unwrap();
    let current = second.binding(next_key).unwrap().handle().unwrap();
    assert_eq!(old.slot(), current.slot());
    assert_ne!(old.generation(), current.generation());
    assert!(matches!(
        domain.retire(old),
        Err(MemoryRuntimeError::StaleAllocationGeneration { .. })
    ));
}

#[test]
fn indirect_payload_envelopes_are_owned_separately_and_charged_once() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let mut left = allocation(0, 0, 0, 0, 11, MemoryLifetime::Activation, None);
    left.role = AllocationRole::VariablePayload;
    left.slot = None;
    left.payload_block_capacity = 1;
    left.alignment = 1;
    let mut right = allocation(1, 0, 0, 0, 13, MemoryLifetime::Activation, None);
    right.role = AllocationRole::VariablePayload;
    right.slot = None;
    right.payload_block_capacity = 1;
    right.alignment = 1;
    let allocations = [left, right];
    let mut payload_arena = arena(0, ArenaBackingKind::IndirectOwnedPayloads, 24, &[0, 1]);
    payload_arena.alignment = 1;
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[payload_arena],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    assert_eq!(realized.bindings().len(), 2);
    assert_eq!(domain.ledger().committed_bytes, 72);
    assert_eq!(domain.allocation_observations().len(), 2);

    let left_key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let right_key = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    let left_allocator = domain.planned_allocator(&realized, left_key).unwrap();
    let right_allocator = domain.planned_allocator(&realized, right_key).unwrap();
    let string = ManagedString::try_new(left_allocator.clone(), "hello world").unwrap();
    let sequence =
        ManagedSequence::try_from_slice(right_allocator.clone(), b"hello, world!").unwrap();
    assert_eq!(string.as_str(), "hello world");
    assert_eq!(sequence.as_slice(), b"hello, world!");
    assert_eq!(left_allocator.allocated_bytes().unwrap(), 11);
    assert_eq!(right_allocator.allocated_bytes().unwrap(), 13);
    assert_eq!(domain.ledger().committed_bytes, 72);
    assert!(matches!(
        ManagedString::try_new(left_allocator.clone(), "x"),
        Err(MemoryRuntimeError::CapacityExceeded { .. })
    ));
    let observations = domain.allocation_observations();
    assert_eq!(observations[0].actual_block_bytes, 11);
    assert_eq!(observations[1].actual_block_bytes, 13);
    assert_eq!(observations[0].payload_owner_pins, 1);

    let left_handle = realized.binding(left_key).unwrap().handle().unwrap();
    domain.retire(left_handle).unwrap();
    assert_eq!(domain.collect_retired().unwrap(), 0);
    drop(string);
    drop(left_allocator);
    assert_eq!(domain.collect_retired().unwrap(), 0);
    drop(realized);
    assert_eq!(domain.collect_retired().unwrap(), 1);
}

#[test]
fn detached_values_share_one_sendable_immutable_payload_root() {
    let value = ValueCell::from_exact(37_u64).unwrap().snapshot().unwrap();
    let clone = value.clone();
    assert!(value.shares_frozen_storage(&clone));
    std::thread::spawn(move || drop(clone)).join().unwrap();
}

#[test]
fn ordinary_dynamic_cell_snapshots_retain_the_frozen_root_after_close() {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Tuple(vec![SchemaBody::Dynamic].into_boxed_slice()),
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Tuple(vec![ValueDataDraft::Dynamic(None)].into_boxed_slice()),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let domain = MemoryDomain::new().unwrap();
    let cell = ValueCell::from_snapshot_in(&domain, value.clone()).unwrap();
    let first = cell.snapshot().unwrap();
    let second = cell.snapshot().unwrap();
    assert!(value.shares_frozen_storage(&first));
    assert!(first.shares_frozen_storage(&second));

    drop(cell);
    domain.close().unwrap();
    assert!(matches!(first.data(), mech_core::ValueData::Tuple(_)));
    assert!(first.shares_frozen_storage(&second));
}

#[test]
fn reclaimed_realizations_release_historical_region_and_revision_metadata() {
    const CAPACITY: u64 = 4 * 1024 * 1024;

    let domain = MemoryDomain::new().unwrap();
    let mut stale_key = None;
    for iteration in 0..12_u64 {
        let revision = domain.issue_plan_revision().unwrap();
        let capacity = CAPACITY + (iteration % 3) * 64;
        let allocations = [allocation(
            0,
            0,
            0,
            1,
            capacity,
            MemoryLifetime::Activation,
            None,
        )];
        let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, capacity, &[0])];
        let realized = domain
            .materialize(
                domain
                    .prepare_realization(runtime_plan_view(
                        revision,
                        &allocations,
                        &arenas,
                        ResourceDemand::default(),
                        MemoryBudgetLimits::default(),
                        &[],
                    ))
                    .unwrap(),
            )
            .unwrap();
        let key = domain
            .plan_object_key(revision, MemoryObjectId::new(0))
            .unwrap();
        let write = domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object: key,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 1,
                    },
                }],
            )
            .unwrap();
        domain
            .acquire_call(&realized, &write)
            .unwrap()
            .with_object_init_writer::<u8>(key, |writer| writer.write_next(7))
            .unwrap();
        stale_key.get_or_insert(key);

        let retained = (iteration == 0).then(|| realized.clone());
        drop(realized);
        if let Some(retained) = retained {
            assert_eq!(domain.collect_retired().unwrap(), 0);
            let metadata = domain.metadata_observation();
            assert_eq!(metadata.regions, 1);
            assert!(metadata.initialization_bytes >= CAPACITY / 8);
            drop(retained);
        }
        assert_eq!(domain.collect_retired().unwrap(), 1);
        assert_eq!(domain.metadata_observation(), Default::default());
        assert_eq!(domain.ledger().committed_bytes, 0);
    }

    let rejected = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let limits = MemoryBudgetLimits {
        max_temporary_bytes: Some(1),
        ..MemoryBudgetLimits::default()
    };
    assert!(
        domain
            .prepare_realization(runtime_plan_view(
                rejected,
                &allocations,
                &arenas,
                ResourceDemand {
                    turn_peak_bytes: 2,
                    ..ResourceDemand::default()
                },
                limits,
                &[],
            ))
            .is_err()
    );
    domain.collect_retired().unwrap();
    assert_eq!(domain.metadata_observation(), Default::default());

    let revision = domain.issue_plan_revision().unwrap();
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    assert!(matches!(
        domain.prepare_call(
            &realized,
            &[CallAccessRequest {
                object: stale_key.unwrap(),
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        ),
        Err(MemoryRuntimeError::InvalidPlanRevision { .. })
            | Err(MemoryRuntimeError::UnknownPlanObject { .. })
    ));
}

#[test]
fn immutable_snapshot_imports_share_one_physical_payload_ticket() {
    let source_cell = ValueCell::from_exact("x".repeat(256 * 1024)).unwrap();
    let source = source_cell.snapshot().unwrap();
    let left_domain = MemoryDomain::new().unwrap();
    let right_domain = MemoryDomain::new().unwrap();

    let left = ValueCell::from_snapshot_in(&left_domain, source.clone()).unwrap();
    let right = ValueCell::from_snapshot_in(&right_domain, source.clone()).unwrap();
    let left_snapshot = left.snapshot().unwrap();
    let right_snapshot = right.snapshot().unwrap();

    assert!(source.shares_frozen_storage(&left_snapshot));
    assert!(left_snapshot.shares_frozen_storage(&right_snapshot));
    assert!(source.shares_retained_payload_ticket(&left_snapshot));
    assert!(left_snapshot.shares_retained_payload_ticket(&right_snapshot));
    assert!(left_domain.ledger().committed_bytes > 0);
    assert!(right_domain.ledger().committed_bytes > 0);

    let deep_copy = ValueCell::from_exact("x".repeat(256 * 1024))
        .unwrap()
        .snapshot()
        .unwrap();
    assert!(!source.shares_frozen_storage(&deep_copy));
    assert!(!source.shares_retained_payload_ticket(&deep_copy));

    drop(source_cell);
    drop(source);
    left_domain.close().unwrap();
    right_domain.close().unwrap();
    drop(left);
    drop(right);
    assert!(matches!(
        left_snapshot.data(),
        mech_core::ValueData::String(value) if value.len() == 256 * 1024
    ));
    assert!(left_snapshot.shares_retained_payload_ticket(&right_snapshot));
}

#[test]
fn publication_versions_change_only_for_changed_candidates() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let binding = realized.binding(key).unwrap();
    assert!(matches!(binding, RuntimeBinding::ManagedHostRegion { .. }));
    let mut unchanged = domain
        .prepare_publication(
            &realized,
            vec![PublicationCandidate {
                object: key,
                binding: binding.clone(),
                shape: vec![1].into_boxed_slice(),
                changed: false,
            }],
        )
        .unwrap();
    let first = domain.commit_publication(&mut unchanged).unwrap();
    assert_eq!(first[0].version.get(), 1);
    assert!(matches!(
        domain.commit_publication(&mut unchanged),
        Err(MemoryRuntimeError::PublicationAlreadyCompleted)
    ));

    let mut changed = domain
        .prepare_publication(
            &realized,
            vec![PublicationCandidate {
                object: key,
                binding,
                shape: vec![1].into_boxed_slice(),
                changed: true,
            }],
        )
        .unwrap();
    let second = domain.commit_publication(&mut changed).unwrap();
    assert_eq!(second[0].version.get(), 2);
    let mut third = domain
        .prepare_publication(
            &realized,
            vec![PublicationCandidate {
                object: key,
                binding: second[0].binding.clone(),
                shape: vec![1].into_boxed_slice(),
                changed: true,
            }],
        )
        .unwrap();
    assert_eq!(
        domain.commit_publication(&mut third).unwrap()[0]
            .version
            .get(),
        3
    );
}

#[test]
fn multi_cell_publication_rejects_late_conflict_before_any_value_changes() {
    let domain = MemoryDomain::new().unwrap();
    let left = ValueCell::from_exact_in(&domain, 1_u64).unwrap();
    let right = ValueCell::from_exact_in(&domain, 2_u64).unwrap();
    let left_alias = left.clone();
    let right_alias = right.clone();
    let left_before_version = left.published_version();
    let right_before_version = right.published_version();
    let left_next = left
        .rebuild_data_draft(mech_core::ValueDataDraft::U64(10))
        .unwrap();
    let right_next = right
        .rebuild_data_draft(mech_core::ValueDataDraft::U64(20))
        .unwrap();

    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None),
        allocation(1, 0, 8, 0, 8, MemoryLifetime::Activation, None),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0, 1])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let left_object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let right_object = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    domain.activate_realization(&realized).unwrap();
    let initialization = domain
        .prepare_call(
            &realized,
            &[
                CallAccessRequest {
                    object: left_object,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                },
                CallAccessRequest {
                    object: right_object,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                },
            ],
        )
        .unwrap();
    {
        let mut frame = domain.acquire_call(&realized, &initialization).unwrap();
        frame
            .with_object_init_writer::<u64>(left_object, |writer| writer.write_next(10))
            .unwrap();
        frame
            .with_object_init_writer::<u64>(right_object, |writer| writer.write_next(20))
            .unwrap();
    }
    let prepared = domain
        .prepare_cell_publication(
            &realized,
            vec![
                CellPublicationCandidate {
                    cell: left.clone(),
                    object: left_object,
                    binding: realized.binding(left_object).unwrap(),
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                    evidence: CellPublicationEvidence::initialized_region(
                        left_next.shape().clone(),
                    ),
                    changed: true,
                },
                CellPublicationCandidate {
                    cell: right.clone(),
                    object: right_object,
                    binding: realized.binding(right_object).unwrap(),
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                    evidence: CellPublicationEvidence::initialized_region(
                        right_next.shape().clone(),
                    ),
                    changed: true,
                },
            ],
        )
        .unwrap();
    let held_shape = right.shape();
    assert!(domain.ready_cell_publication(prepared).is_err());
    drop(held_shape);
    assert_eq!(u64_cell(&left_alias), 1);
    assert_eq!(u64_cell(&right_alias), 2);
    assert_eq!(left.published_version(), left_before_version);
    assert_eq!(right.published_version(), right_before_version);
    let prepared = domain
        .prepare_cell_publication(
            &realized,
            vec![
                CellPublicationCandidate {
                    cell: left.clone(),
                    object: left_object,
                    binding: realized.binding(left_object).unwrap(),
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                    evidence: CellPublicationEvidence::initialized_region(left.shape().clone()),
                    changed: true,
                },
                CellPublicationCandidate {
                    cell: right.clone(),
                    object: right_object,
                    binding: realized.binding(right_object).unwrap(),
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                    evidence: CellPublicationEvidence::initialized_region(right.shape().clone()),
                    changed: true,
                },
            ],
        )
        .unwrap();
    let ready = domain.ready_cell_publication(prepared).unwrap();
    let shape_held_after_ready = right.shape();
    let committed = ready.commit();
    assert!(shape_held_after_ready.parameter_values().is_empty());
    drop(shape_held_after_ready);
    assert_eq!(committed.len(), 2);
    assert_eq!(u64_cell(&left_alias), 10);
    assert_eq!(u64_cell(&right_alias), 20);
    assert!(left.published_version() > left_before_version);
    assert!(right.published_version() > right_before_version);
}

#[test]
fn sibling_call_candidates_in_one_session_publish_as_one_atomic_batch() {
    fn stage(domain: &MemoryDomain, cell: &ValueCell, value: u64) -> PreparedCellPublication {
        let revision = domain.issue_plan_revision().unwrap();
        let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
        let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
        let realized = domain
            .materialize(
                domain
                    .prepare_realization(runtime_plan_view(
                        revision,
                        &allocations,
                        &arenas,
                        ResourceDemand::default(),
                        MemoryBudgetLimits::default(),
                        &[],
                    ))
                    .unwrap(),
            )
            .unwrap();
        let object = domain
            .plan_object_key(revision, MemoryObjectId::new(0))
            .unwrap();
        domain.activate_realization(&realized).unwrap();
        let scope = domain.enter_plan_point(MemoryPlanPoint::new(0)).unwrap();
        let initialization = domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                }],
            )
            .unwrap();
        domain
            .acquire_call(&realized, &initialization)
            .unwrap()
            .with_object_init_writer::<u64>(object, |writer| writer.write_next(value))
            .unwrap();
        let publication = domain
            .prepare_cell_publication(
                &realized,
                vec![CellPublicationCandidate {
                    cell: cell.clone(),
                    object,
                    binding: realized.binding(object).unwrap(),
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                    evidence: CellPublicationEvidence::initialized_region(cell.shape().clone()),
                    changed: true,
                }],
            )
            .unwrap();
        drop(scope);
        publication
    }

    let domain = MemoryDomain::new().unwrap();
    let left = ValueCell::from_exact_in(&domain, 1_u64).unwrap();
    let right = ValueCell::from_exact_in(&domain, 2_u64).unwrap();
    let left_version = left.published_version();
    let right_version = right.published_version();
    let left_candidate = stage(&domain, &left, 10);
    let right_candidate = stage(&domain, &right, 20);

    // Staging owns both candidates but exposes neither of them.
    assert_eq!(u64_cell(&left), 1);
    assert_eq!(u64_cell(&right), 2);
    assert_eq!(left.published_version(), left_version);
    assert_eq!(right.published_version(), right_version);

    PreparedCellPublicationBatch::new(vec![left_candidate, right_candidate])
        .unwrap()
        .ready()
        .unwrap()
        .commit();
    assert_eq!(u64_cell(&left), 10);
    assert_eq!(u64_cell(&right), 20);
    assert!(left.published_version() > left_version);
    assert!(right.published_version() > right_version);
}

fn u64_cell(cell: &ValueCell) -> u64 {
    let snapshot = cell.snapshot().unwrap();
    let mech_core::ValueData::U64(value) = snapshot.data() else {
        panic!("expected U64 cell")
    };
    *value
}

#[test]
fn resident_projection_uses_the_realized_arena_without_a_second_allocation() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(
        0,
        0,
        0,
        16,
        16,
        MemoryLifetime::Activation,
        None,
    )];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0])];
    let reservation = domain
        .prepare_realization(runtime_plan_view(
            revision,
            &allocations,
            &arenas,
            ResourceDemand {
                activation_bytes: 16,
                ..ResourceDemand::default()
            },
            MemoryBudgetLimits::default(),
            &[],
        ))
        .unwrap();
    let realized = domain.materialize(reservation).unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let mut storage = domain
        .project_host_arena::<u64>(&realized, object, 2)
        .unwrap();
    let managed_write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 16,
                },
            }],
        )
        .unwrap();
    assert!(matches!(
        domain.acquire_call(&realized, &managed_write),
        Err(MemoryRuntimeError::BorrowConflict { .. })
    ));
    assert!(
        domain
            .project_host_arena::<u64>(&realized, object, 2)
            .is_err(),
        "one physical arena cannot acquire two simultaneous typed owners",
    );
    let committed = domain.ledger().committed_bytes;
    storage[0] = 11;
    storage[1] = 22;
    assert_eq!(&*storage, &[11, 22]);
    assert_eq!(domain.ledger().committed_bytes, committed);
    drop(storage);
    let held = domain.acquire_call(&realized, &managed_write).unwrap();
    assert!(matches!(
        domain.project_host_arena::<u64>(&realized, object, 2),
        Err(MemoryRuntimeError::BorrowConflict { .. })
    ));
    drop(held);
    assert!(
        domain
            .project_host_arena::<u64>(&realized, object, 2)
            .is_ok(),
        "dropping the typed projection releases only its claim, not the arena",
    );
}
