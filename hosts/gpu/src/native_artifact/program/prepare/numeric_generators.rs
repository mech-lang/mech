//! Source-closed matrix-valued generators containing complete native children.
//! This inventory supplies a kernel planner with the original source graph and
//! bounded synchronous components. It grants no loan, admission or publication.
use super::{numeric_binary, numeric_maps, numeric_phase, tail};
use mech_core::{DimensionExpr, SchemaBody};
use mech_engine::native_artifact::{
    NativeBufferFrames, NativeCollectionPrimitive as Primitive,
    NativeCollectionResult as ResultKind, NativeCollectionSource as Source,
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramReadRole as Read, NativeScalarAggregates,
    NativeScalarKind,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_generator_banks.rs"]
pub(in crate::native_artifact) mod banks;
#[path = "numeric_generator_budget.rs"]
pub(in crate::native_artifact) mod funding;

/// Paid at the original header before any optional caller/owner query.
pub(in crate::native_artifact) const HEADER_WORK: u64 = 32768;

#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Child {
    pub region: usize,
    pub trips: numeric_phase::StateTrips,
}
#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Region {
    pub collection: u32,
    pub iteration: u32,
    pub header: u32,
    pub exit: u32,
    pub output: u32,
    pub result: u32,
    pub element: u32,
    pub start: u64,
    pub count: u64,
    pub extents: [u64; 2],
    pub block_extents: [u64; 2],
    /// Per-root traversal order. Summary entries retain their exact source site.
    pub order: Vec<u32>,
    /// Complete source membership, including each child's ordinary source CFG.
    pub sites: Vec<u32>,
    pub children: Vec<Child>,
    pub maps: Vec<numeric_phase::matrix_maps::MatrixMap>,
    /// Ordinary no-op exits between map loop completion and its finish visit.
    pub map_bridges: Vec<Vec<u32>>,
    pub captures: Vec<u32>,
    pub local_values: Vec<u32>,
    /// Conservative source visits, before any ordinary replay credit is added.
    pub maximum_visits: u64,
}
#[derive(Debug, Default, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
    pub declined: Vec<(u32, &'static str)>,
}

pub(in crate::native_artifact) fn matrix(body: &SchemaBody) -> Option<[u64; 2]> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = body
    else {
        return None;
    };
    if NativeScalarKind::from_schema_body(element) != Some(NativeScalarKind::F64) {
        return None;
    }
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = &dimensions[..]
    else {
        return None;
    };
    rows.checked_mul(*columns)?.checked_mul(8)?;
    Some([*rows, *columns])
}
fn numeric(body: &SchemaBody, depth: usize) -> bool {
    if depth == 16 {
        return false;
    }
    if NativeScalarKind::from_schema_body(body).is_some() {
        return true;
    }
    match body {
        SchemaBody::Matrix { .. } => matrix(body).is_some(),
        SchemaBody::Tuple(fields) => fields.iter().all(|b| numeric(b, depth + 1)),
        SchemaBody::Enum { variants, .. } => variants
            .iter()
            .all(|v| v.payload.as_ref().is_none_or(|b| numeric(b, depth + 1))),
        _ => false,
    }
}
fn constant(plan: &Plan, value: Value) -> Option<u64> {
    let c = plan.constants().iter().find(|c| c.value() == value)?;
    match c.constant().data() {
        mech_core::ValueData::U64(n) => Some(*n),
        _ => None,
    }
}
fn read_inside(role: Read, sites: &BTreeSet<u32>, iterations: &BTreeSet<u32>) -> bool {
    match role {
        Read::Action(at) | Read::Terminator(at) => sites.contains(&at),
        Read::CollectionSource(id)
        | Read::LoopSeed(id)
        | Read::LoopNext(id)
        | Read::LoopReturned(id) => iterations.contains(&id),
        Read::Output(_) | Read::Effect(_) => false,
    }
}

impl Layout {
    pub fn derive(
        plan: &Plan,
        frames: &NativeBufferFrames,
        aggregates: &NativeScalarAggregates,
        tails: &tail::Layout,
    ) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let maps = numeric_phase::matrix_maps::inventory(plan)?;
        let binary = numeric_binary::Layout::derive(plan)?;
        let mut result = Self::default();
        for (id, definition) in plan.collections().iter().enumerate() {
            if !matches!(
                definition.kind(),
                mech_engine::ComprehensionKind::MatrixHorizontalBlocks { .. }
            ) {
                continue;
            }
            match candidate(plan, frames, aggregates, tails, &maps, &binary, id as u32) {
                Ok(region) => result.regions.push(region),
                Err(reason) => result.declined.push((id as u32, reason)),
            }
        }
        Ok(result)
    }
}

fn candidate(
    plan: &Plan,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    tails: &tail::Layout,
    maps: &[numeric_phase::matrix_maps::MatrixMap],
    binary: &numeric_binary::Layout,
    id: u32,
) -> Result<Region, &'static str> {
    let facts = plan.facts();
    let definition = &plan.collections()[id as usize];
    if definition.generators().len() != 1
        || !matches!(definition.result(), ResultKind::Materialized)
    {
        return Err("numeric generator requires one materialized source cursor");
    }
    let iteration = definition.generators()[0];
    let Loop::Collection(root) = &plan.iterations()[iteration as usize] else {
        return Err("numeric generator lacks its canonical collection loop");
    };
    let element = root
        .element()
        .ok_or("numeric generator does not bind its cursor element")?;
    if root.collection() != id
        || root.parent() != definition.parent()
        || plan.values()[element.get() as usize].scalar_kind() != Some(NativeScalarKind::U64)
        || !facts.is_reachable(root.header())?
    {
        return Err("numeric generator has mismatched source ownership");
    }
    if !matches!(
        plan.blocks()[root.header() as usize].action(),
        Action::Scalar(Scalar::None)
    ) || plan.blocks()[root.header() as usize].terminator()
        != (Term::Header {
            iteration,
            body: root.body(),
            exit: root.exit(),
        })
        || !matches!(plan.blocks()[root.latch() as usize].action(), Action::Latch { iteration: owner } if *owner == iteration)
        || plan.blocks()[root.latch() as usize].terminator()
            != Term::Scalar(Branch::Jump(root.header()))
    {
        return Err("numeric generator has an invalid source header or backedge");
    }
    let Source::U64Range(cursor) = root.source() else {
        return Err("numeric generator requires a fixed U64 source cursor");
    };
    let [first, last] = cursor.operands();
    let start = constant(plan, *first).ok_or("numeric generator source start is not fixed")?;
    let end = constant(plan, *last).ok_or("numeric generator source end is not fixed")?;
    let inclusive = match cursor.template().primitive() {
        Primitive::RangeInclusive => true,
        Primitive::RangeExclusive => false,
        _ => return Err("numeric generator lacks its exact range factory"),
    };
    let count = end
        .checked_sub(start)
        .and_then(|n| n.checked_add(u64::from(inclusive)))
        .filter(|&n| n != 0)
        .ok_or("numeric generator source is empty or overflowed")?;
    let Action::CollectionPush { collection, value } =
        plan.blocks()[definition.push() as usize].action()
    else {
        return Err("numeric generator lacks its exact yielded block");
    };
    if *collection != id || !facts.strictly_dominates(definition.push(), root.latch())? {
        return Err("numeric generator yield does not cover every source binding");
    }
    let block_extents = matrix(plan.values()[value.get() as usize].definition().body())
        .ok_or("numeric generator yield is not a fixed F64 matrix")?;
    let extents = [
        block_extents[0],
        block_extents[1]
            .checked_mul(count)
            .ok_or("numeric generator output columns overflow")?,
    ];
    if matrix(
        plan.values()[definition.output().get() as usize]
            .definition()
            .body(),
    ) != Some(extents)
    {
        return Err("numeric generator output geometry is not the exact block concatenation");
    }
    match definition.kind() {
        mech_engine::ComprehensionKind::MatrixHorizontalBlocks {
            rows: mech_engine::MatrixBlockRows::Fixed(rows),
        } if rows == block_extents[0] => {}
        mech_engine::ComprehensionKind::MatrixHorizontalBlocks {
            rows: mech_engine::MatrixBlockRows::InferOrZero,
        } => {}
        _ => return Err("numeric generator row witness is not fixed by its blocks"),
    }
    let mut children = Vec::new();
    let mut child_maps = Vec::new();
    let mut iterations = BTreeSet::from([iteration]);
    let mut elided = BTreeSet::from([cursor.semantic_output().get()]);
    for (child_id, control) in plan.iterations().iter().enumerate() {
        if control.as_ref().parent() != Some(iteration) {
            continue;
        }
        match control {
            Loop::Tail(child) => {
                let (region, selected) = tails
                    .regions
                    .iter()
                    .enumerate()
                    .find(|(_, r)| r.iteration == child_id as u32)
                    .ok_or("numeric generator child has no selected native region")?;
                let proof = selected
                    .numeric
                    .as_ref()
                    .ok_or("numeric generator child has no numeric proof")?;
                if !proof.exits.is_empty()
                    || proof.result.direct_array().is_none()
                    || !proof.map_banks.is_empty()
                    || proof.arrays.iter().any(|a| {
                        a.appends
                            || a.width != 8
                            || plan
                                .values()
                                .iter()
                                .find(|v| v.schema() == a.schema)
                                .is_none_or(|v| matrix(v.definition().body()).is_none())
                    })
                {
                    return Err("numeric generator child is not a complete fixed matrix phase");
                }
                let trips = numeric_phase::fixed_state_trips(&facts, frames, aggregates, child)?;
                children.push(Child { region, trips });
            }
            Loop::Collection(child) => {
                let map = maps
                    .iter()
                    .find(|m| m.region.iteration == child_id as u32)
                    .ok_or("numeric generator contains an unsupported child collection")?;
                if !map.region.nested.is_empty()
                    || map.region.rectangle.is_some()
                    || map.region.output_width != 8
                    || plan.values()[map.region.result as usize].scalar_kind()
                        != Some(NativeScalarKind::F64)
                    || matrix(plan.values()[map.output as usize].definition().body())
                        != Some(map.extents)
                {
                    return Err("numeric generator child map is not a fixed scalar F64 map");
                }
                elided.insert(child.source().semantic_value().get());
                child_maps.push(map.clone());
            }
            _ => return Err("numeric generator has an unsupported child loop"),
        }
        iterations.insert(child_id as u32);
    }
    if children.is_empty() {
        return Err("numeric generator has no complete numeric FSM children");
    }
    let summary_after = |site| {
        if let Some(map) = child_maps.iter().find(|m| m.entry == site) {
            return Some(map.after);
        }
        children.iter().find_map(|child| {
            let tail = &tails.regions[child.region].tail;
            (tail.initialize() == site).then_some(tail.exit())
        })
    };
    let mut pending = vec![root.header()];
    let mut edges = BTreeMap::new();
    while let Some(site) = pending.pop() {
        if site == root.exit() || edges.contains_key(&site) {
            continue;
        }
        let block = &plan.blocks()[site as usize];
        if block.iteration() != Some(iteration) || plan.activation_boundary() == Some(site) {
            return Err("numeric generator crosses an external iteration or activation");
        }
        let next = if site == root.latch() {
            Vec::new()
        } else if let Some(after) = summary_after(site) {
            vec![after]
        } else {
            if (site != root.header()
                && !matches!(
                    block.terminator(),
                    Term::Scalar(
                        Branch::Jump(_)
                            | Branch::Branch { .. }
                            | Branch::LiteralBranch { .. }
                            | Branch::Reject
                            | Branch::IntegrityFailure
                    )
                ))
                || !supported(plan, frames, aggregates, binary, site, iteration, id)
            {
                return Err("numeric generator has an unsupported parent operation");
            }
            facts
                .successors(site)?
                .into_iter()
                .flatten()
                .filter(|&s| s != root.exit())
                .collect()
        };
        pending.extend(&next);
        edges.insert(site, next);
    }
    if !edges.contains_key(&root.latch()) || !edges.contains_key(&definition.push()) {
        return Err("numeric generator source traversal is incomplete");
    }
    let mut degrees = edges
        .keys()
        .map(|&s| (s, 0usize))
        .collect::<BTreeMap<_, _>>();
    for next in edges.values() {
        for s in next {
            *degrees
                .get_mut(s)
                .ok_or("numeric generator has an external source edge")? += 1;
        }
    }
    let mut ready = degrees
        .iter()
        .filter_map(|(&s, &n)| (n == 0).then_some(s))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    while let Some(site) = ready.pop_first() {
        order.push(site);
        for s in &edges[&site] {
            let n = degrees.get_mut(s).unwrap();
            *n -= 1;
            if *n == 0 {
                ready.insert(*s);
            }
        }
    }
    if order.len() != edges.len() || order.first() != Some(&root.header()) {
        return Err("numeric generator has an undeclared source cycle");
    }
    let mut sites = order.iter().copied().collect::<BTreeSet<_>>();
    let mut visits = order.len() as u64;
    let mut map_bridges = Vec::new();
    for map in &child_maps {
        let mut bridge = Vec::new();
        let mut at = map.region.exit;
        let finish = *map
            .completion
            .first()
            .ok_or("numeric map has no completion visit")?;
        while at != finish {
            if bridge.len() == 32 || bridge.contains(&at) || plan.activation_boundary() == Some(at)
            {
                return Err("numeric map exit bridge is cyclic or crosses an activation");
            }
            let block = &plan.blocks()[at as usize];
            if block.iteration() != Some(iteration)
                || !matches!(block.action(), Action::Scalar(Scalar::None))
            {
                return Err("numeric map exit bridge contains an unsupported source action");
            }
            let Term::Scalar(Branch::Jump(next)) = block.terminator() else {
                return Err("numeric map exit bridge is not a synchronous continuation");
            };
            bridge.push(at);
            at = next;
        }
        sites.extend(&bridge);
        visits = visits
            .checked_add(bridge.len() as u64)
            .ok_or("numeric map bridge visits overflow")?;
        map_bridges.push(bridge);
        if !sites.contains(&map.entry) {
            return Err("numeric generator child map is disconnected");
        }
        sites.extend(map.source_sites());
        visits = visits
            .checked_add(
                (map.region.sites.len() as u64)
                    .checked_mul(
                        map.count
                            .checked_add(1)
                            .ok_or("numeric generator map count overflows")?,
                    )
                    .ok_or("numeric generator map visit bound overflows")?,
            )
            .and_then(|n| n.checked_add((map.prefix.len() + map.completion.len()) as u64))
            .ok_or("numeric generator map visit total overflows")?;
    }
    for child in &children {
        let selected = &tails.regions[child.region];
        if !sites.contains(&selected.tail.initialize()) {
            return Err("numeric generator child FSM is disconnected");
        }
        let proof = selected.numeric.as_ref().unwrap();
        sites.extend(proof.sites.iter().map(|&s| s as u32));
        visits = visits
            .checked_add(
                (proof.sites.len() as u64)
                    .checked_mul(child.trips.maximum)
                    .ok_or("numeric generator FSM visit bound overflows")?,
            )
            .ok_or("numeric generator FSM visit total overflows")?;
    }
    for &site in &sites {
        if plan.activation_boundary() == Some(site) {
            return Err("numeric generator contains an activation");
        }
        for &pred in facts.predecessors(site)? {
            if !sites.contains(&pred) && !(site == root.header() && pred == root.initialize()) {
                return Err("numeric generator has an external entry into its body");
            }
        }
        for next in facts.successors(site)?.into_iter().flatten() {
            if !sites.contains(&next) && next != root.exit() {
                return Err("numeric generator has an external continuation inside its body");
            }
        }
    }
    // Keep real same-plan Value identities. Ordinals are descriptive indices,
    // and are never decoded or forged into constructor-private source handles.
    // The first coordinate in direct_action_outputs is a VALUE identity,
    // not a source site. Membership below uses actual definition/read roles.
    let mut values = facts
        .direct_action_outputs()
        .iter()
        .map(|(_, value)| (value.get(), *value))
        .chain(
            plan.inputs()
                .iter()
                .map(|input| (input.value().get(), input.value())),
        )
        .chain(
            plan.constants()
                .iter()
                .map(|c| (c.value().get(), c.value())),
        )
        .collect::<BTreeMap<_, _>>();
    for &site in &sites {
        facts.parallel_action_reads(site, &mut |value| {
            values.insert(value.get(), value);
            Ok(())
        })?;
        match plan.blocks()[site as usize].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => {
                values.insert(condition.get(), condition);
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                values.insert(left.get(), left);
                values.insert(right.get(), right);
            }
            _ => {}
        }
        let produced = match plan.blocks()[site as usize].action() {
            Action::CollectionBegin { collection } | Action::CollectionFinish { collection } => {
                Some(plan.collections()[*collection as usize].output())
            }
            Action::CollectionElement { iteration } => {
                match &plan.iterations()[*iteration as usize] {
                    Loop::Collection(g) => g.element(),
                    _ => None,
                }
            }
            Action::Initialize { iteration } | Action::Latch { iteration } => {
                match &plan.iterations()[*iteration as usize] {
                    Loop::Tail(t) => Some(t.carrier()),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(value) = produced {
            values.insert(value.get(), value);
        }
    }
    let mut captures = Vec::new();
    let mut local_values = Vec::new();
    for (&value, &identity) in &values {
        let slot = &plan.values()[value as usize];
        if elided.contains(&value) {
            continue;
        }
        let reads = facts.read_roles(identity)?;
        let definitions = facts.definition_sites(value)?;
        let inside = definitions
            .iter()
            .any(|d| d.site().is_some_and(|s| sites.contains(&s)));
        if inside {
            if definitions
                .iter()
                .any(|d| d.site().is_none_or(|s| !sites.contains(&s)))
                || reads.iter().any(|&r| !read_inside(r, &sites, &iterations))
            {
                return Err("numeric generator local value escapes its source region");
            }
            if !numeric(slot.definition().body(), 0) {
                return Err("numeric generator local value has an unsupported type");
            }
            local_values.push(value);
        } else if reads.iter().any(|&r| read_inside(r, &sites, &iterations)) {
            if plan.constants().iter().any(|c| c.value().get() == value) {
                continue;
            }
            if (slot.scalar_kind().is_none() && matrix(slot.definition().body()).is_none())
                || !numeric_maps::immutable_before(&facts, identity, root.header(), root.parent())?
            {
                return Err("numeric generator capture is not an immutable scalar or fixed matrix");
            }
            captures.push(value);
        }
    }
    let maximum_visits = visits
        .checked_mul(
            count
                .checked_add(1)
                .ok_or("numeric generator root visits overflow")?,
        )
        .ok_or("numeric generator total visit bound overflows")?;
    Ok(Region {
        collection: id,
        iteration,
        header: root.header(),
        exit: root.exit(),
        output: definition.output().get(),
        result: value.get(),
        element: element.get(),
        start,
        count,
        extents,
        block_extents,
        order,
        sites: sites.into_iter().collect(),
        children,
        maps: child_maps,
        map_bridges,
        captures,
        local_values,
        maximum_visits,
    })
}

fn supported(
    plan: &Plan,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    binary: &numeric_binary::Layout,
    site: u32,
    iteration: u32,
    collection: u32,
) -> bool {
    if frames.action(site as usize).is_some() || aggregates.action(site as usize).is_some() {
        return true;
    }
    match plan.blocks()[site as usize].action() {
        Action::Scalar(Scalar::None | Scalar::Primitive { .. }) => true,
        Action::Scalar(Scalar::Copy { source, output }) => {
            plan.values()[source.get() as usize].definition()
                == plan.values()[output.get() as usize].definition()
                && numeric(plan.values()[output.get() as usize].definition().body(), 0)
        }
        Action::CollectionElement { iteration: owner } => *owner == iteration,
        Action::CollectionPush {
            collection: owner, ..
        } => *owner == collection,
        Action::LibraryCall {
            template,
            operands,
            output,
        } => {
            if let Some(op) = binary.at(site as usize)
                && !matches!(op.kind, numeric_binary::Kind::Transpose { .. })
            {
                return matches!(op.kind, numeric_binary::Kind::Elementwise(_));
            }
            template.primitive() == Primitive::MatrixTranspose
                && operands.len() == 1
                && matrix(
                    plan.values()[operands[0].get() as usize]
                        .definition()
                        .body(),
                )
                .is_some()
                && matrix(plan.values()[output.get() as usize].definition().body()).is_some()
        }
        _ => false,
    }
}
