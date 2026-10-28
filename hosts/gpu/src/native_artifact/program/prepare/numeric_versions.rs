//! Forward must-analysis on one acyclic traversal. Root provenance alone does
//! not permit mutation: every content use and recursive/result publication must
//! refer to the latest functional version on every path reaching that site.
use super::*;
type Key = (u32, Option<usize>);
type State = BTreeMap<Key, bool>;

pub(super) fn validate(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    sites: &BTreeSet<usize>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    returned: Option<usize>,
) -> Result<(), &'static str> {
    let successors = |site: usize| -> Vec<usize> {
        if site == tail.latch() as usize {
            return Vec::new();
        }
        let next = match plan.blocks()[site].terminator() {
            Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => vec![body],
            Term::Scalar(
                Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. },
            ) => vec![yes, no],
            _ => Vec::new(),
        };
        next.into_iter()
            .map(|n| n as usize)
            .filter(|n| sites.contains(n))
            .collect()
    };
    let mut incoming = BTreeMap::<usize, usize>::new();
    for &site in sites {
        incoming.entry(site).or_default();
    }
    for &site in sites {
        for next in successors(site) {
            *incoming.get_mut(&next).unwrap() += 1;
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(&site, &n)| (n == 0).then_some(site))
        .collect::<Vec<_>>();
    let mut states = BTreeMap::<usize, State>::new();
    states.insert(
        tail.header() as usize,
        arrays
            .iter()
            .map(|a| ((tail.carrier().get(), Some(a.field)), true))
            .collect(),
    );
    let mut visited = 0;
    while let Some(site) = ready.pop() {
        visited += 1;
        let mut state = states
            .remove(&site)
            .ok_or("unreachable numeric body root")?;
        let fresh = |state: &State, key: Key| state.get(&key).copied().unwrap_or(false);
        match actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op) {
            Some(Operation::Frame) => match frames.action(site).unwrap().operation() {
                Frame::Pack { output } => {
                    let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                        return Err("version pack lost operands");
                    };
                    for (ordinal, operand) in operands.iter().enumerate() {
                        if provenance.array(operand.get(), None).is_some() {
                            let current = fresh(&state, (operand.get(), None));
                            state.insert((output.get(), Some(ordinal)), current);
                        }
                    }
                }
                Frame::Copy { source, output } => {
                    let copied = frames
                        .fields(*source)
                        .ok_or("version copy lost fields")?
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| matches!(f.lane(), Lane::Managed { .. }))
                        .map(|(ordinal, _)| {
                            (
                                (output.get(), Some(ordinal)),
                                fresh(&state, (source.get(), Some(ordinal))),
                            )
                        })
                        .collect::<Vec<_>>();
                    state.extend(copied);
                }
                Frame::Project {
                    source,
                    ordinal,
                    output,
                } => {
                    if provenance.array(output.get(), None).is_some() {
                        let current = fresh(&state, (source.get(), Some(*ordinal)));
                        state.insert((output.get(), None), current);
                    }
                }
            },
            Some(Operation::ArrayCopy) => {
                let Action::Scalar(Scalar::Copy { source, output }) = plan.blocks()[site].action()
                else {
                    return Err("version copy lost action");
                };
                let current = fresh(&state, (source.get(), None));
                state.insert((output.get(), None), current);
            }
            Some(Operation::Read { .. } | Operation::Write { .. }) => {
                let Action::LibraryCall {
                    operands, output, ..
                } = plan.blocks()[site].action()
                else {
                    return Err("version access lost operands");
                };
                if !fresh(&state, (operands[0].get(), None)) {
                    return Err("content use reads an older array version");
                }
                if let Some(Operation::Write { array, .. }) =
                    actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op)
                {
                    let root = arrays[*array].field;
                    for (&(value, field), current) in &mut state {
                        if provenance.array(value, field) == Some(root) {
                            *current = false;
                        }
                    }
                    state.insert((output.get(), None), true);
                }
            }
            _ => {}
        }
        if site == tail.latch() as usize {
            for array in arrays {
                if !fresh(&state, (tail.next().get(), Some(array.field))) {
                    return Err("recursive edge discards or forwards an older array version");
                }
            }
        }
        if site == tail.finish() as usize
            && returned.is_some()
            && !fresh(&state, (tail.returned().get(), None))
        {
            return Err("leaf returns an older array version");
        }
        for next in successors(site) {
            if let Some(merged) = states.get_mut(&next) {
                // A definition is current only when it exists and is current
                // on every predecessor; branch-specific stale aliases cannot
                // acquire authority merely by meeting the same root.
                merged.retain(|key, current| {
                    *current &= state.get(key).copied().unwrap_or(false);
                    true
                });
            } else {
                states.insert(next, state.clone());
            }
            let count = incoming.get_mut(&next).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.push(next);
            }
        }
    }
    if visited != sites.len() {
        return Err("numeric traversal has an undeclared cycle");
    }
    Ok(())
}
