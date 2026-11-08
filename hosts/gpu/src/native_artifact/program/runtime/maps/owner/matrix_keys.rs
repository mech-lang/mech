//! Borrowed matrix cells retain the admitted source loan, never a String Value.
//! The original producer visit admits selection; the original insertion visit
//! consumes it against the current dictionary token and unchanged storage API.
use super::{State, add, invalid};
use crate::native_artifact::program::runtime::invocation::MapExecution;
use crate::native_artifact::program::runtime::maps::binding::Key;
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, amount};
use crate::native_artifact::{NativeExecutionPhase as Phase, NativeValueHandle};
use mech_core::{ResourceDemand, Value};
use mech_engine::native_artifact::{NativeStringMatrixKeyLoan, NativeStringMatrixKeyOrdinal};

#[derive(Default)]
pub(super) struct Lane {
    pub(super) view: Option<mech_engine::native_artifact::NativeBorrowedMapKeys>,
    pub(super) view_handle: Option<NativeValueHandle>,
    pub(super) handle: Option<NativeValueHandle>,
    loan: Option<NativeStringMatrixKeyLoan>,
    pub(super) pending: Option<NativeStringMatrixKeyOrdinal>,
    declined: bool,
}
impl Lane {
    pub(super) fn ready(&self) -> bool {
        self.view.is_some() || (!self.declined && self.handle.is_some() && self.loan.is_some())
    }
    pub(super) fn nodes(&self) -> u64 {
        self.loan
            .as_ref()
            .map_or(0, |loan| loan.retained_metadata_demand().retained_nodes)
            + self
                .view
                .as_ref()
                .map_or(0, |view| view.retained_metadata_demand().retained_nodes)
    }
    pub(super) fn clear(&mut self) {
        let view = self.view.take();
        let view_handle = self.view_handle;
        *self = Self::default();
        self.view = view;
        self.view_handle = view_handle;
    }
}

impl State<'_> {
    pub(super) fn matrix_key_nodes(&self) -> Result<u64, Error> {
        self.matrix_keys
            .iter()
            .try_fold(0, |total, lane| add(total, lane.nodes()))
    }
    pub(super) fn project_matrix_key(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
        index: u64,
    ) -> Result<bool, Error> {
        self.guard(ctx, site)?;
        let metadata = self.descriptor.metadata();
        ctx.work().charge(
            Phase::Setup,
            amount(metadata.matrix_map_keys.regions.len(), 64, 256)?,
        )?;
        let (id, proof) = metadata
            .matrix_map_keys
            .producer(site)
            .ok_or_else(|| invalid("matrix key outside complete producer proof"))?;
        let owner = self.region(proof.owner)?;
        if ctx.control().current != Some(owner.iteration())
            || self.slots[proof.owner].binding.is_none()
        {
            return Err(invalid("matrix key outside active retained owner"));
        }
        if self.matrix_keys[id].pending.is_some() {
            return Err(invalid("matrix key producer repeated before insertion"));
        }
        if self.matrix_keys[id].declined {
            return Ok(false);
        }
        let Some(geometry) = self.geometry(ctx, site)? else {
            return Ok(false);
        };
        if self.matrix_keys[id].view.is_some() {
            ctx.arena_check_virtual(
                self.matrix_keys[id]
                    .view_handle
                    .ok_or_else(|| invalid("key view lost generation"))?,
            )?;
            let view = self.matrix_keys[id].view.as_mut().unwrap();
            let memory = ctx.library().memory_budget();
            let source_bytes = view
                .snapshot()
                .memory_budget_retained_bytes(&memory)
                .ok_or_else(|| invalid("unadmitted key-view snapshot"))?;
            let source_nodes = view
                .snapshot()
                .memory_budget_retained_nodes(&memory)
                .ok_or_else(|| invalid("unadmitted key-view snapshot nodes"))?;
            let held = view.retained_metadata_demand();
            let coexistence = ResourceDemand {
                turn_peak_bytes: geometry
                    .live_bytes
                    .checked_sub(add(source_bytes, held.turn_peak_bytes)?)
                    .ok_or_else(|| invalid("key view exceeds caller bytes"))?,
                retained_nodes: geometry
                    .live_nodes
                    .checked_sub(add(source_nodes, held.retained_nodes)?)
                    .ok_or_else(|| invalid("key view exceeds caller nodes"))?,
                ..ResourceDemand::default()
            };
            let ordinal = view
                .admit_element(index, ctx.limits(), ctx.library(), coexistence)
                .map_err(|error| Error::Library {
                    origin: metadata.plan.blocks()[site].origin(),
                    error,
                })?;
            self.matrix_keys[id].pending = Some(ordinal);
            self.last_owned_visit = Some(ctx.control().visits);
            return Ok(true);
        }
        let handle = ctx.arena_current_handle(proof.source.get())?;
        let source = ctx.arena_get(handle)?.clone();
        let memory = ctx.library().memory_budget();
        let source_bytes = source
            .memory_budget_retained_bytes(&memory)
            .ok_or_else(|| invalid("matrix key source is not admitted"))?;
        let source_nodes = source
            .memory_budget_retained_nodes(&memory)
            .ok_or_else(|| invalid("matrix key source nodes are not admitted"))?;
        let prior = self.matrix_keys[id]
            .loan
            .as_ref()
            .map_or(ResourceDemand::default(), |loan| {
                loan.retained_metadata_demand()
            });
        let coexistence = ResourceDemand {
            turn_peak_bytes: geometry
                .live_bytes
                .checked_sub(add(source_bytes, prior.turn_peak_bytes)?)
                .ok_or_else(|| invalid("matrix key source/loan exceeds caller bytes"))?,
            retained_nodes: geometry
                .live_nodes
                .checked_sub(add(source_nodes, prior.retained_nodes)?)
                .ok_or_else(|| invalid("matrix key source/loan exceeds caller nodes"))?,
            ..ResourceDemand::default()
        };
        let selected = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("matrix key lost selected producer"))?;
        let limits = ctx.limits();
        let origin = metadata.plan.blocks()[site].origin();
        let lane = &mut self.matrix_keys[id];
        let newly_admitted = lane.loan.is_none();
        if let Some(pinned) = lane.handle {
            if pinned != handle {
                return Err(invalid("matrix key source generation changed"));
            }
        } else {
            let Some(loan) = selected
                .try_string_matrix_key_loan(&source, limits, ctx.library(), coexistence)
                .map_err(|error| Error::Library { origin, error })?
            else {
                lane.declined = true;
                return Ok(false);
            };
            lane.loan = Some(loan);
            lane.handle = Some(handle);
        }
        let ordinal = lane
            .loan
            .as_mut()
            .unwrap()
            .admit_element(&source, index, limits, ctx.library(), coexistence)
            .map_err(|error| Error::Library { origin, error })?;
        lane.pending = Some(ordinal);
        // With every matrix producer admitted, this loop can no longer create
        // canonical String roots. Reconsider the existing complete native-use
        // census at this current visit, after optional loan refusal is settled.
        if newly_admitted && self.caller.is_none() {
            self.caller = self.capture_caller_at(ctx, proof.owner, site)?;
        }
        self.last_owned_visit = Some(ctx.control().visits);
        Ok(true)
    }

    /// A producer miss leaves the canonical key path intact. A producer hit
    /// must be consumed once, at the unique proved insertion, with its exact
    /// current source generation and selected consumer schema.
    pub(super) fn insert_matrix_key(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
        token: u64,
        value: u64,
        geometry: crate::native_artifact::program::runtime::maps::string::PublicationGeometry,
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        if !metadata.matrix_map_keys.regions.is_empty() {
            ctx.work().charge(
                Phase::Setup,
                amount(metadata.matrix_map_keys.regions.len(), 64, 0)?,
            )?;
        }
        let Some((id, proof)) = metadata.matrix_map_keys.consumer(site) else {
            return Ok(None);
        };
        let lane = &mut self.matrix_keys[id];
        let Some(ordinal) = lane.pending.take() else {
            return Ok(None);
        };
        if let Some(view) = &lane.view {
            ctx.arena_check_virtual(
                lane.view_handle
                    .ok_or_else(|| invalid("key consumer lost virtual generation"))?,
            )?;
            let consumer = metadata.calls[site]
                .as_ref()
                .ok_or_else(|| invalid("map key view lost consumer"))?;
            let binding = self.slots[proof.owner]
                .binding
                .as_mut()
                .ok_or_else(|| invalid("map key view lost dictionary"))?;
            return view
                .with_element(consumer, &ordinal, ctx.library(), |text| {
                    binding.insert(
                        site,
                        token,
                        Key::MapCell {
                            value: proof.output,
                            owner: view.snapshot(),
                            text,
                        },
                        value,
                        geometry,
                        ctx.limits(),
                    )
                })
                .map_err(|error| Error::Library {
                    origin: metadata.plan.blocks()[site].origin(),
                    error,
                })?
                .map(Some)
                .map_err(super::storage_error);
        }
        let handle = lane
            .handle
            .ok_or_else(|| invalid("matrix key lost source generation"))?;
        if ctx.arena_current_handle(proof.source.get())? != handle {
            return Err(invalid("matrix key consumer has stale source"));
        }
        let source: Value = ctx.arena_get(handle)?.clone();
        let loan = lane
            .loan
            .as_ref()
            .ok_or_else(|| invalid("matrix key consumer lost loan"))?;
        let consumer = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("matrix key lost selected consumer"))?;
        let binding = self.slots[proof.owner]
            .binding
            .as_mut()
            .ok_or_else(|| invalid("matrix key consumer lost dictionary owner"))?;
        let limits = ctx.limits();
        let origin = metadata.plan.blocks()[site].origin();
        loan.with_element(consumer, &source, &ordinal, ctx.library(), |text| {
            binding.insert(
                site,
                token,
                Key::MatrixCell {
                    value: proof.output,
                    owner: &source,
                    text,
                },
                value,
                geometry,
                limits,
            )
        })
        .map_err(|error| Error::Library { origin, error })?
        .map(Some)
        .map_err(super::storage_error)
    }
}
