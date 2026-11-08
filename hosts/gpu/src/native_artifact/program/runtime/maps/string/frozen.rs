//! Immutable ownership of a completed generic UTF8/U64 table. No source
//! traversal or Map operation is evaluated by this representation bridge.
use super::publication::{Geometry, logical_map_bytes, logical_map_limits, string_u64_schema};
use super::*;
use crate::native_artifact::program::runtime::maps::admission::finite;
use mech_core::{MemoryBudgetLimits, SchemaId, SchemaTable, Value};
use std::sync::Arc;

pub(in crate::native_artifact) enum FreezeChoice<S = DefaultHashBuilder> {
    Frozen(FrozenStringMap<S>),
    Canonical(Value),
}

pub(in crate::native_artifact) struct FrozenStringMap<S = DefaultHashBuilder> {
    // Actual backing retires before its reservations. The table's Box<str>
    // keys remain at stable addresses when this owner moves into a lookup loan.
    owner: OwnedStringMap<S>,
    schemas: Arc<SchemaTable>,
    schema: SchemaId,
    original_limits: MemoryBudgetLimits,
    spent: (u64, u64),
    logical_bytes: u64,
    metadata: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
}
fn cap(n: Option<u64>) -> Result<u64, Error> {
    n.ok_or(Error::Publication(
        "unbounded deferred publication dimension",
    ))
}
impl<S: BuildHasher> OwnedStringMap<S> {
    /// The immediate refusal and ordinary publication share ONE original call
    /// scope. A nested relative cap cannot become a suspendable permission.
    #[cfg(test)]
    pub(in crate::native_artifact) fn freeze_or_publish(
        self,
        schemas: &Arc<SchemaTable>,
        schema: SchemaId,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<FreezeChoice<S>, Error> {
        finite(limits, cap)?;
        let work = self.work.clone();
        let enclosing = work.has_active_call_limit()?;
        let started = work.used();
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                self.freeze_or_publish_in_call(
                    schemas, schema, geometry, limits, started, !enclosing,
                )
            },
        )?
    }

    /// The retained binding will call this only within its original output-copy
    /// scope, passing that scope's REAL starting counters and enclosing status.
    /// This is crate-private storage plumbing, never a caller cost certificate.
    pub(in crate::native_artifact) fn freeze_or_publish_in_call(
        self,
        schemas: &Arc<SchemaTable>,
        schema: SchemaId,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
        started: (u64, u64),
        may_defer: bool,
    ) -> Result<FreezeChoice<S>, Error> {
        if !may_defer {
            return self
                .publish_result(schemas, schema, &[], geometry, limits)
                .map(FreezeChoice::Canonical);
        }
        finite(limits, cap)?;
        let retained = self.retained_bytes()?;
        if geometry.live_bytes < retained || geometry.live_nodes < self.nodes.nodes() {
            return Err(Error::Publication(
                "frozen caller geometry omits retained owner",
            ));
        }
        // The table is a safe typed HashMap<Box<str>,u64>; no native mutation
        // pointer ever escaped it. Its maintained count/key claims supply this
        // exact narrow logical layout without scanning or building Values.
        let extra = (size_of::<FrozenStringMap<S>>() - size_of::<Self>()) as u64;
        self.work.charge(add(mul(extra, 8)?, 512)?, 0)?;
        string_u64_schema(schemas, schema, self.len() as u64)?;
        let logical_bytes = logical_map_bytes(self.len() as u64, self.key_bytes())?;
        logical_map_limits(logical_bytes, limits)?;
        let fits = add(geometry.live_bytes, extra)? <= cap(limits.max_temporary_bytes)?
            && add(geometry.live_nodes, 1)? <= cap(limits.max_retained_nodes)?;
        if !fits {
            return self
                .publish_result(schemas, schema, &[], geometry, limits)
                .map(FreezeChoice::Canonical);
        }
        let memory = self.work.memory_budget().with_limits(
            cap(limits.max_temporary_bytes)?,
            cap(limits.max_retained_nodes)?,
        );
        let metadata = match memory.reserve_capacity(extra) {
            Ok(v) => v,
            Err(_) => {
                return self
                    .publish_result(schemas, schema, &[], geometry, limits)
                    .map(FreezeChoice::Canonical);
            }
        };
        let nodes = match memory.reserve_nodes(1) {
            Ok(v) => v,
            Err(_) => {
                drop(metadata);
                return self
                    .publish_result(schemas, schema, &[], geometry, limits)
                    .map(FreezeChoice::Canonical);
            }
        };
        let used = self.work.used();
        let spent = (
            used.0.checked_sub(started.0).ok_or(Error::Overflow)?,
            used.1.checked_sub(started.1).ok_or(Error::Overflow)?,
        );
        Ok(FreezeChoice::Frozen(FrozenStringMap {
            owner: self,
            schemas: schemas.clone(),
            schema,
            original_limits: limits,
            spent,
            logical_bytes,
            metadata,
            nodes,
        }))
    }
}
impl<S: BuildHasher> FrozenStringMap<S> {
    pub(in crate::native_artifact) fn schema(&self) -> SchemaId {
        self.schema
    }
    pub(in crate::native_artifact) fn len(&self) -> usize {
        self.owner.len()
    }
    pub(in crate::native_artifact) fn key_bytes(&self) -> u64 {
        self.owner.key_bytes()
    }
    pub(in crate::native_artifact) fn logical_bytes(&self) -> u64 {
        self.logical_bytes
    }
    pub(in crate::native_artifact) fn retained_bytes(&self) -> Result<u64, Error> {
        add(self.owner.retained_bytes()?, self.metadata.capacity_bytes())
    }
    pub(in crate::native_artifact) fn retained_nodes(&self) -> u64 {
        self.owner.nodes.nodes() + self.nodes.nodes()
    }
    pub(in crate::native_artifact) fn check_account(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<(), Error> {
        if !self.owner.work.shares_account(work) {
            return Err(Error::Binding("foreign frozen dictionary account"));
        }
        Ok(())
    }
    pub(in crate::native_artifact) fn entries(
        &self,
    ) -> Result<impl ExactSizeIterator<Item = (&str, u64)>, Error> {
        self.owner.entries()
    }

    /// Consume the immutable table once. Intervening calls stay charged to the
    /// invocation but do not consume this source operation's relative cap.
    /// Freeze's spent prefix is never refunded or granted again.
    pub(in crate::native_artifact) fn materialize(
        self,
        geometry: Geometry,
    ) -> Result<Value, Error> {
        let Self {
            owner,
            schemas,
            schema,
            original_limits,
            spent,
            logical_bytes: _,
            metadata,
            nodes,
        } = self;
        let released_bytes = metadata.capacity_bytes();
        let released_nodes = nodes.nodes();
        let mut remaining = original_limits;
        remaining.max_compute_work = Some(
            cap(remaining.max_compute_work)?
                .checked_sub(spent.0)
                .ok_or(Error::Overflow)?,
        );
        remaining.max_comparison_work = Some(
            cap(remaining.max_comparison_work)?
                .checked_sub(spent.1)
                .ok_or(Error::Overflow)?,
        );
        let geometry = Geometry {
            live_bytes: geometry
                .live_bytes
                .checked_sub(released_bytes)
                .ok_or(Error::Overflow)?,
            live_nodes: geometry
                .live_nodes
                .checked_sub(released_nodes)
                .ok_or(Error::Overflow)?,
        };
        // The view's extra representation claims retire before canonical
        // scratch/output. The complete real table and its key claims stay live.
        drop((metadata, nodes));
        owner.publish_result(&schemas, schema, &[], geometry, remaining)
    }
}

#[cfg(test)]
mod tests;
