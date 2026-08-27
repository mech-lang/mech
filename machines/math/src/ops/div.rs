use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
use num_traits::*;

// Div ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathDivisionInvalid {
    pub operand_type: &'static str,
}

impl MechErrorKind for MathDivisionInvalid {
    fn name(&self) -> &str {
        "MathDivisionInvalid"
    }

    fn message(&self) -> String {
        format!(
            "division is undefined or overflows for operand type {}",
            self.operand_type,
        )
    }
}

pub trait RuntimeCheckedDiv: Copy {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_div {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedDiv for $type {
                fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
                    self.checked_div(rhs)
                }
            }
        )+
    };
}

impl_checked_integer_div!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

impl RuntimeCheckedDiv for f32 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        Some(self / rhs)
    }
}

impl RuntimeCheckedDiv for f64 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        Some(self / rhs)
    }
}

#[cfg(feature = "rational")]
impl RuntimeCheckedDiv for R64 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        self.checked_div(rhs)
    }
}

#[cfg(feature = "complex")]
impl RuntimeCheckedDiv for C64 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        Some(self / rhs)
    }
}

fn checked_runtime_div<T: RuntimeCheckedDiv>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_div(rhs).ok_or_else(|| {
        MechError::new(
            MathDivisionInvalid {
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

macro_rules! impl_checked_div_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            pub lhs: Ref<$arg1_type>,
            pub rhs: Ref<$arg2_type>,
            pub out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            #[cfg(feature = "semantic-compiler")]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + CompileConst
                + AsValueKind
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One
                + RuntimeCheckedDiv,
            #[cfg(not(feature = "semantic-compiler"))]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + AsValueKind
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One
                + RuntimeCheckedDiv,
            Ref<$out_type>: ToValue,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
            $out_type: FunctionRuntimeType + FunctionPortBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs: Ref<$arg1_type> = lhs.try_ref()?;
                let rhs: Ref<$arg2_type> = rhs.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new(Self { lhs, rhs, out }))
            }

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Copy
                + Debug
                + Display
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
                + Zero
                + One
                + RuntimeCheckedDiv,
            Ref<$out_type>: ToValue,
            $out_type: FunctionRuntimeType,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(super::arithmetic_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind + RuntimeCheckedDiv,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

macro_rules! div_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_div(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! div_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for (output, (lhs, rhs)) in next.iter_mut().zip((*$lhs).iter().zip((*$rhs).iter())) {
                *output = checked_runtime_div(*lhs, *rhs)?;
            }
            *$out = next;
        }
    };
}

macro_rules! div_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$lhs).len() {
                next[i] = checked_runtime_div((&*$lhs)[i], *$rhs)?;
            }
            *$out = next;
        }
    };
}

macro_rules! div_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for i in 0..(&*$rhs).len() {
                next[i] = checked_runtime_div(*$lhs, (&*$rhs)[i])?;
            }
            *$out = next;
        }
    };
}

macro_rules! div_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in next.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_div(lhs_col[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! div_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in next.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_div(lhs_deref[i], rhs_col[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! div_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in next.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_div(lhs_row[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

macro_rules! div_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in next.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_div(lhs_deref[i], rhs_row[i])?;
                }
            }
            *$out = next;
        }
    };
}

impl_fxns!(Div, T, T, impl_checked_div_binop);

#[cfg(all(test, feature = "i32"))]
mod tests {
    use super::*;

    #[test]
    fn integer_division_rejects_zero_and_signed_overflow_on_reactive_resolve() {
        let lhs = Ref::new(i32::MIN);
        let rhs = Ref::new(2_i32);
        let out = Ref::new(17_i32);
        let function = DivSS {
            lhs,
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        let previous = *out.borrow();
        for invalid in [-1, 0] {
            *rhs.borrow_mut() = invalid;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathDivisionInvalid");
            assert_eq!(*out.borrow(), previous);
        }
    }
}

#[cfg(feature = "source")]
fn impl_div_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_binop_match_arms!(
      Div,
      (lhs_value, rhs_value),
      I8,   i8,   "i8";
      I16,  i16,  "i16";
      I32,  i32,  "i32";
      I64,  i64,  "i64";
      I128, i128, "i128";
      U8,   u8,   "u8";
      U16,  u16,  "u16";
      U32,  u32,  "u32";
      U64,  u64,  "u64";
      U128, u128, "u128";
      F32,  f32,  "f32";
      F64,  f64,  "f64";
      R64, R64, "rational";
      C64, C64, "complex";
    )
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(MathDiv, impl_div_fxn, "math/div");
