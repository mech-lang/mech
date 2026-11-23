//! A private call contract, never authority for an ordinary funded entry.
//!
//! The shared proof still validates every operation, buffer version and cold
//! export. Its child funding exits remain part of the ordinary contract. This
//! caller instead owns exclusive fixed buffers throughout a synchronous call,
//! and exports current source state before any unsupported operation executes.
use super::*;
use crate::native_artifact::program::prepare::{
    numeric_plan::ExitKind, readonly_numeric::Proof, tail::Region,
};

pub(in crate::native_artifact) const HANDOFF: u64 = 0x19304;

pub(super) struct Handoff {
    pub site: usize,
    pub outputs: Vec<(usize, Output)>,
    pub owners: Vec<usize>,
    /// Packet position, source word. Scalar frame fields use shadow coordinates.
    pub exports: Vec<(usize, Export)>,
}

pub(super) enum Export {
    Scalar(usize),
    Shadow(usize),
}

pub(in crate::native_artifact) struct PrivatePhase {
    proof: *const Proof,
    pub(super) handoffs: Vec<Handoff>,
}

impl PrivatePhase {
    pub(super) fn derive(
        prepared: &PreparedNativeProgram,
        region: &Region,
        layout: &Layout,
    ) -> Result<Option<Self>, NativeCompileError> {
        let Some(proof) = region.numeric.as_ref() else {
            return Ok(None);
        };
        // First scope: fixed storage, exact Tail descendants and complete
        // source exports. Growing buffers, prepaid fragments and summarized
        // children keep their original implementation.
        if !proof
            .phase
            .as_ref()
            .is_some_and(|phase| phase.checked_children && phase.cold.is_empty())
            || !proof.prepaid_sites.is_empty()
            || proof.arrays.iter().any(|array| array.appends)
            || proof.actions.iter().any(|(_, action)| action.grows())
            || !proof.writes()
        {
            return Ok(None);
        }
        let plan = prepared.plan();
        let facts = plan.facts();
        let carrier = region.tail.carrier().get();
        let mut handoffs = Vec::new();
        for exit in &proof.exits {
            if exit.kind != ExitKind::Unsupported {
                continue;
            }
            if proof.action(exit.to) != Some(&Operation::Fallback) {
                return Ok(None);
            }
            let mut outputs = BTreeMap::new();
            let mut exports = Vec::new();
            let mut owners = BTreeSet::new();
            for &value in &exit.values {
                let defined = facts
                    .definition_sites(value)
                    .map_err(backend)?
                    .iter()
                    .any(|d| {
                        d.site()
                            .is_some_and(|site| proof.sites.binary_search(&(site as usize)).is_ok())
                    });
                // The callee never changes source homes defined wholly before
                // this region. Those current bindings remain in its caller.
                if !defined {
                    continue;
                }
                if plan.values()[value as usize].scalar_kind().is_some() {
                    scalar(
                        &mut outputs,
                        &mut exports,
                        layout.slot(value),
                        Export::Scalar(value as usize),
                    );
                } else if let Some(frame) = proof.frames.iter().find(|f| f.value == value) {
                    let target_fields = layout.ty(value).leaves();
                    if target_fields.len() != frame.arrays.len() {
                        return Ok(None);
                    }
                    let mut scalars = frame.scalar_paths.iter();
                    for ((offset, ty), array) in target_fields.iter().zip(&frame.arrays) {
                        if let Some(array) = array {
                            if ty.words != 6 {
                                return Ok(None);
                            }
                            let from = layout.array_field(&proof.arrays[*array], carrier)?.0;
                            let to = layout.slot(value) + offset;
                            for lane in 0..6 {
                                outputs.insert(to + lane, Output::Word(from + lane));
                            }
                            owners.insert(to + 5);
                        } else {
                            let Some((path, word)) = scalars.next() else {
                                return Ok(None);
                            };
                            let (scalar_offset, scalar_ty) = layout.ty(value).at_path(path)?;
                            if scalar_offset != *offset || scalar_ty.words != 1 {
                                return Ok(None);
                            }
                            scalar(
                                &mut outputs,
                                &mut exports,
                                layout.slot(value) + offset,
                                Export::Shadow(*word),
                            );
                        }
                    }
                    if scalars.next().is_some() {
                        return Ok(None);
                    }
                } else if let Some(&(_, array)) = exit.aliases.iter().find(|&&(v, _)| v == value) {
                    if layout.ty(value).words != 6 {
                        return Ok(None);
                    }
                    let from = layout.array_field(&proof.arrays[array], carrier)?.0;
                    let to = layout.slot(value);
                    for lane in 0..6 {
                        outputs.insert(to + lane, Output::Word(from + lane));
                    }
                    owners.insert(to + 5);
                } else if let Some((base, count)) = prepared
                    .metadata()
                    .scalar_aggregates
                    .slot_at(value as usize)
                {
                    if !layout.ty(value).owners.is_empty() || layout.ty(value).words != count {
                        return Ok(None);
                    }
                    for lane in 0..count {
                        scalar(
                            &mut outputs,
                            &mut exports,
                            layout.slot(value) + lane,
                            Export::Scalar(base + lane),
                        );
                    }
                } else {
                    return Ok(None);
                }
            }
            handoffs.push(Handoff {
                site: exit.to,
                outputs: outputs.into_iter().collect(),
                owners: owners.into_iter().collect(),
                exports,
            });
        }
        Ok(Some(Self { proof, handoffs }))
    }

    pub(in crate::native_artifact) fn permits(&self, proof: &Proof) -> bool {
        std::ptr::eq(self.proof, proof)
    }

    pub(in crate::native_artifact) fn exports(
        &self,
        shadow: usize,
    ) -> BTreeMap<usize, Vec<(usize, usize)>> {
        self.handoffs
            .iter()
            .map(|exit| {
                (
                    exit.site,
                    exit.exports
                        .iter()
                        .map(|&(at, ref source)| {
                            (
                                at,
                                match *source {
                                    Export::Scalar(word) => word,
                                    Export::Shadow(word) => shadow + word,
                                },
                            )
                        })
                        .collect(),
                )
            })
            .collect()
    }

    pub(super) fn packet_words(&self) -> usize {
        self.handoffs
            .iter()
            .map(|e| 1 + e.exports.len())
            .max()
            .unwrap_or(0)
    }
}

fn scalar(
    outputs: &mut BTreeMap<usize, Output>,
    exports: &mut Vec<(usize, Export)>,
    target: usize,
    source: Export,
) {
    let at = 1 + exports.len();
    exports.push((at, source));
    outputs.insert(target, Output::Stack(at));
}

#[cfg(all(test, feature = "canonical-native-program-runtime"))]
#[path = "private_phase_tests.rs"]
mod tests;
