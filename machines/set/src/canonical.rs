use crate::*;

fn snapshot_error(error: SnapshotValueError) -> MechError {
    MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
}

#[derive(Debug)]
#[cfg(any(feature = "membership", feature = "modify"))]
pub(crate) struct ArbitraryInput(FunctionValueInput);

#[cfg(any(feature = "membership", feature = "modify"))]
impl ArbitraryInput {
    pub(crate) fn canonical(port: FunctionInputPort<'_>) -> Self {
        Self(port.value())
    }

    pub(crate) fn snapshot(&self, frame: &KernelMemoryFrame<'_>) -> MResult<Value> {
        frame.snapshot_function_value_input(&self.0)
    }

    pub(crate) fn planning_snapshot(&self) -> MResult<Value> {
        self.0.snapshot()
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compile_register(
        &self,
        context: &mut dyn BytecodeCompilerContext,
    ) -> MResult<Register> {
        self.0.compile_register(context)
    }
}

#[derive(Debug)]
pub(crate) struct SetInput(FunctionValueInput);

#[derive(Clone, Copy)]
#[cfg(feature = "relations")]
pub(crate) enum SetRelation {
    Disjoint,
    Equal,
    NotEqual,
    ProperSubset,
    ProperSuperset,
    Subset,
    Superset,
}

impl SetInput {
    pub(crate) fn canonical(port: FunctionInputPort<'_>) -> MResult<Self> {
        let role = FunctionArgumentRole::Input(port.index());
        let value = port.value();
        if value.representation() != FunctionValueRepresentation::Set {
            return Err(argument_type_mismatch(role, value.representation()));
        }
        value.set_elements()?;
        Ok(Self(value))
    }

    pub(crate) fn planning_snapshot(&self) -> MResult<Value> {
        self.0.snapshot()
    }

    #[cfg(any(feature = "cartesian_product", feature = "powerset"))]
    pub(crate) fn planning_cardinality(&self) -> MResult<usize> {
        self.planning_snapshot()?
            .set_view()
            .map(|set| set.elements().len())
            .ok_or_else(|| function_shape_contract_violation("set/operation", "input is not a set"))
    }

    pub(crate) fn prospective_binary_footprint(
        &self,
        other: &Self,
        output: &SetOutput,
    ) -> MResult<CurrentMemoryFootprint> {
        let left = self.planning_snapshot()?;
        let right = other.planning_snapshot()?;
        let left = left.set_view().ok_or_else(|| {
            function_shape_contract_violation("set/operation", "left input is not a set")
        })?;
        let right = right.set_view().ok_or_else(|| {
            function_shape_contract_violation("set/operation", "right input is not a set")
        })?;
        output.0.cell().prospective_set_data_memory_footprint(
            left.elements()
                .iter()
                .chain(right.elements())
                .map(|entry| entry.data()),
        )
    }

    #[cfg(feature = "modify")]
    pub(crate) fn prospective_update_footprint(
        &self,
        candidate: &ArbitraryInput,
        output: &SetOutput,
    ) -> MResult<CurrentMemoryFootprint> {
        let set = self.planning_snapshot()?;
        let candidate = candidate.planning_snapshot()?;
        let set = set
            .set_view()
            .ok_or_else(|| function_shape_contract_violation("set/update", "input is not a set"))?;
        output.0.cell().prospective_set_data_memory_footprint(
            set.elements()
                .iter()
                .map(|entry| entry.data())
                .chain(core::iter::once(candidate.data())),
        )
    }

    #[cfg(feature = "relations")]
    pub(crate) fn relation(
        &self,
        frame: &KernelMemoryFrame<'_>,
        other: &Self,
        relation: SetRelation,
    ) -> MResult<bool> {
        let relation = match relation {
            SetRelation::Disjoint => SetValueRelation::Disjoint,
            SetRelation::Equal => SetValueRelation::Equal,
            SetRelation::NotEqual => SetValueRelation::NotEqual,
            SetRelation::ProperSubset => SetValueRelation::ProperSubset,
            SetRelation::ProperSuperset => SetValueRelation::ProperSuperset,
            SetRelation::Subset => SetValueRelation::Subset,
            SetRelation::Superset => SetValueRelation::Superset,
        };
        let left = frame.snapshot_function_value_input(&self.0)?;
        let right = frame.snapshot_function_value_input(&other.0)?;
        let left_schemas = left.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/relation", "left set has no schema table")
        })?;
        let right_schemas = right.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/relation", "right set has no schema table")
        })?;
        left.set_relation(&left_schemas, &right, &right_schemas, relation)
            .map_err(snapshot_error)
    }

    pub(crate) fn contains(
        &self,
        frame: &KernelMemoryFrame<'_>,
        candidate: &ArbitraryInput,
    ) -> MResult<bool> {
        let set = frame.snapshot_function_value_input(&self.0)?;
        let candidate = candidate.snapshot(frame)?;
        let set_schemas = set.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/contains", "set has no schema table")
        })?;
        let candidate_schemas = candidate.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/contains", "candidate has no schema table")
        })?;
        let SchemaBody::Set { element, .. } = set_schemas
            .get(set.schema())
            .ok_or_else(|| {
                function_shape_contract_violation("set/contains", "set schema is missing")
            })?
            .body()
        else {
            return Err(function_shape_contract_violation(
                "set/contains",
                "input is not a set",
            ));
        };
        let candidate_schema = candidate_schemas
            .get(candidate.schema())
            .ok_or_else(|| {
                function_shape_contract_violation("set/contains", "candidate schema is missing")
            })?
            .body();
        if candidate_schema != element.as_ref() {
            return Ok(false);
        }
        set.set_contains(&set_schemas, &candidate, &candidate_schemas)
            .map_err(snapshot_error)
    }

    #[cfg(all(feature = "size", feature = "u64"))]
    pub(crate) fn elements(&self, frame: &KernelMemoryFrame<'_>) -> MResult<Box<[ValueData]>> {
        let set = frame.snapshot_function_value_input(&self.0)?;
        let Some(view) = set.set_view() else {
            return Err(function_shape_contract_violation(
                "set/elements",
                "input is not a set",
            ));
        };
        Ok(view
            .elements()
            .iter()
            .map(|value| value.data().clone())
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }

    #[cfg(any(feature = "cartesian_product", feature = "powerset"))]
    pub(crate) fn element_drafts(
        &self,
        frame: &KernelMemoryFrame<'_>,
    ) -> MResult<Box<[ValueDataDraft]>> {
        let set = frame.snapshot_function_value_input(&self.0)?;
        let schemas = set.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/elements", "set has no schema table")
        })?;
        set.set_element_drafts(&schemas).map_err(snapshot_error)
    }

    pub(crate) fn elements_after_insert(
        &self,
        frame: &KernelMemoryFrame<'_>,
        candidate: &ArbitraryInput,
    ) -> MResult<Box<[ValueData]>> {
        self.with_candidate(frame, candidate, Value::set_elements_after_insert)
    }

    pub(crate) fn elements_after_remove(
        &self,
        frame: &KernelMemoryFrame<'_>,
        candidate: &ArbitraryInput,
    ) -> MResult<Box<[ValueData]>> {
        self.with_candidate(frame, candidate, Value::set_elements_after_remove)
    }

    pub(crate) fn union_elements(
        &self,
        frame: &KernelMemoryFrame<'_>,
        other: &Self,
    ) -> MResult<Box<[ValueData]>> {
        self.with_set(frame, other, Value::set_union_elements)
    }

    pub(crate) fn intersection_elements(
        &self,
        frame: &KernelMemoryFrame<'_>,
        other: &Self,
    ) -> MResult<Box<[ValueData]>> {
        self.with_set(frame, other, Value::set_intersection_elements)
    }

    pub(crate) fn difference_elements(
        &self,
        frame: &KernelMemoryFrame<'_>,
        other: &Self,
    ) -> MResult<Box<[ValueData]>> {
        self.with_set(frame, other, Value::set_difference_elements)
    }

    pub(crate) fn symmetric_difference_elements(
        &self,
        frame: &KernelMemoryFrame<'_>,
        other: &Self,
    ) -> MResult<Box<[ValueData]>> {
        self.with_set(frame, other, Value::set_symmetric_difference_elements)
    }

    fn with_candidate(
        &self,
        frame: &KernelMemoryFrame<'_>,
        candidate: &ArbitraryInput,
        operation: fn(
            &Value,
            &SchemaTable,
            &Value,
            &SchemaTable,
        ) -> Result<Box<[ValueData]>, SnapshotValueError>,
    ) -> MResult<Box<[ValueData]>> {
        let set = frame.snapshot_function_value_input(&self.0)?;
        let candidate = candidate.snapshot(frame)?;
        let set_schemas = set.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/update", "set has no schema table")
        })?;
        let candidate_schemas = candidate.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/update", "candidate has no schema table")
        })?;
        operation(&set, &set_schemas, &candidate, &candidate_schemas).map_err(snapshot_error)
    }

    fn with_set(
        &self,
        frame: &KernelMemoryFrame<'_>,
        other: &Self,
        operation: fn(
            &Value,
            &SchemaTable,
            &Value,
            &SchemaTable,
        ) -> Result<Box<[ValueData]>, SnapshotValueError>,
    ) -> MResult<Box<[ValueData]>> {
        let left = frame.snapshot_function_value_input(&self.0)?;
        let right = frame.snapshot_function_value_input(&other.0)?;
        let left_schemas = left.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/operation", "left set has no schema table")
        })?;
        let right_schemas = right.schemas().ok_or_else(|| {
            function_shape_contract_violation("set/operation", "right set has no schema table")
        })?;
        operation(&left, &left_schemas, &right, &right_schemas).map_err(snapshot_error)
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compile_register(
        &self,
        context: &mut dyn BytecodeCompilerContext,
    ) -> MResult<Register> {
        self.0.compile_register(context)
    }
}

#[derive(Debug)]
pub(crate) struct SetOutput(FunctionValueOutput);

impl SetOutput {
    pub(crate) fn canonical(port: FunctionOutputPort<'_>) -> MResult<Self> {
        let value = port.value();
        if value.representation() != FunctionValueRepresentation::Set
            || value.snapshot()?.set_view().is_none()
        {
            return Err(argument_type_mismatch(
                FunctionArgumentRole::Output,
                value.representation(),
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn with_admitted_set(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        footprint: CurrentMemoryFootprint,
        build: impl FnOnce(&mut KernelMemoryFrame<'_>) -> MResult<Box<[ValueData]>>,
    ) -> MResult<()> {
        frame.with_admitted_canonical_output(self.0.cell(), footprint, |frame, construction| {
            let next = construction.try_build_set_with(&self.0, || build(frame))?;
            Ok(((), next))
        })
    }

    #[cfg(any(feature = "cartesian_product", feature = "powerset"))]
    pub(crate) fn prospective_expansion_footprint(
        &self,
        inputs: &[&SetInput],
        output_elements: usize,
    ) -> MResult<CurrentMemoryFootprint> {
        let output_elements = u64::try_from(output_elements).map_err(|_| {
            MechError::new(
                MemoryPlanError::ArithmeticOverflow {
                    field: "set expansion output elements",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let mut bound = self.0.cell().current_memory_footprint()?;
        let copies = output_elements.checked_add(1).ok_or_else(|| {
            MechError::new(
                MemoryPlanError::ArithmeticOverflow {
                    field: "set expansion copies",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        for footprint in inputs
            .iter()
            .map(|input| input.0.cell().current_memory_footprint())
        {
            let footprint = footprint?;
            macro_rules! add_scaled {
                ($field:ident, $name:literal) => {
                    bound.$field = footprint
                        .$field
                        .checked_mul(copies)
                        .and_then(|amount| bound.$field.checked_add(amount))
                        .ok_or_else(|| {
                            MechError::new(
                                MemoryPlanError::ArithmeticOverflow { field: $name },
                                None,
                            )
                            .with_compiler_loc()
                        })?;
                };
            }
            add_scaled!(fixed_bytes, "set expansion fixed bytes");
            add_scaled!(payload_bytes, "set expansion payload bytes");
            add_scaled!(encoded_bytes, "set expansion encoded bytes");
            add_scaled!(retained_nodes, "set expansion retained nodes");
            add_scaled!(schema_bytes, "set expansion schema bytes");
        }
        bound.logical_elements = output_elements;
        Ok(bound)
    }

    #[cfg(any(feature = "cartesian_product", feature = "powerset"))]
    pub(crate) fn with_admitted_set_drafts(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        footprint: CurrentMemoryFootprint,
        build: impl FnOnce(&mut KernelMemoryFrame<'_>) -> MResult<Box<[ValueDataDraft]>>,
    ) -> MResult<()> {
        frame.with_admitted_canonical_output(self.0.cell(), footprint, |frame, construction| {
            let next = construction.try_build_set_drafts_with(&self.0, || build(frame))?;
            Ok(((), next))
        })
    }

    pub(crate) fn primary_state_port(&self) -> Option<FunctionStatePort<'_>> {
        None
    }

    pub(crate) fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(Vec::new()))
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compile_register(
        &self,
        context: &mut dyn BytecodeCompilerContext,
    ) -> MResult<Register> {
        self.0.compile_register(context)
    }
}

#[cfg(feature = "source")]
pub(crate) fn specialize_dynamic_set<F>(
    invocation: &SpecializationInvocation,
    context: &mut SpecializationContext<'_>,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let inputs = invocation.inputs().iter().collect::<Vec<_>>();
    context.bind_resolved_runtime(
        RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
        ExecutionTarget::DirectRuntime,
        vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
        &inputs,
    )
}

#[cfg(feature = "source")]
pub(crate) fn specialize_bool<F>(
    invocation: &SpecializationInvocation,
    context: &mut SpecializationContext<'_>,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let inputs = invocation.inputs().iter().collect::<Vec<_>>();
    context.bind_resolved_runtime(
        RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
        ExecutionTarget::DirectRuntime,
        vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
        &inputs,
    )
}

#[cfg(all(feature = "source", feature = "u64"))]
pub(crate) fn specialize_u64<F>(
    invocation: &SpecializationInvocation,
    context: &mut SpecializationContext<'_>,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let inputs = invocation.inputs().iter().collect::<Vec<_>>();
    context.bind_resolved_runtime(
        RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
        ExecutionTarget::DirectRuntime,
        vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
        &inputs,
    )
}

fn argument_type_mismatch(
    role: FunctionArgumentRole,
    found: FunctionValueRepresentation,
) -> MechError {
    MechError::new(
        FunctionArgumentTypeMismatch {
            role,
            expected: "canonical Set value".into(),
            found: format!("{found:?}"),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(feature = "relations")]
macro_rules! define_set_relation {
    ($function:ident, $specializer:ident, $relation:ident, $name:literal) => {
        use crate::canonical::{SetInput, SetRelation};
        use crate::*;

        #[derive(Debug)]
        pub(crate) struct $function {
            lhs: SetInput,
            rhs: SetInput,
            out: ManagedPort<bool>,
        }

        impl MechFunctionFactory for $function {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                FunctionValueRepresentation::Bool,
                FunctionValueRepresentation::Set,
                FunctionValueRepresentation::Set,
            );
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                Ok(Box::new(Self {
                    lhs: SetInput::canonical(lhs)?,
                    rhs: SetInput::canonical(rhs)?,
                    out: out.try_managed()?,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_SET_PREDICATE_CONTRACT)
            }
        }

        impl MechFunctionImpl for $function {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                let next = self
                    .lhs
                    .relation(frame, &self.rhs, SetRelation::$relation)?;
                frame.with_port_init_writer(&self.out, |output| output.write_next(next))?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_SET_PREDICATE_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $function {
            fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let destination = compile_value_cell_register(self.out.cell(), context)?;
                let lhs = self.lhs.compile_register(context)?;
                let rhs = self.rhs.compile_register(context)?;
                context.emit_binop(hash_str($name), destination, lhs, rhs);
                Ok(destination)
            }
        }

        #[cfg(feature = "source")]
        pub struct $specializer {}

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                crate::canonical::specialize_bool::<$function>(invocation, context)
            }
        }
    };
}

#[cfg(feature = "relations")]
pub(crate) use define_set_relation;

#[cfg(feature = "membership")]
macro_rules! define_set_membership {
    ($function:ident, $specializer:ident, $negated:literal, $name:literal) => {
        use crate::canonical::{ArbitraryInput, SetInput};
        use crate::*;

        #[derive(Debug)]
        pub(crate) struct $function {
            elem: ArbitraryInput,
            set: SetInput,
            out: ManagedPort<bool>,
        }

        impl MechFunctionFactory for $function {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                FunctionValueRepresentation::Bool,
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::Set,
            );
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, element, set) = invocation.expect_binary()?;
                Ok(Box::new(Self {
                    elem: ArbitraryInput::canonical(element),
                    set: SetInput::canonical(set)?,
                    out: out.try_managed()?,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_SET_PREDICATE_CONTRACT)
            }
        }

        impl MechFunctionImpl for $function {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                let contains = self.set.contains(frame, &self.elem)?;
                let next = if $negated { !contains } else { contains };
                frame.with_port_init_writer(&self.out, |output| output.write_next(next))?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_SET_PREDICATE_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $function {
            fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let destination = compile_value_cell_register(self.out.cell(), context)?;
                let element = self.elem.compile_register(context)?;
                let set = self.set.compile_register(context)?;
                context.emit_binop(hash_str($name), destination, element, set);
                Ok(destination)
            }
        }

        #[cfg(feature = "source")]
        pub struct $specializer {}

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                crate::canonical::specialize_bool::<$function>(invocation, context)
            }
        }
    };
}

#[cfg(feature = "membership")]
pub(crate) use define_set_membership;
