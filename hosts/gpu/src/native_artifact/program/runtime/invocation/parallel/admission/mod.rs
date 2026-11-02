//! Shared admission for retained worker state and a suspended parent. No
//! source work is performed here; unsupported private strategies stay serial.
use crate::native_artifact::program::runtime::invocation::Invocation;
use crate::native_artifact::program::runtime::{
    NativeProgramExecutionError as Error, amount, invalid,
};
use crate::native_artifact::{NativeExecutionPhase, NativeProgramDescriptor};
use mech_core::ManagedNodeReservation;
use mech_engine::native_artifact::{NativeCollectionResult, NativeIterationAction};

pub(in crate::native_artifact::program::runtime) fn supported(
    descriptor: &NativeProgramDescriptor,
    id: usize,
) -> bool {
    let Some(region) = descriptor.metadata().parallel.regions.get(id) else {
        return false;
    };
    for &site in &region.sites {
        let site = site as usize;
        if descriptor.metadata().line_matrices.sites[site].is_some()
            || descriptor.metadata().selected_joins.sites[site].is_some()
            || descriptor.metadata().borrowed_strings.initializers[site].is_some()
            || descriptor
                .metadata()
                .borrowed_strings
                .action(site)
                .is_some()
        {
            return false;
        }
        if let NativeIterationAction::CollectionBegin { collection } =
            descriptor.metadata().plan.blocks()[site].action()
        {
            let collection = &descriptor.metadata().plan.collections()[*collection as usize];
            if collection.string_span_projection().is_some()
                || collection.string_element_map_projection().is_some()
                || !matches!(
                    collection.result(),
                    NativeCollectionResult::Materialized | NativeCollectionResult::TextJoin(_)
                )
            {
                return false;
            }
        }
    }
    true
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;

impl Invocation<'_, '_, '_> {
    /// The caller must retain this claim until every scoped worker has joined.
    /// Fixed invocation roots, collection/cache lanes, materialized drafts,
    /// TextJoin, cache metadata, canonical Values and private maps already own
    /// independent claims. Only frozen, otherwise-unclaimed parent views are
    /// added here; never its entire outside geometry or shared snapshot census.
    pub(in crate::native_artifact::program::runtime) fn parallel_admission(
        &mut self,
        regions: &[usize],
    ) -> Result<Option<ManagedNodeReservation>, Error> {
        if regions.is_empty()
            || self.worker.is_some()
            || self.line_window.is_some()
            || self.window_region.is_some()
            || self.window_hashes.is_some()
            || self.closed_tail.is_some()
            || self.borrowed_strings.is_some()
            || self.selected_join.is_some()
            || self.prefix.as_ref().is_some_and(|prefix| !prefix.input.eof)
        {
            return Ok(None);
        }
        let count = regions.iter().try_fold(0usize, |sum, &id| {
            let region = self
                .descriptor
                .metadata()
                .parallel
                .regions
                .get(id)
                .ok_or_else(|| invalid("parallel admission region outside descriptor"))?;
            sum.checked_add(region.sites.len())
                .ok_or_else(|| invalid("parallel admission sites overflow"))
        })?;
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, amount(count, 128, 512)?)?;
        if regions.iter().any(|&id| !supported(self.descriptor, id)) {
            return Ok(None);
        }
        let memory = self.state.library.memory_budget().with_limits(
            self.limits
                .max_temporary_bytes
                .ok_or_else(|| invalid("parallel admission requires finite bytes"))?,
            self.limits
                .max_retained_nodes
                .ok_or_else(|| invalid("parallel admission requires finite nodes"))?,
        );
        let collection = if let Some(collection) = &self.state.collections {
            let Some(nodes) = collection.parked_private_nodes(&mut self.state.control.work)? else {
                return Ok(None);
            };
            nodes
        } else {
            0
        };
        let nodes = collection
            .checked_add(self.state.line_matrices.parked_view_nodes(&memory)?)
            .ok_or_else(|| invalid("parallel parent node inventory overflows"))?;
        Ok(memory.reserve_nodes(nodes).ok())
    }
}
