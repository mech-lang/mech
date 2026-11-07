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
    publication: Option<PrivatePublication>,
    _nodes: mech_core::ManagedNodeReservation,
    _reservation: mech_core::ManagedMemoryReservation,
}
type PutTicket = crate::native_artifact::values::PreparedValuePut;
enum PrivatePublication {
    Direct(PutTicket),
    Cold(Box<ColdPublications>),
}
// The extra capability is heap-owned only for a tree with real cold aliases.
// Established numeric leaves retain their exact metadata envelope.
const _: () = assert!(
    core::mem::size_of::<Option<PrivatePublication>>() == core::mem::size_of::<Option<PutTicket>>()
);
pub(super) struct ColdPublications {
    direct: Option<PutTicket>,
    entries: Vec<Option<(u32, PutTicket)>>,
    _nodes: mech_core::ManagedNodeReservation,
    _reservation: mech_core::ManagedMemoryReservation,
}
impl ColdPublications {
    pub(super) fn take(
        &mut self,
        index: usize,
        value: u32,
    ) -> Result<PutTicket, NativeProgramExecutionError> {
        let entry = self
            .entries
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| invalid("numeric cold alias lost publication admission"))?;
        if entry.0 != value {
            return Err(invalid("numeric cold alias publication has foreign value"));
        }
        Ok(entry.1)
    }
}
pub(super) enum Field {
    Scalar {
        schema: mech_core::SchemaId,
        value: NativeScalarValue,
    },
    Managed(Value),
}
#[path = "buffer_frames_capture.rs"]
mod capture;
pub(super) use capture::CapturedFrame;
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
                        + match &p.publication {
                            Some(PrivatePublication::Cold(cold)) => cold._nodes.nodes(),
                            _ => 0,
                        }
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
        inputs: &mut [Option<Value>],
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
        for (index, ((field, owner), word)) in
            fields.iter().zip(inputs.iter()).zip(bits).enumerate()
        {
            match field.lane() {
                Lane::Scalar { kind, .. } if owner.is_none() && kind.accepts_bits(*word) => {}
                Lane::Managed { .. }
                    if *word == 0
                        && owner.as_ref().is_some_and(|v| v.schema() == field.schema()) => {}
                Lane::Composite { .. } | Lane::Tagged { .. } if owner.is_none() && *word == 0 => {
                    self.check_copy(
                        layout,
                        layout
                            .fields(operands[index])
                            .ok_or_else(|| invalid("private child has no source lanes"))?,
                        Self::composite_target(layout, field)?,
                    )?;
                }
                _ => return Err(invalid("private frame field representation differs")),
            }
        }
        for (index, ((field, owner), word)) in
            fields.iter().zip(inputs.iter_mut()).zip(bits).enumerate()
        {
            if matches!(field.lane(), Lane::Composite { .. } | Lane::Tagged { .. }) {
                self.copy_fields::<false>(
                    layout,
                    layout.fields(operands[index]).unwrap(),
                    Self::composite_target(layout, field)?,
                )?;
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
                // Admission and complete pack validation precede every take.
                // These temporary owners are otherwise cleared immediately by
                // the caller; original arena roots and outside aliases remain.
                Lane::Managed { at } => inner.owners.values_mut()[at] = owner.take(),
                Lane::Composite { .. } | Lane::Tagged { .. } => unreachable!("handled child span"),
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
    /// Fixed for the owning invocation; the admitted storage never resizes.
    /// No Rust reference to a lane survives the synchronous generated entry.
    pub(super) fn scalar_bank_pointer(
        &mut self,
        layout: &Layout,
    ) -> Result<u64, NativeProgramExecutionError> {
        let expected = Self::word_count(layout)?;
        let Some(inner) = self.inner.as_mut() else {
            // Scalar-state numeric trees can use only ordinary scalar homes
            // and immutable arrays. Their derived frame bank has zero lanes;
            // generated code carries this unused sentinel without reading it.
            return if expected == 0 {
                Ok(0)
            } else {
                Err(invalid("numeric scalar bank missing"))
            };
        };
        if inner.words.values().len() != expected {
            return Err(invalid(
                "numeric scalar bank extent differs from its derived layout",
            ));
        }
        Ok(if expected == 0 {
            0
        } else {
            inner.words.values_mut().as_mut_ptr() as u64
        })
    }
    pub(super) fn current_scalar_binding(
        &self,
        physical: usize,
        kind: mech_engine::native_artifact::NativeScalarKind,
    ) -> bool {
        self.inner
            .as_ref()
            .and_then(|inner| {
                let at = physical.checked_mul(2)?;
                let words = inner.words.values().get(at..at.checked_add(2)?)?;
                Some(words[1] == 1 && kind.accepts_bits(words[0]))
            })
            .unwrap_or(false)
    }
    /// A virtual producer may acquire a physical home only if no earlier
    /// current definition occupies either its payload or validity word.
    pub(super) fn empty_scalar_binding(&self, physical: usize) -> bool {
        self.inner
            .as_ref()
            .and_then(|inner| {
                let at = physical.checked_mul(2)?;
                let words = inner.words.values().get(at..at.checked_add(2)?)?;
                Some(words == [0, 0])
            })
            .unwrap_or(false)
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
            match (from.lane(), to.lane()) {
                (Lane::Tagged { count: a, .. }, Lane::Tagged { count: b, .. }) if a == b => {
                    let ordinal = self.current_tag(*from)?;
                    let source = layout
                        .tagged_variant(*from, ordinal as u32)
                        .ok_or_else(|| invalid("private enum source variant missing"))?;
                    let target = layout
                        .tagged_variant(*to, ordinal as u32)
                        .ok_or_else(|| invalid("private enum target variant missing"))?;
                    self.check_copy(
                        layout,
                        core::slice::from_ref(&source),
                        core::slice::from_ref(&target),
                    )?;
                    continue;
                }
                (Lane::Tagged { .. }, _) | (_, Lane::Tagged { .. }) => {
                    return Err(invalid("private enum copy layouts differ"));
                }
                _ => {}
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
    fn copy_fields<const TAKE: bool>(
        &mut self,
        layout: &Layout,
        source: &[FrameField],
        target: &[FrameField],
    ) -> Result<(), NativeProgramExecutionError> {
        for (from, to) in source.iter().zip(target) {
            if let (Lane::Tagged { .. }, Lane::Tagged { tag, count, .. }) = (from.lane(), to.lane())
            {
                let ordinal = self.current_tag(*from)?;
                let active_from = layout
                    .tagged_variant(*from, ordinal as u32)
                    .ok_or_else(|| invalid("private enum source variant missing"))?;
                let active_to = layout
                    .tagged_variant(*to, ordinal as u32)
                    .ok_or_else(|| invalid("private enum target variant missing"))?;
                // Complete check_copy already staged readiness/type proof for
                // every active leaf. Variant banks are disjoint per value.
                self.copy_fields::<TAKE>(
                    layout,
                    core::slice::from_ref(&active_from),
                    core::slice::from_ref(&active_to),
                )?;
                for inactive in 0..count {
                    if inactive != ordinal {
                        let field = layout
                            .tagged_variant(*to, inactive as u32)
                            .ok_or_else(|| invalid("private enum inactive variant missing"))?;
                        self.clear_fields(layout, core::slice::from_ref(&field))?;
                    }
                }
                let inner = self
                    .inner
                    .as_mut()
                    .ok_or_else(|| invalid("private enum lanes missing"))?;
                inner.words.values_mut()[tag * 2] = ordinal as u64;
                inner.words.values_mut()[tag * 2 + 1] = 1;
                continue;
            }
            if let (Some(a), Some(b)) = (layout.children(*from), layout.children(*to)) {
                self.copy_fields::<TAKE>(layout, a, b)?;
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
                    inner.owners.values_mut()[to] = if TAKE {
                        inner.owners.values_mut()[from].take()
                    } else {
                        inner.owners.values()[from].clone()
                    };
                }
                _ => return Err(invalid("private frame copy lanes differ")),
            }
        }
        Ok(())
    }
    pub(super) fn copy<const TAKE: bool>(
        &mut self,
        layout: &Layout,
        source: NativeMatchValue,
        output: NativeMatchValue,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        // The maintained same-plan layout allocates disjoint recursive lanes
        // for distinct root values. Self-copy always retains the original owner.
        let take_source = TAKE && source != output;
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
        // Complete validation and the original charge precede every take.
        // Scalar readiness and the caller's ordered retirement stay unchanged.
        if take_source {
            self.copy_fields::<true>(layout, source, target)
        } else {
            self.copy_fields::<false>(layout, source, target)
        }
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
            .ok_or_else(|| invalid("private projection lacks child span"))?;
        let source = Self::composite_target(layout, source)?;
        let target = layout
            .fields(output)
            .ok_or_else(|| invalid("private child output lacks lanes"))?;
        work.charge(
            NativeExecutionPhase::Setup,
            amount(Self::field_count(layout, source), 64, 64)?,
        )?;
        self.check_copy(layout, source, target)?;
        self.copy_fields::<false>(layout, source, target)
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
    #[cfg(test)]
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
    /// Slots and schemas are derived from the same validated frame leaves as
    /// the numeric proof. Decoded images rederive and compare this metadata.
    pub(super) fn numeric_owner(&self, at: usize, schema: mech_core::SchemaId) -> Option<&Value> {
        self.inner
            .as_ref()?
            .owners
            .values()
            .get(at)?
            .as_ref()
            .filter(|v| v.schema() == schema)
    }
    pub(super) fn empty_numeric_owner(&self, at: usize) -> bool {
        self.inner
            .as_ref()
            .and_then(|i| i.owners.values().get(at))
            .is_some_and(Option::is_none)
    }
    pub(super) fn project_numeric_owner(
        &self,
        at: usize,
        schema: mech_core::SchemaId,
        work: &mut NativeWorkMeter,
    ) -> Result<Field, NativeProgramExecutionError> {
        work.charge(NativeExecutionPhase::Setup, 64)?;
        self.numeric_owner(at, schema)
            .cloned()
            .map(Field::Managed)
            .ok_or_else(|| invalid("numeric view has no current owner"))
    }
    pub(super) fn take_numeric_owner(
        &mut self,
        at: usize,
        schema: mech_core::SchemaId,
    ) -> Result<Value, NativeProgramExecutionError> {
        if self.numeric_owner(at, schema).is_none() {
            return Err(invalid("numeric handoff lost current carrier owner"));
        }
        self.inner
            .as_mut()
            .and_then(|i| i.owners.values_mut().get_mut(at))
            .and_then(Option::take)
            .ok_or_else(|| invalid("numeric handoff lost current carrier owner"))
    }
    pub(super) fn synchronize_numeric_owner(
        &mut self,
        at: usize,
        schema: mech_core::SchemaId,
        owner: Value,
    ) -> Result<(), NativeProgramExecutionError> {
        if schema != owner.schema() {
            return Err(invalid("numeric managed reconciliation type differs"));
        }
        let slot = self
            .inner
            .as_mut()
            .and_then(|i| i.owners.values_mut().get_mut(at))
            .ok_or_else(|| invalid("numeric managed reconciliation outside frame"))?;
        *slot = Some(owner);
        Ok(())
    }
    pub(super) fn synchronize_scalar_binding(
        &mut self,
        physical: usize,
        kind: mech_engine::native_artifact::NativeScalarKind,
        bits: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        if !kind.accepts_bits(bits) {
            return Err(invalid("numeric reconciliation has invalid scalar bits"));
        }
        let at = physical
            .checked_mul(2)
            .ok_or_else(|| invalid("numeric scalar binding offset overflows"))?;
        let words = self
            .inner
            .as_mut()
            .and_then(|inner| inner.words.values_mut().get_mut(at..at.checked_add(2)?))
            .ok_or_else(|| invalid("numeric scalar binding outside its admitted bank"))?;
        words[0] = bits;
        words[1] = 1;
        Ok(())
    }
    pub(super) fn synchronize_scalar_leaf(
        &mut self,
        field: FrameField,
        bits: u64,
    ) -> Result<(), NativeProgramExecutionError> {
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
    pub(super) fn synchronize_scalar_at(
        &mut self,
        layout: &Layout,
        value: u32,
        ordinal: usize,
        bits: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        let field = layout
            .fields_at(value as usize)
            .and_then(|fields| fields.get(ordinal))
            .ok_or_else(|| invalid("numeric reconciliation outside frame"))?;
        self.synchronize_scalar_leaf(*field, bits)
    }
    pub(super) fn prepare_private(
        &mut self,
        region: usize,
        count: usize,
        append: bool,
        publication: Option<crate::native_artifact::values::PreparedValuePut>,
        cold_aliases: usize,
        memory: &ManagedMemoryBudget,
    ) -> Result<bool, NativeProgramExecutionError> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("private numeric frame lanes missing"))?;
        if inner.private.is_some() {
            return Err(invalid("overlapping private numeric owners"));
        }
        let publication = if cold_aliases == 0 {
            publication.map(PrivatePublication::Direct)
        } else {
            let bytes = Self::cold_publication_bytes(cold_aliases)?;
            let Ok(reservation) = memory.reserve_capacity(bytes) else {
                return Ok(false);
            };
            let Ok(nodes) = memory.reserve_nodes(amount(cold_aliases, 1, 1)?) else {
                return Ok(false);
            };
            let mut entries = Vec::new();
            if entries.try_reserve_exact(cold_aliases).is_err() {
                return Ok(false);
            }
            entries.resize_with(cold_aliases, || None);
            Some(PrivatePublication::Cold(Box::new(ColdPublications {
                direct: publication,
                entries,
                _nodes: nodes,
                _reservation: reservation,
            })))
        };
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
    pub(super) fn cold_publication_bytes(count: usize) -> Result<u64, NativeProgramExecutionError> {
        amount(
            count,
            core::mem::size_of::<Option<(u32, PutTicket)>>() as u64,
            core::mem::size_of::<ColdPublications>() as u64,
        )
    }
    pub(super) fn cold_publication_work(count: usize) -> Result<u64, NativeProgramExecutionError> {
        if count == 0 {
            return Ok(0);
        }
        Self::cold_publication_bytes(count)?
            .checked_mul(8)
            .and_then(|n| n.checked_add(256))
            .ok_or_else(|| invalid("numeric cold alias setup work overflows"))
    }
    pub(super) fn set_private_alias(
        &mut self,
        region: usize,
        index: usize,
        value: u32,
        ticket: PutTicket,
    ) -> Result<(), NativeProgramExecutionError> {
        let Some(PrivatePublication::Cold(cold)) = self
            .inner
            .as_mut()
            .and_then(|inner| inner.private.as_mut())
            .filter(|p| p.region == region)
            .and_then(|p| p.publication.as_mut())
        else {
            return Err(invalid("numeric cold alias has no admitted metadata"));
        };
        let entry = cold
            .entries
            .get_mut(index)
            .filter(|entry| entry.is_none())
            .ok_or_else(|| invalid("numeric cold alias publication index is invalid"))?;
        *entry = Some((value, ticket));
        Ok(())
    }
    pub(super) fn take_private_aliases(
        &mut self,
        region: usize,
    ) -> Result<Option<Box<ColdPublications>>, NativeProgramExecutionError> {
        let private = self
            .inner
            .as_mut()
            .and_then(|inner| inner.private.as_mut())
            .filter(|p| p.region == region)
            .ok_or_else(|| invalid("numeric cold aliases have no active owner bank"))?;
        match private.publication.take() {
            Some(PrivatePublication::Cold(mut cold)) => {
                private.publication = cold.direct.take().map(PrivatePublication::Direct);
                Ok(Some(cold))
            }
            direct => {
                private.publication = direct;
                Ok(None)
            }
        }
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
        Ok(match state.publication {
            Some(PrivatePublication::Direct(ticket)) => Some(ticket),
            Some(PrivatePublication::Cold(cold)) => cold.direct,
            None => None,
        })
    }
    pub(super) fn synchronize_managed(
        &mut self,
        layout: &Layout,
        value: NativeMatchValue,
        ordinal: usize,
        owner: Value,
    ) -> Result<(), NativeProgramExecutionError> {
        self.synchronize_managed_at(layout, value.get(), ordinal, owner)
    }
    pub(super) fn synchronize_managed_at(
        &mut self,
        layout: &Layout,
        value: u32,
        ordinal: usize,
        owner: Value,
    ) -> Result<(), NativeProgramExecutionError> {
        let field = layout
            .fields_at(value as usize)
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
        self.apply_retirement(operations)?;
        Ok(true)
    }
    pub(super) fn has_retirement_backing(&self) -> bool {
        self.inner.is_some()
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(super) fn retirement_is_empty(
        &self,
        recipes: &crate::native_artifact::program::prepare::frame_retirement::FrameRetirements,
        value: u32,
    ) -> bool {
        let Some((_, operations)) = recipes.get(value as usize) else {
            return true;
        };
        let Some(inner) = &self.inner else {
            return false;
        };
        use crate::native_artifact::program::prepare::frame_retirement::Operation;
        operations.iter().all(|op| match op {
            Operation::Scalars(words) => inner.words.values()[words.clone()]
                .iter()
                .all(|&word| word == 0),
            Operation::Managed(at) => inner.owners.values()[*at].is_none(),
        })
    }
    pub(super) fn retire_prepaid(
        &mut self,
        recipes: &crate::native_artifact::program::prepare::frame_retirement::FrameRetirements,
        value: u32,
        paid: &mut crate::native_artifact::work::PaidRetirements<'_>,
    ) -> Result<bool, NativeProgramExecutionError> {
        let Some((charge, operations)) = recipes.get(value as usize) else {
            return Ok(false);
        };
        paid.pay(charge)?;
        self.apply_retirement(operations)?;
        Ok(true)
    }
    fn apply_retirement(
        &mut self,
        operations: &[crate::native_artifact::program::prepare::frame_retirement::Operation],
    ) -> Result<(), NativeProgramExecutionError> {
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
        Ok(())
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "buffer_frame_retirement_tests/mod.rs"]
mod tests;

#[path = "buffer_frames_tagged.rs"]
mod tagged;
