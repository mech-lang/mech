#[macro_use]
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathArithmeticOverflow {
    pub operation: &'static str,
    pub operand_type: &'static str,
}

impl MechErrorKind for MathArithmeticOverflow {
    fn name(&self) -> &str {
        "MathArithmeticOverflow"
    }

    fn message(&self) -> String {
        format!(
            "{} overflows operand type {}",
            self.operation, self.operand_type,
        )
    }
}

/// Arithmetic used by retained native functions must have identical debug and
/// release behavior. Integers therefore use checked operations while the
/// unbounded/IEEE numeric families preserve their existing semantics.
pub trait RuntimeCheckedArithmetic: Copy {
    fn runtime_checked_add(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_sub(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_mul(self, rhs: Self) -> Option<Self>;
}

pub trait RuntimeCheckedPow: Copy {
    fn runtime_checked_pow(self, rhs: Self) -> Option<Self>;
}

pub trait RuntimeCheckedNeg: Sized {
    fn runtime_checked_neg(&self) -> Option<Self>;
}

macro_rules! impl_checked_integer_arithmetic {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedArithmetic for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> {
                    self.checked_add(rhs)
                }

                fn runtime_checked_sub(self, rhs: Self) -> Option<Self> {
                    self.checked_sub(rhs)
                }

                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> {
                    self.checked_mul(rhs)
                }

            }

            impl RuntimeCheckedNeg for $type {
                fn runtime_checked_neg(&self) -> Option<Self> {
                    self.checked_neg()
                }
            }
        )+
    };
}

impl_checked_integer_arithmetic!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! impl_unchecked_arithmetic {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedArithmetic for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> {
                    Some(self + rhs)
                }

                fn runtime_checked_sub(self, rhs: Self) -> Option<Self> {
                    Some(self - rhs)
                }

                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> {
                    Some(self * rhs)
                }

            }

            impl RuntimeCheckedNeg for $type {
                fn runtime_checked_neg(&self) -> Option<Self> {
                    Some(-*self)
                }
            }
        )+
    };
}

impl_unchecked_arithmetic!(f32, f64);
#[cfg(feature = "rational")]
impl_unchecked_arithmetic!(crate::R64);
#[cfg(feature = "complex")]
impl_unchecked_arithmetic!(crate::C64);

impl RuntimeCheckedPow for u8 {
    fn runtime_checked_pow(self, rhs: Self) -> Option<Self> {
        self.checked_pow(u32::from(rhs))
    }
}

impl RuntimeCheckedPow for u16 {
    fn runtime_checked_pow(self, rhs: Self) -> Option<Self> {
        self.checked_pow(u32::from(rhs))
    }
}

impl RuntimeCheckedPow for u32 {
    fn runtime_checked_pow(self, rhs: Self) -> Option<Self> {
        self.checked_pow(rhs)
    }
}

impl RuntimeCheckedPow for f32 {
    fn runtime_checked_pow(self, rhs: Self) -> Option<Self> {
        Some(self.powf(rhs))
    }
}

impl RuntimeCheckedPow for f64 {
    fn runtime_checked_pow(self, rhs: Self) -> Option<Self> {
        Some(self.powf(rhs))
    }
}

macro_rules! impl_checked_matrix_neg {
    ($cfg:meta, $matrix:ident) => {
        #[cfg($cfg)]
        impl<T> RuntimeCheckedNeg for $matrix<T>
        where
            T: RuntimeCheckedNeg + nalgebra::Scalar,
        {
            fn runtime_checked_neg(&self) -> Option<Self> {
                let mut next = self.clone();
                for value in next.iter_mut() {
                    *value = value.runtime_checked_neg()?;
                }
                Some(next)
            }
        }
    };
}

impl_checked_matrix_neg!(feature = "matrix1", Matrix1);
impl_checked_matrix_neg!(feature = "matrix2", Matrix2);
impl_checked_matrix_neg!(feature = "matrix3", Matrix3);
impl_checked_matrix_neg!(feature = "matrix4", Matrix4);
impl_checked_matrix_neg!(feature = "matrix2x3", Matrix2x3);
impl_checked_matrix_neg!(feature = "matrix3x2", Matrix3x2);
impl_checked_matrix_neg!(feature = "row_vector2", RowVector2);
impl_checked_matrix_neg!(feature = "row_vector3", RowVector3);
impl_checked_matrix_neg!(feature = "row_vector4", RowVector4);
impl_checked_matrix_neg!(feature = "row_vectord", RowDVector);
impl_checked_matrix_neg!(feature = "vector2", Vector2);
impl_checked_matrix_neg!(feature = "vector3", Vector3);
impl_checked_matrix_neg!(feature = "vector4", Vector4);
impl_checked_matrix_neg!(feature = "vectord", DVector);
impl_checked_matrix_neg!(feature = "matrixd", DMatrix);

pub(crate) fn arithmetic_overflow<T>(operation: &'static str) -> MechError {
    MechError::new(
        MathArithmeticOverflow {
            operation,
            operand_type: std::any::type_name::<T>(),
        },
        None,
    )
    .with_compiler_loc()
}

/// Fallible counterpart to the legacy generic binop factory. The operation
/// macro computes into staged storage and may use `?`; output replacement only
/// occurs after every element succeeds.
macro_rules! impl_checked_arithmetic_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            pub lhs: Ref<$arg1_type>,
            pub rhs: Ref<$arg2_type>,
            pub out: Ref<$out_type>,
        }

        impl<T> MechFunctionFactory for $struct_name<T>
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
                + RuntimeCheckedArithmetic,
            #[cfg(feature = "compiler")]
            T: ConstElem + CompileConst,
            Ref<$out_type>: ToValue,
            $arg1_type: FunctionRuntimeType,
            $arg2_type: FunctionRuntimeType,
            $out_type: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let lhs: Ref<$arg1_type> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let rhs: Ref<$arg2_type> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let out: Ref<$out_type> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self { lhs, rhs, out }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
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
                + RuntimeCheckedArithmetic,
            Ref<$out_type>: ToValue,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }

            fn out(&self) -> Value {
                self.out.to_value()
            }

            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<Value>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind + RuntimeCheckedArithmetic,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[cfg(feature = "add")]
pub mod add;
#[cfg(feature = "div")]
pub mod div;
#[cfg(feature = "mod")]
pub mod modulus;
#[cfg(feature = "mul")]
pub mod mul;
#[cfg(feature = "neg")]
pub mod negate;
#[cfg(feature = "pow")]
pub mod pow;
#[cfg(feature = "sub")]
pub mod sub;

#[cfg(feature = "add")]
pub use self::add::*;
#[cfg(feature = "div")]
pub use self::div::*;
#[cfg(feature = "mod")]
pub use self::modulus::*;
#[cfg(feature = "mul")]
pub use self::mul::*;
#[cfg(feature = "neg")]
pub use self::negate::*;
#[cfg(feature = "pow")]
pub use self::pow::*;
#[cfg(feature = "sub")]
pub use self::sub::*;
