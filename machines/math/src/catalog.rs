use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult, MechFunctionFactory,
    NativeFunctionCompiler, legacy_source_specializer,
};
#[cfg(feature = "matrix")]
use nalgebra::{
    DMatrix, DVector, Matrix1, Matrix2, Matrix2x3, Matrix3, Matrix3x2, Matrix4, RowDVector,
    RowVector2, RowVector3, RowVector4, Vector2, Vector3, Vector4,
};
#[cfg(feature = "functions")]
use paste::paste;

#[cfg(feature = "abs")]
use crate::arithmetic::abs::*;
#[cfg(feature = "j0")]
use crate::bessel::j0::*;
#[cfg(feature = "j1")]
use crate::bessel::j1::*;
#[cfg(feature = "y0")]
use crate::bessel::y0::*;
#[cfg(feature = "y1")]
use crate::bessel::y1::*;
#[cfg(feature = "lgamma")]
use crate::gamma::lgamma::*;
#[cfg(feature = "tgamma")]
use crate::gamma::tgamma::*;
#[cfg(feature = "log")]
use crate::logarithm::log::*;
#[cfg(feature = "log1p")]
use crate::logarithm::log1p::*;
#[cfg(feature = "log2")]
use crate::logarithm::log2::*;
#[cfg(feature = "log10")]
use crate::logarithm::log10::*;
#[cfg(feature = "op_assign")]
use crate::op_assign::*;
#[cfg(feature = "div")]
use crate::ops::div::*;
#[cfg(feature = "mod")]
use crate::ops::modulus::*;
#[cfg(feature = "mul")]
use crate::ops::mul::*;
#[cfg(feature = "pow")]
use crate::ops::pow::*;
#[cfg(feature = "sub")]
use crate::ops::sub::*;
#[cfg(feature = "cbrt")]
use crate::root::cbrt::*;
#[cfg(feature = "sqrt")]
use crate::root::sqrt::*;
#[cfg(feature = "ceil")]
use crate::rounding::ceil::*;
#[cfg(feature = "floor")]
use crate::rounding::floor::*;
#[cfg(feature = "rint")]
use crate::rounding::rint::*;
#[cfg(feature = "round")]
use crate::rounding::round::*;
#[cfg(feature = "roundeven")]
use crate::rounding::roundeven::*;
#[cfg(feature = "trunc")]
use crate::rounding::trunc::*;
#[cfg(feature = "erf")]
use crate::stat_error::erf::*;
#[cfg(feature = "erfc")]
use crate::stat_error::erfc::*;
#[cfg(feature = "acos")]
use crate::trig::acos::*;
#[cfg(feature = "acosh")]
use crate::trig::acosh::*;
#[cfg(feature = "acot")]
use crate::trig::acot::*;
#[cfg(feature = "acsc")]
use crate::trig::acsc::*;
#[cfg(feature = "asec")]
use crate::trig::asec::*;
#[cfg(feature = "asin")]
use crate::trig::asin::*;
#[cfg(feature = "asinh")]
use crate::trig::asinh::*;
#[cfg(feature = "atan")]
use crate::trig::atan::*;
#[cfg(feature = "atanh")]
use crate::trig::atanh::*;
#[cfg(feature = "cos")]
use crate::trig::cos::*;
#[cfg(feature = "cosh")]
use crate::trig::cosh::*;
#[cfg(feature = "cot")]
use crate::trig::cot::*;
#[cfg(feature = "csc")]
use crate::trig::csc::*;
#[cfg(feature = "sec")]
use crate::trig::sec::*;
#[cfg(feature = "sin")]
use crate::trig::sin::*;
#[cfg(feature = "sinh")]
use crate::trig::sinh::*;
#[cfg(feature = "tan")]
use crate::trig::tan::*;
#[cfg(feature = "tanh")]
use crate::trig::tanh::*;

fn install_legacy_source<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &'static str,
    module: Option<&'static str>,
    item: Option<&'static str>,
    exposure: FunctionExposure,
    compiler: T,
) -> MResult<()>
where
    T: NativeFunctionCompiler + 'static,
{
    let operation =
        builder.insert_specializer(canonical_name, legacy_source_specializer(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: module.map(str::to_string),
        item: item.map(str::to_string),
        exposure,
    })
}

macro_rules! install_prelude {
    ($builder:expr, $name:literal, $compiler:expr) => {
        install_legacy_source(
            $builder,
            $name,
            None,
            None,
            FunctionExposure::Prelude,
            $compiler,
        )?;
    };
}

macro_rules! install_math_module {
    ($builder:expr, $name:literal, $item:literal, $compiler:expr) => {
        install_legacy_source(
            $builder,
            $name,
            Some("math"),
            Some($item),
            FunctionExposure::ModuleOnly,
            $compiler,
        )?;
    };
}

/// Installs the frozen source-specializer surface owned by `mech-math`.
///
/// Each entry follows the feature gate of its concrete implementation. The
/// source surface intentionally excludes legacy descriptors that were not in
/// the compatibility baseline (`exp`, `exp2`, `exp10`, `expm1`, `fdim`,
/// `hypot`, `ilogb`, and `sincos`).
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "abs")]
    install_math_module!(builder, "math/abs", "abs", crate::MathAbs {});
    #[cfg(feature = "acos")]
    install_math_module!(builder, "math/acos", "acos", crate::MathAcos {});
    #[cfg(feature = "acosh")]
    install_math_module!(builder, "math/acosh", "acosh", crate::MathAcosh {});
    #[cfg(feature = "acot")]
    install_math_module!(builder, "math/acot", "acot", crate::MathAcot {});
    #[cfg(feature = "acsc")]
    install_math_module!(builder, "math/acsc", "acsc", crate::MathAcsc {});

    #[cfg(feature = "add")]
    crate::install_math_add_source(builder)?;
    #[cfg(feature = "add_assign")]
    {
        install_prelude!(builder, "math/add-assign", crate::AddAssignMath {});
        install_prelude!(builder, "math/add-assign/range", crate::AddAssignRange {});
        install_prelude!(
            builder,
            "math/add-assign/range-all",
            crate::AddAssignRangeAll {}
        );
    }

    #[cfg(feature = "asec")]
    install_math_module!(builder, "math/asec", "asec", crate::MathAsec {});
    #[cfg(feature = "asin")]
    install_math_module!(builder, "math/asin", "asin", crate::MathAsin {});
    #[cfg(feature = "asinh")]
    install_math_module!(builder, "math/asinh", "asinh", crate::MathAsinh {});
    #[cfg(feature = "atan")]
    install_math_module!(builder, "math/atan", "atan", crate::MathAtan {});
    #[cfg(feature = "atan2")]
    install_math_module!(builder, "math/atan2", "atan2", crate::MathAtan2 {});
    #[cfg(feature = "atanh")]
    install_math_module!(builder, "math/atanh", "atanh", crate::MathAtanh {});

    #[cfg(feature = "j0")]
    install_math_module!(builder, "math/bessel/j0", "bessel/j0", crate::MathJ0 {});
    #[cfg(feature = "j1")]
    install_math_module!(builder, "math/bessel/j1", "bessel/j1", crate::MathJ1 {});
    #[cfg(feature = "jn")]
    install_math_module!(builder, "math/bessel/jn", "bessel/jn", crate::MathJn {});
    #[cfg(feature = "y0")]
    install_math_module!(builder, "math/bessel/y0", "bessel/y0", crate::MathY0 {});
    #[cfg(feature = "y1")]
    install_math_module!(builder, "math/bessel/y1", "bessel/y1", crate::MathY1 {});
    #[cfg(feature = "yn")]
    install_math_module!(builder, "math/bessel/yn", "bessel/yn", crate::MathYn {});

    #[cfg(feature = "cbrt")]
    install_math_module!(builder, "math/cbrt", "cbrt", crate::MathCbrt {});
    #[cfg(feature = "ceil")]
    install_math_module!(builder, "math/ceil", "ceil", crate::MathCeil {});
    #[cfg(feature = "copysign")]
    install_math_module!(builder, "math/copysign", "copysign", crate::MathCopysign {});
    #[cfg(feature = "cos")]
    install_math_module!(builder, "math/cos", "cos", crate::MathCos {});
    #[cfg(feature = "cosh")]
    install_math_module!(builder, "math/cosh", "cosh", crate::MathCosh {});
    #[cfg(feature = "cot")]
    install_math_module!(builder, "math/cot", "cot", crate::MathCot {});
    #[cfg(feature = "csc")]
    install_math_module!(builder, "math/csc", "csc", crate::MathCsc {});

    #[cfg(feature = "div")]
    install_prelude!(builder, "math/div", crate::MathDiv {});
    #[cfg(feature = "div_assign")]
    {
        install_prelude!(builder, "math/div-assign", crate::DivAssignValue {});
        install_prelude!(builder, "math/div-assign/range", crate::DivAssignRange {});
        install_prelude!(
            builder,
            "math/div-assign/range-all",
            crate::DivAssignRangeAll {}
        );
    }

    #[cfg(feature = "erf")]
    install_math_module!(builder, "math/erf", "erf", crate::MathErf {});
    #[cfg(feature = "erfc")]
    install_math_module!(builder, "math/erfc", "erfc", crate::MathErfc {});
    #[cfg(feature = "floor")]
    install_math_module!(builder, "math/floor", "floor", crate::MathFloor {});
    #[cfg(feature = "fmod")]
    install_math_module!(builder, "math/fmod", "fmod", crate::MathFmod {});
    #[cfg(feature = "lgamma")]
    install_math_module!(builder, "math/lgamma", "lgamma", crate::MathLgamma {});
    #[cfg(feature = "log")]
    install_math_module!(builder, "math/log", "log", crate::MathLog {});
    #[cfg(feature = "log10")]
    install_math_module!(builder, "math/log10", "log10", crate::MathLog10 {});
    #[cfg(feature = "log1p")]
    install_math_module!(builder, "math/log1p", "log1p", crate::MathLog1p {});
    #[cfg(feature = "log2")]
    install_math_module!(builder, "math/log2", "log2", crate::MathLog2 {});

    #[cfg(feature = "mod")]
    install_prelude!(builder, "math/mod", crate::MathMod {});
    #[cfg(feature = "mul")]
    install_prelude!(builder, "math/mul", crate::MathMul {});
    #[cfg(feature = "mul_assign")]
    {
        // The baseline contains these two range forms, but no named
        // `math/mul-assign` source specializer.
        install_prelude!(builder, "math/mul-assign/range", crate::MulAssignRange {});
        install_prelude!(
            builder,
            "math/mul-assign/range-all",
            crate::MulAssignRangeAll {}
        );
    }
    #[cfg(feature = "neg")]
    install_prelude!(builder, "math/neg", crate::MathNegate {});

    #[cfg(feature = "nextafter")]
    install_math_module!(
        builder,
        "math/nextafter",
        "nextafter",
        crate::MathNextafter {}
    );
    #[cfg(feature = "pow")]
    install_prelude!(builder, "math/pow", crate::MathPow {});
    #[cfg(feature = "remainder")]
    install_math_module!(
        builder,
        "math/remainder",
        "remainder",
        crate::MathRemainder {}
    );
    #[cfg(feature = "rint")]
    install_math_module!(builder, "math/rint", "rint", crate::MathRint {});
    #[cfg(feature = "round")]
    install_math_module!(builder, "math/round", "round", crate::MathRound {});
    #[cfg(feature = "roundeven")]
    install_math_module!(
        builder,
        "math/roundeven",
        "roundeven",
        crate::MathRoundeven {}
    );
    #[cfg(feature = "sec")]
    install_math_module!(builder, "math/sec", "sec", crate::MathSec {});
    #[cfg(feature = "sin")]
    install_math_module!(builder, "math/sin", "sin", crate::MathSin {});
    #[cfg(feature = "sinh")]
    install_math_module!(builder, "math/sinh", "sinh", crate::MathSinh {});
    #[cfg(feature = "sqrt")]
    install_math_module!(builder, "math/sqrt", "sqrt", crate::MathSqrt {});

    #[cfg(feature = "sub")]
    install_prelude!(builder, "math/sub", crate::MathSub {});
    #[cfg(feature = "sub_assign")]
    {
        install_prelude!(builder, "math/sub-assign", crate::SubAssignValue {});
        install_prelude!(builder, "math/sub-assign/range", crate::SubAssignRange {});
        install_prelude!(
            builder,
            "math/sub-assign/range-all",
            crate::SubAssignRangeAll {}
        );
    }

    #[cfg(feature = "tan")]
    install_math_module!(builder, "math/tan", "tan", crate::MathTan {});
    #[cfg(feature = "tanh")]
    install_math_module!(builder, "math/tanh", "tanh", crate::MathTanh {});
    #[cfg(feature = "tgamma")]
    install_math_module!(builder, "math/tgamma", "tgamma", crate::MathTgamma {});
    #[cfg(feature = "trunc")]
    install_math_module!(builder, "math/trunc", "trunc", crate::MathTrunc {});

    Ok(())
}

macro_rules! install_float_unop {
    ($builder:expr, $family:ident) => {
        mech_core::install_unop_runtime_factories!(
            $builder,
            $family;
            ("f32", f32),
            ("f64", f64),
        )?;
    };
}

macro_rules! install_exact_runtime {
    ($builder:expr, $factory:ident) => {
        $builder
            .insert_runtime_factory(stringify!($factory), <$factory as MechFunctionFactory>::new)?;
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_vv {
    ($builder:expr, $factory:ident, $scalar:ty, $scalar_name:literal, $shape:ident, $shape_name:literal) => {
        $builder.insert_runtime_factory(
            concat!(
                stringify!($factory),
                "<[",
                $scalar_name,
                "]:",
                $shape_name,
                ">"
            ),
            <$factory<$scalar, $shape<$scalar>, $shape<$scalar>> as MechFunctionFactory>::new,
        )?;
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_vv_for_type {
    ($builder:expr, $factory:ident, $scalar:ty, $scalar_name:literal) => {
        #[cfg(feature = "row_vector4")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, RowVector4, "1,4");
        #[cfg(feature = "row_vector3")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, RowVector3, "1,3");
        #[cfg(feature = "row_vector2")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, RowVector2, "1,2");
        #[cfg(feature = "vector2")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Vector2, "2,1");
        #[cfg(feature = "vector3")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Vector3, "3,1");
        #[cfg(feature = "vector4")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Vector4, "4,1");
        #[cfg(feature = "matrix1")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Matrix1, "1,1");
        #[cfg(feature = "matrix2")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Matrix2, "2,2");
        #[cfg(feature = "matrix3")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Matrix3, "3,3");
        #[cfg(feature = "matrix4")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Matrix4, "4,4");
        #[cfg(feature = "matrix2x3")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Matrix2x3, "2,3");
        #[cfg(feature = "matrix3x2")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, Matrix3x2, "3,2");
        #[cfg(feature = "vectord")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, DVector, "0,1");
        #[cfg(feature = "matrixd")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, DMatrix, "0,0");
        #[cfg(feature = "row_vectord")]
        install_op_assign_vv!($builder, $factory, $scalar, $scalar_name, RowDVector, "1,0");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_values {
    ($builder:expr, $operation:ident) => {
        paste! {
            mech_core::install_typed_runtime_factories!(
                $builder,
                [<$operation AssignSS>];
                ("u8", u8, "u8"),
                ("u16", u16, "u16"),
                ("u32", u32, "u32"),
                ("u64", u64, "u64"),
                ("u128", u128, "u128"),
                ("i8", i8, "i8"),
                ("i16", i16, "i16"),
                ("i32", i32, "i32"),
                ("i64", i64, "i64"),
                ("i128", i128, "i128"),
                ("f32", f32, "f32"),
                ("f64", f64, "f64"),
                ("r64", crate::R64, "r64"),
                ("c64", crate::C64, "c64"),
            )?;

            #[cfg(feature = "u8")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], u8, "u8");
            #[cfg(feature = "u16")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], u16, "u16");
            #[cfg(feature = "u32")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], u32, "u32");
            #[cfg(feature = "u64")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], u64, "u64");
            #[cfg(feature = "u128")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], u128, "u128");
            #[cfg(feature = "i8")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], i8, "i8");
            #[cfg(feature = "i16")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], i16, "i16");
            #[cfg(feature = "i32")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], i32, "i32");
            #[cfg(feature = "i64")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], i64, "i64");
            #[cfg(feature = "i128")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], i128, "i128");
            #[cfg(feature = "f32")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], f32, "f32");
            #[cfg(feature = "f64")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], f64, "f64");
            #[cfg(feature = "r64")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], crate::R64, "r64");
            #[cfg(feature = "c64")]
            install_op_assign_vv_for_type!($builder, [<$operation AssignVV>], crate::C64, "c64");
        }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_range_s {
    ($builder:expr, $factory:ident, $scalar:ty, $scalar_name:literal, $sink:ident, $index:ident) => {
        $builder.insert_runtime_factory(
            concat!(
                stringify!($factory),
                "<",
                $scalar_name,
                stringify!($sink),
                stringify!($index),
                ">"
            ),
            <$factory<$scalar, $sink<$scalar>, $index<usize>> as MechFunctionFactory>::new,
        )?;
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_range_v {
    ($builder:expr, $factory:ident, $scalar:ty, $scalar_name:literal, $sink:ident, $source:ident, $index:ident) => {
        $builder.insert_runtime_factory(
            concat!(
                stringify!($factory),
                "<",
                $scalar_name,
                stringify!($sink),
                stringify!($source),
                stringify!($index),
                ">"
            ),
            <$factory<$scalar, $sink<$scalar>, $source<$scalar>, $index<usize>> as MechFunctionFactory>::new,
        )?;
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_ranges_for_sink {
    ($builder:expr, $scalar_factory:ident, $vector_factory:ident, $scalar:ty, $scalar_name:literal, $sink:ident) => {
        #[cfg(feature = "matrix1")]
        install_op_assign_range_s!(
            $builder,
            $scalar_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix1
        );
        #[cfg(feature = "vector2")]
        install_op_assign_range_s!(
            $builder,
            $scalar_factory,
            $scalar,
            $scalar_name,
            $sink,
            Vector2
        );
        #[cfg(feature = "vector3")]
        install_op_assign_range_s!(
            $builder,
            $scalar_factory,
            $scalar,
            $scalar_name,
            $sink,
            Vector3
        );
        #[cfg(feature = "vector4")]
        install_op_assign_range_s!(
            $builder,
            $scalar_factory,
            $scalar,
            $scalar_name,
            $sink,
            Vector4
        );
        #[cfg(feature = "vectord")]
        install_op_assign_range_s!(
            $builder,
            $scalar_factory,
            $scalar,
            $scalar_name,
            $sink,
            DVector
        );

        #[cfg(feature = "matrix1")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix1,
            Matrix1
        );
        #[cfg(all(feature = "matrix2", feature = "vector4"))]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix2,
            Vector4
        );
        #[cfg(feature = "matrix3")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix3,
            DVector
        );
        #[cfg(feature = "matrix4")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix4,
            DVector
        );
        #[cfg(feature = "matrix2x3")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix2x3,
            DVector
        );
        #[cfg(feature = "matrix3x2")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Matrix3x2,
            DVector
        );
        #[cfg(feature = "matrixd")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            DMatrix,
            DVector
        );
        #[cfg(feature = "vectord")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            DVector,
            DVector
        );
        #[cfg(feature = "row_vectord")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            RowDVector,
            DVector
        );
        #[cfg(feature = "vector2")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Vector2,
            Vector2
        );
        #[cfg(feature = "vector3")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Vector3,
            Vector3
        );
        #[cfg(feature = "vector4")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            Vector4,
            Vector4
        );
        #[cfg(feature = "row_vector2")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            RowVector2,
            Vector2
        );
        #[cfg(feature = "row_vector3")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            RowVector3,
            Vector3
        );
        #[cfg(feature = "row_vector4")]
        install_op_assign_range_v!(
            $builder,
            $vector_factory,
            $scalar,
            $scalar_name,
            $sink,
            RowVector4,
            Vector4
        );
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_ranges_for_type {
    ($builder:expr, $scalar_factory:ident, $vector_factory:ident, $scalar:ty, $scalar_name:literal) => {
        #[cfg(feature = "row_vector2")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            RowVector2
        );
        #[cfg(feature = "row_vector3")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            RowVector3
        );
        #[cfg(feature = "row_vector4")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            RowVector4
        );
        #[cfg(feature = "vector2")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Vector2
        );
        #[cfg(feature = "vector3")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Vector3
        );
        #[cfg(feature = "vector4")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Vector4
        );
        #[cfg(feature = "matrix1")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Matrix1
        );
        #[cfg(feature = "matrix2")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Matrix2
        );
        #[cfg(feature = "matrix3")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Matrix3
        );
        #[cfg(feature = "matrix4")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Matrix4
        );
        #[cfg(feature = "matrix2x3")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Matrix2x3
        );
        #[cfg(feature = "matrix3x2")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            Matrix3x2
        );
        #[cfg(feature = "matrixd")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            DMatrix
        );
        #[cfg(feature = "row_vectord")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            RowDVector
        );
        #[cfg(feature = "vectord")]
        install_op_assign_ranges_for_sink!(
            $builder,
            $scalar_factory,
            $vector_factory,
            $scalar,
            $scalar_name,
            DVector
        );
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_ranges {
    ($builder:expr, $scalar_factory:ident, $vector_factory:ident) => {
        #[cfg(feature = "u8")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, u8, "u8");
        #[cfg(feature = "u16")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, u16, "u16");
        #[cfg(feature = "u32")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, u32, "u32");
        #[cfg(feature = "u64")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, u64, "u64");
        #[cfg(feature = "u128")]
        install_op_assign_ranges_for_type!(
            $builder,
            $scalar_factory,
            $vector_factory,
            u128,
            "u128"
        );
        #[cfg(feature = "i8")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, i8, "i8");
        #[cfg(feature = "i16")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, i16, "i16");
        #[cfg(feature = "i32")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, i32, "i32");
        #[cfg(feature = "i64")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, i64, "i64");
        #[cfg(feature = "f32")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, f32, "f32");
        #[cfg(feature = "f64")]
        install_op_assign_ranges_for_type!($builder, $scalar_factory, $vector_factory, f64, "f64");
        #[cfg(feature = "rational")]
        install_op_assign_ranges_for_type!(
            $builder,
            $scalar_factory,
            $vector_factory,
            crate::R64,
            "rational"
        );
        #[cfg(feature = "complex")]
        install_op_assign_ranges_for_type!(
            $builder,
            $scalar_factory,
            $vector_factory,
            crate::C64,
            "complex"
        );
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_op_assign_runtime {
    ($builder:expr, $operation:ident) => {
        paste! {
            install_op_assign_values!($builder, $operation);
            #[cfg(feature = "matrix")]
            {
                install_op_assign_ranges!($builder, [<$operation Assign1DRS>], [<$operation Assign1DRV>]);
                install_op_assign_ranges!($builder, [<$operation Assign2DRAS>], [<$operation Assign2DRAV>]);
            }
        }
    };
}

#[cfg(feature = "add_assign")]
fn install_add_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_op_assign_runtime!(builder, Add);
    Ok(())
}

#[cfg(feature = "div_assign")]
fn install_div_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_op_assign_runtime!(builder, Div);
    Ok(())
}

#[cfg(feature = "mul_assign")]
fn install_mul_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_op_assign_runtime!(builder, Mul);
    Ok(())
}

#[cfg(feature = "sub_assign")]
fn install_sub_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_op_assign_runtime!(builder, Sub);
    Ok(())
}

#[cfg(feature = "atan2")]
fn install_atan2_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    use crate::trig::atan2::*;

    #[cfg(feature = "f32")]
    {
        install_exact_runtime!(builder, Atan2F32);
        #[cfg(feature = "matrix1")]
        install_exact_runtime!(builder, Atan2M1F32);
        #[cfg(feature = "matrix2")]
        install_exact_runtime!(builder, Atan2M2F32);
        #[cfg(feature = "matrix3")]
        {
            install_exact_runtime!(builder, Atan2M3F32);
            // Preserve the legacy generator's matrix3 gate for Matrix3x2.
            install_exact_runtime!(builder, Atan2M3x2F32);
        }
        #[cfg(feature = "matrix2x3")]
        install_exact_runtime!(builder, Atan2M2x3F32);
        #[cfg(feature = "matrix4")]
        install_exact_runtime!(builder, Atan2M4F32);
        #[cfg(feature = "vector2")]
        install_exact_runtime!(builder, Atan2V2F32);
        #[cfg(feature = "vector3")]
        install_exact_runtime!(builder, Atan2V3F32);
        #[cfg(feature = "vector4")]
        install_exact_runtime!(builder, Atan2V4F32);
        #[cfg(feature = "row_vector2")]
        install_exact_runtime!(builder, Atan2R2F32);
        #[cfg(feature = "row_vector3")]
        install_exact_runtime!(builder, Atan2R3F32);
        #[cfg(feature = "row_vector4")]
        install_exact_runtime!(builder, Atan2R4F32);
        #[cfg(feature = "row_vectord")]
        install_exact_runtime!(builder, Atan2RDF32);
        #[cfg(feature = "vectord")]
        install_exact_runtime!(builder, Atan2VDF32);
        #[cfg(feature = "matrixd")]
        install_exact_runtime!(builder, Atan2MDF32);
    }

    #[cfg(feature = "f64")]
    {
        install_exact_runtime!(builder, Atan2F64);
        #[cfg(feature = "matrix1")]
        install_exact_runtime!(builder, Atan2M1F64);
        #[cfg(feature = "matrix2")]
        install_exact_runtime!(builder, Atan2M2F64);
        #[cfg(feature = "matrix3")]
        {
            install_exact_runtime!(builder, Atan2M3F64);
            install_exact_runtime!(builder, Atan2M3x2F64);
        }
        #[cfg(feature = "matrix2x3")]
        install_exact_runtime!(builder, Atan2M2x3F64);
        #[cfg(feature = "matrix4")]
        install_exact_runtime!(builder, Atan2M4F64);
        #[cfg(feature = "vector2")]
        install_exact_runtime!(builder, Atan2V2F64);
        #[cfg(feature = "vector3")]
        install_exact_runtime!(builder, Atan2V3F64);
        #[cfg(feature = "vector4")]
        install_exact_runtime!(builder, Atan2V4F64);
        #[cfg(feature = "row_vector2")]
        install_exact_runtime!(builder, Atan2R2F64);
        #[cfg(feature = "row_vector3")]
        install_exact_runtime!(builder, Atan2R3F64);
        #[cfg(feature = "row_vector4")]
        install_exact_runtime!(builder, Atan2R4F64);
        #[cfg(feature = "row_vectord")]
        install_exact_runtime!(builder, Atan2RDF64);
        #[cfg(feature = "vectord")]
        install_exact_runtime!(builder, Atan2VDF64);
        #[cfg(feature = "matrixd")]
        install_exact_runtime!(builder, Atan2MDF64);
    }

    Ok(())
}

/// Installs every enabled concrete runtime factory owned by `mech-math`.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "add")]
    crate::install_math_add_runtime(builder)?;
    #[cfg(feature = "add_assign")]
    install_add_assign_runtime(builder)?;
    #[cfg(feature = "div")]
    mech_core::install_binop_runtime_factories!(
        builder,
        Div;
        ("i8", i8, "i8"),
        ("i16", i16, "i16"),
        ("i32", i32, "i32"),
        ("i64", i64, "i64"),
        ("i128", i128, "i128"),
        ("u8", u8, "u8"),
        ("u16", u16, "u16"),
        ("u32", u32, "u32"),
        ("u64", u64, "u64"),
        ("u128", u128, "u128"),
        ("f32", f32, "f32"),
        ("f64", f64, "f64"),
        ("rational", crate::R64, "r64"),
        ("complex", crate::C64, "c64"),
    )?;
    #[cfg(feature = "div_assign")]
    install_div_assign_runtime(builder)?;
    #[cfg(feature = "mod")]
    mech_core::install_binop_runtime_factories!(
        builder,
        Mod;
        ("i8", i8, "i8"),
        ("i16", i16, "i16"),
        ("i32", i32, "i32"),
        ("i64", i64, "i64"),
        ("i128", i128, "i128"),
        ("u8", u8, "u8"),
        ("u16", u16, "u16"),
        ("u32", u32, "u32"),
        ("u64", u64, "u64"),
        ("u128", u128, "u128"),
        ("f32", f32, "f32"),
        ("f64", f64, "f64"),
    )?;
    #[cfg(feature = "mul_assign")]
    install_mul_assign_runtime(builder)?;
    #[cfg(feature = "mul")]
    mech_core::install_binop_runtime_factories!(
        builder,
        Mul;
        ("i8", i8, "i8"),
        ("i16", i16, "i16"),
        ("i32", i32, "i32"),
        ("i64", i64, "i64"),
        ("i128", i128, "i128"),
        ("u8", u8, "u8"),
        ("u16", u16, "u16"),
        ("u32", u32, "u32"),
        ("u64", u64, "u64"),
        ("u128", u128, "u128"),
        ("f32", f32, "f32"),
        ("f64", f64, "f64"),
        ("rational", crate::R64, "r64"),
        ("complex", crate::C64, "c64"),
    )?;
    #[cfg(feature = "sub_assign")]
    install_sub_assign_runtime(builder)?;
    #[cfg(feature = "pow")]
    mech_core::install_binop_runtime_factories!(
        builder,
        Pow;
        ("u8", u8, "u8"),
        ("u16", u16, "u16"),
        ("u32", u32, "u32"),
        ("f32", f32, "f32"),
        ("f64", f64, "f64"),
    )?;
    #[cfg(feature = "sub")]
    mech_core::install_binop_runtime_factories!(
        builder,
        Sub;
        ("i8", i8, "i8"),
        ("i16", i16, "i16"),
        ("i32", i32, "i32"),
        ("i64", i64, "i64"),
        ("i128", i128, "i128"),
        ("u8", u8, "u8"),
        ("u16", u16, "u16"),
        ("u32", u32, "u32"),
        ("u64", u64, "u64"),
        ("u128", u128, "u128"),
        ("f32", f32, "f32"),
        ("f64", f64, "f64"),
        ("rational", crate::R64, "r64"),
        ("complex", crate::C64, "c64"),
    )?;

    #[cfg(feature = "neg")]
    {
        use crate::ops::negate::{NegateS, NegateV};
        mech_core::install_typed_runtime_factories!(
            builder,
            NegateV;
            ("i8", i8, "i8"),
            ("i16", i16, "i16"),
            ("i32", i32, "i32"),
            ("i64", i64, "i64"),
            ("i128", i128, "i128"),
            ("f32", f32, "f32"),
            ("f64", f64, "f64"),
            ("r64", crate::R64, "r64"),
            ("c64", crate::C64, "c64"),
        )?;
        mech_core::install_typed_runtime_factories!(
            builder,
            NegateS;
            ("i8", i8, "i8"),
            ("i16", i16, "i16"),
            ("i32", i32, "i32"),
            ("i64", i64, "i64"),
            ("i128", i128, "i128"),
            ("f32", f32, "f32"),
            ("f64", f64, "f64"),
            ("r64", crate::R64, "r64"),
            ("c64", crate::C64, "c64"),
        )?;
    }

    #[cfg(feature = "abs")]
    mech_core::install_unop_runtime_factories!(
        builder,
        MathAbs;
        ("u8", u8),
        ("u16", u16),
        ("u32", u32),
        ("u64", u64),
        ("u128", u128),
        ("i8", i8),
        ("i16", i16),
        ("i32", i32),
        ("i64", i64),
        ("i128", i128),
        ("f32", f32),
        ("f64", f64),
        ("c64", C64),
        ("r64", R64),
    )?;

    #[cfg(feature = "j0")]
    install_float_unop!(builder, MathJ0);
    #[cfg(feature = "j1")]
    install_float_unop!(builder, MathJ1);
    #[cfg(feature = "y0")]
    install_float_unop!(builder, MathY0);
    #[cfg(feature = "y1")]
    install_float_unop!(builder, MathY1);

    #[cfg(feature = "lgamma")]
    install_float_unop!(builder, MathLgamma);
    #[cfg(feature = "tgamma")]
    install_float_unop!(builder, MathTgamma);
    #[cfg(feature = "log")]
    install_float_unop!(builder, MathLog);
    #[cfg(feature = "log10")]
    install_float_unop!(builder, MathLog10);
    #[cfg(feature = "log1p")]
    install_float_unop!(builder, MathLog1p);
    #[cfg(feature = "log2")]
    install_float_unop!(builder, MathLog2);
    #[cfg(feature = "cbrt")]
    install_float_unop!(builder, MathCbrt);
    #[cfg(feature = "sqrt")]
    install_float_unop!(builder, MathSqrt);

    #[cfg(feature = "ceil")]
    install_float_unop!(builder, MathCeil);
    #[cfg(feature = "floor")]
    install_float_unop!(builder, MathFloor);
    #[cfg(feature = "rint")]
    install_float_unop!(builder, MathRint);
    #[cfg(feature = "round")]
    install_float_unop!(builder, MathRound);
    #[cfg(feature = "roundeven")]
    install_float_unop!(builder, MathRoundeven);
    #[cfg(feature = "trunc")]
    install_float_unop!(builder, MathTrunc);
    #[cfg(feature = "erf")]
    install_float_unop!(builder, MathErf);
    #[cfg(feature = "erfc")]
    install_float_unop!(builder, MathErfc);

    #[cfg(feature = "acos")]
    install_float_unop!(builder, MathAcos);
    #[cfg(feature = "acosh")]
    install_float_unop!(builder, MathAcosh);
    #[cfg(feature = "acot")]
    install_float_unop!(builder, MathAcot);
    #[cfg(feature = "acsc")]
    install_float_unop!(builder, MathAcsc);
    #[cfg(feature = "asec")]
    install_float_unop!(builder, MathAsec);
    #[cfg(feature = "asin")]
    install_float_unop!(builder, MathAsin);
    #[cfg(feature = "asinh")]
    install_float_unop!(builder, MathAsinh);
    #[cfg(feature = "atan")]
    install_float_unop!(builder, MathAtan);
    #[cfg(feature = "atan2")]
    install_atan2_runtime(builder)?;
    #[cfg(feature = "atanh")]
    install_float_unop!(builder, MathAtanh);
    #[cfg(feature = "cos")]
    install_float_unop!(builder, MathCos);
    #[cfg(feature = "cosh")]
    install_float_unop!(builder, MathCosh);
    #[cfg(feature = "cot")]
    install_float_unop!(builder, MathCot);
    #[cfg(feature = "csc")]
    install_float_unop!(builder, MathCsc);
    #[cfg(feature = "sec")]
    install_float_unop!(builder, MathSec);
    #[cfg(feature = "sin")]
    install_float_unop!(builder, MathSin);
    #[cfg(feature = "sinh")]
    install_float_unop!(builder, MathSinh);
    #[cfg(feature = "tan")]
    install_float_unop!(builder, MathTan);
    #[cfg(feature = "tanh")]
    install_float_unop!(builder, MathTanh);

    Ok(())
}

/// Installs the complete explicit static catalog fragment owned by
/// `mech-math`.
pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{FunctionCatalog, FunctionDescriptor, OperationId, RuntimeFunctionId};
    use std::collections::{BTreeMap, BTreeSet};

    #[cfg(feature = "math_default")]
    const FROZEN_NAMES: [&str; 64] = [
        "math/abs",
        "math/acos",
        "math/acosh",
        "math/acot",
        "math/acsc",
        "math/add",
        "math/add-assign",
        "math/add-assign/range",
        "math/add-assign/range-all",
        "math/asec",
        "math/asin",
        "math/asinh",
        "math/atan",
        "math/atan2",
        "math/atanh",
        "math/bessel/j0",
        "math/bessel/j1",
        "math/bessel/jn",
        "math/bessel/y0",
        "math/bessel/y1",
        "math/bessel/yn",
        "math/cbrt",
        "math/ceil",
        "math/copysign",
        "math/cos",
        "math/cosh",
        "math/cot",
        "math/csc",
        "math/div",
        "math/div-assign",
        "math/div-assign/range",
        "math/div-assign/range-all",
        "math/erf",
        "math/erfc",
        "math/floor",
        "math/fmod",
        "math/lgamma",
        "math/log",
        "math/log10",
        "math/log1p",
        "math/log2",
        "math/mod",
        "math/mul",
        "math/mul-assign/range",
        "math/mul-assign/range-all",
        "math/neg",
        "math/nextafter",
        "math/pow",
        "math/remainder",
        "math/rint",
        "math/round",
        "math/roundeven",
        "math/sec",
        "math/sin",
        "math/sinh",
        "math/sqrt",
        "math/sub",
        "math/sub-assign",
        "math/sub-assign/range",
        "math/sub-assign/range-all",
        "math/tan",
        "math/tanh",
        "math/tgamma",
        "math/trunc",
    ];

    #[cfg(feature = "math_default")]
    const PRELUDE_NAMES: [&str; 18] = [
        "math/add",
        "math/add-assign",
        "math/add-assign/range",
        "math/add-assign/range-all",
        "math/div",
        "math/div-assign",
        "math/div-assign/range",
        "math/div-assign/range-all",
        "math/mod",
        "math/mul",
        "math/mul-assign/range",
        "math/mul-assign/range-all",
        "math/neg",
        "math/pow",
        "math/sub",
        "math/sub-assign",
        "math/sub-assign/range",
        "math/sub-assign/range-all",
    ];

    #[cfg(feature = "math_default")]
    fn catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        builder.build().unwrap()
    }

    #[cfg(feature = "math_default")]
    #[test]
    fn source_catalog_matches_the_frozen_math_surface() {
        let catalog = catalog();
        let actual = catalog
            .specializer_entries()
            .map(|entry| entry.canonical_name.as_str())
            .collect::<BTreeSet<_>>();
        let expected = FROZEN_NAMES.into_iter().collect::<BTreeSet<_>>();

        assert_eq!(actual, expected);
        assert_eq!(catalog.specializer_count(), 64);

        for excluded in [
            "math/exp",
            "math/exp2",
            "math/exp10",
            "math/expm1",
            "math/fdim",
            "math/hypot",
            "math/ilogb",
            "math/sincos",
            "math/mul-assign",
        ] {
            assert!(
                catalog
                    .specializer(OperationId::from_name(excluded))
                    .is_none()
            );
        }
    }

    #[cfg(feature = "math_default")]
    #[test]
    fn source_catalog_preserves_prelude_and_module_exposure() {
        let catalog = catalog();
        let prelude = PRELUDE_NAMES.into_iter().collect::<BTreeSet<_>>();

        for name in FROZEN_NAMES {
            let operation = OperationId::from_name(name);
            let exports = catalog.exports_for_operation(operation);
            assert_eq!(exports.len(), 1, "unexpected export count for {name}");
            let export = &exports[0];

            if prelude.contains(name) {
                assert_eq!(export.exposure, FunctionExposure::Prelude, "{name}");
                assert_eq!(export.module, None, "{name}");
                assert_eq!(export.item, None, "{name}");
            } else {
                let item = name.strip_prefix("math/").unwrap();
                assert_eq!(export.exposure, FunctionExposure::ModuleOnly, "{name}");
                assert_eq!(export.module.as_deref(), Some("math"), "{name}");
                assert_eq!(export.item.as_deref(), Some(item), "{name}");
                assert_eq!(catalog.module_export("math", item), Some(export), "{name}");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn runtime_catalog_matches_legacy_inventory_names_ids_and_pointers() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let explicit = catalog
            .runtime_entries()
            .map(|entry| (entry.name.clone(), entry.factory as usize))
            .collect::<BTreeMap<_, _>>();
        let mut legacy = BTreeMap::new();
        for descriptor in inventory::iter::<FunctionDescriptor> {
            let stem = descriptor.name.split('<').next().unwrap_or(descriptor.name);
            if stem.starts_with("Math")
                || stem.starts_with("Atan2")
                || stem.starts_with("Negate")
                || ["Add", "Div", "Mod", "Mul", "Pow", "Sub"]
                    .iter()
                    .any(|prefix| stem.starts_with(prefix))
            {
                if let Some(existing) = legacy.insert(descriptor.name, descriptor.ptr as usize) {
                    assert_eq!(existing, descriptor.ptr as usize);
                }
            }
        }

        assert_eq!(
            explicit.keys().cloned().collect::<BTreeSet<_>>(),
            legacy
                .keys()
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>()
        );
        for (name, pointer) in legacy {
            let id = RuntimeFunctionId::from_name(name);
            let entry = catalog
                .runtime_entry(id)
                .unwrap_or_else(|| panic!("missing explicit runtime factory {name}"));
            assert_eq!(entry.id, id, "runtime ID mismatch for {name}");
            assert_eq!(entry.name, name);
            assert_eq!(
                entry.factory as usize, pointer,
                "factory mismatch for {name}"
            );
        }
    }
}
