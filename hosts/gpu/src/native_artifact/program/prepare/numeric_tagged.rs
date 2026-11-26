//! One exact guarded enum variant, sharing the existing numeric phase proof.
//! Unsupported transitions finish the synchronous loan before their original
//! source action. This is independent of program names, timings and datasets.
use super::*;
use mech_engine::native_artifact::{
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Variant {
    pub carrier: u32,
    pub ordinal: u32,
    /// Only these exact source values have the guarded enum schema.
    pub values: Vec<u32>,
    /// Physical active-payload scalar homes for ordinary continuation exports.
    pub scalars: Vec<(u32, Vec<ScalarBinding>)>,
}
impl Variant {
    pub fn contains(&self, value: u32) -> bool {
        self.values.binary_search(&value).is_ok()
    }
}

pub(super) fn derive(
    facts: &NativeProgramFacts<'_>,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    let [root] = frames
        .fields(tail.carrier())
        .ok_or("tagged carrier lacks frame")?
    else {
        return Err("tagged carrier is not a closed enum");
    };
    let Lane::Tagged { count, .. } = root.lane() else {
        return Err("carrier is not tagged");
    };
    let mut selected = None;
    let mut score = 0;
    for ordinal in 0..count {
        if let Ok(proof) = candidate(
            facts,
            tail,
            frames,
            aggregates,
            calls,
            visits,
            ordinal as u32,
        ) {
            // Static profitability only. Prefer represented numeric work; ties
            // retain schema order. Runtime measurements never select a variant.
            let current = proof
                .actions
                .iter()
                .filter(|(_, op)| {
                    matches!(
                        op,
                        Operation::Read { .. }
                            | Operation::CellRead { .. }
                            | Operation::Write { .. }
                            | Operation::CellWrite { .. }
                    )
                })
                .count();
            if selected.is_none() || current > score {
                selected = Some(proof);
                score = current;
            }
        }
    }
    selected.ok_or("no guarded numeric variant retains a native backedge")
}

pub(in crate::native_artifact) fn candidate(
    facts: &NativeProgramFacts<'_>,
    tail: &NativeTailLoop,
    original: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    ordinal: u32,
) -> Result<Proof, &'static str> {
    let plan = facts.plan();
    let schema = plan.values()[tail.carrier().get() as usize].schema();
    let projected = original
        .numeric_variant_projection(plan, tail.carrier(), ordinal)
        .ok_or("variant lacks a closed tuple payload")?;
    let iteration = plan.blocks()[tail.header() as usize].iteration();
    let mut stops = BTreeSet::new();
    for (site, block) in plan.blocks().iter().enumerate() {
        if block.iteration() != iteration {
            continue;
        }
        let supported = if let Some(frame) = projected.action(site) {
            match frame.operation() {
                Frame::Pack { .. } => calls[site]
                    .as_ref()
                    .is_some_and(|c| c.supports_invariant_buffer_frame()),
                Frame::Copy { .. } | Frame::Project { .. } => {
                    if let Some(Frame::EnumPack {
                        ordinal: packed, ..
                    }) = original.action(site).map(|a| a.operation())
                    {
                        *packed == ordinal
                            && calls[site]
                                .as_ref()
                                .is_some_and(|call| call.supports_invariant_tagged_frame(ordinal))
                    } else {
                        true
                    }
                }
                Frame::Observe {
                    source,
                    path,
                    observation,
                    ..
                } => {
                    plan.values()[source.get() as usize].schema() == schema
                        && path.is_empty()
                        && matches!(observation, Observation::EnumOrdinal { .. })
                }
                Frame::EnumPack { .. } => false,
            }
        } else if aggregates.action(site).is_some() {
            true
        } else {
            match block.action() {
                Action::Scalar(Scalar::None | Scalar::Primitive { .. } | Scalar::Copy { .. })
                | Action::Latch { .. } => true,
                Action::LibraryCall { .. } => calls[site].as_ref().is_some_and(|c| {
                    scalar_read(c) || scalar_write(c) || c.supports_unsigned_matrix_length()
                }),
                _ => false,
            }
        };
        if !supported {
            stops.insert(site);
        }
    }
    if stops.is_empty() {
        return Err("guarded phase lacks a cold transition");
    }
    let phase = super::super::phase_numeric::derive_guarded(
        facts, tail, &projected, aggregates, calls, visits, stops,
    )?;
    let sites = phase.sites.iter().copied().collect::<BTreeSet<_>>();
    let native = |site| {
        sites.contains(&site)
            && !phase
                .actions
                .iter()
                .any(|(s, op)| *s == site && matches!(op, Operation::Fallback))
    };
    // Tag identity is an independent induction, not an inference from matching
    // payload schemas or array roots. Ignore only definitions beyond exact
    // cold exits, which cannot produce a value during this synchronous loan.
    fn hot(
        facts: &NativeProgramFacts<'_>,
        original: &NativeBufferFrames,
        tail: &NativeTailLoop,
        ordinal: u32,
        value: u32,
        native: &impl Fn(usize) -> bool,
        active: &mut BTreeSet<u32>,
    ) -> bool {
        if value == tail.carrier().get() {
            return true;
        }
        if !active.insert(value) {
            return false;
        }
        let Ok(definitions) = facts.definition_sites(value) else {
            active.remove(&value);
            return false;
        };
        let definitions = definitions
            .iter()
            .filter_map(|d| d.site())
            .filter(|s| native(*s as usize))
            .collect::<Vec<_>>();
        let answer = !definitions.is_empty()
            && definitions.into_iter().all(|site| {
                match original.action(site as usize).map(|a| a.operation()) {
                    Some(Frame::EnumPack {
                        ordinal: packed, ..
                    }) => *packed == ordinal,
                    Some(Frame::Copy { source, .. }) => {
                        hot(facts, original, tail, ordinal, source.get(), native, active)
                    }
                    _ => match facts.plan().blocks()[site as usize].action() {
                        Action::Scalar(Scalar::Copy { source, .. }) => {
                            hot(facts, original, tail, ordinal, source.get(), native, active)
                        }
                        _ => false,
                    },
                }
            });
        active.remove(&value);
        answer
    }
    if !sites.contains(&(tail.latch() as usize))
        || !hot(
            facts,
            original,
            tail,
            ordinal,
            tail.next().get(),
            &native,
            &mut BTreeSet::new(),
        )
    {
        return Err("native backedge does not preserve the guarded tag");
    }
    for &(site, ref op) in &phase.actions {
        if matches!(op, Operation::Fallback) {
            continue;
        }
        if let Some(Frame::Observe { source, .. } | Frame::Copy { source, .. }) =
            original.action(site).map(|a| a.operation())
            && plan.values()[source.get() as usize].schema() == schema
            && !hot(
                facts,
                original,
                tail,
                ordinal,
                source.get(),
                &native,
                &mut BTreeSet::new(),
            )
        {
            return Err("native observation is not rooted in the guarded tag");
        }
    }
    let mut proof = from_phase(plan, tail, &projected, aggregates, phase)?;
    let values = proof
        .frames
        .iter()
        .filter_map(|frame| {
            (plan.values()[frame.value as usize].schema() == schema).then_some(frame.value)
        })
        .collect::<Vec<_>>();
    let scalars = proof
        .frames
        .iter()
        .filter(|frame| values.binary_search(&frame.value).is_ok())
        .map(|frame| {
            Ok((
                frame.value,
                scalar_bindings(&projected, frame.value, &frame.scalar_paths)?,
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    for binding in &mut proof.scalar_inputs {
        binding.steps += 2;
    }
    for frame in &mut proof.frames {
        if values.binary_search(&frame.value).is_ok() {
            // Include the enum root and payload tuple in the logical output
            // envelope even though the physical bank remains private.
            frame.scalar_node_count += 2;
        }
    }
    proof.tagged = Some(Box::new(Variant {
        carrier: tail.carrier().get(),
        ordinal,
        values,
        scalars,
    }));
    Ok(proof)
}

/// Project the original enum action into the independently proved hot payload.
/// The physical original layout remains the runtime owner and validity bank.
pub(in crate::native_artifact) fn operation(operation: &Frame, variant: &Variant) -> Option<Frame> {
    match operation {
        Frame::EnumPack {
            output,
            payload,
            ordinal,
        } if variant.contains(output.get()) && *ordinal == variant.ordinal => Some(Frame::Copy {
            source: *payload,
            output: *output,
        }),
        Frame::Observe {
            source,
            path,
            observation,
            output,
        } if variant.contains(source.get())
            && matches!(observation, Observation::U64 | Observation::CanonicalValue) =>
        {
            match path.as_ref() {
                [Step::EnumPayload { ordinal }] if *ordinal == variant.ordinal => {
                    Some(Frame::Copy {
                        source: *source,
                        output: *output,
                    })
                }
                [
                    Step::EnumPayload { ordinal },
                    Step::TupleItem { ordinal: field },
                ] if *ordinal == variant.ordinal => Some(Frame::Project {
                    source: *source,
                    ordinal: *field as usize,
                    output: *output,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}
