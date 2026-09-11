//! A finite, source-derived closure gate. Runtime inventories alone cannot
//! witness a source overload whose concrete factory was never registered.
#![cfg(any(feature = "distribution-standard", feature = "distribution-full"))]

use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    BoundCallOrigin, BuiltinScalarKind, BytecodeInstruction, DimensionExpr, ExecutionTarget,
    FunctionCatalog, FunctionExposure, FunctionTypeDeclaration, FunctionTypeOverload,
    InputKindScheme, KindConstraint, KindExpr, KindScheme, MResult, ParsedProgram, SourceInputKind,
    SourceSchemeTemplate, SourceTypeAuthority, TableJoinMode,
};
use mech_engine::{
    ProgramCompilationProduct, decode_program_artifact_bytecode_v1,
    encode_program_artifact_bytecode_v1,
    resident::{ActivationFacts, ResidentActivationOptions, preflight_resident_target},
};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};
use serde_json::{Value as Json, json};

#[derive(Clone, Debug)]
struct Witness {
    operation: String,
    overload: Option<u32>,
    source: String,
    // TableJoin templates currently advertise bytecode/direct support only.
    // This is an explicit semantic-template policy, never inferred from a
    // missing binder (which would turn a regression into a passing skip).
    resident: bool,
    artifact_only_syntax: bool,
}

fn boolean_instantiable(kind: &KindExpr) -> bool {
    match kind {
        KindExpr::Named(id) => *id == BuiltinScalarKind::Bool.kind_id(),
        KindExpr::Parameter(_) => true,
        KindExpr::Matrix {
            element,
            dimensions,
        } => boolean_instantiable(element) && dimensions.len() == 2,
        _ => false,
    }
}

fn boolean_constraint(constraint: &KindConstraint) -> bool {
    match constraint {
        KindConstraint::Equal(left, right) | KindConstraint::Convertible(left, right) => {
            boolean_instantiable(left) && boolean_instantiable(right)
        }
        KindConstraint::Satisfies { kind, predicate } => {
            boolean_instantiable(kind)
                && matches!(
                    predicate,
                    mech_core::BuiltinKindPredicate::Equatable
                        | mech_core::BuiltinKindPredicate::Keyable
                )
        }
        KindConstraint::DimensionEqual(_, _)
        | KindConstraint::DimensionCompatible(_, _)
        | KindConstraint::DimensionLessEqual(_, _) => true,
        KindConstraint::Promotes { .. } => false,
    }
}

fn f64_instantiable(kind: &KindExpr) -> bool {
    match kind {
        KindExpr::Named(id) => {
            *id == BuiltinScalarKind::F64.kind_id() || *id == BuiltinScalarKind::Bool.kind_id()
        }
        KindExpr::Parameter(_) => true,
        KindExpr::Matrix {
            element,
            dimensions,
        } => f64_instantiable(element) && dimensions.len() == 2,
        _ => false,
    }
}

fn f64_constraint(constraint: &KindConstraint) -> bool {
    match constraint {
        KindConstraint::Equal(left, right) | KindConstraint::Convertible(left, right) => {
            f64_instantiable(left) && f64_instantiable(right)
        }
        KindConstraint::Satisfies { kind, predicate } => {
            f64_instantiable(kind)
                && matches!(
                    predicate,
                    mech_core::BuiltinKindPredicate::Number
                        | mech_core::BuiltinKindPredicate::Real
                        | mech_core::BuiltinKindPredicate::FloatingPoint
                        | mech_core::BuiltinKindPredicate::Ordered
                        | mech_core::BuiltinKindPredicate::Negatable
                        | mech_core::BuiltinKindPredicate::RangeEndpoint
                        | mech_core::BuiltinKindPredicate::Equatable
                        | mech_core::BuiltinKindPredicate::Keyable
                )
        }
        KindConstraint::Promotes {
            left,
            right,
            output,
        } => f64_instantiable(left) && f64_instantiable(right) && f64_instantiable(output),
        KindConstraint::DimensionEqual(_, _)
        | KindConstraint::DimensionCompatible(_, _)
        | KindConstraint::DimensionLessEqual(_, _) => true,
    }
}

fn instantiable_overload(
    overload: &FunctionTypeOverload,
    instantiable: fn(&KindExpr) -> bool,
    constraint_admitted: fn(&KindConstraint) -> bool,
) -> bool {
    let InputKindScheme::Fixed(inputs) = overload.scheme.inputs() else {
        return false;
    };
    !inputs.is_empty()
        && overload
            .input_layout
            .iter()
            .all(|kind| *kind == SourceInputKind::Value)
        && inputs
            .iter()
            .chain(overload.scheme.outputs())
            .all(instantiable)
        && overload
            .scheme
            .constraints()
            .iter()
            .all(constraint_admitted)
        && overload
            .scheme
            .kind_parameters()
            .iter()
            .all(|parameter| parameter.upper_bound.is_none())
}

fn boolean_instantiable_overload(overload: &FunctionTypeOverload) -> bool {
    instantiable_overload(overload, boolean_instantiable, boolean_constraint)
}

fn f64_instantiable_overload(overload: &FunctionTypeOverload) -> bool {
    instantiable_overload(overload, f64_instantiable, f64_constraint)
}

fn dimension(expr: &DimensionExpr, parameters: &[u64]) -> u64 {
    match expr {
        DimensionExpr::Constant(value) => *value,
        DimensionExpr::Parameter(id) => parameters[id.get() as usize],
        DimensionExpr::Add(parts) => parts.iter().map(|part| dimension(part, parameters)).sum(),
        DimensionExpr::Multiply(parts) => parts
            .iter()
            .map(|part| dimension(part, parameters))
            .product(),
        DimensionExpr::Min(parts) => parts
            .iter()
            .map(|part| dimension(part, parameters))
            .min()
            .unwrap(),
        DimensionExpr::Max(parts) => parts
            .iter()
            .map(|part| dimension(part, parameters))
            .max()
            .unwrap(),
        DimensionExpr::Hole => {
            panic!("closed Boolean scheme contains an unresolved dimension hole")
        }
    }
}

fn admitted_dimensions(scheme: &KindScheme, parameters: &[u64]) -> bool {
    let within_bounds = scheme.dimension_parameters().iter().all(|parameter| {
        let actual = parameters[parameter.id.get() as usize];
        actual >= dimension(&parameter.lower_bound, parameters)
            && parameter
                .upper_bound
                .as_ref()
                .is_none_or(|bound| actual <= dimension(bound, parameters))
    });
    within_bounds
        && scheme
            .constraints()
            .iter()
            .all(|constraint| match constraint {
                KindConstraint::DimensionEqual(left, right)
                | KindConstraint::DimensionCompatible(left, right) => {
                    dimension(left, parameters) == dimension(right, parameters)
                }
                KindConstraint::DimensionLessEqual(left, right) => {
                    dimension(left, parameters) <= dimension(right, parameters)
                }
                KindConstraint::Equal(_, _)
                | KindConstraint::Convertible(_, _)
                | KindConstraint::Satisfies { .. }
                | KindConstraint::Promotes { .. } => true,
            })
}

fn bool_source(kind: &KindExpr, parameters: &[u64], input: usize) -> String {
    match kind {
        KindExpr::Named(_) | KindExpr::Parameter(_) => {
            if input % 2 == 0 { "true" } else { "false" }.into()
        }
        KindExpr::Matrix { dimensions, .. } => {
            let rows = dimension(&dimensions[0], parameters);
            let columns = dimension(&dimensions[1], parameters);
            let rows = (0..rows)
                .map(|row| {
                    (0..columns)
                        .map(|column| {
                            if (row + column + input as u64) % 2 == 0 {
                                "true"
                            } else {
                                "false"
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("[{rows}]")
        }
        other => panic!("unsupported closed Boolean kind {other:?}"),
    }
}

fn f64_source(kind: &KindExpr, parameters: &[u64], input: usize) -> String {
    match kind {
        KindExpr::Named(id) if *id == BuiltinScalarKind::Bool.kind_id() => {
            if input % 2 == 0 { "true" } else { "false" }.into()
        }
        KindExpr::Named(id) if *id == BuiltinScalarKind::F64.kind_id() => {
            format!("{}.0", input % 2 + 1)
        }
        KindExpr::Parameter(_) => format!("{}.0", input % 2 + 1),
        KindExpr::Matrix {
            element,
            dimensions,
        } => {
            let rows = dimension(&dimensions[0], parameters);
            let columns = dimension(&dimensions[1], parameters);
            let rows = (0..rows)
                .map(|row| {
                    (0..columns)
                        .map(|column| {
                            if row == column
                                && !matches!(
                                    element.as_ref(),
                                    KindExpr::Named(id)
                                        if *id == BuiltinScalarKind::Bool.kind_id()
                                )
                            {
                                format!("{}.0", 10 + input)
                            } else {
                                f64_source(element, parameters, input + (row + column) as usize)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("[{rows}]")
        }
        other => panic!("unsupported closed f64 comparison kind {other:?}"),
    }
}

fn boolean_source_expression(operation: &str, arguments: &[String]) -> String {
    match operation {
        "compare/seq" => {
            assert_eq!(arguments.len(), 2);
            format!("{} === {}", arguments[0], arguments[1])
        }
        "compare/sneq" => {
            assert_eq!(arguments.len(), 2);
            format!("{} !== {}", arguments[0], arguments[1])
        }
        _ => format!("{operation}({})", arguments.join(", ")),
    }
}

fn representative_witnesses(
    operation: &str,
    declaration: &FunctionTypeDeclaration,
    overload_admitted: fn(&FunctionTypeOverload) -> bool,
    source_for_kind: fn(&KindExpr, &[u64], usize) -> String,
    representative: &str,
) -> Vec<Witness> {
    let mut result = Vec::new();
    for overload in &declaration.overloads {
        if !overload_admitted(overload) {
            continue;
        }
        let InputKindScheme::Fixed(inputs) = overload.scheme.inputs() else {
            unreachable!("representative-instantiable overloads have fixed inputs")
        };
        assert!(
            overload
                .input_layout
                .iter()
                .all(|kind| *kind == SourceInputKind::Value)
        );
        let parameter_count = overload.scheme.dimension_parameters().len();
        // A bounded shape basis, independent of the installed factory rows:
        // scalar, rows, columns, square, rectangular, and dynamic extents.
        // Vary the third dimension to cover either direction of broadcasting.
        let basis = [
            (1, 3),
            (3, 1),
            (2, 2),
            (2, 3),
            (3, 2),
            (5, 5),
            (1, 5),
            (5, 1),
        ];
        let mut sources = BTreeSet::new();
        for (rows, columns) in basis {
            for broadcast in [rows, columns] {
                let parameters = (0..parameter_count)
                    .map(|index| match index {
                        0 => rows,
                        1 => columns,
                        _ => broadcast,
                    })
                    .collect::<Vec<_>>();
                if !admitted_dimensions(&overload.scheme, &parameters) {
                    continue;
                }
                let arguments = inputs
                    .iter()
                    .enumerate()
                    .map(|(input, kind)| source_for_kind(kind, &parameters, input))
                    .collect::<Vec<_>>();
                sources.insert(boolean_source_expression(operation, &arguments));
            }
        }
        assert!(
            !sources.is_empty(),
            "{operation} overload {} has no admitted {representative} shape witness",
            overload.id,
        );
        result.extend(sources.into_iter().map(|source| Witness {
            operation: operation.into(),
            overload: Some(overload.id),
            source,
            resident: true,
            artifact_only_syntax: false,
        }));
    }
    result
}

fn boolean_witnesses(operation: &str, declaration: &FunctionTypeDeclaration) -> Vec<Witness> {
    representative_witnesses(
        operation,
        declaration,
        boolean_instantiable_overload,
        bool_source,
        "Boolean",
    )
}

fn f64_witnesses(operation: &str, declaration: &FunctionTypeDeclaration) -> Vec<Witness> {
    representative_witnesses(
        operation,
        declaration,
        f64_instantiable_overload,
        f64_source,
        "f64",
    )
}

fn scalar_literal(kind: BuiltinScalarKind, ordinal: u8) -> String {
    match kind {
        BuiltinScalarKind::Bool => if ordinal % 2 == 0 { "false" } else { "true" }.into(),
        BuiltinScalarKind::String => format!("\"value{ordinal}\""),
        BuiltinScalarKind::C64 => format!("{ordinal}+0i"),
        BuiltinScalarKind::R64 => format!("{ordinal}/1"),
        _ => format!("{ordinal}<{}>", kind.canonical_name()),
    }
}

fn generate(catalog: &FunctionCatalog) -> (Vec<Witness>, Json) {
    let mut witnesses = Vec::new();
    let mut covered = BTreeSet::new();
    let mut deferred = BTreeMap::new();
    let mut overload_coverage = BTreeMap::new();
    for entry in catalog.all_specializers() {
        let name = entry.operation.canonical_name.as_ref();
        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            deferred.insert(name.to_owned(), "syntax-directed input adapter");
            continue;
        };
        if let Some(SourceSchemeTemplate::TableJoin(mode)) = declaration.template {
            // Internal joins are exposed by operator syntax. This exhaustive
            // template-to-syntax adapter contains no runtime factory IDs.
            let operator = match mode {
                TableJoinMode::Inner => "⋈",
                TableJoinMode::LeftOuter => "⟕",
                TableJoinMode::RightOuter => "⟖",
                TableJoinMode::FullOuter => "⟗",
                TableJoinMode::LeftSemi => "⋉",
                TableJoinMode::LeftAnti => "▷",
            };
            witnesses.push(Witness {
                operation: name.into(), overload: None, resident: false, artifact_only_syntax: false,
                source: format!("left := |id<f64> x<f64>| 1 10 | 2 20 |\nright := |id<f64> y<f64>| 2 30 | 3 40 |\nleft {operator} right"),
            });
            covered.insert(name.to_owned());
        } else {
            let mut generated = boolean_witnesses(name, declaration);
            let exports = catalog.exports_for_operation(entry.operation.id);
            let externally_exposed = exports
                .iter()
                .any(|export| export.exposure != FunctionExposure::Internal);
            let read_modify_write = entry.operation.contract.outputs.iter().any(|output| {
                matches!(
                    output.construction,
                    mech_core::OutputConstruction::ReadModifyWrite { .. }
                )
            });
            if externally_exposed && !read_modify_write {
                generated.extend(f64_witnesses(name, declaration));
            }
            if !exports
                .iter()
                .any(|export| export.exposure == FunctionExposure::Prelude)
                && let Some(module) = exports.iter().find_map(|export| {
                    (export.exposure == FunctionExposure::ModuleOnly)
                        .then(|| export.module.as_deref())
                        .flatten()
                })
            {
                for witness in &mut generated {
                    witness.source = format!("+> {module}\n{}", witness.source);
                }
            }
            let witnessed = generated
                .iter()
                .filter_map(|witness| witness.overload)
                .collect::<BTreeSet<_>>();
            let deferred_overloads = declaration
                .overloads
                .iter()
                .map(|overload| overload.id)
                .filter(|id| !witnessed.contains(id))
                .collect::<Vec<_>>();
            overload_coverage.insert(
                name.to_owned(),
                json!({"generated": witnessed, "generation_deferred": deferred_overloads}),
            );
            if generated.is_empty() {
                deferred.insert(
                    name.to_owned(),
                    "outside the current representative or structural templates",
                );
            } else {
                covered.insert(name.to_owned());
                witnesses.extend(generated);
            }
        }
    }
    // Scalar availability is generated from representation features already
    // derived by native linkage. This cannot hide a missing column binder:
    // availability comes from the whole catalog, never from access/column.
    let features = catalog
        .runtime_entries()
        .flat_map(|entry| entry.signature().required_native_features())
        .map(|feature| feature.cargo_feature())
        .collect::<BTreeSet<_>>();
    let mut column_kinds = Vec::new();
    for kind in BuiltinScalarKind::ALL {
        if !features.contains(kind.canonical_name()) {
            continue;
        }
        column_kinds.push(kind.canonical_name());
        witnesses.push(Witness {
            operation: "access/column".into(),
            overload: None,
            resident: true,
            artifact_only_syntax: true,
            source: format!(
                "data := |value<{}>| {} | {} |\ndata.value",
                kind.canonical_name(),
                scalar_literal(kind, 1),
                scalar_literal(kind, 2)
            ),
        });
    }
    assert!(!column_kinds.is_empty(), "no active scalar representations");
    assert!(
        witnesses.iter().any(|witness| witness.overload.is_some()),
        "no representative source overloads were discovered"
    );
    deferred.remove("access/column");
    covered.insert("access/column".into());
    (
        witnesses,
        json!({"covered_operations": covered, "deferred_operations": deferred, "overload_coverage": overload_coverage, "column_scalar_kinds": column_kinds, "syntax_adapters": ["compare/seq", "compare/sneq"], "artifact_only_syntax_adapters": ["access/column"]}),
    )
}

fn validate_emission(
    catalog: &FunctionCatalog,
    product: &ProgramCompilationProduct,
    instructions: &[BytecodeInstruction],
    artifact_only_operation: Option<&str>,
) -> Result<Vec<Json>, String> {
    let bindings = product.instruction_type_bindings();
    let requirements = product.instruction_type_binding_requirements();
    if bindings.len() != instructions.len()
        || requirements.len() != instructions.len()
        || product.instruction_memory_plans().len() != instructions.len()
    {
        return Err("instruction/certificate sidecar length mismatch".into());
    }
    let mut rows = Vec::new();
    for (index, instruction) in instructions.iter().enumerate() {
        let Some(emitted) = instruction.runtime_function() else {
            continue;
        };
        let Some(binding) = bindings[index].as_ref() else {
            if requirements[index] {
                return Err(format!(
                    "runtime instruction {index} requires a selected binding"
                ));
            }
            // Literal construction can emit auxiliary calls without source
            // semantics. Only the compiler's requirement metadata permits
            // this case; runtime names never determine an exemption.
            let entry = catalog.runtime_entry_by_raw(emitted).ok_or_else(|| {
                format!("compiler helper instruction {index} has no runtime factory")
            })?;
            if !entry
                .execution_capability()
                .targets
                .contains(ExecutionTarget::DirectRuntime)
            {
                return Err(format!(
                    "compiler helper instruction {index} has no direct target"
                ));
            }
            if product.instruction_memory_plans()[index].is_some() {
                return Err(format!(
                    "compiler helper instruction {index} has a memory certificate without a binding"
                ));
            }
            rows.push(json!({"instruction": index, "role": "compiler_helper", "operation": null, "overload": null, "selected_id": null, "emitted_id": format!("{emitted:016x}"), "factory": entry.name}));
            continue;
        };
        let selected = binding
            .runtime_function()
            .ok_or_else(|| format!("runtime instruction {index} has a resident binding"))?;
        if emitted != selected.raw() {
            return Err(format!(
                "runtime instruction {index}: selected {:016x}, emitted {emitted:016x}",
                selected.raw()
            ));
        }
        let memory = product.instruction_memory_plans()[index]
            .as_ref()
            .ok_or_else(|| format!("runtime instruction {index} has no memory plan"))?;
        if memory.bound_call != *binding {
            return Err(format!(
                "runtime instruction {index} has a divergent memory certificate"
            ));
        }
        if catalog.runtime_entry(selected).is_none()
            && artifact_only_operation
                == Some(binding.operation_descriptor().canonical_name.as_ref())
            && matches!(binding.origin(), BoundCallOrigin::SyntaxDirected)
            && binding.target() == ExecutionTarget::DirectRuntime
        {
            // Production explicitly permits SyntaxDirected compiler identities
            // without catalog/native availability. Keep this deferred edge
            // visible; it is not a successful concrete-factory closure row.
            rows.push(json!({"instruction": index, "role": "artifact_only_syntax", "operation": binding.operation_descriptor().canonical_name.as_ref(), "overload": null, "selected_id": format!("{:016x}", selected.raw()), "emitted_id": format!("{emitted:016x}"), "factory": null, "deferred_edge": "runtime_id_to_concrete_catalog_factory"}));
            continue;
        }
        let entry = catalog
            .validate_bound_call_for_target(binding, ExecutionTarget::DirectRuntime)
            .map_err(|error| format!("runtime instruction {index}: {error:?}"))?;
        let overload = match binding.origin() {
            BoundCallOrigin::ResolvedOverload(id) => Some(*id),
            _ => None,
        };
        rows.push(json!({"instruction": index, "role": "bound_call", "operation": binding.operation_descriptor().canonical_name.as_ref(), "overload": overload, "selected_id": format!("{:016x}", selected.raw()), "emitted_id": format!("{emitted:016x}"), "factory": entry.name}));
    }
    Ok(rows)
}

#[test]
fn generated_source_catalog_closes_over_bytecode_and_resident_binders() -> MResult<()> {
    let catalog = mech::stdlib::source_catalog();
    let (witnesses, universe) = generate(&catalog);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()?;
    let mut results = Vec::new();
    let mut selected_overloads = BTreeSet::new();
    let mut compilation_failures = Vec::new();
    for witness in &witnesses {
        let product = match compiler.compile_source(&witness.source) {
            Ok(product) => product,
            Err(error) => {
                compilation_failures.push(json!({"operation": witness.operation, "overload": witness.overload, "source": witness.source, "reason": error.kind_message()}));
                continue;
            }
        };
        let parsed = ParsedProgram::from_bytes(product.bytecode())?;
        let runtime_rows = validate_emission(
            &catalog,
            &product,
            &parsed.instructions,
            witness
                .artifact_only_syntax
                .then_some(witness.operation.as_str()),
        )
        .unwrap_or_else(|error| panic!("{}: {error}\n{}", witness.operation, witness.source));
        let artifact_only = runtime_rows
            .iter()
            .filter(|row| row["role"] == "artifact_only_syntax")
            .collect::<Vec<_>>();
        let runtime_validation = if artifact_only.is_empty() {
            parsed
                .validate_runtime_contracts(&catalog)
                .unwrap_or_else(|error| {
                    panic!(
                        "{} emitted invalid runtime register/argument contracts: {error:?}\n{}",
                        witness.operation, witness.source
                    )
                });
            json!({"scope": "complete_instruction_stream"})
        } else {
            // This adapter emits one artifact-only final operation. Validate
            // all real catalog calls before it, without claiming a runtime ABI
            // validator for an implementation that has no runtime factory.
            assert_eq!(artifact_only.len(), 1);
            let instruction = artifact_only[0]["instruction"].as_u64().unwrap() as usize;
            assert!(
                parsed.instructions[instruction + 1..]
                    .iter()
                    .all(|instruction| instruction.runtime_function().is_none())
            );
            let mut prefix = ParsedProgram::from_bytes(product.bytecode())?;
            prefix.instructions.truncate(instruction);
            prefix
                .validate_runtime_contracts(&catalog)
                .unwrap_or_else(|error| {
                    panic!(
                        "{} compiler-helper prefix has invalid runtime contracts: {error:?}",
                        witness.operation
                    )
                });
            json!({"scope": "catalog_prefix", "end_instruction_exclusive": instruction, "deferred_instruction": instruction, "reason": "artifact-only syntax implementation has no concrete runtime factory"})
        };
        assert!(
            runtime_rows
                .iter()
                .any(|row| row["operation"] == witness.operation),
            "witness did not select its claimed source operation {}: {}",
            witness.operation,
            witness.source
        );
        for row in &runtime_rows {
            if row["operation"] == witness.operation
                && let Some(overload) = row["overload"].as_u64()
            {
                selected_overloads.insert((witness.operation.clone(), overload as u32));
            }
        }
        let decoded = decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
        let canonical_bytes = encode_program_artifact_bytecode_v1(&decoded).unwrap();
        let canonical = decode_program_artifact_bytecode_v1(&canonical_bytes).unwrap();
        assert_eq!(
            product.artifact().revision(),
            decoded.revision(),
            "{} changed semantic artifact identity on initial bytecode decode",
            witness.operation
        );
        assert_eq!(
            decoded.revision(),
            canonical.revision(),
            "{} changed semantic artifact identity on canonical roundtrip",
            witness.operation
        );
        assert_eq!(
            encode_program_artifact_bytecode_v1(&canonical).unwrap(),
            canonical_bytes
        );
        let preflight = preflight_resident_target(
            &canonical,
            &catalog,
            &ActivationFacts::default(),
            ResidentActivationOptions::default(),
        );
        let resident = if witness.resident {
            let preflight = preflight.unwrap_or_else(|error| {
                panic!(
                    "{} lost resident closure: {error:?}\n{}",
                    witness.operation, witness.source
                )
            });
            assert_eq!(preflight.concrete_cases.len(), canonical.nodes().len());
            let mut witnessed_nodes = BTreeSet::new();
            for case in &preflight.concrete_cases {
                assert!(
                    witnessed_nodes.insert(case.node.get()),
                    "resident preflight repeated node {:?}",
                    case.node
                );
                let node = &canonical.nodes()[case.node.get() as usize];
                assert_eq!(case.operation, node.operation);
                assert!(case.targets.contains(ExecutionTarget::ResidentCpu));
                assert!(
                    catalog
                        .resident_factory(
                            &case.operation.module_path,
                            &case.operation.operation_name
                        )
                        .is_some()
                );
            }
            let mut runtime = RuntimeBuilder::new()
                .function_catalog(catalog.clone())
                .build()?;
            runtime
                .load_bytecode_program(&canonical_bytes, ResidentDurabilityPolicy::Volatile)
                .unwrap_or_else(|error| {
                    panic!(
                        "{} preflight passed but resident binding/activation failed: {error:?}\n{}",
                        witness.operation, witness.source
                    )
                });
            json!({"outcome": "activated", "node_count": canonical.nodes().len()})
        } else {
            let error = preflight.expect_err("table join target policy changed: qualify its resident implementation and update this template policy");
            assert_eq!(error.target, ExecutionTarget::ResidentCpu);
            let node = error
                .node
                .expect("unsupported join must identify the rejected node");
            let operation = error
                .operation
                .expect("unsupported join must retain semantic identity");
            assert_eq!(operation, canonical.nodes()[node.get() as usize].operation);
            assert_eq!(operation.canonical_name(), witness.operation);
            assert!(
                error.reason.contains("MissingResidentFactory"),
                "unexpected target rejection: {}",
                error.reason
            );
            json!({"outcome": "unsupported", "node": node.get(), "operation": operation.canonical_name(), "reason": error.reason})
        };
        results.push(json!({"operation": witness.operation, "overload": witness.overload, "source": witness.source, "runtime_calls": runtime_rows, "runtime_validation": runtime_validation, "resident": resident}));
    }
    let profile = if cfg!(feature = "distribution-full") {
        "distribution-full"
    } else {
        "distribution-standard"
    };
    if let Ok(expected) = std::env::var("MECH_CATALOG_CLOSURE_PROFILE") {
        assert_eq!(expected, profile);
    }
    let generated_overloads = witnesses
        .iter()
        .filter_map(|witness| {
            witness
                .overload
                .map(|overload| (witness.operation.clone(), overload))
        })
        .collect::<BTreeSet<_>>();
    let selected_overload_rows = selected_overloads
        .iter()
        .map(|(operation, overload)| json!({"operation": operation, "overload": overload}))
        .collect::<Vec<_>>();
    let selection_deferred = generated_overloads
        .difference(&selected_overloads)
        .map(|(operation, overload)| {
            json!({
                "operation": operation,
                "overload": overload,
                "reason": "closed representative syntax selected a higher-ranked compatible overload",
            })
        })
        .collect::<Vec<_>>();
    let report = json!({"schema": "mech.catalog-closure.v1", "profile": profile, "generated_witness_count": witnesses.len(), "witness_count": results.len(), "compilation_failures": compilation_failures, "runtime_factory_count": catalog.runtime_factory_count(), "universe": universe, "selected_overloads": selected_overload_rows, "selection_deferred": selection_deferred, "witnesses": results});
    if let Some(path) = std::env::var_os("MECH_CATALOG_CLOSURE_REPORT") {
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    }
    assert!(
        compilation_failures.is_empty(),
        "{} source-derived witnesses failed to compile:\n{}",
        compilation_failures.len(),
        serde_json::to_string_pretty(&compilation_failures).unwrap()
    );
    for operation in generated_overloads
        .iter()
        .map(|(operation, _)| operation)
        .collect::<BTreeSet<_>>()
    {
        assert!(
            selected_overloads
                .iter()
                .any(|(selected, _)| selected == operation),
            "{operation} never selected any generated overload"
        );
    }
    eprintln!(
        "catalog closure: {} generated witnesses, profile {profile}",
        witnesses.len()
    );
    Ok(())
}

#[test]
fn source_witness_generation_does_not_depend_on_runtime_factory_presence() {
    let declaration = mech_core::maintained_source_type_declaration("logic/not").unwrap();
    let witnesses = boolean_witnesses("logic/not", &declaration);
    assert!(FunctionCatalog::empty().runtime_entries().next().is_none());
    for overload in &declaration.overloads {
        assert!(
            witnesses
                .iter()
                .any(|witness| witness.overload == Some(overload.id))
        );
    }
    assert!(witnesses.iter().any(|witness| witness.source.contains(';')));

    let declaration = mech_core::maintained_source_type_declaration("compare/eq").unwrap();
    let witnesses = boolean_witnesses("compare/eq", &declaration);
    for overload in declaration
        .overloads
        .iter()
        .filter(|overload| boolean_instantiable_overload(overload))
    {
        assert!(
            witnesses
                .iter()
                .any(|witness| witness.overload == Some(overload.id)),
            "Boolean instantiation omitted compare/eq overload {}",
            overload.id
        );
    }

    for operation in ["compare/seq", "compare/sneq"] {
        let declaration = mech_core::maintained_source_type_declaration(operation).unwrap();
        let witnesses = boolean_witnesses(operation, &declaration);
        assert!(witnesses.iter().any(|witness| {
            witness.source.contains(if operation == "compare/seq" {
                " === "
            } else {
                " !== "
            })
        }));
        assert!(
            witnesses
                .iter()
                .all(|witness| !witness.source.contains(operation))
        );
    }

    let catalog = mech::stdlib::source_catalog();
    let mut f64_external_overloads = 0;
    for entry in catalog.all_specializers() {
        let name = entry.operation.canonical_name.as_ref();
        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            continue;
        };
        let exports = catalog.exports_for_operation(entry.operation.id);
        let externally_exposed = exports
            .iter()
            .any(|export| export.exposure != FunctionExposure::Internal);
        let read_modify_write = entry.operation.contract.outputs.iter().any(|output| {
            matches!(
                output.construction,
                mech_core::OutputConstruction::ReadModifyWrite { .. }
            )
        });
        if !externally_exposed || read_modify_write {
            continue;
        }
        let witnesses = f64_witnesses(name, declaration);
        for overload in declaration
            .overloads
            .iter()
            .filter(|overload| f64_instantiable_overload(overload))
        {
            f64_external_overloads += 1;
            assert!(
                witnesses
                    .iter()
                    .any(|witness| witness.overload == Some(overload.id)),
                "externally exposed f64 overload {name}:{} has no witness",
                overload.id
            );
        }
    }
    assert!(f64_external_overloads >= 50);
}

#[test]
fn closure_validator_rejects_broken_runtime_and_resident_edges() -> MResult<()> {
    let catalog = mech::stdlib::source_catalog();
    let product = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()?
        .compile_source("logic/not(true)")?;
    let mut parsed = ParsedProgram::from_bytes(product.bytecode())?;
    assert!(validate_emission(&catalog, &product, &parsed.instructions, None).is_ok());
    assert!(
        validate_emission(
            &FunctionCatalog::empty(),
            &product,
            &parsed.instructions,
            None
        )
        .is_err()
    );
    assert!(
        validate_emission(
            &FunctionCatalog::empty(),
            &product,
            &parsed.instructions,
            Some("logic/not")
        )
        .is_err(),
        "the artifact-only syntax policy must not exempt a source-scheme operation"
    );
    let instruction = parsed
        .instructions
        .iter_mut()
        .find(|instruction| instruction.runtime_function().is_some())
        .unwrap();
    match instruction {
        BytecodeInstruction::RuntimeUnary { function, .. } => *function ^= 1,
        other => panic!("expected a unary generated call, got {other:?}"),
    }
    assert!(
        validate_emission(&catalog, &product, &parsed.instructions, None)
            .unwrap_err()
            .contains("selected")
    );
    // The semantic artifact can remain intact even if an instruction emitter
    // corrupts a register operand. Check the actual runtime bytecode too.
    let mut invalid_operand = ParsedProgram::from_bytes(product.bytecode())?;
    invalid_operand.validate_runtime_contracts(&catalog)?;
    match invalid_operand
        .instructions
        .iter_mut()
        .find(|instruction| instruction.runtime_function().is_some())
        .unwrap()
    {
        BytecodeInstruction::RuntimeUnary { src, .. } => *src = u32::MAX,
        _ => unreachable!(),
    }
    assert!(
        invalid_operand
            .validate_runtime_contracts(&catalog)
            .is_err()
    );
    let artifact = decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
    let missing_binder = preflight_resident_target(
        &artifact,
        &FunctionCatalog::empty(),
        &ActivationFacts::default(),
        ResidentActivationOptions::default(),
    )
    .unwrap_err();
    assert!(missing_binder.reason.contains("MissingResidentFactory"));
    let node = missing_binder
        .node
        .expect("binder rejection identifies the node");
    assert_eq!(
        missing_binder.operation.as_ref(),
        Some(&artifact.nodes()[node.get() as usize].operation)
    );
    assert_eq!(
        missing_binder.operation.unwrap().canonical_name(),
        "logic/not"
    );
    Ok(())
}
