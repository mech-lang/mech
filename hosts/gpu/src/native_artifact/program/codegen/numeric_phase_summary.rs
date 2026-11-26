//! Complete summary work is paid only at its exact selected source operation.
//! Missing finite fuel resumes before that original action, on the same ledger.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{Block, InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn debit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    fuel: usize,
    shadow: usize,
    proof: &Proof,
    code: CodePlan<'_>,
    carrier: u32,
    site: usize,
    miss: Block,
) -> Result<(), NativeCompileError> {
    let phase = proof
        .phase
        .as_ref()
        .ok_or_else(|| backend("summary lacks its phase account"))?;
    let debit = phase
        .summaries
        .iter()
        .find(|debit| debit.site == site)
        .ok_or_else(|| backend("summary lacks its source-derived price"))?;
    // Keep enough for every still possible base traversal and the complete
    // source replay after this attempt. Root inputs remain current throughout
    // the traversal; mutable descendants already use checked child funding.
    let (reserve, work_valid) = super::numeric_fuel::envelope(
        b,
        words,
        code,
        proof,
        shadow,
        carrier,
        &phase.compute_bound,
    )?;
    let (reserve_comparison, comparison_valid) = super::numeric_fuel::envelope(
        b,
        words,
        code,
        proof,
        shadow,
        carrier,
        &phase.comparison_bound,
    )?;
    let work = b.ins().iconst(types::I64, debit.compute as i64);
    let compare = b.ins().iconst(types::I64, debit.comparison as i64);
    let (work, work_overflow) = b.ins().uadd_overflow(reserve, work);
    let (compare, comparison_overflow) = b.ins().uadd_overflow(reserve_comparison, compare);
    let compute = words.read(b, fuel + 1);
    let comparison = words.read(b, fuel + 2);
    let enough = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, compute, work);
    let enough_comparison = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, comparison, compare);
    let enough = b.ins().band(enough, enough_comparison);
    let valid = b.ins().band(work_valid, comparison_valid);
    let overflow = b.ins().bor(work_overflow, comparison_overflow);
    let no_overflow = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    let valid = b.ins().band(valid, no_overflow);
    let enough = b.ins().band(enough, valid);
    let funded = b.create_block();
    b.ins().brif(enough, funded, &[], miss, &[]);
    b.switch_to_block(funded);
    let work = b.ins().iconst(types::I64, debit.compute as i64);
    let compare = b.ins().iconst(types::I64, debit.comparison as i64);
    let compute = b.ins().isub(compute, work);
    let comparison = b.ins().isub(comparison, compare);
    words.write(b, fuel + 1, compute);
    words.write(b, fuel + 2, comparison);
    let count = words.read(b, shadow + debit.counter);
    let count = b.ins().iadd_imm(count, 1);
    words.write(b, shadow + debit.counter, count);
    Ok(())
}
