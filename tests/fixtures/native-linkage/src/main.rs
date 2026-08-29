#[cfg(any(
    feature = "full",
    feature = "representatives",
    feature = "extended-engine",
    feature = "extended-engine-shard",
    feature = "extended-engine-shard-convert",
    feature = "extended-math",
    feature = "extended-math-shard",
    feature = "extended-compare",
    feature = "extended-logic",
    feature = "extended-range",
    feature = "extended-matrix",
    feature = "extended-set",
    feature = "extended-string",
    feature = "extended-stats",
    feature = "extended-combinatorics",
    feature = "extended-engine-runtime",
    feature = "extended-math-runtime",
    feature = "extended-compare-runtime",
    feature = "extended-logic-runtime",
    feature = "extended-range-runtime",
    feature = "extended-matrix-runtime",
    feature = "extended-set-runtime",
    feature = "extended-string-runtime",
    feature = "extended-stats-runtime",
    feature = "extended-combinatorics-runtime",
))]
use mech_core::FunctionCatalog;
#[cfg(any(
    feature = "representatives",
    feature = "extended-engine",
    feature = "extended-engine-shard",
    feature = "extended-engine-shard-convert",
    feature = "extended-math",
    feature = "extended-math-shard",
    feature = "extended-compare",
    feature = "extended-logic",
    feature = "extended-range",
    feature = "extended-matrix",
    feature = "extended-set",
    feature = "extended-string",
    feature = "extended-stats",
    feature = "extended-combinatorics",
    feature = "extended-engine-runtime",
    feature = "extended-math-runtime",
    feature = "extended-compare-runtime",
    feature = "extended-logic-runtime",
    feature = "extended-range-runtime",
    feature = "extended-matrix-runtime",
    feature = "extended-set-runtime",
    feature = "extended-string-runtime",
    feature = "extended-stats-runtime",
    feature = "extended-combinatorics-runtime",
))]
use mech_core::FunctionCatalogBuilder;
#[cfg(any(
    feature = "full",
    feature = "representatives",
    feature = "extended-engine",
    feature = "extended-engine-shard",
    feature = "extended-engine-shard-convert",
    feature = "extended-math",
    feature = "extended-math-shard",
    feature = "extended-compare",
    feature = "extended-logic",
    feature = "extended-range",
    feature = "extended-matrix",
    feature = "extended-set",
    feature = "extended-string",
    feature = "extended-stats",
    feature = "extended-combinatorics",
    feature = "extended-engine-runtime",
    feature = "extended-math-runtime",
    feature = "extended-compare-runtime",
    feature = "extended-logic-runtime",
    feature = "extended-range-runtime",
    feature = "extended-matrix-runtime",
    feature = "extended-set-runtime",
    feature = "extended-string-runtime",
    feature = "extended-stats-runtime",
    feature = "extended-combinatorics-runtime",
))]
use serde::Serialize;

#[cfg(any(
    feature = "full",
    feature = "representatives",
    feature = "extended-engine",
    feature = "extended-engine-shard",
    feature = "extended-engine-shard-convert",
    feature = "extended-math",
    feature = "extended-math-shard",
    feature = "extended-compare",
    feature = "extended-logic",
    feature = "extended-range",
    feature = "extended-matrix",
    feature = "extended-set",
    feature = "extended-string",
    feature = "extended-stats",
    feature = "extended-combinatorics",
))]
#[derive(Debug, Serialize)]
struct LinkageEntry<'a> {
    name: &'a str,
    id_hex: String,
    runtime_signature: String,
    signature_cargo_features: Vec<&'static str>,
    package: Option<&'static str>,
    crate_name: Option<&'static str>,
    installer_path: Option<&'static str>,
    cargo_features: Option<Vec<&'static str>>,
    contract_kind: &'static str,
    output_alias_policy: &'static str,
}

#[cfg(any(
    feature = "full",
    feature = "representatives",
    feature = "extended-engine",
    feature = "extended-engine-shard",
    feature = "extended-engine-shard-convert",
    feature = "extended-math",
    feature = "extended-math-shard",
    feature = "extended-compare",
    feature = "extended-logic",
    feature = "extended-range",
    feature = "extended-matrix",
    feature = "extended-set",
    feature = "extended-string",
    feature = "extended-stats",
    feature = "extended-combinatorics",
))]
fn emit(catalog: &FunctionCatalog) {
    let entries = catalog
        .runtime_entries()
        .map(|entry| {
            let linkage = entry.native_linkage.as_ref();
            let signature = entry.signature();
            let mut signature_cargo_features = signature
                .required_native_features()
                .into_iter()
                .map(mech_core::NativeValueFeature::cargo_feature)
                .collect::<Vec<_>>();
            signature_cargo_features.sort_unstable();
            LinkageEntry {
                name: &entry.name,
                id_hex: format!("{:016x}", entry.id.raw()),
                runtime_signature: format!("{signature:?}"),
                signature_cargo_features,
                package: linkage.map(|value| value.package),
                crate_name: linkage.map(|value| value.crate_name),
                installer_path: linkage.map(|value| value.installer_path),
                cargo_features: linkage.map(|value| value.cargo_features.clone()),
                contract_kind: entry.contract_kind(),
                output_alias_policy: match entry.output_alias_policy() {
                    mech_core::RuntimeOutputAliasPolicy::DisallowInputAlias => {
                        "disallow_input_alias"
                    }
                    mech_core::RuntimeOutputAliasPolicy::AllowInputAlias => "allow_input_alias",
                },
            }
        })
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout(), &entries).unwrap();
}

#[cfg(any(
    feature = "extended-engine-runtime",
    feature = "extended-math-runtime",
    feature = "extended-compare-runtime",
    feature = "extended-logic-runtime",
    feature = "extended-range-runtime",
    feature = "extended-matrix-runtime",
    feature = "extended-set-runtime",
    feature = "extended-string-runtime",
    feature = "extended-stats-runtime",
    feature = "extended-combinatorics-runtime",
))]
#[derive(Debug, Serialize)]
struct RuntimeEntry<'a> {
    name: &'a str,
    id_hex: String,
    runtime_signature: String,
    signature_cargo_features: Vec<&'static str>,
    contract_kind: &'static str,
    output_alias_policy: &'static str,
}

#[cfg(any(
    feature = "extended-engine-runtime",
    feature = "extended-math-runtime",
    feature = "extended-compare-runtime",
    feature = "extended-logic-runtime",
    feature = "extended-range-runtime",
    feature = "extended-matrix-runtime",
    feature = "extended-set-runtime",
    feature = "extended-string-runtime",
    feature = "extended-stats-runtime",
    feature = "extended-combinatorics-runtime",
))]
fn emit_runtime(catalog: &FunctionCatalog) {
    let entries = catalog
        .runtime_entries()
        .map(|entry| {
            let signature = entry.signature();
            let mut signature_cargo_features = signature
                .required_native_features()
                .into_iter()
                .map(mech_core::NativeValueFeature::cargo_feature)
                .collect::<Vec<_>>();
            signature_cargo_features.sort_unstable();
            RuntimeEntry {
                name: &entry.name,
                id_hex: format!("{:016x}", entry.id.raw()),
                runtime_signature: format!("{signature:?}"),
                signature_cargo_features,
                contract_kind: entry.contract_kind(),
                output_alias_policy: match entry.output_alias_policy() {
                    mech_core::RuntimeOutputAliasPolicy::DisallowInputAlias => {
                        "disallow_input_alias"
                    }
                    mech_core::RuntimeOutputAliasPolicy::AllowInputAlias => "allow_input_alias",
                },
            }
        })
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout(), &entries).unwrap();
}

#[cfg(all(feature = "full", not(feature = "representatives")))]
fn main() {
    // The frozen full surface is the broad dynamic-shape runtime catalog. Native
    // planning may add compiler-only factories, which are covered by the
    // extended owner profiles below rather than redefining this product
    // contract.
    emit(&mech_stdlib::runtime_catalog());
}

// The exhaustive owner surfaces intentionally instantiate every compiler-
// emittable concrete factory.  Keep construction off the platform's small
// process-main stack; the resulting catalog and JSON are otherwise identical.
#[cfg(any(
    feature = "extended-engine",
    feature = "extended-engine-shard",
    feature = "extended-engine-shard-convert",
    feature = "extended-math",
    feature = "extended-math-shard",
    feature = "extended-compare",
    feature = "extended-logic",
    feature = "extended-range",
    feature = "extended-matrix",
    feature = "extended-set",
    feature = "extended-string",
    feature = "extended-stats",
    feature = "extended-combinatorics",
    feature = "extended-engine-runtime",
    feature = "extended-math-runtime",
    feature = "extended-compare-runtime",
    feature = "extended-logic-runtime",
    feature = "extended-range-runtime",
    feature = "extended-matrix-runtime",
    feature = "extended-set-runtime",
    feature = "extended-string-runtime",
    feature = "extended-stats-runtime",
    feature = "extended-combinatorics-runtime",
))]
fn run_owner_catalog_on_large_stack(task: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("native-linkage-owner-catalog".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(task)
        .expect("spawn native linkage owner catalog thread")
        .join()
        .expect("native linkage owner catalog thread");
}

macro_rules! extended_owner_native_plan_main {
    ($feature:literal, $installer:path) => {
        #[cfg(feature = $feature)]
        fn main() {
            run_owner_catalog_on_large_stack(|| {
                let mut builder = FunctionCatalogBuilder::new();
                $installer(&mut builder).unwrap();
                emit(&builder.build().unwrap());
            });
        }
    };
}

extended_owner_native_plan_main!("extended-engine", mech_engine::install_intrinsic_native_plan);
extended_owner_native_plan_main!(
    "extended-engine-shard",
    mech_engine::install_intrinsic_native_plan
);
extended_owner_native_plan_main!(
    "extended-engine-shard-convert",
    mech_engine::install_intrinsic_native_plan
);
extended_owner_native_plan_main!("extended-math", mech_math::install_runtime);
extended_owner_native_plan_main!("extended-math-shard", mech_math::install_runtime);
extended_owner_native_plan_main!("extended-compare", mech_compare::install_runtime);
extended_owner_native_plan_main!("extended-logic", mech_logic::install_runtime);
extended_owner_native_plan_main!("extended-range", mech_range::install_runtime);
extended_owner_native_plan_main!("extended-matrix", mech_matrix::install_runtime);
extended_owner_native_plan_main!("extended-set", mech_set::install_runtime);
extended_owner_native_plan_main!("extended-string", mech_string::install_runtime);
extended_owner_native_plan_main!("extended-stats", mech_stats::install_runtime);
extended_owner_native_plan_main!(
    "extended-combinatorics",
    mech_combinatorics::install_runtime
);

macro_rules! extended_owner_runtime_main {
    ($feature:literal, $installer:path) => {
        #[cfg(feature = $feature)]
        fn main() {
            run_owner_catalog_on_large_stack(|| {
                let mut builder = FunctionCatalogBuilder::new();
                $installer(&mut builder).unwrap();
                emit_runtime(&builder.build().unwrap());
            });
        }
    };
}

extended_owner_runtime_main!(
    "extended-engine-runtime",
    mech_engine::install_intrinsic_runtime
);
extended_owner_runtime_main!("extended-math-runtime", mech_math::install_runtime);
extended_owner_runtime_main!("extended-compare-runtime", mech_compare::install_runtime);
extended_owner_runtime_main!("extended-logic-runtime", mech_logic::install_runtime);
extended_owner_runtime_main!("extended-range-runtime", mech_range::install_runtime);
extended_owner_runtime_main!("extended-matrix-runtime", mech_matrix::install_runtime);
extended_owner_runtime_main!("extended-set-runtime", mech_set::install_runtime);
extended_owner_runtime_main!("extended-string-runtime", mech_string::install_runtime);
extended_owner_runtime_main!("extended-stats-runtime", mech_stats::install_runtime);
extended_owner_runtime_main!(
    "extended-combinatorics-runtime",
    mech_combinatorics::install_runtime
);

#[cfg(all(feature = "representatives", not(feature = "full")))]
fn main() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_math::install_runtime(&mut builder).unwrap();
    emit(&builder.build().unwrap());
}

#[cfg(all(
    feature = "installer-profile",
    not(feature = "full"),
    not(feature = "representatives")
))]
fn main() {
    assert!(generated_catalog().runtime_factory_count() >= 1);
}

#[cfg(all(
    not(feature = "full"),
    not(feature = "representatives"),
    not(feature = "installer-profile"),
    not(feature = "owner-native-link"),
    not(feature = "extended-engine"),
    not(feature = "extended-engine-shard"),
    not(feature = "extended-engine-shard-convert"),
    not(feature = "extended-math"),
    not(feature = "extended-math-shard"),
    not(feature = "extended-compare"),
    not(feature = "extended-logic"),
    not(feature = "extended-range"),
    not(feature = "extended-matrix"),
    not(feature = "extended-set"),
    not(feature = "extended-string"),
    not(feature = "extended-stats"),
    not(feature = "extended-combinatorics"),
    not(feature = "extended-engine-runtime"),
    not(feature = "extended-math-runtime"),
    not(feature = "extended-compare-runtime"),
    not(feature = "extended-logic-runtime"),
    not(feature = "extended-range-runtime"),
    not(feature = "extended-matrix-runtime"),
    not(feature = "extended-set-runtime"),
    not(feature = "extended-string-runtime"),
    not(feature = "extended-stats-runtime"),
    not(feature = "extended-combinatorics-runtime")
))]
fn main() {
    panic!("enable exactly one of `full`, `representatives`, or `installers`");
}

#[cfg(all(
    feature = "owner-native-link",
    not(feature = "full"),
    not(feature = "representatives"),
    not(feature = "installer-profile")
))]
fn main() {}

#[cfg(any(
    all(feature = "full", feature = "representatives"),
    all(feature = "full", feature = "installer-profile"),
    all(feature = "representatives", feature = "installer-profile"),
    all(feature = "full", feature = "owner-native-link"),
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
        ("VerticalConcatenateNArgs<f64>", 0x006e_5ef9_27b7_6ce2),
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
