use mech_core::matrix::Matrix as MechMatrix;
use mech_core::{
    ApplicationRequirement, BytecodeCompilerContext, EncodedConstant, FunctionCatalog,
    FunctionCatalogBuilder, FunctionExposure, MResult, MechSet, OperationId, Ref, Register,
    RuntimeFunctionId, LegacyValue, ValueKind, hash_str,
};
use mech_engine as _;
use mech_engine::{FunctionBinding, FunctionEnvironment};
use nalgebra::{DMatrix, DVector};
#[cfg(feature = "fixed-specialization-cases")]
use nalgebra::{Matrix2, Vector2};
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BASE_COMMIT: &str = "f7768e0c6bbde69d27410be6d1ecacbd08c238c5";
const USAGE: &str = "usage: function-system-baseline (--write <function-system-dir> | --check <function-system-dir> | --write-runtime <function-system-dir> | --check-runtime <function-system-dir>)";
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
    cases: Vec<SpecializationCase>,
}

fn runtime_factory(catalog: &FunctionCatalog, id: u64) -> AppResult<RuntimeFactory> {
    let runtime_id = RuntimeFunctionId::from_raw(id);
    let entry = catalog.runtime_entry(runtime_id).ok_or_else(|| {
        format!(
            "specialization references unknown catalog runtime factory ID {}",
            format_id(id)
        )
    })?;
    Ok(RuntimeFactory {
        name: entry.name.clone(),
        id_hex: format_id(id),
    })
}

struct CaseInput {
    name: &'static str,
    operation: &'static str,
    arguments: Vec<LegacyValue>,
}

#[derive(Default)]
struct RuntimeLinkageRecorder {
    registers: BTreeMap<usize, Register>,
    next_register: Register,
    runtime_ids: Vec<RuntimeFunctionId>,
    unexpected_emissions: Vec<&'static str>,
}

impl RuntimeLinkageRecorder {
    fn record_runtime(&mut self, function: u64) {
        let runtime_id = RuntimeFunctionId::from_raw(function);
        self.runtime_ids.push(runtime_id);
    }

    fn resolve(self, catalog: &FunctionCatalog, case_name: &str) -> AppResult<Vec<RuntimeFactory>> {
        if !self.unexpected_emissions.is_empty() {
            return Err(format!(
                "case {case_name} emitted non-runtime linkage operations: {}",
                self.unexpected_emissions.join(", "),
            ));
        }
        if self.runtime_ids.len() != 1 {
            return Err(format!(
                "specialization case {case_name} selected {} runtime factories; expected exactly one",
                self.runtime_ids.len(),
            ));
        }

        self.runtime_ids
            .into_iter()
            .map(|id| {
                runtime_factory(catalog, id.raw())
                    .map_err(|error| format!("case {case_name}: {error}"))
            })
            .collect()
    }
}

impl BytecodeCompilerContext for RuntimeLinkageRecorder {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        if let Some(register) = self.registers.get(&pointer) {
            return (*register, false);
        }
        let register = self.next_register;
        self.next_register += 1;
        self.registers.insert(pointer, register);
        (register, false)
    }

    fn intern_constant(&mut self, _constant: EncodedConstant) -> MResult<u32> {
        self.unexpected_emissions.push("constant");
        Ok(0)
    }

    fn define_symbol(
        &mut self,
        _pointer: usize,
        _register: Register,
        _name: &str,
        _mutable: bool,
    ) -> MResult<()> {
        self.unexpected_emissions.push("symbol");
        Ok(())
    }

    fn intern_requirement(&mut self, _requirement: ApplicationRequirement) -> MResult<u32> {
        self.unexpected_emissions.push("application requirement");
        Ok(0)
    }

    fn emit_const_load(&mut self, _destination: Register, _constant: u32) {
        self.unexpected_emissions.push("constant load");
    }

    fn emit_composite_pack(
        &mut self,
        _destination: Register,
        _template: u32,
        _children: Vec<Register>,
    ) {
        self.unexpected_emissions.push("composite pack");
    }

    fn emit_nullop(&mut self, function: u64, _destination: Register) {
        self.record_runtime(function);
    }

    fn emit_unop(&mut self, function: u64, _destination: Register, _source: Register) {
        self.record_runtime(function);
    }

    fn emit_binop(
        &mut self,
        function: u64,
        _destination: Register,
        _lhs: Register,
        _rhs: Register,
    ) {
        self.record_runtime(function);
    }

    fn emit_ternop(
        &mut self,
        function: u64,
        _destination: Register,
        _a: Register,
        _b: Register,
        _c: Register,
    ) {
        self.record_runtime(function);
    }

    fn emit_quadop(
        &mut self,
        function: u64,
        _destination: Register,
        _a: Register,
        _b: Register,
        _c: Register,
        _d: Register,
    ) {
        self.record_runtime(function);
    }

    fn emit_varop(&mut self, function: u64, _destination: Register, _arguments: Vec<Register>) {
        self.record_runtime(function);
    }

    fn emit_host_call(
        &mut self,
        _requirement: u32,
        _destination: Register,
        _arguments: Vec<Register>,
    ) {
        self.unexpected_emissions.push("host call");
    }

    fn emit_resource_read(&mut self, _requirement: u32, _destination: Register) {
        self.unexpected_emissions.push("resource read");
    }

    fn emit_resource_write(
        &mut self,
        _requirement: u32,
        _destination: Register,
        _source: Register,
    ) {
        self.unexpected_emissions.push("resource write");
    }

    fn emit_resource_send(&mut self, _requirement: u32, _destination: Register, _source: Register) {
        self.unexpected_emissions.push("resource send");
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(USAGE.to_string());
    }

    let command = arguments[0].as_str();
    let output = PathBuf::from(&arguments[1]);
    let catalog = mech_stdlib::source_catalog();

    match command {
        "--write" => {
            let (surface, cases) = generate_json_baselines(&catalog)?;
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
            let (surface, cases) = generate_json_baselines(&catalog)?;
            check_json(&output.join("function-surface.json"), &surface)?;
            check_json(&output.join("specialization-cases.json"), &cases)?;
            Ok(())
        }
        "--write-runtime" => {
            require_dynamic_runtime_profile()?;
            let surface = generate_runtime_factory_surface()?;
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
            let surface = generate_runtime_factory_surface()?;
            check_json(&output.join(RUNTIME_SURFACE_FILE), &surface)?;
            println!(
                "checked {} runtime factories for profile {RUNTIME_SURFACE_PROFILE}",
                surface.runtime_factories.len(),
            );
            Ok(())
        }
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
    owner: &'static str,
}

fn generate_runtime_factory_surface() -> AppResult<RuntimeFactorySurface> {
    let fragments: [(&str, RuntimeInstaller); 11] = [
        (
            "mech-engine::stdlib",
            mech_engine::install_intrinsic_runtime,
        ),
        ("mech-engine::ekf", mech_engine::install_ekf_runtime),
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
    let composed = mech_stdlib::runtime_catalog();
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
        if entry.name != owned.name {
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

fn generate_json_baselines(
    catalog: &Arc<FunctionCatalog>,
) -> AppResult<(FunctionSurface, SpecializationCases)> {
    let function_environment = FunctionEnvironment::from_catalog_defaults(catalog)
        .map_err(|error| format!("failed to derive the default FunctionEnvironment: {error:?}"))?;
    validate_default_function_environment(catalog, &function_environment)?;

    let full_source_specializers = effective_source_specializers(catalog);

    let prelude_source_specializers = prelude_source_specializers(catalog, &function_environment)?;

    let module_exports = collect_module_exports(catalog)?;
    let cases = collect_specialization_cases(catalog)?;

    Ok((
        FunctionSurface {
            schema: 1,
            base_commit: BASE_COMMIT,
            full_source_specializers,
            prelude_source_specializers,
            module_exports,
        },
        SpecializationCases {
            schema: 2,
            base_commit: BASE_COMMIT,
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
        if environment
            .resolve_name(&intrinsic.canonical_name)
            .is_some()
        {
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

    let expected = FunctionEnvironment::from_catalog_defaults(catalog)
        .map_err(|error| format!("failed to derive the default FunctionEnvironment: {error:?}"))?;
    if environment != &expected {
        return Err(
            "fresh standard FunctionEnvironment contains unexpected bindings or enabled operations"
                .to_string(),
        );
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

fn collect_specialization_cases(catalog: &FunctionCatalog) -> AppResult<Vec<SpecializationCase>> {
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
        match specialize_case(catalog, input) {
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

fn specialize_case(catalog: &FunctionCatalog, input: CaseInput) -> AppResult<SpecializationCase> {
    let raw_operation_id = hash_str(input.operation);
    let operation_id = OperationId::from_raw(raw_operation_id);
    let specializer = catalog.specializer(operation_id).ok_or_else(|| {
        format!(
            "specialization case {} cannot find source operation {} ({})",
            input.name,
            input.operation,
            format_id(raw_operation_id),
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

    let mut linkage = RuntimeLinkageRecorder::default();
    function.compile(&mut linkage).map_err(|error| {
        format!(
            "specialization case {} failed to report runtime linkage: {}",
            input.name,
            error.full_chain_message(),
        )
    })?;
    let mut runtime_factories = linkage.resolve(catalog, input.name)?;
    runtime_factories.sort();

    Ok(SpecializationCase {
        name: input.name.to_string(),
        operation: input.operation.to_string(),
        operation_id_hex: format_id(raw_operation_id),
        arguments: argument_specs,
        output,
        runtime_factories,
    })
}

fn value_spec(value: &LegacyValue) -> AppResult<ValueSpec> {
    match value {
        LegacyValue::F64(_) => Ok(ValueSpec {
            kind: "f64".to_string(),
            shape: Vec::new(),
        }),
        LegacyValue::Bool(_) => Ok(ValueSpec {
            kind: "bool".to_string(),
            shape: Vec::new(),
        }),
        LegacyValue::String(_) => Ok(ValueSpec {
            kind: "string".to_string(),
            shape: Vec::new(),
        }),
        LegacyValue::MatrixF64(matrix) => Ok(ValueSpec {
            kind: "f64".to_string(),
            shape: matrix.shape(),
        }),
        LegacyValue::Set(set) => {
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
        LegacyValue::MutableReference(value) => value_spec(&value.borrow()),
        LegacyValue::Typed(value, _) => value_spec(value),
        other => Err(format!(
            "unsupported value kind in baseline case: {}",
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

fn f64_value(value: f64) -> LegacyValue {
    LegacyValue::F64(Ref::new(value))
}

fn bool_value(value: bool) -> LegacyValue {
    LegacyValue::Bool(Ref::new(value))
}

fn string_value(value: &str) -> LegacyValue {
    LegacyValue::String(Ref::new(value.to_string()))
}

fn mutable_value(value: LegacyValue) -> LegacyValue {
    LegacyValue::MutableReference(Ref::new(value))
}

fn dynamic_matrix(values: &[f64], rows: usize, cols: usize) -> LegacyValue {
    LegacyValue::MatrixF64(MechMatrix::DMatrix(Ref::new(DMatrix::from_row_slice(
        rows, cols, values,
    ))))
}

fn dynamic_vector(values: &[f64]) -> LegacyValue {
    LegacyValue::MatrixF64(MechMatrix::DVector(Ref::new(DVector::from_column_slice(
        values,
    ))))
}

#[cfg(feature = "fixed-specialization-cases")]
fn fixed_matrix2(a11: f64, a12: f64, a21: f64, a22: f64) -> LegacyValue {
    LegacyValue::MatrixF64(MechMatrix::Matrix2(Ref::new(Matrix2::new(
        a11, a12, a21, a22,
    ))))
}

#[cfg(feature = "fixed-specialization-cases")]
fn fixed_vector2(first: f64, second: f64) -> LegacyValue {
    LegacyValue::MatrixF64(MechMatrix::Vector2(Ref::new(Vector2::new(first, second))))
}

fn f64_set(values: &[f64]) -> LegacyValue {
    LegacyValue::Set(Ref::new(MechSet::from_vec(
        values.iter().copied().map(f64_value).collect(),
    )))
}
