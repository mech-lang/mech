#[cfg(any(feature = "standard", feature = "representatives"))]
use mech_core::FunctionCatalog;
#[cfg(feature = "representatives")]
use mech_core::FunctionCatalogBuilder;
#[cfg(any(feature = "standard", feature = "representatives"))]
use serde::Serialize;

#[cfg(any(feature = "standard", feature = "representatives"))]
#[derive(Debug, Serialize)]
struct LinkageEntry<'a> {
    name: &'a str,
    id_hex: String,
    package: Option<&'static str>,
    crate_name: Option<&'static str>,
    installer_path: Option<&'static str>,
    cargo_features: Option<&'static [&'static str]>,
}

#[cfg(any(feature = "standard", feature = "representatives"))]
fn emit(catalog: &FunctionCatalog) {
    let entries = catalog
        .runtime_entries()
        .map(|entry| {
            let linkage = entry.native_linkage.as_ref();
            LinkageEntry {
                name: &entry.name,
                id_hex: format!("{:016x}", entry.id.raw()),
                package: linkage.map(|value| value.package),
                crate_name: linkage.map(|value| value.crate_name),
                installer_path: linkage.map(|value| value.installer_path),
                cargo_features: linkage.map(|value| value.cargo_features),
            }
        })
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout(), &entries).unwrap();
}

#[cfg(all(feature = "standard", not(feature = "representatives")))]
fn main() {
    emit(&mech_stdlib::native_plan_catalog());
}

#[cfg(all(feature = "representatives", not(feature = "standard")))]
fn main() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_math::install_runtime(&mut builder).unwrap();
    emit(&builder.build().unwrap());
}

#[cfg(all(
    feature = "installer-profile",
    not(feature = "standard"),
    not(feature = "representatives")
))]
fn main() {
    assert!(generated_catalog().runtime_factory_count() >= 1);
}

#[cfg(all(
    not(feature = "standard"),
    not(feature = "representatives"),
    not(feature = "installer-profile"),
    not(feature = "owner-native-link")
))]
fn main() {
    panic!("enable exactly one of `standard`, `representatives`, or `installers`");
}

#[cfg(all(
    feature = "owner-native-link",
    not(feature = "standard"),
    not(feature = "representatives"),
    not(feature = "installer-profile")
))]
fn main() {}

#[cfg(any(
    all(feature = "standard", feature = "representatives"),
    all(feature = "standard", feature = "installer-profile"),
    all(feature = "representatives", feature = "installer-profile"),
    all(feature = "standard", feature = "owner-native-link"),
    all(feature = "representatives", feature = "owner-native-link"),
    all(feature = "installer-profile", feature = "owner-native-link")
))]
compile_error!("native-linkage-fixture features are mutually exclusive");

#[cfg(feature = "installer-profile")]
fn generated_catalog() -> mech_core::FunctionCatalog {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    #[cfg(feature = "installer-variable-define-f64")]
    mech_engine::__mech_native::install_variable_define_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-add-ss-f64")]
    mech_math::__mech_native::install_add_ss_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-add-m2m2-f64")]
    mech_math::__mech_native::install_add_m2m2_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-add-mdmd-f64")]
    mech_math::__mech_native::install_add_mdmd_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-horizontal-concatenate-rdn-f64")]
    mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-horizontal-concatenate-s2-f64")]
    mech_engine::__mech_native::install_horizontal_concatenate_s2_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-vertical-concatenate-n-args-f64")]
    mech_engine::__mech_native::install_vertical_concatenate_n_args_f64(&mut builder).unwrap();
    #[cfg(feature = "installer-vertical-concatenate-r2-r2-f64")]
    mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64(&mut builder).unwrap();
    builder.build().unwrap()
}

#[cfg(all(test, feature = "installers"))]
mod tests {
    use super::*;
    use mech_core::{FunctionCatalogBuilder, MResult, RuntimeFunctionId};

    const EXPECTED: [(&str, u64); 8] = [
        ("VariableDefineF64", 0x0023_b6ad_86b6_55e7),
        ("AddSS<f64>", 0x000a_2c77_6884_86f3),
        ("AddM2M2<f64>", 0x00eb_049b_7b90_a0d9),
        ("AddMDMD<f64>", 0x008f_a755_537d_c395),
        ("HorizontalConcatenateRDN<f64>", 0x006c_13ae_b8d2_1f6c),
        ("HorizontalConcatenateS2<f64>", 0x00c3_ae9e_fc75_d589),
        (
            "VerticalConcatenateNArgs<f64>",
            0x006e_5ef9_27b7_6ce2,
        ),
        (
            "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>",
            0x00d7_d040_6995_0a49,
        ),
    ];

    #[test]
    fn generated_catalog_uses_all_exact_installer_paths() {
        let catalog = generated_catalog();
        assert_eq!(catalog.runtime_factory_count(), EXPECTED.len());
        for (name, id) in EXPECTED {
            let runtime_id = RuntimeFunctionId::from_raw(id);
            let entry = catalog
                .runtime_entry(runtime_id)
                .unwrap_or_else(|| panic!("generated catalog omitted {name}"));
            assert_eq!(entry.name, name);
            assert_eq!(RuntimeFunctionId::from_name(name).raw(), id);
        }
    }

    #[test]
    fn every_exact_installer_inserts_once_and_rejects_duplicates() {
        let installers: [fn(&mut FunctionCatalogBuilder) -> MResult<()>; 8] = [
            mech_engine::__mech_native::install_variable_define_f64,
            mech_math::__mech_native::install_add_ss_f64,
            mech_math::__mech_native::install_add_m2m2_f64,
            mech_math::__mech_native::install_add_mdmd_f64,
            mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64,
            mech_engine::__mech_native::install_horizontal_concatenate_s2_f64,
            mech_engine::__mech_native::install_vertical_concatenate_n_args_f64,
            mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64,
        ];

        for (installer, (name, _)) in installers.into_iter().zip(EXPECTED) {
            let mut builder = FunctionCatalogBuilder::new();
            installer(&mut builder).unwrap();
            let error = installer(&mut builder).unwrap_err();
            assert_eq!(error.kind_name(), "FunctionCatalogDuplicateRuntimeFactory");
            let catalog = builder.build().unwrap();
            assert_eq!(catalog.runtime_factory_count(), 1, "installer for {name}");
        }
    }
}
