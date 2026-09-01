use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(any(
    feature = "abs",
    feature = "neg",
    feature = "op_assign",
    feature = "atan2",
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
use mech_core::{RuntimeFunctionContract, RuntimeOutputAliasPolicy};
#[cfg(all(feature = "op_assign", feature = "matrix"))]
use mech_core::{
    DimensionExpr, SchemaBody, ValueCell, ValueData, function_shape_contract_violation,
};
#[cfg(all(feature = "op_assign", feature = "matrix"))]
use mech_core::snapshot::SequenceView;
#[cfg(feature = "source")]
use mech_core::{CanonicalFunctionSpecializer, FunctionExport, FunctionExposure};
#[cfg(all(feature = "op_assign", feature = "matrixd"))]
use nalgebra::DMatrix;
#[cfg(all(feature = "op_assign", feature = "vectord"))]
use nalgebra::DVector;
#[cfg(all(feature = "op_assign", feature = "matrix1"))]
use nalgebra::Matrix1;
#[cfg(all(feature = "op_assign", feature = "matrix2"))]
use nalgebra::Matrix2;
#[cfg(all(feature = "op_assign", feature = "matrix2x3"))]
use nalgebra::Matrix2x3;
#[cfg(all(feature = "op_assign", feature = "matrix3"))]
use nalgebra::Matrix3;
#[cfg(all(feature = "op_assign", feature = "matrix3x2"))]
use nalgebra::Matrix3x2;
#[cfg(all(feature = "op_assign", feature = "matrix4"))]
use nalgebra::Matrix4;
#[cfg(all(feature = "op_assign", feature = "row_vectord"))]
use nalgebra::RowDVector;
#[cfg(all(feature = "op_assign", feature = "row_vector2"))]
use nalgebra::RowVector2;
#[cfg(all(feature = "op_assign", feature = "row_vector3"))]
use nalgebra::RowVector3;
#[cfg(all(feature = "op_assign", feature = "row_vector4"))]
use nalgebra::RowVector4;
#[cfg(all(feature = "op_assign", feature = "vector2"))]
use nalgebra::Vector2;
#[cfg(all(feature = "op_assign", feature = "vector3"))]
use nalgebra::Vector3;
#[cfg(all(feature = "op_assign", feature = "vector4"))]
use nalgebra::Vector4;
#[cfg(feature = "source")]
use std::sync::Arc;

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

#[cfg(all(feature = "op_assign", feature = "matrix"))]
fn validate_canonical_op_assign_slice(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let contract = "op_assign_slice";
    let SchemaBody::Matrix { dimensions, .. } = output.closed_schema_body()? else {
        return Err(function_shape_contract_violation(
            contract,
            "output must be matrix-backed",
        ));
    };
    let [DimensionExpr::Constant(rows), DimensionExpr::Constant(columns)] = dimensions.as_ref()
    else {
        return Err(function_shape_contract_violation(
            contract,
            "output matrix dimensions must be resolved",
        ));
    };
    let output_elements = rows.saturating_mul(*columns);
    let Some(index_cell) = inputs.last() else {
        return Err(function_shape_contract_violation(
            contract,
            "indexed assignment is missing its index input",
        ));
    };
    let snapshot = index_cell.snapshot()?;
    let validate_index = |index: u64| -> MResult<()> {
        if index == 0 || index > output_elements {
            return Err(function_shape_contract_violation(
                contract,
                format!("index {index} is outside the valid 1..={output_elements} range"),
            ));
        }
        Ok(())
    };
    match snapshot.data() {
        ValueData::Index(index) => validate_index(*index),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::Index(indices) => {
                for index in indices {
                    validate_index(*index)?;
                }
                Ok(())
            }
            SequenceView::Bool(_) => Ok(()),
            _ => Err(function_shape_contract_violation(
                contract,
                "index matrix must contain index or boolean elements",
            )),
        },
        _ => Err(function_shape_contract_violation(
            contract,
            "index input must be an index scalar or matrix",
        )),
    }
}
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

#[cfg(feature = "source")]
pub(crate) fn install_canonical_source_specializer<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &'static str,
    module: Option<&'static str>,
    item: Option<&'static str>,
    exposure: FunctionExposure,
    specializer: T,
) -> MResult<()>
where
    T: CanonicalFunctionSpecializer + 'static,
{
    let operation = builder.insert_canonical_specializer(canonical_name, Arc::new(specializer))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: module.map(str::to_string),
        item: item.map(str::to_string),
        exposure,
    })
}

#[cfg(all(
    feature = "source",
    any(
        feature = "add_assign",
        feature = "div",
        feature = "div_assign",
        feature = "mod",
        feature = "mul",
        feature = "mul_assign",
        feature = "neg",
        feature = "pow",
        feature = "sub",
        feature = "sub_assign"
    )
))]
macro_rules! install_canonical_prelude {
    ($builder:expr, $name:literal, $compiler:expr) => {
        install_canonical_source_specializer(
            $builder,
            $name,
            None,
            None,
            FunctionExposure::Prelude,
            $compiler,
        )?;
    };
}

#[cfg(all(
    feature = "source",
    any(
        feature = "abs",
        feature = "acos",
        feature = "acosh",
        feature = "acot",
        feature = "acsc",
        feature = "asec",
        feature = "asin",
        feature = "asinh",
        feature = "atan",
        feature = "atan2",
        feature = "atanh",
        feature = "j0",
        feature = "j1",
        feature = "jn",
        feature = "y0",
        feature = "y1",
        feature = "yn",
        feature = "cbrt",
        feature = "ceil",
        feature = "copysign",
        feature = "cos",
        feature = "cosh",
        feature = "cot",
        feature = "csc",
        feature = "erf",
        feature = "erfc",
        feature = "fdim",
        feature = "floor",
        feature = "fmod",
        feature = "lgamma",
        feature = "log",
        feature = "log10",
        feature = "log1p",
        feature = "log2",
        feature = "nextafter",
        feature = "remainder",
        feature = "rint",
        feature = "round",
        feature = "roundeven",
        feature = "sec",
        feature = "sin",
        feature = "sinh",
        feature = "sqrt",
        feature = "tan",
        feature = "tanh",
        feature = "tgamma",
        feature = "trunc"
    )
))]
macro_rules! install_math_module {
    ($builder:expr, $name:literal, $item:literal, $compiler:expr) => {
        install_canonical_source_specializer(
            $builder,
            $name,
            Some("math"),
            Some($item),
            FunctionExposure::ModuleOnly,
            $compiler,
        )?;
    };
}

/// Installs the supported source-specializer surface owned by `mech-math`.
///
/// Each entry follows the feature gate of its concrete implementation. The
/// source surface intentionally excludes legacy descriptors that were not in
/// the compatibility baseline (`exp`, `exp2`, `exp10`, and `expm1`).
#[cfg(feature = "source")]
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
        install_add_assign_source_runtime(builder)?;
        install_canonical_prelude!(builder, "math/add-assign", crate::AddAssignMath {});
        install_canonical_prelude!(builder, "math/add-assign/range", crate::AddAssignRange {});
        install_canonical_prelude!(
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
    install_canonical_prelude!(builder, "math/div", crate::MathDiv {});
    #[cfg(feature = "div_assign")]
    {
        install_div_assign_source_runtime(builder)?;
        install_canonical_prelude!(builder, "math/div-assign", crate::DivAssignValue {});
        install_canonical_prelude!(builder, "math/div-assign/range", crate::DivAssignRange {});
        install_canonical_prelude!(
            builder,
            "math/div-assign/range-all",
            crate::DivAssignRangeAll {}
        );
    }

    #[cfg(feature = "erf")]
    install_math_module!(builder, "math/erf", "erf", crate::MathErf {});
    #[cfg(feature = "erfc")]
    install_math_module!(builder, "math/erfc", "erfc", crate::MathErfc {});
    #[cfg(feature = "fdim")]
    install_math_module!(builder, "math/fdim", "fdim", crate::MathFdim {});
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
    install_canonical_prelude!(builder, "math/mod", crate::MathMod {});
    #[cfg(feature = "mul")]
    install_canonical_prelude!(builder, "math/mul", crate::MathMul {});
    #[cfg(feature = "mul_assign")]
    {
        install_mul_assign_source_runtime(builder)?;
        // The baseline contains these two range forms, but no named
        // `math/mul-assign` source specializer.
        builder
            .insert_canonical_intrinsic_specializer(
                "math/mul-assign",
                Arc::new(crate::MulAssignValue {}),
            )?;
        install_canonical_prelude!(builder, "math/mul-assign/range", crate::MulAssignRange {});
        install_canonical_prelude!(
            builder,
            "math/mul-assign/range-all",
            crate::MulAssignRangeAll {}
        );
    }
    #[cfg(feature = "neg")]
    install_canonical_prelude!(builder, "math/neg", crate::MathNegate {});

    #[cfg(feature = "nextafter")]
    install_math_module!(
        builder,
        "math/nextafter",
        "nextafter",
        crate::MathNextafter {}
    );
    #[cfg(feature = "pow")]
    install_canonical_prelude!(builder, "math/pow", crate::MathPow {});
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
    install_canonical_prelude!(builder, "math/sub", crate::MathSub {});
    #[cfg(feature = "sub_assign")]
    {
        install_sub_assign_source_runtime(builder)?;
        install_canonical_prelude!(builder, "math/sub-assign", crate::SubAssignValue {});
        install_canonical_prelude!(builder, "math/sub-assign/range", crate::SubAssignRange {});
        install_canonical_prelude!(
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

macro_rules! for_each_math_unop_shape {
    ($callback:path, $context:tt) => {
        $callback!($context, S, none);
        #[cfg(feature = "matrix1")]
        $callback!($context, M1, "matrix1");
        #[cfg(feature = "matrix2")]
        $callback!($context, M2, "matrix2");
        #[cfg(feature = "matrix3")]
        $callback!($context, M3, "matrix3");
        #[cfg(feature = "matrix4")]
        $callback!($context, M4, "matrix4");
        #[cfg(feature = "matrix2x3")]
        $callback!($context, M2x3, "matrix2x3");
        #[cfg(feature = "matrix3x2")]
        $callback!($context, M3x2, "matrix3x2");
        #[cfg(feature = "matrixd")]
        $callback!($context, MD, "matrixd");
        #[cfg(feature = "row_vector2")]
        $callback!($context, R2, "row_vector2");
        #[cfg(feature = "row_vector3")]
        $callback!($context, R3, "row_vector3");
        #[cfg(feature = "row_vector4")]
        $callback!($context, R4, "row_vector4");
        #[cfg(feature = "row_vectord")]
        $callback!($context, RD, "row_vectord");
        #[cfg(feature = "vector2")]
        $callback!($context, V2, "vector2");
        #[cfg(feature = "vector3")]
        $callback!($context, V3, "vector3");
        #[cfg(feature = "vector4")]
        $callback!($context, V4, "vector4");
        #[cfg(feature = "vectord")]
        $callback!($context, VD, "vectord");
    };
}

macro_rules! declare_math_float_unop_factory {
    (($operation:ident; $operation_feature:literal; $scalar:ident; $scalar_feature:literal), $suffix:ident, none) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = $scalar_feature),
                registration: [<register_ $operation:snake _ $suffix:lower _ $scalar:lower>],
                installer: [<install_ $operation:snake _ $suffix:lower _ $scalar:lower>],
                name: stringify!([<$operation $scalar:camel $suffix>]),
                factory_type: [<$operation $scalar:camel $suffix>],
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $operation:snake _ $suffix:lower _ $scalar:lower>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
    (($operation:ident; $operation_feature:literal; $scalar:ident; $scalar_feature:literal), $suffix:ident, $shape_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = $scalar_feature),
                registration: [<register_ $operation:snake _ $suffix:lower _ $scalar:lower>],
                installer: [<install_ $operation:snake _ $suffix:lower _ $scalar:lower>],
                name: stringify!([<$operation $scalar:camel $suffix>]),
                factory_type: [<$operation $scalar:camel $suffix>],
                contract: RuntimeFunctionContract::output_matches_input(
                    0,
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $operation:snake _ $suffix:lower _ $scalar:lower>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
}

#[cfg(any(
    feature = "j0", feature = "j1", feature = "y0", feature = "y1",
    feature = "lgamma", feature = "tgamma",
    feature = "log", feature = "log10", feature = "log1p", feature = "log2",
    feature = "cbrt", feature = "sqrt",
    feature = "ceil", feature = "floor", feature = "rint", feature = "round",
    feature = "roundeven", feature = "trunc",
    feature = "erf", feature = "erfc",
    feature = "acos", feature = "acosh", feature = "acot", feature = "acsc",
    feature = "asec", feature = "asin", feature = "asinh", feature = "atan",
    feature = "atanh", feature = "cos", feature = "cosh", feature = "cot",
    feature = "csc", feature = "sec", feature = "sin", feature = "sinh",
    feature = "tan", feature = "tanh"
))]
macro_rules! register_math_float_unop_factory {
    (($builder:ident; $operation:ident; $_operation_feature:literal; $scalar:ident; $_scalar_feature:literal), $suffix:ident, $_shape_feature:tt) => {
        mech_core::paste::paste! { [<register_ $operation:snake _ $suffix:lower _ $scalar:lower>]($builder)?; }
    };
}

#[cfg(all(
    feature = "native-link",
    any(feature = "f32", feature = "f64"),
    any(
        feature = "j0", feature = "j1", feature = "y0", feature = "y1",
        feature = "lgamma", feature = "tgamma",
        feature = "log", feature = "log10", feature = "log1p", feature = "log2",
        feature = "cbrt", feature = "sqrt",
        feature = "ceil", feature = "floor", feature = "rint", feature = "round",
        feature = "roundeven", feature = "trunc",
        feature = "erf", feature = "erfc",
        feature = "acos", feature = "acosh", feature = "acot", feature = "acsc",
        feature = "asec", feature = "asin", feature = "asinh", feature = "atan",
        feature = "atanh", feature = "cos", feature = "cosh", feature = "cot",
        feature = "csc", feature = "sec", feature = "sin", feature = "sinh",
        feature = "tan", feature = "tanh"
    )
))]
macro_rules! export_math_float_unop_factory {
    (($operation:ident; $_operation_feature:literal; $scalar:ident; $_scalar_feature:literal), $suffix:ident, $_shape_feature:tt) => {
        mech_core::paste::paste! { pub use super::[<install_ $operation:snake _ $suffix:lower _ $scalar:lower>]; }
    };
}

macro_rules! declare_math_float_unop {
    ($operation:ident, $operation_feature:literal) => {
        for_each_math_unop_shape!(declare_math_float_unop_factory, ($operation; $operation_feature; f32; "f32"));
        for_each_math_unop_shape!(declare_math_float_unop_factory, ($operation; $operation_feature; f64; "f64"));
    };
}

#[cfg(any(
    feature = "j0", feature = "j1", feature = "y0", feature = "y1",
    feature = "lgamma", feature = "tgamma",
    feature = "log", feature = "log10", feature = "log1p", feature = "log2",
    feature = "cbrt", feature = "sqrt",
    feature = "ceil", feature = "floor", feature = "rint", feature = "round",
    feature = "roundeven", feature = "trunc",
    feature = "erf", feature = "erfc",
    feature = "acos", feature = "acosh", feature = "acot", feature = "acsc",
    feature = "asec", feature = "asin", feature = "asinh", feature = "atan",
    feature = "atanh", feature = "cos", feature = "cosh", feature = "cot",
    feature = "csc", feature = "sec", feature = "sin", feature = "sinh",
    feature = "tan", feature = "tanh"
))]
macro_rules! install_math_float_unop {
    ($builder:ident, $operation:ident, $operation_feature:literal) => {
        #[cfg(feature = "f32")]
        for_each_math_unop_shape!(register_math_float_unop_factory, ($builder; $operation; $operation_feature; f32; "f32"));
        #[cfg(feature = "f64")]
        for_each_math_unop_shape!(register_math_float_unop_factory, ($builder; $operation; $operation_feature; f64; "f64"));
    };
}

macro_rules! math_float_unop_families {
    ($callback:ident) => {
        $callback!(MathJ0, "j0");
        $callback!(MathJ1, "j1");
        $callback!(MathY0, "y0");
        $callback!(MathY1, "y1");
        $callback!(MathLgamma, "lgamma");
        $callback!(MathTgamma, "tgamma");
        $callback!(MathLog, "log");
        $callback!(MathLog10, "log10");
        $callback!(MathLog1p, "log1p");
        $callback!(MathLog2, "log2");
        $callback!(MathCbrt, "cbrt");
        $callback!(MathSqrt, "sqrt");
        $callback!(MathCeil, "ceil");
        $callback!(MathFloor, "floor");
        $callback!(MathRint, "rint");
        $callback!(MathRound, "round");
        $callback!(MathRoundeven, "roundeven");
        $callback!(MathTrunc, "trunc");
        $callback!(MathErf, "erf");
        $callback!(MathErfc, "erfc");
        $callback!(MathAcos, "acos");
        $callback!(MathAcosh, "acosh");
        $callback!(MathAcot, "acot");
        $callback!(MathAcsc, "acsc");
        $callback!(MathAsec, "asec");
        $callback!(MathAsin, "asin");
        $callback!(MathAsinh, "asinh");
        $callback!(MathAtan, "atan");
        $callback!(MathAtanh, "atanh");
        $callback!(MathCos, "cos");
        $callback!(MathCosh, "cosh");
        $callback!(MathCot, "cot");
        $callback!(MathCsc, "csc");
        $callback!(MathSec, "sec");
        $callback!(MathSin, "sin");
        $callback!(MathSinh, "sinh");
        $callback!(MathTan, "tan");
        $callback!(MathTanh, "tanh");
    };
}

math_float_unop_families!(declare_math_float_unop);

macro_rules! for_each_math_abs_scalar {
    ($callback:ident, $($context:tt)*) => {
        $callback!($($context)*; u8; u8; "u8"; "u8");
        $callback!($($context)*; u16; u16; "u16"; "u16");
        $callback!($($context)*; u32; u32; "u32"; "u32");
        $callback!($($context)*; u64; u64; "u64"; "u64");
        $callback!($($context)*; u128; u128; "u128"; "u128");
        $callback!($($context)*; i8; i8; "i8"; "i8");
        $callback!($($context)*; i16; i16; "i16"; "i16");
        $callback!($($context)*; i32; i32; "i32"; "i32");
        $callback!($($context)*; i64; i64; "i64"; "i64");
        $callback!($($context)*; i128; i128; "i128"; "i128");
        $callback!($($context)*; f32; f32; "f32"; "f32");
        $callback!($($context)*; f64; f64; "f64"; "f64");
        $callback!($($context)*; c64; crate::C64; "c64"; "complex");
        $callback!($($context)*; r64; crate::R64; "r64"; "rational");
    };
}

macro_rules! declare_math_abs_factory {
    (($scalar_token:ident; $scalar:ty; $scalar_cfg:literal; $scalar_feature:literal), $suffix:ident, none) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "abs", feature = $scalar_cfg),
                registration: [<register_math_abs_ $scalar_token _ $suffix:lower>],
                installer: [<install_math_abs_ $scalar_token _ $suffix:lower>],
                name: stringify!([<MathAbs $scalar_token:camel $suffix>]),
                factory_type: [<MathAbs $scalar_token:camel $suffix>],
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_math_abs_ $scalar_token _ $suffix:lower>])),
                extra_cargo_features: ["abs"],
            }
        }
    };
    (($scalar_token:ident; $scalar:ty; $scalar_cfg:literal; $scalar_feature:literal), $suffix:ident, $shape_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "abs", feature = $scalar_cfg),
                registration: [<register_math_abs_ $scalar_token _ $suffix:lower>],
                installer: [<install_math_abs_ $scalar_token _ $suffix:lower>],
                name: stringify!([<MathAbs $scalar_token:camel $suffix>]),
                factory_type: [<MathAbs $scalar_token:camel $suffix>],
                contract: RuntimeFunctionContract::output_matches_input(
                    0,
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_math_abs_ $scalar_token _ $suffix:lower>])),
                extra_cargo_features: ["abs"],
            }
        }
    };
}

macro_rules! declare_math_abs_for_scalar {
    (; $scalar_token:ident; $scalar:ty; $scalar_cfg:literal; $scalar_feature:literal) => {
        for_each_math_unop_shape!(declare_math_abs_factory, ($scalar_token; $scalar; $scalar_cfg; $scalar_feature));
    };
}

for_each_math_abs_scalar!(declare_math_abs_for_scalar,);

#[cfg(feature = "abs")]
macro_rules! register_math_abs_factory {
    (($builder:ident; $scalar_token:ident), $suffix:ident, $_shape_feature:tt) => {
        mech_core::paste::paste! { [<register_math_abs_ $scalar_token _ $suffix:lower>]($builder)?; }
    };
}

#[cfg(all(
    feature = "native-link",
    feature = "abs",
    any(
        feature = "u8", feature = "u16", feature = "u32", feature = "u64",
        feature = "u128", feature = "i8", feature = "i16", feature = "i32",
        feature = "i64", feature = "i128", feature = "f32", feature = "f64",
        feature = "complex", feature = "rational"
    )
))]
macro_rules! export_math_abs_factory {
    (($scalar_token:ident), $suffix:ident, $_shape_feature:tt) => {
        mech_core::paste::paste! { pub use super::[<install_math_abs_ $scalar_token _ $suffix:lower>]; }
    };
}

#[cfg(feature = "abs")]
macro_rules! install_math_abs_for_scalar {
    ($builder:ident; $scalar_token:ident; $_scalar:ty; $scalar_cfg:literal; $_scalar_feature:literal) => {
        #[cfg(feature = $scalar_cfg)]
        for_each_math_unop_shape!(register_math_abs_factory, ($builder; $scalar_token));
    };
}

#[cfg(feature = "abs")]
macro_rules! install_math_abs {
    ($builder:ident) => {
        for_each_math_abs_scalar!(install_math_abs_for_scalar, $builder);
    };
}

macro_rules! for_each_math_neg_scalar {
    ($callback:ident, $($context:tt)*) => {
        $callback!($($context)*; i8; i8; "i8"; "i8");
        $callback!($($context)*; i16; i16; "i16"; "i16");
        $callback!($($context)*; i32; i32; "i32"; "i32");
        $callback!($($context)*; i64; i64; "i64"; "i64");
        $callback!($($context)*; i128; i128; "i128"; "i128");
        $callback!($($context)*; f32; f32; "f32"; "f32");
        $callback!($($context)*; f64; f64; "f64"; "f64");
        $callback!($($context)*; r64; crate::R64; "r64"; "rational");
        $callback!($($context)*; c64; crate::C64; "c64"; "complex");
    };
}

macro_rules! declare_math_neg_factory {
    ($_context:tt; $factory:ident; $scalar_token:ident; $scalar:ty; $scalar_cfg:literal; $scalar_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "neg", feature = $scalar_cfg),
                registration: [<register_ $factory:snake _ $scalar_token>],
                installer: [<install_ $factory:snake _ $scalar_token>],
                name: concat!(stringify!($factory), "<", stringify!($scalar_token), ">"),
                factory_type: crate::ops::negate::$factory<$scalar>,
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $factory:snake _ $scalar_token>])),
                extra_cargo_features: ["neg"],
            }
        }
    };
}

macro_rules! declare_math_neg_families_for_scalar {
    (; $scalar_token:ident; $scalar:ty; $scalar_cfg:literal; $scalar_feature:literal) => {
        declare_math_neg_factory!((); NegateS; $scalar_token; $scalar; $scalar_cfg; $scalar_feature);
        declare_math_neg_factory!((); NegateV; $scalar_token; $scalar; $scalar_cfg; $scalar_feature);
    };
}

for_each_math_neg_scalar!(declare_math_neg_families_for_scalar,);

#[cfg(feature = "neg")]
macro_rules! install_math_neg_for_scalar {
    ($builder:ident; $scalar_token:ident; $_scalar:ty; $scalar_cfg:literal; $_scalar_feature:literal) => {
        #[cfg(feature = $scalar_cfg)]
        mech_core::paste::paste! {
            [<register_negate_s_ $scalar_token>]($builder)?;
            [<register_negate_v_ $scalar_token>]($builder)?;
        }
    };
}

#[cfg(feature = "neg")]
macro_rules! install_math_neg {
    ($builder:ident) => {
        for_each_math_neg_scalar!(install_math_neg_for_scalar, $builder);
    };
}

#[cfg(any(
    feature = "j0", feature = "j1", feature = "y0", feature = "y1",
    feature = "lgamma", feature = "tgamma",
    feature = "log", feature = "log10", feature = "log1p", feature = "log2",
    feature = "cbrt", feature = "sqrt",
    feature = "ceil", feature = "floor", feature = "rint", feature = "round",
    feature = "roundeven", feature = "trunc",
    feature = "erf", feature = "erfc",
    feature = "acos", feature = "acosh", feature = "acot", feature = "acsc",
    feature = "asec", feature = "asin", feature = "asinh", feature = "atan",
    feature = "atanh", feature = "cos", feature = "cosh", feature = "cot",
    feature = "csc", feature = "sec", feature = "sin", feature = "sinh",
    feature = "tan", feature = "tanh"
))]
macro_rules! install_float_unop {
    ($builder:ident, $family:ident) => {
        install_math_float_unop!($builder, $family, "");
    };
}

// Assignment factories have three independent dimensions (value type, mutable
// sink storage, and range/index storage).  Keep that traversal owner-local so
// the runtime catalogue, native-plan linkage, and native-link exports expand
// from precisely the same concrete factory list.
#[cfg(feature = "op_assign")]
macro_rules! for_each_op_assign_scalar {
    ($callback:ident, $context:tt) => {
        $callback!($context; u8; u8; "u8"; "u8"; "u8");
        $callback!($context; u16; u16; "u16"; "u16"; "u16");
        $callback!($context; u32; u32; "u32"; "u32"; "u32");
        $callback!($context; u64; u64; "u64"; "u64"; "u64");
        $callback!($context; u128; u128; "u128"; "u128"; "u128");
        $callback!($context; i8; i8; "i8"; "i8"; "i8");
        $callback!($context; i16; i16; "i16"; "i16"; "i16");
        $callback!($context; i32; i32; "i32"; "i32"; "i32");
        $callback!($context; i64; i64; "i64"; "i64"; "i64");
        $callback!($context; i128; i128; "i128"; "i128"; "i128");
        $callback!($context; f32; f32; "f32"; "f32"; "f32");
        $callback!($context; f64; f64; "f64"; "f64"; "f64");
        $callback!($context; r64; crate::R64; "r64"; "r64"; "rational");
        $callback!($context; c64; crate::C64; "c64"; "c64"; "complex");
    };
}

// Range assignment was historically narrower than value assignment: it does
// not instantiate i128 and its canonical runtime names use the feature
// spellings for rational and complex values. Keep that fact in this shared
// traversal instead of approximating it with the value-factory list.
#[cfg(feature = "op_assign")]
macro_rules! for_each_op_assign_range_scalar {
    ($callback:ident, $context:tt) => {
        $callback!($context; u8; u8; "u8"; "u8"; "u8");
        $callback!($context; u16; u16; "u16"; "u16"; "u16");
        $callback!($context; u32; u32; "u32"; "u32"; "u32");
        $callback!($context; u64; u64; "u64"; "u64"; "u64");
        $callback!($context; u128; u128; "u128"; "u128"; "u128");
        $callback!($context; i8; i8; "i8"; "i8"; "i8");
        $callback!($context; i16; i16; "i16"; "i16"; "i16");
        $callback!($context; i32; i32; "i32"; "i32"; "i32");
        $callback!($context; i64; i64; "i64"; "i64"; "i64");
        $callback!($context; f32; f32; "f32"; "f32"; "f32");
        $callback!($context; f64; f64; "f64"; "f64"; "f64");
        $callback!($context; r64; crate::R64; "rational"; "rational"; "rational");
        $callback!($context; c64; crate::C64; "complex"; "complex"; "complex");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! for_each_op_assign_shape {
    ($callback:ident, $context:tt) => {
        $callback!($context; RowVector4; "row_vector4"; "1,4");
        $callback!($context; RowVector3; "row_vector3"; "1,3");
        $callback!($context; RowVector2; "row_vector2"; "1,2");
        $callback!($context; Vector2; "vector2"; "2,1");
        $callback!($context; Vector3; "vector3"; "3,1");
        $callback!($context; Vector4; "vector4"; "4,1");
        $callback!($context; Matrix1; "matrix1"; "1,1");
        $callback!($context; Matrix2; "matrix2"; "2,2");
        $callback!($context; Matrix3; "matrix3"; "3,3");
        $callback!($context; Matrix4; "matrix4"; "4,4");
        $callback!($context; Matrix2x3; "matrix2x3"; "2,3");
        $callback!($context; Matrix3x2; "matrix3x2"; "3,2");
        $callback!($context; DVector; "vectord"; "0,1");
        $callback!($context; DMatrix; "matrixd"; "0,0");
        $callback!($context; RowDVector; "row_vectord"; "1,0");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! for_each_op_assign_index_shape {
    ($callback:ident, $context:tt) => {
        $callback!($context; Matrix1; "matrix1");
        $callback!($context; Vector2; "vector2");
        $callback!($context; Vector3; "vector3");
        $callback!($context; Vector4; "vector4");
        $callback!($context; DVector; "vectord");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! for_each_canonical_op_assign_index_shape {
    ($callback:ident, $context:tt) => {
        for_each_op_assign_index_shape!($callback, $context);
        $callback!($context; RowDVector; "row_vectord");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! for_each_op_assign_vector_range_source {
    ($callback:ident, $context:tt) => {
        $callback!($context; feature = "matrix1"; Matrix1; "matrix1"; Matrix1; "matrix1");
        $callback!($context; all(feature = "matrix2", feature = "vector4"); Matrix2; "matrix2"; Vector4; "vector4");
        $callback!($context; feature = "matrix3"; Matrix3; "matrix3"; DVector; "vectord");
        $callback!($context; feature = "matrix4"; Matrix4; "matrix4"; DVector; "vectord");
        $callback!($context; feature = "matrix2x3"; Matrix2x3; "matrix2x3"; DVector; "vectord");
        $callback!($context; feature = "matrix3x2"; Matrix3x2; "matrix3x2"; DVector; "vectord");
        $callback!($context; feature = "matrixd"; DMatrix; "matrixd"; DVector; "vectord");
        $callback!($context; feature = "vectord"; DVector; "vectord"; DVector; "vectord");
        $callback!($context; feature = "row_vectord"; RowDVector; "row_vectord"; DVector; "vectord");
        $callback!($context; feature = "vector2"; Vector2; "vector2"; Vector2; "vector2");
        $callback!($context; feature = "vector3"; Vector3; "vector3"; Vector3; "vector3");
        $callback!($context; feature = "vector4"; Vector4; "vector4"; Vector4; "vector4");
        $callback!($context; feature = "row_vector2"; RowVector2; "row_vector2"; Vector2; "vector2");
        $callback!($context; feature = "row_vector3"; RowVector3; "row_vector3"; Vector3; "vector3");
        $callback!($context; feature = "row_vector4"; RowVector4; "row_vector4"; Vector4; "vector4");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! for_each_source_only_op_assign_vector_range_source {
    ($callback:ident, $context:tt) => {
        $callback!($context; all(feature = "matrix1", feature = "row_vectord"); Matrix1; "matrix1"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "matrix2", feature = "vector4", feature = "row_vectord"); Matrix2; "matrix2"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "matrix3", feature = "row_vectord"); Matrix3; "matrix3"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "matrix4", feature = "row_vectord"); Matrix4; "matrix4"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "matrix2x3", feature = "row_vectord"); Matrix2x3; "matrix2x3"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "matrix3x2", feature = "row_vectord"); Matrix3x2; "matrix3x2"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "matrixd", feature = "row_vectord"); DMatrix; "matrixd"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "vectord", feature = "row_vectord"); DVector; "vectord"; RowDVector; "row_vectord");
        $callback!($context; feature = "row_vectord"; RowDVector; "row_vectord"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "vector2", feature = "row_vectord"); Vector2; "vector2"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "vector3", feature = "row_vectord"); Vector3; "vector3"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "vector4", feature = "row_vectord"); Vector4; "vector4"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "row_vector2", feature = "row_vectord"); RowVector2; "row_vector2"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "row_vector3", feature = "row_vectord"); RowVector3; "row_vector3"; RowDVector; "row_vectord");
        $callback!($context; all(feature = "row_vector4", feature = "row_vectord"); RowVector4; "row_vector4"; RowDVector; "row_vectord");
    };
}

#[cfg(feature = "op_assign")]
macro_rules! for_each_canonical_op_assign_vector_range_source {
    ($callback:ident, $context:tt) => {
        for_each_op_assign_vector_range_source!($callback, $context);
        for_each_source_only_op_assign_vector_range_source!($callback, $context);
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_ss {
    (($operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = $scalar_cfg),
                registration: [<register_ $operation:snake _assign_ss_ $scalar_token>],
                installer: [<install_ $operation:snake _assign_ss_ $scalar_token>],
                name: concat!(stringify!($operation), "AssignSS<", $scalar_name, ">"),
                factory_type: [<$operation AssignSS>]<$scalar>,
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $operation:snake _assign_ss_ $scalar_token>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_vv {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal); $shape:ident; $shape_feature:literal; $shape_name:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = $scalar_cfg, feature = $shape_feature),
                registration: [<register_ $operation:snake _assign_vv_ $shape:snake _ $scalar_token>],
                installer: [<install_ $operation:snake _assign_vv_ $shape:snake _ $scalar_token>],
                name: concat!(stringify!($operation), "AssignVV<[", $scalar_name, "]:", $shape_name, ">"),
                factory_type: [<$operation AssignVV>]<$scalar, $shape<$scalar>, $shape<$scalar>>,
                contract: RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $operation:snake _assign_vv_ $shape:snake _ $scalar_token>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_range_s {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal; $sink:ident; $sink_feature:literal; $family:ident; $index_scalar:ty); $index:ident; $index_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = $scalar_cfg, feature = $sink_feature, feature = $index_feature),
                registration: [<register_ $operation:snake _assign_ $family:snake _ $sink:snake _ $index:snake _ $scalar_token>],
                installer: [<install_ $operation:snake _assign_ $family:snake _ $sink:snake _ $index:snake _ $scalar_token>],
                name: concat!(stringify!($operation), stringify!($family), "<", $scalar_name, stringify!($sink), stringify!($index), ">"),
                factory_type: [<$operation $family>]<$scalar, $sink<$scalar>, $index<$index_scalar>>,
                contract: RuntimeFunctionContract::canonical_custom(
                    "op_assign_slice",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    validate_canonical_op_assign_slice,
                ),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $operation:snake _assign_ $family:snake _ $sink:snake _ $index:snake _ $scalar_token>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_range_v {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal; $sink:ident; $sink_feature:literal; $family:ident; $index_scalar:ty); $source_cfg:meta; $source:ident; $source_feature:literal; $index:ident; $index_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = $scalar_cfg, feature = $sink_feature, $source_cfg),
                registration: [<register_ $operation:snake _assign_ $family:snake _ $sink:snake _ $source:snake _ $index:snake _ $scalar_token>],
                installer: [<install_ $operation:snake _assign_ $family:snake _ $sink:snake _ $source:snake _ $index:snake _ $scalar_token>],
                name: concat!(stringify!($operation), stringify!($family), "<", $scalar_name, stringify!($sink), stringify!($source), stringify!($index), ">"),
                factory_type: [<$operation $family>]<$scalar, $sink<$scalar>, $source<$scalar>, $index<$index_scalar>>,
                contract: RuntimeFunctionContract::canonical_custom(
                    "op_assign_slice",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    validate_canonical_op_assign_slice,
                ),
                package: "mech-math", crate_name: "mech_math",
                installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $operation:snake _assign_ $family:snake _ $sink:snake _ $source:snake _ $index:snake _ $scalar_token>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_ranges_for_sink {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal); $sink:ident; $sink_feature:literal; $_sink_name:literal) => {
        for_each_canonical_op_assign_index_shape!(declare_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRS; usize));
        for_each_canonical_op_assign_index_shape!(declare_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRB; bool));
        for_each_canonical_op_assign_vector_range_source!(declare_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRV; usize));
        for_each_canonical_op_assign_vector_range_source!(declare_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRVB; bool));
        for_each_canonical_op_assign_index_shape!(declare_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAS; usize));
        for_each_canonical_op_assign_index_shape!(declare_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRASB; bool));
        for_each_canonical_op_assign_vector_range_source!(declare_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAV; usize));
        for_each_canonical_op_assign_vector_range_source!(declare_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAVB; bool));
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_value_for_scalar {
    (($operation:ident; $operation_feature:literal); i128; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        declare_op_assign_ss!(($operation; $operation_feature); i128; $scalar; $scalar_name; $scalar_cfg; $scalar_feature);
        for_each_op_assign_shape!(declare_op_assign_vv, ($operation; $operation_feature; i128; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
    (($operation:ident; $operation_feature:literal); r64; $scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        declare_op_assign_ss!(($operation; $operation_feature); r64; $scalar; "r64"; $scalar_cfg; $scalar_feature);
        for_each_op_assign_shape!(declare_op_assign_vv, ($operation; $operation_feature; r64; $scalar; "r64"; $scalar_cfg; $scalar_feature));
    };
    (($operation:ident; $operation_feature:literal); c64; $scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        declare_op_assign_ss!(($operation; $operation_feature); c64; $scalar; "c64"; $scalar_cfg; $scalar_feature);
        for_each_op_assign_shape!(declare_op_assign_vv, ($operation; $operation_feature; c64; $scalar; "c64"; $scalar_cfg; $scalar_feature));
    };
    (($operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        declare_op_assign_ss!(($operation; $operation_feature); $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature);
        for_each_op_assign_shape!(declare_op_assign_vv, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_op_assign_range_for_scalar {
    (($operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        for_each_op_assign_shape!(declare_op_assign_ranges_for_sink, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
}

#[cfg(feature = "op_assign")]
macro_rules! declare_native_op_assign_runtime_factories {
    ($operation:ident; $operation_feature:literal) => {
        for_each_op_assign_scalar!(declare_op_assign_value_for_scalar, ($operation; $operation_feature));
        for_each_op_assign_range_scalar!(declare_op_assign_range_for_scalar, ($operation; $operation_feature));
    };
}

#[cfg(feature = "op_assign")]
declare_native_op_assign_runtime_factories!(Add; "add_assign");
#[cfg(feature = "op_assign")]
declare_native_op_assign_runtime_factories!(Div; "div_assign");
#[cfg(feature = "op_assign")]
declare_native_op_assign_runtime_factories!(Mul; "mul_assign");
#[cfg(feature = "op_assign")]
declare_native_op_assign_runtime_factories!(Sub; "sub_assign");

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_ss {
    (($builder:ident; $operation:ident; $operation_feature:literal); $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $_scalar_feature:literal) => {
        #[cfg(feature = $scalar_cfg)]
        mech_core::paste::paste! { [<register_ $operation:snake _assign_ss_ $scalar_token>]($builder)?; }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_vv {
    (($builder:ident; $operation:ident; $_operation_feature:literal; $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $_scalar_cfg:literal; $_scalar_feature:literal); $shape:ident; $shape_feature:literal; $_shape_name:literal) => {
        #[cfg(feature = $shape_feature)]
        mech_core::paste::paste! { [<register_ $operation:snake _assign_vv_ $shape:snake _ $scalar_token>]($builder)?; }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_range_s {
    (($builder:ident; $operation:ident; $_operation_feature:literal; $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $_scalar_cfg:literal; $_scalar_feature:literal; $sink:ident; $sink_feature:literal; $family:ident); $index:ident; $index_feature:literal) => {
        #[cfg(all(feature = $sink_feature, feature = $index_feature))]
        mech_core::paste::paste! { [<register_ $operation:snake _assign_ $family:snake _ $sink:snake _ $index:snake _ $scalar_token>]($builder)?; }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_range_v {
    (($builder:ident; $operation:ident; $_operation_feature:literal; $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $_scalar_cfg:literal; $_scalar_feature:literal; $sink:ident; $sink_feature:literal; $family:ident); $source_cfg:meta; $source:ident; $_source_feature:literal; $index:ident; $_index_feature:literal) => {
        #[cfg(all(feature = $sink_feature, $source_cfg))]
        mech_core::paste::paste! { [<register_ $operation:snake _assign_ $family:snake _ $sink:snake _ $source:snake _ $index:snake _ $scalar_token>]($builder)?; }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_ranges_for_sink {
    (($builder:ident; $operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal); $sink:ident; $sink_feature:literal; $_sink_name:literal) => {
        for_each_op_assign_index_shape!(register_op_assign_range_s, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRS));
        for_each_op_assign_vector_range_source!(register_op_assign_range_v, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRV));
        for_each_op_assign_index_shape!(register_op_assign_range_s, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAS));
        for_each_op_assign_vector_range_source!(register_op_assign_range_v, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAV));
    };
}

#[cfg(all(feature = "op_assign", feature = "source"))]
macro_rules! register_source_op_assign_ranges_for_sink {
    (($builder:ident; $operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal); $sink:ident; $sink_feature:literal; $_sink_name:literal) => {
        register_op_assign_range_s!(($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRS); RowDVector; "row_vectord");
        for_each_canonical_op_assign_index_shape!(register_op_assign_range_s, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRB));
        for_each_source_only_op_assign_vector_range_source!(register_op_assign_range_v, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRV));
        for_each_canonical_op_assign_vector_range_source!(register_op_assign_range_v, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRVB));
        register_op_assign_range_s!(($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAS); RowDVector; "row_vectord");
        for_each_canonical_op_assign_index_shape!(register_op_assign_range_s, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRASB));
        for_each_source_only_op_assign_vector_range_source!(register_op_assign_range_v, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAV));
        for_each_canonical_op_assign_vector_range_source!(register_op_assign_range_v, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAVB));
    };
}

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_value_for_scalar {
    (($builder:ident; $operation:ident; $operation_feature:literal); i128; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        register_op_assign_ss!(($builder; $operation; $operation_feature); i128; $scalar; $scalar_name; $scalar_cfg; $scalar_feature);
        #[cfg(feature = $scalar_cfg)]
        for_each_op_assign_shape!(register_op_assign_vv, ($builder; $operation; $operation_feature; i128; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
    (($builder:ident; $operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        register_op_assign_ss!(($builder; $operation; $operation_feature); $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature);
        #[cfg(feature = $scalar_cfg)]
        {
            for_each_op_assign_shape!(register_op_assign_vv, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
        }
    };
}

#[cfg(feature = "op_assign")]
macro_rules! register_op_assign_range_for_scalar {
    (($builder:ident; $operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        #[cfg(feature = $scalar_cfg)]
        for_each_op_assign_shape!(register_op_assign_ranges_for_sink, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
}

#[cfg(all(feature = "op_assign", feature = "source"))]
macro_rules! register_source_op_assign_range_for_scalar {
    (($builder:ident; $operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        #[cfg(feature = $scalar_cfg)]
        for_each_op_assign_shape!(register_source_op_assign_ranges_for_sink, ($builder; $operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
}

#[cfg(feature = "op_assign")]
macro_rules! install_native_op_assign_runtime_factories {
    ($builder:ident, $operation:ident; $operation_feature:literal) => {{
        for_each_op_assign_scalar!(register_op_assign_value_for_scalar, ($builder; $operation; $operation_feature));
        for_each_op_assign_range_scalar!(register_op_assign_range_for_scalar, ($builder; $operation; $operation_feature));
        Ok::<(), mech_core::MechError>(())
    }};
}

#[cfg(all(feature = "op_assign", feature = "source"))]
macro_rules! install_source_op_assign_runtime_factories {
    ($builder:ident, $operation:ident; $operation_feature:literal) => {{
        for_each_op_assign_range_scalar!(register_source_op_assign_range_for_scalar, ($builder; $operation; $operation_feature));
        Ok::<(), mech_core::MechError>(())
    }};
}

#[cfg(all(feature = "source", feature = "add_assign"))]
fn install_add_assign_source_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_source_op_assign_runtime_factories!(builder, Add; "add_assign")
}

#[cfg(all(feature = "source", feature = "div_assign"))]
fn install_div_assign_source_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_source_op_assign_runtime_factories!(builder, Div; "div_assign")
}

#[cfg(all(feature = "source", feature = "mul_assign"))]
fn install_mul_assign_source_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_source_op_assign_runtime_factories!(builder, Mul; "mul_assign")
}

#[cfg(all(feature = "source", feature = "sub_assign"))]
fn install_sub_assign_source_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_source_op_assign_runtime_factories!(builder, Sub; "sub_assign")
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_ss {
    (($operation:ident; $operation_feature:literal); $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $_scalar_feature:literal) => {
        #[cfg(all(feature = $operation_feature, feature = $scalar_cfg))]
        mech_core::paste::paste! { pub use super::[<install_ $operation:snake _assign_ss_ $scalar_token>]; }
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_vv {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $_scalar_feature:literal); $shape:ident; $shape_feature:literal; $_shape_name:literal) => {
        #[cfg(all(feature = $operation_feature, feature = $scalar_cfg, feature = $shape_feature))]
        mech_core::paste::paste! { pub use super::[<install_ $operation:snake _assign_vv_ $shape:snake _ $scalar_token>]; }
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_range_s {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $_scalar_feature:literal; $sink:ident; $sink_feature:literal; $family:ident); $index:ident; $index_feature:literal) => {
        #[cfg(all(feature = $operation_feature, feature = $scalar_cfg, feature = $sink_feature, feature = $index_feature))]
        mech_core::paste::paste! { pub use super::[<install_ $operation:snake _assign_ $family:snake _ $sink:snake _ $index:snake _ $scalar_token>]; }
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_range_v {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $_scalar:ty; $_scalar_name:literal; $scalar_cfg:literal; $_scalar_feature:literal; $sink:ident; $sink_feature:literal; $family:ident); $source_cfg:meta; $source:ident; $_source_feature:literal; $index:ident; $_index_feature:literal) => {
        #[cfg(all(feature = $operation_feature, feature = $scalar_cfg, feature = $sink_feature, $source_cfg))]
        mech_core::paste::paste! { pub use super::[<install_ $operation:snake _assign_ $family:snake _ $sink:snake _ $source:snake _ $index:snake _ $scalar_token>]; }
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_ranges_for_sink {
    (($operation:ident; $operation_feature:literal; $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal); $sink:ident; $sink_feature:literal; $_sink_name:literal) => {
        for_each_canonical_op_assign_index_shape!(export_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRS));
        for_each_canonical_op_assign_index_shape!(export_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRB));
        for_each_canonical_op_assign_vector_range_source!(export_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRV));
        for_each_canonical_op_assign_vector_range_source!(export_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign1DRVB));
        for_each_canonical_op_assign_index_shape!(export_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAS));
        for_each_canonical_op_assign_index_shape!(export_op_assign_range_s, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRASB));
        for_each_canonical_op_assign_vector_range_source!(export_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAV));
        for_each_canonical_op_assign_vector_range_source!(export_op_assign_range_v, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature; $sink; $sink_feature; Assign2DRAVB));
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_value_for_scalar {
    (($operation:ident; $operation_feature:literal); i128; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        export_op_assign_ss!(($operation; $operation_feature); i128; $scalar; $scalar_name; $scalar_cfg; $scalar_feature);
        for_each_op_assign_shape!(export_op_assign_vv, ($operation; $operation_feature; i128; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
    (($operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        export_op_assign_ss!(($operation; $operation_feature); $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature);
        for_each_op_assign_shape!(export_op_assign_vv, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_op_assign_range_for_scalar {
    (($operation:ident; $operation_feature:literal); $scalar_token:ident; $scalar:ty; $scalar_name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        for_each_op_assign_shape!(export_op_assign_ranges_for_sink, ($operation; $operation_feature; $scalar_token; $scalar; $scalar_name; $scalar_cfg; $scalar_feature));
    };
}

#[cfg(all(feature = "op_assign", feature = "native-link"))]
macro_rules! export_native_op_assign_runtime_factories {
    ($operation:ident; $operation_feature:literal) => {
        for_each_op_assign_scalar!(export_op_assign_value_for_scalar, ($operation; $operation_feature));
        for_each_op_assign_range_scalar!(export_op_assign_range_for_scalar, ($operation; $operation_feature));
    };
}

#[cfg(feature = "add_assign")]
fn install_add_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_native_op_assign_runtime_factories!(builder, Add; "add_assign")
}

#[cfg(feature = "div_assign")]
fn install_div_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_native_op_assign_runtime_factories!(builder, Div; "div_assign")
}

#[cfg(feature = "mul_assign")]
fn install_mul_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_native_op_assign_runtime_factories!(builder, Mul; "mul_assign")
}

#[cfg(feature = "sub_assign")]
fn install_sub_assign_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_native_op_assign_runtime_factories!(builder, Sub; "sub_assign")
}

macro_rules! for_each_atan2_factory {
    ($callback:ident, $context:tt) => {
        $callback!($context; feature = "f32"; "f32"; Atan2F32; []);
        $callback!($context; all(feature = "f32", feature = "matrix1"); "f32"; Atan2M1F32; ["matrix1"]);
        $callback!($context; all(feature = "f32", feature = "matrix2"); "f32"; Atan2M2F32; ["matrix2"]);
        $callback!($context; all(feature = "f32", feature = "matrix3"); "f32"; Atan2M3F32; ["matrix3"]);
        $callback!($context; all(feature = "f32", feature = "matrix3x2"); "f32"; Atan2M3x2F32; ["matrix3x2"]);
        $callback!($context; all(feature = "f32", feature = "matrix2x3"); "f32"; Atan2M2x3F32; ["matrix2x3"]);
        $callback!($context; all(feature = "f32", feature = "matrix4"); "f32"; Atan2M4F32; ["matrix4"]);
        $callback!($context; all(feature = "f32", feature = "vector2"); "f32"; Atan2V2F32; ["vector2"]);
        $callback!($context; all(feature = "f32", feature = "vector3"); "f32"; Atan2V3F32; ["vector3"]);
        $callback!($context; all(feature = "f32", feature = "vector4"); "f32"; Atan2V4F32; ["vector4"]);
        $callback!($context; all(feature = "f32", feature = "row_vector2"); "f32"; Atan2R2F32; ["row_vector2"]);
        $callback!($context; all(feature = "f32", feature = "row_vector3"); "f32"; Atan2R3F32; ["row_vector3"]);
        $callback!($context; all(feature = "f32", feature = "row_vector4"); "f32"; Atan2R4F32; ["row_vector4"]);
        $callback!($context; all(feature = "f32", feature = "row_vectord"); "f32"; Atan2RDF32; ["row_vectord"]);
        $callback!($context; all(feature = "f32", feature = "vectord"); "f32"; Atan2VDF32; ["vectord"]);
        $callback!($context; all(feature = "f32", feature = "matrixd"); "f32"; Atan2MDF32; ["matrixd"]);
        $callback!($context; feature = "f64"; "f64"; Atan2F64; []);
        $callback!($context; all(feature = "f64", feature = "matrix1"); "f64"; Atan2M1F64; ["matrix1"]);
        $callback!($context; all(feature = "f64", feature = "matrix2"); "f64"; Atan2M2F64; ["matrix2"]);
        $callback!($context; all(feature = "f64", feature = "matrix3"); "f64"; Atan2M3F64; ["matrix3"]);
        $callback!($context; all(feature = "f64", feature = "matrix3x2"); "f64"; Atan2M3x2F64; ["matrix3x2"]);
        $callback!($context; all(feature = "f64", feature = "matrix2x3"); "f64"; Atan2M2x3F64; ["matrix2x3"]);
        $callback!($context; all(feature = "f64", feature = "matrix4"); "f64"; Atan2M4F64; ["matrix4"]);
        $callback!($context; all(feature = "f64", feature = "vector2"); "f64"; Atan2V2F64; ["vector2"]);
        $callback!($context; all(feature = "f64", feature = "vector3"); "f64"; Atan2V3F64; ["vector3"]);
        $callback!($context; all(feature = "f64", feature = "vector4"); "f64"; Atan2V4F64; ["vector4"]);
        $callback!($context; all(feature = "f64", feature = "row_vector2"); "f64"; Atan2R2F64; ["row_vector2"]);
        $callback!($context; all(feature = "f64", feature = "row_vector3"); "f64"; Atan2R3F64; ["row_vector3"]);
        $callback!($context; all(feature = "f64", feature = "row_vector4"); "f64"; Atan2R4F64; ["row_vector4"]);
        $callback!($context; all(feature = "f64", feature = "row_vectord"); "f64"; Atan2RDF64; ["row_vectord"]);
        $callback!($context; all(feature = "f64", feature = "vectord"); "f64"; Atan2VDF64; ["vectord"]);
        $callback!($context; all(feature = "f64", feature = "matrixd"); "f64"; Atan2MDF64; ["matrixd"]);
    };
}

macro_rules! declare_atan2_factory {
    ($_context:tt; $cfg:meta; $scalar_feature:literal; $factory:ident; [$($shape_feature:literal),* $(,)?]) => {
        mech_core::paste::paste! { mech_core::declare_native_runtime_factory! {
            cfg: all(feature = "atan2", $cfg), registration: [<register_ $factory:snake>], installer: [<install_ $factory:snake>],
            name: stringify!($factory), factory_type: crate::trig::atan2::$factory,
            contract: atan2_runtime_contract!([$($shape_feature),*]),
            package: "mech-math", crate_name: "mech_math", installer_path: concat!("mech_math::__mech_native::", stringify!([<install_ $factory:snake>])),
            extra_cargo_features: ["atan2"],
        }}
    };
}

#[cfg(feature = "atan2")]
macro_rules! atan2_runtime_contract {
    ([]) => {
        RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias)
    };
    ([$first:literal $(, $rest:literal)*]) => {
        RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias)
    };
}
for_each_atan2_factory!(declare_atan2_factory, ());

#[cfg(feature = "atan2")]
fn install_atan2_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register_atan2_factory {
        (($builder:ident); $cfg:meta; $_scalar_feature:literal; $factory:ident; [$($_shape_feature:literal),*]) => {
            #[cfg(all(feature = "atan2", $cfg))]
            mech_core::paste::paste! { [<register_ $factory:snake>]($builder)?; }
        };
    }
    for_each_atan2_factory!(register_atan2_factory, (builder));
    Ok(())
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "pow", feature = "rational", feature = "i32"),
    registration: register_pow_rational,
    installer: install_pow_rational,
    name: "PowRational<r64>",
    factory_type: crate::ops::pow::PowRational,
    contract: RuntimeFunctionContract::no_matrix(
        RuntimeOutputAliasPolicy::DisallowInputAlias
    ),
    package: "mech-math",
    crate_name: "mech_math",
    installer_path: "mech_math::__mech_native::install_pow_rational",
    extra_cargo_features: ["pow"],
}

mech_core::declare_native_binop_runtime_factories! {
    package: "mech-math",
    crate_name: "mech_math",
    operation: Div,
    operation_feature: "div",
    additional_features: [],
    scalars:
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64),
        ("complex", crate::C64, "c64", c64),
}

mech_core::declare_native_binop_runtime_factories! {
    package: "mech-math",
    crate_name: "mech_math",
    operation: Mod,
    operation_feature: "mod",
    additional_features: [],
    scalars:
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
}

mech_core::declare_native_binop_runtime_factories! {
    package: "mech-math",
    crate_name: "mech_math",
    operation: Mul,
    operation_feature: "mul",
    additional_features: [],
    scalars:
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64),
        ("complex", crate::C64, "c64", c64),
}

mech_core::declare_native_binop_runtime_factories! {
    package: "mech-math",
    crate_name: "mech_math",
    operation: Pow,
    operation_feature: "pow",
    additional_features: [],
    scalars:
        ("u8", u8, "u8", u8), ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32), ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
}

mech_core::declare_native_binop_runtime_factories! {
    package: "mech-math",
    crate_name: "mech_math",
    operation: Sub,
    operation_feature: "sub",
    additional_features: [],
    scalars:
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64),
        ("complex", crate::C64, "c64", c64),
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(all(feature = "pow", feature = "rational", feature = "i32"))]
    pub use super::install_pow_rational;

    #[cfg(feature = "add_assign")]
    export_native_op_assign_runtime_factories!(Add; "add_assign");
    #[cfg(feature = "div_assign")]
    export_native_op_assign_runtime_factories!(Div; "div_assign");
    #[cfg(feature = "mul_assign")]
    export_native_op_assign_runtime_factories!(Mul; "mul_assign");
    #[cfg(feature = "sub_assign")]
    export_native_op_assign_runtime_factories!(Sub; "sub_assign");

    macro_rules! export_math_float_unop {
        ($operation:ident, $operation_feature:literal) => {
            #[cfg(all(feature = $operation_feature, feature = "f32"))]
            for_each_math_unop_shape!(
                export_math_float_unop_factory,
                ($operation; $operation_feature; f32; "f32")
            );
            #[cfg(all(feature = $operation_feature, feature = "f64"))]
            for_each_math_unop_shape!(
                export_math_float_unop_factory,
                ($operation; $operation_feature; f64; "f64")
            );
        };
    }

    math_float_unop_families!(export_math_float_unop);

    macro_rules! export_math_abs_for_scalar {
        ($_context:tt; $scalar_token:ident; $_scalar:ty; $scalar_cfg:literal; $_scalar_feature:literal) => {
            #[cfg(all(feature = "abs", feature = $scalar_cfg))]
            for_each_math_unop_shape!(export_math_abs_factory, ($scalar_token));
        };
    }

    for_each_math_abs_scalar!(export_math_abs_for_scalar, ());

    macro_rules! export_math_neg_for_scalar {
        ($_context:tt; $scalar_token:ident; $_scalar:ty; $scalar_cfg:literal; $_scalar_feature:literal) => {
            #[cfg(all(feature = "neg", feature = $scalar_cfg))]
            mech_core::paste::paste! {
                pub use super::[<install_negate_s_ $scalar_token>];
                pub use super::[<install_negate_v_ $scalar_token>];
            }
        };
    }

    for_each_math_neg_scalar!(export_math_neg_for_scalar, ());

    macro_rules! export_atan2_factory {
        ($_context:tt; $cfg:meta; $_scalar_feature:literal; $factory:ident; [$($_shape_feature:literal),* $(,)?]) => {
            #[cfg(all(feature = "atan2", $cfg))]
            mech_core::paste::paste! { pub use super::[<install_ $factory:snake>]; }
        };
    }

    for_each_atan2_factory!(export_atan2_factory, ());

    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "div", operation: Div;
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64), ("complex", crate::C64, "c64", c64),
    }
    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "mod", operation: Mod;
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
    }
    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "mul", operation: Mul;
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64), ("complex", crate::C64, "c64", c64),
    }
    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "pow", operation: Pow;
        ("u8", u8, "u8", u8), ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32), ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
    }
    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "sub", operation: Sub;
        ("i8", i8, "i8", i8), ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32), ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128), ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16), ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64), ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64), ("complex", crate::C64, "c64", c64),
    }
}

/// Installs every enabled concrete runtime factory owned by `mech-math`.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "add")]
    crate::ops::add::install_math_add_runtime(builder)?;
    #[cfg(feature = "add_assign")]
    install_add_assign_runtime(builder)?;
    #[cfg(feature = "div")]
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Div;
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64),
        ("complex", crate::C64, "c64", c64),
    )?;
    #[cfg(feature = "div_assign")]
    install_div_assign_runtime(builder)?;
    #[cfg(feature = "mod")]
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Mod;
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
    )?;
    #[cfg(feature = "mul_assign")]
    install_mul_assign_runtime(builder)?;
    #[cfg(feature = "mul")]
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Mul;
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64),
        ("complex", crate::C64, "c64", c64),
    )?;
    #[cfg(feature = "sub_assign")]
    install_sub_assign_runtime(builder)?;
    #[cfg(feature = "pow")]
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Pow;
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
    )?;
    #[cfg(all(feature = "pow", feature = "rational", feature = "i32"))]
    register_pow_rational(builder)?;
    #[cfg(feature = "sub")]
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Sub;
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
        ("rational", crate::R64, "r64", r64),
        ("complex", crate::C64, "c64", c64),
    )?;

    #[cfg(feature = "neg")]
    install_math_neg!(builder);

    #[cfg(feature = "abs")]
    install_math_abs!(builder);

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

/// Installs the frozen runtime plus compiler-emitted representation bridges.
#[cfg(feature = "native-plan")]
pub fn install_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    #[cfg(all(
        feature = "add",
        feature = "matrixd",
        any(feature = "matrix1", feature = "matrix1_interop")
    ))]
    crate::ops::add::install_math_add_native_plan(builder)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{FunctionCatalog, OperationId, RuntimeFunctionId};
    use std::collections::BTreeSet;

    #[cfg(all(
        feature = "native-plan",
        feature = "abs",
        feature = "complex",
        feature = "rational"
    ))]
    #[test]
    fn complex_and_rational_linkage_features_follow_factory_signatures() {
        let mut builder = FunctionCatalogBuilder::new();
        register_math_abs_c64_s(&mut builder).unwrap();
        register_math_abs_r64_s(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        for (name, feature) in [("MathAbsC64S", "c64"), ("MathAbsR64S", "r64")] {
            let entry = catalog
                .runtime_entry(RuntimeFunctionId::from_name(name))
                .unwrap_or_else(|| panic!("missing runtime factory {name}"));
            let linkage = entry
                .native_linkage
                .as_ref()
                .unwrap_or_else(|| panic!("missing native linkage for {name}"));
            assert!(
                linkage.cargo_features.contains(&feature),
                "{name} did not derive {feature}: {:?}",
                linkage.cargo_features,
            );
        }
    }

    #[cfg(all(feature = "source", feature = "math_default"))]
    const EXPECTED_NAMES: [&str; 65] = [
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
        "math/fdim",
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

    #[cfg(all(feature = "source", feature = "math_default"))]
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

    #[cfg(all(feature = "source", feature = "math_default"))]
    fn catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        builder.build().unwrap()
    }

    #[cfg(all(feature = "source", feature = "math_default"))]
    #[test]
    fn source_catalog_matches_the_supported_math_surface() {
        let catalog = catalog();
        let actual = catalog
            .specializer_entries()
            .map(|entry| entry.canonical_name.as_str())
            .collect::<BTreeSet<_>>();
        let expected = EXPECTED_NAMES.into_iter().collect::<BTreeSet<_>>();

        assert_eq!(actual, expected);
        assert_eq!(catalog.specializer_count(), 65);

        for excluded in [
            "math/exp",
            "math/exp2",
            "math/exp10",
            "math/expm1",
            "math/mul-assign",
        ] {
            assert!(
                catalog
                    .specializer(OperationId::from_name(excluded))
                    .is_none()
            );
        }

        let mul_assign = OperationId::from_name("math/mul-assign");
        assert!(catalog.specializer(mul_assign).is_none());
        assert!(catalog.intrinsic_specializer(mul_assign).is_some());
        assert!(catalog.exports_for_operation(mul_assign).is_empty());
    }

    #[cfg(all(feature = "source", feature = "math_default"))]
    #[test]
    fn source_catalog_preserves_prelude_and_module_exposure() {
        let catalog = catalog();
        let prelude = PRELUDE_NAMES.into_iter().collect::<BTreeSet<_>>();

        for name in EXPECTED_NAMES {
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

    #[test]
    fn runtime_catalog_entries_have_unique_canonical_names_and_ids() {
        std::thread::Builder::new()
            .name("math-runtime-catalog-uniqueness".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let mut builder = FunctionCatalogBuilder::new();
                install_runtime(&mut builder).unwrap();
                let catalog = builder.build().unwrap();

                let mut names = BTreeSet::new();
                for entry in catalog.runtime_entries() {
                    assert_eq!(
                        entry.id,
                        RuntimeFunctionId::from_name(&entry.name),
                        "runtime ID mismatch for {}",
                        entry.name,
                    );
                    assert!(
                        names.insert(entry.name.as_str()),
                        "duplicate runtime factory {}",
                        entry.name,
                    );
                }

                assert_eq!(names.len(), catalog.runtime_factory_count());
            })
            .expect("math runtime catalog uniqueness thread must spawn")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    }
}
