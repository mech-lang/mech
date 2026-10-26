//! Delivery through the existing runtime transaction into one generated body.
use super::*;
use mech_engine::native_artifact::NativeTextLineChunks;
use mech_runtime::{
    CanonicalNativeResourcePrepared, CanonicalNativeResourceTextChunk, RuntimeTextCaptureBuffer,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

enum Failure {
    Error(NativeProgramExecutionError),
    Panic(Box<dyn std::any::Any + Send>),
}
pub(super) struct Capture {
    owner: NativeTextLineChunks,
    library: NativeLibraryWorkBudget,
    offset: u64,
    scanned: u64,
    sealed: bool,
    await_eof: bool,
    failure: Option<Failure>,
    prior: mech_core::ResourceDemand,
}

impl<'a> NativeProgramExecutionSession<'a> {
    pub fn with_text_capture(
        mut self,
        input_limit: u64,
        chunk_bytes: usize,
    ) -> Result<CanonicalNativeResourcePrepared<Self>, NativeProgramExecutionError> {
        let descriptor = self.prepared.descriptor;
        let Some(state) = self.prepared.state.as_mut() else {
            return Err(invalid("capture setup has consumed native state"));
        };
        if !self.capture_offered
            || self.capture.is_some()
            || !descriptor.metadata().plan.inputs().is_empty()
            || descriptor.metadata().plan.observations().len() != 1
        {
            return Ok(CanonicalNativeResourcePrepared::snapshot(self));
        }
        state.work.charge(
            NativeExecutionPhase::Setup,
            amount(descriptor.metadata().plan.blocks().len(), 256, 4096)?,
        )?;
        if capture_admission::preflight(
            descriptor,
            self.prepared.limits,
            state.library.used(),
            input_limit,
            chunk_bytes,
            self.prepared.memory.limit_bytes(),
        )
        .is_none()
        {
            return Ok(CanonicalNativeResourcePrepared::snapshot(self));
        }
        let Ok(buffer) =
            RuntimeTextCaptureBuffer::prepare(input_limit, chunk_bytes, &self.prepared.memory)
        else {
            return Ok(CanonicalNativeResourcePrepared::snapshot(self));
        };
        let fragment = chunk_bytes
            .checked_add(3)
            .ok_or_else(|| invalid("capture fragment capacity overflow"))?;
        let library = state.library.clone();
        let owner = match self.prepared.begin_line_capture(input_limit, fragment) {
            Ok(owner) => owner,
            Err(_) => return Ok(CanonicalNativeResourcePrepared::snapshot(self)),
        };
        let state = self
            .prepared
            .state
            .as_ref()
            .ok_or_else(|| invalid("capture omitted initial state"))?;
        let captured = state.line_matrices.captured.as_ref().unwrap();
        let prior =
            capture::capture_source_coexistence(state, captured.ordinal, &self.prepared.memory)?;
        let initial = match owner.borrow_window(1, 1) {
            Ok(initial) => initial,
            Err(_) => {
                // No generated visit has occurred. Retire private storage
                // and use the ordinary acquisition path, without refunding work.
                self.prepared.state.as_mut().unwrap().line_matrices.captured = None;
                return Ok(CanonicalNativeResourcePrepared::snapshot(self));
            }
        };
        let failure = match catch_unwind(AssertUnwindSafe(|| {
            self.prepared.run_prefix_window(
                &mut |frame| self.entry.invoke(frame),
                0,
                false,
                Some(&initial),
            )
        })) {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(Failure::Error(error)),
            Err(payload) => Some(Failure::Panic(payload)),
        };
        drop(initial);
        let source = descriptor
            .metadata()
            .prefix
            .as_ref()
            .unwrap()
            .frontier
            .read()
            .count()
            .lines();
        if failure.is_none() {
            let state = self
                .prepared
                .state
                .as_ref()
                .ok_or_else(|| invalid("capture omitted initial checkpoint"))?;
            let captured = state.line_matrices.captured.as_ref().unwrap();
            if !captured.produced || captured.length != Some(source.lengths()[0] as usize) {
                return Err(invalid("capture prelude omitted original source visits"));
            }
        }
        self.capture = Some(Capture {
            owner,
            library,
            offset: 0,
            scanned: 0,
            sealed: false,
            await_eof: false,
            failure,
            prior,
        });
        Ok(CanonicalNativeResourcePrepared::single_use_text(
            self, buffer,
        ))
    }

    pub fn capture_chunk(
        &mut self,
        chunk: CanonicalNativeResourceTextChunk<'_>,
    ) -> Result<(), NativeProgramExecutionError> {
        let descriptor = self.prepared.descriptor;
        let capture = self
            .capture
            .as_mut()
            .ok_or_else(|| invalid("text delivery to snapshot session"))?;
        let source = descriptor
            .metadata()
            .plan
            .observations()
            .first()
            .ok_or_else(|| invalid("text delivery has no admitted observation"))?;
        if chunk.invocation() != self.invocation
            || chunk.program_revision() != descriptor.metadata().plan.program_revision()
            || chunk.site().node() != source.node()
            || chunk.site().requirement() != source.requirement()
            || chunk.site().schema() != source.ty().schema()
            || chunk.site().schema_key() != source.ty().schema_key()
            || chunk.offset() != capture.offset
        {
            return Err(invalid(
                "text fragment differs from owning admitted capture",
            ));
        }
        if chunk.text().is_empty() {
            return Ok(());
        }
        capture
            .owner
            .push_fragment(chunk.fragment())
            .map_err(|error| source_error(descriptor, error))?;
        capture.offset = capture
            .offset
            .checked_add(chunk.text().len() as u64)
            .ok_or_else(|| invalid("capture byte offset overflow"))?;
        if capture.failure.is_some() || capture.await_eof {
            return Ok(());
        }
        // Suspend before borrowing rows: the source-work envelope admits each
        // completed row to one window, including the remaining EOF window.
        // Unicode executes through maintained calls after source admission.
        if !chunk.fragment().is_ascii() {
            capture.await_eof = true;
            return Ok(());
        }
        let available = capture
            .owner
            .available_lines()
            .ok_or_else(|| invalid("failed capture source"))?;
        let first = if capture.sealed {
            available + 1
        } else {
            capture.scanned + 1
        };
        if !capture_admission::window_comparison_fits(
            descriptor,
            &capture.library,
            available + 1 - first,
        ) {
            capture.await_eof = true;
            return Ok(());
        }
        let window = capture
            .owner
            .borrow_window(first, available + 1)
            .map_err(|error| source_error(descriptor, error))?;
        debug_assert!(window.is_ascii());
        match catch_unwind(AssertUnwindSafe(|| {
            self.prepared.run_prefix_window(
                &mut |frame| self.entry.invoke(frame),
                available,
                false,
                Some(&window),
            )
        })) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                capture.failure = Some(Failure::Error(error));
                return Ok(());
            }
            Err(payload) => {
                capture.failure = Some(Failure::Panic(payload));
                return Ok(());
            }
        }
        drop(window);
        capture.scanned = available;
        let retained = self
            .prepared
            .retention
            .ok_or_else(|| invalid("scanner omitted retention"))?;
        if !capture.sealed {
            if let Some(end) = retained.end {
                capture
                    .owner
                    .retain_range(retained.first, end)
                    .map_err(|error| source_error(descriptor, error))?;
                capture.sealed = true;
            } else {
                capture
                    .owner
                    .retain_from(retained.first)
                    .map_err(|error| source_error(descriptor, error))?;
            }
        }
        Ok(())
    }

    #[cfg(all(test, feature = "canonical-native-program-host"))]
    pub(super) fn test_capture_has_pending_error(&self) -> bool {
        self.capture
            .as_ref()
            .is_some_and(|capture| capture.failure.is_some())
    }

    pub(super) fn finish_text_capture(
        mut self,
        turn: mech_runtime::CanonicalNativeResourceTurn<'_>,
        inputs: &[Value],
        outputs: &mut [Option<Value>],
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        let mut capture = self
            .capture
            .take()
            .ok_or_else(|| invalid("capture completion without source"))?;
        let descriptor = self.prepared.descriptor;
        let summary = turn
            .text_capture()
            .ok_or_else(|| invalid("capture completion has no successful EOF"))?;
        if !inputs.is_empty()
            || !turn.observations().is_empty()
            || summary.input_bytes() != capture.offset
        {
            return Err(invalid("capture completion differs from native transport"));
        }
        let facts = capture
            .owner
            .finish()
            .map_err(|error| source_error(descriptor, error))?;
        admit_output_retirement(descriptor, outputs, &self.prepared.memory, &capture.library)?;
        if let Some(failure) = capture.failure {
            // The proved prelude orders both source visits before any
            // fallible scanner allocation. Their admissions precede the
            // saved body failure; failed execution state has already retired.
            let source = descriptor
                .metadata()
                .prefix
                .as_ref()
                .unwrap()
                .frontier
                .read()
                .count()
                .lines();
            let view = descriptor.metadata().calls[source.producer() as usize]
                .as_ref()
                .unwrap()
                .text_line_matrix_from_capture(
                    capture.owner,
                    self.prepared.limits.library,
                    &capture.library,
                    capture.prior,
                )
                .map_err(|error| source_error(descriptor, error))?;
            let length = source.lengths()[0] as usize;
            descriptor.metadata().calls[length]
                .as_ref()
                .unwrap()
                .text_line_matrix_length(
                    &view,
                    self.prepared.limits.library,
                    &capture.library,
                    capture.prior,
                )
                .map_err(|error| NativeProgramExecutionError::Library {
                    origin: descriptor.metadata().plan.blocks()[length].origin(),
                    error,
                })?;
            drop(view);
            return match failure {
                Failure::Error(error) => Err(error),
                Failure::Panic(payload) => std::panic::resume_unwind(payload),
            };
        }
        let view = self.prepared.admit_line_capture(capture.owner)?;
        let first = if capture.sealed {
            facts.lines() + 1
        } else {
            capture.scanned + 1
        };
        let window = view
            .captured_chunks()
            .unwrap()
            .borrow_window(first, facts.lines() + 1)
            .map_err(|error| source_error(descriptor, error))?;
        self.prepared.run_prefix_window(
            &mut |frame| self.entry.invoke(frame),
            facts.lines(),
            true,
            Some(&window),
        )?;
        drop(window);
        drop(view);
        self.prepared
            .resume_capture_tail(&mut |frame| self.entry.invoke(frame))?;
        self.prepared.publish(outputs)
    }
}

fn source_error(
    descriptor: &NativeProgramDescriptor,
    error: NativeCollectionInvocationError,
) -> NativeProgramExecutionError {
    let site = descriptor
        .metadata()
        .prefix
        .as_ref()
        .unwrap()
        .frontier
        .read()
        .count()
        .lines()
        .producer();
    NativeProgramExecutionError::Library {
        origin: descriptor.metadata().plan.blocks()[site as usize].origin(),
        error,
    }
}
