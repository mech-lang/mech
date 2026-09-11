use crate::*;
use num_traits::*;

fn checked_runtime_pow<T: RuntimeCheckedPow>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_pow(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("exponentiation"))
}

// Pow ------------------------------------------------------------------------

macro_rules! managed_pow_op {
    (@managed $lhs:expr, $rhs:expr) => {
        checked_runtime_pow($lhs, $rhs)
    };
}

macro_rules! impl_powop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(@bound RuntimeCheckedPow;
            $struct_name, $arg1_type, $arg2_type, $out_type, managed_pow_op,
            crate::managed_binary::arithmetic_full_write_contract);
    };
}
macro_rules! impl_math_fxns_pow {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_powop);
    };
}

impl_math_fxns_pow!(Pow);

#[cfg(all(feature = "rational", feature = "i32"))]
#[derive(Debug)]
pub struct PowRational {
    pub lhs: ManagedPort<R64>,
    pub rhs: ManagedPort<i32>,
    pub out: ManagedPort<R64>,
}

#[cfg(all(feature = "rational", feature = "i32"))]
impl MechFunctionFactory for PowRational {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::R64,
        FunctionValueRepresentation::R64,
        FunctionValueRepresentation::I32,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs = lhs.try_managed::<R64>()?;
        let rhs = rhs.try_managed::<i32>()?;
        let out = out.try_managed::<R64>()?;
        Ok(Box::new(Self { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::managed_binary::arithmetic_full_write_contract(
            FunctionValueRepresentation::R64,
        ))
    }
}

#[cfg(all(feature = "rational", feature = "i32"))]
impl MechFunctionImpl for PowRational {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_binary_typed_port_views(&self.lhs, &self.rhs, &self.out, |lhs, rhs, out| {
            if lhs.len() != 1 || rhs.len() != 1 || out.len() != 1 {
                return Err(
                    MechError::new(MemoryPlanError::DescriptorMismatch, None).with_compiler_loc()
                );
            }
            let next = R64(lhs.get(0, 0).unwrap().0.pow(rhs.get(0, 0).unwrap()));
            out.try_fill_column_major(|_| Ok(next))
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::managed_binary::arithmetic_full_write_contract(
            FunctionValueRepresentation::R64,
        ))
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
}

#[cfg(all(feature = "rational", feature = "i32", feature = "semantic-compiler"))]
impl MechFunctionCompiler for PowRational {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "PowRational<{}>",
            <R64 as FunctionRuntimeType>::REPRESENTATION
        );
        let out = compile_value_cell_register(self.out.cell(), ctx)?;
        let lhs = compile_value_cell_register(self.lhs.cell(), ctx)?;
        let rhs = compile_value_cell_register(self.rhs.cell(), ctx)?;
        let function = ctx.function_id(&name)?;
        ctx.emit_binop(function, out, lhs, rhs);
        Ok(out)
    }
}

impl_canonical_registered_math_binop_specializer!(MathPow, "Pow");

#[cfg(all(test, feature = "rational", feature = "i32"))]
mod rational_port_tests {
    use super::*;

    #[test]
    fn rational_factory_uses_mixed_exact_ports_and_typed_state() {
        let output = ValueCell::from_exact(R64::default()).unwrap();
        let output_alias = output.clone();
        let function = crate::catalog::bind_test_binary::<PowRational>(
            "math/pow",
            "PowRational<r64>",
            ValueCell::from_exact(R64::new(3, 2)).unwrap(),
            ValueCell::from_exact(2_i32).unwrap(),
            output.clone(),
        );

        function.instance().solve_result().unwrap();
        assert!(output.same_cell(&output_alias));
        assert_eq!(
            function
                .instance()
                .implementation()
                .transaction_state_ports()
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        let snapshot = output.snapshot().unwrap();
        assert!(matches!(
            snapshot.data(),
            ValueData::Rational64(value)
                if value.numerator() == 9 && value.denominator() == 4
        ));

        assert!(
            PowRational::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(R64::default()).unwrap(),
                ValueCell::from_exact(R64::new(3, 2)).unwrap(),
                ValueCell::from_exact(2_usize).unwrap(),
            ))
            .is_err()
        );
    }
}

#[cfg(all(test, feature = "u8"))]
mod checked_power_tests {
    use super::*;

    #[test]
    fn integer_power_rejects_overflow_without_publishing_partial_state() {
        let rhs = ValueCell::from_exact(1_u8).unwrap();
        let out = ValueCell::from_exact(17_u8).unwrap();
        let function = crate::catalog::bind_test_binary::<PowSS<u8>>(
            "math/pow",
            "PowSS<u8>",
            ValueCell::from_exact(20_u8).unwrap(),
            rhs.clone(),
            out.clone(),
        );
        function.instance().solve_result().unwrap();
        assert!(matches!(out.snapshot().unwrap().data(), ValueData::U8(20)));
        rhs.replace(&rhs.rebuild_data_draft(ValueDataDraft::U8(2)).unwrap())
            .unwrap();
        assert_eq!(
            function.instance().solve_result().unwrap_err().kind_name(),
            "MathArithmeticOverflow"
        );
        assert!(matches!(out.snapshot().unwrap().data(), ValueData::U8(20)));
    }
}
