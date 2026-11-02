//! Test-only header observations. This never executes a body/leaf or charges,
//! allocates, clones, scans payloads, or substitutes for the production helper.
use crate::native_artifact::program::runtime::collections::text_join::*;
use crate::native_artifact::program::runtime::invocation::CollectionRead;
use mech_engine::native_artifact::NativeIterationAction;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum TextJoinStage {
    Growth,
    Finish,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct TextJoinDemand {
    pub stage: TextJoinStage,
    pub origin: mech_engine::native_artifact::NativeMatchOrigin,
    pub account_origin: mech_engine::native_artifact::NativeMatchOrigin,
    pub collection: u32,
    pub temporary_required: u64,
    pub reservation_bytes: u64,
    pub compute_required: u64,
    pub compute_debit: u64,
    pub cloned_required: u64,
    pub output_required: u64,
    pub nodes_required: u64,
    pub accepted_before: u64,
    pub payload_before: u64,
    pub capacity_before: u64,
    pub separator_bytes: u64,
    pub current_yield_bytes: u64,
    pub untracked_yield_bytes: u64,
    pub old_backing_bytes: u64,
    pub live_bytes: u64,
}
impl State {
    pub(in crate::native_artifact) fn test_text_join_storage(
        &self,
        collection: u32,
    ) -> Option<(u64, u64, u64)> {
        let join = self
            .builders
            .get(collection as usize)?
            .as_ref()?
            .join
            .as_ref()?;
        Some((
            join.accepted,
            join.text.len() as u64,
            join.text.capacity() as u64,
        ))
    }
    pub(in crate::native_artifact) fn test_text_join_demand(
        &self,
        ctx: &impl CollectionRead,
        site: usize,
    ) -> Result<Option<TextJoinDemand>, Error> {
        let Some(block) = ctx.descriptor().metadata().plan.blocks().get(site) else {
            return Ok(None);
        };
        let (id, value, stage) = match block.action() {
            NativeIterationAction::CollectionPush { collection, value } => {
                (*collection, Some(*value), TextJoinStage::Growth)
            }
            NativeIterationAction::TextJoinFinish { collection } => {
                (*collection, None, TextJoinStage::Finish)
            }
            _ => return Ok(None),
        };
        let Some(definition) = ctx
            .descriptor()
            .metadata()
            .plan
            .collections()
            .get(id as usize)
        else {
            return Ok(None);
        };
        let NativeCollectionResult::TextJoin(sink) = definition.result() else {
            return Ok(None);
        };
        let Some(builder) = self.builders.get(id as usize).and_then(Option::as_ref) else {
            return Ok(None);
        };
        let Some(join) = builder.join.as_ref() else {
            return Ok(None);
        };
        let payload = join.text.len() as u64;
        let old = builder.reservation.capacity_bytes();
        let mut demand = TextJoinDemand {
            stage,
            origin: sink.template().origin(),
            account_origin: definition.origin(),
            collection: id,
            temporary_required: 0,
            reservation_bytes: 0,
            compute_required: 0,
            compute_debit: 0,
            cloned_required: 0,
            output_required: 0,
            nodes_required: 7,
            accepted_before: join.accepted,
            payload_before: payload,
            capacity_before: join.text.capacity() as u64,
            separator_bytes: join.separator_bytes,
            current_yield_bytes: 0,
            untracked_yield_bytes: 0,
            old_backing_bytes: old,
            live_bytes: self.live_bytes,
        };
        match stage {
            TextJoinStage::Growth => {
                let value = ctx
                    .arena_test_current_value(value.unwrap().get())
                    .ok_or_else(|| invalid("test observation lacks current selected yield"))?;
                let ValueData::String(part) = value.data() else {
                    return Err(invalid("test observed yield is not String"));
                };
                let ValueData::String(separator) = join.separator.data() else {
                    return Err(invalid("test observed separator is not String"));
                };
                let append = add(
                    part.len() as u64,
                    if join.accepted != 0 {
                        separator.len() as u64
                    } else {
                        0
                    },
                )?;
                let length = add(payload, append)?;
                let capacity = if length > demand.capacity_before {
                    mul(demand.capacity_before, 2)?.max(64).max(length)
                } else {
                    demand.capacity_before
                };
                if capacity <= old {
                    return Ok(None);
                }
                let retained = value
                    .memory_budget_retained_bytes(&ctx.library().memory_budget())
                    .ok_or_else(|| invalid("test observed unadmitted yield"))?;
                demand.current_yield_bytes = retained;
                let tracked = self
                    .sources
                    .iter()
                    .flatten()
                    .filter_map(|source| source.element)
                    .any(|(handle, _)| {
                        ctx.arena_test_handle_value(handle)
                            .is_some_and(|current| current.shares_frozen_storage(value))
                    });
                let untracked = if tracked { 0 } else { retained };
                demand.untracked_yield_bytes = untracked;
                let alias_work = add(mul(self.sources.len() as u64, 64)?, 32)?;
                let compute = add(
                    add(
                        add(mul(append, 4)?, mul(payload, 4)?)?,
                        add(mul(capacity, 4)?, 128)?,
                    )?,
                    128,
                )?;
                demand.reservation_bytes = capacity;
                demand.temporary_required = add(add(self.live_bytes, capacity)?, untracked)?;
                demand.compute_debit = add(alias_work, compute)?;
                demand.compute_required = add(builder.compute, demand.compute_debit)?;
                demand.cloned_required = add(builder.cloned_bytes, add(append, payload)?)?;
                demand.output_required = prospective_string_value_footprint(0, false, 1, length)
                    .map_err(|_| invalid("test append footprint overflow"))?
                    .retained_bytes;
            }
            TextJoinStage::Finish => {
                if !join.parked {
                    return Ok(None);
                }
                let envelope = add(
                    ctx.descriptor().metadata().scalar_bytes,
                    add(mul(payload, 4)?, 4096)?,
                )?;
                let finalization = add(mul(payload, 8)?, 32)?;
                let compute = add(add(mul(envelope, 16)?, finalization)?, 256)?;
                demand.account_origin = demand.origin;
                demand.reservation_bytes = envelope;
                demand.temporary_required = add(self.live_bytes, envelope)?;
                demand.compute_debit = compute;
                demand.compute_required = add(builder.compute, compute)?;
                demand.cloned_required = add(builder.cloned_bytes, payload)?;
                demand.output_required = prospective_string_value_footprint(0, false, 1, payload)
                    .map_err(|_| invalid("test finish footprint overflow"))?
                    .retained_bytes;
            }
        }
        Ok(Some(demand))
    }
}
