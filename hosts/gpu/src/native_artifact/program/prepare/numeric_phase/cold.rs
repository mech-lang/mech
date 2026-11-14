//! Opaque synchronous children in the descriptive traversal graph. Native code
//! never follows these summary edges: it exports at the exact initializer and
//! executes the original child and remaining source traversal ordinarily.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Child")]
pub(in crate::native_artifact) struct Child {
    pub iteration: u32,
    pub initialize: usize,
    pub exit: usize,
}

pub(super) fn children(plan: &Plan, root: u32) -> Result<Vec<Child>, &'static str> {
    let mut children = Vec::new();
    for (iteration, control) in plan.iterations().iter().enumerate() {
        if control.as_ref().parent() != Some(root) {
            continue;
        }
        let NativeControlLoop::Tail(tail) = control else {
            return Err("cold numeric child requires a synchronous Tail boundary");
        };
        if plan.values()[tail.returned().get() as usize]
            .scalar_kind()
            .is_none()
        {
            return Err("cold numeric child requires a scalar result");
        }
        children.push(Child {
            iteration: iteration as u32,
            initialize: tail.initialize() as usize,
            exit: tail.exit() as usize,
        });
    }
    if children.is_empty() {
        return Err("numeric loop has no conditional child boundary");
    }
    Ok(children)
}

pub(super) fn successors(cold: &[Child], plan: &Plan, site: usize) -> Vec<usize> {
    if let Some(child) = cold.iter().find(|child| child.initialize == site) {
        vec![child.exit]
    } else {
        NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
            .into_iter()
            .flatten()
            .map(|site| site as usize)
            .collect()
    }
}
