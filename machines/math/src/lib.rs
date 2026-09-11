#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "matrix")]
extern crate nalgebra as na;

#[cfg(all(not(feature = "dynamic-module"), feature = "runtime"))]
use mech_core::*;

#[cfg(all(not(feature = "dynamic-module"), feature = "math"))]
#[allow(
    unused_imports,
    reason = "minimal math feature profiles can compile without generated paste factories"
)]
use paste::paste;

#[cfg(feature = "matrixd")]
use na::DMatrix;
#[cfg(feature = "vectord")]
use na::DVector;
#[cfg(any(feature = "matrix1", feature = "matrix1_interop"))]
use na::Matrix1;
#[cfg(feature = "matrix2")]
use na::Matrix2;
#[cfg(feature = "matrix2x3")]
use na::Matrix2x3;
#[cfg(feature = "matrix3")]
use na::Matrix3;
#[cfg(feature = "matrix3x2")]
use na::Matrix3x2;
#[cfg(feature = "matrix4")]
use na::Matrix4;
#[cfg(feature = "row_vectord")]
use na::RowDVector;
#[cfg(feature = "row_vector2")]
use na::RowVector2;
#[cfg(feature = "row_vector3")]
use na::RowVector3;
#[cfg(feature = "row_vector4")]
use na::RowVector4;
#[cfg(feature = "vector2")]
use na::Vector2;
#[cfg(feature = "vector3")]
use na::Vector3;
#[cfg(feature = "vector4")]
use na::Vector4;

#[cfg(any(feature = "ops", feature = "op_assign"))]
use std::fmt::{Debug, Display};
#[cfg(any(
    all(feature = "runtime", not(feature = "dynamic-module")),
    feature = "neg",
    feature = "op_assign"
))]
#[allow(
    unused_imports,
    reason = "minimal operation profiles can omit the generic marker and operator traits"
)]
use std::marker::PhantomData;
#[cfg(any(feature = "ops", feature = "op_assign"))]
use std::ops::*;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathArithmeticOverflow {
    pub operation: &'static str,
    pub operand_type: &'static str,
}

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
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

#[cfg(all(
    feature = "runtime",
    not(feature = "dynamic-module"),
    any(
        feature = "add",
        feature = "sub",
        feature = "mul",
        feature = "pow",
        feature = "neg",
        feature = "add_assign",
        feature = "sub_assign",
        feature = "mul_assign",
        feature = "div_assign",
        all(
            feature = "abs",
            any(
                feature = "i8",
                feature = "i16",
                feature = "i32",
                feature = "i64",
                feature = "i128"
            )
        )
    )
))]
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

// The generic implementation type does not rename an existing runtime ABI.
// This table controls both the established same-shape runtime name and its
// native installer. All other suffixes describe newly added broadcasts.
#[cfg(all(
    feature = "runtime",
    not(feature = "dynamic-module"),
    any(feature = "f32", feature = "f64", feature = "semantic-compiler"),
    any(
        feature = "atan2",
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "jn",
        feature = "yn"
    )
))]
macro_rules! define_float_binary_identity {
    ($d:tt; $(($suffix:ident, $shape_name:literal, $installer_suffix:ident, $atan2_installer_suffix:ident)),+ $(,)?) => {
        pub(crate) fn float_binary_runtime_name(
            operation: &str,
            factory: &str,
            scalar: FunctionValueRepresentation,
        ) -> String {
            let suffix = factory
                .strip_prefix(operation)
                .expect("floating binary factory belongs to its declared operation");
            match suffix {
                $(stringify!($suffix) => {
                    format!("{operation}{}{}", $shape_name, scalar.to_string().to_uppercase())
                },)+
                _ => format!("{factory}<{scalar}>"),
            }
        }

        #[cfg(any(feature = "f32", feature = "f64"))]
        macro_rules! with_float_binary_installer {
            // Atan2's established scalar installer omits the shape suffix.
            (Atan2, SS, $d scalar:ident, $d callback:ident, $d context:tt) => {
                mech_core::paste::paste! {
                    $d callback!($d context, [<install_atan2_ $d scalar>]);
                }
            };
            $((Atan2, $suffix, $d scalar:ident, $d callback:ident, $d context:tt) => {
                mech_core::paste::paste! {
                    $d callback!($d context, [<install_atan2_ $atan2_installer_suffix:lower _ $d scalar>]);
                }
            };)+
            $(($d operation:ident, $suffix, $d scalar:ident, $d callback:ident, $d context:tt) => {
                mech_core::paste::paste! {
                    $d callback!($d context, [<install_ $d operation:snake _ $installer_suffix:lower _ $d scalar>]);
                }
            };)+
            ($d operation:ident, $d suffix:ident, $d scalar:ident, $d callback:ident, $d context:tt) => {
                mech_core::paste::paste! {
                    $d callback!($d context, [<install_ $d operation:snake _ $d suffix:lower _ $d scalar>]);
                }
            };
        }
    };
}

#[cfg(all(
    feature = "runtime",
    not(feature = "dynamic-module"),
    any(feature = "f32", feature = "f64", feature = "semantic-compiler"),
    any(
        feature = "atan2",
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "jn",
        feature = "yn"
    )
))]
define_float_binary_identity! {
    $;
    (SS, "", S, S),
    (M1M1, "M1", M1, M1),
    (M2M2, "M2", M2, M2),
    (M3M3, "M3", M3, M3),
    (M4M4, "M4", M4, M4),
    (M2x3M2x3, "M2x3", M2x3, M2x3),
    (M3x2M3x2, "M3x2", M3x2, M3x2),
    (MDMD, "MD", MD, M_D),
    (R2R2, "R2", R2, R2),
    (R3R3, "R3", R3, R3),
    (R4R4, "R4", R4, R4),
    (RDRD, "RD", RD, R_D),
    (V2V2, "V2", V2, V2),
    (V3V3, "V3", V3, V3),
    (V4V4, "V4", V4, V4),
    (VDVD, "VD", VD, V_D),
}

#[cfg(all(
    test,
    feature = "native-plan",
    any(feature = "f32", feature = "f64"),
    any(
        feature = "atan2",
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "jn",
        feature = "yn"
    )
))]
mod float_binary_identity_tests;

/// Defines one binary floating-point factory from the shared physical-shape
/// enumeration. The operation module supplies only the scalar kernel trait;
/// broadcasting, ports, compilation, and memory behavior stay uniform.
#[cfg(all(
    feature = "runtime",
    not(feature = "dynamic-module"),
    any(
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "jn",
        feature = "yn"
    )
))]
macro_rules! impl_managed_math_broadcast_binary_full_write {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $element_bound:path, $op:ident, $semantic:literal, $identity:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            arg1: ManagedPort<T>,
            arg2: ManagedPort<T>,
            out: ManagedPort<T>,
            marker: PhantomData<($arg1_type, $arg2_type, $out_type)>,
        }

        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: $element_bound
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
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> ImplementationMemoryClass {
                ImplementationMemoryClass::NoAdditionalScratch
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                let _ = arg1.try_managed::<$arg1_type>()?;
                let _ = arg2.try_managed::<$arg2_type>()?;
                let _ = out.try_managed::<$out_type>()?;
                Ok(Box::new(Self {
                    arg1: arg1.try_managed_element::<T>()?,
                    arg2: arg2.try_managed_element::<T>()?,
                    out: out.try_managed_element::<T>()?,
                    marker: PhantomData,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(crate::managed_binary::arithmetic_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
        }

        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: $element_bound + ManagedElement + FunctionPortBacking + std::fmt::Debug,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_managed(
                &self,
                frame: &mut KernelMemoryFrame<'_>,
                _services: &mut dyn MechExecutionServices,
            ) -> MResult<ReactiveSolveStatus> {
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
                            let arg1 = crate::managed_binary::managed_broadcast_element(
                                &arg1, row, column, rows, columns,
                            )?;
                            let arg2 = crate::managed_binary::managed_broadcast_element(
                                &arg2, row, column, rows, columns,
                            )?;
                            Ok($op!(arg1, arg2))
                        })
                    },
                )?;
                Ok(ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }

            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(crate::managed_binary::arithmetic_full_write_contract(
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
            T: $element_bound
                + ManagedElement
                + FunctionPortBacking
                + FunctionRuntimeType
                + CanonicalMatrixElementBacking
                + ConstElem
                + CompileConst
                + std::fmt::Debug,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = crate::float_binary_runtime_name(
                    stringify!($identity),
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                );
                let output = compile_value_cell_register(self.out.cell(), ctx)?;
                let first = compile_value_cell_register(self.arg1.cell(), ctx)?;
                let second = compile_value_cell_register(self.arg2.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, output, first, second);
                Ok(output)
            }
        }
    };
}

#[cfg(all(
    feature = "runtime",
    not(feature = "dynamic-module"),
    any(
        feature = "add",
        feature = "sub",
        feature = "mul",
        feature = "div",
        feature = "mod",
        feature = "pow",
        feature = "atan2",
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "jn",
        feature = "yn"
    )
))]
mod managed_binary;

#[cfg(all(
    feature = "runtime",
    not(feature = "dynamic-module"),
    any(
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
    )
))]
mod managed_unary;

#[cfg(any(feature = "round", feature = "dynamic-module"))]
pub mod kernels;

#[cfg(feature = "dynamic-module")]
mod dynamic_module;

#[cfg(feature = "arithmetic")]
pub mod arithmetic;
#[cfg(feature = "bessel")]
pub mod bessel;
#[cfg(feature = "gamma")]
pub mod gamma;
#[cfg(feature = "logarithm")]
pub mod logarithm;
#[cfg(feature = "op_assign")]
pub mod op_assign;
#[cfg(feature = "ops")]
pub mod ops;
#[cfg(feature = "root")]
pub mod root;
#[cfg(feature = "rounding")]
pub mod rounding;
#[cfg(feature = "stat_error")]
pub mod stat_error;
#[cfg(feature = "trig")]
pub mod trig;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub mod catalog;

#[cfg(all(feature = "arithmetic", feature = "source"))]
pub use self::arithmetic::*;
#[cfg(all(feature = "bessel", feature = "source"))]
pub use self::bessel::*;
#[cfg(all(feature = "gamma", feature = "source"))]
pub use self::gamma::*;
#[cfg(all(feature = "logarithm", feature = "source"))]
pub use self::logarithm::*;
#[cfg(all(feature = "op_assign", feature = "source"))]
pub use self::op_assign::*;
#[cfg(all(feature = "ops", feature = "source"))]
pub use self::ops::*;
#[cfg(all(feature = "ops", feature = "runtime", not(feature = "source")))]
pub(crate) use self::ops::*;
#[cfg(all(feature = "root", feature = "source"))]
pub use self::root::*;
#[cfg(all(feature = "rounding", feature = "source"))]
pub use self::rounding::*;
#[cfg(all(feature = "stat_error", feature = "source"))]
pub use self::stat_error::*;
#[cfg(all(feature = "trig", feature = "source"))]
pub use self::trig::*;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub use self::catalog::*;

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(any(
        feature = "add_assign",
        feature = "div_assign",
        feature = "mul_assign",
        feature = "sub_assign",
        feature = "abs",
        feature = "neg",
        feature = "atan2",
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "jn",
        feature = "yn",
        feature = "div",
        feature = "mod",
        feature = "mul",
        feature = "pow",
        feature = "sub",
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
    pub use crate::catalog::__mech_native::*;
    #[cfg(feature = "add")]
    pub use crate::ops::add::__mech_native::*;
}

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_math_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop);
    };
}

#[macro_export]
macro_rules! impl_canonical_registered_math_unop_specializer {
    ($specializer:ident, $factory_prefix:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer;

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 1 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 1,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let input = invocation.input(0).expect("validated unary math input");
                let output_extents = input
                    .cell()?
                    .resolved_descriptor()?
                    .current_extents()
                    .map_err(MechError::from)?;
                context.bind_resolved_runtime(
                    mech_core::RuntimeBindingSelector::Operation(
                        context.resolved_call()?.operation.id,
                    ),
                    mech_core::ExecutionTarget::DirectRuntime,
                    vec![output_extents].into_boxed_slice(),
                    &[input],
                )
            }
        }
    };
}

#[macro_export]
macro_rules! impl_canonical_registered_math_binop_specializer {
    ($specializer:ident, $factory_prefix:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer;

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let first = invocation.input(0).expect("validated binary math lhs");
                let second = invocation.input(1).expect("validated binary math rhs");
                let output_extents = $crate::semantic_broadcast_extents(&[first, second])?;
                context.bind_resolved_runtime(
                    mech_core::RuntimeBindingSelector::Operation(
                        context.resolved_call()?.operation.id,
                    ),
                    mech_core::ExecutionTarget::DirectRuntime,
                    vec![output_extents].into_boxed_slice(),
                    &[first, second],
                )
            }
        }
    };
}

#[cfg(feature = "source")]
pub fn semantic_broadcast_extents(inputs: &[&SpecializationInput]) -> MResult<Box<[u64]>> {
    let mut result: Option<[u64; 2]> = None;
    for input in inputs {
        let extents = input
            .cell()?
            .resolved_descriptor()?
            .current_extents()
            .map_err(MechError::from)?;
        if !extents.is_empty() {
            let [rows, columns] = extents.as_ref() else {
                return Err(MechError::new(
                    GenericError {
                        msg: "numeric broadcasting requires scalar or rank-two inputs".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            result = Some(match result {
                None => [*rows, *columns],
                Some([left_rows, left_columns]) => {
                    let axis = |left: u64, right: u64| {
                        if left == right {
                            Some(left)
                        } else if left == 1 {
                            Some(right)
                        } else if right == 1 {
                            Some(left)
                        } else {
                            None
                        }
                    };
                    [
                        axis(left_rows, *rows).ok_or_else(|| {
                            MechError::new(
                                DimensionMismatch {
                                    dims: vec![
                                        left_rows as usize,
                                        left_columns as usize,
                                        *rows as usize,
                                        *columns as usize,
                                    ],
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?,
                        axis(left_columns, *columns).ok_or_else(|| {
                            MechError::new(
                                DimensionMismatch {
                                    dims: vec![
                                        left_rows as usize,
                                        left_columns as usize,
                                        *rows as usize,
                                        *columns as usize,
                                    ],
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?,
                    ]
                }
            });
        }
    }
    Ok(result.map_or_else(
        || Vec::<u64>::new().into_boxed_slice(),
        |shape| shape.into_iter().collect::<Vec<_>>().into_boxed_slice(),
    ))
}

#[macro_export]
macro_rules! impl_math_unop {
  ($fxn_name:ident, $type:ident, $op_fxn:ident) => {
    paste!{
      impl_unop!([<$fxn_name $type:camel S>], $type, $type, $type, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrix1")]
      impl_unop!([<$fxn_name $type:camel M1>], $type, Matrix1<$type>, Matrix1<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrix2")]
      impl_unop!([<$fxn_name $type:camel M2>], $type, Matrix2<$type>, Matrix2<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrix3")]
      impl_unop!([<$fxn_name $type:camel M3>], $type, Matrix3<$type>, Matrix3<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrix4")]
      impl_unop!([<$fxn_name $type:camel M4>], $type, Matrix4<$type>, Matrix4<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrix2x3")]
      impl_unop!([<$fxn_name $type:camel M2x3>], $type, Matrix2x3<$type>, Matrix2x3<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrix3x2")]
      impl_unop!([<$fxn_name $type:camel M3x2>], $type, Matrix3x2<$type>, Matrix3x2<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "matrixd")]
      impl_unop!([<$fxn_name $type:camel MD>], $type, DMatrix<$type>, DMatrix<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "row_vector2")]
      impl_unop!([<$fxn_name $type:camel R2>], $type, RowVector2<$type>, RowVector2<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "row_vector3")]
      impl_unop!([<$fxn_name $type:camel R3>], $type, RowVector3<$type>, RowVector3<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "row_vector4")]
      impl_unop!([<$fxn_name $type:camel R4>], $type, RowVector4<$type>, RowVector4<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "row_vectord")]
      impl_unop!([<$fxn_name $type:camel RD>], $type, RowDVector<$type>, RowDVector<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "vector2")]
      impl_unop!([<$fxn_name $type:camel V2>], $type, Vector2<$type>, Vector2<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "vector3")]
      impl_unop!([<$fxn_name $type:camel V3>], $type, Vector3<$type>, Vector3<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "vector4")]
      impl_unop!([<$fxn_name $type:camel V4>], $type, Vector4<$type>, Vector4<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
      #[cfg(feature = "vectord")]
      impl_unop!([<$fxn_name $type:camel VD>], $type, DVector<$type>, DVector<$type>, [<$op_fxn _op>], crate::managed_unary::unary_full_write_contract);
    }}}

#[macro_export]
macro_rules! impl_canonical_math_float_unop_specializer {
    ($specializer:ident, $lib:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer {}

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                specialization: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if specialization.len() != 1 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 1,
                            found: specialization.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let input = specialization.input(0).expect("validated unary input");
                let extents = $crate::semantic_broadcast_extents(&[input])?;
                context.bind_resolved_runtime(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    vec![extents].into_boxed_slice(),
                    &[input],
                )
            }
        }
    };
}

#[macro_export]
macro_rules! impl_canonical_math_same_type_binop_specializer {
    ($specializer:ident, $prefix:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer {}

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                specialization: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if specialization.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: specialization.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let first = specialization.input(0).expect("validated first input");
                let second = specialization.input(1).expect("validated second input");
                let extents = $crate::semantic_broadcast_extents(&[first, second])?;
                context.bind_resolved_runtime(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    vec![extents].into_boxed_slice(),
                    &[first, second],
                )
            }
        }
    };
}
