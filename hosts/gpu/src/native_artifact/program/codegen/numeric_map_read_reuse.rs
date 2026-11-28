//! A successful read of one immutable loan view can supply the same exact
//! selector later in this iteration. Source fees and first failures remain.
use super::{Action, Plan, Region, Scalar, Value};
use crate::native_artifact::program::prepare::numeric_maps::MatrixRead;
use mech_engine::native_artifact::NativeScalarKind;
use std::collections::{BTreeMap, BTreeSet};

/// Compiler-only diagnostics and private ablation; never image/ABI authority.
pub(in crate::native_artifact) struct Reuse {
    pub enabled: bool,
    pub sites: Vec<(u32, u32, u32)>,
}
impl Reuse {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            sites: Vec::new(),
        }
    }
}
#[derive(Default)]
pub(super) struct Cache {
    values: BTreeMap<(usize, Vec<u32>), (u32, Value)>,
}
impl Cache {
    pub fn remember(&mut self, read: &MatrixRead, value: Value) {
        self.values
            .insert((read.matrix, read.indices.clone()), (read.site, value));
    }
    pub fn lookup(&self, read: &MatrixRead, region: &Region, plan: &Plan) -> Option<(u32, Value)> {
        if read.indices.iter().any(|id| {
            plan.values()
                .get(*id as usize)
                .is_none_or(|v| v.scalar_kind() != Some(NativeScalarKind::U64))
                || !(region.captures.contains(id)
                    || *id == region.element
                    || region
                        .rectangle
                        .as_ref()
                        .is_some_and(|r| r.inner_element == *id))
        }) {
            return None;
        }
        let &(first, value) = self.values.get(&(read.matrix, read.indices.clone()))?;
        if first == read.site {
            return None;
        }
        let facts = plan.facts();
        let mut at = read.site;
        let mut seen = BTreeSet::new();
        while seen.insert(at) && seen.len() <= region.body.len() {
            let [previous] = facts.predecessors(at).ok()? else {
                return None;
            };
            let previous = *previous;
            if !region.body.contains(&previous) {
                return None;
            }
            // A capture/item should not be redefined inside this body. Verify
            // the actual intervening source too; publication/unknown actions
            // and control cycles never establish a reusable successful word.
            let output = match plan.blocks().get(previous as usize)?.action() {
                Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
                | Action::LibraryCall { output, .. } => Some(output.get()),
                Action::Scalar(Scalar::None) => None,
                _ => return None,
            };
            if output.is_some_and(|id| read.indices.contains(&id)) {
                return None;
            }
            if previous == first {
                return Some((first, value));
            }
            at = previous;
        }
        None
    }
}
