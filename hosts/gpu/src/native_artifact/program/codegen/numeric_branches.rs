//! Current-visit complementary cases in an already selected numeric entry.
//! Descriptive plan facts supply polarity; this local chain proves availability.
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeProgramFacts,
    NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn derive(facts: &NativeProgramFacts<'_>, proof: &Proof) -> BTreeMap<usize, u32> {
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
        let predicate = [left, right].into_iter().find_map(|value| {
            let [definition] = facts.definition_sites(value.get()).ok()? else {
                return None;
            };
            let producer = definition.site()?;
            let block = &plan.blocks()[producer as usize];
            let Action::Scalar(Scalar::Primitive { op, output, .. }) = block.action() else {
                return None;
            };
            (*output == value
                && matches!(
                    op,
                    Op::U64Eq | Op::U64Ne | Op::U64Lt | Op::U64Le | Op::U64Gt | Op::U64Ge
                ))
            .then_some((value, producer))
        });
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
    successors
}
