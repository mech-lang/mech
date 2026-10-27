//! Preserve original visits while retaining one admitted immutable line table.
//! The matrix slot stays empty; no canonical Value represents this private view.
use crate::native_artifact::program::prepare::line_matrix::Site;
use crate::native_artifact::program::runtime::invocation::Invocation;
use crate::native_artifact::program::runtime::{
    NativeProgramExecutionError as Error, amount, invalid,
};
use crate::native_artifact::{NativeExecutionPhase, NativeProgramDescriptor, NativeWorkMeter};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, ResourceDemand};
use mech_engine::native_artifact::{
    NativeMatchValue, NativeTextLineMatrix, NativeTextLineMatrixChoice,
};
use std::rc::Rc;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::runtime::lines) mod tests;

pub(in crate::native_artifact::program::runtime) struct State {
    pub views: Box<[Option<Rc<NativeTextLineMatrix>>]>,
    pub captured: Option<Captured>,
    reservation: Option<ManagedMemoryReservation>,
    _metadata_nodes: Option<mech_core::ManagedNodeReservation>,
}
pub(in crate::native_artifact::program::runtime) struct Captured {
    pub ordinal: usize,
    pub produced: bool,
    pub length: Option<usize>,
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
        let count = descriptor.metadata().line_matrices.regions.len();
        if count == 0 {
            return Ok(Self {
                views: Box::new([]),
                captured: None,
                reservation: None,
                _metadata_nodes: None,
            });
        }
        let bytes = Self::metadata_bytes(count)?;
        if bytes > limit {
            return Err(invalid("line matrix owner lane exceeds transport cap"));
        }
        work.charge(NativeExecutionPhase::Setup, amount(count, 512, 1024)?)?;
        let reservation = memory.reserve_capacity(bytes)?;
        let metadata_nodes = memory.reserve_nodes(1)?;
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
            captured: None,
            reservation: Some(reservation),
            _metadata_nodes: Some(metadata_nodes),
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

    pub(in crate::native_artifact::program::runtime) fn parked_view_nodes(
        &self,
        memory: &ManagedMemoryBudget,
    ) -> Result<u64, Error> {
        self.retained_demand(None, memory)?
            .retained_nodes
            .checked_sub(u64::from(self.reservation.is_some()))
            .ok_or_else(|| invalid("line matrix private node inventory underflows"))
    }
}

#[path = "../invocation/line_calls.rs"]
mod line_calls;
