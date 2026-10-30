//! Planned native words and managed-owner lanes for nonescaping argument frames.
use super::*;
use mech_engine::native_artifact::{
    NativeBufferFrameField as FrameField, NativeBufferFrameLane as Lane,
    NativeBufferFrames as Layout, NativeMatchValue, NativeScalarValue,
};

pub(super) struct BufferFrames {
    inner: Option<Box<FrameLanes>>,
}
// Heap-owned only when selected; scalar-only callers retain their old layout.
struct FrameLanes {
    private: Option<PrivateNumeric>,
    owners: NativeTypedStorage<Option<Value>>,
    // Each scalar has a payload and a current-visit validity word. Its exact
    // kind and schema are immutable prepared metadata, never runtime tags.
    words: NativeWordStorage,
    nodes: mech_core::ManagedNodeReservation,
    _reservation: mech_core::ManagedMemoryReservation,
}
struct PrivateNumeric {
    region: usize,
    arrays: Vec<Option<mech_core::snapshot::PrivateUnsignedMatrix>>,
    append: Option<Vec<Option<mech_core::snapshot::PrivateUnsignedAppendPreparation>>>,
    publication: Option<crate::native_artifact::values::PreparedValuePut>,
    _nodes: mech_core::ManagedNodeReservation,
    _reservation: mech_core::ManagedMemoryReservation,
}
pub(super) enum Field {
    Scalar {
        schema: mech_core::SchemaId,
        value: NativeScalarValue,
    },
    Managed(Value),
}
impl BufferFrames {
    fn word_count(layout: &Layout) -> Result<usize, NativeProgramExecutionError> {
        layout
            .scalar_fields()
            .checked_mul(2)
            .ok_or_else(|| invalid("private scalar frame size overflow"))
    }
    pub(super) fn raw_bytes(layout: &Layout) -> Result<u64, NativeProgramExecutionError> {
        amount(
            layout.managed_fields(),
            core::mem::size_of::<Option<Value>>() as u64,
            0,
        )?
        .checked_add(transport_bytes(Self::word_count(layout)?)?)
        .ok_or_else(|| invalid("private frame storage overflow"))
    }
    pub(super) fn admission_bytes(layout: &Layout) -> Result<u64, NativeProgramExecutionError> {
        if layout.elements() == 0 {
            return Ok(0);
        }
        NativeTypedStorage::<Option<Value>>::admission_bytes(layout.managed_fields())?
            .checked_add(NativeWordStorage::admission_bytes(Self::word_count(
                layout,
            )?)?)
            .ok_or_else(|| invalid("private frame admission overflow"))
    }
    pub(super) fn new(
        layout: &Layout,
        limit: u64,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeProgramExecutionError> {
        let count = layout.elements();
        if count == 0 {
            return Ok(Self { inner: None });
        }
        work.charge(
            NativeExecutionPhase::Setup,
            amount(
                count,
                32,
                128 + core::mem::size_of::<FrameLanes>() as u64 * 8,
            )?,
        )?;
        let reservation = memory.reserve_capacity(Self::metadata_bytes(layout))?;
        let nodes = memory.reserve_nodes(Self::metadata_nodes(layout)?)?;
        let owners = NativeTypedStorage::new_values(layout.managed_fields(), limit, memory)?;
        let words = NativeWordStorage::new(Self::word_count(layout)?, limit, Some(memory))?;
        Ok(Self {
            inner: Some(Box::new(FrameLanes {
                private: None,
                owners,
                words,
                nodes,
                _reservation: reservation,
            })),
        })
    }
    pub(super) fn metadata_bytes(layout: &Layout) -> u64 {
        if layout.elements() == 0 {
            0
        } else {
            core::mem::size_of::<FrameLanes>() as u64
        }
    }
    pub(super) fn metadata_nodes(layout: &Layout) -> Result<u64, NativeProgramExecutionError> {
        if layout.elements() == 0 {
            Ok(0)
        } else {
            amount(layout.elements(), 1, 1)
        }
    }
    pub(super) fn retained_nodes(&self) -> u64 {
        self.inner.as_ref().map_or(0, |inner| {
            inner.nodes.nodes()
                + inner.private.as_ref().map_or(0, |p| {
                    p._nodes.nodes()
                        + p.arrays
                            .iter()
                            .flatten()
                            .map(|a| a.private_metadata_nodes())
                            .sum::<u64>()
                        + p.append
                            .iter()
                            .flatten()
                            .flatten()
                            .map(|a| a.retained_nodes())
                            .sum::<u64>()
                })
        })
    }
    pub(super) fn pack(
        &mut self,
        layout: &Layout,
        output: NativeMatchValue,
        operands: &[NativeMatchValue],
        inputs: &[Option<Value>],
        bits: &[u64],
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let fields = layout
            .fields(output)
            .ok_or_else(|| invalid("private frame output has no lanes"))?;
        if fields.len() != inputs.len()
            || bits.len() != inputs.len()
            || operands.len() != inputs.len()
        {
            return Err(invalid("private frame input count differs"));
        }
        work.charge(
            NativeExecutionPhase::Setup,
            amount(Self::field_count(layout, fields), 64, 64)?,
        )?;
        // Validate the complete pack before changing any previous frame owner.
        for (index, ((field, owner), word)) in fields.iter().zip(inputs).zip(bits).enumerate() {
            match field.lane() {
                Lane::Scalar { kind, .. } if owner.is_none() && kind.accepts_bits(*word) => {}
                Lane::Managed { .. }
                    if *word == 0
                        && owner.as_ref().is_some_and(|v| v.schema() == field.schema()) => {}
                Lane::Composite { .. } if owner.is_none() && *word == 0 => {
                    self.check_copy(
                        layout,
                        layout
                            .fields(operands[index])
                            .ok_or_else(|| invalid("private child has no source lanes"))?,
                        layout.children(*field).unwrap(),
                    )?;
                }
                _ => return Err(invalid("private frame field representation differs")),
            }
        }
        for (index, ((field, owner), word)) in fields.iter().zip(inputs).zip(bits).enumerate() {
            if let Some(children) = layout.children(*field) {
                self.copy_fields(layout, layout.fields(operands[index]).unwrap(), children)?;
                continue;
            }
            let inner = self
                .inner
                .as_mut()
                .ok_or_else(|| invalid("private frame lanes missing"))?;
            match field.lane() {
                Lane::Scalar { at, .. } => {
                    inner.words.values_mut()[at * 2] = *word;
                    inner.words.values_mut()[at * 2 + 1] = 1;
                }
                Lane::Managed { at } => inner.owners.values_mut()[at] = owner.clone(),
                Lane::Composite { .. } => unreachable!("handled child span"),
            }
        }
        Ok(())
    }
    fn field_count(layout: &Layout, fields: &[FrameField]) -> usize {
        fields
            .iter()
            .map(|f| {
                1 + layout
                    .children(*f)
                    .map_or(0, |children| Self::field_count(layout, children))
            })
            .sum()
    }
    pub(super) fn current_leaf(&self, field: FrameField) -> Option<(Option<&Value>, u64)> {
        let inner = self.inner.as_ref()?;
        match field.lane() {
            Lane::Scalar { at, kind }
                if inner.words.values()[at * 2 + 1] == 1
                    && kind.accepts_bits(inner.words.values()[at * 2]) =>
            {
                Some((None, inner.words.values()[at * 2]))
            }
            Lane::Managed { at } => Some((Some(inner.owners.values()[at].as_ref()?), 0)),
            _ => None,
        }
    }
    fn check_copy(
        &self,
        layout: &Layout,
        source: &[FrameField],
        target: &[FrameField],
    ) -> Result<(), NativeProgramExecutionError> {
        if source.len() != target.len() {
            return Err(invalid("private frame copy layouts differ"));
        }
        for (from, to) in source.iter().zip(target) {
            if from.schema() != to.schema() {
                return Err(invalid("private frame copy schemas differ"));
            }
            match (layout.children(*from), layout.children(*to)) {
                (Some(a), Some(b)) => self.check_copy(layout, a, b)?,
                (None, None) => {
                    let same = match (from.lane(), to.lane()) {
                        (Lane::Scalar { kind: a, .. }, Lane::Scalar { kind: b, .. }) => a == b,
                        (Lane::Managed { .. }, Lane::Managed { .. }) => true,
                        _ => false,
                    };
                    if !same || self.current_leaf(*from).is_none() {
                        return Err(invalid("private frame source is not current"));
                    }
                }
                _ => return Err(invalid("private frame source is not current")),
            }
        }
        Ok(())
    }
    fn copy_fields(
        &mut self,
        layout: &Layout,
        source: &[FrameField],
        target: &[FrameField],
    ) -> Result<(), NativeProgramExecutionError> {
        for (from, to) in source.iter().zip(target) {
            if let (Some(a), Some(b)) = (layout.children(*from), layout.children(*to)) {
                self.copy_fields(layout, a, b)?;
                continue;
            }
            let inner = self
                .inner
                .as_mut()
                .ok_or_else(|| invalid("private frame lanes missing"))?;
            match (from.lane(), to.lane()) {
                (Lane::Scalar { at: from, .. }, Lane::Scalar { at: to, .. }) => {
                    inner.words.values_mut()[to * 2] = inner.words.values()[from * 2];
                    inner.words.values_mut()[to * 2 + 1] = 1;
                }
                (Lane::Managed { at: from }, Lane::Managed { at: to }) => {
                    inner.owners.values_mut()[to] = inner.owners.values()[from].clone();
                }
                _ => return Err(invalid("private frame copy lanes differ")),
            }
        }
        Ok(())
    }
    pub(super) fn copy(
        &mut self,
        layout: &Layout,
        source: NativeMatchValue,
        output: NativeMatchValue,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let source = layout
            .fields(source)
            .ok_or_else(|| invalid("private frame source has no lanes"))?;
        let target = layout
            .fields(output)
            .ok_or_else(|| invalid("private frame target has no lanes"))?;
        if source.len() != target.len() {
            return Err(invalid("private frame copy layouts differ"));
        }
        work.charge(
            NativeExecutionPhase::Setup,
            amount(Self::field_count(layout, source), 64, 64)?,
        )?;
        self.check_copy(layout, source, target)?;
        self.copy_fields(layout, source, target)
    }
    pub(super) fn project_composite(
        &mut self,
        layout: &Layout,
        source: NativeMatchValue,
        ordinal: usize,
        output: NativeMatchValue,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let source = layout
            .fields(source)
            .and_then(|f| f.get(ordinal))
            .and_then(|f| layout.children(*f))
            .ok_or_else(|| invalid("private projection lacks child span"))?;
        let target = layout
            .fields(output)
            .ok_or_else(|| invalid("private child output lacks lanes"))?;
        work.charge(
            NativeExecutionPhase::Setup,
            amount(Self::field_count(layout, source), 64, 64)?,
        )?;
        self.check_copy(layout, source, target)?;
        self.copy_fields(layout, source, target)
    }
    pub(super) fn project(
        &self,
        layout: &Layout,
        source: NativeMatchValue,
        ordinal: usize,
        work: &mut NativeWorkMeter,
    ) -> Result<Field, NativeProgramExecutionError> {
        let field = layout
            .fields(source)
            .and_then(|f| f.get(ordinal))
            .ok_or_else(|| invalid("private frame field outside layout"))?;
        work.charge(NativeExecutionPhase::Setup, 64)?;
        let inner = self
            .inner
            .as_ref()
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        match field.lane() {
            Lane::Scalar { at, kind } if inner.words.values()[at * 2 + 1] == 1 => {
                let value = NativeScalarValue::from_bits(kind, inner.words.values()[at * 2])
                    .ok_or_else(|| invalid("private scalar field has invalid bits"))?;
                Ok(Field::Scalar {
                    schema: field.schema(),
                    value,
                })
            }
            Lane::Managed { at } => inner.owners.values()[at]
                .clone()
                .map(Field::Managed)
                .ok_or_else(|| invalid("private frame field is not current")),
            _ => Err(invalid("private frame field is not current")),
        }
    }
    pub(super) fn managed_owner(
        &self,
        layout: &Layout,
        value: NativeMatchValue,
        ordinal: usize,
    ) -> Option<&Value> {
        let Lane::Managed { at } = layout.fields(value)?.get(ordinal)?.lane() else {
            return None;
        };
        self.inner.as_ref()?.owners.values().get(at)?.as_ref()
    }
    /// Transfer this invocation's carrier pin to its selected consuming store.
    /// Other frame fields, arena roots and external aliases remain real owners;
    /// core's exclusive handoff decides whether backing can actually be reused.
    pub(super) fn take_managed_owner(
        &mut self,
        layout: &Layout,
        value: NativeMatchValue,
        ordinal: usize,
    ) -> Result<Value, NativeProgramExecutionError> {
        let Lane::Managed { at } = layout
            .fields(value)
            .and_then(|f| f.get(ordinal))
            .ok_or_else(|| invalid("numeric handoff outside carrier"))?
            .lane()
        else {
            return Err(invalid("numeric handoff has scalar field"));
        };
        self.inner
            .as_mut()
            .and_then(|i| i.owners.values_mut().get_mut(at))
            .and_then(Option::take)
            .ok_or_else(|| invalid("numeric handoff lost current carrier owner"))
    }
    pub(super) fn synchronize_scalar(
        &mut self,
        layout: &Layout,
        value: NativeMatchValue,
        ordinal: usize,
        bits: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        let field = layout
            .fields(value)
            .and_then(|fields| fields.get(ordinal))
            .ok_or_else(|| invalid("numeric reconciliation outside frame"))?;
        let Lane::Scalar { at, kind } = field.lane() else {
            return Err(invalid("numeric reconciliation is not scalar"));
        };
        if !kind.accepts_bits(bits) {
            return Err(invalid("numeric reconciliation has invalid scalar bits"));
        }
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("numeric reconciliation has no lanes"))?;
        inner.words.values_mut()[at * 2] = bits;
        inner.words.values_mut()[at * 2 + 1] = 1;
        Ok(())
    }
    pub(super) fn prepare_private(
        &mut self,
        region: usize,
        count: usize,
        append: bool,
        publication: Option<crate::native_artifact::values::PreparedValuePut>,
        memory: &ManagedMemoryBudget,
    ) -> Result<bool, NativeProgramExecutionError> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private numeric frame lanes missing"))?;
        if inner.private.is_some() {
            return Err(invalid("overlapping private numeric owners"));
        }
        let bytes = amount(
            count,
            (core::mem::size_of::<Option<mech_core::snapshot::PrivateUnsignedMatrix>>()
                + if append {
                    core::mem::size_of::<
                        Option<mech_core::snapshot::PrivateUnsignedAppendPreparation>,
                    >()
                } else {
                    0
                }) as u64,
            core::mem::size_of::<PrivateNumeric>() as u64,
        )?;
        let Ok(reservation) = memory.reserve_capacity(bytes) else {
            return Ok(false);
        };
        let Ok(nodes) = memory.reserve_nodes(count as u64 + 1 + u64::from(append)) else {
            return Ok(false);
        };
        let mut arrays = Vec::new();
        if arrays.try_reserve_exact(count).is_err() {
            return Ok(false);
        }
        arrays.resize_with(count, || None);
        let append = if append {
            let mut values = Vec::new();
            if values.try_reserve_exact(count).is_err() {
                return Ok(false);
            }
            values.resize_with(count, || None);
            Some(values)
        } else {
            None
        };
        inner.private = Some(PrivateNumeric {
            region,
            arrays,
            append,
            publication,
            _nodes: nodes,
            _reservation: reservation,
        });
        Ok(true)
    }
    pub(super) fn private_array_mut(
        &mut self,
        region: usize,
        array: usize,
    ) -> Result<&mut Option<mech_core::snapshot::PrivateUnsignedMatrix>, NativeProgramExecutionError>
    {
        self.inner
            .as_mut()
            .and_then(|i| i.private.as_mut())
            .filter(|p| p.region == region)
            .and_then(|p| p.arrays.get_mut(array))
            .ok_or_else(|| invalid("private numeric array outside active region"))
    }
    pub(super) fn append_preparation_mut(
        &mut self,
        region: usize,
        array: usize,
    ) -> Result<
        &mut Option<mech_core::snapshot::PrivateUnsignedAppendPreparation>,
        NativeProgramExecutionError,
    > {
        self.inner
            .as_mut()
            .and_then(|i| i.private.as_mut())
            .filter(|p| p.region == region)
            .and_then(|p| p.append.as_mut())
            .and_then(|p| p.get_mut(array))
            .ok_or_else(|| invalid("append preparation outside active numeric region"))
    }
    pub(super) fn finish_private(
        &mut self,
        region: usize,
    ) -> Result<Option<crate::native_artifact::values::PreparedValuePut>, NativeProgramExecutionError>
    {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private numeric lanes missing"))?;
        let state = inner
            .private
            .take()
            .ok_or_else(|| invalid("private numeric owners missing"))?;
        if state.region != region || state.arrays.iter().any(Option::is_some) {
            return Err(invalid("private numeric owners not synchronized"));
        }
        Ok(state.publication)
    }
    pub(super) fn synchronize_managed(
        &mut self,
        layout: &Layout,
        value: NativeMatchValue,
        ordinal: usize,
        owner: Value,
    ) -> Result<(), NativeProgramExecutionError> {
        let field = layout
            .fields(value)
            .and_then(|f| f.get(ordinal))
            .ok_or_else(|| invalid("numeric managed reconciliation outside frame"))?;
        let Lane::Managed { at } = field.lane() else {
            return Err(invalid("numeric reconciliation is not managed"));
        };
        if field.schema() != owner.schema() {
            return Err(invalid("numeric managed reconciliation type differs"));
        }
        self.inner
            .as_mut()
            .ok_or_else(|| invalid("numeric managed reconciliation has no lanes"))?
            .owners
            .values_mut()[at] = Some(owner);
        Ok(())
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(super) fn test_scalar_bits(
        &self,
        layout: &Layout,
        value: NativeMatchValue,
        ordinal: usize,
    ) -> Option<u64> {
        let Lane::Scalar { at, kind } = layout.fields(value)?.get(ordinal)?.lane() else {
            return None;
        };
        let words = self.inner.as_ref()?.words.values();
        (words[at * 2 + 1] == 1 && kind.accepts_bits(words[at * 2])).then_some(words[at * 2])
    }
    pub(super) fn retire(
        &mut self,
        recipes: &crate::native_artifact::program::prepare::frame_retirement::FrameRetirements,
        value: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<bool, NativeProgramExecutionError> {
        let Some((charge, operations)) = recipes.get(value as usize) else {
            return Ok(false);
        };
        // Retirement consumes existing owner credit and invalidates word visits.
        work.charge(NativeExecutionPhase::Setup, charge)?;
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        use crate::native_artifact::program::prepare::frame_retirement::Operation;
        for operation in operations {
            match operation {
                Operation::Scalars(words) => inner.words.values_mut()[words.clone()].fill(0),
                Operation::Managed(at) => inner.owners.values_mut()[*at] = None,
            }
        }
        Ok(true)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "buffer_frame_retirement_tests.rs"]
mod tests;
