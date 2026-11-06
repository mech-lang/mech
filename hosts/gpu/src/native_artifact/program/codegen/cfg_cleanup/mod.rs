//! Fold constant native branches before the single backend data pass, then
//! retire unreachable blocks and their phi inputs. No runtime predicate,
//! source operation, failure or work debit is assumed away.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::CfgCleanup;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::backend;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{
    Function, InstBuilder, InstructionData, Opcode, Value, ValueDef, condcodes::IntCC,
};
use cranelift_codegen::isa::TargetIsa;

const MAX_ROUNDS: usize = 8;
const MAX_CONSTANT_DEPTH: usize = 16;
const MAX_CONSTANT_STEPS: usize = 64;

mod boolean_conditions;

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
    let changes = function
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let InstructionData::Brif { arg, blocks, .. } = &function.dfg.insts[inst] else {
                return None;
            };
            let selected = &blocks[usize::from(!constant_condition(function, *arg)?)];
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
    optimize_ordered(controls, context, isa, true)
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
        before_blocks,
        before_instructions,
        after_blocks: before_blocks,
        after_instructions: before_instructions,
        fixed_point: !enabled,
        pre_codegen_data_passes: 0,
    };
    if !enabled {
        return Ok(report);
    }
    // Retire literal source specializations BEFORE data optimization. Running
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
    let (blocks, instructions) = counts(&context.func);
    report.after_blocks = blocks;
    report.after_instructions = instructions;

    Ok(report)
}

#[cfg(test)]
mod tests;
