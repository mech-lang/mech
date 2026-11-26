//! Source-only proof for complete range comprehensions inside numeric phases.
//! Eligibility grants no owner, mutable view, source visit, or resource credit.
use super::super::numeric_maps::{Layout, Region};
use super::reductions::constant;
use super::*;
#[path = "matrix_map_budget.rs"]
mod budget;
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeCollectionSource as Source,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct MatrixMap {
    pub region: Region,
    pub prefix: Vec<u32>,
    pub completion: Vec<u32>,
    pub entry: u32,
    pub after: u32,
    pub first: u32,
    pub last: u32,
    pub inclusive: bool,
    pub count: u64,
    pub output: u32,
    pub extents: [u64; 2],
}
impl MatrixMap {
    pub fn source_sites(&self) -> impl Iterator<Item = u32> + '_ {
        self.prefix
            .iter()
            .chain(&self.region.sites)
            .chain(&self.completion)
            .copied()
    }
    pub fn scalar_values(&self) -> impl Iterator<Item = u32> + '_ {
        [self.first, self.last]
            .into_iter()
            .chain(self.region.captures.iter().copied())
    }
}
fn candidate(plan: &Plan, region: Region) -> Option<MatrixMap> {
    if !region.range
        || region.rectangle.is_some()
        || region.reduction.is_some()
        || !region.nested.is_empty()
    {
        return None;
    }
    let NativeControlLoop::Collection(child) = &plan.iterations()[region.iteration as usize] else {
        return None;
    };
    let Source::U64Range(cursor) = child.source() else {
        return None;
    };
    let definition = &plan.collections()[region.collection as usize];
    if definition.kind() != mech_engine::ComprehensionKind::Matrix {
        return None;
    }
    let [first, last] = cursor.operands();
    let inclusive = match cursor.template().primitive() {
        Primitive::RangeExclusive => false,
        Primitive::RangeInclusive => true,
        _ => return None,
    };
    let count = constant(plan, last.get())?
        .checked_sub(constant(plan, first.get())?)?
        .checked_add(u64::from(inclusive))?;
    // This is a finite scratch-bank eligibility envelope, never a source bound.
    // Larger, dynamic, empty, and overflowed sources retain their original CFG.
    if count == 0 || count.checked_mul(u64::from(region.output_width))? > 65536 {
        return None;
    }
    for schema in [
        cursor.template().output(),
        plan.values()[region.output as usize].definition(),
    ] {
        mech_core::shape_for_resolved_extents(schema, &[1, count]).ok()?;
    }
    let mut completion = vec![definition.finish()];
    let (terminal, output, extents) = if let Some(t) = &region.transpose {
        let Action::LibraryCall { template, .. } = plan.blocks()[t.transpose as usize].action()
        else {
            return None;
        };
        mech_core::shape_for_resolved_extents(&template.inputs()[0], &[1, count]).ok()?;
        mech_core::shape_for_resolved_extents(template.output(), &[count, 1]).ok()?;
        completion.extend(&t.continuation);
        (t.transpose, t.output, [count, 1])
    } else {
        (definition.finish(), region.output, [1, count])
    };
    mech_core::shape_for_resolved_extents(plan.values()[output as usize].definition(), &extents)
        .ok()?;
    if completion
        .iter()
        .any(|&at| plan.activation_boundary() == Some(at))
    {
        return None;
    }
    let Term::Scalar(Branch::Jump(after)) = plan.blocks()[terminal as usize].terminator() else {
        return None;
    };
    let entry = if cursor.before_collection_begin() {
        cursor.prepare()
    } else {
        definition.begin()
    };
    let mut prefix = Vec::new();
    let mut seen = BTreeSet::new();
    let mut at = entry;
    while at != child.header() {
        if prefix.len() == 32 || !seen.insert(at) || plan.activation_boundary() == Some(at) {
            return None;
        }
        let block = &plan.blocks()[at as usize];
        if block.iteration() != child.parent() {
            return None;
        }
        match block.action() {
            Action::PrepareU64Range { iteration }
                if *iteration == region.iteration && at == cursor.prepare() => {}
            Action::CollectionBegin { collection }
                if *collection == region.collection && at == definition.begin() => {}
            Action::Initialize { iteration }
                if *iteration == region.iteration && at == child.initialize() => {}
            Action::Scalar(Scalar::None) => {}
            _ => return None,
        }
        let Term::Scalar(Branch::Jump(next)) = block.terminator() else {
            return None;
        };
        prefix.push(at);
        at = next;
    }
    if ![cursor.prepare(), definition.begin(), child.initialize()]
        .iter()
        .all(|s| prefix.contains(s))
    {
        return None;
    }
    Some(MatrixMap {
        region,
        prefix,
        completion,
        entry,
        after,
        first: first.get(),
        last: last.get(),
        inclusive,
        count,
        output,
        extents,
    })
}
pub(in crate::native_artifact) fn inventory(plan: &Plan) -> Result<Vec<MatrixMap>, &'static str> {
    Ok(Layout::derive(plan)?
        .regions
        .into_iter()
        .filter_map(|r| candidate(plan, r))
        .collect())
}

/// Reconstruct destination identities from the canonical next-state fields.
/// Read operands never choose a destination. Conflicting state roots, swaps,
/// unknown producers, and cycles decline; functional freshness is a separate
/// enclosing phase obligation.
pub(in crate::native_artifact) fn destinations(
    facts: &NativeProgramFacts<'_>,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    calls: &[Option<NativePreparedLibraryCall>],
    maps: &[MatrixMap],
) -> Result<BTreeMap<u32, usize>, &'static str> {
    struct Bindings<'a, 'p> {
        facts: &'a NativeProgramFacts<'p>,
        tail: &'a NativeTailLoop,
        frames: &'a NativeBufferFrames,
        calls: &'a [Option<NativePreparedLibraryCall>],
        maps: &'a [MatrixMap],
        active: BTreeSet<(u32, Option<usize>)>,
        found: BTreeMap<u32, usize>,
    }
    impl Bindings<'_, '_> {
        fn bind(&mut self, value: u32, field: Option<usize>, root: usize) -> Option<()> {
            use mech_engine::native_artifact::NativeBufferFrameOperation as Frame;
            if value == self.tail.carrier().get() {
                return (field == Some(root)).then_some(());
            }
            if field.is_none() && self.maps.iter().any(|m| m.output == value) {
                return match self.found.insert(value, root) {
                    Some(previous) if previous != root => None,
                    _ => Some(()),
                };
            }
            let key = (value, field);
            if !self.active.insert(key) {
                return None;
            }
            let definitions = self.facts.definition_sites(value).ok()?.to_vec();
            if definitions.is_empty() {
                return None;
            }
            for definition in definitions {
                let site = definition.site()? as usize;
                let action = self.facts.plan().blocks()[site].action();
                if let Some(frame) = self.frames.action(site) {
                    match (frame.operation(), field) {
                        (Frame::Pack { output }, Some(ordinal)) if output.get() == value => {
                            let Action::LibraryCall { operands, .. } = action else {
                                return None;
                            };
                            self.bind(operands.get(ordinal)?.get(), None, root)?;
                        }
                        (Frame::Copy { source, output }, Some(ordinal))
                            if output.get() == value =>
                        {
                            if super::super::readonly_numeric::numeric_fields::direct_pack(
                                self.facts.plan(),
                                self.frames,
                                *source,
                                *output,
                            ) {
                                if ordinal != 0 {
                                    return None;
                                }
                                self.bind(source.get(), None, root)?;
                            } else {
                                self.bind(source.get(), Some(ordinal), root)?;
                            }
                        }
                        (
                            Frame::Project {
                                source,
                                ordinal,
                                output,
                            },
                            None,
                        ) if output.get() == value => {
                            self.bind(source.get(), Some(*ordinal), root)?;
                        }
                        _ => return None,
                    }
                } else {
                    match (action, field) {
                        (Action::Scalar(Scalar::Copy { source, output }), None)
                            if output.get() == value =>
                        {
                            self.bind(source.get(), None, root)?;
                        }
                        (
                            Action::LibraryCall {
                                operands, output, ..
                            },
                            None,
                        ) if output.get() == value => {
                            let call = self.calls.get(site)?.as_ref()?;
                            if !super::super::readonly_numeric::scalar_write(call) {
                                return None;
                            }
                            self.bind(operands.first()?.get(), None, root)?;
                        }
                        _ => return None,
                    }
                }
            }
            self.active.remove(&key);
            Some(())
        }
    }
    let mut bindings = Bindings {
        facts,
        tail,
        frames,
        calls,
        maps,
        active: BTreeSet::new(),
        found: BTreeMap::new(),
    };
    for (ordinal, field) in frames
        .fields(tail.carrier())
        .ok_or("map destination lacks private carrier fields")?
        .iter()
        .enumerate()
    {
        if matches!(
            field.lane(),
            mech_engine::native_artifact::NativeBufferFrameLane::Managed { .. }
        ) {
            bindings
                .bind(tail.next().get(), Some(ordinal), ordinal)
                .ok_or("map destination changes or conflicts with its state root")?;
        }
    }
    Ok(bindings.found)
}

/// Complete map replacement under the enclosing phase's root/version proof.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Replacement {
    pub source: MatrixMap,
    pub arrays: Vec<usize>,
    pub target: usize,
    pub budget: super::reductions::FoldBudget,
}
