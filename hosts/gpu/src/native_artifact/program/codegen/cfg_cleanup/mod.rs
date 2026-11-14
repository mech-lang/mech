//! Fold proved native branches, then retire unreachable blocks and values
//! with no path to control or effects before the single backend data pass.
//! Trapping loads, source failures and work debits retain their ordering.
mod adjacent_addresses;
pub(in crate::native_artifact) mod admitted_lengths;
mod boolean_words;
pub(in crate::native_artifact) mod composed_reads;
mod current_predicates;
mod dead_values;
pub(in crate::native_artifact) mod defaulted_read_setup;
pub(in crate::native_artifact) mod future_store;
pub(in crate::native_artifact) mod grouped_histogram;
pub(in crate::native_artifact) mod identity_routes;
pub(in crate::native_artifact) mod independent_private_read;
mod known_widths;
mod local_phis;
pub(in crate::native_artifact) mod packed_halfword_reads;
mod preheaders;
pub(in crate::native_artifact) mod reduction_bound;
mod repeated_bounds;
pub(in crate::native_artifact) mod selected_continuations;
mod selected_homes;
mod selected_increments;
mod selected_ranges;
pub(in crate::native_artifact) mod shared_clamp;
mod threaded_edges;
mod unsigned_intervals;
mod window_bounds;
mod word_ranges;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::CfgCleanup;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::backend;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{
    Block, Function, InstBuilder, InstructionData, Opcode, Value, ValueDef, condcodes::IntCC,
};
use cranelift_codegen::isa::TargetIsa;

const MAX_ROUNDS: usize = 8;
const MAX_CONSTANT_DEPTH: usize = 16;
const MAX_CONSTANT_STEPS: usize = 64;

mod boolean_conditions;
pub(super) use boolean_conditions::masked_integer_comparison;

fn counts(function: &Function) -> (usize, usize) {
    (
        function.layout.blocks().count(),
        function
            .layout
            .blocks()
            .map(|block| function.layout.block_insts(block).count())
            .sum(),
    )
}
fn integer_mask(bits: u32) -> Option<u64> {
    (bits > 0 && bits <= 64).then(|| u64::MAX >> (64 - bits))
}

fn signed_word(value: u64, bits: u32) -> i64 {
    ((value << (64 - bits)) as i64) >> (64 - bits)
}

fn integer_comparison(condition: IntCC, left: u64, right: u64, bits: u32) -> bool {
    use IntCC::*;
    match condition {
        Equal => left == right,
        NotEqual => left != right,
        UnsignedLessThan => left < right,
        UnsignedGreaterThanOrEqual => left >= right,
        UnsignedGreaterThan => left > right,
        UnsignedLessThanOrEqual => left <= right,
        SignedLessThan => signed_word(left, bits) < signed_word(right, bits),
        SignedGreaterThanOrEqual => signed_word(left, bits) >= signed_word(right, bits),
        SignedGreaterThan => signed_word(left, bits) > signed_word(right, bits),
        SignedLessThanOrEqual => signed_word(left, bits) <= signed_word(right, bits),
    }
}

// Only actual constant integer expressions qualify. This evaluator never
// reads a source home, memory, a block parameter or a call result; each root
// has both a depth bound and a shared step budget. Unsupported/trapping
// expressions retain their branch. Work/debits are separate emitted effects.
fn constant_integer(
    function: &Function,
    value: Value,
    depth: usize,
    steps: &mut usize,
) -> Option<u64> {
    if depth == 0 || *steps == 0 {
        return None;
    }
    *steps -= 1;
    let value = function.dfg.resolve_aliases(value);
    let ty = function.dfg.value_type(value);
    if !ty.is_int() {
        return None;
    }
    let mask = integer_mask(ty.bits())?;
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    let mut get = |argument| constant_integer(function, argument, depth - 1, steps);
    let result = match function.dfg.insts[inst] {
        InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
            ..
        } => imm.bits() as u64,
        InstructionData::Unary { opcode, arg } => match opcode {
            Opcode::Uextend | Opcode::Ireduce => get(arg)?,
            Opcode::Sextend => signed_word(get(arg)?, function.dfg.value_type(arg).bits()) as u64,
            _ => return None,
        },
        InstructionData::IntCompare { cond, args, .. } => {
            let [left, right] = args;
            if function.dfg.value_type(left) != function.dfg.value_type(right) {
                return None;
            }
            let bits = function.dfg.value_type(left).bits();
            integer_mask(bits)?;
            if function.dfg.resolve_aliases(left) == function.dfg.resolve_aliases(right) {
                // The two uses refer to the same already-computed scalar.
                // Its producer, including any load/call/trap, stays in place.
                return Some(u64::from(integer_comparison(cond, 0, 0, bits)));
            }
            u64::from(integer_comparison(cond, get(left)?, get(right)?, bits))
        }
        InstructionData::IntCompareImm { cond, arg, imm, .. } => {
            let bits = function.dfg.value_type(arg).bits();
            let right = imm.bits() as u64 & integer_mask(bits)?;
            u64::from(integer_comparison(cond, get(arg)?, right, bits))
        }
        InstructionData::Binary { opcode, args } => {
            let [left, right] = args.map(&mut get);
            let (left, right) = (left?, right?);
            match opcode {
                Opcode::Iadd => left.wrapping_add(right),
                Opcode::Isub => left.wrapping_sub(right),
                Opcode::Imul => left.wrapping_mul(right),
                Opcode::Band => left & right,
                Opcode::Bor => left | right,
                Opcode::Bxor => left ^ right,
                _ => return None,
            }
        }
        InstructionData::BinaryImm64 { opcode, arg, imm } => {
            let left = get(arg)?;
            let right = imm.bits() as u64 & mask;
            match opcode {
                Opcode::IaddImm => left.wrapping_add(right),
                Opcode::ImulImm => left.wrapping_mul(right),
                Opcode::BandImm => left & right,
                Opcode::BorImm => left | right,
                Opcode::BxorImm => left ^ right,
                _ => return None,
            }
        }
        InstructionData::Ternary {
            opcode: Opcode::Select,
            args,
        } => {
            let selected = if get(args[0])? == 0 { args[2] } else { args[1] };
            get(selected)?
        }
        _ => return None,
    };
    Some(result & mask)
}

fn constant_condition(function: &Function, value: Value) -> Option<bool> {
    let mut steps = MAX_CONSTANT_STEPS;
    constant_integer(function, value, MAX_CONSTANT_DEPTH, &mut steps).map(|value| value != 0)
}
fn fold(function: &mut Function) -> usize {
    let mut facts = word_ranges::Facts::new(function);
    let changes = function
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let InstructionData::Brif { arg, blocks, .. } = &function.dfg.insts[inst] else {
                return None;
            };
            let condition = constant_condition(function, *arg).or_else(|| facts.condition(*arg))?;
            let selected = &blocks[usize::from(!condition)];
            Some((
                inst,
                selected.block(&function.dfg.value_lists),
                selected.args(&function.dfg.value_lists).collect::<Vec<_>>(),
            ))
        })
        .collect::<Vec<_>>();
    for (inst, target, args) in &changes {
        function.dfg.replace(*inst).jump(*target, args);
    }
    changes.len()
}

// Backend data optimization also knows these identities, but it runs after
// invariant block-parameter removal. Resolve them first so a loop's x+0
// recurrence does not look like a new independent value to that earlier pass.
// Only one-result, same-width, scalar wrapping integer operations qualify.
// Operand producers and every separately emitted check/debit remain intact.
fn identity_operand(function: &Function, inst: cranelift_codegen::ir::Inst) -> Option<Value> {
    let &[result] = function.dfg.inst_results(inst) else {
        return None;
    };
    let ty = function.dfg.value_type(result);
    if !ty.is_int() {
        return None;
    }
    let mask = integer_mask(ty.bits())?;
    let literal = |value| {
        let mut steps = MAX_CONSTANT_STEPS;
        constant_integer(function, value, MAX_CONSTANT_DEPTH, &mut steps)
    };
    let identity = |opcode, value, constant, right| {
        let neutral = match opcode {
            Opcode::Iadd
            | Opcode::IaddImm
            | Opcode::Bor
            | Opcode::BorImm
            | Opcode::Bxor
            | Opcode::BxorImm => constant == 0,
            Opcode::Isub => right && constant == 0,
            Opcode::Imul | Opcode::ImulImm => constant == 1,
            Opcode::Band | Opcode::BandImm => constant == mask,
            _ => false,
        };
        (neutral && function.dfg.value_type(value) == ty)
            .then(|| function.dfg.resolve_aliases(value))
    };
    match function.dfg.insts[inst] {
        InstructionData::BinaryImm64 { opcode, arg, imm } => {
            identity(opcode, arg, imm.bits() as u64 & mask, true)
        }
        InstructionData::Binary {
            opcode,
            args: [left, right],
        } => {
            if let Some(constant) = literal(right) {
                if let Some(value) = identity(opcode, left, constant, true) {
                    return Some(value);
                }
            }
            literal(left).and_then(|constant| identity(opcode, right, constant, false))
        }
        _ => None,
    }
}

fn fold_identities(function: &mut Function) -> usize {
    let instructions = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .collect::<Vec<_>>();
    let mut changed = 0;
    for inst in instructions {
        let Some(source) = identity_operand(function, inst) else {
            continue;
        };
        let result = function.dfg.inst_results(inst)[0];
        if result == source {
            continue;
        }
        function.dfg.detach_inst_results(inst);
        function.dfg.change_to_alias(result, source);
        function.layout.remove_inst(inst);
        changed += 1;
    }
    changed
}

pub(in crate::native_artifact) fn optimize(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<CfgCleanup, NativeCompileError> {
    let mut report = optimize_ordered(controls, context, isa, true)?;
    finish_imported_passes(controls, context, isa, &mut report)?;
    Ok(report)
}

// Numeric range proofs still consume exact equality-to-one facts on phi
// words. Keep that vocabulary until the existing guard pass has finished.
pub(in crate::native_artifact) fn optimize_before_unsigned_guards(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<CfgCleanup, NativeCompileError> {
    optimize_ordered(controls, context, isa, false)
}

pub(in crate::native_artifact) fn finish_boolean_conditions(
    controls: &CompilationControls,
    context: &mut Context,
    report: &mut CfgCleanup,
) {
    if !report.enabled || !controls.enabled(OptimizationPass::BooleanConditions) {
        return;
    }
    let simplified = boolean_conditions::optimize(&mut context.func);
    report.boolean_conditions_simplified += simplified;
    if simplified != 0 {
        // Inverted tests swap successor order. Even with the same edge set,
        // Cranelift's cached postorder must be refreshed.
        context.compute_cfg();
        context.compute_domtree();
    }
}

fn optimize_ordered(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
    early_boolean_conditions: bool,
) -> Result<CfgCleanup, NativeCompileError> {
    let (before_blocks, before_instructions) = counts(&context.func);

    let enabled = !!controls.enabled(OptimizationPass::CfgCleanup);
    let mut report = CfgCleanup {
        enabled,
        rounds: 0,
        constant_branches_removed: 0,
        neutral_operations_removed: 0,
        boolean_conditions_simplified: 0,
        unsigned_intervals_combined: 0,
        selected_range_branches_removed: 0,
        boolean_phi_values: 0,
        known_width_branches_removed: 0,
        current_predicate_branches: 0,
        selected_home_constant_operands: 0,
        selected_home_merge_parameters: 0,
        selected_increment_guards_removed: 0,
        identity_edges: 0,
        loop_edges: 0,
        bitwise_constant_branches: 0,
        window_bounds_combined: 0,
        ordered_capture_spans: 0,
        adjacent_addresses_reused: 0,
        before_blocks,
        before_instructions,
        after_blocks: before_blocks,
        after_instructions: before_instructions,
        fixed_point: !enabled,
        pre_codegen_data_passes: 0,
        local_phi_parameters_removed: 0,
        local_phi_preheaders: 0,
        dead_parameters_removed: 0,
        dead_instructions_removed: 0,
        dead_loads_removed: 0,
    };
    if !enabled {
        return Ok(report);
    }
    // Retire proved source specializations BEFORE data optimization. Running
    // the egraph repeatedly here and once again in define_function can change
    // placement/rematerialization and increase register pressure. Leave exactly
    // one ordinary data/speed pass to the final backend compilation.
    for _ in 0..MAX_ROUNDS {
        if controls.enabled(OptimizationPass::EarlyIdentities) {
            report.neutral_operations_removed += fold_identities(&mut context.func);
        }
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
        if early_boolean_conditions {
            finish_boolean_conditions(controls, context, &mut report);
        }
        report.rounds += 1;
        let removed = fold(&mut context.func);
        report.constant_branches_removed += removed;
        if removed == 0 {
            report.fixed_point = true;
            break;
        }
    }
    // A zero-fold round already ended with valid reachability and cleaned phi
    // inputs. Repeating that exact sweep cannot expose another branch. Only a
    // final fold at the round limit leaves changed edges requiring cleanup.
    if !report.fixed_point {
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
    }
    report.local_phi_preheaders =
        preheaders::normalize(&mut context.func, &context.cfg, &context.domtree);
    if report.local_phi_preheaders != 0 {
        context.compute_cfg();
        context.compute_domtree();
    }
    report.local_phi_parameters_removed =
        local_phis::simplify(&mut context.func, &context.cfg, &context.domtree);
    report.unsigned_intervals_combined = unsigned_intervals::combine(&mut context.func);
    (
        report.selected_range_branches_removed,
        report.boolean_phi_values,
    ) = selected_ranges::fold(&mut context.func, &context.cfg, &context.domtree);
    if report.selected_range_branches_removed != 0 {
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
    }
    if controls.enabled(OptimizationPass::ProvedWordWidths) {
        report.known_width_branches_removed =
            known_widths::fold(&mut context.func, &context.cfg, &context.domtree);
        if report.known_width_branches_removed != 0 {
            context.compute_cfg();
            context.compute_domtree();
            context.eliminate_unreachable_code(isa).map_err(backend)?;
            context.remove_constant_phis(isa).map_err(backend)?;
        }
    }
    if controls.enabled(OptimizationPass::DemandLiveness) {
        let dead = dead_values::eliminate(&mut context.func);
        report.dead_parameters_removed = dead.parameters;
        report.dead_instructions_removed = dead.instructions;
        report.dead_loads_removed = dead.loads;
    }
    let (blocks, instructions) = counts(&context.func);
    report.after_blocks = blocks;
    report.after_instructions = instructions;

    Ok(report)
}

#[cfg(test)]
mod tests;

pub(in crate::native_artifact) fn forward_identity_routes(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<identity_routes::Report, NativeCompileError> {
    forward_identity_routes_from(controls, context, isa, None)
}

pub(in crate::native_artifact) fn forward_identity_routes_from(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
    origins: Option<&[Block]>,
) -> Result<identity_routes::Report, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::IdentityRoutes)
    {
        return Ok(identity_routes::Report::default());
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let report =
        identity_routes::optimize(&mut context.func, &context.cfg, &context.domtree, origins);
    if report.threaded_edges != 0 {
        optimize_ordered(controls, context, isa, true)?;
        context.func.dfg.resolve_all_aliases();
        context.verify(isa).map_err(backend)?;
    }
    Ok(report)
}

pub(in crate::native_artifact) fn refine_current_predicates(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<current_predicates::Report, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::CurrentPredicates)
    {
        return Ok(current_predicates::Report::default());
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let report = current_predicates::fold(&mut context.func, &context.cfg, &context.domtree);
    if report.removed_branches != 0 {
        optimize_ordered(controls, context, isa, true)?;
        context.func.dfg.resolve_all_aliases();
        context.verify(isa).map_err(backend)?;
    }
    Ok(report)
}
pub(in crate::native_artifact) fn reuse_dominating_window_facts(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup) {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let replaced =
        current_predicates::relations::fold(&mut context.func, &context.cfg, &context.domtree);
    if replaced != 0 {
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
        context.verify(isa).map_err(backend)?;
    }
    Ok(replaced)
}

enum ScalarSpanProof {
    CheckedWindow,
    SelectedRead,
    PackedBase,
}

pub(in crate::native_artifact) fn prove_scalar_spans(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    fold_scalar_span_guards(controls, context, isa, ScalarSpanProof::CheckedWindow)
}

pub(in crate::native_artifact) fn prove_packed_bases(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    fold_scalar_span_guards(controls, context, isa, ScalarSpanProof::PackedBase)
}

pub(in crate::native_artifact) fn coalesce_equal_join_arguments(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup) {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    let removed = local_phis::equal_arguments::simplify(&mut context.func, &context.cfg);
    if removed != 0 {
        context.func.dfg.resolve_all_aliases();
        let branches = fold(&mut context.func);
        if branches != 0 {
            context.compute_cfg();
            context.compute_domtree();
            context.eliminate_unreachable_code(isa).map_err(backend)?;
            context.remove_constant_phis(isa).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("constant branches after equal argument coalescing: {branches}");
            }
        }
        context.verify(isa).map_err(backend)?;
    }
    Ok(removed)
}

pub(in crate::native_artifact) fn reuse_selected_read_bounds(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    fold_scalar_span_guards(controls, context, isa, ScalarSpanProof::SelectedRead)
}

fn fold_scalar_span_guards(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
    source: ScalarSpanProof,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup) {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let replaced = current_predicates::scalar_spans::fold(
        &mut context.func,
        &context.cfg,
        &context.domtree,
        source,
    );
    if replaced != 0 {
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
        context.verify(isa).map_err(backend)?;
    }
    Ok(replaced)
}

pub(in crate::native_artifact) fn fold_proved_widths(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::ProvedWordWidths)
    {
        return Ok(0);
    }
    context.compute_cfg();
    context.compute_domtree();
    let removed = known_widths::fold(&mut context.func, &context.cfg, &context.domtree);
    if removed != 0 {
        optimize_ordered(controls, context, isa, true)?;
        context.func.dfg.resolve_all_aliases();
        context.verify(isa).map_err(backend)?;
    }
    Ok(removed)
}

pub(in crate::native_artifact) fn compose_readonly_lookups<M: cranelift_module::Module>(
    controls: &CompilationControls,
    context: &mut Context,
    module: &mut M,
    tables: &mut composed_reads::DenseTables,
    capture: &composed_reads::Capture<'_>,
) -> Result<Option<composed_reads::Admission>, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup) {
        return Ok(None);
    }
    context.compute_cfg();
    context.compute_domtree();
    let selected = composed_reads::compose(
        module,
        tables,
        &mut context.func,
        &context.cfg,
        &context.domtree,
        capture,
    )?;
    if selected.is_some() {
        optimize(controls, context, module.isa())?;
        context.verify(module.isa()).map_err(backend)?;
    }
    Ok(selected)
}

pub(in crate::native_artifact) fn fold_ordered_capture_spans(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::OrderedCaptureSpans)
    {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let selected =
        window_bounds::ordered_spans::fold(&mut context.func, &context.cfg, &context.domtree);
    if selected != 0 {
        context.verify(isa).map_err(backend)?;
    }
    Ok(selected)
}

pub(in crate::native_artifact) fn reuse_adjacent_addresses(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::AdjacentAddresses)
    {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let changed = adjacent_addresses::fold(&mut context.func, &context.cfg, &context.domtree);
    if changed != 0 {
        context.verify(isa).map_err(backend)?;
    }
    Ok(changed)
}

pub(in crate::native_artifact) fn combine_window_bounds(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::FixedWindowBounds)
    {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let removed = window_bounds::combine(&mut context.func, &context.cfg, &context.domtree);
    if removed != 0 {
        optimize_ordered(controls, context, isa, true)?;
        context.func.dfg.resolve_all_aliases();
        context.verify(isa).map_err(backend)?;
    }
    Ok(removed)
}

/// A current loop edge may bypass proved pure dispatch, carrying live-outs
/// through typed destination phis. Ordinary entries keep their original route.
pub(in crate::native_artifact) fn thread_loop_edges(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<threaded_edges::Report, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::LoopEdgeThreading)
    {
        return Ok(threaded_edges::Report::default());
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let report = threaded_edges::optimize(&mut context.func, &context.cfg, &context.domtree);
    if report.threaded_edges != 0 {
        // No data/egraph pass is added. The existing structural cleanup now
        // retires dispatch tags whose only remaining incoming value is fixed.
        optimize_ordered(controls, context, isa, true)?;
        context.func.dfg.resolve_all_aliases();
        context.verify(isa).map_err(backend)?;
    }
    Ok(report)
}

/// Run retained DEFLATE SSA passes after the ordinary checked numeric guard
/// proofs. They consume immutable machine words, never runtime permissions.
pub(in crate::native_artifact) fn finish_imported_passes(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
    report: &mut CfgCleanup,
) -> Result<(), NativeCompileError> {
    if !report.enabled {
        return Ok(());
    }
    if controls.enabled(OptimizationPass::ScopedBitwiseConstants) {
        let changed = fold_bitwise_constants(controls, &mut context.func);
        report.bitwise_constant_branches += changed;
        if changed != 0 {
            optimize_ordered(controls, context, isa, true)?;
        }
    }
    report.loop_edges += thread_loop_edges(controls, context, isa)?.threaded_edges;
    report.identity_edges += forward_identity_routes(controls, context, isa)?.threaded_edges;
    report.current_predicate_branches +=
        refine_current_predicates(controls, context, isa)?.removed_branches;
    report.known_width_branches_removed += fold_proved_widths(controls, context, isa)?;
    report.window_bounds_combined += combine_window_bounds(controls, context, isa)?;
    report.ordered_capture_spans += fold_ordered_capture_spans(controls, context, isa)?;
    report.adjacent_addresses_reused += reuse_adjacent_addresses(controls, context, isa)?;
    let homes = rematerialize_boolean_homes(controls, context, isa)?;
    report.selected_home_constant_operands += homes.constant_operands;
    report.selected_home_merge_parameters += homes.merge_parameters;
    if controls.enabled(OptimizationPass::SelectedUnsignedIncrements) {
        context.compute_cfg();
        context.compute_domtree();
        let removed =
            selected_increments::optimize(&mut context.func, &context.cfg, &context.domtree);
        report.selected_increment_guards_removed += removed;
        if removed != 0 {
            optimize_ordered(controls, context, isa, true)?;
            context.func.dfg.resolve_all_aliases();
            context.verify(isa).map_err(backend)?;
        }
    }
    (report.after_blocks, report.after_instructions) = counts(&context.func);
    Ok(())
}

pub(in crate::native_artifact) fn rematerialize_boolean_homes(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<selected_homes::Report, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup)
        || !controls.enabled(OptimizationPass::ScalarEdgeHomes)
    {
        return Ok(selected_homes::Report::default());
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let report = selected_homes::optimize(&mut context.func, &context.cfg, &context.domtree);
    if report.constant_operands != 0 || report.merge_parameters != 0 {
        optimize_ordered(controls, context, isa, true)?;
        context.func.dfg.resolve_all_aliases();
        context.verify(isa).map_err(backend)?;
    }
    Ok(report)
}

pub(in crate::native_artifact) fn fold_bitwise_constants(
    controls: &CompilationControls,
    function: &mut Function,
) -> usize {
    if !controls.enabled(OptimizationPass::CfgCleanup) {
        return 0;
    }
    let mut facts = word_ranges::Facts::with_bitwise_constants(function);
    let changes = function
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let InstructionData::Brif { arg, blocks, .. } = &function.dfg.insts[inst] else {
                return None;
            };
            let selected = &blocks[usize::from(!facts.condition(*arg)?)];
            Some((
                inst,
                selected.block(&function.dfg.value_lists),
                selected.args(&function.dfg.value_lists).collect::<Vec<_>>(),
            ))
        })
        .collect::<Vec<_>>();
    for (inst, target, args) in &changes {
        function.dfg.replace(*inst).jump(*target, args);
    }
    changes.len()
}

/// Route a repeated unsigned check to its original rejection on the first false edge.
pub(in crate::native_artifact) fn share_repeated_bounds(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::CfgCleanup) {
        return Ok(0);
    }
    context.func.dfg.resolve_all_aliases();
    context.compute_cfg();
    context.compute_domtree();
    let changed = repeated_bounds::fold(&mut context.func, &context.domtree);
    if changed != 0 {
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
        context.verify(isa).map_err(backend)?;
    }
    Ok(changed)
}
