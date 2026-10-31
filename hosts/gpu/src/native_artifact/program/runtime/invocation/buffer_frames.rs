//! Exact-site helpers for private buffer-containing tuple/record frames.
use super::*;
use mech_engine::native_artifact::NativeBufferFrameOperation as Operation;

impl Invocation<'_, '_, '_> {
    pub(super) fn buffer_frame_action(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        words: &[u64],
    ) -> Result<Option<Option<u64>>, Error> {
        let descriptor = self.descriptor;
        let layout = &descriptor.metadata().buffer_frames;
        let Some(action) = layout.action(site) else {
            return Ok(None);
        };
        let mut result = None;
        match action.operation() {
            Operation::Pack { output } if tag == GlobalHelper::Library => {
                let NativeIterationAction::LibraryCall {
                    operands,
                    output: expected,
                    ..
                } = descriptor.metadata().plan.blocks()[site].action()
                else {
                    return Err(invalid("private frame pack outside selected call"));
                };
                if expected != output || words.len() != operands.len() {
                    return Err(invalid("private frame pack operands differ"));
                }
                for (index, operand) in operands.iter().enumerate() {
                    self.state.call_inputs.values_mut()[index] = if let Some(kind) =
                        descriptor.metadata().plan.values()[operand.get() as usize].scalar_kind()
                    {
                        if !kind.accepts_bits(words[index]) {
                            return Err(invalid("private frame scalar word exceeds its type"));
                        }
                        None
                    } else {
                        if words[index] != 0 {
                            return Err(invalid("private frame managed input has native bits"));
                        }
                        if layout.fields(*operand).is_some() {
                            None
                        } else {
                            Some(
                                self.state
                                    .arena
                                    .copy_current(operand.get(), &mut self.state.control.work)?,
                            )
                        }
                    };
                }
                let call = descriptor.metadata().calls[site]
                    .as_ref()
                    .ok_or_else(|| invalid("private frame lost selected factory"))?;
                let coexistence = self.selected_join_coexistence(None)?;
                let outcome = if operands.iter().any(|v| layout.fields(*v).is_some()) {
                    call.admit_buffer_frame_composition(
                        &self.state.call_inputs.values()[..words.len()],
                        words,
                        |field| self.state.buffer_frames.current_leaf(field),
                        None,
                        self.limits,
                        &self.state.library,
                        coexistence,
                    )
                } else {
                    call.admit_buffer_frame_words(
                        &self.state.call_inputs.values()[..words.len()],
                        words,
                        self.limits,
                        &self.state.library,
                        coexistence,
                    )
                };
                if let Err(error) = outcome {
                    self.state.call_inputs.values_mut()[..words.len()].fill(None);
                    return Err(Error::Library {
                        origin: descriptor.metadata().plan.blocks()[site].origin(),
                        error,
                    });
                }
                let packed = self.state.buffer_frames.pack(
                    layout,
                    *output,
                    operands,
                    &mut self.state.call_inputs.values_mut()[..words.len()],
                    words,
                    &mut self.state.control.work,
                );
                self.state.call_inputs.values_mut()[..words.len()].fill(None);
                packed?;
            }
            Operation::Copy { source, output } if tag == GlobalHelper::Move && words.is_empty() => {
                let expected = self
                    .needs_move(site)
                    .ok_or_else(|| invalid("private frame copy outside managed move"))?;
                if expected != (*source, *output) {
                    return Err(invalid("private frame move differs from current action"));
                }
                self.state.buffer_frames.copy(
                    layout,
                    *source,
                    *output,
                    &mut self.state.control.work,
                )?;
            }
            Operation::Project {
                source,
                ordinal,
                output,
            } if (tag == GlobalHelper::StructuralObservation && words.is_empty())
                || (tag == GlobalHelper::Library && words == [0, 0]) =>
            {
                let action = descriptor.metadata().plan.blocks()[site].action();
                let matches_visit = match (tag, action) {
                    (
                        GlobalHelper::StructuralObservation,
                        NativeIterationAction::Structural {
                            source: expected_source, output: expected_output, ..
                        },
                    ) => expected_source == source && expected_output == output,
                    (
                        GlobalHelper::Library,
                        NativeIterationAction::LibraryCall {
                            operands, output: expected_output, template,
                        },
                    ) => operands.len() == 2 && operands[0] == *source
                        && expected_output == output
                        && template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::RecordFieldAccess,
                    _ => false,
                };
                if !matches_visit {
                    return Err(invalid("private frame projection differs from visit"));
                }
                let field = layout
                    .fields(*source)
                    .and_then(|f| f.get(*ordinal))
                    .ok_or_else(|| invalid("private projection field missing"))?;
                if layout.children(*field).is_some() {
                    if let NativeIterationAction::LibraryCall { operands, .. } = action {
                        // Preserve the original paid read and error order. Lend
                        // only geometry/work fields while this owner stays read.
                        let selector = self
                            .state
                            .arena
                            .read_current(operands[1].get(), &mut self.state.control.work)?;
                        let call = descriptor.metadata().calls[site]
                            .as_ref()
                            .ok_or_else(|| invalid("private child lost selected factory"))?;
                        let coexistence = super::joins::coexistence(
                            self.state.collections.as_ref(),
                            &self.state.line_matrices,
                            descriptor,
                            &self.state.library,
                            &mut self.state.control.work,
                            None,
                        )?;
                        call.admit_buffer_frame_composition(
                            &[None, None],
                            &[0, 0],
                            |field| self.state.buffer_frames.current_leaf(field),
                            Some((selector, *ordinal)),
                            self.limits,
                            &self.state.library,
                            coexistence,
                        )
                        .map_err(|error| Error::Library {
                            origin: descriptor.metadata().plan.blocks()[site].origin(),
                            error,
                        })?;
                    }
                    self.state.buffer_frames.project_composite(
                        layout,
                        *source,
                        *ordinal,
                        *output,
                        &mut self.state.control.work,
                    )?;
                } else {
                    let value = self.state.buffer_frames.project(
                        layout,
                        *source,
                        *ordinal,
                        &mut self.state.control.work,
                    )?;
                    if let NativeIterationAction::LibraryCall { operands, .. } = action {
                        // Preserve the original paid read and error order. Lend
                        // only geometry/work fields while this owner stays read.
                        let selector = self
                            .state
                            .arena
                            .read_current(operands[1].get(), &mut self.state.control.work)?;
                        let call = descriptor.metadata().calls[site]
                            .as_ref()
                            .ok_or_else(|| invalid("private record field lost selected factory"))?;
                        let coexistence = super::joins::coexistence(
                            self.state.collections.as_ref(),
                            &self.state.line_matrices,
                            descriptor,
                            &self.state.library,
                            &mut self.state.control.work,
                            None,
                        )?;
                        match &value {
                            super::super::buffer_frames::Field::Managed(value) => call
                                .admit_buffer_record_field(
                                    selector,
                                    *ordinal,
                                    value,
                                    self.limits,
                                    &self.state.library,
                                    coexistence,
                                ),
                            super::super::buffer_frames::Field::Scalar { schema, value } => call
                                .admit_buffer_record_word(
                                    selector,
                                    *ordinal,
                                    *schema,
                                    *value,
                                    self.limits,
                                    &self.state.library,
                                    coexistence,
                                ),
                        }
                        .map_err(|error| Error::Library {
                            origin: descriptor.metadata().plan.blocks()[site].origin(),
                            error,
                        })?;
                    }
                    let ty = &descriptor.metadata().plan.values()[output.get() as usize];
                    match value {
                        super::super::buffer_frames::Field::Scalar { schema, value } => {
                            if schema != ty.schema() || ty.scalar_kind() != Some(value.kind()) {
                                return Err(invalid("private frame scalar field schema differs"));
                            }
                            result = Some(value.to_bits());
                        }
                        super::super::buffer_frames::Field::Managed(value) => {
                            // This exact value index/schema/definition/key relation
                            // is checked at preparation and independent restoration.
                            let slot = &descriptor.metadata().types[output.get() as usize];
                            if value.schema() != slot.schema
                                || value.schema_key() != slot.key
                                || ty.scalar_kind().is_some()
                            {
                                return Err(invalid("private frame managed field schema differs"));
                            }
                            self.state.arena.put(
                                output.get(),
                                &value,
                                &mut self.state.control.work,
                            )?;
                        }
                    }
                }
            }
            _ => {
                return Err(invalid(
                    "private frame helper differs from selected operation",
                ));
            }
        }
        // Same-plan liveness checks include exact loop seed/latch reads. Retire
        // only after all current reads; a fresh definition kills the old visit.
        for value in action.retire() {
            if !self.state.buffer_frames.retire(
                &descriptor.metadata().frame_retirements,
                value.get(),
                &mut self.state.control.work,
            )? {
                self.state
                    .arena
                    .retire_current(value.get(), &mut self.state.control.work)?;
            }
        }
        Ok(Some(result))
    }
}
