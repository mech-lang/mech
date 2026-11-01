use super::*;
use crate::snapshot::ValueDataDraft;
use core::cell::{Cell, RefCell};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObservedInput {
    extents: Box<[u64]>,
    footprint: CurrentMemoryFootprint,
}

struct DemandFunction {
    input: ValueCell,
    output: ManagedPort<u64>,
    demand: Rc<Cell<PlannedExecutionDemand>>,
    planning: Rc<RefCell<Vec<ObservedInput>>>,
    executed: Rc<RefCell<Vec<ObservedInput>>>,
}
impl MechFunctionImpl for DemandFunction {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        panic!("request-aware footprint planning fell back to the legacy hook")
    }
    fn planned_output_footprints_with_request(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        self.planning.borrow_mut().push(ObservedInput {
            extents: request.inputs[0].current_extents()?,
            footprint: request
                .input_footprint(0)
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
        });
        Ok(None)
    }
    fn planned_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        assert_eq!(
            request.input_footprint(0).unwrap(),
            self.planning.borrow().last().unwrap().footprint
        );
        Ok(self.demand.get())
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let observed = ObservedInput {
            extents: frame
                .input_cell_descriptor(&self.input, 0)?
                .current_extents()?,
            footprint: frame.input_cell_footprint(&self.input, 0)?,
        };
        assert!(frame.input_cell_footprint(&self.input, 1).is_err());
        let count = observed.footprint.logical_elements;
        self.executed.borrow_mut().push(observed);
        frame.with_port_init_writer(&self.output, |writer| writer.write_next(count))?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn to_string(&self) -> String {
        "metadata demand fixture".into()
    }
}

struct DefaultFunction {
    output: ManagedPort<u64>,
    legacy_calls: Rc<Cell<usize>>,
}
impl MechFunctionImpl for DefaultFunction {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        self.legacy_calls.set(self.legacy_calls.get() + 1);
        Ok(None)
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_port_init_writer(&self.output, |writer| writer.write_next(17))?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn to_string(&self) -> String {
        "default demand fixture".into()
    }
}

struct CanonicalWorkspaceFunction {
    output: ValueCell,
    footprint: CurrentMemoryFootprint,
    observed_remaining: Rc<Cell<(u64, u64)>>,
}
impl MechFunctionImpl for CanonicalWorkspaceFunction {
    fn planned_output_footprints_with_request(
        &self,
        _: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(Box::new([self.footprint])))
    }
    fn planned_execution_demand(
        &self,
        _: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        Ok(PlannedExecutionDemand {
            temporary_bytes: 32,
            ..PlannedExecutionDemand::default()
        })
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_admitted_canonical_output(&self.output, self.footprint, |_, construction| {
            let before = construction.remaining_temporary_bytes();
            let mut scratch = construction.try_vec_with_capacity::<u64>(4)?;
            let after = construction.remaining_temporary_bytes();
            self.observed_remaining.set((before, after));
            assert_eq!(before - after, 32);
            assert!(
                construction
                    .try_vec_with_capacity::<u8>(usize::try_from(after).unwrap() + 1)
                    .is_err()
            );
            scratch.push(29);
            let schemas = self.output.schema_table();
            let value = crate::cell_binding::finalize_draft_with_construction(
                self.output.schema(),
                &self.output.shape(),
                schemas.as_ref(),
                ValueDataDraft::U64(scratch[0]),
                construction,
            )?;
            Ok(((), value))
        })?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn to_string(&self) -> String {
        "canonical workspace fixture".into()
    }
}

#[cfg(feature = "semantic-compiler")]
macro_rules! compiler_stub {
    ($($kind:ty),* $(,)?) => {$(
        impl MechFunctionCompiler for $kind {
            fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> { Ok(0) }
        }
    )*};
}
#[cfg(feature = "semantic-compiler")]
compiler_stub!(
    DemandFunction,
    DefaultFunction,
    CanonicalWorkspaceFunction,
    CanonicalIndexRowCopy,
    InputSnapshotFunction
);

fn instance(
    input: &ValueCell,
    output: &ValueCell,
    implementation: Box<dyn MechFunction>,
    implementation_memory: ImplementationMemoryClass,
    limits: MemoryBudgetLimits,
) -> FunctionInstance {
    let invocation = FunctionInvocation::unary(output.clone(), input.clone());
    let operation = ResolvedOperationDescriptor::from_name(
        "test/planned-execution-demand",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(Box::new([InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }])),
            outputs: Box::new([OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    // Use the maintained specialization planner so immutable canonical inputs
    // retain their actual capabilities. A logical String representation does
    // not request a writable physical StringHeader codec for a canonical root.
    let function = SpecializedFunction::syntax_directed(
        (implementation, invocation),
        operation,
        RuntimeFunctionId::from_name("test/planned-execution-demand"),
        ExecutionTarget::DirectRuntime,
        implementation_memory,
    )
    .unwrap()
    .into_instance();
    if limits == MemoryBudgetLimits::default() {
        return function;
    }
    // The denial fixture supplies stricter limits, then re-runs the complete
    // planner with the same maintained physical and semantic authorities.
    let mut template = function.memory_plan().clone();
    template.target.limits = limits;
    let plan = replan_call_memory(&template, &template.bound_call).unwrap();
    let FunctionInstance {
        implementation,
        invocation,
        managed_inputs,
        managed,
    } = function;
    let domain = output.memory_domain().unwrap();
    drop(managed);
    domain.collect_retired().unwrap();
    FunctionInstance::new_with_managed_inputs(
        implementation,
        invocation,
        Rc::new(plan),
        managed_inputs,
    )
    .unwrap()
}

struct Fixture {
    instance: FunctionInstance,
    input: ValueCell,
    output: ValueCell,
    domain: MemoryDomain,
    demand: Rc<Cell<PlannedExecutionDemand>>,
    planning: Rc<RefCell<Vec<ObservedInput>>>,
    executed: Rc<RefCell<Vec<ObservedInput>>>,
}
fn fixture(limits: MemoryBudgetLimits) -> Fixture {
    let domain = MemoryDomain::new().unwrap();
    let input =
        ValueCell::from_exact_in(&domain, nalgebra::DMatrix::from_element(2, 3, 11.0_f64)).unwrap();
    let output = ValueCell::from_exact_in(&domain, 99_u64).unwrap();
    let demand = Rc::new(Cell::new(PlannedExecutionDemand::default()));
    let planning = Rc::new(RefCell::new(Vec::new()));
    let executed = Rc::new(RefCell::new(Vec::new()));
    let invocation = FunctionInvocation::unary(output.clone(), input.clone());
    let implementation = with_semantic_operation(
        "test/wrapped-demand",
        Box::new(DemandFunction {
            input: input.clone(),
            output: invocation.output().try_managed::<u64>().unwrap(),
            demand: demand.clone(),
            planning: planning.clone(),
            executed: executed.clone(),
        }),
    );
    let instance = instance(
        &input,
        &output,
        implementation,
        ImplementationMemoryClass::NoAdditionalScratch,
        limits,
    );
    Fixture {
        instance,
        input,
        output,
        domain,
        demand,
        planning,
        executed,
    }
}

#[test]
fn execution_demand_defaults_preserve_legacy_hooks_and_fixed_plan() {
    let domain = MemoryDomain::new().unwrap();
    let input = ValueCell::from_exact_in(&domain, 1_u64).unwrap();
    let output = ValueCell::from_exact_in(&domain, 0_u64).unwrap();
    let invocation = FunctionInvocation::unary(output.clone(), input.clone());
    let legacy_calls = Rc::new(Cell::new(0));
    let function = instance(
        &input,
        &output,
        Box::new(DefaultFunction {
            output: invocation.output().try_managed::<u64>().unwrap(),
            legacy_calls: legacy_calls.clone(),
        }),
        ImplementationMemoryClass::NoAdditionalScratch,
        MemoryBudgetLimits::default(),
    );
    assert_eq!(
        function.memory_plan().output_storage[0].slot,
        PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))
    );
    let revision = function.managed_plan_revision();
    for _ in 0..3 {
        function.solve_result().unwrap();
    }
    let current = function.managed.current.borrow();
    assert_eq!(current.execution_demand, PlannedExecutionDemand::default());
    assert!(Rc::ptr_eq(&current.base_plan, &current.plan));
    assert_eq!(function.managed_plan_revision(), revision);
    assert_eq!(legacy_calls.get(), 3);
    assert!(matches!(
        output.snapshot().unwrap().data(),
        ValueData::U64(17)
    ));
}

#[test]
fn execution_demand_refreshes_without_accumulation_and_semantic_wrapper_forwards() {
    let f = fixture(MemoryBudgetLimits::default());
    let first = PlannedExecutionDemand {
        temporary_bytes: 32,
        cloned_bytes: 7,
        retained_nodes: 2,
        work: WorkDemand {
            compute: 13,
            comparison: 5,
            canonicalization: 3,
            scalar_instructions: 2,
        },
    };
    let second = PlannedExecutionDemand {
        temporary_bytes: 64,
        cloned_bytes: 1,
        retained_nodes: 1,
        work: WorkDemand {
            compute: 4,
            ..WorkDemand::default()
        },
    };
    let mut previous = f.instance.managed_plan_revision();
    for (turn, demand) in [first, first, second, PlannedExecutionDemand::default()]
        .into_iter()
        .enumerate()
    {
        f.demand.set(demand);
        f.instance.solve_result().unwrap();
        let current = f.instance.managed.current.borrow();
        let base = &current.base_plan;
        assert_eq!(current.execution_demand, demand);
        assert_eq!(
            current.plan.demand.turn_peak_bytes,
            base.demand.turn_peak_bytes + demand.temporary_bytes
        );
        assert_eq!(
            current.plan.demand.cloned_bytes,
            base.demand.cloned_bytes + demand.cloned_bytes
        );
        assert_eq!(
            current.plan.demand.retained_nodes,
            base.demand.retained_nodes + demand.retained_nodes
        );
        assert_eq!(
            current.plan.demand.work.compute,
            base.demand.work.compute + demand.work.compute
        );
        assert_eq!(
            current.plan.demand.work.comparison,
            base.demand.work.comparison + demand.work.comparison
        );
        assert_eq!(
            current.plan.demand.work.canonicalization,
            base.demand.work.canonicalization + demand.work.canonicalization
        );
        assert_eq!(
            current.plan.demand.work.scalar_instructions,
            base.demand.work.scalar_instructions + demand.work.scalar_instructions
        );
        assert_eq!(
            current
                .plan
                .allocations
                .iter()
                .filter(|a| a.role == AllocationRole::ConstructionWorkspace)
                .count(),
            usize::from(demand.temporary_bytes != 0)
        );
        let revision = current.realized.revision();
        if turn == 1 {
            assert_eq!(revision, previous);
        } else {
            assert_ne!(revision, previous);
        }
        previous = revision;
    }
    assert_eq!(*f.planning.borrow(), *f.executed.borrow());
    assert_eq!(f.executed.borrow().len(), 4);
}

#[test]
fn execution_demand_uses_current_geometry_and_frame_witness_after_replacement() {
    let f = fixture(MemoryBudgetLimits::default());
    f.demand.set(PlannedExecutionDemand {
        temporary_bytes: 32,
        work: WorkDemand {
            compute: 8,
            ..WorkDemand::default()
        },
        ..PlannedExecutionDemand::default()
    });
    for (turn, (rows, columns)) in [(2, 3), (3, 2), (0, 7), (1, 4)].into_iter().enumerate() {
        if turn != 0 {
            let next =
                ValueCell::from_exact(nalgebra::DMatrix::from_element(rows, columns, 23.0_f64))
                    .unwrap()
                    .snapshot()
                    .unwrap();
            f.input.replace(&next).unwrap();
        }
        f.instance.solve_result().unwrap();
        let actual = f.executed.borrow()[turn].clone();
        assert_eq!(actual.extents.as_ref(), &[rows as u64, columns as u64]);
        assert_eq!(actual.footprint.logical_elements, (rows * columns) as u64);
        // Fixed-width byte extents come from the same current port layout;
        // its footprint witness may omit bytes derived directly from the slot.
        assert_eq!(
            f.instance.managed.current.borrow().plan.inputs[0]
                .value
                .current_address_span_bytes,
            (rows * columns * 8) as u64,
        );
        assert_eq!(f.planning.borrow()[turn], actual);
    }
}

#[test]
fn execution_demand_budget_and_overflow_fail_before_execute_without_publication() {
    let f = fixture(MemoryBudgetLimits {
        max_compute_work: Some(10),
        ..MemoryBudgetLimits::default()
    });
    for denied in [
        PlannedExecutionDemand {
            work: WorkDemand {
                compute: 11,
                ..WorkDemand::default()
            },
            ..PlannedExecutionDemand::default()
        },
        PlannedExecutionDemand {
            temporary_bytes: u64::MAX,
            ..PlannedExecutionDemand::default()
        },
    ] {
        f.demand.set(denied);
        let version = f.output.published_version();
        let revision = f.instance.managed_plan_revision();
        let ledger = f.domain.ledger();
        assert!(f.instance.solve_result().is_err());
        assert!(f.executed.borrow().is_empty());
        assert_eq!(f.output.published_version(), version);
        assert_eq!(f.instance.managed_plan_revision(), revision);
        assert_eq!(f.domain.ledger(), ledger);
        assert!(matches!(
            f.output.snapshot().unwrap().data(),
            ValueData::U64(99)
        ));
    }
    f.demand.set(PlannedExecutionDemand::default());
    f.instance.solve_result().unwrap();
    assert!(matches!(
        f.output.snapshot().unwrap().data(),
        ValueData::U64(6)
    ));
}

#[test]
fn input_footprints_reject_missing_deferred_wrong_and_stale_leases() {
    let f = fixture(MemoryBudgetLimits::default());
    let current = f.instance.managed.current.borrow();
    let inputs = current.base_plan.bound_call.inputs();
    for stage in [
        MemoryWitnessStage::Activation,
        MemoryWitnessStage::Turn,
        MemoryWitnessStage::ExternalIngress,
    ] {
        let witnesses = [MemoryFootprintWitness::Deferred(stage)];
        let request = ExecutionDemandPlanningRequest {
            inputs,
            input_witnesses: &witnesses,
        };
        assert_eq!(
            request.input_footprint(0),
            Err(MemoryPlanError::MissingFootprintWitness { stage })
        );
    }
    let missing = ExecutionDemandPlanningRequest {
        inputs,
        input_witnesses: &[],
    };
    assert_eq!(
        missing.input_footprint(0),
        Err(MemoryPlanError::DescriptorArityMismatch)
    );
    let _scope = current
        .domain
        .enter_realized_plan_point(&current.realized, MemoryPlanPoint::new(0))
        .unwrap();
    current
        .initialize_inputs(core::slice::from_ref(&f.input))
        .unwrap();
    let frame = current
        .domain
        .acquire_call(&current.realized, &current.execution)
        .unwrap();
    let expected = current.base_plan.input_witnesses[0];
    assert_eq!(
        MemoryFootprintWitness::Known(frame.input_cell_footprint(&f.input, 0).unwrap()),
        expected
    );
    assert!(frame.input_cell_footprint(&f.output, 0).is_err());
    assert!(frame.input_cell_footprint(&f.input, 1).is_err());
    let key = current
        .domain
        .plan_object_key(current.realized.revision(), current.plan.inputs[0].object)
        .unwrap();
    let old = {
        let mut state = current.domain.state.borrow_mut();
        let region = state.regions.get_mut(&key).unwrap();
        let old = region.incarnation;
        region.incarnation = old.checked_successor().unwrap();
        old
    };
    let stale = frame.input_cell_footprint(&f.input, 0);
    current
        .domain
        .state
        .borrow_mut()
        .regions
        .get_mut(&key)
        .unwrap()
        .incarnation = old;
    assert!(stale.is_err());
    assert_eq!(
        MemoryFootprintWitness::Known(frame.input_cell_footprint(&f.input, 0).unwrap()),
        expected
    );
}

#[test]
fn execution_demand_workspace_is_debited_by_real_canonical_construction() {
    let domain = MemoryDomain::new().unwrap();
    let input = ValueCell::from_exact_in(&domain, 1_u64).unwrap();
    let descriptor = input.resolved_descriptor().unwrap();
    let output = ValueCell::allocate_for_descriptor_in(
        &domain,
        &descriptor,
        FunctionValueRepresentation::AnyValue,
    )
    .unwrap();
    let footprint = output.current_memory_footprint().unwrap();
    let observed_remaining = Rc::new(Cell::new((0, 0)));
    let function = instance(
        &input,
        &output,
        Box::new(CanonicalWorkspaceFunction {
            output: output.clone(),
            footprint,
            observed_remaining: observed_remaining.clone(),
        }),
        ImplementationMemoryClass::CanonicalFinalize,
        MemoryBudgetLimits::default(),
    );
    function.solve_result().unwrap();
    let current = function.managed.current.borrow();
    let base_scratch = current
        .base_plan
        .allocations
        .iter()
        .filter(|a| a.role == AllocationRole::ConstructionWorkspace)
        .map(|a| a.capacity_bytes)
        .sum::<u64>();
    let MemoryFootprintWitness::Known(output_witness) = current.base_plan.output_witnesses[0]
    else {
        panic!("canonical construction requires a known prospective output footprint");
    };
    let finalization = canonical_snapshot_finalization_bytes(output_witness).unwrap();
    // begin_construction withholds finalization authority before exposing its
    // temporary allocator. The extra 32 bytes belong wholly to that allocator.
    let base_temporary = base_scratch.checked_sub(finalization).unwrap();
    let admitted_scratch = current
        .plan
        .allocations
        .iter()
        .filter(|allocation| allocation.role == AllocationRole::ConstructionWorkspace)
        .map(|allocation| allocation.capacity_bytes)
        .sum::<u64>();
    assert_eq!(admitted_scratch, base_scratch + 32);
    assert_eq!(observed_remaining.get().0, base_temporary + 32);
    assert_eq!(observed_remaining.get().1, base_temporary);
    assert!(matches!(
        output.snapshot().unwrap().data(),
        ValueData::U64(29)
    ));
}

#[test]
fn execution_demand_payload_hooks_and_frame_share_refreshed_string_witnesses() {
    let domain = MemoryDomain::new().unwrap();
    let input = ValueCell::from_schema_data(SchemaBody::String, ValueDataDraft::String("é".into()))
        .unwrap();
    let output = ValueCell::from_exact_in(&domain, 0_u64).unwrap();
    let invocation = FunctionInvocation::unary(output.clone(), input.clone());
    let demand = Rc::new(Cell::new(PlannedExecutionDemand::default()));
    let planning = Rc::new(RefCell::new(Vec::new()));
    let executed = Rc::new(RefCell::new(Vec::new()));
    let function = instance(
        &input,
        &output,
        with_semantic_operation(
            "test/string-witness",
            Box::new(DemandFunction {
                input: input.clone(),
                output: invocation.output().try_managed::<u64>().unwrap(),
                demand,
                planning: planning.clone(),
                executed: executed.clone(),
            }),
        ),
        ImplementationMemoryClass::NoAdditionalScratch,
        MemoryBudgetLimits::default(),
    );
    assert!(input.has_managed_canonical_storage().unwrap());
    assert_eq!(input.representation(), FunctionValueRepresentation::String);
    assert_eq!(input.closed_schema_body().unwrap(), SchemaBody::String);
    assert_eq!(
        function.memory_plan().input_storage[0].slot,
        PlannedSlotKind::CanonicalValueHandle
    );
    assert_eq!(
        function.memory_plan().input_storage[0].capabilities,
        input.storage_capabilities()
    );
    for (turn, text) in ["é", "", "🧑‍🚀é"].into_iter().enumerate() {
        if turn != 0 {
            let next = ValueCell::from_schema_data(
                SchemaBody::String,
                ValueDataDraft::String(text.into()),
            )
            .unwrap()
            .snapshot()
            .unwrap();
            input.replace(&next).unwrap();
        }
        let expected = input.current_memory_footprint().unwrap();
        function.solve_result().unwrap();
        assert_eq!(planning.borrow()[turn].footprint, expected);
        assert_eq!(executed.borrow()[turn].footprint, expected);
        if turn != 0 {
            assert_ne!(
                executed.borrow()[turn].footprint.payload_bytes,
                executed.borrow()[turn - 1].footprint.payload_bytes
            );
        }
    }
}

// This fixture deliberately retains a logical numeric representation over a
// canonical root, exactly as an explicitly canonical factory result does.
struct CanonicalIndexRowCopy {
    input: ValueCell,
    output: ValueCell,
}
impl CanonicalIndexRowCopy {
    fn output_footprint(&self, input: CurrentMemoryFootprint) -> CurrentMemoryFootprint {
        CurrentMemoryFootprint {
            schema_bytes: self
                .output
                .retained_schema_table()
                .clone_allocation_bound_bytes()
                .unwrap(),
            ..input
        }
    }
}
impl MechFunctionImpl for CanonicalIndexRowCopy {
    fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
        let extents = self.input.resolved_descriptor()?.current_extents()?;
        let schemas = self.output.retained_schema_table();
        let schema = schemas.get(self.output.schema()).unwrap();
        Ok(Some(Box::new([shape_for_resolved_extents(
            schema, &extents,
        )?])))
    }
    fn planned_output_footprints_with_request(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(Box::new([self.output_footprint(
            request
                .input_footprint(0)
                .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
        )])))
    }
    fn planned_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        let count = request
            .input_footprint(0)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?
            .logical_elements;
        Ok(PlannedExecutionDemand {
            temporary_bytes: count
                .checked_mul(core::mem::size_of::<ValueDataDraft>() as u64)
                .and_then(|bytes| bytes.checked_add(16))
                .unwrap(),
            work: WorkDemand {
                compute: count.checked_mul(2).unwrap(),
                ..WorkDemand::default()
            },
            ..PlannedExecutionDemand::default()
        })
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let footprint = self.output_footprint(frame.input_cell_footprint(&self.input, 0)?);
        frame.with_admitted_canonical_output(&self.output, footprint, |frame, construction| {
            frame.with_input_cell_value_view::<usize, _>(&self.input, 0, |view| {
                let elements = construction.try_boxed_slice_with(view.len(), |_, index| {
                    Ok(ValueDataDraft::Index(
                        view.get_column_major(index).unwrap() as u64
                    ))
                })?;
                let dimensions = construction.try_boxed_slice_with(2, |_, axis| {
                    Ok(if axis == 0 {
                        view.rows()
                    } else {
                        view.columns()
                    } as u64)
                })?;
                let value =
                    construction.try_rebuild_matrix_drafts(&self.output, dimensions, elements)?;
                Ok(((), value))
            })
        })?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn to_string(&self) -> String {
        "canonical numeric output fixture".into()
    }
}

fn canonical_index_row(domain: &MemoryDomain, values: &[u64]) -> ValueCell {
    use crate::snapshot::SnapshotValidationContext;
    let schema = SchemaDraft {
        dimension_parameters: Box::new([DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(16)),
        }]),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: Box::new([
                DimensionExpr::Constant(1),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]),
        },
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema: id,
        shape_values: Box::new([values.len() as u64]),
        data: ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::Index).collect()),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    ValueCell::from_snapshot_in(domain, value).unwrap()
}

fn assert_canonical_output_envelope(function: &FunctionInstance, output: &ValueCell) {
    let current = function.managed.current.borrow();
    assert_eq!(
        current.plan.output_storage[0].slot,
        PlannedSlotKind::CanonicalValueHandle
    );
    assert_eq!(
        current.plan.output_storage[0].capabilities,
        output.storage_capabilities()
    );
    let header = current
        .plan
        .allocations
        .iter()
        .find(|allocation| allocation.id == current.plan.outputs[0].object)
        .unwrap();
    assert!(current.plan.allocations.iter().any(|allocation| {
        allocation.role == AllocationRole::VariablePayload
            && allocation.owner == header.owner
            && allocation.lifetime == header.lifetime
    }));
}

#[test]
fn canonical_numeric_outputs_keep_factory_construction_authority() {
    for class in [
        ImplementationMemoryClass::CanonicalFinalize,
        ImplementationMemoryClass::CanonicalSortUnique,
        ImplementationMemoryClass::CanonicalCloneInput { input: 0 },
    ] {
        let domain = MemoryDomain::new().unwrap();
        let source = ValueCell::from_exact_in(&domain, 1_u64).unwrap();
        let input = ValueCell::from_snapshot_in(&domain, source.snapshot().unwrap()).unwrap();
        let output = ValueCell::from_snapshot_in(&domain, source.snapshot().unwrap()).unwrap();
        assert_eq!(output.representation(), FunctionValueRepresentation::U64);
        assert!(output.requires_canonical_output_builder().unwrap());
        let function = instance(
            &input,
            &output,
            Box::new(CanonicalWorkspaceFunction {
                output: output.clone(),
                footprint: output.current_memory_footprint().unwrap(),
                observed_remaining: Rc::new(Cell::new((0, 0))),
            }),
            class,
            MemoryBudgetLimits::default(),
        );
        assert_eq!(
            function.memory_plan().input_storage[0].slot,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))
        );
        assert_canonical_output_envelope(&function, &output);
        if class == ImplementationMemoryClass::CanonicalFinalize {
            function.solve_result().unwrap();
            assert!(matches!(
                output.snapshot().unwrap().data(),
                ValueData::U64(29)
            ));
            assert_canonical_output_envelope(&function, &output);
        }
    }
}

#[test]
fn canonical_index_matrix_output_refreshes_empty_nonempty_empty() {
    use crate::snapshot::SequenceView;
    let domain = MemoryDomain::new().unwrap();
    let input = canonical_index_row(&domain, &[]);
    let output = canonical_index_row(&domain, &[]);
    assert!(matches!(
        output.representation(),
        FunctionValueRepresentation::Matrix {
            element: FunctionMatrixElement::Index,
            ..
        }
    ));
    assert!(output.requires_canonical_output_builder().unwrap());
    let function = instance(
        &input,
        &output,
        Box::new(CanonicalIndexRowCopy {
            input: input.clone(),
            output: output.clone(),
        }),
        ImplementationMemoryClass::CanonicalFinalize,
        MemoryBudgetLimits::default(),
    );
    assert_eq!(
        function.memory_plan().input_storage[0].slot,
        PlannedSlotKind::FixedScalar(ScalarMemoryKind::Index)
    );
    assert_canonical_output_envelope(&function, &output);
    let mut revision = function.managed_plan_revision();
    for (turn, expected) in [&[][..], &[3, 1, 2][..], &[][..]].into_iter().enumerate() {
        if turn != 0 {
            input
                .replace(&canonical_index_row(&domain, expected).snapshot().unwrap())
                .unwrap();
        }
        function.solve_result().unwrap();
        assert_canonical_output_envelope(&function, &output);
        let current = function.managed.current.borrow();
        assert_eq!(
            current.plan.outputs[0]
                .descriptor
                .current_extents()
                .unwrap()
                .as_ref(),
            &[1, expected.len() as u64]
        );
        if turn != 0 {
            assert_ne!(function.managed_plan_revision(), revision);
        }
        revision = function.managed_plan_revision();
        drop(current);
        let value = output.snapshot().unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("{value:?}")
        };
        let SequenceView::Index(values) = matrix.elements() else {
            panic!("{value:?}")
        };
        assert_eq!(values, expected);
        let descriptor = output.resolved_descriptor().unwrap();
        assert_eq!(
            descriptor.current_extents().unwrap().as_ref(),
            &[1, expected.len() as u64]
        );
    }
}

struct InputSnapshotFunction {
    input: ValueCell,
    retained_input: Value,
    output: ValueCell,
    output_footprint: CurrentMemoryFootprint,
    fund_reconstruction: Rc<Cell<bool>>,
    observations: Rc<RefCell<Vec<(u64, u64, u64, u64, bool)>>>,
}
impl MechFunctionImpl for InputSnapshotFunction {
    fn planned_output_footprints_with_request(
        &self,
        _: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(Box::new([self.output_footprint])))
    }
    fn planned_execution_demand(
        &self,
        request: &ExecutionDemandPlanningRequest<'_>,
    ) -> MResult<PlannedExecutionDemand> {
        let input = request
            .input_footprint(0)
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        let canonical = CurrentMemoryFootprint {
            payload_bytes: input.payload_bytes.max(input.fixed_bytes),
            ..input
        };
        let temporary = canonical_snapshot_draft_bytes(canonical)
            .unwrap()
            .checked_add(canonical_snapshot_finalization_bytes(canonical).unwrap())
            .unwrap();
        Ok(PlannedExecutionDemand {
            temporary_bytes: if self.fund_reconstruction.get() {
                temporary
            } else {
                0
            },
            cloned_bytes: input.fixed_bytes.checked_add(input.schema_bytes).unwrap(),
            retained_nodes: input.logical_elements.checked_add(3).unwrap(),
            work: WorkDemand {
                compute: input.fixed_bytes.checked_add(input.schema_bytes).unwrap(),
                ..WorkDemand::default()
            },
        })
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_admitted_canonical_output(
            &self.output,
            self.output_footprint,
            |frame, construction| {
                let temporary_before = construction.remaining_temporary_bytes();
                let finalization_before = construction.remaining_output_finalization_bytes();
                let input =
                    frame.snapshot_input_cell_with_construction(&self.input, 0, construction)?;
                let temporary_after = construction.remaining_temporary_bytes();
                let finalization_after = construction.remaining_output_finalization_bytes();
                self.observations.borrow_mut().push((
                    temporary_before,
                    temporary_after,
                    finalization_before,
                    finalization_after,
                    input.shares_frozen_storage(&self.retained_input),
                ));
                let ValueData::Matrix(matrix) = input.data() else {
                    panic!("{input:?}")
                };
                let crate::snapshot::SequenceView::U64(values) = matrix.elements() else {
                    panic!("{input:?}")
                };
                assert_eq!(values.len(), 256);
                assert!(values.iter().all(|value| *value == 7));
                let schemas = self.output.retained_schema_table();
                let output = crate::cell_binding::finalize_draft_with_construction(
                    self.output.schema(),
                    &self.output.shape(),
                    schemas.as_ref(),
                    ValueDataDraft::U64(values.len() as u64),
                    construction,
                )?;
                Ok(((), output))
            },
        )?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn to_string(&self) -> String {
        "temporary input snapshot fixture".into()
    }
}

#[test]
fn input_snapshot_reconstruction_preserves_output_reserve_and_rolls_back_on_denial() {
    let domain = MemoryDomain::new().unwrap();
    let input =
        ValueCell::from_exact_in(&domain, nalgebra::DMatrix::from_element(2, 128, 7_u64)).unwrap();
    assert!(!input.requires_canonical_output_builder().unwrap());
    let seed = ValueCell::from_exact_in(&domain, 99_u64).unwrap();
    let output = ValueCell::from_snapshot_in(&domain, seed.snapshot().unwrap()).unwrap();
    let output_footprint = output.current_memory_footprint().unwrap();
    let input_footprint = input.current_memory_footprint().unwrap();
    assert_eq!(input_footprint.logical_elements, 256);
    assert_eq!(output_footprint.logical_elements, 1);
    let observations = Rc::new(RefCell::new(Vec::new()));
    let fund_reconstruction = Rc::new(Cell::new(true));
    let function = instance(
        &input,
        &output,
        Box::new(InputSnapshotFunction {
            input: input.clone(),
            retained_input: input.snapshot().unwrap(),
            output: output.clone(),
            output_footprint,
            fund_reconstruction: fund_reconstruction.clone(),
            observations: observations.clone(),
        }),
        ImplementationMemoryClass::CanonicalFinalize,
        MemoryBudgetLimits::default(),
    );
    assert_eq!(
        function.memory_plan().input_storage[0].slot,
        PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))
    );
    function.solve_result().unwrap();
    let first = observations.borrow()[0];
    assert!(
        !first.4,
        "dense input requires an admitted temporary snapshot"
    );
    assert!(first.0 - first.1 >= input_footprint.fixed_bytes + input_footprint.schema_bytes);
    assert_eq!(
        first.2,
        canonical_snapshot_finalization_bytes(output_footprint).unwrap()
    );
    assert_eq!(
        first.3, first.2,
        "input reconstruction cannot spend output finalization"
    );
    assert!(matches!(
        output.snapshot().unwrap().data(),
        ValueData::U64(256)
    ));

    for injected in [false, true] {
        fund_reconstruction.set(injected);
        let before = output.snapshot().unwrap();
        let version = output.published_version();
        let revision = function.managed_plan_revision();
        let ledger = domain.ledger();
        let observed = observations.borrow().len();
        if injected {
            domain
                .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
                .unwrap();
        }
        let error = function.solve_result().unwrap_err();
        if !injected {
            assert!(format!("{error:?}").contains("CapacityExceeded"));
        }
        assert_eq!(observations.borrow().len(), observed);
        assert_eq!(output.published_version(), version);
        assert_eq!(function.managed_plan_revision(), revision);
        assert_eq!(domain.ledger(), ledger);
        let after = output.snapshot().unwrap();
        assert!(
            before
                .snapshot_eq(
                    &before.schemas().unwrap(),
                    &after,
                    &after.schemas().unwrap()
                )
                .unwrap()
        );
        fund_reconstruction.set(true);
        function.solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::U64(256)
        ));
        let latest = *observations.borrow().last().unwrap();
        assert_eq!(latest.2, latest.3);
    }
}

#[test]
fn canonical_input_snapshot_keeps_its_root_without_temporary_reconstruction() {
    let domain = MemoryDomain::new().unwrap();
    let dense =
        ValueCell::from_exact_in(&domain, nalgebra::DMatrix::from_element(2, 128, 7_u64)).unwrap();
    let input = ValueCell::from_snapshot_in(&domain, dense.snapshot().unwrap()).unwrap();
    let retained_input = input.snapshot().unwrap();
    assert!(input.requires_canonical_output_builder().unwrap());
    let output = ValueCell::from_snapshot_in(
        &domain,
        ValueCell::from_exact_in(&domain, 0_u64)
            .unwrap()
            .snapshot()
            .unwrap(),
    )
    .unwrap();
    let observations = Rc::new(RefCell::new(Vec::new()));
    let function = instance(
        &input,
        &output,
        Box::new(InputSnapshotFunction {
            input: input.clone(),
            retained_input,
            output: output.clone(),
            output_footprint: output.current_memory_footprint().unwrap(),
            fund_reconstruction: Rc::new(Cell::new(false)),
            observations: observations.clone(),
        }),
        ImplementationMemoryClass::CanonicalFinalize,
        MemoryBudgetLimits::default(),
    );
    function.solve_result().unwrap();
    let observed = observations.borrow()[0];
    assert!(observed.4);
    assert_eq!(observed.0, observed.1);
    assert_eq!(observed.2, observed.3);
    assert!(matches!(
        output.snapshot().unwrap().data(),
        ValueData::U64(256)
    ));
}
