//! Total pointwise unsigned collections, derived from the canonical scalar CFG.
//! This does not execute source or admit a buffer. Current owners and funding
//! are checked at the original fresh collection initializer by the runtime.
use super::map_admission;
use mech_engine::native_artifact::{
    NativeCollectionResult, NativeCollectionSource, NativeControlLoop,
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts, NativeProgramProducerRole as Producer,
    NativeProgramReadRole as Read, NativeScalarKind,
};
use std::collections::{BTreeMap, BTreeSet};
mod bounds;

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Region {
    pub collection: u32,
    pub iteration: u32,
    pub initialize: u32,
    pub header: u32,
    pub body: u32,
    pub latch: u32,
    pub source: u32,
    pub element: u32,
    pub push: u32,
    pub yielded: u32,
    pub input_width: u8,
    pub output_width: u8,
    pub sites: Box<[u32]>,
    pub captures: Box<[u32]>,
    pub words: Box<[u32]>,
    /// The existing ordinary header/element/scalar-group/push/latch prices.
    /// The ordinary final header remains after the native traversal.
    pub visit_work: u64,
}
fn width(kind: NativeScalarKind) -> Option<u8> {
    Some(match kind {
        NativeScalarKind::U8 => 1,
        NativeScalarKind::U16 => 2,
        NativeScalarKind::U32 => 4,
        NativeScalarKind::U64 => 8,
        _ => return None,
    })
}
pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    admission: &map_admission::Layout,
) -> Vec<Region> {
    let facts = plan.facts();
    (0..plan.collections().len())
        .filter_map(|id| select(plan, &facts, visits, admission, id))
        .collect()
}
fn select(
    plan: &Plan,
    facts: &NativeProgramFacts<'_>,
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    admission: &map_admission::Layout,
    id: usize,
) -> Option<Region> {
    let collection = &plan.collections()[id];
    if !matches!(collection.result(), NativeCollectionResult::Materialized) {
        return None;
    }
    let [iteration] = collection.generators() else {
        return None;
    };
    let NativeControlLoop::Collection(generator) = &plan.iterations()[*iteration as usize] else {
        return None;
    };
    let NativeCollectionSource::Value(source) = generator.source() else {
        return None;
    };
    let element = generator.element()?;
    let input_kind = plan.values()[element.get() as usize].scalar_kind()?;
    let input_width = width(input_kind)?;
    let mech_core::SchemaBody::Matrix {
        element: source_type,
        dimensions,
    } = plan.values()[source.get() as usize].definition().body()
    else {
        return None;
    };
    if dimensions.len() != 2 || NativeScalarKind::from_schema_body(source_type)? != input_kind {
        return None;
    }
    let Action::CollectionPush {
        collection: owner,
        value: yielded,
    } = plan.blocks()[collection.push() as usize].action()
    else {
        return None;
    };
    if *owner != id as u32 {
        return None;
    }
    let output_width = width(plan.values()[yielded.get() as usize].scalar_kind()?)?;
    let mech_core::SchemaBody::Matrix {
        element: output_type,
        dimensions,
    } = plan.values()[collection.output().get() as usize]
        .definition()
        .body()
    else {
        return None;
    };
    if dimensions.len() != 2
        || width(NativeScalarKind::from_schema_body(output_type)?)? != output_width
    {
        return None;
    }
    let mut sites = BTreeSet::new();
    let mut order = Vec::new();
    let mut active = BTreeSet::new();
    fn graph(
        plan: &Plan,
        at: u32,
        push: u32,
        iteration: u32,
        sites: &mut BTreeSet<u32>,
        active: &mut BTreeSet<u32>,
        order: &mut Vec<u32>,
    ) -> Option<()> {
        if active.contains(&at) {
            return None;
        }
        if sites.contains(&at) {
            return Some(());
        }
        let block = plan.blocks().get(at as usize)?;
        if block.iteration() != Some(iteration) {
            return None;
        }
        if !matches!(
            block.action(),
            Action::Scalar(_) | Action::CollectionElement { .. } | Action::CollectionPush { .. }
        ) {
            return None;
        }
        active.insert(at);
        if at != push {
            match block.terminator() {
                Term::Scalar(Branch::Jump(next)) => {
                    graph(plan, next, push, iteration, sites, active, order)?
                }
                Term::Scalar(
                    Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. },
                ) => {
                    graph(plan, yes, push, iteration, sites, active, order)?;
                    graph(plan, no, push, iteration, sites, active, order)?;
                }
                Term::Scalar(Branch::Reject | Branch::IntegrityFailure) => {}
                _ => return None,
            }
        }
        active.remove(&at);
        sites.insert(at);
        order.push(at);
        Some(())
    }
    graph(
        plan,
        generator.body(),
        collection.push(),
        *iteration,
        &mut sites,
        &mut active,
        &mut order,
    )?;
    order.reverse();
    if !sites.contains(&collection.push()) {
        return None;
    }
    if !matches!(plan.blocks()[generator.body() as usize].action(), Action::CollectionElement { iteration: actual } if actual == iteration)
        || plan.blocks()[collection.push() as usize].terminator()
            != Term::Scalar(Branch::Jump(generator.latch()))
        || !matches!(plan.blocks()[generator.latch() as usize].action(), Action::Latch { iteration: actual } if actual == iteration)
        || plan.blocks()[generator.latch() as usize].terminator()
            != Term::Scalar(Branch::Jump(generator.header()))
    {
        return None;
    }
    let mut reads = BTreeSet::new();
    let mut definitions = BTreeSet::new();
    let mut written_values = BTreeMap::new();
    let mut words = BTreeSet::new();
    for &site in &sites {
        let block = &plan.blocks()[site as usize];
        match block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::Scalar(Scalar::Copy { source, output }) => {
                reads.insert(source.get());
                definitions.insert(output.get());
                written_values.insert(output.get(), *output);
            }
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            }) => {
                reads.extend(operands.iter().map(|v| v.get()));
                definitions.insert(output.get());
                written_values.insert(output.get(), *output);
            }
            Action::CollectionElement { .. } if site == generator.body() => {
                definitions.insert(element.get());
                written_values.insert(element.get(), element);
            }
            Action::CollectionPush { value, .. } if site == collection.push() => {
                reads.insert(value.get());
            }
            _ => return None,
        }
        match block.terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => {
                reads.insert(condition.get());
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                reads.extend([left.get(), right.get()]);
            }
            _ => {}
        }
    }
    words.extend(&reads);
    words.extend(&definitions);
    if words.iter().any(|&v| {
        plan.values()[v as usize]
            .scalar_kind()
            .and_then(bounds::Range::kind)
            .is_none()
    }) {
        return None;
    }
    let captures = reads
        .difference(&definitions)
        .copied()
        .collect::<BTreeSet<_>>();
    for &value in &captures {
        for definition in facts.definition_sites(value).ok()? {
            if let Some(site) = definition.site() {
                if sites.contains(&site)
                    || !facts.strictly_dominates(site, generator.header()).ok()?
                {
                    return None;
                }
            } else if !matches!(definition.role(), Producer::Input | Producer::Constant) {
                return None;
            }
        }
    }
    for &value in &definitions {
        if facts.read_roles(written_values[&value]).ok()?.iter().any(|read| !matches!(read, Read::Action(site) | Read::Terminator(site) if sites.contains(site))) { return None; }
    }
    let mut initial = bounds::State::default();
    for &value in &captures {
        initial.ranges.insert(
            value,
            bounds::Range::kind(plan.values()[value as usize].scalar_kind()?)?,
        );
    }
    for constant in plan.constants() {
        let value = constant.value().get();
        if !captures.contains(&value) {
            continue;
        }
        use mech_core::ValueData as D;
        let word = match constant.constant().data() {
            D::Bool(v) => u64::from(*v),
            D::U8(v) => u64::from(*v),
            D::U16(v) => u64::from(*v),
            D::U32(v) => u64::from(*v),
            D::U64(v) => *v,
            _ => return None,
        };
        initial.ranges.insert(value, bounds::Range(word, word));
    }
    let mut states = BTreeMap::from([(generator.body(), initial)]);
    let mut group = None;
    for &site in &order {
        let Some(mut state) = states.remove(&site) else {
            continue;
        };
        let block = &plan.blocks()[site as usize];
        if matches!(block.action(), Action::Scalar(_)) {
            let current = admission.group(site as usize)?;
            if current.owner.is_some() || group.is_some_and(|id| id != current.start()) {
                return None;
            }
            group = Some(current.start());
        }
        match block.action() {
            Action::CollectionElement { .. } => {
                state
                    .ranges
                    .insert(element.get(), bounds::Range::kind(input_kind)?);
            }
            Action::CollectionPush { value, .. } => {
                state.ranges.get(&value.get())?;
                continue;
            }
            Action::Scalar(Scalar::None) => {}
            Action::Scalar(Scalar::Copy { source, output }) => state.copy(*source, *output)?,
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) => state.primitive(*op, operands, *output)?,
            _ => return None,
        }
        let next = match block.terminator() {
            Term::Scalar(Branch::Jump(next)) => vec![(next, Some(state))],
            Term::Scalar(Branch::Branch { condition, yes, no }) => vec![
                (yes, state.clone().boolean(condition, true)),
                (no, state.boolean(condition, false)),
            ],
            Term::Scalar(Branch::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => vec![
                (yes, state.clone().literal(left, right, true)),
                (no, state.literal(left, right, false)),
            ],
            _ => return None, // A reached rejection or assertion keeps ordinary execution.
        };
        for (next, state) in next {
            if let Some(state) = state {
                states
                    .entry(next)
                    .and_modify(|old| old.merge(&state))
                    .or_insert(state);
            }
        }
    }
    let scalar_work = group.map_or(Some(0), |site| {
        admission.group(site as usize).map(|g| g.work)
    })?;
    let mut visit_work = scalar_work;
    for site in [
        generator.header(),
        generator.body(),
        generator.latch(),
        collection.push(),
    ] {
        visit_work = visit_work.checked_add(visits[site as usize].work)?;
    }
    Some(Region {
        collection: id as u32,
        iteration: *iteration,
        initialize: generator.initialize(),
        header: generator.header(),
        body: generator.body(),
        latch: generator.latch(),
        source: source.get(),
        element: element.get(),
        push: collection.push(),
        yielded: yielded.get(),
        input_width,
        output_width,
        sites: sites.into_iter().collect(),
        captures: captures.into_iter().collect(),
        words: words.into_iter().collect(),
        visit_work,
    })
}
