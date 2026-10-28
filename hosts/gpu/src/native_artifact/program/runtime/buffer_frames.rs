//! Planned native words and managed-owner lanes for nonescaping argument frames.
use super::*;
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrames as Layout, NativeMatchValue,
    NativeScalarValue,
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
            inner.nodes.nodes() + inner.private.as_ref().map_or(0, |p| p._nodes.nodes())
        })
    }
    pub(super) fn pack(
        &mut self,
        layout: &Layout,
        output: NativeMatchValue,
        inputs: &[Option<Value>],
        bits: &[u64],
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let fields = layout
            .fields(output)
            .ok_or_else(|| invalid("private frame output has no lanes"))?;
        if fields.len() != inputs.len() || bits.len() != inputs.len() {
            return Err(invalid("private frame input count differs"));
        }
        work.charge(NativeExecutionPhase::Setup, amount(fields.len(), 64, 64)?)?;
        // Validate the complete pack before changing any previous frame owner.
        for ((field, owner), word) in fields.iter().zip(inputs).zip(bits) {
            match field.lane() {
                Lane::Scalar { kind, .. } if owner.is_none() && kind.accepts_bits(*word) => {}
                Lane::Managed { .. }
                    if *word == 0
                        && owner.as_ref().is_some_and(|v| v.schema() == field.schema()) => {}
                _ => return Err(invalid("private frame field representation differs")),
            }
        }
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        for ((field, owner), word) in fields.iter().zip(inputs).zip(bits) {
            match field.lane() {
                Lane::Scalar { at, .. } => {
                    inner.words.values_mut()[at * 2] = *word;
                    inner.words.values_mut()[at * 2 + 1] = 1;
                }
                Lane::Managed { at } => inner.owners.values_mut()[at] = owner.clone(),
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
        work.charge(NativeExecutionPhase::Setup, amount(source.len(), 64, 64)?)?;
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        for (from, to) in source.iter().zip(target) {
            if from.schema() != to.schema() {
                return Err(invalid("private frame copy schemas differ"));
            }
            match (from.lane(), to.lane()) {
                (Lane::Scalar { at, kind }, Lane::Scalar { kind: other, .. })
                    if kind == other
                        && inner.words.values()[at * 2 + 1] == 1
                        && kind.accepts_bits(inner.words.values()[at * 2]) => {}
                (Lane::Managed { at }, Lane::Managed { .. })
                    if inner.owners.values()[at].is_some() => {}
                _ => return Err(invalid("private frame source is not current")),
            }
        }
        for (from, to) in source.iter().zip(target) {
            match (from.lane(), to.lane()) {
                (Lane::Scalar { at: from, .. }, Lane::Scalar { at: to, .. }) => {
                    let bits = inner.words.values()[from * 2];
                    inner.words.values_mut()[to * 2] = bits;
                    inner.words.values_mut()[to * 2 + 1] = 1;
                }
                (Lane::Managed { at: from }, Lane::Managed { at: to }) => {
                    let value = inner.owners.values()[from].clone();
                    inner.owners.values_mut()[to] = value;
                }
                _ => unreachable!("validated frame layout"),
            }
        }
        Ok(())
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
            core::mem::size_of::<Option<mech_core::snapshot::PrivateUnsignedMatrix>>() as u64,
            core::mem::size_of::<PrivateNumeric>() as u64,
        )?;
        let Ok(reservation) = memory.reserve_capacity(bytes) else {
            return Ok(false);
        };
        let Ok(nodes) = memory.reserve_nodes(count as u64 + 1) else {
            return Ok(false);
        };
        let mut arrays = Vec::new();
        if arrays.try_reserve_exact(count).is_err() {
            return Ok(false);
        }
        arrays.resize_with(count, || None);
        inner.private = Some(PrivateNumeric {
            region,
            arrays,
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
        layout: &Layout,
        value: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<bool, NativeProgramExecutionError> {
        let Some(fields) = layout.fields_at(value as usize) else {
            return Ok(false);
        };
        // Retirement consumes existing owner credit and invalidates word visits.
        work.charge(NativeExecutionPhase::Setup, amount(fields.len(), 32, 32)?)?;
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        for field in fields {
            match field.lane() {
                Lane::Scalar { at, .. } => inner.words.values_mut()[at * 2..at * 2 + 2].fill(0),
                Lane::Managed { at } => inner.owners.values_mut()[at] = None,
            }
        }
        Ok(true)
    }
}
