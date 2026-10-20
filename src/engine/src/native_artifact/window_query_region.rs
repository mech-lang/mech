//! Compiler proof of a closed ordinary Mech window/histogram or query loop. This metadata
//! neither executes a loop nor grants a runtime visit/round permission.
use super::{
    NativeCollectionPrimitive as Primitive, NativeCollectionResult, NativeCollectionSource,
    NativeControlLoop, NativeIterationAction as Action, NativeIterationTerminator as End,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeScalarOp as Op,
};

/// Selected canonical operations and complete CFG structure, not source text
/// or a benchmark identity. A backend can use this to emit one compiled region
/// after binding its exact String owners and selector geometry at entry.
/// It must still implement the selected semantics and finite round policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeWindowRegion {
    collection: u32,
    iteration: u32,
    source: Value,
    length: Value,
    width: Value,
    query: Option<Value>,
    sites: Box<[u32]>,
}
impl NativeWindowRegion {
    pub const fn collection(&self) -> u32 {
        self.collection
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn source(&self) -> Value {
        self.source
    }
    pub const fn length(&self) -> Value {
        self.length
    }
    pub const fn width(&self) -> Value {
        self.width
    }
    pub const fn query(&self) -> Option<Value> {
        self.query
    }
    /// Exactly the inner loop sites proved below, including guard-false/latch
    /// paths. No body operation outside this set may be elided by a backend.
    pub fn sites(&self) -> &[u32] {
        &self.sites
    }
}

impl Plan {
    /// Compilation-only selection. An unsupported body remains ordinary CFG;
    /// no alternative parser, evaluator or runtime policy is selected here.
    pub fn window_regions(&self) -> Vec<NativeWindowRegion> {
        self.collections()
            .iter()
            .enumerate()
            .filter_map(|(id, _)| select(self, u32::try_from(id).ok()?))
            .collect()
    }
    pub fn window_query_regions(&self) -> Vec<NativeWindowRegion> {
        self.window_regions()
            .into_iter()
            .filter(|region| region.query.is_some())
            .collect()
    }
    pub fn window_histogram_regions(&self) -> Vec<NativeWindowRegion> {
        self.window_regions()
            .into_iter()
            .filter(|region| region.query.is_none())
            .collect()
    }
}

struct Cursor<'a> {
    plan: &'a Plan,
    iteration: u32,
    visited: Vec<u32>,
}
impl Cursor<'_> {
    fn action(&mut self, mut site: u32) -> Option<(u32, &Action, End)> {
        loop {
            if self.visited.contains(&site) {
                return None;
            }
            let block = self.plan.blocks().get(site as usize)?;
            if block.iteration() != Some(self.iteration) {
                return None;
            }
            self.visited.push(site);
            if let (Action::Scalar(Scalar::None), End::Scalar(Branch::Jump(next))) =
                (block.action(), block.terminator())
            {
                site = next;
                continue;
            }
            return Some((site, block.action(), block.terminator()));
        }
    }
    fn primitive(&mut self, site: u32, wanted: Op) -> Option<(Box<[Value]>, Value, End)> {
        let (
            _,
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
                ..
            }),
            end,
        ) = self.action(site)?
        else {
            return None;
        };
        (*op == wanted).then(|| (operands.clone(), *output, end))
    }
}
fn jump(end: End) -> Option<u32> {
    if let End::Scalar(Branch::Jump(next)) = end {
        Some(next)
    } else {
        None
    }
}
fn guard(end: End, value: Value, latch: u32) -> Option<u32> {
    match end {
        End::Scalar(Branch::Branch { condition, yes, no }) if condition == value && no == latch => {
            Some(yes)
        }
        _ => None,
    }
}
fn constant(plan: &Plan, value: Value, wanted: u64) -> bool {
    plan.constants().iter().any(|entry| entry.value() == value &&
        matches!(entry.constant().data(), mech_core::ValueData::U64(actual) if *actual == wanted))
}
// Prove that every positive, fitting unit range has valid selected geometry.
// Bounded/fixed/compound schemas stay on the ordinary canonical CFG; their
// rejection must not be erased by the whole-loop lowering. This is a static
// classification, not a second shape solver or an unchecked runtime mode.
fn unrestricted_selector(schema: &mech_core::Schema) -> bool {
    use mech_core::{DimensionExpr as D, IntegerWidth, SchemaBody as S};
    let S::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        return false;
    };
    matches!((element.as_ref(), dimensions.as_ref(), schema.dimension_parameters()),
        (S::UnsignedInteger(IntegerWidth::W64), [D::Constant(1), D::Parameter(id)], [parameter])
        if id.get() == 0 && parameter.lower_bound() == &D::Constant(0)
            && parameter.upper_bound().is_none())
}
fn select(plan: &Plan, id: u32) -> Option<NativeWindowRegion> {
    let collection = plan.collections().get(id as usize)?;
    let query = match collection.result() {
        NativeCollectionResult::GuardedHistogramQuery(query) => Some(query.query()),
        NativeCollectionResult::Histogram(_) => None,
        _ => return None,
    };
    let projection = collection.string_span_projection()?;
    if projection.range_template().primitive() != Primitive::RangeInclusive
        || !unrestricted_selector(projection.range_template().output())
        || !unrestricted_selector(projection.selection_template().inputs().get(1)?)
    {
        return None;
    }
    let [iteration] = collection.generators() else {
        return None;
    };
    let NativeControlLoop::Collection(generator) = plan.iterations().get(*iteration as usize)?
    else {
        return None;
    };
    if generator.collection() != id {
        return None;
    }
    let NativeCollectionSource::U64Range(range) = generator.source() else {
        return None;
    };
    if range.template().primitive() != Primitive::RangeInclusive
        || !constant(plan, range.operands()[0], 0)
    {
        return None;
    }
    // The original selected 0..=length preparation still executes before
    // this region and checks the U64 selector's physical Layout for length+1.
    // Every inner inclusive range has 0 < width <= length elements, so the
    // SAME representation bound is already proved; unrestricted_selector
    // also proves its geometry cannot fail. No per-window shape binder needed.
    let length = range.operands()[1];
    let index = generator.element()?;
    // Length must be the SAME selected String's maintained sequence length,
    // not a caller-supplied bound or a same-sized unrelated source.
    if !plan.blocks().iter().any(|block| {
        matches!(block.action(),
        Action::LibraryCall { template, operands, output }
            if *output == length && operands.as_ref() == [projection.source()]
                && template.primitive() == Primitive::SequenceLength
                && block.iteration() != Some(*iteration))
    }) {
        return None;
    }
    let header = plan.blocks().get(generator.header() as usize)?;
    if !matches!((header.action(), header.terminator()),
        (Action::Scalar(Scalar::None), End::Header { iteration: owner, body, exit })
        if owner == *iteration && body == generator.body() && exit == generator.exit())
    {
        return None;
    }
    let latch = plan.blocks().get(generator.latch() as usize)?;
    if !matches!(latch.action(), Action::Latch { iteration: owner } if owner == iteration)
        || jump(latch.terminator()) != Some(generator.header())
    {
        return None;
    }
    let mut cursor = Cursor {
        plan,
        iteration: *iteration,
        visited: vec![generator.header(), generator.latch()],
    };
    let (_, Action::CollectionElement { iteration: owner }, end) =
        cursor.action(generator.body())?
    else {
        return None;
    };
    if owner != iteration {
        return None;
    }
    let next = jump(end)?;
    let (positive, positive_result, end) = cursor.primitive(next, Op::U64Gt)?;
    let [width, zero] = positive.as_ref() else {
        return None;
    };
    let width = *width;
    if !constant(plan, *zero, 0) {
        return None;
    }
    let next = guard(end, positive_result, generator.latch())?;
    let (remaining, remaining_result, end) = cursor.primitive(next, Op::U64Sub)?;
    if remaining.as_ref() != [length, index] {
        return None;
    }
    let next = jump(end)?;
    let (fits, fits_result, end) = cursor.primitive(next, Op::U64Le)?;
    if fits.as_ref() != [width, remaining_result] {
        return None;
    }
    let next = guard(end, fits_result, generator.latch())?;
    let (first, first_result, end) = cursor.primitive(next, Op::U64Add)?;
    let [element, one] = first.as_ref() else {
        return None;
    };
    if *element != index || !constant(plan, *one, 1) {
        return None;
    }
    let next = jump(end)?;
    let (last, last_result, end) = cursor.primitive(next, Op::U64Add)?;
    if last.as_ref() != [index, width] || projection.operands() != &[first_result, last_result] {
        return None;
    }
    let next = jump(end)?;
    let (site, Action::PrepareStringSpanSelector { collection: owner }, end) =
        cursor.action(next)?
    else {
        return None;
    };
    if *owner != id || site != projection.range_site() {
        return None;
    }
    let next = jump(end)?;
    let (site, Action::ProjectStringSpan { collection: owner }, end) = cursor.action(next)? else {
        return None;
    };
    if *owner != id || site != projection.selection_site() {
        return None;
    }
    let next = jump(end)?;
    let (
        site,
        Action::CollectionPush {
            collection: owner,
            value,
        },
        end,
    ) = cursor.action(next)?
    else {
        return None;
    };
    if *owner != id
        || *value != projection.output()
        || site != collection.push()
        || jump(end)? != generator.latch()
    {
        return None;
    }
    // Reject hidden operations, alternate branches, nested loops and extra
    // consumers, even if the recognized positive path itself looked right.
    if plan.blocks().iter().enumerate().any(|(site, block)| {
        block.iteration() == Some(*iteration) && !cursor.visited.contains(&(site as u32))
    }) {
        return None;
    }
    cursor.visited.sort_unstable();
    Some(NativeWindowRegion {
        collection: id,
        iteration: *iteration,
        source: projection.source(),
        length,
        width,
        query,
        sites: cursor.visited.into_boxed_slice(),
    })
}
