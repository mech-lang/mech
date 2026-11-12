//! Checked current tags and disjoint private variant banks. No canonical enum.
use super::*;
use mech_engine::native_artifact::{
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};

impl BufferFrames {
    pub(in crate::native_artifact::program::runtime) fn current_tag(
        &self,
        field: FrameField,
    ) -> Result<usize, NativeProgramExecutionError> {
        let Lane::Tagged { tag, count, .. } = field.lane() else {
            return Err(invalid("private frame is not tagged"));
        };
        let words = self
            .inner
            .as_ref()
            .ok_or_else(|| invalid("private enum lanes missing"))?
            .words
            .values();
        if words[tag * 2 + 1] != 1 {
            return Err(invalid("private enum tag is not current"));
        }
        let ordinal = usize::try_from(words[tag * 2])
            .map_err(|_| invalid("private enum tag exceeds host"))?;
        if ordinal >= count {
            return Err(invalid("private enum tag exceeds variants"));
        }
        Ok(ordinal)
    }

    pub(super) fn composite_target<'a>(
        layout: &'a Layout,
        field: &'a FrameField,
    ) -> Result<&'a [FrameField], NativeProgramExecutionError> {
        match field.lane() {
            Lane::Tagged { .. } => Ok(core::slice::from_ref(field)),
            Lane::Composite { .. } => layout
                .children(*field)
                .ok_or_else(|| invalid("private child span missing")),
            _ => Err(invalid("private child is not a structural span")),
        }
    }

    pub(super) fn clear_fields(
        &mut self,
        layout: &Layout,
        fields: &[FrameField],
    ) -> Result<(), NativeProgramExecutionError> {
        for field in fields {
            if let Some(children) = layout.children(*field) {
                self.clear_fields(layout, children)?;
            }
            let inner = self
                .inner
                .as_mut()
                .ok_or_else(|| invalid("private frame lanes missing"))?;
            match field.lane() {
                Lane::Scalar { at, .. } => inner.words.values_mut()[at * 2..at * 2 + 2].fill(0),
                Lane::Tagged { tag, .. } => inner.words.values_mut()[tag * 2..tag * 2 + 2].fill(0),
                Lane::Managed { at } => inner.owners.values_mut()[at] = None,
                Lane::Composite { .. } => {}
            }
        }
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime) fn enum_pack(
        &mut self,
        layout: &Layout,
        output: NativeMatchValue,
        payload: NativeMatchValue,
        ordinal: u32,
        owner: &mut Option<Value>,
        word: u64,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeProgramExecutionError> {
        let root = layout
            .fields(output)
            .and_then(|f| match f {
                [root] => Some(*root),
                _ => None,
            })
            .ok_or_else(|| invalid("private enum root missing"))?;
        let Lane::Tagged { tag, count, .. } = root.lane() else {
            return Err(invalid("private enum root is not tagged"));
        };
        let ordinal = ordinal as usize;
        let target = layout
            .tagged_variant(root, ordinal as u32)
            .ok_or_else(|| invalid("private enum variant missing"))?;
        work.charge(
            NativeExecutionPhase::Setup,
            amount(
                Self::field_count(layout, core::slice::from_ref(&root)),
                64,
                128,
            )?,
        )?;
        // Validate all active source lanes before changing an old destination.
        // Publishing the ready tag is the final write after inactive cleanup.
        match target.lane() {
            Lane::Composite { .. } => {
                if owner.is_some() || word != 0 {
                    return Err(invalid("private enum payload has canonical bits"));
                }
                let source = layout
                    .fields(payload)
                    .ok_or_else(|| invalid("private enum payload is not private"))?;
                let target_fields = Self::composite_target(layout, &target)?;
                self.check_copy(layout, source, target_fields)?;
                self.copy_fields::<false>(layout, source, target_fields)?;
            }
            Lane::Managed { at } => {
                if word != 0
                    || owner
                        .as_ref()
                        .is_none_or(|value| value.schema() != target.schema())
                {
                    return Err(invalid("private enum managed payload differs"));
                }
                self.inner
                    .as_mut()
                    .ok_or_else(|| invalid("private enum lanes missing"))?
                    .owners
                    .values_mut()[at] = owner.take();
            }
            Lane::Scalar { at, kind } => {
                if owner.is_some() || !kind.accepts_bits(word) {
                    return Err(invalid("private enum scalar payload differs"));
                }
                let inner = self
                    .inner
                    .as_mut()
                    .ok_or_else(|| invalid("private enum lanes missing"))?;
                inner.words.values_mut()[at * 2] = word;
                inner.words.values_mut()[at * 2 + 1] = 1;
            }
            Lane::Tagged { .. } => return Err(invalid("nested tagged enum payload unsupported")),
        }
        for inactive in 0..count {
            if inactive != ordinal {
                let field = layout
                    .tagged_variant(root, inactive as u32)
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
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime) fn observe(
        &mut self,
        layout: &Layout,
        source: NativeMatchValue,
        path: &[Step],
        observation: Observation,
        output: NativeMatchValue,
        work: &mut NativeWorkMeter,
    ) -> Result<Option<Field>, NativeProgramExecutionError> {
        enum Cursor<'a> {
            Span(&'a [FrameField]),
            Leaf(FrameField),
        }
        let fields = layout
            .fields(source)
            .ok_or_else(|| invalid("private structural source missing"))?;
        let mut cursor = match fields {
            [field] if matches!(field.lane(), Lane::Tagged { .. }) => Cursor::Leaf(*field),
            _ => Cursor::Span(fields),
        };
        work.charge(NativeExecutionPhase::Setup, amount(path.len(), 64, 64)?)?;
        for step in path {
            cursor = match (step, cursor) {
                (Step::TupleItem { ordinal }, Cursor::Span(fields)) => Cursor::Leaf(
                    *fields
                        .get(*ordinal as usize)
                        .ok_or_else(|| invalid("private structural tuple index"))?,
                ),
                (Step::TupleItem { ordinal }, Cursor::Leaf(field))
                    if matches!(field.lane(), Lane::Composite { .. }) =>
                {
                    Cursor::Leaf(
                        *layout
                            .children(field)
                            .and_then(|f| f.get(*ordinal as usize))
                            .ok_or_else(|| invalid("private structural tuple index"))?,
                    )
                }
                (Step::EnumPayload { ordinal }, Cursor::Leaf(field))
                    if matches!(field.lane(), Lane::Tagged { .. }) =>
                {
                    if self.current_tag(field)? != *ordinal as usize {
                        return Err(invalid(
                            "structural observation path differs from its admitted schema",
                        ));
                    }
                    Cursor::Leaf(
                        layout
                            .tagged_variant(field, *ordinal)
                            .ok_or_else(|| invalid("private structural variant missing"))?,
                    )
                }
                _ => {
                    return Err(invalid(
                        "structural observation path differs from its admitted schema",
                    ));
                }
            };
        }
        let Cursor::Leaf(field) = cursor else {
            return Err(invalid("private structural result is not a field"));
        };
        if let Observation::EnumOrdinal { ordinal } = observation {
            let value = NativeScalarValue::Bool(self.current_tag(field)? == ordinal as usize);
            return Ok(Some(Field::Scalar {
                schema: field.schema(),
                value,
            }));
        }
        if !matches!(observation, Observation::U64 | Observation::CanonicalValue) {
            return Err(invalid("private structural observation unsupported"));
        }
        if matches!(field.lane(), Lane::Composite { .. } | Lane::Tagged { .. }) {
            let source = Self::composite_target(layout, &field)?;
            let target = layout
                .fields(output)
                .ok_or_else(|| invalid("private structural output missing"))?;
            work.charge(
                NativeExecutionPhase::Setup,
                amount(Self::field_count(layout, source), 64, 64)?,
            )?;
            self.check_copy(layout, source, target)?;
            self.copy_fields::<false>(layout, source, target)?;
            return Ok(None);
        }
        let (owner, word) = self
            .current_leaf(field)
            .ok_or_else(|| invalid("private structural leaf is not current"))?;
        Ok(Some(match owner {
            Some(value) => Field::Managed(value.clone()),
            None => {
                let Lane::Scalar { kind, .. } = field.lane() else {
                    return Err(invalid("private structural scalar missing"));
                };
                Field::Scalar {
                    schema: field.schema(),
                    value: NativeScalarValue::from_bits(kind, word)
                        .ok_or_else(|| invalid("private structural bits invalid"))?,
                }
            }
        }))
    }
}
