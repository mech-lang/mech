//! Same-layout immutable transfer of private native fields, never reboxing a
//! source tuple or copying an array payload into a separate worker heap.
use super::*;

enum Leaf {
    Word(FrameField, u64),
    Owner(FrameField, Value),
    Tag(FrameField, usize),
}
pub(in crate::native_artifact::program::runtime) struct CapturedFrame {
    slot: u32,
    layout_address: usize,
    leaves: Vec<Leaf>,
    _nodes: mech_core::ManagedNodeReservation,
    _memory: mech_core::ManagedMemoryReservation,
}
impl BufferFrames {
    pub(in crate::native_artifact::program::runtime) fn capture_frame(
        &self,
        layout: &Layout,
        slot: u32,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<std::sync::Arc<CapturedFrame>, NativeProgramExecutionError> {
        // The whole static layout bounds the recipe scan, including inactive
        // variants. Fund that scan before deriving this capture's exact bound.
        work.charge(
            NativeExecutionPhase::Setup,
            amount(layout.elements(), 64, 256)?,
        )?;
        let fields = layout
            .fields_at(slot as usize)
            .ok_or_else(|| invalid("worker frame capture has no same-plan lanes"))?;
        let count = Self::field_count(layout, fields);
        let arc_bytes = std::alloc::Layout::new::<[usize; 2]>()
            .extend(std::alloc::Layout::new::<CapturedFrame>())
            .map_err(|_| invalid("worker capture Arc layout"))?
            .0
            .pad_to_align()
            .size() as u64;
        let bytes = amount(count, core::mem::size_of::<Leaf>() as u64, arc_bytes)?;
        let reservation = memory.reserve_capacity(bytes)?;
        let nodes = memory.reserve_nodes(amount(count, 1, 2)?)?;
        let mut leaves = Vec::new();
        leaves
            .try_reserve_exact(count)
            .map_err(|_| invalid("worker field capture allocation"))?;
        leaves = super::super::collections::admitted_capacity(
            leaves,
            count,
            "worker field capture capacity",
        )?;
        self.capture_fields(layout, fields, &mut leaves)?;
        Ok(std::sync::Arc::new(CapturedFrame {
            slot,
            layout_address: layout as *const Layout as usize,
            leaves,
            _nodes: nodes,
            _memory: reservation,
        }))
    }
    fn capture_fields(
        &self,
        layout: &Layout,
        fields: &[FrameField],
        to: &mut Vec<Leaf>,
    ) -> Result<(), NativeProgramExecutionError> {
        for field in fields {
            match field.lane() {
                Lane::Scalar { kind, .. } => {
                    let Some((None, word)) = self.current_leaf(*field) else {
                        return Err(invalid("worker scalar capture is not current"));
                    };
                    if !kind.accepts_bits(word) {
                        return Err(invalid("worker scalar capture bits differ"));
                    }
                    to.push(Leaf::Word(*field, word));
                }
                Lane::Managed { .. } => {
                    let Some((Some(value), 0)) = self.current_leaf(*field) else {
                        return Err(invalid("worker owner capture is not current"));
                    };
                    if value.schema() != field.schema() {
                        return Err(invalid("worker owner capture schema differs"));
                    }
                    to.push(Leaf::Owner(*field, value.clone()));
                }
                Lane::Composite { .. } => self.capture_fields(
                    layout,
                    layout
                        .children(*field)
                        .ok_or_else(|| invalid("worker composite capture lacks children"))?,
                    to,
                )?,
                Lane::Tagged { .. } => {
                    let ordinal = self.current_tag(*field)?;
                    to.push(Leaf::Tag(*field, ordinal));
                    self.capture_fields(
                        layout,
                        core::slice::from_ref(
                            &layout
                                .tagged_variant(*field, ordinal as u32)
                                .ok_or_else(|| invalid("worker tagged capture lacks variant"))?,
                        ),
                        to,
                    )?;
                }
            }
        }
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn import_frame(
        &mut self,
        layout: &Layout,
        slot: u32,
        capture: &CapturedFrame,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        work.charge(
            NativeExecutionPhase::Setup,
            amount(layout.elements(), 64, 256)?,
        )?;
        if slot != capture.slot || capture.layout_address != layout as *const Layout as usize {
            return Err(invalid("worker private capture names a foreign slot"));
        }
        let fields = layout
            .fields_at(slot as usize)
            .ok_or_else(|| invalid("worker captured frame has no target lanes"))?;
        let mut leaves = capture.leaves.iter();
        self.validate_capture(layout, fields, &mut leaves)?;
        if leaves.next().is_some() {
            return Err(invalid("worker captured frame has extra leaves"));
        }
        // Complete schema, readiness and recipe checks precede all writes.
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| invalid("worker private lanes missing"))?;
        for leaf in &capture.leaves {
            match leaf {
                Leaf::Word(field, word) => {
                    let Lane::Scalar { at, .. } = field.lane() else {
                        unreachable!()
                    };
                    inner.words.values_mut()[at * 2] = *word;
                    inner.words.values_mut()[at * 2 + 1] = 1;
                }
                Leaf::Owner(field, value) => {
                    let Lane::Managed { at } = field.lane() else {
                        unreachable!()
                    };
                    inner.owners.values_mut()[at] = Some(value.clone());
                }
                Leaf::Tag(field, ordinal) => {
                    let Lane::Tagged { tag, .. } = field.lane() else {
                        unreachable!()
                    };
                    inner.words.values_mut()[tag * 2] = *ordinal as u64;
                    inner.words.values_mut()[tag * 2 + 1] = 1;
                }
            }
        }
        Ok(())
    }
    fn validate_capture<'a>(
        &self,
        layout: &Layout,
        fields: &[FrameField],
        leaves: &mut impl Iterator<Item = &'a Leaf>,
    ) -> Result<(), NativeProgramExecutionError> {
        let inner = self
            .inner
            .as_ref()
            .ok_or_else(|| invalid("worker private target lanes missing"))?;
        for field in fields {
            if let Lane::Composite { .. } = field.lane() {
                self.validate_capture(
                    layout,
                    layout
                        .children(*field)
                        .ok_or_else(|| invalid("worker private target span missing"))?,
                    leaves,
                )?;
                continue;
            }
            let leaf = leaves
                .next()
                .ok_or_else(|| invalid("worker captured frame lacks a leaf"))?;
            let same = |other: &FrameField| {
                other.schema() == field.schema() && other.lane() == field.lane()
            };
            match (field.lane(), leaf) {
                (Lane::Scalar { at, kind }, Leaf::Word(other, word))
                    if same(other)
                        && kind.accepts_bits(*word)
                        && inner.words.values()[at * 2 + 1] == 0 => {}
                (Lane::Managed { at }, Leaf::Owner(other, value))
                    if same(other)
                        && value.schema() == field.schema()
                        && inner.owners.values()[at].is_none() => {}
                (Lane::Tagged { tag, count, .. }, Leaf::Tag(other, ordinal))
                    if same(other)
                        && *ordinal < count
                        && inner.words.values()[tag * 2 + 1] == 0 =>
                {
                    self.validate_capture(
                        layout,
                        core::slice::from_ref(
                            &layout
                                .tagged_variant(*field, *ordinal as u32)
                                .ok_or_else(|| invalid("worker private target variant missing"))?,
                        ),
                        leaves,
                    )?;
                }
                _ => {
                    return Err(invalid(
                        "worker captured frame differs from same-plan private fields",
                    ));
                }
            }
        }
        Ok(())
    }
}

const _: fn() = || {
    fn check<T: Send + Sync>() {}
    check::<CapturedFrame>();
};
