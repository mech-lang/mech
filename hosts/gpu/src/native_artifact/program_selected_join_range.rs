//! Private range geometry at the original selected producer visit.
use super::*;

impl Context<'_, '_> {
    pub(super) fn prepare_selected_join_range(
        &mut self,
        site: usize,
        index: usize,
        lines: Rc<NativeTextLineMatrix>,
        words: [u64; 2],
    ) -> Result<(), Error> {
        let descriptor = self.control.descriptor;
        let region = &descriptor.selected_joins.regions[index];
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("selected range has no prepared factory"))?;
        let state_bytes = size_of::<State>() as u64;
        let witness_bytes = call.selected_u64_string_span_selector_metadata_bytes();
        let metadata = add(state_bytes, witness_bytes)?;
        let price = add(
            call.selected_u64_string_span_selector_work(),
            add(mul(size_of::<State>() as u64, 8)?, 256)?,
        )?;
        let mut prior = self.selected_join_coexistence(None)?;
        prior.turn_peak_bytes = add(prior.turn_peak_bytes, metadata)?;
        prior.retained_nodes = add(prior.retained_nodes, 1)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        range_tests::before_range_admission(self, site, prior, metadata, price);
        let compute_limit = self
            .limits
            .max_compute_work
            .ok_or_else(|| invalid("selected range requires finite compute"))?;
        let comparison_limit = self
            .limits
            .max_comparison_work
            .ok_or_else(|| invalid("selected range requires finite comparison work"))?;
        let (selector, reservation) = self
            .library
            .with_call_limits(compute_limit, comparison_limit, || {
                require(
                    self,
                    site,
                    Dimension::TemporaryBytes,
                    prior.turn_peak_bytes,
                    self.limits.max_temporary_bytes,
                )?;
                require(
                    self,
                    site,
                    Dimension::RetainedNodes,
                    prior.retained_nodes,
                    self.limits.max_retained_nodes,
                )?;
                self.library
                    .charge(price, 0)
                    .map_err(|error| library(self, site, error.into()))?;
                let reservation = self
                    .library
                    .memory_budget()
                    .reserve_capacity(state_bytes)
                    .map_err(|error| library(self, site, error.into()))?;
                // The actual Range shape/witness scratch dies in this call.
                // Carry only the inline pending State reservation onward;
                // the later selection admits its own real witness scratch.
                // Retiring capacity does not refund any cumulative work.
                let _witness = self
                    .library
                    .memory_budget()
                    .reserve_capacity(witness_bytes)
                    .map_err(|error| library(self, site, error.into()))?;
                // Same finite account and maintained cardinality/shape/layout
                // checks as other compact selected ranges. No selector Value,
                // payload allocation, renewed allowance or hidden visit.
                let selector = call
                    .selected_u64_string_span_selector(words[0], words[1])
                    .map_err(|error| library(self, site, error))?;
                Ok::<_, Error>((selector, reservation))
            })
            .map_err(|error| library(self, site, error.into()))??;
        // Retire stale results from a previous ordinary lexical invocation
        // only after the current original range succeeds. Both slots stay
        // empty throughout this private producer/consumer sequence.
        self.arena
            .retire_current(region.selector().get(), &mut self.control.work)?;
        self.arena
            .retire_current(region.selected().get(), &mut self.control.work)?;
        self.selected_join = Some(State {
            phase: Phase::Range { lines, selector },
            region: index,
            parent: self.control.current,
            reservation,
        });
        #[cfg(all(test, feature = "canonical-native-program"))]
        range_tests::after_range(self, index);
        Ok(())
    }
}
