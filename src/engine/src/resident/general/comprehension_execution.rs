use super::*;
use crate::resident::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::resident::general::{ActivatedCollectionStep, ActivatedComprehensionNode};
use mech_core::snapshot::{F64Bits, SnapshotCanonicalizationBudget, SnapshotValidationContext};
use mech_core::{ValueData, ValueDataDraft, ValueDraft};

#[derive(Clone, Copy)]
enum Item {
    Bool(bool),
    Index(u64),
    F64(f64),
}

impl Item {
    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Bool(value) => Some(Self::Bool(*value)),
            ValueData::Index(value) => Some(Self::Index(*value)),
            ValueData::F64(value) => Some(Self::F64(value.to_f64())),
            _ => None,
        }
    }
    fn from_scalar(value: ResidentValueRef<'_>) -> Option<Self> {
        match value {
            ResidentValueRef::Bool([value @ (0 | 1)]) => Some(Self::Bool(*value != 0)),
            ResidentValueRef::Index([value]) => Some(Self::Index(*value)),
            ResidentValueRef::F64([value]) => Some(Self::F64(*value)),
            _ => None,
        }
    }
    fn equals(self, other: Self) -> bool {
        match (self, other) {
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Index(a), Self::Index(b)) => a == b,
            (Self::F64(a), Self::F64(b)) => a == b,
            _ => false,
        }
    }
    fn data(self) -> ValueData {
        match self {
            Self::Bool(value) => ValueData::Bool(value),
            Self::Index(value) => ValueData::Index(value),
            Self::F64(value) => ValueData::F64(F64Bits::from_f64(value)),
        }
    }
    fn draft(self) -> ValueDataDraft {
        match self {
            Self::Bool(value) => ValueDataDraft::Bool(value),
            Self::Index(value) => ValueDataDraft::Index(value),
            Self::F64(value) => ValueDataDraft::F64(F64Bits::from_f64(value)),
        }
    }
}

fn collection_len(value: ResidentValueRef<'_>) -> Option<usize> {
    match value {
        ResidentValueRef::Snapshot([Some(value)]) => match value.data() {
            ValueData::Set(set) => Some(set.elements().len()),
            ValueData::Matrix(matrix) => Some(matrix.elements().len()),
            _ => None,
        },
        ResidentValueRef::Bool(value) => Some(value.len()),
        ResidentValueRef::Index(value) => Some(value.len()),
        ResidentValueRef::F64(value) => Some(value.len()),
        _ => None,
    }
}

fn collection_item(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
    ordinal: usize,
) -> Option<Item> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        return match value.data() {
            ValueData::Set(set) => Item::from_data(set.elements().get(ordinal)?.data()),
            ValueData::Matrix(matrix) => match matrix.elements() {
                mech_core::snapshot::SequenceView::F64(values) => {
                    Some(Item::F64(values.get(ordinal)?.to_f64()))
                }
                mech_core::snapshot::SequenceView::Bool(values) => {
                    Some(Item::Bool(*values.get(ordinal)?))
                }
                mech_core::snapshot::SequenceView::Index(values) => {
                    Some(Item::Index(*values.get(ordinal)?))
                }
                mech_core::snapshot::SequenceView::Values(values) => {
                    Item::from_data(values.get(ordinal)?)
                }
                _ => None,
            },
            _ => None,
        };
    }
    // Native matrices are column-major; the canonical collection order is
    // row-major, shared with snapshot-backed matrices.
    let columns = region.shape.columns as usize;
    let rows = region.shape.rows as usize;
    if columns == 0 {
        return None;
    }
    let offset = (ordinal % columns)
        .checked_mul(rows)?
        .checked_add(ordinal / columns)?;
    match value {
        ResidentValueRef::Bool(value) => match *value.get(offset)? {
            0 => Some(Item::Bool(false)),
            1 => Some(Item::Bool(true)),
            _ => None,
        },
        ResidentValueRef::Index(value) => Some(Item::Index(*value.get(offset)?)),
        ResidentValueRef::F64(value) => Some(Item::F64(*value.get(offset)?)),
        _ => None,
    }
}

fn region(location: ResidentReadLocation) -> ResidentRegion {
    match location {
        ResidentReadLocation::Constant(region)
        | ResidentReadLocation::Input(region)
        | ResidentReadLocation::Scratch(region)
        | ResidentReadLocation::State { region, .. } => region,
    }
}

fn admit_output(count: usize, meter: ResidentBudgetMeter) -> Result<(), ResidentKernelError> {
    let data = count
        .checked_mul(
            core::mem::size_of::<ValueDataDraft>() + core::mem::size_of::<ValueData>() + 16,
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    let temporary = data
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(core::mem::size_of::<Value>() + 64))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new((), budget::resident_cost! {
        output_elements: count,
        output_bytes: temporary,
        temporary_bytes: temporary,
        container_bytes: temporary,
        retained_nodes: count.checked_mul(2).and_then(|count| count.checked_add(2)).ok_or(ResidentKernelError::InvalidShape)?,
        ..meter.estimate()
    }).admit()?.into_plan();
    Ok(())
}

impl ReactiveInstance {
    pub(super) fn execute_comprehension(
        &mut self,
        index: ActivatedNodeIndex,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        let ActivatedTurnStep::Comprehension(control) = &self.plan.steps[index.get() as usize]
        else {
            unreachable!()
        };
        let control = control.clone();
        let result = budget::with_control_work_budget(|| {
            self.with_kernel_turn_plan(index, before, working, |this| {
                this.execute_collection_planned(index, &control, before, working, probe)
            })
        });
        // Lexical payloads have no consumers after this control invocation.
        // This also releases every completed inner allocation on a failed turn.
        for region in &control.locals {
            self.workspace.scratch.discard_payload_write(*region);
        }
        result
    }

    fn execute_collection_planned(
        &mut self,
        index: ActivatedNodeIndex,
        control: &ActivatedComprehensionNode,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: control.artifact_node,
            error,
        };
        let mut meter = ResidentBudgetMeter::default();
        let mut values = Vec::new();
        self.collection_from(control, 0, &mut values, &mut meter, before, working, probe)?;
        let count = values.len();
        let canonical_work = (count as u64)
            .checked_mul((count.max(1).ilog2() as u64 + 1) * 64)
            .and_then(|work| work.checked_add(64))
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        meter.charge_compute_work(canonical_work).map_err(fail)?;
        if let ResidentValueRef::Snapshot([Some(current)]) =
            self.workspace.scratch.read(control.write.region)
        {
            let footprint =
                budget::published_canonical_footprint(&mut meter, current, &self.plan.schemas)
                    .map_err(fail)?;
            meter
                .charge_comparison_work(footprint.encoded_bytes)
                .map_err(fail)?;
        }
        admit_output(count, meter).map_err(fail)?;
        let schema = self
            .plan
            .schemas
            .get(control.output_schema)
            .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
        let (shape_values, data) = match control.kind {
            crate::ComprehensionKind::Matrix => {
                let mech_core::SchemaBody::Matrix { dimensions, .. } = schema.body() else {
                    return Err(fail(ResidentKernelError::InvalidOutput));
                };
                let shape = super::super::matrix_shape_for_extents(schema, &[1, count as u64])
                    .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
                if dimensions.len() != 2 {
                    return Err(fail(ResidentKernelError::InvalidShape));
                }
                (
                    shape.parameter_values().to_vec().into_boxed_slice(),
                    ValueDataDraft::Matrix(values.into_iter().map(Item::draft).collect()),
                )
            }
            crate::ComprehensionKind::Set => {
                let mech_core::SchemaBody::Set { element, .. } = schema.body() else {
                    return Err(fail(ResidentKernelError::InvalidOutput));
                };
                // The core key relation owns float normalization and set identity.
                // Admission above covers sorting and finalization before either runs.
                let compare = |left: &Item, right: &Item| {
                    mech_core::snapshot::compare_key_data(element, &left.data(), &right.data())
                        .expect("validated primitive collection element")
                };
                values.sort_unstable_by(compare);
                values.dedup_by(|left, right| compare(left, right).is_eq());
                (
                    Box::new([]) as Box<[u64]>,
                    ValueDataDraft::Set(values.into_iter().map(Item::draft).collect()),
                )
            }
        };
        let canonical_budget = SnapshotCanonicalizationBudget::new(canonical_work);
        let next = ValueDraft {
            schema: control.output_schema,
            shape_values,
            data,
        }
        .finalize(
            &SnapshotValidationContext::new(&self.plan.schemas)
                .with_canonicalization_budget(&canonical_budget),
        )
        .map_err(|_| fail(ResidentKernelError::InvalidOutput))?;
        let ResidentValueMut::Snapshot([target]) =
            self.workspace.scratch.write(control.write.region)
        else {
            return Err(fail(ResidentKernelError::InvalidOutput));
        };
        let changed = target
            .as_ref()
            .map_or(Ok(true), |old| {
                old.language_eq(&self.plan.schemas, &next, &self.plan.schemas)
                    .map(|equal| !equal)
            })
            .map_err(|_| fail(ResidentKernelError::InvalidOutput))?;
        *target = Some(next);
        set_bit(
            &mut self.workspace.initialized_output_bits,
            index.get() as usize,
        );
        Ok(changed)
    }

    fn collection_from(
        &mut self,
        control: &ActivatedComprehensionNode,
        start: usize,
        values: &mut Vec<Item>,
        meter: &mut ResidentBudgetMeter,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<(), ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: control.artifact_node,
            error,
        };
        for position in start..control.steps.len() {
            meter.charge_compute_work(1).map_err(fail)?;
            match &control.steps[position] {
                ActivatedCollectionStep::Operation { node, work } => {
                    meter.charge_compute_work(*work).map_err(fail)?;
                    self.execute_kernel(*node, before, working, probe)?;
                }
                ActivatedCollectionStep::Filter(source) => {
                    match self.read_location(*source, working) {
                        Some(ResidentValueRef::Bool([1])) => {}
                        Some(ResidentValueRef::Bool([0])) => return Ok(()),
                        _ => return Err(fail(ResidentKernelError::InvalidInput)),
                    }
                }
                ActivatedCollectionStep::Generator { source, pattern } => {
                    let count = self
                        .read_location(*source, working)
                        .and_then(collection_len)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    for ordinal in 0..count {
                        meter.charge_compute_work(1).map_err(fail)?;
                        let item = self
                            .read_location(*source, working)
                            .and_then(|value| collection_item(value, region(*source), ordinal))
                            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                        let matched = match pattern {
                            crate::CollectionPattern::Wildcard => true,
                            crate::CollectionPattern::Equal(source) => {
                                meter.charge_comparison_work(1).map_err(fail)?;
                                let peer = self
                                    .read_location(*source, working)
                                    .and_then(Item::from_scalar)
                                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                                item.equals(peer)
                            }
                            crate::CollectionPattern::Bind { schema: region, .. } => {
                                match (item, self.workspace.scratch.write(*region)) {
                                    (Item::Bool(value), ResidentValueMut::Bool([target])) => {
                                        *target = u8::from(value)
                                    }
                                    (Item::Index(value), ResidentValueMut::Index([target])) => {
                                        *target = value
                                    }
                                    (Item::F64(value), ResidentValueMut::F64([target])) => {
                                        *target = value
                                    }
                                    _ => return Err(fail(ResidentKernelError::InvalidOutput)),
                                }
                                true
                            }
                            _ => return Err(fail(ResidentKernelError::InvalidInput)),
                        };
                        if matched {
                            self.collection_from(
                                control,
                                position + 1,
                                values,
                                meter,
                                before,
                                working,
                                probe,
                            )?;
                        }
                    }
                    return Ok(());
                }
            }
        }
        let item = self
            .read_location(control.yield_value, working)
            .and_then(Item::from_scalar)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        let next = values
            .len()
            .checked_add(1)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        admit_output(next, *meter).map_err(fail)?;
        values
            .try_reserve_exact(1)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        values.push(item);
        Ok(())
    }
}
