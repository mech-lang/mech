//! Own the same selected line producer across generated scan continuations.
//! Capture storage is private; no unfinished String, Matrix, or count enters
//! the canonical arena. Runtime delivery/EOF authorization is wired separately.
use crate::native_artifact::program::runtime::session::*;
#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
use mech_engine::native_artifact::NativeTextLineChunks;
#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
use mech_engine::native_artifact::NativeTextLineMatrix;
#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
use std::rc::Rc;

#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
pub(in crate::native_artifact::program::runtime::session) fn capture_source_coexistence(
    state: &ExecutionState<'_>,
    skip: usize,
    memory: &ManagedMemoryBudget,
) -> Result<mech_core::ResourceDemand, NativeProgramExecutionError> {
    let mut prior = state
        .collections
        .as_ref()
        .map_or(Ok(mech_core::ResourceDemand::default()), |collections| {
            collections.retained_demand()
        })?;
    let lines = state.line_matrices.retained_demand(Some(skip), memory)?;
    prior.turn_peak_bytes = prior
        .turn_peak_bytes
        .checked_add(lines.turn_peak_bytes)
        .ok_or_else(|| invalid("capture source coexistence bytes overflow"))?;
    prior.retained_nodes = prior
        .retained_nodes
        .checked_add(lines.retained_nodes)
        .ok_or_else(|| invalid("capture source coexistence nodes overflow"))?;
    Ok(prior)
}

#[cfg(any(
    feature = "canonical-native-program-host-runtime",
    all(test, feature = "canonical-native-program")
))]
impl Prepared<'_> {
    pub(in crate::native_artifact::program::runtime::session) fn install_final_line_count(
        &mut self,
        count: u64,
    ) -> Result<(), NativeProgramExecutionError> {
        let region = self
            .descriptor
            .metadata()
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("final count without selected line frontier"))?;
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let matrix = region.frontier.read().count().lines().matrix();
        let ordinal = self.descriptor.metadata().line_matrices.matrices[matrix.get() as usize]
            .ok_or_else(|| invalid("final count lost its selected line producer"))?;
        let actual = if let Some(view) = &state.line_matrices.views[ordinal] {
            view.count() as u64
        } else {
            let value = state
                .arena
                .copy_current(matrix.get(), &mut state.control.work)?;
            let ValueData::Matrix(matrix) = value.data() else {
                return Err(invalid("final count has no admitted line matrix"));
            };
            matrix.elements().len() as u64
        };
        if actual != count {
            return Err(invalid("final count differs from admitted source geometry"));
        }
        let counts = region.frontier.read().count().counts();
        state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, amount(counts.len(), 16, 128)?)?;
        let first = self.descriptor.metadata().plan.transport_input_count();
        for value in counts {
            self.storage.values_mut()[first + value.get() as usize] = actual;
        }
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime::session) fn begin_line_capture(
        &mut self,
        input_limit: u64,
        fragment: usize,
    ) -> Result<NativeTextLineChunks, NativeProgramExecutionError> {
        let descriptor = self.descriptor;
        let region = descriptor
            .metadata()
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("capture lacks compiled retention proof"))?;
        let source = region.frontier.read().count().lines();
        if !self.activated
            || self.prefix.is_some()
            || descriptor.metadata().plan.transport_input_count() != 1
            || descriptor.metadata().plan.transport_input_value(0) != source.source()
            || source.lengths().len() != 1
            || descriptor.metadata().plan.blocks()[source.producer() as usize]
                .iteration()
                .is_some()
            || descriptor.metadata().plan.blocks()[source.lengths()[0] as usize]
                .iteration()
                .is_some()
        {
            return Err(invalid(
                "capture setup is outside supported original source visits",
            ));
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let ordinal = descriptor.metadata().line_matrices.matrices[source.matrix().get() as usize]
            .ok_or_else(|| invalid("capture lacks selected line owner slot"))?;
        if state.line_matrices.captured.is_some() || state.line_matrices.views[ordinal].is_some() {
            return Err(invalid("capture source already installed"));
        }
        let site = source.producer() as usize;
        let owner = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("capture source selection absent"))?
            .prepare_text_line_chunks(input_limit, fragment, self.limits.library, &state.library)
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
        state.line_matrices.captured =
            Some(crate::native_artifact::program::runtime::lines::Captured {
                ordinal,
                produced: false,
                length: None,
            });
        Ok(owner)
    }

    pub(in crate::native_artifact::program::runtime::session) fn admit_line_capture(
        &mut self,
        owner: NativeTextLineChunks,
    ) -> Result<Rc<NativeTextLineMatrix>, NativeProgramExecutionError> {
        let descriptor = self.descriptor;
        let source = descriptor
            .metadata()
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("capture lost compiled retention proof"))?
            .frontier
            .read()
            .count()
            .lines();
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| invalid("consumed native state"))?;
        let captured = state
            .line_matrices
            .captured
            .as_ref()
            .ok_or_else(|| invalid("capture was not selected"))?;
        let length = captured
            .length
            .filter(|site| source.lengths() == [*site as u32])
            .ok_or_else(|| invalid("capture omitted original length visit"))?;
        if !captured.produced || state.line_matrices.views[captured.ordinal].is_some() {
            return Err(invalid(
                "capture omitted or repeated original producer visit",
            ));
        }
        let site = source.producer() as usize;
        let prior = capture_source_coexistence(state, captured.ordinal, &self.memory)?;
        let view = descriptor.metadata().calls[site]
            .as_ref()
            .unwrap()
            .text_line_matrix_from_capture(owner, self.limits.library, &state.library, prior)
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
        descriptor.metadata().calls[length]
            .as_ref()
            .unwrap()
            .text_line_matrix_length(&view, self.limits.library, &state.library, prior)
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.metadata().plan.blocks()[length].origin(),
                error,
            })?;
        let view = Rc::new(view);
        state.line_matrices.views[captured.ordinal] = Some(view.clone());
        Ok(view)
    }
}

#[path = "../invocation/captured_calls.rs"]
mod captured_calls;
