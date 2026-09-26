//! Boolean conjunction over every element of a scalar, vector, or matrix.
//! The empty reduction is true. No numeric-to-Boolean conversion is applied.

use crate::*;

#[derive(Debug)]
pub struct All {
    arg: ManagedPort<bool>,
    out: ManagedPort<bool>,
}

pub(crate) fn validate_all(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [input] = inputs else {
        return Err(function_shape_contract_violation(
            "logic/all",
            "expected one Boolean scalar or matrix",
        ));
    };
    let valid_input = match input.closed_schema_body()? {
        SchemaBody::Bool => true,
        SchemaBody::Matrix { element, .. } => *element == SchemaBody::Bool,
        _ => false,
    };
    if !valid_input || output.closed_schema_body()? != SchemaBody::Bool {
        return Err(function_shape_contract_violation(
            "logic/all",
            "expected a Boolean scalar or matrix input and a scalar Boolean output",
        ));
    }
    Ok(())
}

impl MechFunctionFactory for All {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
    );

    fn implementation_memory_class() -> ImplementationMemoryClass {
        ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            arg: arg.try_managed_element::<bool>()?,
            out: out.try_managed::<bool>()?,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&crate::PURE_LOGIC_UNARY_EXACT_SCALAR)
    }
}

impl MechFunctionImpl for All {
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_unary_port_views(&self.arg, &self.out, |arg, out| {
            if out.len() != 1 {
                return Err(function_shape_contract_violation(
                    "logic/all",
                    "the result must be a scalar Boolean",
                ));
            }
            let result = (0..arg.len()).all(|index| {
                arg.get_column_major(index)
                    .expect("Boolean reduction index is in bounds")
            });
            out.try_fill_column_major(|_| Ok(result))
        })?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Self::declared_operation_contract()
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for All {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(self.out.cell(), ctx)?;
        let input = compile_value_cell_register(self.arg.cell(), ctx)?;
        let function = ctx.function_id("logic/all")?;
        ctx.emit_unop(function, output, input);
        Ok(output)
    }
}

#[cfg(feature = "source")]
pub struct LogicAll;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for LogicAll {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = invocation.input(0).expect("validated unary input");
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            ExecutionTarget::DirectRuntime,
            vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
            &[input],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: ValueCell, expected: bool) {
        let output = ValueCell::from_exact(!expected).unwrap();
        validate_all(&output, &[input.clone()]).unwrap();
        let invocation = FunctionInvocation::unary(output.clone(), input);
        let implementation = All::new_invocation(invocation.clone()).unwrap();
        let function = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name(
                "logic/all",
                All::declared_operation_contract().unwrap().clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("logic/all"),
            ExecutionTarget::DirectRuntime,
            All::implementation_memory_class(),
        )
        .unwrap();
        function.instance().solve_result().unwrap();
        let value = output.snapshot().unwrap();
        assert!(matches!(value.data(), ValueData::Bool(actual) if *actual == expected));
    }

    #[test]
    fn scalar_identity() {
        check(ValueCell::from_exact(true).unwrap(), true);
        check(ValueCell::from_exact(false).unwrap(), false);
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn all_matrix_elements_and_empty_identity() {
        for (rows, columns) in [(1, 3), (3, 1), (2, 3), (0, 0), (0, 3), (3, 0)] {
            let matrix = |values: Vec<bool>| {
                ValueCell::from_exact_matrix_ref(
                    Ref::new(nalgebra::DMatrix::from_vec(rows, columns, values)),
                    rows,
                    columns,
                )
                .unwrap()
            };
            check(matrix(vec![true; rows * columns]), true);
            for index in 0..rows * columns {
                let mut values = vec![true; rows * columns];
                values[index] = false;
                check(matrix(values), false);
            }
        }
    }

    #[test]
    fn rejects_non_boolean_input() {
        let output = ValueCell::from_exact(false).unwrap();
        let input = ValueCell::from_exact(1usize).unwrap();
        assert!(validate_all(&output, &[input.clone()]).is_err());
        assert!(All::new_invocation(FunctionInvocation::unary(output, input)).is_err());
    }
}
