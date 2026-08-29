use crate::canonical::{SetInput, SetOutput};
#[cfg(feature = "source")]
use crate::canonical::specialize_dynamic_set;
use crate::*;

const MAX_POWERSET_INPUT_CARDINALITY: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetPowersetLimitExceeded {
    pub found: usize,
    pub maximum: usize,
}

impl MechErrorKind for SetPowersetLimitExceeded {
    fn name(&self) -> &str {
        "SetPowersetLimitExceeded"
    }
    fn message(&self) -> String {
        format!(
            "set/powerset input has {} elements; the maximum supported cardinality is {}",
            self.found, self.maximum,
        )
    }
}

fn powerset_output_len(input_cardinality: usize) -> MResult<usize> {
    if input_cardinality > MAX_POWERSET_INPUT_CARDINALITY {
        return Err(MechError::new(
            SetPowersetLimitExceeded {
                found: input_cardinality,
                maximum: MAX_POWERSET_INPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc());
    }
    1usize.checked_shl(input_cardinality as u32).ok_or_else(|| {
        MechError::new(
            SetPowersetLimitExceeded {
                found: input_cardinality,
                maximum: MAX_POWERSET_INPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc()
    })
}

fn powerset<T: Clone>(set: &[T]) -> Vec<Vec<T>> {
    let mut subsets = vec![Vec::new()];
    for element in set {
        let with_element = subsets
            .iter()
            .map(|subset| {
                let mut next = subset.clone();
                next.push(element.clone());
                next
            })
            .collect::<Vec<_>>();
        subsets.extend(with_element);
    }
    subsets.sort_by_key(Vec::len);
    subsets
}

#[derive(Debug)]
pub(crate) struct SetPowersetFxn {
    input: SetInput,
    out: SetOutput,
}

impl MechFunctionFactory for SetPowersetFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );
    const OUTPUT_SCHEMA_RULE: FunctionOutputSchemaRule =
        FunctionOutputSchemaRule::DynamicSetPowerset;
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, input) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            input: SetInput::canonical(input)?,
            out: SetOutput::canonical(out)?,
        }))
    }
}

impl MechFunctionImpl for SetPowersetFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        self.out.primary_state_port()
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        self.out.transaction_state_ports()
    }
    fn solve_result(&self) -> MResult<()> {
        let elements = self
            .input
            .canonical_value()
            .set_element_drafts()?
            .into_vec();
        let output_len = powerset_output_len(elements.len())?;
        let subsets = powerset(&elements);
        debug_assert_eq!(subsets.len(), output_len);
        self.out.canonical_value().replace_set_drafts(
            subsets
                .into_iter()
                .map(|subset| ValueDataDraft::Set(subset.into_boxed_slice()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetPowersetFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let input = self.input.compile_register(ctx)?;
        ctx.emit_unop(hash_str("SetPowersetFxn"), destination, input);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct SetPowerset {}

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for SetPowerset {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_dynamic_set::<SetPowersetFxn>(invocation)
    }
}
