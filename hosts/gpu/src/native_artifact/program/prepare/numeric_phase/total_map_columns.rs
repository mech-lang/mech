//! Pure inner-row-invariant prefixes of already proved identity tails.
use crate::native_artifact::program::prepare::numeric_maps::Region;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeProgramArtifactPlan as Plan,
    NativeScalarKind as Kind,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
struct State {
    invariant: BTreeSet<u32>,
    pure: bool,
}
fn merge(states: &mut BTreeMap<u32, State>, target: u32, incoming: State) {
    if let Some(current) = states.get_mut(&target) {
        current.pure &= incoming.pure;
        current.invariant.retain(|v| incoming.invariant.contains(v));
    } else {
        states.insert(target, incoming);
    }
}
fn set(state: &mut State, output: u32, invariant: bool) {
    if invariant {
        state.invariant.insert(output);
    } else {
        state.invariant.remove(&output);
    }
}
pub(super) fn derive(plan: &Plan, region: &Region, identities: &BTreeSet<u32>) -> BTreeSet<u32> {
    if region.rectangle.is_none() || region.body.is_empty() {
        return BTreeSet::new();
    }
    // Only immutable captures survive between cell evaluations. Do not infer
    // invariance for any word assigned by a source action in this body.
    let writes = region
        .body
        .iter()
        .filter_map(|&site| match plan.blocks()[site as usize].action() {
            Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
            | Action::LibraryCall { output, .. } => Some(output.get()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut invariant = region
        .captures
        .iter()
        .copied()
        .filter(|v| !writes.contains(v))
        .collect::<BTreeSet<_>>();
    if !writes.contains(&region.element) {
        invariant.insert(region.element);
    }
    // This coordinate changes between cells even if a future producer happens
    // to include it in the scalar capture inventory. Rows of length one are
    // deliberately handled by the same conservative rule.
    invariant.remove(&region.rectangle.as_ref().unwrap().inner_element);

    let body = region.body.iter().copied().collect::<BTreeSet<_>>();
    let mut states = BTreeMap::from([(
        region.body[0],
        State {
            invariant,
            pure: true,
        },
    )]);
    let mut selected = BTreeSet::new();
    for &site in &region.body {
        let Some(mut state) = states.remove(&site) else {
            continue;
        };
        if state.pure && identities.contains(&site) {
            selected.insert(site);
        }
        match plan.blocks()[site as usize].action() {
            Action::Scalar(Scalar::Copy { source, output }) => {
                let invariant = state.invariant.contains(&source.get());
                set(&mut state, output.get(), invariant);
            }
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) => {
                let invariant = operands.iter().all(|v| state.invariant.contains(&v.get()));
                let (inputs, result) = op.signature();
                state.pure &= result != Kind::F64 && !inputs.contains(&Kind::F64);
                set(&mut state, output.get(), invariant);
            }
            Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
            Action::LibraryCall { output, .. } => {
                state.pure = false;
                state.invariant.remove(&output.get());
            }
            _ => state.pure = false,
        }
        let targets = match plan.blocks()[site as usize].terminator() {
            Terminator::Scalar(Term::Jump(to)) => vec![to],
            Terminator::Scalar(Term::Branch { condition, yes, no }) => {
                state.pure &= state.invariant.contains(&condition.get());
                vec![yes, no]
            }
            Terminator::Scalar(Term::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                state.pure &= state.invariant.contains(&left.get())
                    && state.invariant.contains(&right.get())
                    && plan.values()[left.get() as usize].scalar_kind() != Some(Kind::F64)
                    && plan.values()[right.get() as usize].scalar_kind() != Some(Kind::F64);
                vec![yes, no]
            }
            _ => vec![],
        };
        for target in targets {
            if body.contains(&target) {
                merge(&mut states, target, state.clone());
            }
        }
    }
    selected
}
