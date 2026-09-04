use crate::*;
use std::sync::LazyLock;

static PURE_BINARY_FULL_WRITE_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_binary_full_write(ChangeDetectionPolicy::ExactScalar));
static PURE_BINARY_FULL_WRITE_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_binary_full_write(ChangeDetectionPolicy::KernelReported));
#[cfg(any(
    feature = "abs",
    feature = "neg",
    feature = "j0",
    feature = "j1",
    feature = "y0",
    feature = "y1",
    feature = "lgamma",
    feature = "tgamma",
    feature = "log",
    feature = "log10",
    feature = "log1p",
    feature = "log2",
    feature = "cbrt",
    feature = "sqrt",
    feature = "ceil",
    feature = "floor",
    feature = "rint",
    feature = "round",
    feature = "roundeven",
    feature = "trunc",
    feature = "erf",
    feature = "erfc",
    feature = "acos",
    feature = "acosh",
    feature = "acot",
    feature = "acsc",
    feature = "asec",
    feature = "asin",
    feature = "asinh",
    feature = "atan",
    feature = "atanh",
    feature = "cos",
    feature = "cosh",
    feature = "cot",
    feature = "csc",
    feature = "sec",
    feature = "sin",
    feature = "sinh",
    feature = "tan",
    feature = "tanh"
))]
static PURE_UNARY_FULL_WRITE_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_unary_full_write(ChangeDetectionPolicy::ExactScalar));
#[cfg(any(
    feature = "abs",
    feature = "neg",
    feature = "j0",
    feature = "j1",
    feature = "y0",
    feature = "y1",
    feature = "lgamma",
    feature = "tgamma",
    feature = "log",
    feature = "log10",
    feature = "log1p",
    feature = "log2",
    feature = "cbrt",
    feature = "sqrt",
    feature = "ceil",
    feature = "floor",
    feature = "rint",
    feature = "round",
    feature = "roundeven",
    feature = "trunc",
    feature = "erf",
    feature = "erfc",
    feature = "acos",
    feature = "acosh",
    feature = "acot",
    feature = "acsc",
    feature = "asec",
    feature = "asin",
    feature = "asinh",
    feature = "atan",
    feature = "atanh",
    feature = "cos",
    feature = "cosh",
    feature = "cot",
    feature = "csc",
    feature = "sec",
    feature = "sin",
    feature = "sinh",
    feature = "tan",
    feature = "tanh"
))]
static PURE_UNARY_FULL_WRITE_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_unary_full_write(ChangeDetectionPolicy::KernelReported));

#[cfg(any(
    feature = "abs",
    feature = "neg",
    feature = "j0",
    feature = "j1",
    feature = "y0",
    feature = "y1",
    feature = "lgamma",
    feature = "tgamma",
    feature = "log",
    feature = "log10",
    feature = "log1p",
    feature = "log2",
    feature = "cbrt",
    feature = "sqrt",
    feature = "ceil",
    feature = "floor",
    feature = "rint",
    feature = "round",
    feature = "roundeven",
    feature = "trunc",
    feature = "erf",
    feature = "erfc",
    feature = "acos",
    feature = "acosh",
    feature = "acot",
    feature = "acsc",
    feature = "asec",
    feature = "asin",
    feature = "asinh",
    feature = "atan",
    feature = "atanh",
    feature = "cos",
    feature = "cosh",
    feature = "cot",
    feature = "csc",
    feature = "sec",
    feature = "sin",
    feature = "sinh",
    feature = "tan",
    feature = "tanh"
))]
fn pure_unary_full_write(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(any(
    feature = "abs",
    feature = "neg",
    feature = "j0",
    feature = "j1",
    feature = "y0",
    feature = "y1",
    feature = "lgamma",
    feature = "tgamma",
    feature = "log",
    feature = "log10",
    feature = "log1p",
    feature = "log2",
    feature = "cbrt",
    feature = "sqrt",
    feature = "ceil",
    feature = "floor",
    feature = "rint",
    feature = "round",
    feature = "roundeven",
    feature = "trunc",
    feature = "erf",
    feature = "erfc",
    feature = "acos",
    feature = "acosh",
    feature = "acot",
    feature = "acsc",
    feature = "asec",
    feature = "asin",
    feature = "asinh",
    feature = "atan",
    feature = "atanh",
    feature = "cos",
    feature = "cosh",
    feature = "cot",
    feature = "csc",
    feature = "sec",
    feature = "sin",
    feature = "sinh",
    feature = "tan",
    feature = "tanh"
))]
pub(crate) fn unary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_UNARY_FULL_WRITE_KERNEL_REPORTED,
        _ => &PURE_UNARY_FULL_WRITE_EXACT_SCALAR,
    }
}

fn pure_binary_full_write(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

pub(crate) fn arithmetic_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_BINARY_FULL_WRITE_KERNEL_REPORTED,
        _ => &PURE_BINARY_FULL_WRITE_EXACT_SCALAR,
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
impl RuntimeCheckedArithmetic for crate::R64 {
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

#[cfg(feature = "rational")]
impl RuntimeCheckedNeg for crate::R64 {
    fn runtime_checked_neg(&self) -> Option<Self> {
        (*self).checked_neg()
    }
}
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

/// Fallible counterpart to the legacy generic binop factory. The operation
/// macro computes into staged storage and may use `?`; output replacement only
/// occurs after every element succeeds.
macro_rules! arithmetic_semantic_contract {
    ($output:ty) => {
        None
    };
    ($output:ty, $semantic_contract:path) => {
        Some($semantic_contract(
            <$output as FunctionRuntimeType>::REPRESENTATION,
        ))
    };
}

macro_rules! impl_checked_arithmetic_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident $(, $semantic_contract:path)?) => {
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
                + FunctionRuntimeType
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
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                arithmetic_semantic_contract!($out_type $(, $semantic_contract)?)
            }

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
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
                arithmetic_semantic_contract!($out_type $(, $semantic_contract)?)
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
                + ConstElem
                + CompileConst
                + FunctionRuntimeType
                + RuntimeCheckedArithmetic,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), <T as FunctionRuntimeType>::REPRESENTATION);
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

#[cfg(all(feature = "add", feature = "source"))]
pub use self::add::*;
#[cfg(all(feature = "div", feature = "source"))]
pub use self::div::*;
#[cfg(all(feature = "mod", feature = "source"))]
pub use self::modulus::*;
#[cfg(all(feature = "mul", feature = "source"))]
pub use self::mul::*;
#[cfg(all(feature = "neg", feature = "source"))]
pub use self::negate::*;
#[cfg(all(feature = "pow", feature = "source"))]
pub use self::pow::*;
#[cfg(all(feature = "sub", feature = "source"))]
pub use self::sub::*;

#[cfg(all(test, feature = "rational"))]
mod checked_rational_tests {
    use super::*;

    #[test]
    fn bounded_rationals_reject_every_overflowing_runtime_operation() {
        let max = R64::new(i64::MAX, 1);
        let min = R64::new(i64::MIN, 1);
        let one = R64::new(1, 1);
        let two = R64::new(2, 1);
        let negative_one = R64::new(-1, 1);

        assert!(RuntimeCheckedArithmetic::runtime_checked_add(max, one).is_none());
        assert!(RuntimeCheckedArithmetic::runtime_checked_sub(min, one).is_none());
        assert!(RuntimeCheckedArithmetic::runtime_checked_mul(max, two).is_none());
        assert!(min.runtime_checked_neg().is_none());
        assert!(min.checked_div(negative_one).is_none());
    }
}
