//! Immutable canonical output at an owner boundary. No source traversal,
//! accumulation or execution authority is implemented by this storage bridge.
use super::*;
use mech_core::{
    CardinalitySpec, DimensionExpr, IntegerWidth, MemoryBudgetLimits, SchemaBody, SchemaId,
    SchemaTable, Value, ValueData,
    snapshot::{
        MapEntryDraft, SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueDataDraft,
        ValueDraft,
    },
};
use std::sync::Arc;

/// Complete simultaneous caller geometry, INCLUDING this retained table and
/// any prior output. A future native caller must derive this from its live
/// frame/owners; these descriptive numbers confer no publication authority.
#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct Geometry {
    pub live_bytes: u64,
    pub live_nodes: u64,
}
struct Plan {
    workspace: u64,
    finalization: u64,
    compute: u64,
    staging_nodes: u64,
}
fn exact_vec<T>(count: usize) -> Result<Vec<T>, Error> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    if values.capacity() != count {
        return Err(Error::Allocation);
    }
    Ok(values)
}
fn required(limit: Option<u64>) -> Result<u64, Error> {
    limit.ok_or(Error::Publication("unbounded call dimension"))
}
impl<S: BuildHasher> OwnedStringMap<S> {
    fn publication_plan(
        &self,
        schemas: &Arc<SchemaTable>,
        schema: SchemaId,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<Plan, Error> {
        for limit in [
            limits.max_output_elements,
            limits.max_output_bytes,
            limits.max_temporary_bytes,
            limits.max_cloned_bytes,
            limits.max_retained_nodes,
            limits.max_compute_work,
            limits.max_comparison_work,
        ] {
            required(limit)?;
        }
        let retained = self.retained_bytes()?;
        let count = self.len() as u64;
        if geometry.live_bytes < retained || geometry.live_nodes < add(count, 1)? {
            return Err(Error::Publication("caller geometry omits retained owner"));
        }
        self.work.charge(128, 0)?;
        let target = schemas
            .get(schema)
            .ok_or(Error::Publication("missing schema"))?;
        if !target.dimension_parameters().is_empty() {
            return Err(Error::Publication("parameterized output schema"));
        }
        let SchemaBody::Map {
            key,
            value,
            cardinality,
        } = target.body()
        else {
            return Err(Error::Publication("output is not a map"));
        };
        if key.as_ref() != &SchemaBody::String
            || value.as_ref() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
        {
            return Err(Error::Publication("output key/value kind"));
        }
        match cardinality {
            CardinalitySpec::Exact(DimensionExpr::Constant(n)) if *n == count => {}
            CardinalitySpec::Dynamic { upper_bound: None } => {}
            CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Constant(n)),
            } if count <= *n => {}
            _ => return Err(Error::Publication("output cardinality")),
        }
        // Sharing the schema arena still imports its complete ownership.
        let schema_bytes = schemas
            .clone_allocation_bound_bytes_with_precharge(|bytes| self.work.charge(bytes, 0))
            .map_err(|error| match error {
                mech_core::SchemaAllocationBoundError::Precharge(error) => Error::Work(error),
                mech_core::SchemaAllocationBoundError::ArithmeticOverflow => Error::Overflow,
            })?;
        let payload = self.keys.capacity_bytes();
        // Exact narrow canonical layout, matching Value::retained_footprint:
        // Value + root ValueData + MapEntryValue + two child ValueDatas/key.
        let published = add(
            add(size_of::<Value>() as u64, size_of::<ValueData>() as u64)?,
            add(
                mul(
                    count,
                    (size_of::<mech_core::snapshot::MapEntryValue>() + 2 * size_of::<ValueData>())
                        as u64,
                )?,
                payload,
            )?,
        )?;
        let nodes = add(mul(count, 2)?, 2)?;
        let drafts = mul(
            count,
            (size_of::<MapEntryDraft>() + 2 * size_of::<ValueDataDraft>() + size_of::<String>())
                as u64,
        )?;
        let order = mul(count, size_of::<(&str, u64)>() as u64)?;
        let owner = mul(
            add(
                add(schema_bytes, Value::canonical_owner_allocation_bytes())?,
                Value::memory_budget_claim_metadata_bytes(),
            )?,
            4,
        )?;
        let workspace = add(
            add(owner, 4096)?,
            add(
                order,
                add(mul(drafts, 4)?, add(mul(published, 4)?, mul(payload, 4)?)?)?,
            )?,
        )?;
        check(1, required(limits.max_output_elements)?, "output elements")?;
        check(
            published,
            required(limits.max_output_bytes)?,
            "output bytes",
        )?;
        check(
            published,
            required(limits.max_cloned_bytes)?,
            "cloned bytes",
        )?;
        check(
            add(geometry.live_nodes, mul(nodes, 3)?)?,
            required(limits.max_retained_nodes)?,
            "retained nodes",
        )?;
        check(
            add(geometry.live_bytes, workspace)?,
            required(limits.max_temporary_bytes)?,
            "temporary bytes",
        )?;
        // Sorted drafts take the core finalizer's ordered append route. Each
        // adjacent String comparison is covered by three total payload scans.
        let finalization = add(add(mul(payload, 3)?, mul(count, 8)?)?, 8)?;
        let compute = add(mul(workspace, 16)?, 256)?;
        Ok(Plan {
            workspace,
            finalization,
            compute,
            staging_nodes: mul(nodes, 2)?,
        })
    }

    /// Copy one complete current table into an independently owned immutable
    /// map. The table stays intact on success and every denial/failure. The
    /// caller must still prove the exact native site, owner and live geometry.
    pub(in crate::native_artifact) fn publish(
        &self,
        schemas: &Arc<SchemaTable>,
        schema: SchemaId,
        geometry: super::PublicationGeometry,
        limits: MemoryBudgetLimits,
    ) -> Result<Value, Error> {
        let compute = required(limits.max_compute_work)?;
        let comparison = required(limits.max_comparison_work)?;
        self.work.with_call_limits(compute, comparison, || {
            let plan = self.publication_plan(schemas, schema, geometry, limits)?;
            self.work.charge(plan.compute, plan.finalization)?;
            let memory = self.metadata.budget().with_limits(required(limits.max_temporary_bytes)?,
                required(limits.max_retained_nodes)?);
            // The retained map, construction scratch and admitted canonical
            // output are distinct live owners. Scratch covers the unadmitted
            // draft/finalizer; import atomically adds the canonical witness.
            let _staging = memory.reserve_nodes(plan.staging_nodes)?;
            let mut reservation = memory.reserve_capacity(plan.workspace)?;
            let mut order = exact_vec(self.len())?;
            order.extend(self.entries()?);
            sort(&mut order, &self.work)?;
            let mut entries = exact_vec(order.len())?;
            for (key, value) in order {
                let mut text = String::new();
                text.try_reserve_exact(key.len())
                    .map_err(|_| Error::Allocation)?;
                if text.capacity() != key.len() {
                    return Err(Error::Allocation);
                }
                text.push_str(key);
                let mut items = exact_vec(2)?;
                items.push(ValueDataDraft::String(text));
                items.push(ValueDataDraft::U64(value));
                entries.push(MapEntryDraft {
                    items: items.into_boxed_slice(),
                });
            }
            let canonical = SnapshotCanonicalizationBudget::new(plan.finalization);
            let context = SnapshotValidationContext::with_shared_schemas(schemas)
                .with_canonicalization_budget(&canonical);
            let value = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Map(entries.into_boxed_slice()),
            }
            .finalize(&context)
            .map_err(|_| Error::Publication("canonical finalization"))?;
            value
                .into_memory_budget(&mut reservation)
                .map_err(Error::Memory)
        })?
    }
}

// Fallible, allocation-free heapsort of BORROWED entries. Every comparison is
// prepaid before inspecting key bytes. A denial only disturbs this workspace,
// never the retained table; no panic or fresh comparison allowance is used.
fn sort(values: &mut [(&str, u64)], work: &NativeLibraryWorkBudget) -> Result<(), Error> {
    // This leaf only compares borrowed bytes and swaps borrowed entries. No
    // allocation, host callback or budget observer runs while a batch is held.
    work.with_debit_batch(|batch| sort_with_debits(values, &mut |compute, comparison|
        batch.charge(compute, comparison)))?
}

fn sort_with_debits(
    values: &mut [(&str, u64)],
    charge: &mut impl FnMut(u64, u64) -> Result<(), ResidentKernelError>,
) -> Result<(), Error> {
    fn less(a: &str, b: &str,
        charge: &mut impl FnMut(u64, u64) -> Result<(), ResidentKernelError>,
    ) -> Result<bool, Error> {
        let bytes = add(add(a.len() as u64, b.len() as u64)?, 1)?;
        charge(add(mul(bytes, 8)?, 32)?, bytes)?;
        Ok(a.as_bytes() < b.as_bytes())
    }
    fn sift(
        values: &mut [(&str, u64)],
        mut root: usize,
        charge: &mut impl FnMut(u64, u64) -> Result<(), ResidentKernelError>,
    ) -> Result<(), Error> {
        loop {
            charge(32, 0)?;
            let left = root
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or(Error::Overflow)?;
            if left >= values.len() {
                return Ok(());
            }
            let mut child = left;
            if left + 1 < values.len() && less(values[left].0, values[left + 1].0, charge)? {
                child += 1;
            }
            if !less(values[root].0, values[child].0, charge)? {
                return Ok(());
            }
            values.swap(root, child);
            root = child;
        }
    }
    for root in (0..values.len() / 2).rev() {
        sift(values, root, charge)?;
    }
    for end in (1..values.len()).rev() {
        charge(32, 0)?;
        values.swap(0, end);
        sift(&mut values[..end], 0, charge)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "owned_string_map_publication_tests.rs"]
mod tests;
