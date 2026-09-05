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
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.out.primary_state_port()
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.out.transaction_state_ports()
    }
    fn solve_result(&self) -> MResult<()> {
        let lhs = self.lhs.canonical_value().set_element_drafts()?.into_vec();
        let rhs = self.rhs.canonical_value().set_element_drafts()?.into_vec();
        let output_len = cartesian_product_output_len(lhs.len(), rhs.len())?;
        let mut next = Vec::with_capacity(output_len);
        for lhs in &lhs {
            for rhs in &rhs {
                next.push(ValueDataDraft::Tuple(
                    vec![lhs.clone(), rhs.clone()].into_boxed_slice(),
                ));
            }
        }
        self.out
            .canonical_value()
            .replace_set_drafts(next.into_boxed_slice())
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
