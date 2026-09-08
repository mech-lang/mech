use crate::*;
#[cfg(feature = "f64")]
use libm::atan2;
#[cfg(feature = "f32")]
use libm::atan2f;

// Atan2 ------------------------------------------------------------------------

macro_rules! impl_two_arg_fxn {
    ($struct_name:ident, $element:ty, $kind1:ty, $kind2:ty, $out_kind:ty, $op:path) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name {
            arg1: ManagedPort<$element>,
            arg2: ManagedPort<$element>,
            out: ManagedPort<$element>,
            marker: PhantomData<($kind1, $kind2, $out_kind)>,
        }
        impl MechFunctionFactory for $struct_name
        where
            $kind1: FunctionPortBacking,
            $kind2: FunctionPortBacking,
            $out_kind: FunctionStateBacking,
        {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_kind as FunctionRuntimeType>::REPRESENTATION,
                <$kind1 as FunctionRuntimeType>::REPRESENTATION,
                <$kind2 as FunctionRuntimeType>::REPRESENTATION,
            );

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(crate::ops::arithmetic_full_write_contract(
                    <$out_kind as FunctionRuntimeType>::REPRESENTATION,
                ))
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                let _ = arg1.try_managed::<$kind1>()?;
                let _ = arg2.try_managed::<$kind2>()?;
                let _ = out.try_managed::<$out_kind>()?;
                let arg1 = arg1.try_managed_element::<$element>()?;
                let arg2 = arg2.try_managed_element::<$element>()?;
                let out = out.try_managed_element::<$element>()?;
                Ok(Box::new($struct_name {
                    arg1,
                    arg2,
                    out,
                    marker: PhantomData,
                }))
            }
        }
        impl MechFunctionImpl for $struct_name {
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
                        if arg1.len() != arg2.len() || arg1.len() != out.len() {
                            return Err(function_shape_contract_violation(
                                "math/atan2",
                                "managed input and output geometry disagree",
                            ));
                        }
                        out.try_fill_column_major(|index| {
                            let arg1 = arg1
                                .get_column_major(index)
                                .expect("validated atan2 input lane");
                            let arg2 = arg2
                                .get_column_major(index)
                                .expect("validated atan2 input lane");
                            Ok($op(arg1, arg2))
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
                    <$out_kind as FunctionRuntimeType>::REPRESENTATION,
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
        impl MechFunctionCompiler for $struct_name {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let mut registers = [0, 0, 0];

                registers[0] = compile_value_cell_register(self.out.cell(), ctx)?;
                registers[1] = compile_value_cell_register(self.arg1.cell(), ctx)?;
                registers[2] = compile_value_cell_register(self.arg2.cell(), ctx)?;

                ctx.emit_binop(
                    hash_str(stringify!($struct_name)),
                    registers[0],
                    registers[1],
                    registers[2],
                );

                return Ok(registers[0]);
            }
        }
    };
}

macro_rules! impl_atan2 {
  ($type:tt, $type_string:tt, $op:ident, $($struct_name:ident, $kind:ty, $feature:literal);* $(;)?) => {
    paste!{
      $(
        #[cfg(all(feature = $type_string, feature = $feature))]
        impl_two_arg_fxn!([<$struct_name $type:camel>], $type, $kind<$type>, $kind<$type>, $kind<$type>, $op);
      )*
    }
  };
}

impl_atan2!(
  f64, "f64", atan2,
  Atan2M1, Matrix1, "matrix1";
  Atan2M2, Matrix2, "matrix2";
  Atan2M3, Matrix3, "matrix3";
  Atan2M2x3, Matrix2x3, "matrix2x3";
  Atan2M3x2, Matrix3x2, "matrix3x2";
  Atan2M4, Matrix4, "matrix4";
  Atan2V2, Vector2, "vector2";
  Atan2V3, Vector3, "vector3";
  Atan2V4, Vector4, "vector4";
  Atan2R2, RowVector2, "row_vector2";
  Atan2R3, RowVector3, "row_vector3";
  Atan2R4, RowVector4, "row_vector4";
  Atan2RD, RowDVector, "row_vectord";
  Atan2VD, DVector, "vectord";
  Atan2MD, DMatrix, "matrixd";
);

impl_atan2!(
  f32, "f32", atan2f,
  Atan2M1, Matrix1, "matrix1";
  Atan2M2, Matrix2, "matrix2";
  Atan2M3, Matrix3, "matrix3";
  Atan2M2x3, Matrix2x3, "matrix2x3";
  Atan2M3x2, Matrix3x2, "matrix3x2";
  Atan2M4, Matrix4, "matrix4";
  Atan2V2, Vector2, "vector2";
  Atan2V3, Vector3, "vector3";
  Atan2V4, Vector4, "vector4";
  Atan2R2, RowVector2, "row_vector2";
  Atan2R3, RowVector3, "row_vector3";
  Atan2R4, RowVector4, "row_vector4";
  Atan2RD, RowDVector, "row_vectord";
  Atan2VD, DVector, "vectord";
  Atan2MD, DMatrix, "matrixd";
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(Atan2F32, f32, f32, f32, f32, atan2f);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(Atan2F64, f64, f64, f64, f64, atan2);

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
        let implementation = Atan2F64::new_invocation(invocation.clone()).unwrap();
        let operation = ResolvedOperationDescriptor::from_name(
            "math/atan2",
            Atan2F64::declared_operation_contract().unwrap().clone(),
        )
        .unwrap();
        let function = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            operation,
            RuntimeFunctionId::from_name("Atan2F64"),
            ExecutionTarget::DirectRuntime,
            Atan2F64::implementation_memory_class(),
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
