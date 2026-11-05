//! Current-visit complementary cases in an already selected numeric entry.
//! Descriptive plan facts supply polarity; this local chain proves availability.
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue,
    NativeProgramFacts, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Facts {
    pub successors: BTreeMap<usize, u32>,
    pub subtractions: BTreeSet<usize>,
}

fn comparison(
    facts: &NativeProgramFacts<'_>,
    predicate: NativeMatchValue,
) -> Option<(u32, Op, [NativeMatchValue; 2])> {
    let [definition] = facts.definition_sites(predicate.get()).ok()? else {
        return None;
    };
    let producer = definition.site()?;
    let Action::Scalar(Scalar::Primitive {
        op,
        operands,
        output,
    }) = facts.plan().blocks()[producer as usize].action()
    else {
        return None;
    };
    let [left, right] = operands.as_ref() else {
        return None;
    };
    (*output == predicate
        && matches!(
            op,
            Op::U64Eq | Op::U64Ne | Op::U64Lt | Op::U64Le | Op::U64Gt | Op::U64Ge
        ))
    .then_some((producer, *op, [*left, *right]))
}

fn implies_ge(
    op: Op,
    truth: bool,
    compared: [NativeMatchValue; 2],
    subtract: [NativeMatchValue; 2],
) -> bool {
    let forward = compared == subtract;
    let reverse = [compared[1], compared[0]] == subtract;
    match (op, truth) {
        (Op::U64Ge | Op::U64Gt, true) | (Op::U64Lt | Op::U64Le, false) => forward,
        (Op::U64Le | Op::U64Lt, true) | (Op::U64Gt | Op::U64Ge, false) => reverse,
        (Op::U64Eq, true) | (Op::U64Ne, false) => forward || reverse,
        _ => false,
    }
}

pub(super) fn derive(facts: &NativeProgramFacts<'_>, proof: &Proof) -> Facts {
    let plan = facts.plan();
    let sites = proof.sites.iter().copied().collect::<BTreeSet<_>>();
    let mut frontiers = BTreeSet::new();
    for iteration in plan.iterations() {
        if let NativeControlLoop::Tail(tail) = iteration {
            frontiers.extend([
                tail.header(),
                tail.initialize(),
                tail.latch(),
                tail.finish(),
                tail.exit(),
            ]);
        }
    }
    for exit in &proof.exits {
        frontiers.extend([exit.from as u32, exit.to as u32]);
    }
    if let Some(phase) = &proof.phase {
        for debit in &phase.loops {
            frontiers.extend(
                [
                    debit.header,
                    debit.initialize,
                    debit.latch,
                    debit.finish,
                    debit.exit,
                ]
                .map(|site| site as u32),
            );
        }
    }
    let eligible = |site: u32| sites.contains(&(site as usize)) && !frontiers.contains(&site);
    let mut successors = BTreeMap::new();
    for &site in &proof.sites {
        let current = &plan.blocks()[site];
        let Term::Scalar(Branch::LiteralBranch {
            left, right, yes, ..
        }) = current.terminator()
        else {
            continue;
        };
        // Only a fresh comparison-produced 0/1 word grants the Bool domain.
        // Arbitrary raw Bool/tag homes keep their fail-closed default checks.
        let predicate = [left, right]
            .into_iter()
            .find_map(|value| comparison(facts, value).map(|(producer, _, _)| (value, producer)));
        let Some((predicate, producer)) = predicate else {
            continue;
        };
        let Ok([previous]) = facts.predecessors(site as u32) else {
            continue;
        };
        let prior = &plan.blocks()[*previous as usize];
        if ![site as u32, *previous, producer].into_iter().all(eligible)
            || !matches!(current.action(), Action::Scalar(Scalar::None))
            || !matches!(prior.action(), Action::Scalar(Scalar::None))
            || current.iteration() != prior.iteration()
            || current.iteration() != plan.blocks()[producer as usize].iteration()
            || facts.predecessors(*previous).ok() != Some(&[producer][..])
            || !matches!(plan.blocks()[producer as usize].terminator(), Term::Scalar(Branch::Jump(next)) if next == *previous)
        {
            continue;
        }
        let Term::Scalar(Branch::LiteralBranch { no, .. }) = prior.terminator() else {
            continue;
        };
        if no != site as u32 {
            continue;
        }
        let Some((prior_false, prior_true)) = facts.boolean_branch_edges(*previous, predicate)
        else {
            continue;
        };
        let Some((current_false, current_true)) =
            facts.boolean_branch_edges(site as u32, predicate)
        else {
            continue;
        };
        let successor = if prior_true == site as u32 {
            current_true
        } else if prior_false == site as u32 {
            current_false
        } else {
            continue;
        };
        if successor == yes {
            successors.insert(site, successor);
        }
    }
    // A selected arm has exactly one empty Jump before the checked operation.
    // The guard's comparison is fresh either immediately before its first case,
    // or through the exact complementary-case chain already proved above.
    let mut subtractions = BTreeSet::new();
    for &site in &proof.sites {
        let current = &plan.blocks()[site];
        let Action::Scalar(Scalar::Primitive {
            op: Op::U64Sub,
            operands,
            ..
        }) = current.action()
        else {
            continue;
        };
        let [left, right] = operands.as_ref() else {
            continue;
        };
        let Ok([arm]) = facts.predecessors(site as u32) else {
            continue;
        };
        let arm_block = &plan.blocks()[*arm as usize];
        let Ok([guard]) = facts.predecessors(*arm) else {
            continue;
        };
        let guard_block = &plan.blocks()[*guard as usize];
        let Term::Scalar(Branch::LiteralBranch {
            left: a,
            right: b,
            yes,
            ..
        }) = guard_block.terminator()
        else {
            continue;
        };
        if yes != *arm
            || !matches!(arm_block.action(), Action::Scalar(Scalar::None))
            || !matches!(arm_block.terminator(), Term::Scalar(Branch::Jump(next)) if next == site as u32)
            || !matches!(guard_block.action(), Action::Scalar(Scalar::None))
        {
            continue;
        }
        for predicate in [a, b] {
            let Some((producer, op, compared)) = comparison(facts, predicate) else {
                continue;
            };
            if ![site as u32, *arm, *guard, producer].into_iter().all(|at| {
                eligible(at)
                    && plan.blocks()[at as usize].iteration() == current.iteration()
                    && proof.action(at as usize).is_none()
            }) {
                continue;
            }
            let direct = facts.predecessors(*guard).ok() == Some(&[producer][..])
                && matches!(plan.blocks()[producer as usize].terminator(), Term::Scalar(Branch::Jump(next)) if next == *guard);
            if !direct && successors.get(&(*guard as usize)) != Some(arm) {
                continue;
            }
            let Some((no, yes)) = facts.boolean_branch_edges(*guard, predicate) else {
                continue;
            };
            let truth = if yes == *arm {
                true
            } else if no == *arm {
                false
            } else {
                continue;
            };
            if implies_ge(op, truth, compared, [*left, *right]) {
                subtractions.insert(site);
            }
        }
    }
    Facts {
        successors,
        subtractions,
    }
}

#[cfg(test)]
#[path = "numeric_branches_tests.rs"]
mod tests;
