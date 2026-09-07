#![cfg(feature = "full")]

use mech_core::snapshot::F64Bits;
use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, CallAccessRequest,
    CellPublicationCandidate, FunctionInvocation, KernelMemoryFrame, ManagedCallAccessRequest,
    ManagedFunctionInstance, ManagedMechFunctionImpl, ManagedPort, ManagedSequence, ManagedString,
    MechExecutionServices, MemoryAccessMode, MemoryAccessRegion, MemoryArenaId, MemoryBudgetLimits,
    MemoryDomain, MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanPoint,
    MemoryRuntimeError, MemorySpace, NoMechExecutionServices, PublicationCandidate,
    ReactiveSolveStatus, ResourceDemand, ReuseGroupId, RuntimeBinding, RuntimePlanView, ValueCell,
};

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
        space: MemorySpace::Host,
        current_bytes: current,
        capacity_bytes: capacity,
        alignment: 8,
        lifetime,
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(arena),
            offset,
        },
        reuse_group: reuse_group.map(ReuseGroupId::new),
    }
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
            .prepare_realization(RuntimePlanView::new(
                revision,
                &allocations,
                &arenas,
                ResourceDemand::default(),
                MemoryBudgetLimits::default(),
                &[],
            ))
            .unwrap();
        assert_eq!(reservation.reserved_bytes(), 64);
        assert_eq!(domain.ledger().reserved_bytes, 64);
        assert_eq!(domain.ledger().active_reservations, 1);
    }
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
        .prepare_realization(RuntimePlanView::new(
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
        .with_bytes_mut(key, |bytes| bytes.copy_from_slice(&42_u64.to_ne_bytes()))
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
                .prepare_realization(RuntimePlanView::new(
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
                port,
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
        .with_port_slice_mut(port, |values| values[0] = 17)
        .unwrap();
    drop(frame);

    let prepared = domain
        .prepare_managed_call(
            &realized,
            &[ManagedCallAccessRequest::new(
                port,
                object,
                MemoryAccessMode::Read,
                MemoryAccessRegion::WholeInitialized,
            )],
        )
        .unwrap();
    let frame = domain.acquire_call(&realized, &prepared).unwrap();
    assert_eq!(frame.with_port_slice(port, |values| values[0]).unwrap(), 17);
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

    impl ManagedMechFunctionImpl for Double {
        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            _: &mut dyn MechExecutionServices,
        ) -> mech_core::MResult<ReactiveSolveStatus> {
            let value = frame.with_port_slice(self.input, |values| values[0])?;
            frame.with_port_slice_mut(self.output, |values| values[0] = value * 2)?;
            Ok(ReactiveSolveStatus::Changed)
        }
    }

    let input_cell = ValueCell::from_exact(0_u64).unwrap();
    let output_cell = ValueCell::from_exact(0_u64).unwrap();
    let invocation = FunctionInvocation::unary(output_cell, input_cell);
    let input = invocation.input(0).unwrap().try_managed::<u64>().unwrap();
    let output = invocation.output().try_managed::<u64>().unwrap();
    let function = ManagedFunctionInstance::new(Box::new(Double { input, output }), invocation);

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
                .prepare_realization(RuntimePlanView::new(
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
                input,
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
        .with_port_slice_mut(input, |values| values[0] = 21)
        .unwrap();
    let prepared = domain
        .prepare_managed_call(
            &realized,
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
                output,
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
            .with_port_slice(output, |values| values[0])
            .unwrap(),
        42,
    );
}

#[test]
fn bound_dynamic_matrix_consumer_follows_admitted_physical_growth() {
    struct Sum {
        input: ManagedPort<f64>,
        output: ManagedPort<f64>,
    }

    impl ManagedMechFunctionImpl for Sum {
        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            _: &mut dyn MechExecutionServices,
        ) -> mech_core::MResult<ReactiveSolveStatus> {
            let sum = frame.with_port_slice(self.input, |values| values.iter().sum::<f64>())?;
            frame.with_port_slice_mut(self.output, |values| values[0] = sum)?;
            Ok(ReactiveSolveStatus::Changed)
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
    let function = ManagedFunctionInstance::new(Box::new(Sum { input, output }), invocation);

    let domain = MemoryDomain::new().unwrap();
    let initial_revision = domain.issue_plan_revision().unwrap();
    let initial_allocations = [
        allocation(0, 0, 0, 16, 16, MemoryLifetime::Activation, None),
        allocation(1, 0, 16, 0, 8, MemoryLifetime::Activation, None),
    ];
    let initial_arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 24, &[0, 1])];
    let initial = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
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
    initialize_managed_f64(&domain, &initial, input, initial_input, &[1.0, 2.0]);
    assert_eq!(
        execute_sum(
            &domain,
            &initial,
            &function,
            input,
            initial_input,
            output,
            initial_output,
        ),
        3.0,
    );
    let initial_handle = initial.binding(initial_input).unwrap().handle().unwrap();

    let grown_revision = domain.issue_plan_revision().unwrap();
    let grown_allocations = [
        allocation(0, 1, 0, 32, 32, MemoryLifetime::Activation, None),
        allocation(1, 1, 32, 0, 8, MemoryLifetime::Activation, None),
    ];
    let grown_arenas = [arena(1, ArenaBackingKind::ContiguousBytes, 40, &[0, 1])];
    let grown = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
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
    let grown_input = domain
        .plan_object_key(grown_revision, MemoryObjectId::new(0))
        .unwrap();
    let grown_output = domain
        .plan_object_key(grown_revision, MemoryObjectId::new(1))
        .unwrap();
    let grown_handle = grown.binding(grown_input).unwrap().handle().unwrap();
    assert_ne!(initial_handle, grown_handle);
    initialize_managed_f64(&domain, &grown, input, grown_input, &[1.0, 2.0, 3.0, 4.0]);
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
            input,
            grown_input,
            output,
            grown_output,
        ),
        10.0,
    );

    domain.retire(initial_handle).unwrap();
    let initial_output_handle = initial.binding(initial_output).unwrap().handle().unwrap();
    if initial_output_handle != initial_handle {
        domain.retire(initial_output_handle).unwrap();
    }
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
        domain.prepare_realization(RuntimePlanView::new(
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
    port: ManagedPort<f64>,
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
        .with_port_slice_mut(port, |target| target.copy_from_slice(values))
        .unwrap();
}

fn execute_sum(
    domain: &MemoryDomain,
    realized: &mech_core::RealizedMemoryPlan,
    function: &ManagedFunctionInstance,
    input: ManagedPort<f64>,
    input_object: mech_core::PlanObjectKey,
    output: ManagedPort<f64>,
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
                .prepare_realization(RuntimePlanView::new(
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
            .with_bytes_mut(key, |bytes| bytes.fill(key.object().get() as u8))
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
    let reader_two = domain.acquire_call(&realized, &left_read).unwrap();
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
                .prepare_realization(RuntimePlanView::new(
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
    assert!(matches!(
        empty_domain.prepare_call(
            &empty_realized,
            &[CallAccessRequest {
                object: empty_key,
                mode: MemoryAccessMode::Write,
                region: gap,
            }],
        ),
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
            .prepare_realization(RuntimePlanView::new(
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
        invalid_domain.prepare_realization(RuntimePlanView::new(
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
                .prepare_realization(RuntimePlanView::new(
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
            .with_bytes_mut(first, |bytes| bytes.fill(11))
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
        let _scope = domain.enter_plan_point(MemoryPlanPoint::new(2)).unwrap();
        assert!(matches!(
            domain.acquire_call(&realized, &first_write),
            Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
        ));
        domain
            .acquire_call(&realized, &second_write)
            .unwrap()
            .with_bytes_mut(second, |bytes| bytes.fill(22))
            .unwrap();
    }
}

#[test]
fn retired_allocations_wait_for_held_leases_before_reclamation() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let realized = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
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
        .with_bytes_mut(object, |bytes| bytes.copy_from_slice(&23_u64.to_ne_bytes()))
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
    assert!(matches!(
        domain.acquire_call(&realized, &read),
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
    assert_eq!(domain.collect_retired().unwrap(), 1);
    assert!(matches!(
        domain.acquire_call(&realized, &read),
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
                .prepare_realization(RuntimePlanView::new(
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
    assert_eq!(domain.collect_retired().unwrap(), 1);
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
                .prepare_realization(RuntimePlanView::new(
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
                .prepare_realization(RuntimePlanView::new(
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
    assert_eq!(domain.collect_retired().unwrap(), 1);

    let next_revision = domain.issue_plan_revision().unwrap();
    let second = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
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
    left.alignment = 1;
    let mut right = allocation(1, 0, 0, 0, 13, MemoryLifetime::Activation, None);
    right.role = AllocationRole::VariablePayload;
    right.alignment = 1;
    let allocations = [left, right];
    let mut payload_arena = arena(0, ArenaBackingKind::IndirectOwnedPayloads, 24, &[0, 1]);
    payload_arena.alignment = 1;
    let realized = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
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
    assert_eq!(domain.ledger().committed_bytes, 24);
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
    assert_eq!(domain.ledger().committed_bytes, 24);
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
    assert_eq!(domain.collect_retired().unwrap(), 1);
}

#[test]
fn detached_payload_ticket_is_send_and_charges_shared_bytes_once() {
    let domain = MemoryDomain::new().unwrap();
    let ticket = domain.retain_payload_charge(37).unwrap();
    let clone = ticket.clone();
    assert_eq!(ticket.bytes(), 37);
    assert_eq!(domain.ledger().exported_snapshot_bytes, 37);
    drop(ticket);
    assert_eq!(domain.ledger().exported_snapshot_bytes, 37);
    std::thread::spawn(move || drop(clone)).join().unwrap();
    assert_eq!(domain.ledger().exported_snapshot_bytes, 0);
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
                .prepare_realization(RuntimePlanView::new(
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
fn multi_cell_publication_rolls_back_every_applied_value_on_late_conflict() {
    let left = ValueCell::from_exact(1_u64).unwrap();
    let right = ValueCell::from_exact(2_u64).unwrap();
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
                .prepare_realization(RuntimePlanView::new(
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
    let mut prepared = domain
        .prepare_cell_publication(
            &realized,
            vec![
                CellPublicationCandidate {
                    cell: left.clone(),
                    object: left_object,
                    binding: realized.binding(left_object).unwrap(),
                    value: left_next,
                    changed: true,
                },
                CellPublicationCandidate {
                    cell: right.clone(),
                    object: right_object,
                    binding: realized.binding(right_object).unwrap(),
                    value: right_next,
                    changed: true,
                },
            ],
        )
        .unwrap();
    let held_shape = right.shape();
    assert!(domain.commit_cell_publication(&mut prepared).is_err());
    drop(held_shape);
    assert_eq!(u64_cell(&left_alias), 1);
    assert_eq!(u64_cell(&right_alias), 2);
    assert_eq!(left.published_version(), left_before_version);
    assert_eq!(right.published_version(), right_before_version);
    let committed = domain.commit_cell_publication(&mut prepared).unwrap();
    assert_eq!(committed.len(), 2);
    assert_eq!(u64_cell(&left_alias), 10);
    assert_eq!(u64_cell(&right_alias), 20);
    assert!(left.published_version() > left_before_version);
    assert!(right.published_version() > right_before_version);
}

fn u64_cell(cell: &ValueCell) -> u64 {
    let snapshot = cell.snapshot().unwrap();
    let mech_core::ValueData::U64(value) = snapshot.data() else {
        panic!("expected U64 cell")
    };
    *value
}
