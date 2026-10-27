//! Exact-site helpers for private buffer-containing tuple argument frames.
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
                    let value = if descriptor.metadata().plan.values()[operand.get() as usize]
                        .scalar_kind()
                        .is_some()
                    {
                        self.scalar(*operand, words[index])?
                    } else {
                        if words[index] != 0 {
                            return Err(invalid("private frame managed input has native bits"));
                        }
                        self.state
                            .arena
                            .copy_current(operand.get(), &mut self.state.control.work)?
                    };
                    self.state.call_inputs.values_mut()[index] = Some(value);
                }
                let call = descriptor.metadata().calls[site]
                    .as_ref()
                    .ok_or_else(|| invalid("private frame lost selected factory"))?;
                let coexistence = self.selected_join_coexistence(None)?;
                let outcome = call.admit_buffer_frame(
                    &self.state.call_inputs.values()[..words.len()],
                    self.limits,
                    &self.state.library,
                    coexistence,
                );
                if let Err(error) = outcome {
                    self.state.call_inputs.values_mut()[..words.len()].fill(None);
                    return Err(Error::Library {
                        origin: descriptor.metadata().plan.blocks()[site].origin(),
                        error,
                    });
                }
                self.state.buffer_frames.pack(
                    layout,
                    *output,
                    &self.state.call_inputs.values()[..words.len()],
                    &mut self.state.control.work,
                )?;
                self.state.call_inputs.values_mut()[..words.len()].fill(None);
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
            } if tag == GlobalHelper::StructuralObservation && words.is_empty() => {
                let NativeIterationAction::Structural {
                    source: expected_source,
                    output: expected_output,
                    ..
                } = descriptor.metadata().plan.blocks()[site].action()
                else {
                    return Err(invalid("private frame projection outside structural visit"));
                };
                if expected_source != source || expected_output != output {
                    return Err(invalid("private frame projection differs from visit"));
                }
                let value = self.state.buffer_frames.project(
                    layout,
                    *source,
                    *ordinal,
                    &mut self.state.control.work,
                )?;
                let ty = &descriptor.metadata().plan.values()[output.get() as usize];
                if value.schema() != ty.schema() || value.schema_key() != ty.schema_key() {
                    return Err(invalid("private frame field schema differs"));
                }
                if let Some(kind) = ty.scalar_kind() {
                    result = Some(bits(&value, kind)?);
                } else {
                    self.state
                        .arena
                        .put(output.get(), &value, &mut self.state.control.work)?;
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
                layout,
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
