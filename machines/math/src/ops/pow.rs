use crate::*;
use num_traits::*;

fn checked_runtime_pow<T: RuntimeCheckedPow>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_pow(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("exponentiation"))
}

// Pow ------------------------------------------------------------------------

macro_rules! pow_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_pow(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! pow_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$lhs).len() {
                next[i] = checked_runtime_pow((&*$lhs)[i], (&*$rhs)[i])?;
            }
            *$out = next;
        }
    };
}

macro_rules! pow_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$lhs).len() {
                next[i] = checked_runtime_pow((&*$lhs)[i], *$rhs)?;
            }
            *$out = next;
        }
    };
}

macro_rules! pow_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$rhs).len() {
                next[i] = checked_runtime_pow(*$lhs, (&*$rhs)[i])?;
            }
            *$out = next;
        }
    };
}

macro_rules! pow_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in next.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_pow(lhs_col[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! pow_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in next.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_pow(lhs_deref[i], rhs_col[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! pow_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in next.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_pow(lhs_row[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! pow_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in next.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_pow(lhs_deref[i], rhs_row[i])?;
                }
            }
            *$out = next;
        }
    };
}

#[macro_export]
macro_rules! impl_powop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            lhs: Ref<$arg1_type>,
            rhs: Ref<$arg2_type>,
            out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Pow<T, Output = T>
                + RuntimeCheckedPow
                + FunctionRuntimeType
                + Zero
                + One,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + CompileConst + ConstElem,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
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
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs: Ref<$arg1_type> = lhs.try_ref()?;
                let rhs: Ref<$arg2_type> = rhs.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new(Self { lhs, rhs, out }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Pow<T, Output = T>
                + RuntimeCheckedPow
                + Zero
                + One,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
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
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CanonicalMatrixElementBacking
                + CompileConst
                + ConstElem
                + FunctionRuntimeType
                + RuntimeCheckedPow,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_math_fxns_pow {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_powop);
    };
}

impl_math_fxns_pow!(Pow);

#[cfg(all(feature = "rational", feature = "i32"))]
#[derive(Debug)]
pub struct PowRational {
    pub lhs: Ref<R64>,
    pub rhs: Ref<i32>,
    pub out: Ref<R64>,
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
        let lhs: Ref<R64> = lhs.try_ref()?;
        let rhs: Ref<i32> = rhs.try_ref()?;
        let out: Ref<R64> = out.try_ref()?;
        Ok(Box::new(Self { lhs, rhs, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::ops::arithmetic_full_write_contract(
            FunctionValueRepresentation::R64,
        ))
    }
}

#[cfg(all(feature = "rational", feature = "i32"))]
impl MechFunctionImpl for PowRational {
    fn solve_result(&self) -> MResult<()> {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            (*out_ptr).0 = (*lhs_ptr).0.pow(*rhs_ptr);
        };
        Ok(())
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::ops::arithmetic_full_write_contract(
            FunctionValueRepresentation::R64,
        ))
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}

#[cfg(all(feature = "rational", feature = "i32", feature = "semantic-compiler"))]
impl MechFunctionCompiler for PowRational {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "PowRational<{}>",
            <R64 as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
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
        let function = PowRational::new_invocation(FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact(R64::new(3, 2)).unwrap(),
            ValueCell::from_exact(2_i32).unwrap(),
        ))
        .unwrap();

        function.solve_result().unwrap();
        assert!(output.same_cell(&output_alias));
        assert_eq!(
            function.transaction_state_ports().unwrap().unwrap().len(),
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
        let rhs = Ref::new(1_u8);
        let out = Ref::new(17_u8);
        let function = PowSS {
            lhs: Ref::new(20_u8),
            rhs: rhs.clone(),
            out: out.clone(),
        };
        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 20);
        *rhs.borrow_mut() = 2;
        assert_eq!(
            function.solve_result().unwrap_err().kind_name(),
            "MathArithmeticOverflow"
        );
        assert_eq!(*out.borrow(), 20);
    }
}
