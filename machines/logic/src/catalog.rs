use mech_core::{
    FunctionCatalogBuilder, MResult, RuntimeFunctionContract, RuntimeOutputAliasPolicy,
};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
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
                factory_type: crate::$module::[<$operation $suffix>],
                contract: mech_core::__mech_elementwise_binop_contract!($suffix),
                package: "mech-logic",
                crate_name: "mech_logic",
                installer_path: concat!(
                    "mech_logic::__mech_native::install_logic_",
                    stringify!([<$operation:lower>]), "_", stringify!([<$suffix:lower>]),
                ),
                extra_cargo_features: [$operation_feature],
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
                factory_type: crate::$module::[<$operation $suffix>],
                contract: mech_core::__mech_elementwise_binop_contract!($suffix),
                package: "mech-logic",
                crate_name: "mech_logic",
                installer_path: concat!(
                    "mech_logic::__mech_native::install_logic_",
                    stringify!([<$operation:lower>]), "_", stringify!([<$suffix:lower>]),
                ),
                extra_cargo_features: [$operation_feature],
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

#[cfg(feature = "native-link")]
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
    factory_type: crate::not::NotS<bool>,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-logic",
    crate_name: "mech_logic",
    installer_path: "mech_logic::__mech_native::install_logic_not_s",
    extra_cargo_features: ["not"],
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
