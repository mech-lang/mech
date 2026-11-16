//! Query-local definition kills for values read by one selected exit graph.
use mech_engine::native_artifact::NativeProgramFacts;
use std::collections::BTreeSet;

pub(super) fn selected(
    facts: &NativeProgramFacts<'_>,
    ordinals: &[usize],
    reads: &[BTreeSet<u32>],
    wanted: Option<&BTreeSet<u32>>,
) -> Result<Vec<Vec<u32>>, &'static str> {
    let value_count = facts.plan().values().len();
    let mut values = reads
        .iter()
        .flatten()
        .copied()
        .filter(|&value| (value as usize) < value_count)
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();

    // The former value loop demanded definitions even with no reads. Its first
    // lookup constructs/validates the WHOLE same-plan inventory, including
    // unused producers. Keep that demand, after graph/read validation, iff the
    // old loop would make it. Empty/out-of-range wanted sets demand nothing.
    let first = match wanted {
        Some(wanted) => wanted.first().copied(),
        None => Some(0),
    };
    if let Some(value) = first.filter(|&value| (value as usize) < value_count) {
        facts.definition_sites(value)?;
    }

    let mut definitions = vec![Vec::new(); reads.len()];
    for value in values {
        if wanted.is_some_and(|wanted| !wanted.contains(&value)) {
            continue;
        }
        for definition in facts.definition_sites(value)? {
            if let Some(site) = definition.site()
                && let Some(&index) = ordinals.get(site as usize)
                && index != usize::MAX
            {
                let killed = &mut definitions[index];
                // Values are ordered; duplicate definition records preserve the
                // former set semantics without allocating a tree per node.
                if killed.last() != Some(&value) {
                    killed.push(value);
                }
            }
        }
    }
    Ok(definitions)
}
