//! Source-only transpose/product consumers. No owner, pointer, fee or loan.
//! Independent derivation never accepts a serialized view or current sample.
use super::numeric_binary::{Kind, Layout};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts, NativeProgramReadRole as Read,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct ProductView {
    pub entry: u32,
    pub product: u32,
    pub continuation: Vec<u32>,
    pub elided_values: Vec<u32>,
    /// Actual original owners; the selected operand has exchanged logical axes.
    pub operands: [u32; 2],
    pub transposed_operand: usize,
    pub output: u32,
    pub resume: u32,
    pub parent: Option<u32>,
}

fn sole_consumer(
    plan: &Plan,
    facts: &Facts<'_>,
    mut value: Value,
    copies: &mut BTreeSet<u32>,
    elided: &mut BTreeSet<u32>,
) -> Option<u32> {
    for _ in 0..64 {
        if !elided.insert(value.get()) {
            return None;
        }
        let [Read::Action(site)] = facts.read_roles(value).ok()? else {
            return None;
        };
        match plan.blocks().get(*site as usize)?.action() {
            Action::Scalar(Scalar::Copy { source, output })
                if *source == value
                    && plan.values().get(source.get() as usize)?.definition()
                        == plan.values().get(output.get() as usize)?.definition()
                    && facts.definition_sites(output.get()).ok()?.len() == 1 =>
            {
                copies.insert(*site);
                value = *output;
            }
            _ => return Some(*site),
        }
    }
    None
}

fn candidate(plan: &Plan, facts: &Facts<'_>, binary: &Layout, entry: u32) -> Option<ProductView> {
    let transpose = binary.at(entry as usize)?;
    if !matches!(transpose.kind, Kind::Transpose { .. }) || transpose.operands.len() != 1 {
        return None;
    }
    let block = plan.blocks().get(entry as usize)?;
    let Action::LibraryCall {
        operands: transpose_operands,
        output: transpose_output,
        ..
    } = block.action()
    else {
        return None;
    };
    let parent = block.iteration();
    if plan.activation_boundary() == Some(entry)
        || !super::numeric_maps::immutable_before(
            facts,
            *transpose_operands.first()?,
            entry,
            parent,
        )
        .ok()?
    {
        return None;
    }
    let mut copies = BTreeSet::new();
    let mut elided = BTreeSet::new();
    let product = sole_consumer(plan, facts, *transpose_output, &mut copies, &mut elided)?;
    let consumer = binary.at(product as usize)?;
    if consumer.kind != Kind::Product || consumer.operands.len() != 2 {
        return None;
    }
    let selected = consumer
        .operands
        .iter()
        .enumerate()
        .filter(|(_, value)| elided.contains(value))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [transposed_operand] = selected.as_slice() else {
        return None;
    };
    let transposed_operand = *transposed_operand;
    let Action::LibraryCall {
        operands: product_operands,
        ..
    } = plan.blocks().get(product as usize)?.action()
    else {
        return None;
    };
    let other = 1 - transposed_operand;
    if !super::numeric_maps::immutable_before(facts, *product_operands.get(other)?, entry, parent)
        .ok()?
    {
        return None;
    }
    let Terminator::Scalar(Term::Jump(mut at)) = block.terminator() else {
        return None;
    };
    let mut previous = entry;
    let mut continuation = Vec::new();
    let mut seen = BTreeSet::new();
    loop {
        let block = plan.blocks().get(at as usize)?;
        if continuation.len() >= 64
            || !seen.insert(at)
            || plan.activation_boundary() == Some(at)
            || block.iteration() != parent
            || facts.predecessors(at).ok()? != [previous]
        {
            return None;
        }
        continuation.push(at);
        if at == product {
            break;
        }
        if !copies.contains(&at) && !matches!(block.action(), Action::Scalar(Scalar::None)) {
            return None;
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        previous = at;
        at = next;
    }
    if !copies.iter().all(|site| continuation.contains(site)) {
        return None;
    }
    let Terminator::Scalar(Term::Jump(resume)) = plan.blocks()[product as usize].terminator()
    else {
        return None;
    };
    let mut operands = [consumer.operands[0], consumer.operands[1]];
    operands[transposed_operand] = transpose.operands[0];
    Some(ProductView {
        entry,
        product,
        continuation,
        elided_values: elided.into_iter().collect(),
        operands,
        transposed_operand,
        output: consumer.output,
        resume,
        parent,
    })
}

pub(in crate::native_artifact) fn derive_with_binary(
    plan: &Plan,
    binary: &Layout,
) -> Result<Vec<ProductView>, &'static str> {
    let facts = plan.facts();
    Ok((0..plan.blocks().len())
        .filter_map(|site| candidate(plan, &facts, binary, u32::try_from(site).ok()?))
        .collect())
}
#[cfg(test)]
pub(in crate::native_artifact) fn derive(plan: &Plan) -> Result<Vec<ProductView>, &'static str> {
    let binary = Layout::derive(plan)?;
    Ok((0..plan.blocks().len())
        .filter_map(|site| binary.product_view(site).cloned())
        .collect())
}
impl ProductView {
    /// Return actual source IDs, never manufacture a scalar/managed capability.
    pub fn source_operands(&self, plan: &Plan) -> Option<[Value; 2]> {
        let Action::LibraryCall {
            operands: producer, ..
        } = plan.blocks().get(self.entry as usize)?.action()
        else {
            return None;
        };
        let Action::LibraryCall {
            operands: consumer, ..
        } = plan.blocks().get(self.product as usize)?.action()
        else {
            return None;
        };
        let mut values = [*consumer.first()?, *consumer.get(1)?];
        values[self.transposed_operand] = *producer.first()?;
        (values.map(|value| value.get()) == self.operands).then_some(values)
    }
}
