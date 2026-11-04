//! Physical consumers at existing numeric boundaries; source operations remain.
use super::*;
use crate::native_artifact::program::prepare::tail::Region;

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
    let proof = region
        .numeric
        .as_ref()
        .ok_or_else(|| backend("numeric snapshot lacks its checked proof"))?;
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric snapshot lacks its private home"))?;
    let CodePlan::Program(program) = code else {
        return Err(backend("numeric snapshot lacks its canonical program"));
    };
    let mut live = (region.words..region.words + 3)
        .chain(
            tail::numeric_exit_words(region, mode, continuation)?
                .into_iter()
                .map(|word| shadow + word),
        )
        .collect::<BTreeSet<_>>();
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
    // Non-SSA values retain their ordinary canonical owner/home. Every
    // synchronization lane above is registered in this numeric word bank.
    live.retain(|word| registered.contains(word));
    words.spill_live(b, locals, &live)?;
    Ok(live.len())
}
