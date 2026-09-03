use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure};
#[cfg(feature = "source")]
use std::sync::Arc;
#[cfg(all(feature = "source", feature = "n_choose_k"))]
use crate::CombinatoricsNChooseK;

macro_rules! for_each_combinatorics_scalar {
    ($callback:ident, $($context:tt)*) => {
        $callback!($($context)*; feature = "u8"; u8; "u8"; u8);
        $callback!($($context)*; feature = "i8"; i8; "i8"; i8);
        $callback!($($context)*; feature = "u16"; u16; "u16"; u16);
        $callback!($($context)*; feature = "i16"; i16; "i16"; i16);
        $callback!($($context)*; feature = "u32"; u32; "u32"; u32);
        $callback!($($context)*; feature = "i32"; i32; "i32"; i32);
        $callback!($($context)*; feature = "u64"; u64; "u64"; u64);
        $callback!($($context)*; feature = "i64"; i64; "i64"; i64);
        $callback!($($context)*; feature = "u128"; u128; "u128"; u128);
        $callback!($($context)*; feature = "i128"; i128; "i128"; i128);
        $callback!($($context)*; feature = "f32"; f32; "f32"; f32);
        $callback!($($context)*; feature = "f64"; f64; "f64"; f64);
        $callback!($($context)*; feature = "r64"; mech_core::R64; "r64"; r64);
        $callback!($($context)*; feature = "c64"; mech_core::C64; "c64"; c64);
    };
}

macro_rules! declare_n_choose_k_scalar {
    (; $cfg:meta; $scalar:ty; $scalar_name:literal; $scalar_token:ident) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: $cfg,
                registration: [<register_n_choose_k_ $scalar_token>],
                installer: [<install_n_choose_k_ $scalar_token>],
                name: concat!("NChooseK<", $scalar_name, ">"),
                factory_type: crate::n_choose_k::NChooseK<$scalar>,
                contract: mech_core::RuntimeFunctionContract::canonical_custom(
                    "n_choose_k_scalar",
                    mech_core::RuntimeOutputAliasPolicy::DisallowInputAlias,
                    crate::n_choose_k::validate_canonical_n_choose_k_scalar_contract,
                ),
                package: "mech-combinatorics",
                crate_name: "mech_combinatorics",
                installer_path: concat!("mech_combinatorics::__mech_native::", stringify!([<install_n_choose_k_ $scalar_token>])),
                extra_cargo_features: ["n_choose_k"],
            }
        }
    };
}

macro_rules! declare_n_choose_k_matrix {
    (; $cfg:meta; $scalar:ty; $scalar_name:literal; $scalar_token:ident) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "matrix", feature = "matrixd", $cfg),
                registration: [<register_n_choose_k_matrix_ $scalar_token>],
                installer: [<install_n_choose_k_matrix_ $scalar_token>],
                name: concat!("NChooseKMatrix<", $scalar_name, ">"),
                factory_type: crate::n_choose_k::NChooseKMatrix<$scalar>,
                contract: mech_core::RuntimeFunctionContract::canonical_custom(
                    "n_choose_k_matrix",
                    mech_core::RuntimeOutputAliasPolicy::DisallowInputAlias,
                    crate::n_choose_k::validate_canonical_n_choose_k_matrix_contract,
                ),
                package: "mech-combinatorics",
                crate_name: "mech_combinatorics",
                installer_path: concat!("mech_combinatorics::__mech_native::", stringify!([<install_n_choose_k_matrix_ $scalar_token>])),
                extra_cargo_features: ["n_choose_k"],
            }
        }
    };
}

for_each_combinatorics_scalar!(declare_n_choose_k_scalar,);
for_each_combinatorics_scalar!(declare_n_choose_k_matrix,);

/// Installs the frozen named source-specializer surface for the combinatorics machine.
#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "n_choose_k")]
    {
        let canonical_name = "combinatorics/n-choose-k";
        let declaration = mech_core::maintained_source_type_declaration(canonical_name)?;
        let operation = builder.insert_canonical_specializer(
            canonical_name,
            declaration,
            Arc::new(CombinatoricsNChooseK {}),
        )?;
        builder.insert_export(FunctionExport {
            operation,
            canonical_name: canonical_name.to_string(),
            module: Some("combinatorics".to_string()),
            item: Some("n-choose-k".to_string()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}

/// Installs the concrete scalar and matrix n-choose-k runtime factories.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register_n_choose_k_scalar {
        (; $cfg:meta; $_scalar:ty; $_scalar_name:literal; $scalar_token:ident) => {
            #[cfg($cfg)]
            mech_core::paste::paste! { [<register_n_choose_k_ $scalar_token>](builder)?; }
        };
    }
    #[cfg(all(feature = "n_choose_k", feature = "matrixd"))]
    macro_rules! register_n_choose_k_matrix {
        (; $cfg:meta; $_scalar:ty; $_scalar_name:literal; $scalar_token:ident) => {
            #[cfg(all(feature = "matrix", feature = "matrixd", $cfg))]
            mech_core::paste::paste! { [<register_n_choose_k_matrix_ $scalar_token>](builder)?; }
        };
    }

    #[cfg(feature = "n_choose_k")]
    for_each_combinatorics_scalar!(register_n_choose_k_scalar,);
    #[cfg(all(feature = "n_choose_k", feature = "matrix", feature = "matrixd"))]
    for_each_combinatorics_scalar!(register_n_choose_k_matrix,);
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_n_choose_k_scalar {
        (; $cfg:meta; $_scalar:ty; $_scalar_name:literal; $scalar_token:ident) => {
            #[cfg($cfg)]
            mech_core::paste::paste! { pub use super::[<install_n_choose_k_ $scalar_token>]; }
        };
    }
    #[cfg(all(feature = "n_choose_k", feature = "matrixd"))]
    macro_rules! export_n_choose_k_matrix {
        (; $cfg:meta; $_scalar:ty; $_scalar_name:literal; $scalar_token:ident) => {
            #[cfg(all(feature = "matrix", feature = "matrixd", $cfg))]
            mech_core::paste::paste! { pub use super::[<install_n_choose_k_matrix_ $scalar_token>]; }
        };
    }

    #[cfg(feature = "n_choose_k")]
    for_each_combinatorics_scalar!(export_n_choose_k_scalar,);
    #[cfg(all(feature = "n_choose_k", feature = "matrix", feature = "matrixd"))]
    for_each_combinatorics_scalar!(export_n_choose_k_matrix,);
}

#[cfg(all(test, feature = "source", feature = "n_choose_k"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    #[test]
    fn n_choose_k_is_the_only_module_only_source_operation() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let operation = OperationId::from_name("combinatorics/n-choose-k");

        assert_eq!(catalog.specializer_count(), 1);
        let export = catalog
            .module_export("combinatorics", "n-choose-k")
            .unwrap();
        assert_eq!(export.operation, operation);
        assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
        assert_eq!(catalog.exports_for_operation(operation), [export.clone()]);
    }
}
