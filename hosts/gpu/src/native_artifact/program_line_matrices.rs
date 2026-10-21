//! Preserve original visits while retaining one admitted immutable line table.
//! The matrix slot stays empty; no canonical Value represents this private view.
use super::{Context, NativeProgramExecutionError as Error, amount, invalid};
use crate::native_artifact::{
    NativeExecutionPhase, NativeProgramDescriptor, NativeWorkMeter,
    program_line_matrix_layout::Site,
};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, ResourceDemand};
use mech_engine::native_artifact::{
    NativeMatchValue, NativeTextLineMatrix, NativeTextLineMatrixChoice,
};
use std::rc::Rc;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_line_matrix_tests.rs"]
mod tests;

pub(super) struct State {
    pub views: Box<[Option<Rc<NativeTextLineMatrix>>]>,
    reservation: Option<ManagedMemoryReservation>,
}
impl State {
    pub fn metadata_bytes(count: usize) -> Result<u64, Error> {
        if count == 0 {
            return Ok(0);
        }
        amount(
            count,
            (2 * size_of::<Option<Rc<NativeTextLineMatrix>>>()) as u64,
            128,
        )
    }
    pub fn new(
        descriptor: &NativeProgramDescriptor,
        limit: u64,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, Error> {
        let count = descriptor.line_matrices.regions.len();
        if count == 0 {
            return Ok(Self {
                views: Box::new([]),
                reservation: None,
            });
        }
        let bytes = Self::metadata_bytes(count)?;
        if bytes > limit {
            return Err(invalid("line matrix owner lane exceeds transport cap"));
        }
        work.charge(NativeExecutionPhase::Setup, amount(count, 512, 1024)?)?;
        let reservation = memory.reserve_capacity(bytes)?;
        let mut views = Vec::new();
        views
            .try_reserve_exact(count)
            .map_err(|_| invalid("line matrix owner lane allocation"))?;
        if views.capacity() != count {
            return Err(invalid("line matrix owner lane capacity"));
        }
        views.resize_with(count, || None);
        Ok(Self {
            views: views.into_boxed_slice(),
            reservation: Some(reservation),
        })
    }
    pub fn retained_demand(
        &self,
        skip: Option<usize>,
        memory: &ManagedMemoryBudget,
    ) -> Result<ResourceDemand, Error> {
        let mut demand = ResourceDemand::default();
        if let Some(reservation) = &self.reservation {
            demand.turn_peak_bytes = reservation.capacity_bytes();
            demand.retained_nodes = 1;
        }
        for (ordinal, view) in self.views.iter().enumerate() {
            if skip == Some(ordinal) {
                continue;
            }
            if let Some(view) = view {
                demand.turn_peak_bytes = demand
                    .turn_peak_bytes
                    .checked_add(
                        view.retained_bytes(memory)
                            .map_err(|_| invalid("line matrix owner admission changed"))?,
                    )
                    .ok_or_else(|| invalid("line matrix retained bytes overflow"))?;
                demand.retained_nodes = demand
                    .retained_nodes
                    .checked_add(view.count() as u64)
                    .and_then(|n| n.checked_add(2))
                    .ok_or_else(|| invalid("line matrix retained nodes overflow"))?;
            }
        }
        Ok(demand)
    }
}

impl Context<'_, '_> {
    pub(super) fn line_matrix(&self, value: NativeMatchValue) -> Option<&Rc<NativeTextLineMatrix>> {
        let ordinal = self
            .control
            .descriptor
            .line_matrices
            .matrices
            .get(value.get() as usize)
            .copied()
            .flatten()?;
        self.line_matrices.views.get(ordinal)?.as_ref()
    }
    pub(super) fn line_matrix_coexistence(
        &mut self,
        skip: Option<NativeMatchValue>,
    ) -> Result<ResourceDemand, Error> {
        if self.line_matrices.views.is_empty() {
            return Ok(ResourceDemand::default());
        }
        self.control.work.charge(
            NativeExecutionPhase::Setup,
            amount(self.line_matrices.views.len(), 256, 256)?,
        )?;
        let ordinal = skip
            .and_then(|value| self.control.descriptor.line_matrices.matrices[value.get() as usize]);
        self.line_matrices
            .retained_demand(ordinal, &self.library.memory_budget())
    }
    pub(super) fn line_matrix_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<Option<Option<u64>>, Error> {
        let descriptor = self.control.descriptor;
        let Some(role) = descriptor.line_matrices.sites[site] else {
            return Ok(None);
        };
        if words != [0] {
            return Err(invalid("line matrix managed operand lane differs"));
        }
        self.control.work.charge(NativeExecutionPhase::Setup, 512)?;
        let ordinal = match role {
            Site::Produce(index) | Site::Length(index) => index,
        };
        let region = &descriptor.line_matrices.regions[ordinal];
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("line matrix selected call absent"))?;
        let prior = self.selected_join_coexistence(Some(region.matrix()))?;
        let failure = |error| Error::Library {
            origin: descriptor.plan.blocks()[site].origin(),
            error,
        };
        match role {
            Site::Produce(_) => {
                if self.line_matrices.views[ordinal].is_some() {
                    return Err(invalid("line matrix producer repeated"));
                }
                let source = self
                    .arena
                    .copy_current(region.source().get(), &mut self.control.work)?;
                match call
                    .text_line_matrix_or_canonical(&source, self.limits, &self.library, prior)
                    .map_err(failure)?
                {
                    NativeTextLineMatrixChoice::View(view) => {
                        if !view.is_ascii() {
                            return Err(invalid("native line view lacks byte identity"));
                        }
                        self.arena
                            .retire_current(region.matrix().get(), &mut self.control.work)?;
                        self.line_matrices.views[ordinal] = Some(Rc::new(view));
                    }
                    NativeTextLineMatrixChoice::Canonical(value) => {
                        self.arena
                            .put(region.matrix().get(), &value, &mut self.control.work)?;
                    }
                }
                #[cfg(all(test, feature = "canonical-native-program"))]
                tests::observe(self, ordinal);
                Ok(Some(None))
            }
            Site::Length(_) => {
                let Some(view) = self.line_matrices.views[ordinal].as_ref() else {
                    return Ok(None);
                };
                let count = call
                    .text_line_matrix_length(view, self.limits, &self.library, prior)
                    .map_err(failure)?;
                Ok(Some(Some(count)))
            }
        }
    }
}
