//! Checked current-account fuel at exact synchronous child source boundaries.
use super::*;

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    options: crate::native_artifact::codegen::options::OptimizationOptions,
    words: &Words,
    region: &crate::native_artifact::program::prepare::tail::Region,
    proof: &Proof,
    site: usize,
    frame: Value,
    locals: Value,
    hooks: &Hooks,
    ordinary: &[Option<Block>],
    error: Block,
    boundaries: &mut BTreeSet<Block>,
) -> Result<(), NativeCompileError> {
    let Some(phase) = proof.phase.as_ref().filter(|phase| phase.checked_children) else {
        return Ok(());
    };
    let Some(debit) = phase
        .loops
        .iter()
        .skip(1)
        .find(|debit| debit.initialize == site || debit.header == site)
    else {
        return Ok(());
    };
    let shadow = region
        .shadow
        .ok_or_else(|| backend("child fuel lacks scalar homes"))?;
    let first = shadow
        + debit
            .first_visit
            .ok_or_else(|| backend("child fuel lacks its initial header flag"))?;
    let check = b.create_block();
    let done = b.create_block();
    if site == debit.header {
        let paid = words.read(b, first);
        b.ins().brif(paid, done, &[], check, &[]);
    } else {
        b.ins().jump(check, &[]);
    }
    b.switch_to_block(check);
    let compute = words.read(b, region.words + 1);
    let comparison = words.read(b, region.words + 2);
    let enough = b.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        compute,
        debit.compute as i64,
    );
    let enough_comparison = b.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        comparison,
        debit.comparison as i64,
    );
    let enough = b.ins().band(enough, enough_comparison);
    let funded = b.create_block();
    let cold = b.create_block();
    b.set_cold_block(cold);
    boundaries.insert(cold);
    b.ins().brif(enough, funded, &[], cold, &[]);
    b.switch_to_block(cold);
    words.spill_all(b, locals)?;
    set_origin(b, frame, site as u64);
    let origin = b.ins().iconst(types::I64, site as i64);
    tail::leave_at(
        options,
        b,
        locals,
        region,
        origin,
        7,
        Some(site),
        hooks,
        error,
    )?;
    let one = b.ins().iconst(types::I64, 1);
    b.ins().store(
        tail::flags(options, region),
        one,
        locals,
        offset(
            region
                .cold_resume()
                .ok_or_else(|| backend("child fuel lacks its ordinary resume marker"))?,
        )?,
    );
    b.ins().jump(
        ordinary[site]
            .ok_or_else(|| backend("child fuel lacks its original source continuation"))?,
        &[],
    );
    b.switch_to_block(funded);
    let compute = b.ins().iadd_imm(compute, -(debit.compute as i64));
    let comparison = b.ins().iadd_imm(comparison, -(debit.comparison as i64));
    words.write(b, region.words + 1, compute);
    words.write(b, region.words + 2, comparison);
    let count = words.read(b, shadow + debit.counter);
    let count = b.ins().iadd_imm(count, 1);
    words.write(b, shadow + debit.counter, count);
    let paid = b
        .ins()
        .iconst(types::I64, i64::from(site == debit.initialize));
    words.write(b, first, paid);
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    if site == debit.header {
        let zero = b.ins().iconst(types::I64, 0);
        words.write(b, first, zero);
    }
    Ok(())
}
