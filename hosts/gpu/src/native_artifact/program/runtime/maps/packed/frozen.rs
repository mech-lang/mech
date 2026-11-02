//! Complete immutable packed storage after generated mutation authority ends.
//! No key is discarded, including keys never requested by subsequent reads.
use crate::native_artifact::program::runtime::maps::packed::*;
use crate::native_artifact::program::runtime::maps::string::{
    logical_map_bytes, logical_map_limits, string_u64_schema,
};
use mech_core::{SchemaId, SchemaTable};

pub(in crate::native_artifact) struct FrozenPackedMap {
    // No mutable frame or table accessor escapes this type. The original
    // same-account backing/node reservations remain with the entire table.
    owner: PackedMap,
    updates: u64,
    traversals: u64,
}

impl PackedMap {
    /// Validate the complete map at the original closed-tail exit, before a
    /// later source instruction runs. Logical publication caps are checked at
    /// the original result-copy boundary, not moved before its visit debit.
    pub(in crate::native_artifact) fn freeze(mut self) -> Result<FrozenPackedMap, Error> {
        let work = self.work.clone();
        work.with_call_limits(
            cap(self.limits.max_compute_work)?,
            cap(self.limits.max_comparison_work)?,
            || {
                self.validate_frame()?;
                let traversals = self
                    .funded
                    .checked_sub(self.frame.fuel)
                    .ok_or(Error::Overflow)?;
                if self.frame.updates > mul(traversals, self.body.writes)? {
                    return Err(Error::Binding(
                        "packed-map updates exceed source write sites",
                    ));
                }
                // Validate every slot and complete key. This allocation-free leaf
                // uses bounded exact debits: lock ownership is released after the
                // established small charge/work threshold, with no speculative
                // credits and no per-entry call-cap renewal.
                work.charge(512, 0)?;
                work.with_debit_batch(|batch| -> Result<(), Error> {
                    let mut count = 0;
                    for (index, slot) in self.slots.iter().enumerate() {
                        batch.charge(64, 0)?;
                        let control = self.controls[index];
                        if control == EMPTY {
                            continue;
                        }
                        batch.charge(mul(self.width, 64)?, 0)?;
                        if control != fingerprint(hash(slot.low, slot.high))
                            || !self.valid_key(slot.low, slot.high)
                        {
                            return Err(Error::Binding("invalid frozen packed-map entry"));
                        }
                        // A duplicate or an entry hidden behind an earlier empty
                        // group cannot resolve to this exact occupied slot.
                        if Self::probe_debits(
                            &self.slots,
                            &self.controls,
                            slot.low,
                            slot.high,
                            &mut |compute, comparison| batch.charge(compute, comparison),
                        )? != index
                        {
                            return Err(Error::Binding(
                                "duplicate or unreachable frozen packed-map entry",
                            ));
                        }
                        count = add(count, 1)?;
                    }
                    if count != self.frame.len {
                        return Err(Error::Binding("frozen packed-map count mismatch"));
                    }
                    Ok(())
                })??;
                let updates = self.frame.updates;
                // The source pin belongs to the retired mutation owner. Frozen
                // lookup uses retained keys only and must carry no dangling view.
                self.frame.source_ptr = 0;
                self.frame.source_len = 0;
                self.frame.previous_start = u64::MAX;
                self.frame.entry_pointer = 0;
                self.frame.entry_known = 0;
                Ok(FrozenPackedMap {
                    owner: self,
                    updates,
                    traversals,
                })
            },
        )?
    }
}

impl FrozenPackedMap {
    pub(in crate::native_artifact) fn updates(&self) -> u64 {
        self.updates
    }
    pub(in crate::native_artifact) fn traversals(&self) -> u64 {
        self.traversals
    }
    pub(in crate::native_artifact) fn retained_nodes(&self) -> u64 {
        self.owner.nodes.nodes()
    }

    pub(in crate::native_artifact) fn admit_output(
        &self,
        schemas: &SchemaTable,
        schema: SchemaId,
        limits: MemoryBudgetLimits,
    ) -> Result<(), Error> {
        finite(limits, cap)?;
        self.owner.work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                self.owner.work.charge(512, 0)?;
                string_u64_schema(schemas, schema, self.owner.frame.len)?;
                logical_map_limits(
                    logical_map_bytes(
                        self.owner.frame.len,
                        mul(self.owner.frame.len, self.owner.width)?,
                    )?,
                    limits,
                )?;
                let memory = self.owner.work.memory_budget();
                check(
                    memory.used_bytes(),
                    cap(limits.max_temporary_bytes)?,
                    "temporary bytes",
                )?;
                check(
                    memory.retained_nodes(),
                    cap(limits.max_retained_nodes)?,
                    "retained nodes",
                )
            },
        )?
    }

    /// Called inside the read's one source call scope, which also funds typed
    /// scalar output construction. Absence is distinct from a stored zero.
    pub(in crate::native_artifact) fn lookup(&self, query: &str) -> Result<Option<u64>, Error> {
        let map = &self.owner;
        map.validate_frame()?;
        let bytes = u64::try_from(query.len()).map_err(|_| Error::Overflow)?;
        map.work.charge(add(mul(bytes, 64)?, 256)?, bytes)?;
        if bytes != map.width {
            return Ok(None);
        }
        let mut key = 0u128;
        let bits = map.domain.bits();
        let shift = map
            .domain
            .shift()
            .ok_or(Error::Binding("frozen packed-map domain lacks bit window"))?;
        for byte in query.bytes() {
            // Membership precedes masking. Non-ASCII and out-of-domain bytes
            // must never alias an admitted packed key with matching low bits.
            if !map.domain.contains(byte) {
                return Ok(None);
            }
            key = (key << bits) | u128::from((byte >> shift) & ((1u8 << bits) - 1));
        }
        let low = key as u64;
        let high = (key >> 64) as u64;
        let index = PackedMap::probe_debits(
            &map.slots,
            &map.controls,
            low,
            high,
            &mut |compute, comparison| map.work.charge(compute, comparison),
        )?;
        if map.controls[index] == EMPTY {
            return Ok(None);
        }
        let slot = map.slots.get(index);
        if slot.low != low || slot.high != high {
            return Err(Error::Binding("frozen packed-map probe differs"));
        }
        Ok(Some(slot.value))
    }
}
