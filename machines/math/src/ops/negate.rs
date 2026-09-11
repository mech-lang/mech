use crate::*;

// Factory type parameters remain implementation markers. They do not own a
// scalar or matrix; the frame resolves the current logical port backing.
pub(crate) trait ManagedNegationBacking: FunctionPortBacking {
    type Element: ManagedElement + FunctionPortBacking + RuntimeCheckedNeg + Debug;
}

macro_rules! scalar_negation_backing {
    ($($type:ty => $feature:literal),+ $(,)?) => { $(
        #[cfg(feature = $feature)]
        impl ManagedNegationBacking for $type {
            type Element = Self;
        }
    )+ };
}

scalar_negation_backing!(
    i8 => "i8", i16 => "i16", i32 => "i32", i64 => "i64", i128 => "i128",
    f32 => "f32", f64 => "f64", C64 => "complex", R64 => "rational",
);

macro_rules! matrix_negation_backing {
    ($($matrix:ident => $feature:literal),+ $(,)?) => { $(
        #[cfg(feature = $feature)]
        impl<T> ManagedNegationBacking for $matrix<T>
        where
            T: ManagedElement + FunctionPortBacking + RuntimeCheckedNeg + Debug,
        {
            type Element = T;
        }
    )+ };
}

matrix_negation_backing!(
    Matrix1 => "matrix1", Matrix2 => "matrix2", Matrix3 => "matrix3",
    Matrix4 => "matrix4", Matrix2x3 => "matrix2x3", Matrix3x2 => "matrix3x2",
    RowVector2 => "row_vector2", RowVector3 => "row_vector3", RowVector4 => "row_vector4",
    RowDVector => "row_vectord", Vector2 => "vector2", Vector3 => "vector3",
    Vector4 => "vector4", DVector => "vectord", DMatrix => "matrixd",
);

macro_rules! managed_negation {
    ($factory:ident) => {
        #[derive(Debug)]
        pub(crate) struct $factory<O: ManagedNegationBacking> {
            arg: ManagedPort<O::Element>,
            out: ManagedPort<O::Element>,
            marker: PhantomData<fn() -> O>,
        }

        impl<O> MechFunctionFactory for $factory<O>
        where
            O: ManagedNegationBacking + FunctionStateBacking + Debug,
        {
            const SIGNATURE: RuntimeFunctionSignature =
                RuntimeFunctionSignature::unary(O::REPRESENTATION, O::REPRESENTATION);

            fn implementation_memory_class() -> ImplementationMemoryClass {
                ImplementationMemoryClass::NoAdditionalScratch
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg) = invocation.expect_unary()?;
                Ok(Box::new(Self {
                    arg: arg.try_managed_element::<O::Element>()?,
                    out: out.try_managed_element::<O::Element>()?,
                    marker: PhantomData,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(crate::managed_unary::unary_full_write_contract(
                    O::REPRESENTATION,
                ))
            }
        }

        impl<O> MechFunctionImpl for $factory<O>
        where
            O: ManagedNegationBacking + FunctionStateBacking + Debug,
        {
            fn solve_managed(
                &self,
                frame: &mut KernelMemoryFrame<'_>,
                _services: &mut dyn MechExecutionServices,
            ) -> MResult<ReactiveSolveStatus> {
                frame.with_unary_port_views(&self.arg, &self.out, |input, output| {
                    let rows = output.rows();
                    let geometry_error = || MemoryRuntimeError::InvalidLayout {
                        object: None,
                        size: input.len() as u64,
                        alignment: core::mem::align_of::<O::Element>() as u32,
                        reason: "negation input and output geometry disagree",
                    };
                    if (input.rows(), input.columns()) != (rows, output.columns()) {
                        return Err(geometry_error().into());
                    }
                    output.try_fill_column_major(|index| {
                        input
                            .get(index % rows, index / rows)
                            .ok_or_else(geometry_error)?
                            .runtime_checked_neg()
                            .ok_or_else(|| arithmetic_overflow::<O::Element>("negation"))
                    })
                })?;
                Ok(ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(crate::managed_unary::unary_full_write_contract(
                    O::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl<O: ManagedNegationBacking> MechFunctionCompiler for $factory<O> {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($factory), O::REPRESENTATION);
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let arg = compile_value_cell_register(self.arg.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_unop(function, out, arg);
                Ok(out)
            }
        }
    };
}

managed_negation!(NegateS);
managed_negation!(NegateV);

impl_canonical_registered_math_unop_specializer!(MathNegate, "NegateS");

#[cfg(all(test, feature = "i8"))]
mod canonical_port_tests {
    use super::*;

    fn i8_value(cell: &ValueCell) -> i8 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::I8(value) = snapshot.data() else {
            panic!("expected i8 negate output")
        };
        *value
    }

    #[test]
    fn negation_uses_exact_ports_and_rejects_overflow_atomically() {
        let input = ValueCell::from_exact(7_i8).unwrap();
        let output = ValueCell::from_exact(0_i8).unwrap();
        let function = crate::catalog::bind_test_unary::<NegateS<i8>>(
            "math/neg",
            "NegateS<i8>",
            input.clone(),
            output.clone(),
        );
        function.instance().solve_result().unwrap();
        assert_eq!(i8_value(&output), -7);

        input
            .replace(&ValueCell::from_exact(i8::MIN).unwrap().snapshot().unwrap())
            .unwrap();
        assert_eq!(
            function.instance().solve_result().unwrap_err().kind_name(),
            "MathArithmeticOverflow"
        );
        assert_eq!(i8_value(&output), -7);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            output.replace(&ValueCell::from_exact(99_i8)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(i8_value(&output), -7);
    }
}
