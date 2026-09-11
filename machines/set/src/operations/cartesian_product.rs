#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::canonical::{SetInput, SetOutput};
use crate::*;

const MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetCartesianProductLimitExceeded {
    pub lhs: usize,
    pub rhs: usize,
    pub maximum: usize,
}

impl MechErrorKind for SetCartesianProductLimitExceeded {
    fn name(&self) -> &str {
        "SetCartesianProductLimitExceeded"
    }
    fn message(&self) -> String {
        format!(
            "set/cartesian-product inputs have cardinalities {} and {}, exceeding the maximum output cardinality of {}",
            self.lhs, self.rhs, self.maximum,
        )
    }
}

fn cartesian_product_output_len(lhs: usize, rhs: usize) -> MResult<usize> {
    let output_len = lhs.checked_mul(rhs).ok_or_else(|| {
        MechError::new(
            SetCartesianProductLimitExceeded {
                lhs,
                rhs,
                maximum: MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc()
    })?;
    if output_len > MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY {
        return Err(MechError::new(
            SetCartesianProductLimitExceeded {
                lhs,
                rhs,
                maximum: MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(output_len)
}

#[derive(Debug)]
pub(crate) struct SetCartesianProductFxn {
    lhs: SetInput,
    rhs: SetInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetCartesianProductFxn {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalSortUnique
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: SetInput::canonical(lhs)?,
            rhs: SetInput::canonical(rhs)?,
            out: SetOutput::canonical(out)?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_BINARY_CONTRACT)
    }
}

impl MechFunctionImpl for SetCartesianProductFxn {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        let output_len = cartesian_product_output_len(
            self.lhs.planning_cardinality()?,
            self.rhs.planning_cardinality()?,
        )?;
        Ok(Some(
            vec![self
                .out
                .prospective_expansion_footprint(&[&self.lhs, &self.rhs], output_len)?]
            .into_boxed_slice(),
        ))
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.out.primary_state_port()
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.out.transaction_state_ports()
    }
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let output_len = cartesian_product_output_len(
            self.lhs.planning_cardinality()?,
            self.rhs.planning_cardinality()?,
        )?;
        let footprint = self
            .out
            .prospective_expansion_footprint(&[&self.lhs, &self.rhs], output_len)?;
        self.out
            .with_admitted_set_drafts(frame, footprint, |frame| {
                let lhs = self.lhs.element_drafts(frame)?.into_vec();
                let rhs = self.rhs.element_drafts(frame)?.into_vec();
                let mut next = Vec::with_capacity(output_len);
                for lhs in &lhs {
                    for rhs in &rhs {
                        next.push(ValueDataDraft::Tuple(
                            vec![lhs.clone(), rhs.clone()].into_boxed_slice(),
                        ));
                    }
                }
                Ok(next.into_boxed_slice())
            })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_BINARY_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetCartesianProductFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("SetCartesianProductFxn"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetCartesianProduct {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetCartesianProduct {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetCartesianProductFxn>(invocation, context)
    }
}
