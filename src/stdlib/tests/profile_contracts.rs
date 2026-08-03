#[cfg(feature = "standard_runtime")]
use std::collections::BTreeMap;

use mech_core::{FunctionCatalog, FunctionExposure};
#[cfg(feature = "standard_runtime")]
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[cfg(feature = "standard_runtime")]
const EXPECTED_RUNTIME_FACTORIES: usize = 9_019;
#[cfg(feature = "standard_source")]
const EXPECTED_NAMED_SPECIALIZERS: usize = 119;
#[cfg(feature = "standard_source")]
const EXPECTED_INTRINSIC_SPECIALIZERS: usize = 10;
#[cfg(feature = "standard_source")]
const EXPECTED_PRELUDE_EXPORTS: usize = 52;
#[cfg(feature = "standard_source")]
const EXPECTED_MODULE_EXPORTS: usize = 50;
#[cfg(feature = "standard_source")]
const EXPECTED_ALL_EXPORTS: usize = 120;
#[cfg(feature = "standard_runtime")]
const EXPECTED_RUNTIME_SURFACE_DIGEST: &str =
    "b9db9003bb9da704d5b61a5a6a3d5fcc6438ef7e433f49fc1918c466fc2fcc62";

#[cfg(feature = "standard_runtime")]
const RUNTIME_SURFACE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/function-system/runtime-factory-surface.json"
));
#[cfg(feature = "standard_source")]
const SOURCE_SURFACE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/function-system/function-surface.json"
));

#[cfg(feature = "standard_runtime")]
#[derive(Debug, Deserialize)]
struct FrozenRuntimeSurface {
    runtime_factories: Vec<FrozenRuntimeFactory>,
}

#[cfg(feature = "standard_runtime")]
#[derive(Debug, Deserialize)]
struct FrozenRuntimeFactory {
    name: String,
    id_hex: String,
}

#[cfg(feature = "standard_source")]
#[derive(Debug, Deserialize)]
struct FrozenSourceSurface {
    full_source_specializers: Vec<FrozenSpecializer>,
    prelude_source_specializers: Vec<FrozenSpecializer>,
    module_exports: Vec<FrozenModuleExport>,
}

#[cfg(feature = "standard_source")]
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FrozenSpecializer {
    name: String,
    id_hex: String,
}

#[cfg(feature = "standard_source")]
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FrozenModuleExport {
    module: String,
    item: String,
    canonical_name: String,
    id_hex: String,
}

fn id_hex(id: u64) -> String {
    format!("{id:016x}")
}

/// Hashes a selected catalog independently of any JSON formatting. Entries are
/// sorted lexicographically as `id_hex<TAB>name<LF>` before SHA-256 is applied.
fn canonical_runtime_surface_digest(catalog: &FunctionCatalog) -> String {
    let mut entries = catalog
        .runtime_entries()
        .map(|entry| format!("{}\t{}\n", id_hex(entry.id.raw()), entry.name))
        .collect::<Vec<_>>();
    entries.sort();
    Sha256::digest(entries.concat().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn with_catalog_test_stack(test: impl FnOnce() + Send + 'static) {
    let test = std::thread::Builder::new()
        .name("stdlib-profile-contract".to_string())
        // Native-linkage registration now follows the complete owner-local
        // factory surface.  Keep the contract test isolated from the small
        // platform default thread stack while it constructs that catalog.
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .expect("profile contract thread must spawn");
    if let Err(payload) = test.join() {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn distribution_size_report_catalog_counts() {
    with_catalog_test_stack(|| {
        #[cfg(feature = "source")]
        let catalog = mech_stdlib::source_catalog();
        #[cfg(not(feature = "source"))]
        let catalog = mech_stdlib::runtime_catalog();
        let runtime_factory_count = catalog.runtime_factory_count();
        let runtime_surface_digest = canonical_runtime_surface_digest(&catalog);

        println!(
            "MECH_CATALOG_COUNTS\t{}\t{}\t{}",
            runtime_factory_count,
            catalog.specializer_count(),
            runtime_surface_digest,
        );

        if let Ok(expected) = std::env::var("MECH_EXPECT_RUNTIME_FACTORY_COUNT") {
            assert_eq!(runtime_factory_count, expected.parse::<usize>().unwrap());
        }
        if let Ok(expected) = std::env::var("MECH_EXPECT_RUNTIME_SURFACE_DIGEST") {
            assert_eq!(runtime_surface_digest, expected);
        }
        if let Ok(expected) = std::env::var("MECH_EXPECT_SOURCE_SPECIALIZER_COUNT") {
            assert_eq!(
                catalog.specializer_count(),
                expected.parse::<usize>().unwrap(),
            );
        }
        if let Ok(expected) = std::env::var("MECH_EXPECT_INTRINSIC_COUNT") {
            assert_eq!(
                catalog.intrinsic_specializer_count(),
                expected.parse::<usize>().unwrap(),
            );
        }
        if let Ok(expected) = std::env::var("MECH_EXPECT_PRELUDE_COUNT") {
            assert_eq!(
                catalog
                    .all_exports()
                    .filter(|export| export.exposure == FunctionExposure::Prelude)
                    .count(),
                expected.parse::<usize>().unwrap(),
            );
        }
        if let Ok(expected) = std::env::var("MECH_EXPECT_MODULE_EXPORT_COUNT") {
            assert_eq!(
                catalog
                    .all_exports()
                    .filter(|export| export.module.is_some() && export.item.is_some())
                    .count(),
                expected.parse::<usize>().unwrap(),
            );
        }
        if let Ok(expected) = std::env::var("MECH_EXPECT_TOTAL_EXPORT_COUNT") {
            assert_eq!(
                catalog.all_exports().len(),
                expected.parse::<usize>().unwrap(),
            );
        }
    });
}

#[cfg(feature = "standard_runtime")]
fn assert_runtime_surface(catalog: &FunctionCatalog) {
    assert_eq!(catalog.runtime_factory_count(), EXPECTED_RUNTIME_FACTORIES);

    let frozen: FrozenRuntimeSurface =
        serde_json::from_slice(RUNTIME_SURFACE).expect("frozen runtime surface must be valid JSON");
    assert_eq!(frozen.runtime_factories.len(), EXPECTED_RUNTIME_FACTORIES);

    let expected = frozen
        .runtime_factories
        .into_iter()
        .map(|entry| (entry.id_hex, entry.name))
        .collect::<BTreeMap<_, _>>();
    let actual = catalog
        .runtime_entries()
        .map(|entry| (id_hex(entry.id.raw()), entry.name.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected, "runtime catalog diverged from PR2");

    let digest = Sha256::digest(RUNTIME_SURFACE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(digest, EXPECTED_RUNTIME_SURFACE_DIGEST);
}

#[cfg(feature = "standard_source")]
fn assert_source_surface(catalog: &FunctionCatalog) {
    assert_eq!(catalog.specializer_count(), EXPECTED_NAMED_SPECIALIZERS);
    assert_eq!(
        catalog.intrinsic_specializer_count(),
        EXPECTED_INTRINSIC_SPECIALIZERS
    );
    assert_eq!(catalog.all_exports().len(), EXPECTED_ALL_EXPORTS);
    assert_eq!(
        catalog
            .all_exports()
            .filter(|export| export.exposure == FunctionExposure::Prelude)
            .count(),
        EXPECTED_PRELUDE_EXPORTS,
    );

    let frozen: FrozenSourceSurface =
        serde_json::from_str(SOURCE_SURFACE).expect("frozen source surface must be valid JSON");

    let mut actual_specializers = catalog
        .all_specializers()
        .map(|entry| FrozenSpecializer {
            name: entry.canonical_name.clone(),
            id_hex: id_hex(entry.operation.raw()),
        })
        .collect::<Vec<_>>();
    actual_specializers
        .sort_by(|left, right| (&left.name, &left.id_hex).cmp(&(&right.name, &right.id_hex)));
    assert_eq!(actual_specializers, frozen.full_source_specializers);

    let mut actual_prelude = catalog
        .all_specializers()
        .filter(|entry| {
            catalog
                .exports_for_operation(entry.operation)
                .iter()
                .any(|export| export.exposure == FunctionExposure::Prelude)
        })
        .map(|entry| FrozenSpecializer {
            name: entry.canonical_name.clone(),
            id_hex: id_hex(entry.operation.raw()),
        })
        .collect::<Vec<_>>();
    actual_prelude
        .sort_by(|left, right| (&left.name, &left.id_hex).cmp(&(&right.name, &right.id_hex)));
    assert_eq!(actual_prelude.len(), EXPECTED_PRELUDE_EXPORTS);
    assert_eq!(actual_prelude, frozen.prelude_source_specializers);

    let mut actual_modules = catalog
        .all_exports()
        .filter_map(|export| match (&export.module, &export.item) {
            (Some(module), Some(item)) => Some(FrozenModuleExport {
                module: module.clone(),
                item: item.clone(),
                canonical_name: export.canonical_name.clone(),
                id_hex: id_hex(export.operation.raw()),
            }),
            (None, None) => None,
            _ => panic!("validated export has a partial module/item pair"),
        })
        .collect::<Vec<_>>();
    actual_modules.sort_by(|left, right| {
        (&left.module, &left.item, &left.canonical_name).cmp(&(
            &right.module,
            &right.item,
            &right.canonical_name,
        ))
    });
    assert_eq!(actual_modules.len(), EXPECTED_MODULE_EXPORTS);
    assert_eq!(actual_modules, frozen.module_exports);
}

#[cfg(feature = "standard_runtime")]
#[test]
fn standard_runtime_matches_the_frozen_runtime_surface() {
    with_catalog_test_stack(|| {
        let catalog = mech_stdlib::runtime_catalog();
        assert_runtime_surface(&catalog);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    });
}

#[cfg(feature = "standard_source")]
#[test]
fn standard_source_matches_the_frozen_source_surface() {
    with_catalog_test_stack(|| {
        let catalog = mech_stdlib::source_catalog();
        assert_runtime_surface(&catalog);
        assert_source_surface(&catalog);
    });
}

#[cfg(feature = "standard_compiler")]
#[test]
fn standard_compiler_preserves_the_standard_source_catalog() {
    with_catalog_test_stack(|| {
        let catalog = mech_stdlib::source_catalog();
        assert_runtime_surface(&catalog);
        assert_source_surface(&catalog);
    });
}
