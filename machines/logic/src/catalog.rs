use mech_core::{FunctionCatalogBuilder, MResult, MechFunctionFactory};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
use paste::paste;
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(feature = "source")]
fn install_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
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
        exposure: FunctionExposure::Prelude,
    })
}

#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "and")]
    install_operation(builder, "logic/and", crate::LogicAnd {})?;
    #[cfg(feature = "not")]
    install_operation(builder, "logic/not", crate::LogicNot {})?;
    #[cfg(feature = "or")]
    install_operation(builder, "logic/or", crate::LogicOr {})?;
    #[cfg(feature = "xor")]
    install_operation(builder, "logic/xor", crate::LogicXor {})?;
    Ok(())
}

macro_rules! install_logic_factory {
    ($builder:expr, $module:ident, $operation:ident, $suffix:ident) => {
        paste! {
            $builder.insert_runtime_factory(
                concat!(stringify!($operation), stringify!($suffix), "<bool>"),
                <crate::$module::[<$operation $suffix>] as MechFunctionFactory>::new,
            )?;
        }
    };
}

macro_rules! install_logic_group {
    ($builder:expr, $module:ident, $operation:ident; $($suffix:ident),+ $(,)?) => {
        $(install_logic_factory!($builder, $module, $operation, $suffix);)+
    };
}

macro_rules! install_logic_binop_runtime {
    ($builder:expr, $module:ident, $operation:ident) => {{
        install_logic_group!($builder, $module, $operation; SS);

        #[cfg(feature = "matrix1")]
        install_logic_group!($builder, $module, $operation; SM1, M1S, M1M1);
        #[cfg(feature = "matrix2")]
        install_logic_group!($builder, $module, $operation; SM2, M2S, M2M2);
        #[cfg(feature = "matrix3")]
        install_logic_group!($builder, $module, $operation; SM3, M3S, M3M3);
        #[cfg(feature = "matrix4")]
        install_logic_group!($builder, $module, $operation; SM4, M4S, M4M4);
        #[cfg(feature = "matrix2x3")]
        install_logic_group!($builder, $module, $operation; SM2x3, M2x3S, M2x3M2x3);
        #[cfg(feature = "matrix3x2")]
        install_logic_group!($builder, $module, $operation; SM3x2, M3x2S, M3x2M3x2);
        #[cfg(feature = "matrixd")]
        install_logic_group!($builder, $module, $operation; SMD, MDS, MDMD);

        #[cfg(feature = "row_vector2")]
        install_logic_group!($builder, $module, $operation; SR2, R2S, R2R2);
        #[cfg(feature = "row_vector3")]
        install_logic_group!($builder, $module, $operation; SR3, R3S, R3R3);
        #[cfg(feature = "row_vector4")]
        install_logic_group!($builder, $module, $operation; SR4, R4S, R4R4);
        #[cfg(feature = "row_vectord")]
        install_logic_group!($builder, $module, $operation; SRD, RDS, RDRD);

        #[cfg(feature = "vector2")]
        install_logic_group!($builder, $module, $operation; SV2, V2S, V2V2);
        #[cfg(feature = "vector3")]
        install_logic_group!($builder, $module, $operation; SV3, V3S, V3V3);
        #[cfg(feature = "vector4")]
        install_logic_group!($builder, $module, $operation; SV4, V4S, V4V4);
        #[cfg(feature = "vectord")]
        install_logic_group!($builder, $module, $operation; SVD, VDS, VDVD);

        #[cfg(all(feature = "matrix2", feature = "vector2"))]
        install_logic_group!($builder, $module, $operation; M2V2, V2M2);
        #[cfg(all(feature = "matrix3", feature = "vector3"))]
        install_logic_group!($builder, $module, $operation; M3V3, V3M3);
        #[cfg(all(feature = "matrix4", feature = "vector4"))]
        install_logic_group!($builder, $module, $operation; M4V4, V4M4);
        #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
        install_logic_group!($builder, $module, $operation; M2x3V2, V2M2x3);
        #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
        install_logic_group!($builder, $module, $operation; M3x2V3, V3M3x2);
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        install_logic_group!($builder, $module, $operation; MDVD, VDMD);
        #[cfg(all(feature = "matrixd", feature = "vector2"))]
        install_logic_group!($builder, $module, $operation; MDV2, V2MD);
        #[cfg(all(feature = "matrixd", feature = "vector3"))]
        install_logic_group!($builder, $module, $operation; MDV3, V3MD);
        #[cfg(all(feature = "matrixd", feature = "vector4"))]
        install_logic_group!($builder, $module, $operation; MDV4, V4MD);

        #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
        install_logic_group!($builder, $module, $operation; M2R2, R2M2);
        #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
        install_logic_group!($builder, $module, $operation; M3R3, R3M3);
        #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
        install_logic_group!($builder, $module, $operation; M4R4, R4M4);
        #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
        install_logic_group!($builder, $module, $operation; M2x3R3, R3M2x3);
        #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
        install_logic_group!($builder, $module, $operation; M3x2R2, R2M3x2);
        #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
        install_logic_group!($builder, $module, $operation; MDRD, RDMD);
        #[cfg(all(feature = "matrixd", feature = "row_vector2"))]
        install_logic_group!($builder, $module, $operation; MDR2, R2MD);
        #[cfg(all(feature = "matrixd", feature = "row_vector3"))]
        install_logic_group!($builder, $module, $operation; MDR3, R3MD);
        #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
        install_logic_group!($builder, $module, $operation; MDR4, R4MD);
    }};
}

macro_rules! declare_logic_native_factory {
    (
        ($module:ident; $operation:ident; $operation_feature:literal),
        $_lib:ident, $suffix:ident, none,
        $_scalar:ty, $_scalar_name:literal, $_scalar_token:ident
    ) => {
        paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = "bool"),
                registration: [<register_logic_ $operation:lower _ $suffix:lower>],
                installer: [<install_logic_ $operation:lower _ $suffix:lower>],
                name: concat!(stringify!($operation), stringify!($suffix), "<bool>"),
                factory: <crate::$module::[<$operation $suffix>] as MechFunctionFactory>::new,
                package: "mech-logic",
                crate_name: "mech_logic",
                installer_path: concat!(
                    "mech_logic::__mech_native::install_logic_",
                    stringify!([<$operation:lower>]), "_", stringify!([<$suffix:lower>]),
                ),
                cargo_features: ["bool", $operation_feature, "native-link", "runtime"],
            }
        }
    };
    (
        ($module:ident; $operation:ident; $operation_feature:literal),
        $_lib:ident, $suffix:ident, $shape_feature:literal,
        $_scalar:ty, $_scalar_name:literal, $_scalar_token:ident
    ) => {
        paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = $operation_feature, feature = "bool"),
                registration: [<register_logic_ $operation:lower _ $suffix:lower>],
                installer: [<install_logic_ $operation:lower _ $suffix:lower>],
                name: concat!(stringify!($operation), stringify!($suffix), "<bool>"),
                factory: <crate::$module::[<$operation $suffix>] as MechFunctionFactory>::new,
                package: "mech-logic",
                crate_name: "mech_logic",
                installer_path: concat!(
                    "mech_logic::__mech_native::install_logic_",
                    stringify!([<$operation:lower>]), "_", stringify!([<$suffix:lower>]),
                ),
                cargo_features: ["bool", $operation_feature, $shape_feature, "native-link", "runtime"],
            }
        }
    };
}

macro_rules! register_logic_native_factory {
    (
        ($builder:ident; $_module:ident; $operation:ident; $_operation_feature:literal),
        $_lib:ident, $suffix:ident, $_shape_feature:tt,
        $_scalar:ty, $_scalar_name:literal, $_scalar_token:ident
    ) => {
        paste::paste! { [<register_logic_ $operation:lower _ $suffix:lower>]($builder)?; }
    };
}

macro_rules! export_logic_native_factory {
    (
        ($_module:ident; $operation:ident; $operation_feature:literal),
        $_lib:ident, $suffix:ident, $_shape_feature:tt,
        $_scalar:ty, $_scalar_name:literal, $_scalar_token:ident
    ) => {
        #[cfg(all(feature = $operation_feature, feature = "bool"))]
        paste::paste! { pub use super::[<install_logic_ $operation:lower _ $suffix:lower>]; }
    };
}

macro_rules! declare_logic_binop_runtime {
    ($module:ident, $operation:ident, $operation_feature:literal) => {
        mech_core::__mech_for_each_binop_runtime_factory_for_type!(
            declare_logic_native_factory,
            ($module; $operation; $operation_feature),
            $operation,
            bool,
            "bool",
            bool
        );
    };
}

declare_logic_binop_runtime!(and, And, "and");
declare_logic_binop_runtime!(or, Or, "or");
declare_logic_binop_runtime!(xor, Xor, "xor");

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "not", feature = "bool"),
    registration: register_logic_not_s,
    installer: install_logic_not_s,
    name: "NotS<bool>",
    factory: <crate::not::NotS<bool> as MechFunctionFactory>::new,
    package: "mech-logic",
    crate_name: "mech_logic",
    installer_path: "mech_logic::__mech_native::install_logic_not_s",
    cargo_features: ["bool", "native-link", "not", "runtime"],
}

macro_rules! install_native_logic_binop_runtime {
    ($builder:ident, $module:ident, $operation:ident, $operation_feature:literal) => {
        mech_core::__mech_for_each_binop_runtime_factory_for_type!(
            register_logic_native_factory,
            ($builder; $module; $operation; $operation_feature),
            $operation,
            bool,
            "bool",
            bool
        );
    };
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_logic_binop_runtime {
        ($module:ident, $operation:ident, $operation_feature:literal) => {
            mech_core::__mech_for_each_binop_runtime_factory_for_type!(
                export_logic_native_factory,
                ($module; $operation; $operation_feature),
                $operation,
                bool,
                "bool",
                bool
            );
        };
    }
    export_logic_binop_runtime!(and, And, "and");
    export_logic_binop_runtime!(or, Or, "or");
    export_logic_binop_runtime!(xor, Xor, "xor");
    #[cfg(all(feature = "not", feature = "bool"))]
    pub use super::install_logic_not_s;
}

/// Installs every enabled concrete bytecode factory owned by `mech-logic`.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "and")]
    install_native_logic_binop_runtime!(builder, and, And, "and");
    #[cfg(feature = "or")]
    install_native_logic_binop_runtime!(builder, or, Or, "or");
    #[cfg(feature = "xor")]
    install_native_logic_binop_runtime!(builder, xor, Xor, "xor");
    #[cfg(all(feature = "not", feature = "bool"))]
    register_logic_not_s(builder)?;
    Ok(())
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    fn expected_operations() -> Vec<&'static str> {
        let mut expected = Vec::new();
        #[cfg(feature = "and")]
        expected.push("logic/and");
        #[cfg(feature = "not")]
        expected.push("logic/not");
        #[cfg(feature = "or")]
        expected.push("logic/or");
        #[cfg(feature = "xor")]
        expected.push("logic/xor");
        expected
    }

    #[test]
    fn source_catalog_matches_the_frozen_logic_surface() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let expected = expected_operations();

        #[cfg(all(feature = "and", feature = "not", feature = "or", feature = "xor"))]
        assert_eq!(expected.len(), 4);
        assert_eq!(catalog.specializer_count(), expected.len());
        assert_eq!(catalog.runtime_factory_count(), 0);
        for name in expected {
            let operation = OperationId::from_name(name);
            assert_eq!(catalog.specializer(operation).unwrap().canonical_name, name);
            assert_eq!(
                catalog.exports_for_operation(operation),
                &[FunctionExport {
                    operation,
                    canonical_name: name.to_string(),
                    module: None,
                    item: None,
                    exposure: FunctionExposure::Prelude,
                }],
            );
        }
    }
}
