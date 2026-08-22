#[cfg(feature = "full_runtime")]
use std::collections::BTreeMap;
use std::sync::Mutex;

use mech_core::{FunctionCatalog, FunctionExposure};
#[cfg(feature = "full_runtime")]
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[cfg(feature = "full_runtime")]
const EXPECTED_RUNTIME_FACTORIES: usize = 9_046;
#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
const EXPECTED_STANDARD_COMPILER_RUNTIME_FACTORIES: usize = 1_341;
#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
const EXPECTED_STANDARD_SOURCE_SPECIALIZERS: usize = 82;
#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
const EXPECTED_STANDARD_COMPILER_RUNTIME_SURFACE_DIGEST: &str =
    "435526da1d03bc4e295192320422886ee10e85c72b9d2a8fdd8aeb371050c5f3";
#[cfg(feature = "full_compiler")]
const EXPECTED_FULL_COMPILER_RUNTIME_FACTORIES: usize = 9_099;
#[cfg(feature = "full_compiler")]
const EXPECTED_FULL_COMPILER_RUNTIME_SURFACE_DIGEST: &str =
    "b32454f34e5d22a37aae7b007c45c3e6d6e9865ce23e84be6ace932a621de350";
#[cfg(feature = "full_runtime")]
const EXPECTED_EXTENDED_RUNTIME_FACTORIES: usize = 120_017;
#[cfg(feature = "full_source")]
const EXPECTED_NAMED_SPECIALIZERS: usize = 137;
#[cfg(feature = "full_source")]
const EXPECTED_INTRINSIC_SPECIALIZERS: usize = 10;
#[cfg(feature = "full_source")]
const EXPECTED_PRELUDE_EXPORTS: usize = 52;
#[cfg(feature = "full_source")]
const EXPECTED_MODULE_EXPORTS: usize = 68;
#[cfg(feature = "full_source")]
const EXPECTED_ALL_EXPORTS: usize = 138;
#[cfg(feature = "full_runtime")]
const EXPECTED_RUNTIME_SURFACE_FILE_SHA256: &str =
    "ac50eae527981dcdfb1d2173a3ce95581d6e6e34caf4062c298e0b6c6d11908d";
#[cfg(feature = "full_runtime")]
const SOURCE_ONLY_RUNTIME_FACTORY: &str = "HorizontalConcatenateNArgs<f64>";
#[cfg(feature = "full_runtime")]
const EXPECTED_EXTENDED_RUNTIME_SURFACE_DIGEST: &str =
    "4bf16c1523cdc584d4e0479c3210903f0000679ba180601804388e94938b9c07";

static CATALOG_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "full_runtime")]
const RUNTIME_SURFACE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/function-system/runtime-factory-surface.json"
));
#[cfg(feature = "full_source")]
const SOURCE_SURFACE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/function-system/function-surface.json"
));

#[cfg(feature = "full_runtime")]
#[derive(Debug, Deserialize)]
struct FrozenRuntimeSurface {
    runtime_factories: Vec<FrozenRuntimeFactory>,
}

#[cfg(feature = "full_runtime")]
#[derive(Debug, Deserialize)]
struct FrozenRuntimeFactory {
    name: String,
    id_hex: String,
}

#[cfg(feature = "full_source")]
#[derive(Debug, Deserialize)]
struct FrozenSourceSurface {
    full_source_specializers: Vec<FrozenSpecializer>,
    prelude_source_specializers: Vec<FrozenSpecializer>,
    module_exports: Vec<FrozenModuleExport>,
}

#[cfg(feature = "full_source")]
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FrozenSpecializer {
    name: String,
    id_hex: String,
}

#[cfg(feature = "full_source")]
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

#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
fn assert_standard_compiler_surface(catalog: &FunctionCatalog) {
    assert_eq!(
        catalog.runtime_factory_count(),
        EXPECTED_STANDARD_COMPILER_RUNTIME_FACTORIES
    );
    assert_eq!(
        canonical_runtime_surface_digest(catalog),
        EXPECTED_STANDARD_COMPILER_RUNTIME_SURFACE_DIGEST
    );
    assert_eq!(
        catalog.specializer_count(),
        EXPECTED_STANDARD_SOURCE_SPECIALIZERS
    );
}

#[cfg(feature = "compiler")]
fn assert_runtime_catalog_is_a_compiler_subset(
    runtime: &FunctionCatalog,
    compiler: &FunctionCatalog,
) {
    for entry in runtime.runtime_entries() {
        let compiled = compiler
            .runtime_entry(entry.id)
            .unwrap_or_else(|| panic!("compiler catalog omitted runtime factory {}", entry.name));
        assert_eq!(compiled.name, entry.name);
    }
}

fn with_catalog_test_stack(test: impl FnOnce() + Send + 'static) {
    let _guard = CATALOG_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let test = std::thread::Builder::new()
        .name("stdlib-profile-contract".to_string())
        // Native-linkage registration now follows the complete owner-local
        // factory surface.  Keep the contract test isolated from the small
        // platform default thread stack while it constructs that catalog.
        // Match the exhaustive native-linkage fixture: all-features expands
        // the owner-local catalog far beyond the frozen standard profile.
        .stack_size(64 * 1024 * 1024)
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

#[cfg(feature = "full_runtime")]
fn assert_runtime_surface(catalog: &FunctionCatalog, allow_source_only_factory: bool) {
    let count = catalog.runtime_factory_count();
    if count == EXPECTED_EXTENDED_RUNTIME_FACTORIES {
        assert_eq!(
            canonical_runtime_surface_digest(catalog),
            EXPECTED_EXTENDED_RUNTIME_SURFACE_DIGEST,
            "extended runtime catalog diverged from the exhaustive all-features contract",
        );
        return;
    }

    let expected_count = EXPECTED_RUNTIME_FACTORIES + usize::from(allow_source_only_factory);
    assert_eq!(
        count, expected_count,
        "selected runtime catalog is neither the frozen standard nor exhaustive profile",
    );

    let frozen: FrozenRuntimeSurface =
        serde_json::from_slice(RUNTIME_SURFACE).expect("frozen runtime surface must be valid JSON");
    assert_eq!(frozen.runtime_factories.len(), EXPECTED_RUNTIME_FACTORIES);

    let expected = frozen
        .runtime_factories
        .into_iter()
        .map(|entry| (entry.id_hex, entry.name))
        .collect::<BTreeMap<_, _>>();
    let mut actual = catalog
        .runtime_entries()
        .map(|entry| (id_hex(entry.id.raw()), entry.name.clone()))
        .collect::<BTreeMap<_, _>>();
    if allow_source_only_factory {
        let source_only = actual
            .iter()
            .find_map(|(id, name)| (name == SOURCE_ONLY_RUNTIME_FACTORY).then(|| id.clone()))
            .expect("full source catalog must contain its one source-only runtime factory");
        actual.remove(&source_only);
    }
    assert_eq!(actual, expected, "runtime catalog diverged from PR2");

    let digest = Sha256::digest(RUNTIME_SURFACE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(digest, EXPECTED_RUNTIME_SURFACE_FILE_SHA256);
}

#[cfg(feature = "full_source")]
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

#[cfg(feature = "full_runtime")]
#[test]
fn selected_runtime_matches_a_frozen_runtime_surface() {
    with_catalog_test_stack(|| {
        let catalog = mech_stdlib::runtime_catalog();
        assert_runtime_surface(&catalog, false);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    });
}

#[cfg(all(feature = "full_source", not(feature = "full_compiler")))]
#[test]
fn selected_source_matches_the_frozen_source_surface() {
    with_catalog_test_stack(|| {
        let catalog = mech_stdlib::source_catalog();
        assert_runtime_surface(&catalog, true);
        assert_source_surface(&catalog);
    });
}

#[cfg(feature = "full_compiler")]
#[test]
fn selected_compiler_closes_the_emitted_factory_surface() {
    with_catalog_test_stack(|| {
        let runtime = mech_stdlib::runtime_catalog();
        assert_runtime_surface(&runtime, false);
        let catalog = mech_stdlib::source_catalog();
        assert_runtime_catalog_is_a_compiler_subset(&runtime, &catalog);
        assert_eq!(
            catalog.runtime_factory_count(),
            EXPECTED_FULL_COMPILER_RUNTIME_FACTORIES
        );
        assert_eq!(
            canonical_runtime_surface_digest(&catalog),
            EXPECTED_FULL_COMPILER_RUNTIME_SURFACE_DIGEST
        );
        assert_source_surface(&catalog);
    });
}

#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
#[test]
fn standard_compiler_closes_the_emitted_factory_surface() {
    with_catalog_test_stack(|| {
        let runtime = mech_stdlib::runtime_catalog();
        let catalog = mech_stdlib::source_catalog();
        assert_runtime_catalog_is_a_compiler_subset(&runtime, &catalog);
        assert_standard_compiler_surface(&catalog);
    });
}
