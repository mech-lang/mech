use crate::*;
#[cfg(feature = "f64")]
use libm::atan2;
#[cfg(feature = "f32")]
use libm::atan2f;

// Atan2 ------------------------------------------------------------------------

trait RuntimeAtan2: Copy {
    fn runtime_atan2(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeAtan2 for f32 {
    fn runtime_atan2(self, rhs: Self) -> Self {
        atan2f(self, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeAtan2 for f64 {
    fn runtime_atan2(self, rhs: Self) -> Self {
        atan2(self, rhs)
    }
}

macro_rules! impl_atan2_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            arg1: ManagedPort<T>,
            arg2: ManagedPort<T>,
            out: ManagedPort<T>,
            marker: PhantomData<($arg1_type, $arg2_type, $out_type)>,
        }

        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: RuntimeAtan2
                + ManagedElement
                + FunctionPortBacking
                + FunctionRuntimeType
                + std::fmt::Debug,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(crate::ops::arithmetic_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                let _ = arg1.try_managed::<$arg1_type>()?;
                let _ = arg2.try_managed::<$arg2_type>()?;
                let _ = out.try_managed::<$out_type>()?;
                let arg1 = arg1.try_managed_element::<T>()?;
                let arg2 = arg2.try_managed_element::<T>()?;
                let out = out.try_managed_element::<T>()?;
                Ok(Box::new($struct_name {
                    arg1,
                    arg2,
                    out,
                    marker: PhantomData,
                }))
            }
        }

        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: RuntimeAtan2 + ManagedElement + FunctionPortBacking + std::fmt::Debug,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_binary_port_views(
                    &self.arg1,
                    &self.arg2,
                    &self.out,
                    |arg1, arg2, out| {
                        let rows = out.rows();
                        let columns = out.columns();
                        out.try_fill_column_major(|index| {
                            let row = index % rows;
                            let column = index / rows;
                            let arg1 = crate::ops::managed_broadcast_element(
                                &arg1, row, column, rows, columns,
                            )?;
                            let arg2 = crate::ops::managed_broadcast_element(
                                &arg2, row, column, rows, columns,
                            )?;
                            Ok(arg1.runtime_atan2(arg2))
                        })
                    },
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(crate::ops::arithmetic_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
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
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: RuntimeAtan2
                + ManagedElement
                + FunctionPortBacking
                + FunctionRuntimeType
                + CanonicalMatrixElementBacking
                + ConstElem
                + CompileConst
                + std::fmt::Debug,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                let output = compile_value_cell_register(self.out.cell(), ctx)?;
                let arg1 = compile_value_cell_register(self.arg1.cell(), ctx)?;
                let arg2 = compile_value_cell_register(self.arg2.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, output, arg1, arg2);
                Ok(output)
            }
        }
    };
}

impl_fxns!(Atan2, T, T, impl_atan2_binop);

impl_canonical_math_same_type_binop_specializer!(MathAtan2, Atan2, "math/atan2");

#[cfg(all(test, feature = "runtime", feature = "f64"))]
mod canonical_port_tests {
    use super::*;

    #[test]
    fn scalar_atan2_uses_exact_ports_and_typed_state() {
        let output = ValueCell::from_exact(0.0_f64).unwrap();
        let alias = output.clone();
        let invocation = FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact(1.0_f64).unwrap(),
            ValueCell::from_exact(1.0_f64).unwrap(),
        );
        let implementation = Atan2SS::<f64>::new_invocation(invocation.clone()).unwrap();
        let operation = ResolvedOperationDescriptor::from_name(
            "math/atan2",
            Atan2SS::<f64>::declared_operation_contract()
                .unwrap()
                .clone(),
        )
        .unwrap();
        let function = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            operation,
            RuntimeFunctionId::from_name("Atan2SS<f64>"),
            ExecutionTarget::DirectRuntime,
            Atan2SS::<f64>::implementation_memory_class(),
        )
        .unwrap();
        function.instance().solve_result().unwrap();
        let value = output.snapshot().unwrap();
        let ValueData::F64(value) = value.data() else {
            panic!("expected f64 atan2 output")
        };
        assert!((value.to_f64() - core::f64::consts::FRAC_PI_4).abs() < f64::EPSILON);
        assert!(output.same_cell(&alias));

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            output.replace(&ValueCell::from_exact(99.0_f64)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        let restored = output.snapshot().unwrap();
        let ValueData::F64(restored) = restored.data() else {
            panic!("expected restored f64 atan2 output")
        };
        assert!((restored.to_f64() - core::f64::consts::FRAC_PI_4).abs() < f64::EPSILON);
    }
}
