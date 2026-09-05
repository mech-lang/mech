use crate::*;
use num_traits::*;

// Mod ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathRemainderInvalid {
    pub operand_type: &'static str,
}

impl MechErrorKind for MathRemainderInvalid {
    fn name(&self) -> &str {
        "MathRemainderInvalid"
    }

    fn message(&self) -> String {
        format!(
            "remainder is undefined or overflows for operand type {}",
            self.operand_type,
        )
    }
}

pub trait RuntimeCheckedRem: Copy {
    fn runtime_checked_rem(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_rem {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedRem for $type {
                fn runtime_checked_rem(self, rhs: Self) -> Option<Self> {
                    self.checked_rem(rhs)
                }
            }
        )+
    };
}

impl_checked_integer_rem!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

impl RuntimeCheckedRem for f32 {
    fn runtime_checked_rem(self, rhs: Self) -> Option<Self> {
        Some(self % rhs)
    }
}

impl RuntimeCheckedRem for f64 {
    fn runtime_checked_rem(self, rhs: Self) -> Option<Self> {
        Some(self % rhs)
    }
}

fn checked_runtime_rem<T: RuntimeCheckedRem>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_rem(rhs).ok_or_else(|| {
        MechError::new(
            MathRemainderInvalid {
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

#[macro_export]
macro_rules! impl_binop2 {
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
                + Rem<Output = T>
                + RemAssign
                + Zero
                + One
                + FunctionRuntimeType
                + RuntimeCheckedRem,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + CompileConst + ConstElem,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(super::arithmetic_full_write_contract(
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
                + Rem<Output = T>
                + RemAssign
                + Zero
                + One
                + RuntimeCheckedRem,
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
                Some(super::arithmetic_full_write_contract(
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
                + RuntimeCheckedRem,
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

macro_rules! mod_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_rem(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! mod_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (o, (l, r)) in next.iter_mut().zip(lhs_deref.iter().zip(rhs_deref.iter())) {
                *o = checked_runtime_rem(*l, *r)?;
            }
            *$out = next;
        }
    };
}

macro_rules! mod_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = (*$rhs);
            for (o, l) in next.iter_mut().zip(lhs_deref.iter()) {
                *o = checked_runtime_rem(*l, rhs_deref)?;
            }
            *$out = next;
        }
    };
}

macro_rules! mod_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = (*$lhs);
            let rhs_deref = &(*$rhs);
            for (o, r) in next.iter_mut().zip(rhs_deref.iter()) {
                *o = checked_runtime_rem(lhs_deref, *r)?;
            }
            *$out = next;
        }
    };
}

macro_rules! mod_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in next.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_rem(lhs_col[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! mod_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in next.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_rem(lhs_deref[i], rhs_col[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! mod_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in next.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_rem(lhs_row[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! mod_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in next.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_rem(lhs_deref[i], rhs_row[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! impl_math_fxns2 {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop2);
    };
}

impl_math_fxns2!(Mod);

#[cfg(all(test, feature = "i32"))]
mod tests {
    use super::*;

    #[test]
    fn integer_remainder_rejects_zero_and_signed_overflow_on_reactive_resolve() {
        let lhs = Ref::new(i32::MIN);
        let rhs = Ref::new(2_i32);
        let out = Ref::new(17_i32);
        let function = ModSS {
            lhs,
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        let previous = *out.borrow();
        for invalid in [-1, 0] {
            *rhs.borrow_mut() = invalid;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathRemainderInvalid");
            assert_eq!(*out.borrow(), previous);
        }
    }
}

impl_canonical_registered_math_binop_specializer!(MathMod, "Mod");
