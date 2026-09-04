use crate::*;

#[derive(Debug)]
#[cfg(any(feature = "membership", feature = "modify"))]
pub(crate) struct ArbitraryInput(FunctionValueInput);

#[cfg(any(feature = "membership", feature = "modify"))]
impl ArbitraryInput {
    pub(crate) fn canonical(port: FunctionInputPort<'_>) -> Self {
        Self(port.value())
    }

    pub(crate) fn canonical_value(&self) -> &FunctionValueInput {
        &self.0
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

    pub(crate) fn canonical_value(&self) -> &FunctionValueInput {
        &self.0
    }

    #[cfg(feature = "relations")]
    pub(crate) fn relation(&self, other: &Self, relation: SetRelation) -> MResult<bool> {
        let relation = match relation {
            SetRelation::Disjoint => SetValueRelation::Disjoint,
            SetRelation::Equal => SetValueRelation::Equal,
            SetRelation::NotEqual => SetValueRelation::NotEqual,
            SetRelation::ProperSubset => SetValueRelation::ProperSubset,
            SetRelation::ProperSuperset => SetValueRelation::ProperSuperset,
            SetRelation::Subset => SetValueRelation::Subset,
            SetRelation::Superset => SetValueRelation::Superset,
        };
        self.0.set_relation(&other.0, relation)
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

    pub(crate) fn canonical_value(&self) -> &FunctionValueOutput {
        &self.0
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
        RuntimeBindingSelector::Operation(context.resolved_call()?.operation),
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
        RuntimeBindingSelector::Operation(context.resolved_call()?.operation),
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
        RuntimeBindingSelector::Operation(context.resolved_call()?.operation),
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
            out: Ref<bool>,
        }

        impl MechFunctionFactory for $function {
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
                    out: out.try_ref()?,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_SET_PREDICATE_CONTRACT)
            }
        }

        impl MechFunctionImpl for $function {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn solve_result(&self) -> MResult<()> {
                *self.out.borrow_mut() = self.lhs.relation(&self.rhs, SetRelation::$relation)?;
                Ok(())
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
                let destination = compile_register_brrw!(self.out, context);
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
            out: Ref<bool>,
        }

        impl MechFunctionFactory for $function {
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
                    out: out.try_ref()?,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_SET_PREDICATE_CONTRACT)
            }
        }

        impl MechFunctionImpl for $function {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn solve_result(&self) -> MResult<()> {
                let contains = self
                    .set
                    .canonical_value()
                    .set_contains(self.elem.canonical_value())?;
                *self.out.borrow_mut() = if $negated { !contains } else { contains };
                Ok(())
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
                let destination = compile_register_brrw!(self.out, context);
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
