//! Complete common-prefix FSM phases. The original tag bank and source facts
//! remain authoritative; this metadata grants no ownership or visit by itself.
use super::readonly_numeric::*;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Prefix {
    pub longest: u32,
    pub counts: Vec<usize>,
    /// Discriminators are private words outside the original scalar/view bank.
    pub tags: Vec<(u32, usize)>,
    pub variants: Vec<Physical>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Physical {
    pub value: u32,
    pub ordinal: u32,
    pub scalars: Vec<ScalarBinding>,
    pub arrays: Vec<(usize, usize)>,
}
impl Prefix {
    pub fn tag(&self, value: u32) -> Option<usize> {
        self.tags
            .binary_search_by_key(&value, |(v, _)| *v)
            .ok()
            .map(|at| self.tags[at].1)
    }
    pub fn physical(&self, value: u32, ordinal: u32) -> Option<&Physical> {
        self.variants
            .iter()
            .find(|p| p.value == value && p.ordinal == ordinal)
    }
}

use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeIterationTerminator as Term, NativeMatchTerminator as Branch, NativeMatchValue,
    NativePreparedLibraryCall, NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts,
    NativeScalarAggregates, NativeStructuralObservation as Observation,
    NativeStructuralStep as Step, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};

fn schema(plan: &Plan, value: NativeMatchValue, root: mech_core::SchemaId) -> bool {
    plan.values()[value.get() as usize].schema() == root
}
/// Every payload observation is reached only with its exact current tag. Tag
/// sets are joined conservatively after cutting the source-declared latch;
/// every constructor is still checked against its actual selected factory.
fn tag_flow(
    facts: &Facts<'_>,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    count: usize,
) -> Result<(), &'static str> {
    let plan = facts.plan();
    let root = plan.values()[tail.carrier().get() as usize].schema();
    let forest = super::numeric_phase::Forest::guarded_intervals(plan, tail, BTreeSet::new())?;
    if forest.loops.len() != 1 || !forest.stops.is_empty() {
        return Err("prefix phase is not one complete Tail");
    }
    let all = (1u64
        .checked_shl(count as u32)
        .ok_or("prefix tags exceed word")?)
    .checked_sub(1)
    .ok_or("prefix has no tags")?;
    let mut states = BTreeMap::from([(
        tail.header() as usize,
        BTreeMap::from([(tail.carrier().get(), all)]),
    )]);
    for &site in &forest.order {
        let Some(mut state) = states.remove(&site) else {
            continue;
        };
        if let Some(action) = frames.action(site) {
            match action.operation() {
                Frame::EnumPack {
                    output, ordinal, ..
                } if schema(plan, *output, root) => {
                    if *ordinal as usize >= count {
                        return Err("prefix constructor tag is out of range");
                    }
                    state.insert(output.get(), 1u64 << ordinal);
                }
                Frame::Copy { source, output } if schema(plan, *output, root) => {
                    let tags = *state
                        .get(&source.get())
                        .ok_or("prefix copy lacks current tag")?;
                    state.insert(output.get(), tags);
                }
                Frame::Observe { source, path, .. } if schema(plan, *source, root) => {
                    if let Some(Step::EnumPayload { ordinal }) = path.first() {
                        if state.get(&source.get()) != Some(&(1u64 << ordinal)) {
                            return Err("prefix payload is not guarded by its exact source tag");
                        }
                    }
                }
                _ => {}
            }
        }
        if site == tail.latch() as usize {
            if state
                .get(&tail.next().get())
                .is_none_or(|&tags| tags == 0 || tags & !all != 0)
            {
                return Err("prefix backedge loses current tag");
            }
            continue;
        }
        let candidates = match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => Vec::new(),
        };
        let mut predicate = None;
        for value in candidates {
            let mut source = value;
            let mut seen = BTreeSet::new();
            while seen.insert(source.get()) {
                match facts.unique_identity_copy(source) {
                    Some(v) => source = v,
                    None => break,
                }
            }
            let Ok([definition]) = facts.definition_sites(source.get()) else {
                continue;
            };
            let Some(at) = definition.site() else {
                continue;
            };
            let Some(Frame::Observe {
                source,
                path,
                observation: Observation::EnumOrdinal { ordinal },
                ..
            }) = frames.action(at as usize).map(|a| a.operation())
            else {
                continue;
            };
            if !path.is_empty() || !schema(plan, *source, root) {
                continue;
            }
            let Some((no, yes)) = facts.boolean_branch_edges(site as u32, value) else {
                continue;
            };
            predicate = Some((source.get(), *ordinal, no, yes));
            break;
        }
        for next in forest.successors(plan, site).into_iter().map(|n| n as u32) {
            if next == tail.exit() {
                continue;
            }
            let mut next_state = state.clone();
            if let Some((source, ordinal, no, yes)) = predicate {
                let old = *state
                    .get(&source)
                    .ok_or("prefix predicate lacks current tag")?;
                let bit = 1u64 << ordinal;
                let tags = if next == yes {
                    old & bit
                } else if next == no {
                    old & !bit
                } else {
                    return Err("prefix tag predicate has a foreign edge");
                };
                if tags == 0 {
                    continue;
                }
                next_state.insert(source, tags);
            }
            if let Some(old) = states.get_mut(&(next as usize)) {
                let keys = old
                    .keys()
                    .chain(next_state.keys())
                    .copied()
                    .collect::<BTreeSet<_>>();
                for key in keys {
                    let tags = old.get(&key).copied().unwrap_or(all)
                        | next_state.get(&key).copied().unwrap_or(all);
                    old.insert(key, tags);
                }
            } else {
                states.insert(next as usize, next_state);
            }
        }
    }
    Ok(())
}

pub(in crate::native_artifact) fn derive(
    facts: &Facts<'_>,
    tail: &NativeTailLoop,
    original: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    let plan = facts.plan();
    let (projected, longest) = original
        .numeric_prefix_projection(plan, tail.carrier())
        .ok_or("enum payloads lack a numeric common prefix")?;
    let [root] = original
        .fields(tail.carrier())
        .ok_or("prefix carrier lacks enum")?
    else {
        return Err("prefix carrier has multiple roots");
    };
    let Lane::Tagged { count, .. } = root.lane() else {
        return Err("prefix carrier lacks tag");
    };
    tag_flow(facts, tail, original, count)?;
    for (site, block) in plan.blocks().iter().enumerate() {
        if block.iteration() != plan.blocks()[tail.header() as usize].iteration() {
            continue;
        }
        if let Some(Frame::EnumPack { ordinal, .. }) = original.action(site).map(|a| a.operation())
        {
            if !calls
                .get(site)
                .and_then(Option::as_ref)
                .is_some_and(|c| c.supports_invariant_tagged_frame(*ordinal))
            {
                return Err("prefix constructor lacks exact invariant factory");
            }
        }
    }
    let phase = super::phase_numeric::derive_guarded(
        facts,
        tail,
        &projected,
        aggregates,
        calls,
        visits,
        BTreeSet::new(),
    )?;
    if phase.exits.iter().any(|exit| {
        !phase.actions.iter().any(|(site, action)| {
            *site == exit.to && matches!(action, super::numeric_plan::Operation::Reduce { .. })
        })
    }) {
        return Err("prefix phase has an ordinary cold action");
    }
    let mut proof = super::readonly_numeric::from_phase(plan, tail, &projected, aggregates, phase)?;
    if proof.result.direct_array().is_none() {
        return Err("prefix phase needs a direct managed result");
    }
    for array in &mut proof.arrays {
        if let ArrayStorage::Carrier { field, .. } = array.storage {
            array.storage = ArrayStorage::TaggedCarrier {
                value: tail.carrier().get(),
                field,
            };
        }
    }
    for binding in &mut proof.scalar_inputs {
        binding.steps += 2;
    }
    let root_schema = plan.values()[tail.carrier().get() as usize].schema();
    let values = proof
        .frames
        .iter()
        .filter(|f| plan.values()[f.value as usize].schema() == root_schema)
        .map(|f| f.value)
        .collect::<Vec<_>>();
    let mut tags = Vec::new();
    let mut physical = Vec::new();
    let mut counts = Vec::new();
    for ordinal in 0..count {
        let variant = original
            .tagged_variant(*root, ordinal as u32)
            .ok_or("prefix variant missing")?;
        counts.push(
            original
                .children(variant)
                .unwrap_or(core::slice::from_ref(&variant))
                .len(),
        );
    }
    for frame in &mut proof.frames {
        if values.binary_search(&frame.value).is_err() {
            continue;
        }
        let tag_word = proof.words;
        proof.words = proof
            .words
            .checked_add(1)
            .ok_or("prefix tag homes overflow")?;
        tags.push((frame.value, tag_word));
        let [root] = original
            .fields_at(frame.value as usize)
            .ok_or("prefix frame missing")?
        else {
            return Err("prefix frame is not an enum");
        };
        for ordinal in 0..count {
            let variant = original
                .tagged_variant(*root, ordinal as u32)
                .ok_or("prefix physical variant missing")?;
            let fields = original
                .children(variant)
                .unwrap_or(core::slice::from_ref(&variant));
            let mut scalars = Vec::new();
            let mut arrays = Vec::new();
            for (at, field) in fields.iter().enumerate() {
                match field.lane() {
                    Lane::Scalar { at: physical, kind } => scalars.push(ScalarBinding {
                        value: frame.value,
                        word: frame.fields[at].ok_or("prefix scalar word missing")?,
                        physical,
                        kind,
                        steps: 3,
                    }),
                    Lane::Managed { at: physical } => arrays.push((
                        physical,
                        frame.arrays[at].ok_or("prefix array root missing")?,
                    )),
                    _ => return Err("prefix payload is not flat"),
                }
            }
            physical.push(Physical {
                value: frame.value,
                ordinal: ordinal as u32,
                scalars,
                arrays,
            });
        }
        let mut owned = OwnedFrameWords {
            value: frame.value,
            fields: frame.fields.clone(),
            arrays: frame.arrays.clone(),
            array_bindings: frame.array_bindings.clone(),
            maximum_arity: frame.maximum_arity,
            scalar_paths: frame.scalar_paths.clone(),
            scalar_node_count: frame.scalar_node_count + 2,
        };
        owned.fields.push(Some(tag_word));
        owned.arrays.push(None);
        owned
            .scalar_paths
            .push((vec![frame.fields.len()], tag_word));
        *frame = FrameWords::from(owned);
    }
    proof.boundary_words = proof
        .boundary_words
        .checked_add(tags.len())
        .ok_or("prefix boundary words overflow")?;
    let extra = u64::try_from(tags.len())
        .ok()
        .and_then(|n| n.checked_mul(16))
        .and_then(|n| n.checked_add(64))
        .and_then(|n| {
            n.checked_add(
                (count as u64).checked_mul(
                    (proof.scalar_inputs.len() as u64)
                        .checked_add(1)?
                        .checked_mul(2)?
                        .checked_add(8)?,
                )?,
            )
        })
        .ok_or("prefix boundary price overflows")?;
    proof.boundary_work = proof
        .boundary_work
        .checked_add(extra)
        .ok_or("prefix boundary price overflows")?;
    let schedule = proof
        .phase
        .as_mut()
        .ok_or("prefix lacks its complete phase schedule")?;
    if schedule.loops.len() != 1 {
        return Err("prefix schedule is not one complete Tail");
    }
    schedule.boundary_work = proof.boundary_work;
    schedule.loops[0].compute = schedule.loops[0]
        .compute
        .checked_add(extra)
        .ok_or("prefix traversal price overflows")?;
    schedule.compute_bound = super::numeric_phase::Bound::add(
        schedule.compute_bound.clone(),
        super::numeric_phase::Bound::Constant(extra),
    );
    let Lane::Tagged {
        tag: physical_tag, ..
    } = root.lane()
    else {
        unreachable!()
    };
    let tag_word = tags
        .iter()
        .find(|(value, _)| *value == tail.carrier().get())
        .ok_or("prefix carrier tag word missing")?
        .1;
    proof.scalar_inputs.push(ScalarBinding {
        value: tail.carrier().get(),
        word: tag_word,
        physical: physical_tag,
        kind: mech_engine::native_artifact::NativeScalarKind::U64,
        steps: 1,
    });
    let prefix = Prefix {
        longest,
        counts,
        tags,
        variants: physical,
    };
    proof.tagged = Some(Box::new(super::readonly_numeric::tagged::Variant {
        prefix: Some(Box::new(prefix)),
        carrier: tail.carrier().get(),
        ordinal: longest,
        values,
        scalars: Vec::new(),
    }));
    Ok(proof)
}
