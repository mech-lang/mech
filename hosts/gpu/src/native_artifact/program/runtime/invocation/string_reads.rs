//! Selected String-matrix scalar projection; the source loop stays generated.
use super::{Error, Invocation, invalid};
use mech_core::ResourceDemand;
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeScalarKind};

impl Invocation<'_, '_, '_> {
    pub(super) fn string_matrix_element_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<bool, Error> {
        let metadata = self.descriptor.metadata();
        let Some(call) = metadata.calls[site].as_ref() else {
            return Err(invalid("String read lacks selected factory"));
        };
        let Action::LibraryCall {
            operands, output, ..
        } = metadata.plan.blocks()[site].action()
        else {
            return Err(invalid("String read outside selected library action"));
        };
        if operands.len() != 2
            || words.len() != 2
            || words[0] != 0
            || !call.supports_string_matrix_element()
            || metadata.plan.values()[operands[0].get() as usize]
                .scalar_kind()
                .is_some()
            || metadata.plan.values()[operands[1].get() as usize].scalar_kind()
                != Some(NativeScalarKind::U64)
            || metadata.plan.values()[output.get() as usize]
                .scalar_kind()
                .is_some()
        {
            return Err(invalid("String read native transport differs"));
        }
        let Some(geometry) = self.retained_caller_geometry(site)? else {
            return Ok(false);
        };
        let source = self
            .state
            .arena
            .copy_current(operands[0].get(), &mut self.state.control.work)?;
        let memory = self.state.library.memory_budget();
        let source_bytes = source
            .memory_budget_retained_bytes(&memory)
            .ok_or_else(|| invalid("String source is not admitted"))?;
        let source_nodes = source
            .memory_budget_retained_nodes(&memory)
            .ok_or_else(|| invalid("String source nodes are not admitted"))?;
        let coexistence = ResourceDemand {
            turn_peak_bytes: geometry
                .live_bytes
                .checked_sub(source_bytes)
                .ok_or_else(|| invalid("String source exceeds caller bytes"))?,
            retained_nodes: geometry
                .live_nodes
                .checked_sub(source_nodes)
                .ok_or_else(|| invalid("String source exceeds caller nodes"))?,
            ..ResourceDemand::default()
        };
        let result = call
            .string_matrix_element(
                &source,
                words[1],
                self.limits,
                &self.state.library,
                coexistence,
            )
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[site].origin(),
                error,
            })?;
        let Some(result) = result else {
            return Ok(false);
        };
        self.state
            .arena
            .put(output.get(), &result, &mut self.state.control.work)?;
        self.state.consumed = true;
        Ok(true)
    }
}
