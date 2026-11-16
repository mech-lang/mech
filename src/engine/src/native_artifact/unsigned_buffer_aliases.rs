//! Possible canonical aliases through original structural transport. This is
//! candidate enumeration only: last use and current frozen identity are checked
//! independently before an existing owner can be retired or consumed.
use super::{Action, Facts, Plan, Primitive, Scalar, Value};
use crate::native_artifact::{
    NativeControlLoop as Loop, NativeStructuralObservation as Observation,
    NativeStructuralStep as Step,
};
use mech_core::SchemaBody;
use std::collections::BTreeSet;

fn valid_path(plan: &Plan, value: Value, path: &[usize]) -> bool {
    let Some(ty) = plan.values().get(value.get() as usize) else {
        return false;
    };
    let mut body = ty.definition().body();
    for &ordinal in path {
        body = match body {
            SchemaBody::Tuple(fields) => match fields.get(ordinal) {
                Some(field) => field,
                None => return false,
            },
            SchemaBody::Record(fields) => match fields.get(ordinal) {
                Some(field) => &field.schema,
                None => return false,
            },
            _ => return false,
        };
    }
    true
}

pub(super) fn possible(facts: &Facts<'_>, source: Value) -> Result<Vec<Value>, &'static str> {
    let plan = facts.plan();
    let mut pending: Vec<(Value, Box<[usize]>)> = vec![(source, Box::new([]))];
    let mut seen = BTreeSet::new();
    let mut values = BTreeSet::new();
    while let Some((value, path)) = pending.pop() {
        // Schema-valid paths have finite depth. Together with the visited key,
        // this also closes identity cycles through recursive seed/latch fields.
        if !valid_path(plan, value, &path) || !seen.insert((value.get(), path.clone())) {
            continue;
        }
        if path.is_empty() && plan.values()[value.get() as usize].scalar_kind().is_none() {
            values.insert(value.get());
        }
        for definition in facts.definition_sites(value.get())? {
            let Some(site) = definition.site() else {
                continue;
            };
            let action = plan.blocks()[site as usize].action();
            match action {
                Action::Scalar(Scalar::Copy { source, output }) if *output == value => {
                    pending.push((*source, path.clone()));
                }
                Action::Initialize { iteration } | Action::Latch { iteration } => {
                    let initial = matches!(action, Action::Initialize { .. });
                    let (from, output) = match &plan.iterations()[*iteration as usize] {
                        Loop::Tail(t) => (if initial { t.seed() } else { t.next() }, t.carrier()),
                        Loop::Iteration(t) => {
                            (if initial { t.seed() } else { t.next() }, t.carrier())
                        }
                        Loop::Collection(_) => continue,
                    };
                    if output == value {
                        pending.push((from, path.clone()));
                    }
                }
                Action::LibraryCall {
                    template,
                    operands,
                    output,
                } if *output == value
                    && matches!(template.primitive(), Primitive::CompositePack(n)
                            if n == operands.len()) =>
                {
                    if let Some((&ordinal, rest)) = path.split_first()
                        && let Some(&operand) = operands.get(ordinal)
                    {
                        pending.push((operand, rest.into()));
                    }
                }
                Action::Structural {
                    source,
                    path: steps,
                    observation: Observation::CanonicalValue,
                    output,
                } if *output == value => {
                    let Some(ordinals) = steps
                        .iter()
                        .map(|step| match step {
                            Step::TupleItem { ordinal } => Some(*ordinal as usize),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()
                    else {
                        continue;
                    };
                    let fields = ordinals.into_iter().chain(path.iter().copied()).collect();
                    pending.push((*source, fields));
                }
                Action::LibraryCall { output, .. } if *output == value => {
                    if let Some((from, ordinal, _)) =
                        super::super::record_projection::constant_field(plan, action)
                    {
                        let fields = core::iter::once(ordinal)
                            .chain(path.iter().copied())
                            .collect();
                        pending.push((from, fields));
                    }
                }
                // In particular, a replacement result is not an identity edge
                // back to its old source. Unknown geometry/operations stop here.
                _ => {}
            }
        }
    }
    Ok(values.into_iter().map(Value).collect())
}
