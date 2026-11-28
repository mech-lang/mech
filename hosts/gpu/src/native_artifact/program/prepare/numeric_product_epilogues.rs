//! Source-closed product epilogues and pointwise producers of ordered sums.
//! Private facts only; no pointers,
//! current shape, owners, fees, serialized authority or execution permission.
use super::numeric_binary::{Kind, Layout};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts, NativeProgramReadRole as Read,
    NativeScalarKind,
};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::native_artifact) const MAX_INPUTS: usize = 8;
const MAX_NODES: usize = 16;
const MAX_STEPS: usize = 64;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Source {
    External(usize),
    Node(usize),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Node {
    pub site: u32,
    pub inputs: [Source; 2],
    pub output: Value,
    pub kind: Kind,
}
impl Node {
    pub fn operands(&self) -> &[Source] {
        &self.inputs[..if matches!(self.kind, Kind::Sum { .. }) {
            1
        } else {
            2
        }]
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Step {
    Operator {
        site: u32,
        node: usize,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Candidate {
    pub entry: u32,
    pub end: u32,
    pub resume: u32,
    pub parent: Option<u32>,
    pub external: Vec<Value>,
    pub scalars: Vec<(usize, Value)>,
    pub nodes: Vec<Node>,
    pub steps: Vec<Step>,
    pub output: Value,
    pub elided_values: Vec<u32>,
}
impl Candidate {
    pub fn reduction_axis(&self) -> Option<usize> {
        match self.nodes.last()?.kind {
            Kind::Sum { axis, .. } => Some(axis),
            _ => None,
        }
    }
}
fn typed(plan: &Plan, value: Value) -> bool {
    let ty = &plan.values()[value.get() as usize];
    ty.definition().dimension_parameters().len() <= 2
        && (ty.scalar_kind() == Some(NativeScalarKind::F64)
            || matches!(ty.definition().body(),
            mech_core::SchemaBody::Matrix {element,dimensions} if dimensions.len()==2 && element.as_ref()==&mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)))
}

fn source(
    plan: &Plan,
    facts: &Facts<'_>,
    value: Value,
    entry: u32,
    parent: Option<u32>,
    bindings: &BTreeMap<u32, (Value, Source)>,
    external: &mut Vec<Value>,
) -> Option<Source> {
    if let Some((_, source)) = bindings.get(&value.get()) {
        return Some(*source);
    }
    if !typed(plan, value)
        || !super::numeric_maps::immutable_before(facts, value, entry, parent).ok()?
    {
        return None;
    }
    let at = match external.iter().position(|&v| v == value) {
        Some(at) => at,
        None => {
            if external.len() == MAX_INPUTS {
                return None;
            }
            let at = external.len();
            external.push(value);
            at
        }
    };
    Some(Source::External(at))
}
fn output(
    facts: &Facts<'_>,
    bindings: &BTreeMap<u32, (Value, Source)>,
    sites: &BTreeSet<u32>,
    nodes: &[Node],
) -> Option<Value> {
    if nodes.len() < 2 {
        return None;
    }
    let mut escaped = Vec::new();
    for &(value, source) in bindings.values() {
        if facts
            .read_roles(value)
            .ok()?
            .iter()
            .any(|r| !matches!(r,Read::Action(site) if sites.contains(site)))
        {
            escaped.push((value, source));
        }
    }
    let [(value, Source::Node(final_node))] = escaped.as_slice() else {
        return None;
    };
    if *final_node != nodes.len() - 1 {
        return None;
    }
    let mut used = BTreeSet::new();
    let mut pending = vec![*final_node];
    while let Some(at) = pending.pop() {
        if !used.insert(at) {
            continue;
        }
        pending.extend(nodes[at].operands().iter().filter_map(|s| match s {
            Source::Node(n) => Some(*n),
            _ => None,
        }));
    }
    (used.len() == nodes.len()).then_some(*value)
}
fn discover(plan: &Plan, facts: &Facts<'_>, operators: &Layout, entry: u32) -> Option<Candidate> {
    let first = operators.at(entry as usize)?.kind;
    if !matches!(first, Kind::Product | Kind::Elementwise(_)) {
        return None;
    }
    let product = first == Kind::Product;
    let parent = plan.blocks()[entry as usize].iteration();
    let mut external = Vec::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut steps = Vec::new();
    let mut bindings = BTreeMap::new();
    let mut sites = BTreeSet::new();
    let mut at = entry;
    let mut previous = None;
    let mut best = None;
    loop {
        let block = plan.blocks().get(at as usize)?;
        if steps.len() == MAX_STEPS
            || plan.activation_boundary() == Some(at)
            || block.iteration() != parent
            || sites.contains(&at)
            || previous.is_some_and(|p| facts.predecessors(at).ok() != Some([p].as_slice()))
        {
            break;
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            break;
        };
        let step = match block.action() {
            Action::LibraryCall {
                operands, output, ..
            } => {
                let Some(op) = operators.at(at as usize) else {
                    break;
                };
                let sum = matches!(op.kind, Kind::Sum { .. });
                if nodes.len() == MAX_NODES
                    || !typed(plan, *output)
                    || operands.len() != if sum { 1 } else { 2 }
                    || nodes
                        .last()
                        .is_some_and(|n| matches!(n.kind, Kind::Sum { .. }))
                    || (nodes.is_empty() && op.kind != first)
                    || (!nodes.is_empty() && product && !matches!(op.kind, Kind::Elementwise(_)))
                    || (!product && !matches!(op.kind, Kind::Elementwise(_) | Kind::Sum { .. }))
                {
                    break;
                }
                let Some(left) = source(
                    plan,
                    facts,
                    operands[0],
                    entry,
                    parent,
                    &bindings,
                    &mut external,
                ) else {
                    break;
                };
                let Some(right) = source(
                    plan,
                    facts,
                    operands[if sum { 0 } else { 1 }],
                    entry,
                    parent,
                    &bindings,
                    &mut external,
                ) else {
                    break;
                };
                if sum && !matches!(left, Source::Node(_)) {
                    break;
                }
                let node = nodes.len();
                nodes.push(Node {
                    site: at,
                    inputs: [left, right],
                    output: *output,
                    kind: op.kind,
                });
                bindings.insert(output.get(), (*output, Source::Node(node)));
                Step::Operator { site: at, node }
            }
            Action::Scalar(Scalar::Copy {
                source: from,
                output,
            }) => {
                let a = &plan.values()[from.get() as usize];
                let b = &plan.values()[output.get() as usize];
                if a.schema() != b.schema() || a.definition() != b.definition() {
                    break;
                }
                let Some(source) =
                    source(plan, facts, *from, entry, parent, &bindings, &mut external)
                else {
                    break;
                };
                bindings.insert(output.get(), (*output, source));
                Step::Alias {
                    site: at,
                    source,
                    output: *output,
                }
            }
            Action::Scalar(Scalar::None) => Step::Empty { site: at },
            _ => break,
        };
        sites.insert(at);
        steps.push(step);
        if (product
            || nodes
                .last()
                .is_some_and(|n| matches!(n.kind, Kind::Sum { .. })))
            && let Some(output) = output(facts, &bindings, &sites, &nodes)
        {
            best = Some(Candidate {
                entry,
                end: steps.last()?.site(),
                resume: next,
                parent,
                external: external.clone(),
                scalars: external
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, v)| {
                        plan.values()[v.get() as usize].scalar_kind() == Some(NativeScalarKind::F64)
                    })
                    .collect(),
                nodes: nodes.clone(),
                steps: steps.clone(),
                output,
                elided_values: bindings
                    .keys()
                    .copied()
                    .filter(|&v| v != output.get())
                    .collect(),
            });
        }
        previous = Some(at);
        at = next;
    }
    best
}
pub(in crate::native_artifact) fn derive(plan: &Plan, operators: &Layout) -> Vec<Candidate> {
    let mut covered = BTreeSet::new();
    (0..plan.blocks().len())
        .filter_map(|at| discover(plan, &plan.facts(), operators, u32::try_from(at).ok()?))
        .filter(|recipe| {
            // Prefer the earliest closed producer and its longest proved
            // continuation. The ordinary fallback stays complete; do not emit
            // overlapping suffix loans inside the same source expression.
            if recipe
                .steps
                .iter()
                .any(|step| covered.contains(&step.site()))
            {
                return false;
            }
            covered.extend(recipe.steps.iter().map(Step::site));
            true
        })
        .collect()
}
