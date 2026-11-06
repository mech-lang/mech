//! Reuse one freshly executed source window check, never a saved permission.
use super::*;

fn eligible(f: &Finder<'_, '_>, site: usize, iteration: Option<u32>) -> bool {
    f.sites.contains(&site)
        && !f.boundaries.contains(&site)
        && f.facts.plan().blocks()[site].iteration() == iteration
        && !f
            .proof
            .exits
            .iter()
            .any(|exit| exit.from == site || exit.to == site)
}

fn previous(f: &Finder<'_, '_>, site: usize, iteration: Option<u32>) -> Option<usize> {
    let &[previous] = f.facts.predecessors(site as u32).ok()? else {
        return None;
    };
    let previous = previous as usize;
    eligible(f, previous, iteration).then_some(previous)
}

fn jump(f: &Finder<'_, '_>, site: usize, next: usize) -> bool {
    matches!(f.facts.plan().blocks()[site].terminator(), Term::Scalar(Branch::Jump(to)) if to as usize == next)
}

fn empty(f: &Finder<'_, '_>, site: usize) -> bool {
    matches!(
        f.facts.plan().blocks()[site].action(),
        Action::Scalar(Scalar::None)
    ) && f.proof.action(site).is_none()
}

fn checked(f: &Finder<'_, '_>, pack: &Pack) -> Option<()> {
    // A positive first offset also proves the one-based minimum. Zero-based
    // packs retain their original nonzero guard, without a partial proof here.
    if pack.offset == 0 {
        return None;
    }
    let plan = f.facts.plan();
    let iteration = plan.blocks()[pack.first].iteration();
    if !eligible(f, pack.first, iteration) {
        return None;
    }
    let mut current = pack.first;
    let mut seen = BTreeSet::new();
    let (predicate, producer, limit) = loop {
        if !seen.insert(current) {
            return None;
        }
        let prior = previous(f, current, iteration)?;
        if !empty(f, prior) {
            return None;
        }
        if jump(f, prior, current) {
            current = prior;
            continue;
        }
        let Term::Scalar(Branch::LiteralBranch { left, right, .. }) =
            plan.blocks()[prior].terminator()
        else {
            return None;
        };
        let matched = [left, right].into_iter().find_map(|predicate| {
            let producer = f.definition(predicate)?;
            let Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) = plan.blocks()[producer].action()
            else {
                return None;
            };
            let [left, right] = operands.as_ref() else {
                return None;
            };
            let limit = match op {
                Op::U64Le if *left == pack.base => *right,
                Op::U64Ge if *right == pack.base => *left,
                _ => return None,
            };
            (*output == predicate).then_some((predicate, producer, limit))
        })?;
        break matched;
    };

    // Every edge from this fresh comparison to the first byte belongs to its
    // true arm. Both source literal orders work; joins and stale predicates do
    // not. No source action may intervene or redefine an operand.
    current = pack.first;
    seen.clear();
    loop {
        if !seen.insert(current) {
            return None;
        }
        let prior = previous(f, current, iteration)?;
        if prior == producer {
            if !jump(f, prior, current) {
                return None;
            }
            break;
        }
        if !empty(f, prior)
            || !(jump(f, prior, current)
                || f.facts
                    .boolean_branch_edges(prior as u32, predicate)
                    .is_some_and(|(_, yes)| yes as usize == current))
        {
            return None;
        }
        current = prior;
    }

    // The checked subtraction itself must have just run. Its successful edge
    // proves length>=end; the source comparison proves base<=length-end.
    // These source operations and their original rejection order stay emitted.
    let subtract = previous(f, producer, iteration)?;
    if f.definition(limit) != Some(subtract)
        || !jump(f, subtract, producer)
        || f.proof.action(producer).is_some()
        || f.proof.action(subtract).is_some()
    {
        return None;
    }
    let Action::Scalar(Scalar::Primitive {
        op: Op::U64Sub,
        operands,
        output,
    }) = plan.blocks()[subtract].action()
    else {
        return None;
    };
    let [length, end] = operands.as_ref() else {
        return None;
    };
    if *output != limit || f.constants.get(&end.get()) != Some(&pack.end) {
        return None;
    }
    let length_site = f.definition(*length)?;
    if !matches!(f.proof.action(length_site), Some(Operation::Length { array, output })
        if *array == pack.array && *output == length.get())
    {
        return None;
    }

    // The length and current view must be from this visit. Between that read
    // and the subtraction accept only scalar operations/edges: no array/frame
    // forwarding, replacement, append, helper or ownership boundary.
    current = subtract;
    seen.clear();
    loop {
        if !seen.insert(current) {
            return None;
        }
        let prior = previous(f, current, iteration)?;
        if prior == length_site {
            return jump(f, prior, current).then_some(());
        }
        let block = &plan.blocks()[prior];
        if f.proof.action(prior).is_some()
            || !matches!(
                block.action(),
                Action::Scalar(Scalar::None | Scalar::Primitive { .. })
            )
            || !(jump(f, prior, current)
                || matches!(block.terminator(), Term::Scalar(Branch::LiteralBranch { yes, no, .. })
                    if yes as usize == current || no as usize == current))
        {
            return None;
        }
        current = prior;
    }
}

pub(super) fn derive(f: &Finder<'_, '_>, pack: &Pack) -> bool {
    checked(f, pack).is_some()
}
