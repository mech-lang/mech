use std::collections::BTreeMap;
use std::fmt::Debug;

use mech_core::{
    FunctionCatalog, ManagedMemoryBudget, MechCode, NoMechExecutionServices, Program,
    ReactiveInstanceId, SectionElement, Statement,
};
use mech_engine::__resident::{
    ActivationFacts, ReactiveInstance, ResidentActivationOptions, ResidentExecutionError,
    ResidentStorageClass, ResidentValueBorrow, activate, activate_with_options,
};
use mech_engine::program::{CompilerPlanningConfig, CompilerPlanningProgram};
use mech_engine::{ProgramArtifact, ProgramArtifactDraft, decode_program_artifact_bytecode_v1};

const SOURCE: &str = include_str!("r-stack-proof/trust-program.mec");

// This is deliberately literal test data, not a second implementation of the
// recurrence. Every value can be checked by hand from the five-line Mech source.
const EXPECTED_COMMITTED_OUTPUTS: [f64; 7] = [5.0, 13.0, 29.0, 61.0, 125.0, 253.0, 509.0];
const EXPECTED_REJECTED_CANDIDATE: f64 = 1021.0;
const APPROVED_OPERATIONS: [&str; 4] = ["math/mul", "math/add", "compare/lt", "core/assign"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dump {
    None,
    Parse,
    Artifact,
    Activation,
    Memory,
    All,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("\nTRUST PROOF FAILED: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let dump = requested_dump()?;

    heading("AUDIT-GRADE R-STACK PROOF");
    println!("Claim under test : ordinary Mech composition reaches protected resident execution");
    println!(
        "Non-claim        : primitive arithmetic is not native code (it is, as disclosed below)"
    );
    println!("Failure policy   : any mismatch or missing protection exits non-zero");

    heading("1 / THE COMPLETE MECH PROGRAM");
    for (index, line) in SOURCE.lines().enumerate() {
        println!("{:>3} | {line}", index + 1);
    }
    println!("\nHuman-checkable recurrence: x(next) = x * 2 + 3; only candidates < 1000 commit.");

    let tree = mech_syntax::parser::parse(SOURCE).map_err(debug_error)?;
    let parse = summarize_parse(&tree);
    heading("2 / REAL PARSER OUTPUT");
    println!("sections       : {}", tree.body.sections.len());
    println!("Mech code items: {}", parse.total);
    for (kind, count) in &parse.kinds {
        println!("  {kind:<22} {count:>2}");
    }
    println!("parser status  : PASS — complete source accepted");
    if matches!(dump, Dump::Parse | Dump::All) {
        println!("\n--- FULL PARSE TREE (Debug) ---\n{tree:#?}");
    } else {
        println!("inspect more   : ./scripts/demo-r-stack.sh --dump parse");
    }

    let catalog = mech_stdlib::source_catalog();
    let mut compiler = CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        catalog.clone(),
    );
    let output = compiler
        .plan_tree_with_services(&tree, &mut NoMechExecutionServices)
        .map_err(debug_error)?
        .ok_or("source produced no output")?;
    compiler.publish_compiler_root_output(output);
    let product = compiler.compile_program_product().map_err(debug_error)?;

    let planned_instruction_count = product.instruction_type_bindings().len();
    let required_bindings = product
        .instruction_type_binding_requirements()
        .iter()
        .filter(|required| **required)
        .count();
    let present_bindings = product
        .instruction_type_bindings()
        .iter()
        .filter(|binding| binding.is_some())
        .count();
    let present_memory_plans = product
        .instruction_memory_plans()
        .iter()
        .filter(|plan| plan.is_some())
        .count();

    let (source_artifact, bytecode) = product.into_parts();
    let decoded_artifact = decode_program_artifact_bytecode_v1(&bytecode).map_err(debug_error)?;
    require(
        source_artifact.revision() == decoded_artifact.revision(),
        "source and bytecode-decoded artifact revisions differ",
    )?;

    let actual_operations = source_artifact
        .nodes()
        .iter()
        .map(|node| node.operation.canonical_name())
        .collect::<Vec<_>>();
    require(
        actual_operations == APPROVED_OPERATIONS,
        &format!(
            "artifact operation allowlist mismatch: expected {APPROVED_OPERATIONS:?}, got {actual_operations:?}"
        ),
    )?;
    require(
        actual_operations.iter().all(|name| !name.contains("ekf")),
        "artifact contains a hidden EKF operation",
    )?;
    require(
        source_artifact.constraints().len() == 1,
        "artifact must retain exactly one integrity constraint",
    )?;

    heading("3 / COMPILER PRODUCT + IMMUTABLE ARTIFACT");
    println!("artifact revision : {}", revision_hex(&source_artifact));
    println!("bytecode-v1 bytes : {}", bytecode.len());
    println!("round trip        : PASS — decoded revision is identical");
    println!("planned instr.    : {planned_instruction_count}");
    println!("typed bindings    : {present_bindings} present / {required_bindings} required");
    println!("memory plans      : {present_memory_plans} attached");
    println!(
        "schemas/constants : {} / {}",
        source_artifact.schemas().len(),
        source_artifact.constants().len()
    );
    println!(
        "slots/nodes       : {} / {}",
        source_artifact.slots().len(),
        source_artifact.nodes().len()
    );
    println!(
        "outputs/constraints: {} / {}",
        source_artifact.outputs().len(),
        source_artifact.constraints().len()
    );
    println!("\nnode  operation       boundary");
    for node in source_artifact.nodes() {
        println!(
            "{:>4}  {:<15} native primitive selected from the standard catalog",
            node.node.get(),
            node.operation.canonical_name()
        );
    }
    println!("\nallowlist        : PASS — exactly {APPROVED_OPERATIONS:?}");
    println!("hidden ekf/*     : NONE");
    println!("algorithm kernel : NONE — recurrence exists only as the four-node Mech graph above");
    if matches!(dump, Dump::Artifact | Dump::All) {
        println!("\n--- FULL SOURCE PROGRAM ARTIFACT (Debug) ---\n{source_artifact:#?}");
        println!("\n--- FULL DECODED PROGRAM ARTIFACT (Debug) ---\n{decoded_artifact:#?}");
    } else {
        println!("inspect more     : ./scripts/demo-r-stack.sh --dump artifact");
    }

    let missing_catalog_error = activate(
        ReactiveInstanceId::new(90, 0),
        &source_artifact,
        &FunctionCatalog::empty(),
        &ActivationFacts::default(),
    )
    .expect_err("activation without standard primitives must fail");

    let mut source_instance = activate(
        ReactiveInstanceId::new(1, 0),
        &source_artifact,
        catalog.as_ref(),
        &ActivationFacts::default(),
    )
    .map_err(debug_error)?;
    let mut decoded_instance = activate(
        ReactiveInstanceId::new(2, 0),
        &decoded_artifact,
        catalog.as_ref(),
        &ActivationFacts::default(),
    )
    .map_err(debug_error)?;
    require(
        source_instance.plan.program_revision == source_artifact.revision(),
        "activated plan lost the artifact revision",
    )?;
    require(
        output_f64(&source_instance)? == 1.0 && state_f64(&source_instance)? == 1.0,
        "activated state did not start from the source initializer 1.0",
    )?;

    heading("4 / ACTIVATED RESIDENT PLAN");
    println!("artifact match  : PASS");
    println!("initial x/output: 1 / 1");
    println!(
        "plan generation: {}",
        source_instance.plan.plan_generation.get()
    );
    println!(
        "layout gen.    : {}",
        source_instance.plan.layout_generation.get()
    );
    println!("resident steps : {}", source_instance.plan.steps.len());
    println!(
        "execution nodes: {}",
        source_instance.plan.execution_node_count()
    );
    println!(
        "inputs/outputs : {} / {}",
        source_instance.plan.inputs.len(),
        source_instance.plan.outputs.len()
    );
    println!(
        "constraints    : {} ({:?})",
        source_instance.plan.constraints.len(),
        source_instance.plan.integrity_mode
    );
    println!("\nslot  role          storage       region(kind / offset / len)  shape");
    for slot in &source_instance.plan.slots {
        println!(
            "{:>4}  {:<13} {:<13} {:?} / {:>3} / {:>3}  {:?}",
            slot.artifact_id.get(),
            format!("{:?}", slot.role),
            format!("{:?}", slot.storage),
            slot.region.kind,
            slot.region.offset,
            slot.region.len,
            slot.shape,
        );
    }
    println!(
        "\nmissing catalog: PASS — activation rejected\n                 {}",
        compact_debug(&missing_catalog_error)
    );
    if matches!(dump, Dump::Activation | Dump::All) {
        println!(
            "\n--- FULL ACTIVATED PLAN (Debug) ---\n{:#?}",
            source_instance.plan
        );
    } else {
        println!("inspect more   : ./scripts/demo-r-stack.sh --dump activation");
    }

    let memory = &source_instance.plan.memory_plan;
    require(
        memory.budget_violations.is_empty(),
        "admitted plan has a budget violation",
    )?;
    heading("5 / PLANNED + REALIZED MEMORY");
    println!("value layouts  : {}", memory.values.len());
    println!("allocations    : {}", memory.allocations.len());
    println!("arenas         : {}", memory.arenas.len());
    println!("transfers      : {}", memory.transfers.len());
    println!("budget findings: {}", memory.budget_violations.len());
    println!(
        "demand (bytes) : persistent={} activation={} turn-peak={} transaction-peak={} cloned={} transfer={}",
        memory.peak.persistent_bytes,
        memory.peak.activation_bytes,
        memory.peak.turn_peak_bytes,
        memory.peak.transaction_peak_bytes,
        memory.peak.cloned_bytes,
        memory.peak.transfer_bytes,
    );
    println!(
        "state buffers  : {} bytes candidate / {} bytes dual-version",
        source_instance.state.candidate_bytes(),
        source_instance.state.dual_payload_bytes(),
    );
    println!("\narena  backing                     capacity  members  space");
    for arena in &memory.arenas {
        println!(
            "{:>5}  {:<27} {:>8}  {:>7}  {:?}",
            arena.id.get(),
            format!("{:?}", arena.backing),
            arena.capacity_bytes,
            arena.members.len(),
            arena.space,
        );
    }
    println!("\nstate/output placements");
    println!("slot  class             object  bytes  transaction");
    for value in memory.values.iter().filter(|value| {
        matches!(
            value.class,
            mech_engine::memory_planner::PlannedValueClass::State
                | mech_engine::memory_planner::PlannedValueClass::PublishedOutput
        )
    }) {
        println!(
            "{:>4}  {:<17} {:>6}  {:>5}  {:?}",
            value.slot.get(),
            format!("{:?}", value.class),
            value.object.get(),
            value.layout.capacity_bytes + value.layout.payload.required_bytes,
            value.transaction,
        );
    }
    println!(
        "output alias   : output 0 reads physical slot {}",
        source_instance.plan.outputs[0].slot.get()
    );
    if matches!(dump, Dump::Memory | Dump::All) {
        println!(
            "\n--- FULL POINTER-FREE MEMORY PLAN ---\n{}",
            memory.diagnostic_text()
        );
    } else {
        println!("inspect more   : ./scripts/demo-r-stack.sh --dump memory");
    }

    heading("6 / EXECUTION AGAINST A LITERAL ORACLE");
    println!(
        "The oracle is the literal list {:?}; it calls no Mech/runtime math.",
        EXPECTED_COMMITTED_OUTPUTS
    );
    println!("\nturn  expected  source artifact  decoded bytecode  epoch  result");
    let mut first_probe = None;
    for (index, expected) in EXPECTED_COMMITTED_OUTPUTS.iter().copied().enumerate() {
        let source_prepared = source_instance.prepare_turn(&[]).map_err(debug_error)?;
        if first_probe.is_none() {
            first_probe = Some(source_prepared.structural_probe());
        }
        let source_summary = source_prepared.publish().map_err(debug_error)?;
        let decoded_summary = decoded_instance.turn(&[]).map_err(debug_error)?;
        let source_value = output_f64(&source_instance)?;
        let decoded_value = output_f64(&decoded_instance)?;
        require(
            source_value == expected,
            &format!(
                "source artifact turn {} produced {source_value}, expected {expected}",
                index + 1
            ),
        )?;
        require(
            decoded_value == expected,
            &format!(
                "decoded artifact turn {} produced {decoded_value}, expected {expected}",
                index + 1
            ),
        )?;
        require(
            source_summary.program_revision == decoded_summary.program_revision
                && source_summary.before_epoch == decoded_summary.before_epoch
                && source_summary.after_epoch == decoded_summary.after_epoch
                && source_summary.state_hash == decoded_summary.state_hash
                && source_summary.touched_slots == decoded_summary.touched_slots
                && source_summary.changed_slots == decoded_summary.changed_slots
                && source_summary.dirty_nodes == decoded_summary.dirty_nodes,
            &format!("source and decoded receipts diverged on turn {}", index + 1),
        )?;
        println!(
            "{:>4}  {:>8.0}  {:>15.0}  {:>16.0}  {:>5}  COMMIT",
            index + 1,
            expected,
            source_value,
            decoded_value,
            source_summary.after_epoch.get(),
        );
    }
    let probe = first_probe.ok_or("no successful turn probe was captured")?;
    println!("\nsource=bytecode: PASS — every value and receipt matched");
    println!("candidate seed : {} bytes", probe.candidate_seed_bytes);
    println!(
        "candidate write: {} bytes",
        probe.candidate_materialized_bytes
    );
    println!(
        "published copies: {} bytes",
        probe.published_buffer_copy_bytes
    );
    println!("publication ops : {}", probe.publication_store_count);

    heading("7 / PROTECTIONS — REJECTION MUST LEAVE NO SCAR");

    let syntax_error = mech_syntax::parser::parse("~x := [1.0; 2.0").unwrap_err();
    println!(
        "[PASS] malformed source rejected\n       {}",
        compact_debug(&syntax_error)
    );

    let bad_type_tree = mech_syntax::parser::parse("~x := 1.0\nnext := x * \"two\"\nx = next\nx")
        .map_err(debug_error)?;
    let mut bad_type_compiler = CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        catalog.clone(),
    );
    let type_error = bad_type_compiler
        .plan_tree_with_services(&bad_type_tree, &mut NoMechExecutionServices)
        .expect_err("numeric/string multiplication must fail semantic typing");
    println!(
        "[PASS] incompatible source rejected by semantic typing\n       {}",
        compact_debug(&type_error)
    );

    let bytecode_error = decode_program_artifact_bytecode_v1(&bytecode[..bytecode.len() - 1])
        .expect_err("truncated bytecode must fail decoding");
    println!(
        "[PASS] truncated bytecode rejected before activation\n       {}",
        compact_debug(&bytecode_error)
    );

    let artifact_error = tampered_artifact(&source_artifact)
        .expect_err("non-canonical node identity must fail artifact validation");
    println!(
        "[PASS] tampered artifact rejected before receiving a revision\n       {}",
        compact_debug(&artifact_error)
    );

    let tiny_budget = ManagedMemoryBudget::new(1);
    let budget_error = activate_with_options(
        ReactiveInstanceId::new(91, 0),
        &source_artifact,
        catalog.as_ref(),
        &ActivationFacts::default(),
        ResidentActivationOptions {
            memory_budget: Some(tiny_budget.clone()),
            ..ResidentActivationOptions::default()
        },
    )
    .expect_err("a one-byte budget must not admit this plan");
    require(
        tiny_budget.used_bytes() == 0,
        "failed activation leaked memory budget",
    )?;
    println!(
        "[PASS] one-byte activation budget rejected atomically (used=0)\n       {}",
        compact_debug(&budget_error)
    );

    let protected_source = protection_snapshot(&source_instance)?;
    let protected_decoded = protection_snapshot(&decoded_instance)?;
    let source_integrity_error = source_instance
        .turn(&[])
        .expect_err("candidate 1021 must violate safe!");
    let decoded_integrity_error = decoded_instance
        .turn(&[])
        .expect_err("decoded candidate 1021 must violate safe!");
    require(
        matches!(
            source_integrity_error,
            ResidentExecutionError::Integrity { .. }
        ) && matches!(
            decoded_integrity_error,
            ResidentExecutionError::Integrity { .. }
        ),
        "unsafe candidate failed outside the integrity boundary",
    )?;
    require(
        protection_snapshot(&source_instance)? == protected_source,
        "failed source-artifact turn changed epoch, hash, state, or output",
    )?;
    require(
        protection_snapshot(&decoded_instance)? == protected_decoded,
        "failed decoded-artifact turn changed epoch, hash, state, or output",
    )?;
    println!(
        "[PASS] candidate {:.0} rejected by safe!; epoch/hash/state/output stayed ({}, {}, {:.0}, {:.0})\n       {}",
        EXPECTED_REJECTED_CANDIDATE,
        protected_source.0,
        protected_source.1,
        protected_source.2,
        protected_source.3,
        compact_debug(&source_integrity_error),
    );

    heading("WHAT THIS PROVES — AND WHAT IT DOES NOT");
    println!(
        "PROVED  parser -> typed plan -> artifact -> bytecode -> decoder -> activation -> memory -> turns"
    );
    println!("PROVED  the algorithm is a visible Mech graph, not an ekf/* Rust operation");
    println!("PROVED  seven literal expected values match both artifact routes exactly");
    println!(
        "PROVED  malformed/type-invalid/tampered/truncated/under-budget/unsafe cases are contained"
    );
    println!(
        "NOT     that Rust disappears: the four disclosed standard primitives are native implementations"
    );
    println!("NOT     an independent implementation of arbitrary EKF mathematics");

    heading("VERDICT");
    println!("ALL CLAIMED CHECKS PASSED");
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProtectionSnapshot(u64, u64, f64, f64);

fn protection_snapshot(instance: &ReactiveInstance) -> Result<ProtectionSnapshot, String> {
    Ok(ProtectionSnapshot(
        instance.published_epoch().get(),
        instance.published_state_hash(),
        state_f64(instance)?,
        output_f64(instance)?,
    ))
}

fn state_f64(instance: &ReactiveInstance) -> Result<f64, String> {
    let slots = instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .collect::<Vec<_>>();
    require(slots.len() == 1, "expected exactly one resident state slot")?;
    match instance
        .state_borrow(slots[0].artifact_id)
        .ok_or("resident state is unavailable")?
    {
        ResidentValueBorrow::F64 {
            values: [value], ..
        } => Ok(*value),
        other => Err(format!("resident state is not scalar f64: {other:?}")),
    }
}

fn output_f64(instance: &ReactiveInstance) -> Result<f64, String> {
    match instance
        .output_borrow(0)
        .ok_or("published output 0 is unavailable")?
    {
        ResidentValueBorrow::F64 {
            values: [value], ..
        } => Ok(*value),
        other => Err(format!("published output is not scalar f64: {other:?}")),
    }
}

fn tampered_artifact(
    artifact: &ProgramArtifact,
) -> Result<ProgramArtifact, mech_engine::ArtifactBuildError> {
    let mut nodes = artifact.nodes().to_vec();
    nodes[0].node = mech_core::NodeId::new(nodes.len() as u32 + 7);
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: artifact.bindings().to_vec().into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
        compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
}

struct ParseSummary {
    total: usize,
    kinds: BTreeMap<&'static str, usize>,
}

fn summarize_parse(program: &Program) -> ParseSummary {
    let mut total = 0;
    let mut kinds = BTreeMap::new();
    for section in &program.body.sections {
        for element in &section.elements {
            let SectionElement::MechCode(code) = element else {
                continue;
            };
            for (item, _) in code {
                total += 1;
                let kind = match item {
                    MechCode::Statement(statement) => statement_kind(statement),
                    MechCode::ActivationScope(_) => "activation scope",
                    MechCode::Expression(_) => "expression",
                    MechCode::FsmImplementation(_) => "FSM implementation",
                    MechCode::FsmSpecification(_) => "FSM specification",
                    MechCode::FunctionDefine(_) => "function definition",
                    MechCode::Import(_) => "module import",
                    MechCode::Comment(_) => "comment",
                    MechCode::Error(_, _) => "recovery error",
                };
                *kinds.entry(kind).or_insert(0) += 1;
            }
        }
    }
    ParseSummary { total, kinds }
}

fn statement_kind(statement: &Statement) -> &'static str {
    match statement {
        Statement::ImportDeclaration(_) => "import declaration",
        Statement::ExportDeclaration(_) => "export declaration",
        Statement::ContextDeclaration(_) => "context declaration",
        Statement::EnumDefine(_) => "enum definition",
        Statement::FsmDeclare(_) => "FSM declaration",
        Statement::KindDefine(_) => "kind definition",
        Statement::OpAssign(_) => "operator assignment",
        Statement::VariableAssign(_) => "state assignment",
        Statement::VariableDefine(_) => "variable definition",
        Statement::ContextSend(_) => "context send",
        Statement::InvariantDefine(_) => "integrity constraint",
        Statement::TupleDestructure(_) => "tuple destructure",
        Statement::SplitTable => "split table",
        Statement::FlattenTable => "flatten table",
    }
}

fn requested_dump() -> Result<Dump, String> {
    let mut arguments = std::env::args().skip(1);
    let Some(flag) = arguments.next() else {
        return Ok(Dump::None);
    };
    if flag == "--help" || flag == "-h" {
        println!(
            "Usage: r-stack-trust-proof [--dump parse|artifact|activation|memory|all]\n\
             Default output shows the complete source and concise live views of every stage."
        );
        std::process::exit(0);
    }
    if flag != "--dump" {
        return Err(format!("unknown argument `{flag}`; try --help"));
    }
    let value = arguments
        .next()
        .ok_or("--dump requires parse, artifact, activation, memory, or all")?;
    if arguments.next().is_some() {
        return Err("unexpected arguments after --dump value".to_owned());
    }
    match value.as_str() {
        "parse" => Ok(Dump::Parse),
        "artifact" => Ok(Dump::Artifact),
        "activation" => Ok(Dump::Activation),
        "memory" => Ok(Dump::Memory),
        "all" => Ok(Dump::All),
        _ => Err(format!("unknown dump `{value}`; try --help")),
    }
}

fn revision_hex(artifact: &ProgramArtifact) -> String {
    artifact
        .revision()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn compact_debug(value: &impl Debug) -> String {
    let text = format!("{value:?}").replace('\n', " ");
    let mut compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() > 280 {
        compact.truncate(277);
        compact.push_str("...");
    }
    compact
}

fn debug_error(error: impl Debug) -> String {
    format!("{error:?}")
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}

fn heading(title: &str) {
    println!("\n==============================================================================");
    println!("{title}");
    println!("==============================================================================");
}
