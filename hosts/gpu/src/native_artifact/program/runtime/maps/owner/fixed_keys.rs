//! One private, current-funded fixed-key native owner. No source iteration,
//! branch, ordinal selection, value computation or successful store runs here.
use super::*;
use crate::native_artifact::program::protocol::OwnerOperation;
use crate::native_artifact::program::runtime::maps::admission::Error as StorageError;
use crate::native_artifact::program::runtime::maps::fixed_keys::{self, Frame, Lane};

impl Owner<'_> {
    pub(super) fn validate_fixed(&self) -> Result<(), Error> {
        let lane = self
            .fixed
            .as_ref()
            .ok_or_else(|| invalid("fixed-key lane missing"))?;
        let f = &lane.frame;
        if f.cells != lane.native.cells
            || f.count != lane.native.count
            || f.epoch == 0
            || f.epoch != lane.native.epoch
            || f.values != lane.value_address
            || f.token != lane.token
            || f.used_bytes != lane.counters.used_bytes
            || f.snapshot_nodes != lane.counters.snapshot_nodes
            || f.fixed_nodes != lane.fixed_nodes
            || Some(f.temporary_limit) != self.limits.max_temporary_bytes
            || Some(f.node_limit) != self.limits.max_retained_nodes
            || f.map_nodes
                != add(
                    f.count
                        .checked_mul(2)
                        .ok_or_else(|| invalid("fixed-key node price overflow"))?,
                    2,
                )?
            || f.updates > self.funded
        {
            return Err(invalid("fixed-key native frame changed"));
        }
        Ok(())
    }
    pub(super) fn fixed_call(
        &mut self,
        site: usize,
        op: OwnerOperation,
        args: [u64; 3],
    ) -> Result<u64, Error> {
        self.validate_fixed()?;
        if op == OwnerOperation::Refill {
            if site != self.header_site || args != [0; 3] {
                return Err(invalid("fixed-key refill outside header"));
            }
            let lane = self.fixed.as_ref().unwrap();
            let unit = (lane.compute, lane.comparison);
            let funded = self.funded;
            let next = self
                .work
                .with_unscoped_debit(|remaining| {
                    let count = (remaining.0 / unit.0.max(1) / 4)
                        .min(remaining.1 / unit.1.max(1) / 4)
                        .min(1024)
                        .max(1);
                    let next = funded.checked_add(count).ok_or(StorageError::Overflow)?;
                    Ok::<_, StorageError>((
                        unit.0.checked_mul(count).ok_or(StorageError::Overflow)?,
                        unit.1.checked_mul(count).ok_or(StorageError::Overflow)?,
                        (count, next),
                    ))
                })
                .map_err(storage_error)?
                .ok_or_else(|| invalid("fixed-key refill entered foreign work scope"))?;
            self.funded = next.1;
            #[cfg(test)]
            fixed_keys::record(2, 1);
            return Ok(next.0);
        }
        if op != OwnerOperation::FixedKeyFailure {
            return Err(invalid("unexpected fixed-key native callback"));
        }
        #[cfg(test)]
        fixed_keys::record(3, 1);
        let lane = self.fixed.as_ref().unwrap();
        let nodes = add(args[2], lane.fixed_nodes)?;
        let geometry = PublicationGeometry {
            live_bytes: args[1],
            live_nodes: nodes,
        };
        if site == self.selection_site {
            let fail = |bytes: u64, nodes: u64| -> Result<(), Error> {
                let demand = mech_core::ResourceDemand {
                    turn_peak_bytes: add(geometry.live_bytes, bytes)?,
                    retained_nodes: add(geometry.live_nodes, nodes)?,
                    ..Default::default()
                };
                if let Some(v) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::DirectCallPort {
                        call: 0,
                        direction: mech_core::PortDirection::Input,
                        port: 0,
                    },
                    demand,
                    0,
                    0,
                    self.limits,
                ) {
                    return Err(Error::Library {
                        origin: self.descriptor.metadata().plan.blocks()[site].origin(),
                        error:
                            mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                                mech_core::ResidentKernelError::ResourceLimit {
                                    phase: mech_core::ResidentResourcePhase::Admission,
                                    dimension: v.dimension,
                                    required: v.required,
                                    limit: v.limit,
                                },
                            ),
                    });
                }
                Ok(())
            };
            // Match the maintained producer's order: initial coexistence,
            // checked one-based ordinal, then the selected cell's footprint.
            fail(0, 0)?;
            if args[0] == 0 || args[0] > lane.frame.count {
                return Err(Error::Library {
                    origin: self.descriptor.metadata().plan.blocks()[site].origin(),
                    error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                        if args[0] == 0 || args[0] > mech_core::PORTABLE_SELECTOR_INDEX_MAX {
                            mech_core::ResidentKernelError::InvalidInput
                        } else {
                            mech_core::ResidentKernelError::IndexOutOfRange {
                                index: args[0],
                                upper_bound: lane.frame.count,
                            }
                        },
                    ),
                });
            }
            // SAFETY: the genuine lane owns this exact immutable cell table;
            // bounds were just checked and no native code runs during callback.
            let cell = unsafe {
                &*(lane.native.cells
                    as *const mech_engine::native_artifact::NativeStringMatrixKeyCell)
                    .add(args[0] as usize - 1)
            };
            fail(cell.logical_temporary_bytes, cell.logical_extra_nodes)?;
        } else {
            if site != lane.insert {
                return Err(invalid("fixed-key failure outside selected call"));
            }
            self.binding
                .fixed_call_geometry(geometry, self.limits)
                .map_err(storage_error)?;
        }
        Err(invalid("fixed-key guard denied an allowed operation"))
    }
}
impl<'plan> State<'plan> {
    pub(super) fn try_begin_fixed(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, 'plan>,
        region: usize,
    ) -> Result<Option<[u64; 4]>, Error> {
        let closed = &self.descriptor.metadata().closed_tails.regions[region];
        let owner = closed
            .map
            .ok_or_else(|| invalid("fixed-key closed owner missing"))?;
        let recipe = self
            .descriptor
            .metadata()
            .fixed_key_loops
            .owner(owner)
            .ok_or_else(|| invalid("fixed-key complete proof missing"))?;
        if ctx.library().unscoped_remaining().is_none() {
            return Ok(None);
        }
        if !self.slots[owner]
            .binding
            .as_ref()
            .is_some_and(Binding::has_borrowed_keys)
        {
            return Ok(None);
        }
        let (id, _) = self
            .descriptor
            .metadata()
            .matrix_map_keys
            .consumer(recipe.insert)
            .ok_or_else(|| invalid("fixed-key lane not selected"))?;
        if self.matrix_keys[id].pending.is_some() {
            return Err(invalid("fixed-key entry overlaps pending key"));
        }
        let Some(view) = self.matrix_keys[id].view.as_ref() else {
            return Ok(None);
        };
        ctx.arena_check_virtual(
            self.matrix_keys[id]
                .view_handle
                .ok_or_else(|| invalid("fixed-key source generation missing"))?,
        )?;
        let consumer = self.descriptor.metadata().calls[recipe.insert]
            .as_ref()
            .ok_or_else(|| invalid("fixed-key consumer factory missing"))?;
        let native = match view.native_view_for_consumer(consumer, ctx.library()) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let source = view.snapshot().clone();
        let Some(_geometry) = self.geometry(ctx, recipe.initialize)? else {
            return Ok(None);
        };
        let bytes = (size_of::<Owner<'plan>>() + 128) as u64;
        let lane_bytes = size_of::<Lane>() as u64;
        ctx.work().charge(
            Phase::Setup,
            add(
                add(bytes, lane_bytes)?
                    .checked_mul(64)
                    .ok_or_else(|| invalid("fixed-key owner setup overflows"))?,
                1024,
            )?,
        )?;
        let memory = ctx.library().memory_budget().with_limits(
            ctx.limits().max_temporary_bytes.unwrap_or(0),
            ctx.limits().max_retained_nodes.unwrap_or(0),
        );
        let created = (|| -> Result<_, StorageError> {
            Ok((
                memory.reserve_capacity(bytes)?,
                memory.reserve_nodes(bytes)?,
                memory.reserve_capacity(lane_bytes)?,
                memory.reserve_nodes(1)?,
            ))
        })();
        let (reservation, private_nodes, metadata, lane_nodes) = match created {
            Ok(v) => v,
            Err(StorageError::Memory(_)) => return Ok(None),
            Err(e) => return Err(storage_error(e)),
        };
        let base = self
            .caller
            .as_ref()
            .ok_or_else(|| invalid("fixed-key caller census missing"))?
            .fixed_nodes;
        let fixed_nodes = add(
            add(
                add(
                    base,
                    ctx.retained_caller_dynamic_private_nodes(recipe.initialize)?,
                )?,
                self.private_geometry_nodes(ctx)?,
            )?,
            add(bytes, 1)?,
        )?;
        let geometry = PublicationGeometry {
            live_bytes: memory.used_bytes(),
            live_nodes: add(fixed_nodes, memory.retained_snapshot_nodes())?,
        };
        // SAFETY: native_view_for_consumer checked this genuine current loan;
        // State retains it unchanged until ownership moves into Lane below.
        let quote =
            match unsafe { fixed_keys::quote(native, geometry, ctx.limits(), ctx.library()) } {
                Ok(Some(q)) => q,
                Ok(None)
                | Err(
                    StorageError::Work(_)
                    | StorageError::Limit { .. }
                    | StorageError::Memory(_)
                    | StorageError::Allocation,
                ) => return Ok(None),
                Err(e) => return Err(storage_error(e)),
            };
        if ctx.limits().max_compute_work.unwrap_or(0) < 512 {
            return Ok(None);
        }
        let values = match self.slots[owner]
            .binding
            .as_mut()
            .unwrap()
            .fixed_native_values(&source, geometry, ctx.limits())
        {
            Ok(Some(v)) => v,
            Ok(None)
            | Err(
                StorageError::Limit { .. } | StorageError::Memory(_) | StorageError::Allocation,
            ) => return Ok(None),
            Err(e) => return Err(storage_error(e)),
        };
        if values.1 != native.count {
            return Err(invalid("fixed-key native lengths differ"));
        }
        // Exact native allocations and node claims precede allocation. Nothing
        // is removed from State until all fallible preparation has succeeded.
        let allocated = (|| -> Result<_, StorageError> {
            // SAFETY: exact nonzero Owner/Lane metadata and nodes held above.
            Ok((
                unsafe { fixed_keys::allocate_one::<Owner<'plan>>()? },
                unsafe { fixed_keys::allocate_one::<Lane>()? },
            ))
        })();
        let (mut owner_storage, mut lane_storage) = match allocated {
            Ok(v) => v,
            Err(StorageError::Allocation) => return Ok(None),
            Err(e) => return Err(storage_error(e)),
        };
        let compute = closed
            .compute
            .checked_add(quote.compute)
            .and_then(|n| n.checked_add(512))
            .ok_or_else(|| invalid("fixed-key unit work overflows"))?;
        let counters = memory.native_counters();
        let frame = Frame {
            cells: native.cells,
            count: native.count,
            values: values.0,
            token: values.2,
            updates: 0,
            epoch: native.epoch,
            used_bytes: counters.used_bytes,
            snapshot_nodes: counters.snapshot_nodes,
            fixed_nodes,
            temporary_limit: ctx.limits().max_temporary_bytes.unwrap(),
            node_limit: ctx.limits().max_retained_nodes.unwrap(),
            map_nodes: native
                .count
                .checked_mul(2)
                .and_then(|n| n.checked_add(2))
                .ok_or_else(|| invalid("fixed-key output nodes overflow"))?,
        };
        let view = self.matrix_keys[id].view.take().unwrap();
        lane_storage.write(Lane {
            frame,
            view,
            lane: id,
            compute,
            comparison: quote.comparison,
            insert: recipe.insert,
            native,
            value_address: values.0,
            token: values.2,
            counters,
            fixed_nodes,
            _metadata: metadata,
            _nodes: lane_nodes,
        });
        // SAFETY: exactly one complete Lane was written; its Box never moves
        // its allocation during the synchronous generated loan.
        let mut lane = unsafe { lane_storage.assume_init() };
        let pointer = (&mut lane.frame as *mut Frame) as u64;
        let binding = self.slots[owner].take_binding().unwrap();
        owner_storage.write(Owner {
            packed_frame: pointer,
            packed: None,
            fixed: Some(lane),
            packed_ssa_entered: false,
            guarded_loop_entered: false,
            guarded_calls_entered: false,
            header_site: closed.tail.header() as usize,
            binding,
            source,
            range: self.descriptor.metadata().calls[recipe.cell]
                .as_ref()
                .unwrap(),
            selection_site: recipe.cell,
            descriptor: self.descriptor,
            selector: None,
            selector_current: false,
            key: None,
            probe: None,
            work: ctx.library().clone(),
            limits: ctx.limits(),
            outside_nodes: 0,
            body_work: closed.compute,
            funded: 0,
            updates: 0,
            result: ctx.result_lane(),
            error: None,
            _reservation: reservation,
            _nodes: private_nodes,
        });
        // SAFETY: exactly one complete Owner was written above. It owns the
        // bank, real key loan, fixed frame, account and every admission claim.
        let mut owner = unsafe { owner_storage.assume_init() };
        let pointer = (&mut *owner as *mut Owner<'plan>) as u64;
        self.slots[closed.map.unwrap()].close(owner);
        #[cfg(test)]
        fixed_keys::record(0, 1);
        Ok(Some([0, 0, call as *const () as u64, pointer]))
    }
}
