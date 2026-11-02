//! A fixed-width key candidate derived from the original checked U64 additions.
//! This proof neither admits storage nor removes endpoint arithmetic. Runtime
//! entry must snapshot both invariant offsets, check the maintained range shape,
//! and decline unsupported widths before selecting a packed representation.
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationValueScope as Scope, NativeMapOwnerWords, NativeMatchAction as Scalar,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan, NativeScalarOp as Op,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Proof {
    // These indices come only from derive or the enclosing unsafe build-owned
    // descriptor reader. No public source/image constructor grants this proof.
    start_offset: Option<u32>,
    end_offset: Option<u32>,
    inclusive: bool,
    range_site: u32,
    selection_site: u32,
    domain: Option<crate::native_artifact::program::runtime::maps::key_domain::Domain>,
}
impl Proof {
    /// None denotes the literal zero in an unmodified base endpoint.
    pub fn start_offset(&self) -> Option<u32> {
        self.start_offset
    }
    pub fn end_offset(&self) -> Option<u32> {
        self.end_offset
    }
    pub fn inclusive(&self) -> bool {
        self.inclusive
    }
    #[cfg(any(test, feature = "canonical-native-program"))]
    pub fn range_site(&self) -> u32 {
        self.range_site
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub fn selection_site(&self) -> u32 {
        self.selection_site
    }
    #[cfg(test)]
    pub fn domain(
        &self,
    ) -> Option<crate::native_artifact::program::runtime::maps::key_domain::Domain> {
        self.domain
    }
    pub fn packing_domain(
        &self,
    ) -> crate::native_artifact::program::runtime::maps::key_domain::Domain {
        self.domain
            .unwrap_or_else(
                crate::native_artifact::program::runtime::maps::key_domain::Domain::ascii,
            )
            .packing()
    }
}

fn u64_value(plan: &Plan, value: Value) -> bool {
    plan.values().get(value.get() as usize).is_some_and(|ty| {
        ty.definition().dimension_parameters().is_empty()
            && ty.definition().body() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    })
}

fn strict_ancestor(plan: &Plan, scope: Option<u32>, iteration: u32) -> bool {
    if scope == Some(iteration) {
        return false;
    }
    let Some(control) = plan.iterations().get(iteration as usize) else {
        return false;
    };
    let mut parent = control.as_ref().parent();
    // Bounds malformed ancestry even though the enclosing plan was validated.
    for _ in 0..=plan.iterations().len() {
        if parent == scope {
            return true;
        }
        let Some(id) = parent else {
            return false;
        };
        let Some(control) = plan.iterations().get(id as usize) else {
            return false;
        };
        if id == iteration {
            return false;
        }
        parent = control.as_ref().parent();
    }
    false
}

fn output(plan: &Plan, action: &Action) -> Option<Value> {
    match action {
        Action::Scalar(Scalar::Primitive { output, .. } | Scalar::Copy { output, .. })
        | Action::LibraryCall { output, .. }
        | Action::Structural { output, .. } => Some(*output),
        Action::CollectionElement { iteration } => {
            match plan.iterations().get(*iteration as usize)? {
                NativeControlLoop::Collection(collection) => collection.element(),
                _ => None,
            }
        }
        _ => None,
    }
}

fn definition(plan: &Plan, value: Value) -> Option<usize> {
    let mut sites = plan
        .blocks()
        .iter()
        .enumerate()
        .filter_map(|(site, block)| (output(plan, block.action()) == Some(value)).then_some(site));
    let site = sites.next()?;
    sites.next().is_none().then_some(site)
}

fn reachable(
    visits: &[NativeIterationVisit],
    entry: u32,
    target: u32,
    omit: Option<usize>,
) -> bool {
    let mut seen = vec![false; visits.len()];
    let mut pending = vec![entry];
    while let Some(site) = pending.pop() {
        let site = site as usize;
        if Some(site) == omit {
            continue;
        }
        let Some(seen) = seen.get_mut(site) else {
            return false;
        };
        if core::mem::replace(seen, true) {
            continue;
        }
        if site == target as usize {
            return true;
        }
        pending.extend(visits[site].successors.into_iter().flatten());
    }
    false
}

fn dominates(
    visits: &[NativeIterationVisit],
    entry: u32,
    definition: usize,
    use_site: u32,
) -> bool {
    definition != use_site as usize
        && reachable(visits, entry, use_site, None)
        && !reachable(visits, entry, use_site, Some(definition))
}

pub(in crate::native_artifact) fn invariant(
    plan: &Plan,
    visits: &[NativeIterationVisit],
    value: Value,
    iteration: u32,
) -> bool {
    if !u64_value(plan, value) {
        return false;
    }
    let Some(Scope::Immutable { iteration: scope }) = plan.value_scopes().get(value.get() as usize)
    else {
        return false;
    };
    if !strict_ancestor(plan, *scope, iteration) {
        return false;
    }
    if plan.inputs().iter().any(|input| input.value() == value)
        || plan
            .constants()
            .iter()
            .any(|constant| constant.value() == value)
    {
        return scope.is_none();
    }
    let Some(site) = definition(plan, value) else {
        return false;
    };
    let Some(control) = plan.iterations().get(iteration as usize) else {
        return false;
    };
    // Lexical ancestry alone is insufficient: the producer must be completed
    // on every path into THIS invocation, not merely belong to an outer scope.
    plan.blocks()[site].iteration() == *scope
        && dominates(visits, plan.entry(), site, control.as_ref().initialize())
}

fn endpoints(
    plan: &Plan,
    visits: &[NativeIterationVisit],
    value: Value,
    iteration: u32,
    range_site: u32,
) -> Vec<(Value, Option<Value>)> {
    if !u64_value(plan, value) {
        return Vec::new();
    }
    let mut candidates = vec![(value, None)];
    let Some(site) = definition(plan, value) else {
        return candidates;
    };
    let Action::Scalar(Scalar::Primitive {
        op: Op::U64Add,
        operands,
        output,
    }) = plan.blocks()[site].action()
    else {
        return candidates;
    };
    if *output != value
        || operands.len() != 2
        || !dominates(visits, plan.entry(), site, range_site)
        || operands.iter().any(|&operand| !u64_value(plan, operand))
    {
        return candidates;
    }
    // U64Add is commutative, but its checked source operation still executes.
    for (base, offset) in [(operands[0], operands[1]), (operands[1], operands[0])] {
        if invariant(plan, visits, offset, iteration) {
            candidates.push((base, Some(offset)));
        }
    }
    candidates
}

/// Derive from the validated same-plan owner/span inventory, never spelling,
/// constant key contents, an expected result, or a benchmark-specific width.
pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    owners: &NativeMapOwnerWords,
    owner: usize,
    visits: &[NativeIterationVisit],
) -> Option<Proof> {
    if visits.len() != plan.blocks().len() {
        return None;
    }
    let region = owners.regions().get(owner)?;
    let mut spans = owners.spans().iter().filter(|span| span.region() == owner);
    let span = spans.next()?;
    if spans.next().is_some() {
        return None;
    }
    let Action::LibraryCall {
        template, operands, ..
    } = plan.blocks().get(span.range_site() as usize)?.action()
    else {
        return None;
    };
    let inclusive = match template.primitive() {
        Primitive::RangeInclusive => true,
        Primitive::RangeExclusive => false,
        _ => return None,
    };
    if operands.as_ref() != span.operands().as_slice() {
        return None;
    }
    let start = endpoints(
        plan,
        visits,
        operands[0],
        region.iteration(),
        span.range_site(),
    );
    let end = endpoints(
        plan,
        visits,
        operands[1],
        region.iteration(),
        span.range_site(),
    );
    for &(base, start_offset) in &start {
        if let Some(&(_, end_offset)) = end.iter().find(|&&(other, _)| other == base) {
            return Some(Proof {
                start_offset: start_offset.map(Value::get),
                end_offset: end_offset.map(Value::get),
                inclusive,
                range_site: span.range_site(),
                selection_site: span.selection_site(),
                domain: crate::native_artifact::program::runtime::maps::key_domain::derive(
                    plan,
                    span.source(),
                ),
            });
        }
    }
    None
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;
