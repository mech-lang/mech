//! Phase proof for the SAME native CFG. A checkpoint is not another program,
//! source rewrite, evaluator, authorization token or saved reactive instance.
use super::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeProgramArtifactPlan,
};

const BAD: &str = "invalid native activation/turn phase boundary";
fn successors(term: Terminator) -> [Option<u32>; 2] {
    match term {
        Terminator::Scalar(Term::Jump(next)) => [Some(next), None],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
            [Some(yes), Some(no)]
        }
        Terminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        Terminator::TailHeader { body, .. } => [Some(body), None],
        _ => [None, None],
    }
}
pub(super) fn validate(plan: &NativeProgramArtifactPlan) -> Result<(), &'static str> {
    // Only true managed plans enter this proof. Scalar intermediate views
    // use their unchanged generic current-visit validator before conversion.
    // Clearing BOTH fields must not let a forged managed plan evade phases.
    let boundary = plan.activation_boundary().ok_or(BAD)?;
    let blocks = plan.blocks();
    let gate = blocks.get(boundary as usize).ok_or(BAD)?;
    let first = blocks.get(plan.entry() as usize).ok_or(BAD)?;
    if first.iteration().is_some()
        || !matches!(first.action(), Action::Scalar(Scalar::None))
        || !matches!(first.terminator(), Terminator::Scalar(Term::Jump(_)))
        || boundary == plan.entry()
        || gate.iteration().is_some()
        || !matches!(gate.action(), Action::Scalar(Scalar::None))
        || !matches!(gate.terminator(), Terminator::Scalar(Term::Jump(_)))
        || plan
            .activation_nodes()
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(BAD);
    }
    let mut before = vec![false; blocks.len()];
    let mut after = vec![false; blocks.len()];
    let mut pending = vec![plan.entry()];
    let mut reaches_boundary = false;
    while let Some(id) = pending.pop() {
        if id == boundary {
            reaches_boundary = true;
            continue;
        }
        let block = blocks.get(id as usize).ok_or(BAD)?;
        if std::mem::replace(&mut before[id as usize], true) {
            continue;
        }
        if matches!(block.terminator(), Terminator::Scalar(Term::Complete)) {
            return Err(BAD);
        }
        pending.extend(successors(block.terminator()).into_iter().flatten());
    }
    if !reaches_boundary {
        return Err(BAD);
    }
    pending.push(boundary);
    while let Some(id) = pending.pop() {
        let block = blocks.get(id as usize).ok_or(BAD)?;
        if before[id as usize] {
            return Err(BAD);
        }
        if std::mem::replace(&mut after[id as usize], true) {
            continue;
        }
        if id != boundary && successors(block.terminator()).contains(&Some(boundary)) {
            return Err(BAD);
        }
        pending.extend(successors(block.terminator()).into_iter().flatten());
    }
    let mut observed_activation = vec![false; plan.activation_nodes().len()];
    for (id, block) in blocks.iter().enumerate() {
        if matches!(block.terminator(), Terminator::Scalar(Term::Complete))
            && !matches!(block.action(), Action::Scalar(Scalar::None))
        {
            return Err(BAD);
        }
        if id == plan.entry() as usize
            || id == boundary as usize
            || matches!(block.terminator(), Terminator::Scalar(Term::Complete))
        {
            continue;
        }
        // Dormant typed Reject arms have no action or visible result.
        if !before[id]
            && !after[id]
            && matches!(block.action(), Action::Scalar(Scalar::None))
            && matches!(block.terminator(), Terminator::Scalar(Term::Reject))
        {
            continue;
        }
        let owner = plan
            .activation_nodes()
            .binary_search(&block.origin().node());
        if before[id] != owner.is_ok() || (!before[id] && !after[id]) {
            return Err(BAD);
        }
        if let Ok(owner) = owner {
            observed_activation[owner] = true;
        }
    }
    if observed_activation.iter().any(|seen| !seen) {
        return Err(BAD);
    }
    Ok(())
}
