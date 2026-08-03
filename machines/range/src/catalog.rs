use mech_core::{FunctionCatalogBuilder, MResult, MechFunctionFactory};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(all(not(feature = "matrix1"), feature = "matrixd"))]
use nalgebra::DMatrix;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;

#[cfg(feature = "source")]
fn install_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
    exposure: FunctionExposure,
) -> MResult<()>
where
    T: FunctionSpecializer + 'static,
{
    let operation = builder.insert_specializer(canonical_name, Arc::new(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: None,
        item: None,
        exposure,
    })
}

#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "exclusive")]
    install_operation(
        builder,
        "range/exclusive",
        crate::exclusive::RangeExclusive {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "exclusive_increment")]
    install_operation(
        builder,
        "range/exclusive-increment",
        crate::exclusive_increment::RangeIncrementExclusive {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "inclusive")]
    install_operation(
        builder,
        "range/inclusive",
        crate::inclusive::RangeInclusive {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "inclusive_increment")]
    install_operation(
        builder,
        "range/inclusive-increment",
        crate::inclusive_increment::RangeIncrementInclusive {},
        FunctionExposure::Internal,
    )?;
    Ok(())
}

macro_rules! install_range_factory {
    ($builder:expr, $module:ident, $factory:ident, $scalar:ty, $scalar_name:literal, $shape:ident) => {
        $builder.insert_runtime_factory(
            concat!(
                stringify!($factory),
                "<",
                $scalar_name,
                stringify!($shape),
                ">"
            ),
            <crate::$module::$factory<$scalar, $shape<$scalar>> as MechFunctionFactory>::new,
        )?;
    };
}

macro_rules! install_range_factories_for_type {
    ($builder:expr, $module:ident, $factory:ident, $scalar:ty, $scalar_name:literal) => {{
        #[cfg(feature = "matrix1")]
        install_range_factory!($builder, $module, $factory, $scalar, $scalar_name, Matrix1);
        #[cfg(all(not(feature = "matrix1"), feature = "matrixd"))]
        install_range_factory!($builder, $module, $factory, $scalar, $scalar_name, DMatrix);
        #[cfg(feature = "row_vector2")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowVector2
        );
        #[cfg(feature = "row_vector3")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowVector3
        );
        #[cfg(feature = "row_vector4")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowVector4
        );
        #[cfg(feature = "row_vectord")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowDVector
        );
    }};
}

macro_rules! install_range_operation_runtime {
    ($builder:expr, $module:ident, $factory:ident) => {{
        #[cfg(feature = "f32")]
        install_range_factories_for_type!($builder, $module, $factory, f32, "f32");
        #[cfg(feature = "f64")]
        install_range_factories_for_type!($builder, $module, $factory, f64, "f64");
        #[cfg(feature = "i8")]
        install_range_factories_for_type!($builder, $module, $factory, i8, "i8");
        #[cfg(feature = "i16")]
        install_range_factories_for_type!($builder, $module, $factory, i16, "i16");
        #[cfg(feature = "i32")]
        install_range_factories_for_type!($builder, $module, $factory, i32, "i32");
        #[cfg(feature = "i64")]
        install_range_factories_for_type!($builder, $module, $factory, i64, "i64");
        #[cfg(feature = "i128")]
        install_range_factories_for_type!($builder, $module, $factory, i128, "i128");
        #[cfg(feature = "u8")]
        install_range_factories_for_type!($builder, $module, $factory, u8, "u8");
        #[cfg(feature = "u16")]
        install_range_factories_for_type!($builder, $module, $factory, u16, "u16");
        #[cfg(feature = "u32")]
        install_range_factories_for_type!($builder, $module, $factory, u32, "u32");
        #[cfg(feature = "u64")]
        install_range_factories_for_type!($builder, $module, $factory, u64, "u64");
        #[cfg(feature = "u128")]
        install_range_factories_for_type!($builder, $module, $factory, u128, "u128");
    }};
}

/// Legacy direct-registration implementation retained while the native
/// declaration traversal below owns the active runtime path.
fn install_legacy_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "exclusive")]
    {
        install_range_operation_runtime!(builder, exclusive, RangeExclusiveScalar);
        // The legacy module gate compiles increment factories with `exclusive`,
        // even when the named source operation is not exported.
        install_range_operation_runtime!(
            builder,
            exclusive_increment,
            RangeIncrementExclusiveScalar
        );
    }
    #[cfg(feature = "inclusive")]
    {
        install_range_operation_runtime!(builder, inclusive, RangeInclusiveScalar);
        // Preserve the matching legacy module-gate quirk for parity.
        install_range_operation_runtime!(
            builder,
            inclusive_increment,
            RangeIncrementInclusiveScalar
        );
    }
    Ok(())
}

macro_rules! for_each_range_scalar {
    ($callback:ident, $context:tt; [$cfg:meta]; $module:ident; $factory:ident; $operation_feature:literal; $shape:ident; [$($shape_feature:literal),* $(,)?]) => {
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "f32"; "f32"; f32; f32);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "f64"; "f64"; f64; f64);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i8"; "i8"; i8; i8);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i16"; "i16"; i16; i16);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i32"; "i32"; i32; i32);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i64"; "i64"; i64; i64);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i128"; "i128"; i128; i128);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u8"; "u8"; u8; u8);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u16"; "u16"; u16; u16);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u32"; "u32"; u32; u32);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u64"; "u64"; u64; u64);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u128"; "u128"; u128; u128);
    };
}

macro_rules! for_each_range_shape {
    ($callback:ident, $context:tt; $module:ident; $factory:ident; $operation_feature:literal) => {
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "matrix1")]; $module; $factory; $operation_feature; Matrix1; ["matrix1"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, not(feature = "matrix1"), feature = "matrixd")]; $module; $factory; $operation_feature; DMatrix; ["matrixd"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vector2")]; $module; $factory; $operation_feature; RowVector2; ["row_vector2"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vector3")]; $module; $factory; $operation_feature; RowVector3; ["row_vector3"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vector4")]; $module; $factory; $operation_feature; RowVector4; ["row_vector4"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vectord")]; $module; $factory; $operation_feature; RowDVector; ["row_vectord"]);
    };
}

macro_rules! for_each_range_family_with_context {
    ($callback:ident, $context:tt) => {
        for_each_range_shape!($callback, $context; exclusive; RangeExclusiveScalar; "exclusive");
        for_each_range_shape!($callback, $context; exclusive_increment; RangeIncrementExclusiveScalar; "exclusive");
        for_each_range_shape!($callback, $context; inclusive; RangeInclusiveScalar; "inclusive");
        for_each_range_shape!($callback, $context; inclusive_increment; RangeIncrementInclusiveScalar; "inclusive");
    };
}

macro_rules! for_each_range_family {
    ($callback:ident) => { for_each_range_family_with_context!($callback, ()); };
    ($callback:ident, $context:tt) => { for_each_range_family_with_context!($callback, $context); };
}

macro_rules! declare_range_runtime_factory {
    ($_context:tt; [$cfg:meta]; $module:ident; $factory:ident; $operation_feature:literal; $shape:ident; [$($shape_feature:literal),* $(,)?]; $scalar_cfg:meta; $scalar_feature:literal; $scalar:ty; $scalar_token:ident) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all($cfg, $scalar_cfg),
                registration: [<register_ $factory:snake _ $scalar_token _ $shape:snake>],
                installer: [<install_ $factory:snake _ $scalar_token _ $shape:snake>],
                name: concat!(stringify!($factory), "<", $scalar_feature, stringify!($shape), ">"),
                factory: <crate::$module::$factory<$scalar, $shape<$scalar>> as MechFunctionFactory>::new,
                package: "mech-range", crate_name: "mech_range",
                installer_path: concat!("mech_range::__mech_native::", stringify!([<install_ $factory:snake _ $scalar_token _ $shape:snake>])),
                cargo_features: [$operation_feature, $scalar_feature, $($shape_feature,)* "native-link", "runtime"],
            }
        }
    };
}

for_each_range_family!(declare_range_runtime_factory);

/// Installs every concrete runtime factory declared by the range family traversal.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register_range_runtime_factory {
        (($builder:ident); [$cfg:meta]; $_module:ident; $factory:ident; $_operation_feature:literal; $shape:ident; [$($_shape_feature:literal),* $(,)?]; $scalar_cfg:meta; $_scalar_feature:literal; $_scalar:ty; $scalar_token:ident) => {
            #[cfg(all($cfg, $scalar_cfg))]
            mech_core::paste::paste! { [<register_ $factory:snake _ $scalar_token _ $shape:snake>]($builder)?; }
        };
    }
    for_each_range_family!(register_range_runtime_factory, (builder));
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_range_runtime_factory {
        ($_context:tt; [$cfg:meta]; $_module:ident; $factory:ident; $_operation_feature:literal; $shape:ident; [$($_shape_feature:literal),* $(,)?]; $scalar_cfg:meta; $_scalar_feature:literal; $_scalar:ty; $scalar_token:ident) => {
            #[cfg(all($cfg, $scalar_cfg))]
            mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $scalar_token _ $shape:snake>]; }
        };
    }
    for_each_range_family!(export_range_runtime_factory);
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    fn expected_operations() -> Vec<(&'static str, FunctionExposure)> {
        let mut expected = Vec::new();
        #[cfg(feature = "exclusive")]
        expected.push(("range/exclusive", FunctionExposure::Prelude));
        #[cfg(feature = "exclusive_increment")]
        expected.push(("range/exclusive-increment", FunctionExposure::Internal));
        #[cfg(feature = "inclusive")]
        expected.push(("range/inclusive", FunctionExposure::Prelude));
        #[cfg(feature = "inclusive_increment")]
        expected.push(("range/inclusive-increment", FunctionExposure::Internal));
        expected
    }

    #[test]
    fn source_catalog_matches_the_frozen_range_surface() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let expected = expected_operations();

        #[cfg(all(
            feature = "exclusive",
            feature = "exclusive_increment",
            feature = "inclusive",
            feature = "inclusive_increment",
        ))]
        assert_eq!(expected.len(), 4);
        assert_eq!(catalog.specializer_count(), expected.len());
        assert_eq!(catalog.runtime_factory_count(), 0);
        for (name, exposure) in expected {
            let operation = OperationId::from_name(name);
            assert_eq!(catalog.specializer(operation).unwrap().canonical_name, name);
            assert_eq!(
                catalog.exports_for_operation(operation),
                &[FunctionExport {
                    operation,
                    canonical_name: name.to_string(),
                    module: None,
                    item: None,
                    exposure,
                }],
            );
        }
    }
}
