//! Planned reference lanes for nonescaping tuple argument frames.
use super::*;
use mech_engine::native_artifact::{NativeBufferFrames as Layout, NativeMatchValue};

pub(super) struct BufferFrames {
    inner: Option<Box<FrameLanes>>,
}
// Heap-owned only when selected; keep the existing scalar caller layout small.
struct FrameLanes {
    values: NativeTypedStorage<Option<Value>>,
    nodes: mech_core::ManagedNodeReservation,
    _reservation: mech_core::ManagedMemoryReservation,
}
impl BufferFrames {
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
        let values = NativeTypedStorage::new_values(count, limit, memory)?;
        Ok(Self {
            inner: Some(Box::new(FrameLanes {
                values,
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
        self.inner.as_ref().map_or(0, |inner| inner.nodes.nodes())
    }
    pub(super) fn pack(
        &mut self,
        layout: &Layout,
        output: NativeMatchValue,
        inputs: &[Option<Value>],
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let (at, count) = layout
            .slot(output)
            .ok_or_else(|| invalid("private frame output has no lanes"))?;
        if count != inputs.len() || inputs.iter().any(Option::is_none) {
            return Err(invalid("private frame input count differs"));
        }
        work.charge(NativeExecutionPhase::Setup, amount(count, 64, 64)?)?;
        let values = self
            .inner
            .as_mut()
            .map(|inner| &mut inner.values)
            .ok_or_else(|| invalid("private frame lanes missing"))?
            .values_mut();
        values[at..at + count].clone_from_slice(inputs);
        Ok(())
    }
    pub(super) fn copy(
        &mut self,
        layout: &Layout,
        source: NativeMatchValue,
        output: NativeMatchValue,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let (from, count) = layout
            .slot(source)
            .ok_or_else(|| invalid("private frame source has no lanes"))?;
        let (to, output_count) = layout
            .slot(output)
            .ok_or_else(|| invalid("private frame target has no lanes"))?;
        if count != output_count {
            return Err(invalid("private frame copy layouts differ"));
        }
        work.charge(NativeExecutionPhase::Setup, amount(count, 64, 64)?)?;
        let values = self
            .inner
            .as_mut()
            .map(|inner| &mut inner.values)
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        if values.values()[from..from + count]
            .iter()
            .any(Option::is_none)
        {
            return Err(invalid("private frame source is not current"));
        }
        if from != to {
            for index in 0..count {
                let value = values.values()[from + index].clone();
                values.values_mut()[to + index] = value;
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
    ) -> Result<Value, NativeProgramExecutionError> {
        let (at, count) = layout
            .slot(source)
            .ok_or_else(|| invalid("private frame projection has no lanes"))?;
        if ordinal >= count {
            return Err(invalid("private frame field outside layout"));
        }
        work.charge(NativeExecutionPhase::Setup, 64)?;
        self.inner
            .as_ref()
            .map(|inner| &inner.values)
            .and_then(|v| v.values()[at + ordinal].clone())
            .ok_or_else(|| invalid("private frame field is not current"))
    }
    pub(super) fn retire(
        &mut self,
        layout: &Layout,
        value: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<bool, NativeProgramExecutionError> {
        let Some((at, count)) = layout.slot_at(value as usize) else {
            return Ok(false);
        };
        // Children were inserted from prepaid canonical arena/scalar owners.
        // This transfers their retirement credit; it never creates fuel.
        work.charge(NativeExecutionPhase::Setup, amount(count, 32, 32)?)?;
        let values = self
            .inner
            .as_mut()
            .map(|inner| &mut inner.values)
            .ok_or_else(|| invalid("private frame lanes missing"))?;
        values.values_mut()[at..at + count].fill(None);
        Ok(true)
    }
}
