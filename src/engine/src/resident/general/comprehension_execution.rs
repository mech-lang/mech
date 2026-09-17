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

#[derive(Clone, Copy)]
enum PatternItem<'a> {
    Atom,
    Scalar(Item),
    Data(&'a ValueData),
}

impl<'a> PatternItem<'a> {
    fn scalar(self) -> Option<Item> {
        match self {
            Self::Scalar(item) => Some(item),
            Self::Data(data) => Item::from_data(data),
            Self::Atom => None,
        }
    }
    fn child(self, index: usize) -> Option<Self> {
        match self {
            Self::Data(ValueData::Tuple(items)) => items.get(index).map(Self::Data),
            Self::Data(ValueData::Matrix(matrix)) => sequence_item(matrix.elements(), index),
            _ => None,
        }
    }
}

fn sequence_item(
    sequence: mech_core::snapshot::SequenceView<'_>,
    index: usize,
) -> Option<PatternItem<'_>> {
    use mech_core::snapshot::SequenceView;
    Some(match sequence {
        SequenceView::F64(values) => PatternItem::Scalar(Item::F64(values.get(index)?.to_f64())),
        SequenceView::Bool(values) => PatternItem::Scalar(Item::Bool(*values.get(index)?)),
        SequenceView::Index(values) => PatternItem::Scalar(Item::Index(*values.get(index)?)),
        SequenceView::Values(values) => PatternItem::Data(values.get(index)?),
        SequenceView::Unit(count) if (index as u64) < count => PatternItem::Atom,
        _ => return None,
    })
}

fn collection_item(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
    ordinal: usize,
) -> Option<PatternItem<'_>> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        return match value.data() {
            ValueData::Set(set) => set
                .elements()
                .get(ordinal)
                .map(|item| PatternItem::Data(item.data())),
            ValueData::Matrix(matrix) => sequence_item(matrix.elements(), ordinal),
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
            0 => Some(PatternItem::Scalar(Item::Bool(false))),
            1 => Some(PatternItem::Scalar(Item::Bool(true))),
            _ => None,
        },
        ResidentValueRef::Index(value) => {
            Some(PatternItem::Scalar(Item::Index(*value.get(offset)?)))
        }
        ResidentValueRef::F64(value) => Some(PatternItem::Scalar(Item::F64(*value.get(offset)?))),
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
    let admitted_bytes = budget::checked_u64(temporary)?;
    PreparedKernel::new((), budget::resident_cost! {
        output_elements: count,
        output_bytes: admitted_bytes,
        temporary_bytes: admitted_bytes,
        container_bytes: admitted_bytes,
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
        if control.kind == crate::ComprehensionKind::Set {
            meter.charge_comparison_work(canonical_work).map_err(fail)?;
        }
        let target = if control.write.storage == ResidentStorageClass::Constant {
            &self.activation
        } else {
            &self.workspace.scratch
        };
        if let ResidentValueRef::Snapshot([Some(current)]) = target.read(control.write.region) {
            let footprint =
                budget::published_canonical_footprint(&mut meter, current, &self.plan.schemas)
                    .map_err(fail)?;
            meter
                .charge_comparison_work(footprint.encoded_bytes)
                .map_err(fail)?;
        }
        // `with_kernel_turn_plan` owns the one payload scope for this output.
        // Admitting the concrete control plan below charges that enclosing
        // scope; opening another same-region scope here would reserve the
        // materialization peak twice while both reservations are live.
        admit_output(count, meter).map_err(fail)?;
        let next = {
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
            ValueDraft {
                schema: control.output_schema,
                shape_values,
                data,
            }
            .finalize(
                &SnapshotValidationContext::new(&self.plan.schemas)
                    .with_canonicalization_budget(&canonical_budget),
            )
            .map_err(|_| fail(ResidentKernelError::InvalidOutput))
        }?;
        let target = if control.write.storage == ResidentStorageClass::Constant {
            &mut self.activation
        } else {
            &mut self.workspace.scratch
        };
        let changed = match target.read(control.write.region) {
            ResidentValueRef::Snapshot([current]) => current.as_ref().map_or(Ok(true), |old| {
                old.snapshot_eq(&self.plan.schemas, &next, &self.plan.schemas)
                    .map(|equal| !equal)
                    .map_err(|_| ())
            }),
            _ => Err(()),
        };
        let changed = match changed {
            Ok(changed) => changed,
            Err(_) => return Err(fail(ResidentKernelError::InvalidOutput)),
        };
        let ResidentValueMut::Snapshot([target_value]) = target.write(control.write.region) else {
            return Err(fail(ResidentKernelError::InvalidOutput));
        };
        *target_value = Some(next);
        set_bit(
            &mut self.workspace.initialized_output_bits,
            index.get() as usize,
        );
        Ok(changed)
    }

    fn collection_pattern_item<'a>(
        &'a self,
        source: ResidentReadLocation,
        ordinal: usize,
        path: &[usize],
        working: InstanceEpoch,
    ) -> Option<PatternItem<'a>> {
        let mut item = collection_item(
            self.read_location(source, working)?,
            region(source),
            ordinal,
        )?;
        for index in path {
            item = item.child(*index)?;
        }
        Some(item)
    }

    fn match_collection_pattern(
        &mut self,
        source: ResidentReadLocation,
        ordinal: usize,
        pattern: &crate::CollectionPattern<ResidentRegion, ResidentReadLocation>,
        path: &mut [usize; crate::MAX_COLLECTION_PATTERN_DEPTH],
        depth: usize,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentKernelError> {
        meter.charge_compute_work(1 + depth as u64)?;
        match pattern {
            crate::CollectionPattern::Wildcard => Ok(true),
            crate::CollectionPattern::Bind { schema: target, .. } => {
                let item = self
                    .collection_pattern_item(source, ordinal, &path[..depth], working)
                    .and_then(PatternItem::scalar)
                    .ok_or(ResidentKernelError::InvalidInput)?;
                // Partial writes on a failed pattern are private lexical locals.
                // Dominance requires a fresh binding before any later equality read.
                match (item, self.workspace.scratch.write(*target)) {
                    (Item::Bool(value), ResidentValueMut::Bool([target])) => {
                        *target = u8::from(value)
                    }
                    (Item::Index(value), ResidentValueMut::Index([target])) => *target = value,
                    (Item::F64(value), ResidentValueMut::F64([target])) => *target = value,
                    _ => return Err(ResidentKernelError::InvalidOutput),
                }
                Ok(true)
            }
            crate::CollectionPattern::Equal(peer) => {
                meter.charge_comparison_work(1)?;
                let item = self
                    .collection_pattern_item(source, ordinal, &path[..depth], working)
                    .ok_or(ResidentKernelError::InvalidInput)?;
                let peer = self
                    .read_location(*peer, working)
                    .ok_or(ResidentKernelError::InvalidInput)?;
                if let (Some(item), Some(peer)) = (item.scalar(), Item::from_scalar(peer)) {
                    return Ok(item.equals(peer));
                }
                // Atom identity is owned by the exact schema validated at binding.
                Ok(
                    matches!((item, peer), (PatternItem::Atom | PatternItem::Data(ValueData::Atom), ResidentValueRef::Snapshot([Some(peer)])) if matches!(peer.data(), ValueData::Atom)),
                )
            }
            crate::CollectionPattern::Tuple(items) => {
                let Some(PatternItem::Data(ValueData::Tuple(values))) =
                    self.collection_pattern_item(source, ordinal, &path[..depth], working)
                else {
                    return Ok(false);
                };
                if values.len() != items.len() {
                    return Ok(false);
                }
                for (index, item) in items.iter().enumerate() {
                    path[depth] = index;
                    if !self.match_collection_pattern(
                        source,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                let Some(PatternItem::Data(ValueData::Matrix(values))) =
                    self.collection_pattern_item(source, ordinal, &path[..depth], working)
                else {
                    return Ok(false);
                };
                let count = values.elements().len();
                let required = prefix
                    .len()
                    .checked_add(suffix.len())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                if count < required || (rest.is_none() && count != required) {
                    return Ok(false);
                }
                for (index, item) in prefix.iter().enumerate() {
                    path[depth] = index;
                    if !self.match_collection_pattern(
                        source,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                // Binding a middle slice needs admitted composite storage; the
                // current target accepts only an ignored wildcard middle.
                if rest
                    .as_deref()
                    .is_some_and(|rest| !matches!(rest, crate::CollectionPattern::Wildcard))
                {
                    return Err(ResidentKernelError::InvalidInput);
                }
                for (index, item) in suffix.iter().enumerate() {
                    path[depth] = count - suffix.len() + index;
                    if !self.match_collection_pattern(
                        source,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
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
                        let matched = self
                            .match_collection_pattern(
                                *source,
                                ordinal,
                                pattern,
                                &mut [0; crate::MAX_COLLECTION_PATTERN_DEPTH],
                                0,
                                working,
                                meter,
                            )
                            .map_err(fail)?;
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
            .try_reserve(1)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        values.push(item);
        Ok(())
    }
}
