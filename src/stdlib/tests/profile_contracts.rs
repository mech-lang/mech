#[cfg(feature = "full_runtime")]
use std::collections::BTreeMap;
use std::sync::Mutex;

use mech_core::{FunctionCatalog, FunctionExposure};
#[cfg(feature = "full_runtime")]
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[cfg(feature = "full_runtime")]
const EXPECTED_RUNTIME_FACTORIES: usize = 9_726;
#[cfg(feature = "full_source")]
const EXPECTED_SOURCE_ENABLED_RUNTIME_FACTORIES: usize = 9_728;
#[cfg(feature = "full_source")]
const EXPECTED_SOURCE_ENABLED_RUNTIME_SURFACE_DIGEST: &str =
    "ab6547f38932ef29a675a3aef0e6c11a6db59e7fc712adc2f4a51bb85b961793";
#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
const EXPECTED_STANDARD_COMPILER_RUNTIME_FACTORIES: usize = 1_425;
#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
const EXPECTED_STANDARD_SOURCE_SPECIALIZERS: usize = 73;
#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
const EXPECTED_STANDARD_COMPILER_RUNTIME_SURFACE_DIGEST: &str =
    "a5c6a86ad547f744aeb16abb17ccd4a2faa54509d58b69e7fc1715e165a49bde";
#[cfg(all(feature = "full_source", not(feature = "full_compiler")))]
const EXPECTED_FULL_SOURCE_RUNTIME_FACTORIES: usize = 15_705;
#[cfg(all(feature = "full_source", not(feature = "full_compiler")))]
const EXPECTED_FULL_SOURCE_RUNTIME_SURFACE_DIGEST: &str =
    "c5c2bae913e4482925289b54e58559b7676bb9ad66aede1e2ce6caf9501f2745";
#[cfg(feature = "full_compiler")]
const EXPECTED_FULL_COMPILER_RUNTIME_FACTORIES: usize = 15_709;
#[cfg(feature = "full_compiler")]
const EXPECTED_FULL_COMPILER_RUNTIME_SURFACE_DIGEST: &str =
    "3b4c4ee4f6c5978e8bc5bbe7800630a25664cd1ddaf69aa8a5b03b426ad89740";
#[cfg(feature = "full_runtime")]
const EXPECTED_EXTENDED_RUNTIME_FACTORIES: usize = 122_093;
#[cfg(feature = "full_source")]
const EXPECTED_NAMED_SPECIALIZERS: usize = 129;
#[cfg(feature = "full_source")]
const EXPECTED_INTRINSIC_SPECIALIZERS: usize = 13;
#[cfg(feature = "full_source")]
const EXPECTED_PRELUDE_EXPORTS: usize = 52;
#[cfg(feature = "full_source")]
const EXPECTED_MODULE_EXPORTS: usize = 60;
#[cfg(feature = "full_source")]
const EXPECTED_ALL_EXPORTS: usize = 130;
#[cfg(feature = "full_runtime")]
const EXPECTED_RUNTIME_SURFACE_FILE_SHA256: &str =
    "bcd742d6c4857b06d538f087c7024fd4d82fbf9d292699c0619868b967b073da";
#[cfg(feature = "full_runtime")]
const EXPECTED_EXTENDED_RUNTIME_SURFACE_DIGEST: &str =
    "34db793ac637b3b1bc532978c6e8a6e0be1b8be63c33ec5f3043045f765616f5";

// Current complete identities are derived from the qualified catalog capture.
// Historical retained projections remain independently asserted below.
#[cfg(feature = "full_runtime")]
const EXPECTED_CURRENT_RUNTIME_SURFACE_DIGEST: &str =
    "3ab7408f06e6a8459a410804854e35ee751ad08092e4fdb33a3152878b1ec21e";
#[cfg(feature = "full_source")]
const EXPECTED_CURRENT_SOURCE_ENABLED_RUNTIME_SURFACE_DIGEST: &str =
    "a74bcef6119bd7a06ad15a4c95f3d32851e8ce12136a40ce7812593bbfe50da9";
#[cfg(feature = "full_compiler")]
const EXPECTED_CURRENT_FULL_COMPILER_RUNTIME_SURFACE_DIGEST: &str =
    "2ab905a5774fbe6fccc6ce7c5df3a3a5d7536d136816d0b10be26cd4e2438923";

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

/// Preserve every historical profile digest as an independent compatibility
/// check. Only the four exact text/number additions, the three exact map
/// additions, and the three exact ordinary sort/text additions, independently
/// checked below, are removed from the historical projection; current fixtures
/// include every addition too.
#[cfg(any(feature = "full_runtime", feature = "standard_compiler"))]
fn retained_runtime_surface_digest(catalog: &FunctionCatalog) -> String {
    use mech_core::{
        AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
        FunctionMatrixElement, FunctionMatrixStoragePattern, FunctionValueRepresentation as Value,
        ImplementationMemoryClass, InputPortLayout, InputPortPolicy, OperationContractDeclaration,
        OperationId, OutputConstruction, OutputPortPolicy, RuntimeFunctionId,
        RuntimeFunctionSignature, RuntimeOperationBinding, ShapeRule,
    };
    let additions = [
        (
            "number/parse-u64",
            "number/parse-u64",
            1,
            false,
            RuntimeFunctionSignature::unary(Value::AnyValue, Value::String),
        ),
        (
            "NumberFormatScalarCanonical",
            "number/format",
            2,
            false,
            RuntimeFunctionSignature::binary(Value::String, Value::AnyValue, Value::AnyValue),
        ),
        (
            "NumberFormatMatrixCanonical",
            "number/format",
            2,
            true,
            RuntimeFunctionSignature::binary(
                Value::Matrix {
                    element: FunctionMatrixElement::String,
                    storage: FunctionMatrixStoragePattern::AnyStorage,
                },
                Value::AnyValue,
                Value::AnyValue,
            ),
        ),
        (
            "sequence/length",
            "sequence/length",
            1,
            false,
            RuntimeFunctionSignature::unary(Value::AnyValue, Value::AnyValue),
        ),
    ];
    for (factory, operation, arity, matrix, signature) in &additions {
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name(factory))
            .unwrap_or_else(|| panic!("profile omitted new factory {factory}"));
        assert_eq!(entry.name, *factory);
        assert_eq!(
            entry.id.raw(),
            match *factory {
                "number/parse-u64" => 0x0037_5c52_4331_ece1,
                "NumberFormatScalarCanonical" => 0x0098_6ef1_0109_4a31,
                "NumberFormatMatrixCanonical" => 0x0041_7c7a_5c2d_07f0,
                "sequence/length" => 0x0052_30d2_7927_cb75,
                _ => unreachable!(),
            }
        );
        assert_eq!(entry.signature(), *signature);
        assert!(
            matches!(entry.operation_binding(), RuntimeOperationBinding::Fixed(ids)
            if ids.as_ref() == [OperationId::from_name(operation)])
        );
        assert_eq!(
            entry.implementation_memory_class(),
            ImplementationMemoryClass::CanonicalFinalize
        );
        let expected = OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal
                    };
                    *arity
                ]
                .into_boxed_slice(),
            ),
            outputs: Box::new([OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: if *operation == "number/format" {
                        ShapeRule::SameAsInput { input: 0 }
                    } else {
                        ShapeRule::Declared
                    },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: if *matrix {
                    ChangeDetectionPolicy::KernelReported
                } else {
                    ChangeDetectionPolicy::ExactScalar
                },
            }]),
            interaction: ExternalInteraction::Pure,
        };
        assert_eq!(
            entry.operation_contract(OperationId::from_name(operation)),
            Some(&expected)
        );
    }
    let map_additions = assert_map_runtime_additions(catalog);
    let collection_text_additions = assert_collection_text_runtime_additions(catalog);
    let mut retained = catalog
        .runtime_entries()
        .filter(|entry| {
            !additions.iter().any(|(name, ..)| entry.name == *name)
                && !map_additions.contains(&entry.name.as_str())
                && !collection_text_additions.contains(&entry.name.as_str())
        })
        .map(|entry| format!("{}\t{}\n", id_hex(entry.id.raw()), entry.name))
        .collect::<Vec<_>>();
    assert_eq!(
        retained.len() + additions.len() + map_additions.len() + collection_text_additions.len(),
        catalog.runtime_factory_count()
    );
    retained.sort_unstable();
    Sha256::digest(retained.concat().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(any(feature = "full_runtime", feature = "standard_compiler"))]
fn assert_map_runtime_additions(catalog: &FunctionCatalog) -> [&'static str; 3] {
    use mech_core::{
        ExecutionTarget, FunctionValueRepresentation as Value, ImplementationMemoryClass,
        OperationId, RuntimeFunctionId, RuntimeFunctionSignature, RuntimeOperationBinding,
        RuntimeOutputAliasPolicy,
    };
    let additions = [
        (
            "map/histogram",
            0x0089_e5df_42d1_3de4,
            "install_map_histogram",
            ImplementationMemoryClass::CanonicalSortUnique,
        ),
        (
            "map/keys",
            0x0054_5bda_103a_f406,
            "install_map_keys",
            ImplementationMemoryClass::CanonicalFinalize,
        ),
        (
            "map/values",
            0x009a_fda2_41ea_5663,
            "install_map_values",
            ImplementationMemoryClass::CanonicalFinalize,
        ),
    ];
    for (name, id, installer, memory) in additions {
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .unwrap_or_else(|| panic!("profile omitted map factory {name}"));
        assert_eq!(entry.name, name);
        assert_eq!(entry.id.raw(), id);
        assert_eq!(OperationId::from_name(name).raw(), id);
        let operation = OperationId::from_raw(id);
        assert_eq!(
            entry.signature(),
            RuntimeFunctionSignature::unary(Value::AnyValue, Value::AnyValue)
        );
        assert!(
            matches!(entry.operation_binding(), RuntimeOperationBinding::Fixed(ids)
            if ids.as_ref() == [operation])
        );
        assert_eq!(entry.contract_kind(), name);
        assert_eq!(
            entry.output_alias_policy(),
            RuntimeOutputAliasPolicy::DisallowInputAlias
        );
        assert_eq!(entry.implementation_memory_class(), memory);
        assert_eq!(
            entry.operation_contract(operation),
            Some(&map_operation_contract())
        );
        assert!(entry.supports_target(ExecutionTarget::DirectRuntime));
        #[cfg(feature = "native-plan")]
        {
            assert!(entry.supports_target(ExecutionTarget::Native));
            let linkage = entry
                .native_linkage
                .as_ref()
                .expect("map native linkage is required");
            assert_eq!(linkage.package, "mech-engine");
            assert_eq!(linkage.crate_name, "mech_engine");
            let expected_installer = format!("mech_engine::__mech_native::{installer}");
            assert_eq!(linkage.installer_path, expected_installer.as_str());
            assert_eq!(
                linkage.cargo_features,
                ["map_ops", "native-link", "runtime"]
            );
        }
        #[cfg(not(feature = "native-plan"))]
        let _ = installer;
    }
    ["map/histogram", "map/keys", "map/values"]
}

#[cfg(any(feature = "full_runtime", feature = "standard_compiler"))]
fn map_operation_contract() -> mech_core::OperationContractDeclaration {
    use mech_core::{
        AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
        InputPortLayout, InputPortPolicy, OperationContractDeclaration, OutputConstruction,
        OutputPortPolicy, ShapeRule,
    };
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([InputPortPolicy {
            access: AccessMode::Read,
            delivery: DeliveryMode::Signal,
        }])),
        outputs: Box::new([OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(any(feature = "full_source", feature = "standard_compiler"))]
fn assert_map_source_exports(catalog: &FunctionCatalog) {
    for (item, id) in [
        ("histogram", 0x0089_e5df_42d1_3de4),
        ("keys", 0x0054_5bda_103a_f406),
        ("values", 0x009a_fda2_41ea_5663),
    ] {
        let name = format!("map/{item}");
        let operation = mech_core::OperationId::from_name(&name);
        assert_eq!(operation.raw(), id);
        let specializer = catalog.specializer(operation).expect("map specializer");
        assert_eq!(specializer.operation.id, operation);
        assert_eq!(specializer.operation.canonical_name.as_ref(), name.as_str());
        assert_eq!(
            catalog.source_type_declaration(&name),
            Some(&mech_core::maintained_source_type_declaration(&name).unwrap())
        );
        let export = catalog
            .module_export("map", item)
            .expect("map module export");
        assert_eq!(export.operation, operation);
        assert_eq!(export.canonical_name, name);
        assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
        assert_eq!(catalog.exports_for_operation(operation).len(), 1);
        assert_eq!(
            catalog.source_operation_contract(&name, 1, item != "histogram"),
            Some(&map_operation_contract())
        );
    }
    assert_eq!(catalog.module_exports("map").len(), 3);
}

#[cfg(any(feature = "full_runtime", feature = "standard_compiler"))]
fn assert_collection_text_runtime_additions(catalog: &FunctionCatalog) -> [&'static str; 3] {
    use mech_core::{
        ExecutionTarget, FunctionValueRepresentation as Value, ImplementationMemoryClass,
        OperationId, RuntimeFunctionId, RuntimeFunctionSignature, RuntimeOperationBinding,
        RuntimeOutputAliasPolicy,
    };
    let additions = [
        (
            "sequence/sort-indices",
            4,
            false,
            "install_sequence_sort_indices",
            "sequence_sort",
            RuntimeFunctionSignature::quaternary(
                Value::AnyValue,
                Value::AnyValue,
                Value::String,
                Value::AnyValue,
                Value::String,
            ),
        ),
        (
            "text/lines",
            1,
            false,
            "install_text_lines",
            "pure_text",
            RuntimeFunctionSignature::unary(Value::AnyValue, Value::AnyValue),
        ),
        (
            "text/join",
            2,
            true,
            "install_text_join",
            "pure_text",
            RuntimeFunctionSignature::binary(Value::AnyValue, Value::AnyValue, Value::AnyValue),
        ),
    ];
    for (name, arity, scalar, installer, feature, signature) in additions {
        let operation = OperationId::from_name(name);
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .unwrap_or_else(|| panic!("profile omitted ordinary factory {name}"));
        assert_eq!(entry.name, name);
        assert_eq!(entry.id.raw(), operation.raw());
        assert_eq!(entry.signature(), signature);
        assert!(
            matches!(entry.operation_binding(), RuntimeOperationBinding::Fixed(ids)
            if ids.as_ref() == [operation])
        );
        assert_eq!(entry.contract_kind(), name);
        assert_eq!(
            entry.output_alias_policy(),
            RuntimeOutputAliasPolicy::DisallowInputAlias
        );
        assert_eq!(
            entry.implementation_memory_class(),
            ImplementationMemoryClass::CanonicalFinalize
        );
        assert_eq!(
            entry.operation_contract(operation),
            Some(&collection_text_operation_contract(arity, scalar))
        );
        assert!(entry.supports_target(ExecutionTarget::DirectRuntime));
        #[cfg(feature = "native-plan")]
        {
            assert!(entry.supports_target(ExecutionTarget::Native));
            let linkage = entry
                .native_linkage
                .as_ref()
                .expect("ordinary factory native linkage");
            assert_eq!(linkage.package, "mech-engine");
            assert_eq!(linkage.crate_name, "mech_engine");
            assert_eq!(
                linkage.installer_path,
                format!("mech_engine::__mech_native::{installer}")
            );
            let mut features = vec![feature, "native-link", "runtime"];
            if name == "sequence/sort-indices" {
                features.push("string"); // The two direction ports are concrete Strings.
            }
            features.sort_unstable();
            assert_eq!(linkage.cargo_features, features);
        }
        #[cfg(not(feature = "native-plan"))]
        let _ = (installer, feature);
    }
    ["sequence/sort-indices", "text/lines", "text/join"]
}

#[cfg(any(feature = "full_runtime", feature = "standard_compiler"))]
fn collection_text_operation_contract(
    arity: usize,
    scalar: bool,
) -> mech_core::OperationContractDeclaration {
    use mech_core::{
        AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
        InputPortLayout, InputPortPolicy, OperationContractDeclaration, OutputConstruction,
        OutputPortPolicy, ShapeRule,
    };
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                arity
            ]
            .into_boxed_slice(),
        ),
        outputs: Box::new([OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: if scalar {
                ChangeDetectionPolicy::ExactScalar
            } else {
                ChangeDetectionPolicy::KernelReported
            },
        }]),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(any(feature = "full_source", feature = "standard_compiler"))]
fn assert_collection_text_source_exports(catalog: &FunctionCatalog) {
    for (module, item, arity, scalar) in [
        ("sequence", "sort-indices", 4, false),
        ("text", "lines", 1, false),
        ("text", "join", 2, true),
    ] {
        let name = format!("{module}/{item}");
        let operation = mech_core::OperationId::from_name(&name);
        let specializer = catalog
            .specializer(operation)
            .expect("ordinary specializer");
        assert_eq!(specializer.operation.id, operation);
        assert_eq!(specializer.operation.canonical_name.as_ref(), name.as_str());
        assert_eq!(
            catalog.source_type_declaration(&name),
            Some(&mech_core::maintained_source_type_declaration(&name).unwrap())
        );
        let export = catalog
            .module_export(module, item)
            .expect("ordinary module export");
        assert_eq!(export.operation, operation);
        assert_eq!(export.canonical_name, name);
        assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
        assert_eq!(catalog.exports_for_operation(operation).len(), 1);
        assert_eq!(
            catalog.source_operation_contract(&name, arity, !scalar),
            Some(&collection_text_operation_contract(arity, scalar))
        );
    }
    assert_eq!(catalog.module_exports("text").len(), 2);
    assert_eq!(catalog.module_exports("sequence").len(), 2);
}

#[cfg(any(feature = "full_source", feature = "standard_compiler"))]
fn assert_pure_text_source_exports(catalog: &FunctionCatalog) {
    let additions = [
        ("number", "format"),
        ("number", "parse-u64"),
        ("sequence", "length"),
    ];
    for (module, item) in additions {
        let name = format!("{module}/{item}");
        let operation = mech_core::OperationId::from_name(&name);
        assert!(
            catalog.specializer(operation).is_some(),
            "missing {name} specializer"
        );
        let export = catalog.module_export(module, item).unwrap();
        assert_eq!(export.operation, operation);
        assert_eq!(export.canonical_name, name);
        assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
        assert_eq!(
            catalog.exports_for_operation(operation).len(),
            1,
            "{name} gained another exposure"
        );
    }
}

#[cfg(all(feature = "standard_compiler", not(feature = "full_compiler")))]
fn assert_standard_compiler_surface(catalog: &FunctionCatalog) {
    assert_pure_text_source_exports(catalog);
    assert_map_source_exports(catalog);
    assert_collection_text_source_exports(catalog);
    assert_eq!(
        catalog.runtime_factory_count(),
        EXPECTED_STANDARD_COMPILER_RUNTIME_FACTORIES
    );
    assert_eq!(
        retained_runtime_surface_digest(catalog),
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
fn frozen_runtime_surface() -> BTreeMap<String, String> {
    let frozen: FrozenRuntimeSurface =
        serde_json::from_slice(RUNTIME_SURFACE).expect("frozen runtime surface must be valid JSON");
    assert_eq!(frozen.runtime_factories.len(), EXPECTED_RUNTIME_FACTORIES);
    frozen
        .runtime_factories
        .into_iter()
        .map(|entry| (entry.id_hex, entry.name))
        .collect()
}

#[cfg(feature = "full_runtime")]
fn assert_runtime_surface(catalog: &FunctionCatalog) {
    assert_map_runtime_additions(catalog);
    assert_collection_text_runtime_additions(catalog);
    let count = catalog.runtime_factory_count();
    #[cfg(feature = "full_source")]
    if count == EXPECTED_SOURCE_ENABLED_RUNTIME_FACTORIES {
        assert_eq!(
            canonical_runtime_surface_digest(catalog),
            EXPECTED_CURRENT_SOURCE_ENABLED_RUNTIME_SURFACE_DIGEST
        );
        assert_eq!(
            retained_runtime_surface_digest(catalog),
            EXPECTED_SOURCE_ENABLED_RUNTIME_SURFACE_DIGEST,
            "source-enabled runtime catalog diverged from its frozen contract",
        );
        return;
    }
    if count == EXPECTED_EXTENDED_RUNTIME_FACTORIES {
        assert_eq!(
            retained_runtime_surface_digest(catalog),
            EXPECTED_EXTENDED_RUNTIME_SURFACE_DIGEST,
            "extended runtime catalog diverged from the exhaustive all-features contract",
        );
        return;
    }

    assert_eq!(
        count, EXPECTED_RUNTIME_FACTORIES,
        "selected runtime catalog is neither the frozen standard nor exhaustive profile",
    );

    assert_eq!(
        canonical_runtime_surface_digest(catalog),
        EXPECTED_CURRENT_RUNTIME_SURFACE_DIGEST
    );
    let expected = frozen_runtime_surface();
    let actual = catalog
        .runtime_entries()
        .map(|entry| (id_hex(entry.id.raw()), entry.name.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual, expected,
        "runtime catalog diverged from the standard-linked-dynamic-shape contract"
    );

    let digest = Sha256::digest(RUNTIME_SURFACE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(digest, EXPECTED_RUNTIME_SURFACE_FILE_SHA256);
}

#[cfg(any(feature = "full_source", feature = "full_compiler"))]
fn assert_source_runtime_surface(
    catalog: &FunctionCatalog,
    expected_count: usize,
    expected_digest: &str,
) {
    assert_eq!(catalog.runtime_factory_count(), expected_count);
    #[cfg(feature = "full_compiler")]
    assert_eq!(
        canonical_runtime_surface_digest(catalog),
        EXPECTED_CURRENT_FULL_COMPILER_RUNTIME_SURFACE_DIGEST
    );
    assert_eq!(retained_runtime_surface_digest(catalog), expected_digest);

    let actual = catalog
        .runtime_entries()
        .map(|entry| (id_hex(entry.id.raw()), entry.name.clone()))
        .collect::<BTreeMap<_, _>>();
    for (id, name) in frozen_runtime_surface() {
        assert_eq!(
            actual.get(&id),
            Some(&name),
            "source-capable catalog changed or omitted frozen runtime factory {name}",
        );
    }
}

#[cfg(feature = "full_source")]
fn assert_source_surface(catalog: &FunctionCatalog) {
    assert_pure_text_source_exports(catalog);
    assert_map_source_exports(catalog);
    assert_collection_text_source_exports(catalog);
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
            name: entry.operation.canonical_name.to_string(),
            id_hex: id_hex(entry.operation.id.raw()),
        })
        .collect::<Vec<_>>();
    actual_specializers
        .sort_by(|left, right| (&left.name, &left.id_hex).cmp(&(&right.name, &right.id_hex)));
    assert_eq!(actual_specializers, frozen.full_source_specializers);

    let mut actual_prelude = catalog
        .all_specializers()
        .filter(|entry| {
            catalog
                .exports_for_operation(entry.operation.id)
                .iter()
                .any(|export| export.exposure == FunctionExposure::Prelude)
        })
        .map(|entry| FrozenSpecializer {
            name: entry.operation.canonical_name.to_string(),
            id_hex: id_hex(entry.operation.id.raw()),
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
        assert_runtime_surface(&catalog);
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
        assert_source_runtime_surface(
            &catalog,
            EXPECTED_FULL_SOURCE_RUNTIME_FACTORIES,
            EXPECTED_FULL_SOURCE_RUNTIME_SURFACE_DIGEST,
        );
        assert_source_surface(&catalog);
    });
}

#[cfg(feature = "full_compiler")]
#[test]
fn selected_compiler_closes_the_emitted_factory_surface() {
    with_catalog_test_stack(|| {
        let runtime = mech_stdlib::runtime_catalog();
        assert_runtime_surface(&runtime);
        let catalog = mech_stdlib::source_catalog();
        assert_runtime_catalog_is_a_compiler_subset(&runtime, &catalog);
        assert_source_runtime_surface(
            &catalog,
            EXPECTED_FULL_COMPILER_RUNTIME_FACTORIES,
            EXPECTED_FULL_COMPILER_RUNTIME_SURFACE_DIGEST,
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
