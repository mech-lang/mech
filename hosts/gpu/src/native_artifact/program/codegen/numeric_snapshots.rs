//! Physical consumers at existing numeric boundaries; source operations remain.
use super::*;
use crate::native_artifact::program::prepare::tail::Region;

pub(super) fn source_words(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    values: &[u32],
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let CodePlan::Program(program) = code else {
        return Err(backend("numeric snapshot lacks its canonical program"));
    };
    let mut live = BTreeSet::new();
    for &value in values {
        if program.metadata().plan.values()[value as usize]
            .scalar_kind()
            .is_some()
        {
            live.insert(value as usize);
        }
        if let Some(fields) = proof.fields(value) {
            live.extend(fields.iter().flatten().map(|field| shadow + field));
        }
        if let Some((base, count)) = program.metadata().scalar_aggregates.slot_at(value as usize) {
            live.extend(base..base + count);
        }
    }
    Ok(live)
}

pub(super) fn spill(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    locals: Value,
    code: CodePlan<'_>,
    region: &Region,
    mode: i64,
    continuation: Option<usize>,
    values: &[u32],
    registered: &BTreeSet<usize>,
) -> Result<usize, NativeCompileError> {
    let live = live_words(code, region, mode, continuation, values, registered)?;
    words.spill_live(b, locals, &live)?;
    Ok(live.len())
}

/// Exact existing physical consumers, shared by canonical and dense exits.
pub(in crate::native_artifact::program::codegen) fn live_words(
    code: CodePlan<'_>,
    region: &Region,
    mode: i64,
    continuation: Option<usize>,
    values: &[u32],
    registered: &BTreeSet<usize>,
) -> Result<BTreeSet<usize>, NativeCompileError> {
    let proof = region
        .numeric
        .as_ref()
        .ok_or_else(|| backend("numeric snapshot lacks its checked proof"))?;
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric snapshot lacks its private home"))?;
    let mut live = (region.words..region.words + 3)
        .chain(
            tail::numeric_exit_words(region, mode, continuation)?
                .into_iter()
                .map(|word| shadow + word),
        )
        .collect::<BTreeSet<_>>();
    live.extend(source_words(code, proof, shadow, values)?);
    // Non-SSA values retain their ordinary canonical owner/home. Every
    // synchronization lane above is registered in this numeric word bank.
    live.retain(|word| registered.contains(word));
    Ok(live)
}
