//! Closed private map-state candidates. No runtime representation is selected
//! here. Entry must COPY the canonical seed into a private owner, and exit
//! must publish an independent canonical result before any external read.
use super::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationValueScope as Scope, NativeMapWriteUses, NativeMatchAction as Scalar,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation as Observation, NativeTailLoop,
};
use mech_core::{IntegerWidth, SchemaBody as Schema};

#[derive(Clone, Debug)]
pub struct NativeMapOwnerRegion {
    iteration: u32,
    tail: NativeTailLoop,
    values: Box<[Value]>,
    writes: Box<[u32]>,
    reads: Box<[u32]>,
}
impl NativeMapOwnerRegion {
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn tail(&self) -> &NativeTailLoop {
        &self.tail
    }
    /// Private region-local map-bearing values. The seed and canonical result
    /// are excluded; no outside alias may be turned into mutable storage.
    pub fn values(&self) -> &[Value] {
        &self.values
    }
    pub fn writes(&self) -> &[u32] {
        &self.writes
    }
    pub fn reads(&self) -> &[u32] {
        &self.reads
    }
}

fn map(schema: &Schema) -> bool {
    matches!(schema, Schema::Map { key, value, .. }
        if key.as_ref() == &Schema::String
        && value.as_ref() == &Schema::UnsignedInteger(IntegerWidth::W64))
}
fn u64_(schema: &Schema) -> bool {
    *schema == Schema::UnsignedInteger(IntegerWidth::W64)
}
// The initial mixed-state family extends the existing fixed U64 state layout:
// one map per active arm, with up to 31 scalar U64 fields. Record labels,
// multiple independent maps and nested variable-size payloads need their own
// owner/layout proof and keep the canonical route until that exists.
fn payload(schema: &Schema) -> bool {
    map(schema)
        || matches!(schema, Schema::Tuple(items)
        if !items.is_empty() && items.len() <= 32
        && items.iter().filter(|item| map(item)).count() == 1
        && items.iter().all(|item| map(item) || u64_(item)))
}
fn shape(schema: &Schema) -> bool {
    payload(schema)
        || matches!(schema, Schema::Enum { variants, .. }
        if !variants.is_empty() && variants.len() <= 32
        && variants.iter().all(|v| v.payload.as_ref().is_some_and(payload)))
}
fn owns_iteration(plan: &Plan, value: Value, iteration: u32) -> bool {
    matches!(plan.value_scopes().get(value.get() as usize),
        Some(Scope::Carrier { iteration: owner }
            | Scope::Immutable { iteration: Some(owner) }) if *owner == iteration)
}
fn output(action: &Action) -> Option<Value> {
    match action {
        Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
        | Action::LibraryCall { output, .. }
        | Action::Structural { output, .. } => Some(*output),
        _ => None,
    }
}
fn pack(action: &Action, known: &[bool]) -> bool {
    let Action::LibraryCall {
        template, operands, ..
    } = action
    else {
        return false;
    };
    match template.primitive() {
        Primitive::CompositePack(arity) => {
            arity == operands.len()
                && operands.iter().filter(|v| known[v.get() as usize]).count() == 1
        }
        Primitive::EnumPack => operands.len() == 2 && known[operands[1].get() as usize],
        _ => false,
    }
}
impl Plan {
    /// Derive closed candidates from selected semantic operations and complete
    /// read inventory. This grants neither mutation nor native call authority.
    pub fn map_owner_regions(&self) -> Result<Box<[NativeMapOwnerRegion]>, &'static str> {
        self.validate_current_visits()?;
        let facts = self.map_write_uses()?;
        let actions = super::fusion_semantics::semantic_actions(self)?;
        let uses = super::fusion_semantics::reads(self, &actions)?;
        let mut regions = Vec::new();
        for (iteration, loop_) in self.iterations().iter().enumerate() {
            let NativeControlLoop::Tail(tail) = loop_ else {
                continue;
            };
            let iteration = iteration as u32;
            let Some(region) = select(self, iteration, tail, &actions, &uses, &facts) else {
                continue;
            };
            regions.push(region);
        }
        Ok(regions.into())
    }
}

fn select(
    plan: &Plan,
    iteration: u32,
    tail: &NativeTailLoop,
    actions: &super::fusion_semantics::SemanticActions<'_>,
    uses: &[Vec<super::fusion_semantics::Read>],
    facts: &[NativeMapWriteUses],
) -> Option<NativeMapOwnerRegion> {
    let eligible = |value: Value| {
        let ty = &plan.values()[value.get() as usize];
        owns_iteration(plan, value, iteration)
            && ty.definition().dimension_parameters().is_empty()
            && shape(ty.definition().body())
    };
    if !eligible(tail.carrier())
        || !map(plan.values()[tail.returned().get() as usize]
            .definition()
            .body())
        || !shape(
            plan.values()[tail.seed().get() as usize]
                .definition()
                .body(),
        )
        || tail.seed() == tail.carrier()
        || tail.result() == tail.returned()
        || plan
            .activation_boundary()
            .is_some_and(|site| plan.blocks()[site as usize].iteration() == Some(iteration))
    {
        return None;
    }
    let mut known = vec![false; plan.values().len()];
    known[tail.carrier().get() as usize] = true;
    loop {
        let mut changed = false;
        for (site, action) in actions.iter().enumerate() {
            let Some(out) = output(action) else {
                continue;
            };
            if plan.blocks()[site].iteration() != Some(iteration)
                || known[out.get() as usize]
                || !eligible(out)
            {
                continue;
            }
            let derived = match action {
                Action::Scalar(Scalar::Copy { source, .. })
                | Action::Structural {
                    source,
                    observation: Observation::CanonicalValue,
                    ..
                } => known[source.get() as usize],
                Action::LibraryCall {
                    template, operands, ..
                } if template.primitive() == Primitive::MapInsert && operands.len() == 3 => {
                    known[operands[0].get() as usize]
                }
                _ => pack(action, &known),
            };
            if derived {
                known[out.get() as usize] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if !known[tail.next().get() as usize] || !known[tail.returned().get() as usize] {
        return None;
    }
    // No alias to a private carrier may become an input/seed of another loop,
    // a collection source or a transport/effect result. The sole exit export
    // below is the original Tail's canonical result copy.
    for index in 0..plan.transport_output_count() {
        if known[plan.transport_output_value(index).get() as usize] {
            return None;
        }
    }
    for (other, loop_) in plan.iterations().iter().enumerate() {
        if other == iteration as usize {
            continue;
        }
        let escaped = match loop_ {
            NativeControlLoop::Collection(g) => known[g.source().semantic_value().get() as usize],
            NativeControlLoop::Iteration(g) => {
                known[g.seed().get() as usize] || known[g.next().get() as usize]
            }
            NativeControlLoop::Tail(g) => {
                known[g.seed().get() as usize]
                    || known[g.next().get() as usize]
                    || known[g.returned().get() as usize]
            }
        };
        if escaped {
            return None;
        }
    }
    let mut sites = vec![false; plan.blocks().len()];
    for (index, owned) in known.iter().enumerate() {
        if !owned {
            continue;
        }
        for usage in &uses[index] {
            use super::fusion_semantics::Read;
            match usage {
                Read::Action(site) => sites[*site as usize] = true,
                Read::Terminator(_) => return None,
                Read::Other
                    if index == tail.next().get() as usize
                        || index == tail.returned().get() as usize => {}
                Read::Other => return None,
            }
        }
    }
    let mut writes = Vec::new();
    let mut reads = Vec::new();
    for (site, action) in actions.iter().enumerate() {
        if !sites[site] && !output(action).is_some_and(|v| known[v.get() as usize]) {
            continue;
        }
        if site == tail.exit() as usize
            && matches!(action,
            Action::Scalar(Scalar::Copy { source, output })
                if *source == tail.returned() && *output == tail.result())
        {
            continue;
        }
        if plan.blocks()[site].iteration() != Some(iteration) {
            return None;
        }
        match action {
            Action::Scalar(Scalar::Copy { source, output })
                if known[source.get() as usize] && known[output.get() as usize] => {}
            Action::Structural {
                source,
                observation,
                output,
                ..
            } if known[source.get() as usize] => match observation {
                Observation::U64 | Observation::EnumOrdinal { .. } => {}
                Observation::CanonicalValue if known[output.get() as usize] => {}
                _ => return None,
            },
            Action::LibraryCall {
                template,
                operands,
                output,
            } => {
                match template.primitive() {
                    Primitive::MapInsert
                        if operands.len() == 3
                            && known[operands[0].get() as usize]
                            && known[output.get() as usize] =>
                    {
                        let fact = facts.iter().find(|fact| fact.site() as usize == site)?;
                        if fact.iteration() != iteration || !fact.late_reads().is_empty() {
                            return None;
                        }
                        writes.push(site as u32);
                    }
                    Primitive::MapKeyAccess | Primitive::MapContainsKey
                        if operands.len() == 2
                            && known[operands[0].get() as usize]
                            && map(plan.values()[operands[0].get() as usize]
                                .definition()
                                .body()) =>
                    {
                        reads.push(site as u32);
                    }
                    Primitive::CompositePack(_) | Primitive::EnumPack
                        if known[output.get() as usize] && pack(action, &known) =>
                    {
                        // A phi/constructor may not smuggle an unrelated map
                        // into a value whose other definition follows the owner.
                        for operand in operands {
                            let body = plan.values()[operand.get() as usize].definition().body();
                            if shape(body) && !known[operand.get() as usize] {
                                return None;
                            }
                        }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
    if writes.is_empty() {
        return None;
    }
    Some(NativeMapOwnerRegion {
        iteration,
        tail: tail.clone(),
        values: known
            .iter()
            .enumerate()
            .filter_map(|(i, known)| known.then_some(Value(i as u32)))
            .collect(),
        writes: writes.into(),
        reads: reads.into(),
    })
}
