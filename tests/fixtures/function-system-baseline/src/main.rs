use mech_core::matrix::Matrix as MechMatrix;
use mech_core::{
    DecodedInstr, FunctionCatalog, FunctionCatalogBuilder, FunctionCompilerDescriptor,
    FunctionDescriptor, FunctionExposure, MechSet, NativeFunctionCompiler, ParsedProgram, Ref,
    RuntimeFunctionId, Value, ValueKind, hash_str,
};
use mech_engine as _;
use mech_engine::{
    FunctionBinding, FunctionEnvironment, MechProgram, MechProgramConfig, default_function_catalog,
};
use nalgebra::{DMatrix, DVector};
#[cfg(feature = "fixed-specialization-cases")]
use nalgebra::{Matrix2, Vector2};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BASE_COMMIT: &str = "f7768e0c6bbde69d27410be6d1ecacbd08c238c5";
const BYTECODE_VERSION: u8 = 1;
const USAGE: &str = "usage: function-system-baseline (--write <function-system-dir> | --check <function-system-dir> | --write-runtime <function-system-dir> | --check-runtime <function-system-dir> | --write-bytecode <legacy-bytecode-dir>)";
const RUNTIME_SURFACE_FILE: &str = "runtime-factory-surface.json";
const RUNTIME_SURFACE_PROFILE: &str = "standard-linked-dynamic-shape";

type AppResult<T> = Result<T, String>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct SourceSpecializer {
    name: String,
    id_hex: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ModuleExport {
    module: String,
    item: String,
    canonical_name: String,
    id_hex: String,
}

#[derive(Debug, Serialize)]
struct FunctionSurface {
    schema: u32,
    base_commit: &'static str,
    full_source_specializers: Vec<SourceSpecializer>,
    prelude_source_specializers: Vec<SourceSpecializer>,
    module_exports: Vec<ModuleExport>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ValueSpec {
    kind: String,
    shape: Vec<usize>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RuntimeFactory {
    name: String,
    id_hex: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RuntimeFactorySurfaceEntry {
    name: String,
    id_hex: String,
    owner: String,
}

#[derive(Debug, Serialize)]
struct RuntimeFactorySurface {
    schema: u32,
    base_commit: &'static str,
    profile: &'static str,
    runtime_factories: Vec<RuntimeFactorySurfaceEntry>,
}

#[derive(Debug, Serialize)]
struct SpecializationCase {
    name: String,
    operation: String,
    operation_id_hex: String,
    arguments: Vec<ValueSpec>,
    output: ValueSpec,
    runtime_factories: Vec<RuntimeFactory>,
}

#[derive(Debug, Serialize)]
struct SpecializationCases {
    schema: u32,
    base_commit: &'static str,
    bytecode_version: u8,
    cases: Vec<SpecializationCase>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LegacyExpected {
    F64 {
        value: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    MatrixF64 {
        rows: usize,
        cols: usize,
        values: Vec<f64>,
    },
}

#[derive(Debug, Serialize)]
struct LegacyCase {
    name: String,
    file: String,
    source: String,
    expected: LegacyExpected,
    runtime_factory_ids: Vec<RuntimeFactory>,
}

#[derive(Debug, Serialize)]
struct LegacyManifest {
    schema: u32,
    base_commit: &'static str,
    bytecode_version: u8,
    cases: Vec<LegacyCase>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DuplicateRegistration {
    registry: &'static str,
    name: String,
    id: u64,
    count: usize,
}

struct RegistryInventory {
    runtime_names: BTreeMap<u64, BTreeSet<String>>,
    runtime_factories: BTreeMap<u64, (&'static str, usize)>,
    source_names: BTreeMap<u64, BTreeSet<String>>,
    duplicates: Vec<DuplicateRegistration>,
}

impl RegistryInventory {
    fn collect() -> AppResult<Self> {
        let mut runtime_names = BTreeMap::<u64, BTreeSet<String>>::new();
        let mut runtime_counts = BTreeMap::<(u64, String), usize>::new();
        let mut runtime_factories = BTreeMap::<u64, (&'static str, usize)>::new();
        for descriptor in inventory::iter::<FunctionDescriptor> {
            let id = hash_str(descriptor.name);
            runtime_names
                .entry(id)
                .or_default()
                .insert(descriptor.name.to_string());
            let incoming = (descriptor.name, descriptor.ptr as usize);
            if let Some(existing) = runtime_factories.insert(id, incoming)
                && existing != incoming
            {
                return Err(format!(
                    "conflicting legacy runtime factory registration for {}: name/pointer {:?} versus {:?}",
                    format_id(id),
                    existing,
                    incoming,
                ));
            }
            *runtime_counts
                .entry((id, descriptor.name.to_string()))
                .or_default() += 1;
        }

        let mut source_names = BTreeMap::<u64, BTreeSet<String>>::new();
        let mut source_counts = BTreeMap::<(u64, String), usize>::new();
        for descriptor in inventory::iter::<FunctionCompilerDescriptor> {
            let id = hash_str(descriptor.name);
            source_names
                .entry(id)
                .or_default()
                .insert(descriptor.name.to_string());
            *source_counts
                .entry((id, descriptor.name.to_string()))
                .or_default() += 1;
        }

        validate_distinct_name_collisions("runtime factory", &runtime_names)?;
        validate_distinct_name_collisions("source specializer", &source_names)?;

        let mut duplicates = Vec::new();
        for ((id, name), count) in runtime_counts {
            if count > 1 {
                duplicates.push(DuplicateRegistration {
                    registry: "runtime factory",
                    name,
                    id,
                    count,
                });
            }
        }
        for ((id, name), count) in source_counts {
            if count > 1 {
                duplicates.push(DuplicateRegistration {
                    registry: "source specializer",
                    name,
                    id,
                    count,
                });
            }
        }
        duplicates.sort();

        Ok(Self {
            runtime_names,
            runtime_factories,
            source_names,
            duplicates,
        })
    }

    fn print_duplicate_diagnostics(&self) {
        for duplicate in &self.duplicates {
            eprintln!(
                "duplicate {} registration: {} ({}, {}) x{}",
                duplicate.registry,
                duplicate.name,
                format_id(duplicate.id),
                duplicate.id,
                duplicate.count,
            );
        }
    }

    fn runtime_factory(&self, id: u64) -> AppResult<RuntimeFactory> {
        let names = self.runtime_names.get(&id).ok_or_else(|| {
            format!(
                "bytecode references unknown runtime factory ID {}",
                format_id(id)
            )
        })?;
        if names.len() != 1 {
            return Err(format!(
                "runtime factory ID {} maps to conflicting names: {}",
                format_id(id),
                names.iter().cloned().collect::<Vec<_>>().join(", "),
            ));
        }
        Ok(RuntimeFactory {
            name: names.iter().next().expect("one name was required").clone(),
            id_hex: format_id(id),
        })
    }
}

struct CaseInput {
    name: &'static str,
    operation: &'static str,
    arguments: Vec<Value>,
}

struct LegacyInput {
    name: &'static str,
    source: &'static str,
    module: Option<&'static str>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    // This machine is discovered only through inventory in the frozen baseline.
    // Referencing a concrete compiler symbol prevents the native linker from
    // discarding its inventory registrations as an otherwise-unused rlib.
    std::hint::black_box(
        <mech_combinatorics::CombinatoricsNChooseK as NativeFunctionCompiler>::compile,
    );

    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(USAGE.to_string());
    }

    let command = arguments[0].as_str();
    let output = PathBuf::from(&arguments[1]);
    let inventory = RegistryInventory::collect()?;

    match command {
        "--write" => {
            inventory.print_duplicate_diagnostics();
            let (surface, cases) = generate_json_baselines(&inventory)?;
            fs::create_dir_all(&output).map_err(|error| {
                format!(
                    "failed to create baseline directory {}: {error}",
                    output.display()
                )
            })?;
            write_json(&output.join("function-surface.json"), &surface)?;
            write_json(&output.join("specialization-cases.json"), &cases)?;
            Ok(())
        }
        "--check" => {
            let (surface, cases) = generate_json_baselines(&inventory)?;
            check_json(&output.join("function-surface.json"), &surface)?;
            check_json(&output.join("specialization-cases.json"), &cases)?;
            Ok(())
        }
        "--write-runtime" => {
            require_dynamic_runtime_profile()?;
            let surface = generate_runtime_factory_surface(&inventory)?;
            fs::create_dir_all(&output).map_err(|error| {
                format!(
                    "failed to create baseline directory {}: {error}",
                    output.display()
                )
            })?;
            write_json(&output.join(RUNTIME_SURFACE_FILE), &surface)?;
            println!(
                "wrote {} runtime factories for profile {RUNTIME_SURFACE_PROFILE}",
                surface.runtime_factories.len(),
            );
            Ok(())
        }
        "--check-runtime" => {
            require_dynamic_runtime_profile()?;
            let surface = generate_runtime_factory_surface(&inventory)?;
            check_json(&output.join(RUNTIME_SURFACE_FILE), &surface)?;
            println!(
                "checked {} runtime factories for profile {RUNTIME_SURFACE_PROFILE}",
                surface.runtime_factories.len(),
            );
            Ok(())
        }
        "--write-bytecode" => write_legacy_bytecode(&output, &inventory),
        _ => Err(USAGE.to_string()),
    }
}

fn require_dynamic_runtime_profile() -> AppResult<()> {
    if cfg!(feature = "fixed-specialization-cases") {
        Err(
            "the runtime-factory surface must use the standard linked dynamic-shape profile; \
             rerun this command with `--no-default-features`"
                .to_string(),
        )
    } else {
        Ok(())
    }
}

type RuntimeInstaller = fn(&mut FunctionCatalogBuilder) -> mech_core::MResult<()>;

#[derive(Clone)]
struct OwnedRuntimeFactory {
    name: String,
    id: RuntimeFunctionId,
    pointer: usize,
    owner: &'static str,
}

fn generate_runtime_factory_surface(
    inventory: &RegistryInventory,
) -> AppResult<RuntimeFactorySurface> {
    let fragments: [(&str, RuntimeInstaller); 10] = [
        (
            "mech-engine::stdlib",
            mech_engine::stdlib::catalog::install_runtime,
        ),
        ("mech-math", mech_math::install_runtime),
        ("mech-compare", mech_compare::install_runtime),
        ("mech-logic", mech_logic::install_runtime),
        ("mech-range", mech_range::install_runtime),
        ("mech-matrix", mech_matrix::install_runtime),
        ("mech-set", mech_set::install_runtime),
        ("mech-string", mech_string::install_runtime),
        ("mech-stats", mech_stats::install_runtime),
        ("mech-combinatorics", mech_combinatorics::install_runtime),
    ];

    let mut explicit = BTreeMap::<RuntimeFunctionId, OwnedRuntimeFactory>::new();
    for (owner, installer) in fragments {
        let catalog = runtime_fragment(owner, installer)?;
        for entry in catalog.runtime_entries() {
            let incoming = OwnedRuntimeFactory {
                name: entry.name.clone(),
                id: entry.id,
                pointer: entry.factory as usize,
                owner,
            };
            if RuntimeFunctionId::from_name(&incoming.name) != incoming.id {
                return Err(format!(
                    "explicit runtime factory {:?} in {owner} has ID {}, but its name hashes to {}",
                    incoming.name,
                    format_id(incoming.id.raw()),
                    format_id(RuntimeFunctionId::from_name(&incoming.name).raw()),
                ));
            }
            if let Some(existing) = explicit.insert(incoming.id, incoming.clone()) {
                return Err(format!(
                    "runtime factory {} ({}) is owned by both {} and {}",
                    incoming.name,
                    format_id(incoming.id.raw()),
                    existing.owner,
                    incoming.owner,
                ));
            }
        }
    }

    validate_composed_runtime_catalog(&explicit)?;
    validate_legacy_runtime_inventory(&explicit, inventory)?;

    let mut runtime_factories = explicit
        .into_values()
        .map(|entry| RuntimeFactorySurfaceEntry {
            name: entry.name,
            id_hex: format_id(entry.id.raw()),
            owner: entry.owner.to_string(),
        })
        .collect::<Vec<_>>();
    runtime_factories.sort();

    Ok(RuntimeFactorySurface {
        schema: 1,
        base_commit: BASE_COMMIT,
        profile: RUNTIME_SURFACE_PROFILE,
        runtime_factories,
    })
}

fn runtime_fragment(owner: &str, installer: RuntimeInstaller) -> AppResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    installer(&mut builder).map_err(|error| {
        format!(
            "failed to install explicit runtime catalog fragment {owner}: {}",
            error.full_chain_message(),
        )
    })?;
    builder.build().map_err(|error| {
        format!(
            "failed to build explicit runtime catalog fragment {owner}: {}",
            error.full_chain_message(),
        )
    })
}

fn validate_composed_runtime_catalog(
    explicit: &BTreeMap<RuntimeFunctionId, OwnedRuntimeFactory>,
) -> AppResult<()> {
    let composed = mech_engine::default_function_catalog();
    if composed.runtime_factory_count() != explicit.len() {
        return Err(format!(
            "the composed standard catalog has {} runtime factories, but its explicit fragments have {}",
            composed.runtime_factory_count(),
            explicit.len(),
        ));
    }

    for entry in composed.runtime_entries() {
        let owned = explicit.get(&entry.id).ok_or_else(|| {
            format!(
                "composed standard catalog contains factory {} ({}) that has no owning fragment",
                entry.name,
                format_id(entry.id.raw()),
            )
        })?;
        if entry.name != owned.name || entry.factory as usize != owned.pointer {
            return Err(format!(
                "composed standard catalog factory {} ({}) does not match owning fragment {} exactly",
                entry.name,
                format_id(entry.id.raw()),
                owned.owner,
            ));
        }
    }
    Ok(())
}

fn validate_legacy_runtime_inventory(
    explicit: &BTreeMap<RuntimeFunctionId, OwnedRuntimeFactory>,
    inventory: &RegistryInventory,
) -> AppResult<()> {
    let mut failures = Vec::new();

    for (id, entry) in explicit {
        match inventory.runtime_factories.get(&id.raw()) {
            None => failures.push(format!(
                "explicit-only {} ({}) owned by {}",
                entry.name,
                format_id(id.raw()),
                entry.owner,
            )),
            Some((legacy_name, legacy_pointer))
                if *legacy_name != entry.name || *legacy_pointer != entry.pointer =>
            {
                failures.push(format!(
                    "mismatch for {} ({}): explicit owner {}, legacy name {:?}, factory pointers {:#x} versus {:#x}",
                    entry.name,
                    format_id(id.raw()),
                    entry.owner,
                    legacy_name,
                    entry.pointer,
                    legacy_pointer,
                ));
            }
            Some(_) => {}
        }
    }

    for (id, (name, _)) in &inventory.runtime_factories {
        if !explicit.contains_key(&RuntimeFunctionId::from_raw(*id)) {
            failures.push(format!("legacy-only {name} ({})", format_id(*id)));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        const DIAGNOSTIC_LIMIT: usize = 25;
        let omitted = failures.len().saturating_sub(DIAGNOSTIC_LIMIT);
        failures.truncate(DIAGNOSTIC_LIMIT);
        let mut message = format!(
            "explicit runtime fragments do not exactly match the legacy FunctionDescriptor inventory ({} explicit, {} legacy):\n- {}",
            explicit.len(),
            inventory.runtime_factories.len(),
            failures.join("\n- "),
        );
        if omitted > 0 {
            message.push_str(&format!("\n- ... and {omitted} more mismatch(es)"));
        }
        Err(message)
    }
}

fn validate_distinct_name_collisions(
    registry: &str,
    names_by_id: &BTreeMap<u64, BTreeSet<String>>,
) -> AppResult<()> {
    let collisions = names_by_id
        .iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(id, names)| {
            format!(
                "{} => {}",
                format_id(*id),
                names.iter().cloned().collect::<Vec<_>>().join(", "),
            )
        })
        .collect::<Vec<_>>();

    if collisions.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "distinct-name hash collision(s) in {registry} registry: {}",
            collisions.join("; "),
        ))
    }
}

fn generate_json_baselines(
    inventory: &RegistryInventory,
) -> AppResult<(FunctionSurface, SpecializationCases)> {
    let catalog = default_function_catalog();
    let program = MechProgram::new(MechProgramConfig::default());
    if !Arc::ptr_eq(&catalog, program.function_catalog()) {
        return Err("fresh standard program did not retain the composed standard catalog".into());
    }
    let function_environment = program
        .interpreter()
        .state
        .borrow()
        .function_environment
        .clone();
    validate_default_function_environment(&catalog, &function_environment)?;

    let full_source_specializers = effective_source_specializers(&catalog);
    validate_effective_source_names(&full_source_specializers, &inventory.source_names)?;

    let prelude_source_specializers =
        prelude_source_specializers(&catalog, &function_environment)?;

    let module_exports = collect_module_exports(&catalog)?;
    let cases = collect_specialization_cases(&catalog, inventory)?;

    Ok((
        FunctionSurface {
            schema: 1,
            base_commit: BASE_COMMIT,
            full_source_specializers,
            prelude_source_specializers,
            module_exports,
        },
        SpecializationCases {
            schema: 1,
            base_commit: BASE_COMMIT,
            bytecode_version: BYTECODE_VERSION,
            cases,
        },
    ))
}

fn effective_source_specializers(catalog: &FunctionCatalog) -> Vec<SourceSpecializer> {
    let mut specializers = catalog
        .all_specializers()
        .map(|entry| SourceSpecializer {
            name: entry.canonical_name.clone(),
            id_hex: format_id(entry.operation.raw()),
        })
        .collect::<Vec<_>>();
    specializers
        .sort_by(|left, right| (&left.name, &left.id_hex).cmp(&(&right.name, &right.id_hex)));
    specializers
}

fn prelude_source_specializers(
    catalog: &FunctionCatalog,
    environment: &FunctionEnvironment,
) -> AppResult<Vec<SourceSpecializer>> {
    let mut specializers = Vec::new();
    for entry in catalog.all_specializers().filter(|entry| {
        catalog
            .exports_for_operation(entry.operation)
            .iter()
            .any(|export| export.exposure == FunctionExposure::Prelude)
    }) {
        if environment.resolve_name(&entry.canonical_name)
            != Some(FunctionBinding::CatalogOperation(entry.operation))
        {
            return Err(format!(
                "fresh standard FunctionEnvironment did not bind Prelude operation {} ({})",
                entry.canonical_name,
                format_id(entry.operation.raw()),
            ));
        }
        if !environment.operation_is_enabled(entry.operation) {
            return Err(format!(
                "fresh standard FunctionEnvironment did not enable Prelude operation {} ({})",
                entry.canonical_name,
                format_id(entry.operation.raw()),
            ));
        }
        specializers.push(SourceSpecializer {
            name: entry.canonical_name.clone(),
            id_hex: format_id(entry.operation.raw()),
        });
    }
    specializers
        .sort_by(|left, right| (&left.name, &left.id_hex).cmp(&(&right.name, &right.id_hex)));
    Ok(specializers)
}

fn validate_default_function_environment(
    catalog: &FunctionCatalog,
    environment: &FunctionEnvironment,
) -> AppResult<()> {
    for intrinsic in catalog.intrinsic_specializer_entries() {
        if !environment.operation_is_enabled(intrinsic.operation) {
            return Err(format!(
                "fresh standard FunctionEnvironment did not enable intrinsic {} ({})",
                intrinsic.canonical_name,
                format_id(intrinsic.operation.raw()),
            ));
        }
        if environment.resolve_name(&intrinsic.canonical_name).is_some() {
            return Err(format!(
                "fresh standard FunctionEnvironment exposed intrinsic {} as a named function",
                intrinsic.canonical_name,
            ));
        }
    }

    for export in catalog.all_exports() {
        let operation_exports = catalog.exports_for_operation(export.operation);
        let enabled_by_default = operation_exports.iter().any(|candidate| {
            matches!(
                candidate.exposure,
                FunctionExposure::Internal | FunctionExposure::Prelude
            )
        });
        if environment.operation_is_enabled(export.operation) != enabled_by_default {
            return Err(format!(
                "fresh standard FunctionEnvironment enabled-state mismatch for {} ({})",
                export.canonical_name,
                format_id(export.operation.raw()),
            ));
        }

        let named_by_default = operation_exports.iter().any(|candidate| {
            candidate.exposure == FunctionExposure::Prelude
                && candidate.canonical_name == export.canonical_name
        });
        let binding = environment.resolve_name(&export.canonical_name);
        if named_by_default {
            if binding != Some(FunctionBinding::CatalogOperation(export.operation)) {
                return Err(format!(
                    "fresh standard FunctionEnvironment binding mismatch for Prelude export {}",
                    export.canonical_name,
                ));
            }
        } else if binding.is_some() {
            return Err(format!(
                "fresh standard FunctionEnvironment exposed non-Prelude export {} by name",
                export.canonical_name,
            ));
        }
    }

    Ok(())
}

fn validate_effective_source_names(
    effective: &[SourceSpecializer],
    raw_names: &BTreeMap<u64, BTreeSet<String>>,
) -> AppResult<()> {
    for specializer in effective {
        let id = hash_str(&specializer.name);
        let Some(names) = raw_names.get(&id) else {
            return Err(format!(
                "effective source specializer {} ({}) has no raw descriptor",
                specializer.name, specializer.id_hex,
            ));
        };
        if !names.contains(&specializer.name) {
            return Err(format!(
                "effective source specializer {} ({}) does not match its raw descriptor",
                specializer.name, specializer.id_hex,
            ));
        }
    }
    Ok(())
}

fn collect_module_exports(catalog: &FunctionCatalog) -> AppResult<Vec<ModuleExport>> {
    let mut exports = catalog
        .all_exports()
        .filter_map(|export| match (&export.module, &export.item) {
            (Some(module), Some(item)) => Some(ModuleExport {
                module: module.clone(),
                item: item.clone(),
                canonical_name: export.canonical_name.clone(),
                id_hex: format_id(export.operation.raw()),
            }),
            (None, None) => None,
            _ => unreachable!("validated catalog exports have complete module/item pairs"),
        })
        .collect::<Vec<_>>();

    exports.sort_by(|left, right| {
        (&left.module, &left.item, &left.canonical_name).cmp(&(
            &right.module,
            &right.item,
            &right.canonical_name,
        ))
    });
    Ok(exports)
}

fn collect_specialization_cases(
    catalog: &FunctionCatalog,
    inventory: &RegistryInventory,
) -> AppResult<Vec<SpecializationCase>> {
    let mut inputs = vec![
        CaseInput {
            name: "math_add_f64_f64",
            operation: "math/add",
            arguments: vec![f64_value(2.0), f64_value(3.0)],
        },
        CaseInput {
            name: "math_add_matrixd_f64",
            operation: "math/add",
            arguments: vec![dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2), f64_value(2.0)],
        },
        CaseInput {
            name: "math_add_matrixd_matrixd",
            operation: "math/add",
            arguments: vec![
                dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2),
                dynamic_matrix(&[5.0, 6.0, 7.0, 8.0], 2, 2),
            ],
        },
        CaseInput {
            name: "math_add_f64_matrixd",
            operation: "math/add",
            arguments: vec![f64_value(2.0), dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2)],
        },
        CaseInput {
            name: "math_add_assign_f64",
            operation: "math/add-assign",
            arguments: vec![mutable_value(f64_value(10.0)), f64_value(20.0)],
        },
        CaseInput {
            name: "compare_lt_f64",
            operation: "compare/lt",
            arguments: vec![f64_value(1.0), f64_value(2.0)],
        },
        CaseInput {
            name: "logic_and_bool",
            operation: "logic/and",
            arguments: vec![bool_value(true), bool_value(false)],
        },
        CaseInput {
            name: "range_inclusive_f64",
            operation: "range/inclusive",
            arguments: vec![f64_value(1.0), f64_value(3.0)],
        },
        CaseInput {
            name: "range_inclusive_increment_f64",
            operation: "range/inclusive-increment",
            arguments: vec![f64_value(1.0), f64_value(2.0), f64_value(5.0)],
        },
        CaseInput {
            name: "matrix_solve_matrixd_vectord_f64",
            operation: "matrix/solve",
            arguments: vec![
                dynamic_matrix(&[4.0, 1.0, 2.0, 3.0], 2, 2),
                dynamic_vector(&[1.0, 2.0]),
            ],
        },
        CaseInput {
            name: "set_union_f64",
            operation: "set/union",
            arguments: vec![f64_set(&[1.0, 2.0]), f64_set(&[2.0, 3.0])],
        },
        CaseInput {
            name: "string_concat",
            operation: "string/concat",
            arguments: vec![string_value("ab"), string_value("cd")],
        },
        CaseInput {
            name: "math_cos_f64",
            operation: "math/cos",
            arguments: vec![f64_value(0.0)],
        },
        CaseInput {
            name: "stats_sum_column_f64",
            operation: "stats/sum/column",
            arguments: vec![dynamic_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2)],
        },
        CaseInput {
            name: "combinatorics_n_choose_k_f64",
            operation: "combinatorics/n-choose-k",
            arguments: vec![f64_value(5.0), f64_value(2.0)],
        },
    ];
    #[cfg(feature = "fixed-specialization-cases")]
    inputs.extend([
        CaseInput {
            name: "math_add_vector2_f64",
            operation: "math/add",
            arguments: vec![fixed_vector2(1.0, 2.0), f64_value(3.0)],
        },
        CaseInput {
            name: "matrix_matmul_matrix2_f64",
            operation: "matrix/matmul",
            arguments: vec![
                fixed_matrix2(1.0, 2.0, 3.0, 4.0),
                fixed_matrix2(5.0, 6.0, 7.0, 8.0),
            ],
        },
    ]);
    inputs.sort_by_key(|input| input.name);

    let mut cases = Vec::with_capacity(inputs.len());
    let mut failures = Vec::new();
    for input in inputs {
        match compile_specialization_case(catalog, inventory, input) {
            Ok(case) => cases.push(case),
            Err(error) => failures.push(error),
        }
    }

    #[cfg(not(feature = "fixed-specialization-cases"))]
    failures.push(
        "the required Matrix2<f64> and Vector2<f64> cases need the fixture feature \
         `fixed-specialization-cases`; use the documented default fixture configuration or \
         enable that feature explicitly"
            .to_string(),
    );

    if failures.is_empty() {
        cases.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(cases)
    } else {
        Err(format!(
            "specialization baseline generation is blocked:\n- {}",
            failures.join("\n- "),
        ))
    }
}

fn compile_specialization_case(
    catalog: &FunctionCatalog,
    inventory: &RegistryInventory,
    input: CaseInput,
) -> AppResult<SpecializationCase> {
    let operation_id = hash_str(input.operation);
    let specializer = catalog
        .specializer(mech_core::OperationId::from_raw(operation_id))
        .ok_or_else(|| {
            format!(
                "specialization case {} cannot find source operation {} ({})",
                input.name,
                input.operation,
                format_id(operation_id),
            )
        })?;
    let argument_specs = input
        .arguments
        .iter()
        .map(value_spec)
        .collect::<AppResult<Vec<_>>>()?;
    let function = specializer
        .specializer
        .specialize(&input.arguments)
        .map_err(|error| {
        format!(
            "specialization case {} failed to compile {}: {}",
            input.name,
            input.operation,
            error.full_chain_message(),
        )
    })?;
    let output = value_spec(&function.out())?;

    let mut context = mech_bytecode::CompileCtx::new();
    function.compile(&mut context).map_err(|error| {
        format!(
            "specialization case {} failed to emit bytecode: {}",
            input.name,
            error.full_chain_message(),
        )
    })?;
    let bytecode = context.compile().map_err(|error| {
        format!(
            "specialization case {} failed to assemble bytecode: {}",
            input.name,
            error.full_chain_message(),
        )
    })?;
    let parsed = ParsedProgram::from_bytes(&bytecode).map_err(|error| {
        format!(
            "specialization case {} emitted invalid bytecode: {}",
            input.name,
            error.full_chain_message(),
        )
    })?;
    validate_bytecode_version(&parsed, input.name)?;
    let mut runtime_factories = runtime_factories_from_program(&parsed, inventory, input.name)?;
    if runtime_factories.len() != 1 {
        return Err(format!(
            "specialization case {} emitted {} operation instructions; expected exactly one",
            input.name,
            runtime_factories.len(),
        ));
    }
    runtime_factories.sort();

    Ok(SpecializationCase {
        name: input.name.to_string(),
        operation: input.operation.to_string(),
        operation_id_hex: format_id(operation_id),
        arguments: argument_specs,
        output,
        runtime_factories,
    })
}

fn runtime_factories_from_program(
    parsed: &ParsedProgram,
    inventory: &RegistryInventory,
    case_name: &str,
) -> AppResult<Vec<RuntimeFactory>> {
    let mut ids = Vec::new();
    for instruction in &parsed.instrs {
        match instruction {
            DecodedInstr::NullOp { fxn_id, .. }
            | DecodedInstr::UnOp { fxn_id, .. }
            | DecodedInstr::BinOp { fxn_id, .. }
            | DecodedInstr::TernOp { fxn_id, .. }
            | DecodedInstr::QuadOp { fxn_id, .. }
            | DecodedInstr::VarArg { fxn_id, .. } => ids.push(*fxn_id),
            DecodedInstr::ConstLoad { .. } | DecodedInstr::Ret { .. } => {}
            DecodedInstr::Unknown { opcode, .. } => {
                return Err(format!(
                    "case {case_name} contains unknown bytecode opcode {opcode:#04x}",
                ));
            }
        }
    }
    if ids.is_empty() {
        return Err(format!("case {case_name} emitted no operation instruction",));
    }
    ids.into_iter()
        .map(|id| {
            inventory
                .runtime_factory(id)
                .map_err(|error| format!("case {case_name}: {error}"))
        })
        .collect()
}

fn validate_bytecode_version(parsed: &ParsedProgram, case_name: &str) -> AppResult<()> {
    if parsed.header.version == BYTECODE_VERSION {
        Ok(())
    } else {
        Err(format!(
            "case {case_name} emitted bytecode version {}; expected {}",
            parsed.header.version, BYTECODE_VERSION,
        ))
    }
}

fn value_spec(value: &Value) -> AppResult<ValueSpec> {
    match value {
        Value::F64(_) => Ok(ValueSpec {
            kind: "f64".to_string(),
            shape: Vec::new(),
        }),
        Value::Bool(_) => Ok(ValueSpec {
            kind: "bool".to_string(),
            shape: Vec::new(),
        }),
        Value::String(_) => Ok(ValueSpec {
            kind: "string".to_string(),
            shape: Vec::new(),
        }),
        Value::MatrixF64(matrix) => Ok(ValueSpec {
            kind: "f64".to_string(),
            shape: matrix.shape(),
        }),
        Value::Set(set) => {
            let set = set.borrow();
            let kind = match &set.kind {
                ValueKind::F64 => "{f64}".to_string(),
                other => {
                    return Err(format!(
                        "unsupported set element kind in baseline case: {other}",
                    ));
                }
            };
            Ok(ValueSpec {
                kind,
                shape: Vec::new(),
            })
        }
        Value::MutableReference(value) => value_spec(&value.borrow()),
        Value::Typed(value, _) => value_spec(value),
        other => Err(format!(
            "unsupported value kind in baseline case: {}",
            other.kind(),
        )),
    }
}

fn write_legacy_bytecode(output: &Path, inventory: &RegistryInventory) -> AppResult<()> {
    let mut inputs = vec![
        LegacyInput {
            name: "scalar-add",
            source: "1.0 + 2.0",
            module: None,
        },
        LegacyInput {
            name: "matrix-scalar-add",
            source: "[1.0 2.0] + 1.0",
            module: None,
        },
        LegacyInput {
            name: "string-concat",
            source: "\"a\" + \"bc\"",
            module: None,
        },
        LegacyInput {
            name: "range-inclusive",
            source: "1.0..=3.0",
            module: None,
        },
        LegacyInput {
            name: "math-cos",
            source: "math/cos(0.0)",
            module: Some("math"),
        },
    ];
    inputs.sort_by_key(|input| input.name);

    let mut generated = Vec::<(LegacyCase, Vec<u8>)>::with_capacity(inputs.len());
    for input in inputs {
        let mut program = MechProgram::new(MechProgramConfig::default());
        if let Some(module) = input.module {
            program.load_function_module(module).map_err(|error| {
                format!(
                    "legacy case {} failed to load module {module:?}: {}",
                    input.name,
                    error.full_chain_message(),
                )
            })?;
        }
        let value = program.run_string(input.source).map_err(|error| {
            format!(
                "legacy case {} failed to run source {:?}: {}",
                input.name,
                input.source,
                error.full_chain_message(),
            )
        })?;
        let expected = legacy_expected(&value)?;
        let bytecode = program.compile_bytecode().map_err(|error| {
            format!(
                "legacy case {} failed to compile bytecode: {}",
                input.name,
                error.full_chain_message(),
            )
        })?;
        let parsed = ParsedProgram::from_bytes(&bytecode).map_err(|error| {
            format!(
                "legacy case {} emitted invalid bytecode: {}",
                input.name,
                error.full_chain_message(),
            )
        })?;
        validate_bytecode_version(&parsed, input.name)?;
        let mut runtime_factory_ids =
            runtime_factories_from_program(&parsed, inventory, input.name)?;
        runtime_factory_ids.sort();
        let file = format!("{}.mecb", input.name);
        generated.push((
            LegacyCase {
                name: input.name.to_string(),
                file,
                source: input.source.to_string(),
                expected,
                runtime_factory_ids,
            },
            bytecode,
        ));
    }

    generated.sort_by(|left, right| left.0.name.cmp(&right.0.name));
    fs::create_dir_all(output).map_err(|error| {
        format!(
            "failed to create legacy-bytecode directory {}: {error}",
            output.display(),
        )
    })?;
    for (case, bytecode) in &generated {
        fs::write(output.join(&case.file), bytecode).map_err(|error| {
            format!(
                "failed to write legacy bytecode file {}: {error}",
                output.join(&case.file).display(),
            )
        })?;
    }
    let manifest = LegacyManifest {
        schema: 1,
        base_commit: BASE_COMMIT,
        bytecode_version: BYTECODE_VERSION,
        cases: generated.into_iter().map(|(case, _)| case).collect(),
    };
    write_json(&output.join("manifest.json"), &manifest)
}

fn legacy_expected(value: &Value) -> AppResult<LegacyExpected> {
    match value {
        Value::F64(value) => Ok(LegacyExpected::F64 {
            value: *value.borrow(),
        }),
        Value::Bool(value) => Ok(LegacyExpected::Bool {
            value: *value.borrow(),
        }),
        Value::String(value) => Ok(LegacyExpected::String {
            value: value.borrow().clone(),
        }),
        Value::MatrixF64(matrix) => Ok(LegacyExpected::MatrixF64 {
            rows: matrix.rows(),
            cols: matrix.cols(),
            values: matrix.as_vec(),
        }),
        other => Err(format!(
            "unsupported legacy expected value kind: {}",
            other.kind(),
        )),
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    let contents = json_contents(value)?;
    fs::write(path, contents)
        .map_err(|error| format!("failed to write JSON baseline {}: {error}", path.display()))
}

fn check_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    let expected = fs::read(path).map_err(|error| {
        format!(
            "failed to read committed baseline {}: {error}",
            path.display()
        )
    })?;
    let actual = json_contents(value)?.into_bytes();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "committed baseline {} does not match the current function system; regenerate it with --write and review the compatibility change",
            path.display(),
        ))
    }
}

fn json_contents(value: &impl Serialize) -> AppResult<String> {
    let mut contents = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize JSON baseline: {error}"))?;
    while contents.ends_with('\n') {
        contents.pop();
    }
    contents.push('\n');
    Ok(contents)
}

fn format_id(id: u64) -> String {
    format!("{id:016x}")
}

fn f64_value(value: f64) -> Value {
    Value::F64(Ref::new(value))
}

fn bool_value(value: bool) -> Value {
    Value::Bool(Ref::new(value))
}

fn string_value(value: &str) -> Value {
    Value::String(Ref::new(value.to_string()))
}

fn mutable_value(value: Value) -> Value {
    Value::MutableReference(Ref::new(value))
}

fn dynamic_matrix(values: &[f64], rows: usize, cols: usize) -> Value {
    Value::MatrixF64(MechMatrix::DMatrix(Ref::new(DMatrix::from_row_slice(
        rows, cols, values,
    ))))
}

fn dynamic_vector(values: &[f64]) -> Value {
    Value::MatrixF64(MechMatrix::DVector(Ref::new(DVector::from_column_slice(
        values,
    ))))
}

#[cfg(feature = "fixed-specialization-cases")]
fn fixed_matrix2(a11: f64, a12: f64, a21: f64, a22: f64) -> Value {
    Value::MatrixF64(MechMatrix::Matrix2(Ref::new(Matrix2::new(
        a11, a12, a21, a22,
    ))))
}

#[cfg(feature = "fixed-specialization-cases")]
fn fixed_vector2(first: f64, second: f64) -> Value {
    Value::MatrixF64(MechMatrix::Vector2(Ref::new(Vector2::new(first, second))))
}

fn f64_set(values: &[f64]) -> Value {
    Value::Set(Ref::new(MechSet::from_vec(
        values.iter().copied().map(f64_value).collect(),
    )))
}
