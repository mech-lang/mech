//! One-time numeric initialization from the canonical artifact graph.
//!
//! This uses the compute operation mapping and scalar semantics, and core's
//! canonical range visitor. No source parser or turn executor participates.

use crate::{
    ConcatenationInput, ElementwiseInstruction, ElementwiseLowering, elementwise_lowering,
    resolve_compute_slot_dimensions,
};
use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, AliasPolicy, CellSlotId, DeliveryMode, ExternalInteraction, OutputConstruction,
    ResolvedOperationContract, ResolvedRangeMode, ValueData,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, InitializerReference, ProducerReference, ProgramArtifact,
    SlotRole,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
struct NumericValue {
    values: Arc<[f32]>,
    rows: usize,
    columns: usize,
}

/// Memoized activation values shared by every state in one compute compilation.
/// Values are canonical row-major snapshots; targets perform their storage
/// layout conversion only after initialization succeeds.
pub struct ComputeActivationValues<'a> {
    artifact: &'a ProgramArtifact,
    dimensions: BTreeMap<CellSlotId, Box<[u64]>>,
    values: BTreeMap<ArtifactSource, NumericValue>,
}

impl<'a> ComputeActivationValues<'a> {
    pub fn new(artifact: &'a ProgramArtifact) -> Self {
        Self {
            artifact,
            dimensions: resolve_compute_slot_dimensions(artifact),
            values: BTreeMap::new(),
        }
    }

    pub fn initializer(
        &mut self,
        initializer: Option<InitializerReference>,
    ) -> Result<Arc<[f32]>, String> {
        let source = match initializer {
            Some(InitializerReference::Constant(id)) => ArtifactSource::Constant(id),
            Some(InitializerReference::Activation(slot)) => ArtifactSource::Slot(slot),
            None => return Err("state has no initializer".into()),
        };
        self.materialize(source)?;
        Ok(self.values[&source].values.clone())
    }

    fn materialize(&mut self, root: ArtifactSource) -> Result<(), String> {
        // Explicit postorder traversal bounds stack depth by the artifact graph.
        // In particular, a state read must never follow its recurrence producer.
        let mut pending = vec![(root, false)];
        let mut active = BTreeSet::new();
        while let Some((source, ready)) = pending.pop() {
            if self.values.contains_key(&source) {
                continue;
            }
            if let ArtifactSource::Constant(id) = source {
                let value = self
                    .artifact
                    .constants()
                    .get(id)
                    .ok_or("missing activation constant")?;
                let dimensions =
                    crate::shape::source_dimensions(self.artifact, &self.dimensions, source)
                        .ok_or("activation constant has unresolved dimensions")?;
                let (rows, columns) = matrix_shape(&dimensions)?;
                let values: Arc<[f32]> = match value.data() {
                    ValueData::F32(value) => Arc::from([value.to_f32()]),
                    ValueData::Matrix(matrix) => match matrix.elements() {
                        SequenceView::F32(values) => {
                            values.iter().map(|value| value.to_f32()).collect()
                        }
                        _ => return Err("activation constant must contain f32 values".into()),
                    },
                    _ => return Err("activation constant must be f32 numeric data".into()),
                };
                if values.len() != extent(rows, columns)? {
                    return Err("activation constant shape mismatch".into());
                }
                self.values.insert(
                    source,
                    NumericValue {
                        values,
                        rows,
                        columns,
                    },
                );
                continue;
            }
            let ArtifactSource::Slot(slot) = source else {
                unreachable!()
            };
            let declaration = self
                .artifact
                .slots()
                .get(slot.get() as usize)
                .ok_or("missing activation slot")?;
            if matches!(declaration.role, SlotRole::Input | SlotRole::State) {
                return Err(format!(
                    "initializer depends on live {:?} slot {}; unavailable at activation",
                    declaration.role,
                    slot.get()
                ));
            }
            let (node, inputs) = match declaration.producer {
                ProducerReference::Output { source, .. } => (None, vec![source]),
                ProducerReference::NodeOutput { node, .. } => {
                    let node = self
                        .artifact
                        .nodes()
                        .get(node.get() as usize)
                        .and_then(mech_engine::NodeDeclaration::as_operation)
                        .ok_or("initializer requires a numeric operation")?;
                    let inputs = node
                        .input_bindings
                        .clone()
                        .map(|index| match self.artifact.bindings().get(index as usize) {
                            Some(BindingDeclaration::Input { source, .. }) => Ok(*source),
                            _ => Err("invalid activation input binding"),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    (Some(node), inputs)
                }
                ProducerReference::Input(_) => {
                    return Err("initializer input is unavailable at activation".into());
                }
            };
            if !ready {
                if !active.insert(source) {
                    return Err("cyclic activation dependency".into());
                }
                pending.push((source, true));
                pending.extend(inputs.iter().rev().map(|input| (*input, false)));
                continue;
            }
            let Some(node) = node else {
                self.values.insert(source, self.values[&inputs[0]].clone());
                active.remove(&source);
                continue;
            };
            let operation = crate::display_operation(&node.operation);
            let contract = self.artifact.contracts().get(node.contract);
            if !matches!(contract, Some(ResolvedOperationContract::Declared(contract)) if contract.interaction == ExternalInteraction::Pure
                && contract.inputs.iter().all(|input| input.access == AccessMode::Read && input.delivery == DeliveryMode::Signal)
                && contract.outputs.iter().all(|output| output.access == AccessMode::Write
                    && output.delivery == DeliveryMode::Signal && output.alias == AliasPolicy::NoAlias
                    && matches!(output.construction, OutputConstruction::FullWrite { .. } | OutputConstruction::Replace { .. } | OutputConstruction::Build { .. })))
            {
                return Err(format!(
                    "activation operation {operation} requires pure signal reads and independent whole-value writes"
                ));
            }
            let outputs = node
                .output_bindings
                .clone()
                .filter_map(|index| match self.artifact.bindings().get(index as usize) {
                    Some(BindingDeclaration::Output { target, .. }) => Some(*target),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if outputs.as_slice() != [slot] {
                return Err(format!(
                    "activation operation {operation} requires one output"
                ));
            }
            let dimensions = self
                .dimensions
                .get(&slot)
                .ok_or_else(|| format!("activation slot {} has unresolved shape", slot.get()))?;
            let (rows, columns) = matrix_shape(dimensions)?;
            let count = extent(rows, columns)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| "activation allocation failed")?;
            if let Some(mode) = node.operation.resolved_range_mode() {
                let endpoints = inputs
                    .iter()
                    .map(|input| {
                        let value = &self.values[input];
                        if value.values.len() != 1 {
                            return Err("range endpoint must be scalar");
                        }
                        Ok(ValueData::F32(mech_core::snapshot::F32Bits::from_f32(
                            value.values[0],
                        )))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let (inclusive, incremented) = match mode {
                    ResolvedRangeMode::Exclusive => (false, false),
                    ResolvedRangeMode::ExclusiveIncrement => (false, true),
                    ResolvedRangeMode::Inclusive => (true, false),
                    ResolvedRangeMode::InclusiveIncrement => (true, true),
                };
                mech_core::visit_canonical_value_range(
                    &endpoints,
                    inclusive,
                    incremented,
                    count,
                    |value| {
                        let ValueData::F32(value) = value else {
                            return Err("range output must be f32");
                        };
                        values.push(value.to_f32());
                        Ok(())
                    },
                )
                .map_err(|error| format!("invalid activation range: {error:?}"))?;
            } else {
                match elementwise_lowering(&node.operation)
                    .ok_or_else(|| format!("unsupported activation operation {operation}"))?
                {
                    ElementwiseLowering::Apply(operation) => {
                        if inputs.len() != operation.arity() {
                            return Err("activation operation arity mismatch".into());
                        }
                        for input in &inputs {
                            let input = &self.values[input];
                            if (input.rows != 1 && input.rows != rows)
                                || (input.columns != 1 && input.columns != columns)
                            {
                                return Err("activation broadcast axis mismatch".into());
                            }
                        }
                        let instruction = ElementwiseInstruction::Apply {
                            operation,
                            inputs: inputs.clone().into_boxed_slice(),
                            output: slot,
                            elements: count as u64,
                        };
                        instruction.evaluate_into(&mut values, |source, index, _| {
                            let input = &self.values[&source];
                            let row = index / columns;
                            let column = index % columns;
                            Ok::<_, String>(
                                input.values[(if input.rows == 1 { 0 } else { row })
                                    * input.columns
                                    + if input.columns == 1 { 0 } else { column }],
                            )
                        })?;
                    }
                    ElementwiseLowering::Concatenate(axis) => {
                        let dimensions = inputs
                            .iter()
                            .map(|source| {
                                let input = &self.values[source];
                                vec![input.rows as u64, input.columns as u64]
                            })
                            .collect::<Vec<_>>();
                        if crate::concatenate_shapes(
                            axis,
                            &dimensions,
                            &[rows as u64, columns as u64],
                        )
                        .is_none()
                        {
                            return Err("activation concatenation shape mismatch".into());
                        }
                        let instruction = ElementwiseInstruction::Concatenate {
                            axis,
                            output: slot,
                            rows: rows as u64,
                            columns: columns as u64,
                            inputs: inputs
                                .iter()
                                .map(|source| {
                                    let input = &self.values[source];
                                    ConcatenationInput {
                                        source: *source,
                                        rows: input.rows as u64,
                                        columns: input.columns as u64,
                                    }
                                })
                                .collect(),
                        };
                        instruction.evaluate_into(&mut values, |source, index, _| {
                            Ok::<_, String>(self.values[&source].values[index])
                        })?;
                    }
                }
            }
            if values.len() != count {
                return Err("activation result shape mismatch".into());
            }
            self.values.insert(
                source,
                NumericValue {
                    values: values.into(),
                    rows,
                    columns,
                },
            );
            active.remove(&source);
        }
        Ok(())
    }
}

fn matrix_shape(dimensions: &[u64]) -> Result<(usize, usize), String> {
    match dimensions {
        [] => Ok((1, 1)),
        [rows, columns] => Ok((
            usize::try_from(*rows).map_err(|_| "activation row extent overflow")?,
            usize::try_from(*columns).map_err(|_| "activation column extent overflow")?,
        )),
        _ => Err("activation supports scalar and matrix f32 values".into()),
    }
}

fn extent(rows: usize, columns: usize) -> Result<usize, String> {
    let elements = rows
        .checked_mul(columns)
        .ok_or("activation extent overflow")?;
    if elements > u32::MAX as usize {
        return Err("activation extent exceeds compute addressing".into());
    }
    Ok(elements)
}
