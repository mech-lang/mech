//! Per-write old-version use analysis. This is a necessary ownership fact,
//! never a storage selection, library witness or execution/publication permit.
use super::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation as Observation, NativeStructuralStep,
};
use mech_core::{IntegerWidth, SchemaBody, ValueData};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Step {
    Tuple(u32),
    Variant(u32),
}
#[derive(Clone, Debug)]
pub struct NativeMapWriteUses {
    site: u32,
    iteration: u32,
    base: Value,
    output: Value,
    carrier_derived: bool,
    late_reads: Box<[u32]>,
}
impl NativeMapWriteUses {
    pub const fn site(&self) -> u32 {
        self.site
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn base(&self) -> Value {
        self.base
    }
    pub const fn output(&self) -> Value {
        self.output
    }
    pub const fn carrier_derived(&self) -> bool {
        self.carrier_derived
    }
    pub fn late_reads(&self) -> &[u32] {
        &self.late_reads
    }
}
struct Fields {
    count: usize,
    by_value: Vec<BTreeMap<Vec<Step>, usize>>,
}
fn paths(
    body: &SchemaBody,
    path: &mut Vec<Step>,
    out: &mut Vec<Vec<Step>>,
    depth: usize,
) -> Result<(), &'static str> {
    if depth > 32 {
        return Err("native map alias schema depth");
    }
    match body {
        SchemaBody::Map { key, value, .. }
            if key.as_ref() == &SchemaBody::String
                && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64) =>
        {
            out.push(path.clone())
        }
        SchemaBody::Tuple(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(Step::Tuple(index as u32));
                paths(item, path, out, depth + 1)?;
                path.pop();
            }
        }
        SchemaBody::Enum { variants, .. } => {
            for (index, variant) in variants.iter().enumerate() {
                if let Some(body) = &variant.payload {
                    path.push(Step::Variant(index as u32));
                    paths(body, path, out, depth + 1)?;
                    path.pop();
                }
            }
        }
        _ => {}
    }
    Ok(())
}
impl Fields {
    fn new(plan: &Plan) -> Result<Self, &'static str> {
        let mut count = 0;
        let mut by_value = Vec::new();
        for ty in plan.values() {
            let mut found = Vec::new();
            paths(ty.definition().body(), &mut Vec::new(), &mut found, 0)?;
            let mut entries = BTreeMap::new();
            for path in found {
                if count >= 4096 {
                    return Err("native map alias field limit");
                }
                entries.insert(path, count);
                count += 1;
            }
            by_value.push(entries);
        }
        Ok(Self { count, by_value })
    }
    fn link(&self, source: Value, output: Value, prefix: &[Step], pairs: &mut Vec<(usize, usize)>) {
        for (path, &from) in &self.by_value[source.get() as usize] {
            let mut target = prefix.to_vec();
            target.extend_from_slice(path);
            if let Some(&to) = self.by_value[output.get() as usize].get(&target) {
                pairs.push((from, to));
            }
        }
    }
}
struct Transfer {
    output: Value,
    pairs: Vec<(usize, usize)>,
}
fn transfer(plan: &Plan, fields: &Fields, action: &Action) -> Option<Transfer> {
    match action {
        Action::Scalar(Scalar::Copy { source, output }) => {
            let mut pairs = Vec::new();
            fields.link(*source, *output, &[], &mut pairs);
            Some(Transfer {
                output: *output,
                pairs,
            })
        }
        Action::Structural {
            source,
            path,
            observation: Observation::CanonicalValue,
            output,
        } => {
            let prefix = path
                .iter()
                .map(|step| match step {
                    NativeStructuralStep::TupleItem { ordinal } => Step::Tuple(*ordinal),
                    NativeStructuralStep::EnumPayload { ordinal } => Step::Variant(*ordinal),
                })
                .collect::<Vec<_>>();
            let mut pairs = Vec::new();
            for (path, &to) in &fields.by_value[output.get() as usize] {
                let mut from = prefix.clone();
                from.extend_from_slice(path);
                if let Some(&from) = fields.by_value[source.get() as usize].get(&from) {
                    pairs.push((from, to));
                }
            }
            Some(Transfer {
                output: *output,
                pairs,
            })
        }
        Action::LibraryCall {
            template,
            operands,
            output,
        } => {
            let mut pairs = Vec::new();
            match (template.primitive(), template.output().body()) {
                (Primitive::CompositePack(_), SchemaBody::Tuple(_)) => {
                    for (index, &source) in operands.iter().enumerate() {
                        fields.link(source, *output, &[Step::Tuple(index as u32)], &mut pairs);
                    }
                }
                (Primitive::EnumPack, SchemaBody::Enum { variants, .. }) if operands.len() == 2 => {
                    let ordinal = plan.constants().iter().find_map(|entry| {
                        if entry.value() != operands[0] {
                            return None;
                        }
                        let ValueData::Index(value) = entry.constant().data() else {
                            return None;
                        };
                        value
                            .checked_sub(1)
                            .and_then(|value| u32::try_from(value).ok())
                    });
                    for (index, variant) in variants.iter().enumerate() {
                        if ordinal.is_some_and(|ordinal| ordinal != index as u32) {
                            continue;
                        }
                        if variant.payload.as_ref()
                            == Some(
                                plan.values()[operands[1].get() as usize]
                                    .definition()
                                    .body(),
                            )
                        {
                            fields.link(
                                operands[1],
                                *output,
                                &[Step::Variant(index as u32)],
                                &mut pairs,
                            );
                        }
                    }
                }
                _ => return None,
            }
            Some(Transfer {
                output: *output,
                pairs,
            })
        }
        _ => None,
    }
}
fn successors(plan: &Plan, site: usize) -> Vec<usize> {
    match plan.blocks()[site].terminator() {
        Terminator::Scalar(Term::Jump(next)) => vec![next as usize],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
            vec![yes as usize, no as usize]
        }
        Terminator::TailHeader { body, .. } => vec![body as usize],
        Terminator::Header { body, exit, .. } => vec![body as usize, exit as usize],
        Terminator::Scalar(Term::Complete | Term::Reject | Term::IntegrityFailure) => vec![],
    }
}
fn find(parent: &mut [usize], value: usize) -> usize {
    if parent[value] != value {
        parent[value] = find(parent, parent[value]);
    }
    parent[value]
}
fn join(parent: &mut [usize], a: usize, b: usize) {
    let a = find(parent, a);
    let b = find(parent, b);
    parent[b] = a;
}
impl Plan {
    /// Report old-version reads reachable after each maintained map insertion,
    /// stopping at the owning latch only AFTER checking its carried reads.
    /// Transports use strong updates so an inactive enum field in another arm
    /// cannot invent an alias in the arm actually reached after the write.
    /// These facts alone do not authorize mutable storage or hash selection.
    pub fn map_write_uses(&self) -> Result<Box<[NativeMapWriteUses]>, &'static str> {
        self.validate_current_visits()?;
        if self.blocks().len() > 16384 {
            return Err("native map alias graph limit");
        }
        let fields = Fields::new(self)?;
        let actions = super::fusion_semantics::semantic_actions(self)?;
        let transfers = actions
            .iter()
            .map(|action| transfer(self, &fields, action))
            .collect::<Vec<_>>();
        // Transport definitions begin with no active fields; branch copies
        // accumulate only the fields their source constructor can define.
        let mut active = vec![true; fields.count];
        for transport in transfers.iter().flatten() {
            for &leaf in fields.by_value[transport.output.get() as usize].values() {
                active[leaf] = false;
            }
        }
        loop {
            let mut changed = false;
            for transport in transfers.iter().flatten() {
                for &(from, to) in &transport.pairs {
                    if active[from] && !active[to] {
                        active[to] = true;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut parents = (0..fields.count).collect::<Vec<_>>();
        for transport in transfers.iter().flatten() {
            for &(from, to) in &transport.pairs {
                if active[from] {
                    join(&mut parents, from, to);
                }
            }
        }
        let roots = (0..parents.len())
            .map(|index| find(&mut parents, index))
            .collect::<Vec<_>>();
        let mut reads = vec![Vec::<usize>::new(); self.blocks().len()];
        let all = |value: Value, into: &mut Vec<usize>| {
            into.extend(
                fields.by_value[value.get() as usize]
                    .values()
                    .copied()
                    .filter(|&leaf| active[leaf]),
            )
        };
        for (site, action) in actions.iter().enumerate() {
            if let Some(transport) = &transfers[site] {
                reads[site].extend(
                    transport
                        .pairs
                        .iter()
                        .filter_map(|&(from, _)| active[from].then_some(from)),
                );
            } else {
                match action {
                    Action::LibraryCall { operands, .. }
                    | Action::Scalar(Scalar::Primitive { operands, .. }) => {
                        for &value in operands {
                            all(value, &mut reads[site]);
                        }
                    }
                    Action::CollectionPush { value, .. } => all(*value, &mut reads[site]),
                    // Scalar and tag observations do not read map contents.
                    Action::Structural { .. } => {}
                    _ => {}
                }
            }
        }
        for loop_ in self.iterations() {
            match loop_ {
                NativeControlLoop::Tail(loop_) => {
                    all(loop_.seed(), &mut reads[loop_.initialize() as usize]);
                    all(loop_.next(), &mut reads[loop_.latch() as usize]);
                    all(loop_.returned(), &mut reads[loop_.finish() as usize]);
                }
                NativeControlLoop::Iteration(loop_) => {
                    all(loop_.seed(), &mut reads[loop_.initialize() as usize]);
                    all(loop_.next(), &mut reads[loop_.latch() as usize]);
                    all(loop_.carrier(), &mut reads[loop_.exit() as usize]);
                }
                NativeControlLoop::Collection(loop_) => all(
                    loop_.source().semantic_value(),
                    &mut reads[loop_.initialize() as usize],
                ),
            }
        }
        for (site, block) in self.blocks().iter().enumerate() {
            if matches!(block.terminator(), Terminator::Scalar(Term::Complete)) {
                for index in 0..self.transport_output_count() {
                    all(self.transport_output_value(index), &mut reads[site]);
                }
            }
        }
        let mut result = Vec::new();
        for (site, action) in actions.iter().enumerate() {
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = action
            else {
                continue;
            };
            if template.primitive() != Primitive::MapInsert || operands.len() != 3 {
                continue;
            }
            let Some(&base_leaf) = fields.by_value[operands[0].get() as usize].get(&Vec::new())
            else {
                continue;
            };
            let Some(iteration) = self.blocks()[site].iteration() else {
                continue;
            };
            let (carrier, latch) = match &self.iterations()[iteration as usize] {
                NativeControlLoop::Tail(loop_) => (loop_.carrier(), loop_.latch()),
                NativeControlLoop::Iteration(loop_) => (loop_.carrier(), loop_.latch()),
                _ => continue,
            };
            let root = roots[base_leaf];
            let carrier_derived = fields.by_value[carrier.get() as usize]
                .values()
                .any(|&leaf| roots[leaf] == root);
            let mut initial = roots
                .iter()
                .map(|&candidate| candidate == root)
                .collect::<Vec<_>>();
            for &leaf in fields.by_value[output.get() as usize].values() {
                initial[leaf] = false;
            }
            let mut inputs = vec![None::<Vec<bool>>; self.blocks().len()];
            let mut queue = VecDeque::new();
            for next in successors(self, site) {
                inputs[next] = Some(initial.clone());
                queue.push_back(next);
            }
            let mut late = BTreeSet::new();
            while let Some(at) = queue.pop_front() {
                let mut state = inputs[at].clone().ok_or("native map alias worklist")?;
                if reads[at].iter().any(|&leaf| state[leaf]) {
                    late.insert(at as u32);
                }
                if at == latch as usize {
                    continue;
                }
                if let Some(transport) = &transfers[at] {
                    let mapped = transport
                        .pairs
                        .iter()
                        .map(|&(from, to)| (to, state[from]))
                        .collect::<Vec<_>>();
                    for &leaf in fields.by_value[transport.output.get() as usize].values() {
                        state[leaf] = false;
                    }
                    for (to, old) in mapped {
                        state[to] |= old;
                    }
                } else if let Action::LibraryCall { output, .. } = actions[at] {
                    for &leaf in fields.by_value[output.get() as usize].values() {
                        state[leaf] = false;
                    }
                }
                for next in successors(self, at) {
                    let mut changed = false;
                    match &mut inputs[next] {
                        Some(current) => {
                            for (old, new) in current.iter_mut().zip(&state) {
                                if *new && !*old {
                                    *old = true;
                                    changed = true;
                                }
                            }
                        }
                        slot @ None => {
                            *slot = Some(state.clone());
                            changed = true;
                        }
                    }
                    if changed {
                        queue.push_back(next);
                    }
                }
            }
            result.push(NativeMapWriteUses {
                site: site as u32,
                iteration,
                base: operands[0],
                output: *output,
                carrier_derived,
                late_reads: late.into_iter().collect(),
            });
        }
        Ok(result.into())
    }
}
