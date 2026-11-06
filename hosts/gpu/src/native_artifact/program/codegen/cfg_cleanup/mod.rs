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

pub(in crate::native_artifact) fn optimize(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<CfgCleanup, NativeCompileError> {
    let (before_blocks, before_instructions) = counts(&context.func);

    let enabled = !!controls.enabled(OptimizationPass::CfgCleanup);
    let mut report = CfgCleanup {
        enabled,
        rounds: 0,
        constant_branches_removed: 0,
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
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
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
