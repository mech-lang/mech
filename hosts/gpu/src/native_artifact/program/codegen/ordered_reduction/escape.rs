//! Skipped prefix temporaries must not be observable outside their child.
use super::*;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeProgramReadRole as Read,
};

pub(super) fn private_temporaries(code: CodePlan<'_>, tail: &NativeTailLoop) -> bool {
    let CodePlan::Program(program) = code else {
        return false;
    };
    let plan = program.plan();
    let Some(owner) = plan.blocks()[tail.header() as usize].iteration() else {
        return false;
    };
    let facts = plan.facts();
    let mut defined = std::collections::BTreeMap::new();
    for block in plan
        .blocks()
        .iter()
        .filter(|b| b.iteration() == Some(owner))
    {
        if let Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
        | Action::LibraryCall { output, .. }
        | Action::Structural { output, .. } = block.action()
        {
            defined.insert(output.get(), *output);
        }
    }
    defined.insert(tail.carrier().get(), tail.carrier());
    for value in defined.into_values() {
        if value == tail.returned() {
            // The shared scalar suffix and terminal traversal construct the
            // complete returned value through its original source operations.
            continue;
        }
        let Ok(reads) = facts.read_roles(value) else {
            return false;
        };
        if reads.iter().any(|read| match *read {
            Read::Action(site) | Read::Terminator(site) => {
                plan.blocks()[site as usize].iteration() != Some(owner)
            }
            Read::LoopNext(id) => id != owner,
            _ => true,
        }) {
            return false;
        }
    }
    true
}
