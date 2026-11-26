//! One native scalar body for a rectangular nest of canonical collections.
//! The inner row and its transpose have exactly one consumer each. Neither
//! can escape, so their scalar bank can fill the enclosing block builder.
use super::*;
use mech_engine::native_artifact::NativeU64RangeCursor;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Rectangle {
    pub inner_collection: u32,
    pub inner_iteration: u32,
    pub inner_element: u32,
    pub range_prepare: u32,
    pub transpose: u32,
    pub start: u64,
    pub end: u64,
    /// A provisional bound, certified by the selected range factory per loan.
    pub rows: u64,
    pub elided_values: Vec<u32>,
}
fn constant(plan: &Plan, value: Value) -> Option<u64> {
    let entry = plan.constants().iter().find(|c| c.value() == value)?;
    match entry.constant().data() {
        mech_core::ValueData::U64(n) => Some(*n),
        _ => None,
    }
}
fn range(plan: &Plan, cursor: &NativeU64RangeCursor) -> Option<(u64, u64, u64)> {
    let [first, last] = cursor.operands();
    let start = constant(plan, *first)?;
    let end = constant(plan, *last)?;
    let difference = end.checked_sub(start)?;
    let rows = match cursor.template().primitive() {
        Primitive::RangeExclusive => difference,
        Primitive::RangeInclusive => difference.checked_add(1)?,
        _ => return None,
    };
    (rows != 0).then_some((start, end, rows))
}
fn segment(plan: &Plan, from: u32, until: u32, iteration: u32) -> Option<Vec<u32>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut at = from;
    while at != until {
        if !seen.insert(at) || plan.activation_boundary() == Some(at) {
            return None;
        }
        let block = plan.blocks().get(at as usize)?;
        if block.iteration() != Some(iteration) {
            return None;
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        out.push(at);
        at = next;
    }
    Some(out)
}
fn only_read(facts: &Facts<'_>, value: Value, at: u32) -> Option<()> {
    let roles = facts.read_roles(value).ok()?;
    (roles == [Read::Action(at)]).then_some(())
}
fn candidate(plan: &Plan, facts: &Facts<'_>, layout: &Layout, id: usize) -> Option<Region> {
    let outer = plan.collections().get(id)?;
    if !matches!(
        outer.kind(),
        mech_engine::ComprehensionKind::MatrixHorizontalBlocks { .. }
    ) || !matches!(outer.result(), ResultKind::Materialized)
        || outer.generators().len() != 1
    {
        return None;
    }
    let iteration = outer.generators()[0];
    let Loop::Collection(generator) = plan.iterations().get(iteration as usize)? else {
        return None;
    };
    let element = generator.element()?;
    if generator.collection() != id as u32
        || generator.parent() != outer.parent()
        || plan.values()[element.get() as usize].scalar_kind() != Some(NativeScalarKind::U64)
        || !matches!(
            plan.blocks()[generator.header() as usize].action(),
            Action::Scalar(Scalar::None)
        )
        || plan.blocks()[generator.header() as usize].terminator()
            != (Terminator::Header {
                iteration,
                body: generator.body(),
                exit: generator.exit(),
            })
        || !matches!(plan.blocks()[generator.latch() as usize].action(),Action::Latch{iteration:owner} if *owner==iteration)
        || plan.blocks()[generator.latch() as usize].terminator()
            != Terminator::Scalar(Term::Jump(generator.header()))
    {
        return None;
    }
    let (source, source_is_range) = match generator.source() {
        Source::Value(value) => {
            let SchemaBody::Matrix {
                element: kind,
                dimensions,
            } = plan.values()[value.get() as usize].definition().body()
            else {
                return None;
            };
            if dimensions.len() != 2
                || NativeScalarKind::from_schema_body(kind) != Some(NativeScalarKind::U64)
                || !immutable_before(facts, *value, generator.initialize(), generator.parent())
                    .ok()?
            {
                return None;
            }
            (*value, false)
        }
        Source::U64Range(cursor) => (cursor.semantic_output(), true),
    };
    let mut children = layout.regions.iter().filter(|r| {
        r.rectangle.is_none()
            && plan.iterations()[r.iteration as usize].as_ref().parent() == Some(iteration)
            && plan.collections()[r.collection as usize].parent_collection() == Some(id as u32)
    });
    let inner = children.next()?;
    if children.next().is_some() {
        return None;
    }
    let inner_definition = &plan.collections()[inner.collection as usize];
    let Loop::Collection(inner_generator) = &plan.iterations()[inner.iteration as usize] else {
        return None;
    };
    let Source::U64Range(cursor) = inner_generator.source() else {
        return None;
    };
    let (start, end, rows) = range(plan, cursor)?;
    let prefix = segment(plan, generator.body(), inner_generator.header(), iteration)?;
    let mut saw_element = false;
    let mut saw_range = false;
    let mut saw_begin = false;
    let mut saw_initialize = false;
    for &site in &prefix {
        match plan.blocks()[site as usize].action() {
            Action::Scalar(Scalar::None) => {}
            Action::CollectionElement { iteration: owner }
                if *owner == iteration && !saw_element && site == generator.body() =>
            {
                saw_element = true
            }
            Action::PrepareU64Range { iteration: owner }
                if *owner == inner.iteration && !saw_range && site == cursor.prepare() =>
            {
                saw_range = true
            }
            Action::CollectionBegin { collection }
                if *collection == inner.collection
                    && !saw_begin
                    && site == inner_definition.begin() =>
            {
                saw_begin = true
            }
            Action::Initialize { iteration: owner }
                if *owner == inner.iteration
                    && !saw_initialize
                    && site == inner_generator.initialize() =>
            {
                saw_initialize = true
            }
            _ => return None,
        }
    }
    if !(saw_element && saw_range && saw_begin && saw_initialize) {
        return None;
    }
    let suffix = segment(plan, inner_generator.exit(), generator.latch(), iteration)?;
    let mut finished = false;
    let mut transpose = None;
    let mut pushed = false;
    for &site in &suffix {
        match plan.blocks()[site as usize].action() {
            Action::Scalar(Scalar::None) => {}
            Action::CollectionFinish { collection }
                if *collection == inner.collection
                    && !finished
                    && site == inner_definition.finish() =>
            {
                finished = true
            }
            Action::LibraryCall {
                template,
                operands,
                output,
            } if finished
                && transpose.is_none()
                && template.primitive() == Primitive::MatrixTranspose
                && operands.as_ref() == [inner_definition.output()] =>
            {
                only_read(facts, inner_definition.output(), site)?;
                transpose = Some((site, *output));
            }
            Action::CollectionPush { collection, value }
                if *collection == id as u32
                    && !pushed
                    && site == outer.push()
                    && transpose.is_some_and(|(_, output)| output == *value) =>
            {
                only_read(facts, *value, site)?;
                pushed = true;
            }
            _ => return None,
        }
    }
    let (transpose, transposed) = transpose?;
    if !finished || !pushed {
        return None;
    }
    let kind = |v: Value| match plan.values()[v.get() as usize].definition().body() {
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 => NativeScalarKind::from_schema_body(element),
        _ => None,
    };
    if kind(outer.output()).is_none()
        || kind(outer.output()) != kind(inner_definition.output())
        || kind(outer.output()) != kind(transposed)
    {
        return None;
    }
    let mut sites: BTreeSet<u32> = inner.sites.iter().copied().collect();
    sites.extend(prefix);
    sites.extend(suffix);
    sites.insert(generator.header());
    sites.insert(generator.latch());
    if plan.blocks().iter().enumerate().any(|(at, b)| {
        matches!(b.iteration(),Some(n) if n==iteration||n==inner.iteration)
            && !sites.contains(&(at as u32))
            && facts.is_reachable(at as u32).unwrap_or(true)
    }) {
        return None;
    }
    // The only newly live local is the outer element. All other captures must
    // already be immutable before this entire synchronous rectangular loan.
    if facts
        .read_roles(element)
        .ok()?
        .iter()
        .any(|r| !matches!(r,Read::Action(at)|Read::Terminator(at) if inner.sites.contains(at)))
    {
        return None;
    }
    for &value in &inner.captures {
        if value != element.get()
            && !immutable_before(
                facts,
                inner
                    .body
                    .iter()
                    .flat_map(|&site| nested::reads(plan, site, &inner.nested))
                    .find(|v| v.get() == value)?,
                generator.initialize(),
                generator.parent(),
            )
            .ok()?
        {
            return None;
        }
    }
    for matrix in &inner.matrices {
        if !immutable_before(
            facts,
            match plan.blocks()[matrix.witness as usize].action() {
                Action::LibraryCall { operands, .. } => operands[0],
                _ => return None,
            },
            generator.initialize(),
            generator.parent(),
        )
        .ok()?
        {
            return None;
        }
    }
    let mut region = inner.clone();
    region.collection = id as u32;
    region.iteration = iteration;
    region.header = generator.header();
    region.exit = generator.exit();
    region.source = source.get();
    region.range = source_is_range;
    region.element = element.get();
    region.output = outer.output().get();
    region.input_width = 8;
    region.captures.retain(|&v| v != element.get());
    region.sites = sites.into_iter().collect();
    region.rectangle = Some(Rectangle {
        inner_collection: inner.collection,
        inner_iteration: inner.iteration,
        inner_element: inner.element,
        range_prepare: cursor.prepare(),
        transpose,
        start,
        end,
        rows,
        elided_values: vec![
            inner.output,
            transposed.get(),
            cursor.semantic_output().get(),
        ],
    });
    Some(region)
}
pub(super) fn extend(plan: &Plan, layout: &mut Layout) -> Result<(), &'static str> {
    let facts = plan.facts();
    let regions = (0..plan.collections().len())
        .filter_map(|id| candidate(plan, &facts, layout, id))
        .collect::<Vec<_>>();
    for region in regions {
        if layout.regions.len() == 256 {
            break;
        }
        if layout.headers.is_empty() {
            layout.headers.resize(plan.blocks().len(), None);
        }
        if layout.headers[region.header as usize].is_some() {
            return Err("rectangular header overlaps scalar map");
        }
        layout.headers[region.header as usize] = Some(layout.regions.len());
        layout.regions.push(region);
    }
    Ok(())
}
