//! Pin actual canonical owners once; generated code alone traverses the FSM.
use crate::native_artifact::NativeExecutionPhase;
use crate::native_artifact::memory::{NativeTypedStorage, NativeWordStorage};
use crate::native_artifact::program::runtime::invocation::Invocation;
use crate::native_artifact::program::runtime::{
    NativeProgramExecutionError as Error, amount, invalid,
};
use mech_core::snapshot::SequenceView;
use mech_core::{Value, ValueData};
use mech_engine::native_artifact::{NativeBorrowedStringAction as Action, NativeIterationAction};

#[cfg(all(test, feature = "canonical-native-program"))]
thread_local! {
    static WINDOW_CAPTURE: std::cell::Cell<Option<(u64, u64, u64)>> = const { std::cell::Cell::new(None) };
}
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) fn take_window_capture() -> Option<(u64, u64, u64)>
{
    WINDOW_CAPTURE.with(std::cell::Cell::take)
}

pub(in crate::native_artifact::program::runtime) struct State {
    pub region: usize,
    pub active: bool,
    // Owners outlive every generated pointer, and are dropped before the arena.
    owners: NativeTypedStorage<Option<Value>>,
    _rows: NativeWordStorage,
}
fn texts(value: &Value) -> Option<&[Box<str>]> {
    let ValueData::Matrix(matrix) = value.data() else {
        return None;
    };
    match matrix.elements() {
        SequenceView::String(texts) => Some(texts),
        _ => None,
    }
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn borrowed_action(
        &self,
        site: usize,
    ) -> Option<&Action> {
        let state = self.borrowed_strings.as_ref()?;
        let (region, action) = self.descriptor.metadata().borrowed_strings.action(site)?;
        (state.active && state.region == region).then_some(action)
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_borrowed_strings(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let ordinal = self.descriptor.metadata().borrowed_strings.initializers[site]
            .ok_or_else(|| invalid("borrowed String preparation outside initializer"))?;
        self.prepare_borrowed_region(site, args, ordinal)
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_borrowed_region(
        &mut self,
        site: usize,
        args: [u64; 3],
        ordinal: usize,
    ) -> Result<Option<u64>, Error> {
        if self.borrowed_strings.is_some() || self.state.consumed || args != [0; 3] {
            return Err(invalid(
                "duplicate or malformed borrowed String preparation",
            ));
        }
        let descriptor = self.descriptor;
        let region = &descriptor.metadata().borrowed_strings.regions[ordinal];
        if self.state.control.current != Some(region.proof.iteration()) {
            return Err(invalid("borrowed String preparation outside current loop"));
        }
        let count = region.proof.matrices().len() + region.proof.literals().len();
        let memory = self.state.library.memory_budget();
        let limit = self
            .limits
            .max_temporary_bytes
            .ok_or_else(|| invalid("unbounded String view metadata"))?;
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, amount(count, 256, 4096)?)?;
        let mut owners = NativeTypedStorage::new_values(count, limit, &memory)?;
        let mut rows = 0usize;
        let mut source_bytes = 0u64;
        let mut line_metadata_bytes = 0u64;
        let mut active = true;
        for (i, value) in region
            .proof
            .matrices()
            .iter()
            .chain(region.proof.literals())
            .enumerate()
        {
            let window = if i < region.proof.matrices().len() {
                self.prefix_line_window(*value)
            } else {
                None
            };
            if let Some(window) = window {
                // This immutable Invocation loan owns the actual source cells.
                // No canonical source String or Matrix exists before EOF,
                // and the window's admitted capacity includes its row table.
                self.state
                    .control
                    .work
                    .charge(NativeExecutionPhase::Setup, 128)?;
                active &= window.is_ascii();
                source_bytes = source_bytes
                    .checked_add(window.retained_capacity_bytes())
                    .ok_or_else(|| invalid("borrowed line window footprint overflow"))?;
                continue;
            }
            let lines = if i < region.proof.matrices().len() {
                self.line_matrix(*value).cloned()
            } else {
                None
            };
            let owner = if let Some(lines) = &lines {
                self.state
                    .control
                    .work
                    .charge(NativeExecutionPhase::Setup, 128)?;
                line_metadata_bytes = line_metadata_bytes
                    .checked_add(lines.metadata_bytes())
                    .ok_or_else(|| invalid("borrowed line metadata overflow"))?;
                active &= lines.is_ascii();
                lines
                    .source()
                    .ok_or_else(|| invalid("captured line matrix requires an actual row window"))?
                    .clone()
            } else {
                self.state
                    .arena
                    .copy_current(value.get(), &mut self.state.control.work)?
            };
            if self.prefix.is_some() && i < region.proof.matrices().len() {
                let actual = lines
                    .as_ref()
                    .map(|lines| lines.count())
                    .or_else(|| texts(&owner).map(<[_]>::len))
                    .ok_or_else(|| invalid("prefix source has no genuine line extent"))?;
                self.validate_prefix_extent(*value, actual)?;
            }
            source_bytes = source_bytes
                .checked_add(
                    owner
                        .memory_budget_retained_bytes(&memory)
                        .ok_or_else(|| invalid("borrowed String capture is not admitted"))?,
                )
                .ok_or_else(|| invalid("borrowed String owner footprint overflow"))?;
            if i < region.proof.matrices().len() && lines.is_none() {
                if let Some(strings) = texts(&owner) {
                    rows = rows
                        .checked_add(strings.len())
                        .ok_or_else(|| invalid("String view row count overflow"))?;
                } else {
                    active = false;
                }
            }
            owners.values_mut()[i] = Some(owner);
        }
        let words = rows
            .checked_mul(2)
            .ok_or_else(|| invalid("String view table overflow"))?;
        let metadata = amount(count, core::mem::size_of::<Option<Value>>() as u64, 8192)?;
        let temporary = amount(rows, 16, metadata)?
            .checked_add(source_bytes)
            .and_then(|bytes| bytes.checked_add(line_metadata_bytes))
            .ok_or_else(|| invalid("borrowed String temporary footprint overflow"))?;
        if temporary > limit {
            return Err(Error::Collection {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error: mech_core::ResidentKernelError::ResourceLimit {
                    phase: mech_core::ResidentResourcePhase::Admission,
                    dimension: mech_core::MemoryBudgetDimension::TemporaryBytes,
                    required: temporary,
                    limit,
                },
            });
        }
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, amount(rows, 128, 4096)?)?;
        let mut table = NativeWordStorage::new(words, limit, Some(&memory))?;
        let mut cursor = 0;
        for (i, owner) in owners.values()[..region.proof.matrices().len()]
            .iter()
            .enumerate()
        {
            if self.line_matrix(region.proof.matrices()[i]).is_some()
                || self
                    .prefix_line_window(region.proof.matrices()[i])
                    .is_some()
            {
                continue;
            }
            let owner = owner
                .as_ref()
                .ok_or_else(|| invalid("borrowed matrix lacks its source owner"))?;
            if let Some(strings) = texts(owner) {
                for text in strings {
                    // ASCII except CRLF has exactly one byte per canonical grapheme.
                    // Admit ASCII + CR-absence + possible pair scans before
                    // examining payload bytes, under the same4*len bound.
                    self.state
                        .control
                        .work
                        .charge(NativeExecutionPhase::Setup, amount(text.len(), 4, 32)?)?;
                    active &= mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf(text);
                    table.values_mut()[cursor] = text.as_ptr() as u64;
                    table.values_mut()[cursor + 1] = text.len() as u64;
                    cursor += 2;
                }
            }
        }
        let mut offset = 0;
        let mut capture = 0;
        for (i, owner) in owners.values().iter().enumerate() {
            let (pointer, length) = if i < region.proof.matrices().len() {
                if let Some(window) = self.prefix_line_window(region.proof.matrices()[i]) {
                    window.native_rows()
                } else if let Some(lines) = self.line_matrix(region.proof.matrices()[i]) {
                    // The Invocation/session pins this exact immutable table across
                    // the native scanner; its real String owner is also pinned
                    // in owners. No second pointer table or Matrix is allocated.
                    let rows = lines.rows().ok_or_else(|| {
                        invalid("captured line matrix requires an actual row window")
                    })?;
                    (rows.as_ptr() as u64, lines.count() as u64)
                } else {
                    let owner = owner
                        .as_ref()
                        .ok_or_else(|| invalid("borrowed matrix lacks its source owner"))?;
                    let length = texts(owner).map_or(0, |s| s.len());
                    // add is only used to form an in-bounds or one-past empty view.
                    let pointer = table.values().as_ptr().wrapping_add(offset) as u64;
                    offset += length * 2;
                    (pointer, length as u64)
                }
            } else {
                let owner = owner
                    .as_ref()
                    .ok_or_else(|| invalid("borrowed literal lacks its source owner"))?;
                let ValueData::String(text) = owner.data() else {
                    return Err(invalid("String literal owner kind differs"));
                };
                if text.len() > 32
                    || !mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf(text)
                {
                    return Err(invalid("String literal representation differs from proof"));
                }
                (text.as_ptr() as u64, text.len() as u64)
            };
            self.write_borrowed_word(capture, pointer)?;
            self.write_borrowed_word(capture + 1, length)?;
            capture += 2;
            if i < region.proof.matrices().len() {
                let first = self
                    .prefix_line_window(region.proof.matrices()[i])
                    .map_or(1, |window| window.first());
                self.write_borrowed_word(capture, first)?;
                #[cfg(all(test, feature = "canonical-native-program"))]
                if self
                    .prefix_line_window(region.proof.matrices()[i])
                    .is_some()
                {
                    WINDOW_CAPTURE.with(|capture| capture.set(Some((pointer, length, first))));
                }
                capture += 1;
            }
        }
        self.borrowed_strings = Some(State {
            region: ordinal,
            active,
            owners,
            _rows: table,
        });
        Ok(Some(u64::from(active)))
    }
    /// Only a generated failing bounds branch reaches this callback. Recreate
    /// its private source, then let the SAME selected canonical leaf report
    /// the exact error. It can never resume or publish a successful result.
    pub(in crate::native_artifact::program::runtime) fn borrowed_string_failure(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let action = self
            .borrowed_action(site)
            .cloned()
            .ok_or_else(|| invalid("String failure outside native view"))?;
        let descriptor = self.descriptor;
        let NativeIterationAction::LibraryCall { operands, .. } =
            descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("String failure outside selected library leaf"));
        };
        let state = self.borrowed_strings.take().unwrap();
        if let Action::MatrixElement { source, .. } = &action
            && let Some(window) = self.prefix_line_window(*source)
        {
            let prior = self.selected_join_coexistence(Some(*source))?;
            let call = descriptor.metadata().calls[site]
                .as_ref()
                .ok_or_else(|| invalid("line window bounds failure lost its selected call"))?;
            let result = call.text_line_window_element(
                window,
                args[2],
                self.limits,
                &self.state.library,
                prior,
            );
            let error = match result {
                Err(error) => Error::Library {
                    origin: descriptor.metadata().plan.blocks()[site].origin(),
                    error,
                },
                Ok(_) => invalid("borrowed line window bounds failure unexpectedly succeeded"),
            };
            drop(state);
            return Err(error);
        }
        if let Action::MatrixElement { source, .. } = &action
            && let Some(lines) = self.line_matrix(*source).cloned()
        {
            let prior = self.selected_join_coexistence(Some(*source))?;
            let call = descriptor.metadata().calls[site]
                .as_ref()
                .ok_or_else(|| invalid("line matrix bounds failure lost its selected call"))?;
            let result = call.text_line_matrix_element(
                &lines,
                args[2],
                self.limits,
                &self.state.library,
                prior,
            );
            let error = match result {
                Err(error) => Error::Library {
                    origin: descriptor.metadata().plan.blocks()[site].origin(),
                    error,
                },
                Ok(_) => invalid("borrowed line bounds failure unexpectedly succeeded"),
            };
            drop(state);
            return Err(error);
        }
        if matches!(action, Action::FixedRange { .. }) {
            // On a failure only, reconstruct an elided selector through its
            // actual maintained leaf so the original String leaf can report
            // its exact error. This does not change generated control progress.
            let selector = operands[1];
            self.state.control.work.charge(
                NativeExecutionPhase::Setup,
                amount(
                    descriptor.metadata().borrowed_strings.regions[state.region]
                        .proof
                        .actions()
                        .len(),
                    64,
                    128,
                )?,
            )?;
            let restore = descriptor.metadata().borrowed_strings.regions[state.region]
                .proof
                .actions()
                .iter()
                .find_map(|(range_site, action)| match action {
                    Action::ConstantRange {
                        first,
                        last,
                        output,
                    } if *output == selector => Some((*range_site as usize, *first, *last)),
                    _ => None,
                });
            if let Some((range_site, first, last)) = restore {
                self.restore_borrowed_selector(range_site, [first, last])?;
            }
        }
        if !matches!(action, Action::MatrixElement { .. }) {
            let source = match action {
                Action::ScalarSelect { source, .. } | Action::FixedRange { source, .. } => source,
                _ => return Err(invalid("String failure outside a bounds operation")),
            };
            let mut inspected = 0usize;
            for owner in state.owners.values().iter().flatten() {
                inspected = inspected
                    .checked_add(texts(owner).map_or(1, |s| s.len()))
                    .ok_or_else(|| invalid("String failure owner scan overflow"))?;
            }
            let region = &descriptor.metadata().borrowed_strings.regions[state.region];
            let mut window_inspected = 0usize;
            for &matrix in region.proof.matrices() {
                if let Some(window) = self.prefix_line_window(matrix) {
                    window_inspected = window_inspected
                        .checked_add(window.row_count())
                        .ok_or_else(|| invalid("String failure window scan overflow"))?;
                }
            }
            self.state.control.work.charge(
                NativeExecutionPhase::Setup,
                amount(inspected, 64, 128)?
                    .checked_add(amount(window_inspected, 256, 0)?)
                    .ok_or_else(|| invalid("String failure scan work overflow"))?,
            )?;
            let end = args[0]
                .checked_add(args[1])
                .ok_or_else(|| invalid("String failure view overflow"))?;
            let slice = |text: &str| -> Option<(usize, usize)> {
                let start = text.as_ptr() as u64;
                let bound = start.checked_add(text.len() as u64)?;
                if args[0] >= start && end <= bound {
                    Some(((args[0] - start) as usize, (end - start) as usize))
                } else {
                    None
                }
            };
            let mut found = None;
            for &matrix in region.proof.matrices() {
                if let Some(window) = self.prefix_line_window(matrix) {
                    for ordinal in window.first()..window.end() {
                        let text = window
                            .line(ordinal)
                            .ok_or_else(|| invalid("String failure window lost its real cell"))?;
                        if let Some((a, b)) = slice(text) {
                            found = text.get(a..b);
                            break;
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            for owner in state.owners.values().iter().flatten() {
                if found.is_some() {
                    break;
                }
                if let Some(strings) = texts(owner) {
                    for text in strings {
                        if let Some((a, b)) = slice(text) {
                            found = text.get(a..b);
                            break;
                        }
                    }
                } else if let ValueData::String(text) = owner.data() {
                    if let Some((a, b)) = slice(text) {
                        found = text.get(a..b);
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            let text = found.ok_or_else(|| invalid("String failure view is not owned"))?;
            let bytes = amount(text.len(), 8, descriptor.metadata().scalar_bytes)?;
            self.state
                .control
                .work
                .charge(NativeExecutionPhase::Setup, bytes)?;
            let mut reservation = self.state.library.memory_budget().reserve_capacity(bytes)?;
            let value = mech_core::snapshot::ValueDraft {
                schema: descriptor.metadata().plan.values()[source.get() as usize].schema(),
                shape_values: Box::new([]),
                data: mech_core::snapshot::ValueDataDraft::String(text.to_owned()),
            }
            .finalize(
                &mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(
                    &descriptor.metadata().schemas,
                ),
            )
            .map_err(Error::Snapshot)?
            .into_memory_budget(&mut reservation)?;
            self.state
                .arena
                .put(source.get(), &value, &mut self.state.control.work)?;
        }
        for (i, operand) in operands.iter().enumerate() {
            let word = if descriptor.metadata().plan.values()[operand.get() as usize]
                .scalar_kind()
                .is_some()
            {
                args[2]
            } else {
                0
            };
            self.write_borrowed_word(i, word)?;
        }
        let result = self.helper(
            site as u64,
            crate::native_artifact::program::protocol::abi::LIBRARY,
            [self.arguments as u64, operands.len() as u64, 0],
        );
        // Retain all backing owners until the canonical leaf has finished.
        drop(state);
        match result {
            Err(error) => Err(error),
            Ok(_) => Err(invalid("borrowed bounds failure unexpectedly succeeded")),
        }
    }

    fn restore_borrowed_selector(&mut self, site: usize, words: [u64; 2]) -> Result<(), Error> {
        let descriptor = self.descriptor;
        self.state.control.work.charge(
            NativeExecutionPhase::Setup,
            amount(self.state.call_inputs.values().len(), 16, 128)?,
        )?;
        let NativeIterationAction::LibraryCall {
            operands, output, ..
        } = descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("elided String selector lost its selected call"));
        };
        if operands.len() != 2 || self.state.call_inputs.values().iter().any(Option::is_some) {
            return Err(invalid("elided String selector has stale call owners"));
        }
        for (i, &operand) in operands.iter().enumerate() {
            let value = self.scalar(operand, words[i])?;
            self.state.call_inputs.values_mut()[i] = Some(value);
        }
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("elided String selector lost its prepared leaf"))?;
        let mut result = None;
        let outcome = self.state.call_caches.invoke(
            site,
            call,
            &self.state.call_inputs.values()[..2],
            &mut result,
            self.limits,
            &self.state.library,
        );
        self.state.call_inputs.values_mut()[..2].fill(None);
        outcome.map_err(|error| Error::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
            error,
        })?;
        let result = result.ok_or_else(|| invalid("elided String selector omitted its result"))?;
        self.state
            .arena
            .put(output.get(), &result, &mut self.state.control.work)?;
        Ok(())
    }
}
