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

pub(crate) fn managed_broadcast_element<T: ManagedElement>(
    input: &ManagedValueView<'_, T>,
    row: usize,
    column: usize,
    output_rows: usize,
    output_columns: usize,
) -> MResult<T> {
    let coordinate = match (input.rows(), input.columns()) {
        (1, 1) => Some((0, 0)),
        (rows, columns) if rows == output_rows && columns == output_columns => Some((row, column)),
        (1, columns) if columns == output_columns => Some((0, column)),
        (rows, 1) if rows == output_rows => Some((row, 0)),
        _ => None,
    };
    coordinate
        .and_then(|(row, column)| input.get(row, column))
        .ok_or_else(|| {
            MechError::from(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: input.len() as u64,
                alignment: core::mem::align_of::<T>() as u32,
                reason: "managed arithmetic broadcast geometry is incompatible",
            })
        })
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
        impl_checked_arithmetic_binop!(@bound RuntimeCheckedArithmetic;
            $struct_name, $arg1_type, $arg2_type, $out_type, $op $(, $semantic_contract)?);
    };
    (@bound $checked:path; $struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            pub lhs: ManagedPort<T>,
            pub rhs: ManagedPort<T>,
            pub out: ManagedPort<T>,
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
                + FunctionPortBacking
                + ManagedElement
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
                + $checked,
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

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                arithmetic_semantic_contract!($out_type $(, $semantic_contract)?)
            }

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs = lhs.try_managed_element::<T>()?;
                let rhs = rhs.try_managed_element::<T>()?;
                let out = out.try_managed_element::<T>()?;
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
                + FunctionPortBacking
                + ManagedElement
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
                + $checked,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_binary_port_views(&self.lhs, &self.rhs, &self.out, |lhs, rhs, out| {
                    let rows = out.rows();
                    let columns = out.columns();
                    out.try_fill_column_major(|index| {
                        let row = index % rows;
                        let column = index / rows;
                        let lhs = crate::ops::managed_broadcast_element(&lhs, row, column, rows, columns)?;
                        let rhs = crate::ops::managed_broadcast_element(&rhs, row, column, rows, columns)?;
                        $op!(@managed lhs, rhs)
                    })
                })?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }

            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                arithmetic_semantic_contract!($out_type $(, $semantic_contract)?)
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
            T: CanonicalMatrixElementBacking
                + ConstElem
                + CompileConst
                + FunctionRuntimeType
                + FunctionPortBacking
                + ManagedElement
                + $checked,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), <T as FunctionRuntimeType>::REPRESENTATION);
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let lhs = compile_value_cell_register(self.lhs.cell(), ctx)?;
                let rhs = compile_value_cell_register(self.rhs.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, out, lhs, rhs);
                Ok(out)
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
