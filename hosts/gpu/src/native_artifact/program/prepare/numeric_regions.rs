//! Source-closed root matrix-expression recipes. No pointer, owner, budget
//! credit or execution permission is cached here. Current admission is separate.
use super::numeric_binary::{Kind, Layout as Operators};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeIterationValueScope as Scope, NativeMatchAction as Scalar, NativeMatchTerminator as Term,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts,
    NativeProgramReadRole as Read, NativeScalarKind,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) enum Source {
    External(usize),
    Bank(usize),
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) enum Step {
    Operator {
        site: u32,
        kind: Kind,
        inputs: Box<[Source]>,
        output: Value,
        bank: usize,
    },
    Alias {
        site: u32,
        source: Source,
        output: Value,
    },
    Empty {
        site: u32,
    },
}
impl Step {
    pub fn site(&self) -> u32 {
        match *self {
            Self::Operator { site, .. } | Self::Alias { site, .. } | Self::Empty { site } => site,
        }
    }
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Output {
    pub value: Value,
    pub source: Source,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Candidate {
    pub entry: u32,
    pub exit: Option<u32>,
    pub steps: Box<[Step]>,
    pub external: Box<[Value]>,
    pub outputs: Box<[Output]>,
    pub private_banks: Box<[usize]>,
    /// A complete bank's last read, or steps.len() if it escapes the region.
    pub last_reads: Box<[usize]>,
    /// Logical banks can share a slot only across strictly disjoint lifetimes.
    /// Admission still sizes each slot against every current logical extent.
    pub storage_slots: Box<[usize]>,
    pub slots: usize,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub candidates: Box<[Candidate]>,
}

fn plain(plan: &Plan, value: Value) -> bool {
    let Some(ty) = plan.values().get(value.get() as usize) else {
        return false;
    };
    if !matches!(
        plan.value_scopes().get(value.get() as usize),
        Some(Scope::Immutable { iteration: None })
    ) {
        return false;
    }
    ty.scalar_kind() == Some(NativeScalarKind::F64)
        || matches!(ty.definition().body(), mech_core::SchemaBody::Matrix { element, dimensions }
            if dimensions.len() == 2 && element.as_ref() == &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64))
}

fn source(
    plan: &Plan,
    facts: &Facts<'_>,
    entry: u32,
    value: Value,
    values: &BTreeMap<u32, (Value, Source)>,
    external: &mut Vec<Value>,
) -> Result<Option<Source>, &'static str> {
    if let Some((_, source)) = values.get(&value.get()) {
        return Ok(Some(*source));
    }
    if !plain(plan, value) {
        return Ok(None);
    }
    let [definition] = facts.definition_sites(value.get())? else {
        return Ok(None);
    };
    if let Some(site) = definition.site()
        && !facts.strictly_dominates(site, entry)?
    {
        return Ok(None);
    }
    let at = match external.iter().position(|v| *v == value) {
        Some(at) => at,
        None => {
            let at = external.len();
            external.push(value);
            at
        }
    };
    Ok(Some(Source::External(at)))
}

fn discover(
    plan: &Plan,
    facts: &Facts<'_>,
    operators: &Operators,
    entry: u32,
    claimed: &BTreeSet<u32>,
) -> Result<Option<Candidate>, &'static str> {
    let mut steps = Vec::new();
    let mut external = Vec::new();
    let mut values = BTreeMap::new();
    let mut sites = BTreeSet::new();
    let mut starts = Vec::new();
    let mut dependent = false;
    let mut at = entry;
    let exit;
    loop {
        if claimed.contains(&at) || plan.activation_boundary() == Some(at) {
            exit = Some(at);
            break;
        }
        if !sites.insert(at) {
            return Ok(None);
        }
        let block = plan
            .blocks()
            .get(at as usize)
            .ok_or("matrix region site is absent")?;
        if block.iteration().is_some() {
            sites.remove(&at);
            exit = Some(at);
            break;
        }
        let next = match block.terminator() {
            Terminator::Scalar(Term::Jump(next)) => Some(next),
            Terminator::Scalar(Term::Complete) => None,
            _ => {
                sites.remove(&at);
                exit = Some(at);
                break;
            }
        };
        let step = match block.action() {
            Action::LibraryCall {
                operands, output, ..
            } => {
                let Some(op) = operators.at(at as usize) else {
                    sites.remove(&at);
                    exit = Some(at);
                    break;
                };
                if !plain(plan, *output) {
                    return Ok(None);
                }
                let mut inputs = Vec::with_capacity(operands.len());
                for &value in operands {
                    let Some(input) = source(plan, facts, entry, value, &values, &mut external)?
                    else {
                        return Ok(None);
                    };
                    dependent |= matches!(input, Source::Bank(_));
                    inputs.push(input);
                }
                let bank = starts.len();
                starts.push(steps.len());
                values.insert(output.get(), (*output, Source::Bank(bank)));
                Step::Operator {
                    site: at,
                    kind: op.kind,
                    inputs: inputs.into_boxed_slice(),
                    output: *output,
                    bank,
                }
            }
            Action::Scalar(Scalar::Copy {
                source: from,
                output,
            }) => {
                let left = &plan.values()[from.get() as usize];
                let right = &plan.values()[output.get() as usize];
                if !plain(plan, *output)
                    || left.schema() != right.schema()
                    || left.definition() != right.definition()
                {
                    sites.remove(&at);
                    exit = Some(at);
                    break;
                }
                let Some(from) = source(plan, facts, entry, *from, &values, &mut external)? else {
                    return Ok(None);
                };
                values.insert(output.get(), (*output, from));
                Step::Alias {
                    site: at,
                    source: from,
                    output: *output,
                }
            }
            Action::Scalar(Scalar::None) => Step::Empty { site: at },
            _ => {
                sites.remove(&at);
                exit = Some(at);
                break;
            }
        };
        steps.push(step);
        match next {
            None => {
                exit = None;
                break;
            }
            Some(next) => {
                if facts.predecessors(next)? != [at] {
                    exit = Some(next);
                    break;
                }
                at = next;
            }
        }
    }
    if starts.len() < 2 || !dependent {
        return Ok(None);
    }
    let mut last_reads = starts.clone();
    for (index, step) in steps.iter().enumerate() {
        let inputs: &[Source] = match step {
            Step::Operator { inputs, .. } => inputs,
            Step::Alias { source, .. } => core::slice::from_ref(source),
            Step::Empty { .. } => &[],
        };
        for input in inputs {
            if let Source::Bank(bank) = *input {
                last_reads[bank] = index;
            }
        }
    }
    let mut outputs = Vec::new();
    let mut escaping = BTreeSet::new();
    for &(value, source) in values.values() {
        let escape = facts
            .read_roles(value)?
            .iter()
            .any(|role| !matches!(role, Read::Action(site) if sites.contains(site)));
        if escape {
            if let Source::Bank(bank) = source {
                last_reads[bank] = steps.len();
                escaping.insert(bank);
            }
            outputs.push(Output { value, source });
        }
    }
    let mut slot_ends = Vec::new();
    let mut storage_slots = Vec::new();
    for (bank, &start) in starts.iter().enumerate() {
        // An input read by this operation remains live through its stores.
        let slot = slot_ends
            .iter()
            .position(|&end| end < start)
            .unwrap_or(slot_ends.len());
        if slot == slot_ends.len() {
            slot_ends.push(last_reads[bank]);
        } else {
            slot_ends[slot] = last_reads[bank];
        }
        storage_slots.push(slot);
    }
    let private_banks = (0..starts.len())
        .filter(|bank| !escaping.contains(bank))
        .collect();
    Ok(Some(Candidate {
        entry,
        exit,
        steps: steps.into_boxed_slice(),
        external: external.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        private_banks,
        last_reads: last_reads.into_boxed_slice(),
        storage_slots: storage_slots.into_boxed_slice(),
        slots: slot_ends.len(),
    }))
}

impl Layout {
    pub fn derive(plan: &Plan, operators: &Operators) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let facts = plan.facts();
        // Source execution order, including independently entered branch arms.
        // Numeric block ordinals need not be in topological order.
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        let mut pending = vec![plan.entry()];
        while let Some(site) = pending.pop() {
            if !seen.insert(site) {
                continue;
            }
            order.push(site);
            pending.extend(facts.successors(site)?.into_iter().rev().flatten());
        }
        let mut claimed = BTreeSet::new();
        let mut candidates = Vec::new();
        for entry in order {
            if claimed.contains(&entry) || operators.at(entry as usize).is_none() {
                continue;
            }
            if let Some(candidate) = discover(plan, &facts, operators, entry, &claimed)? {
                claimed.extend(candidate.steps.iter().map(Step::site));
                candidates.push(candidate);
            }
        }
        Ok(Self {
            candidates: candidates.into_boxed_slice(),
        })
    }
}

impl Candidate {
    /// Leave the ordinary terminal visit/output publication in the source CFG.
    /// Scalar aliases observed outside this body need the existing word-home
    /// bridge, so they retain ordinary execution rather than inventing homes.
    pub fn body<'a>(&'a self, plan: &Plan) -> Option<(&'a [Step], u32)> {
        if self.outputs.iter().any(|o| {
            plan.values()[o.value.get() as usize]
                .scalar_kind()
                .is_some()
        }) {
            return None;
        }
        let (steps, exit) = match self.exit {
            Some(exit) => (self.steps.as_ref(), exit),
            None => match self.steps.last()? {
                Step::Empty { site } => (&self.steps[..self.steps.len() - 1], *site),
                _ => return None,
            },
        };
        // The existing original entry header pays all bounded metadata
        // discovery before current dynamic source envelopes are available.
        let mut discovery = 4096u64.checked_add((self.external.len() as u64).checked_mul(256)?)?;
        for step in steps {
            discovery = discovery.checked_add(256)?;
            if let Step::Operator { output, .. } = step {
                discovery = discovery.checked_add(
                    plan.values()[output.get() as usize]
                        .definition()
                        .clone_allocation_bound_bytes()?
                        .checked_mul(4)?,
                )?;
            }
        }
        (discovery <= super::numeric_binary::HEADER_WORK).then_some((steps, exit))
    }
}
impl Layout {
    pub fn at_entry(&self, site: usize) -> Option<(usize, &Candidate)> {
        self.candidates
            .iter()
            .enumerate()
            .find(|(_, c)| c.entry as usize == site)
    }
    pub fn argument_words(&self, plan: &Plan) -> usize {
        self.candidates
            .iter()
            .filter(|c| c.body(plan).is_some())
            .map(|c| {
                c.external
                    .iter()
                    .filter(|v| {
                        plan.values()[v.get() as usize].scalar_kind() == Some(NativeScalarKind::F64)
                    })
                    .count()
            })
            .max()
            .unwrap_or(0)
    }
}
