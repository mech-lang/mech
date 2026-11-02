//! Test-only protocol mutations before the actual emitted selected helper.
use crate::native_artifact::program::runtime::collections::string_map_entry::tests::*;
use crate::native_artifact::program::runtime::invocation::{CollectionExecution, CollectionRead};

pub(in crate::native_artifact::program::runtime::collections) fn before_stage(
    state: &mut State,
    ctx: &mut CollectionExecution<'_, '_, '_, '_>,
    site: usize,
) -> Result<bool, Error> {
    let Some(fault) = FAULT.with(Cell::get) else {
        return Ok(false);
    };
    if FIRED.with(Cell::get) {
        return Ok(false);
    }
    let (id, stage) = match ctx.descriptor().metadata().plan.blocks()[site].action() {
        Action::ProjectStringMapEntry { collection } => (*collection, 0),
        Action::CollectionPush { collection, .. }
            if ctx.descriptor().metadata().plan.collections()[*collection as usize]
                .string_element_map_projection()
                .is_some() =>
        {
            (*collection, 1)
        }
        _ => return Ok(false),
    };
    let expected = match fault {
        Fault::KeyReplay
        | Fault::MissingKey
        | Fault::WrongKeyVisit
        | Fault::OldKeyGeneration
        | Fault::ForeignKeyHandle
        | Fault::ForeignSite
        | Fault::EntryReplay => 0,
        _ => 1,
    };
    if stage != expected {
        return Ok(false);
    }
    FIRED.with(|cell| cell.set(true));
    let proof = ctx.descriptor().metadata().plan.collections()[id as usize]
        .string_element_map_projection()
        .ok_or_else(|| invalid("test String map proof"))?;
    match fault {
        Fault::KeyReplay => state.prepare_string_map_key(
            ctx,
            proof.element_site() as usize,
            proof.iteration(),
            0,
        )?,
        Fault::MissingKey => {
            drop(state.string_maps[id as usize].take());
            state.active_string_maps = state
                .active_string_maps
                .checked_sub(1)
                .ok_or_else(|| invalid("test missing key counter setup"))?;
        }
        Fault::WrongKeyVisit => {
            if let Some(Lane::Key { visit, .. }) = state.string_maps[id as usize].as_mut() {
                visit.remaining = visit.remaining.wrapping_add(1);
            }
        }
        Fault::WrongEntryVisit => {
            if let Some(Lane::Entry { visit, .. }) = state.string_maps[id as usize].as_mut() {
                visit.remaining = visit.remaining.wrapping_add(1);
            }
        }
        Fault::OldKeyGeneration => {
            if GENERATED.with(Cell::get) {
                let Some(Lane::Key { owner, .. }) = state.string_maps[id as usize].as_ref() else {
                    return Err(invalid("test generated source lost key pin"));
                };
                let external_distinct =
                    ctx.descriptor()
                        .metadata()
                        .plan
                        .inputs()
                        .iter()
                        .all(|input| {
                            ctx.arena_test_current_value(input.value().get())
                                .is_none_or(|input| !input.shares_frozen_storage(owner))
                        });
                let arena_aliases = (0..ctx.descriptor().metadata().types.len())
                    .filter(|&slot| {
                        ctx.arena_test_current_value(slot as u32)
                            .is_some_and(|value| value.shares_frozen_storage(owner))
                    })
                    .count();
                let source_aliases = state.sources.iter().flatten().filter(|source|
                    matches!(&source.backing, SourceBacking::Value(value) if value.shares_frozen_storage(owner))).count();
                let no_call_alias = ctx
                    .inputs()
                    .iter()
                    .flatten()
                    .all(|value| !value.shares_frozen_storage(owner));
                let witness =
                    external_distinct && arena_aliases == 1 && source_aliases == 1 && no_call_alias;
                LAST_SOURCE_PIN.with(|cell| cell.set(witness));
                if !witness {
                    return Err(invalid("test generated source retains an unrelated alias"));
                }
                // Error cleanup drops the Source backing BEFORE the private
                // lane. After this retirement the lane becomes its last pin.
            }
            ctx.arena_retire_current(proof.source().get())?;
        }
        Fault::OldMapGeneration => ctx.arena_retire_current(proof.map().get())?,
        Fault::ForeignKeyHandle => {
            let foreign = ctx.arena_current_handle(proof.map().get())?;
            if let Some(Lane::Key { handle, .. }) = state.string_maps[id as usize].as_mut() {
                *handle = foreign;
            }
        }
        Fault::ForeignMapHandle => {
            let foreign = ctx.arena_current_handle(proof.source().get())?;
            if let Some(Lane::Entry { map_handle, .. }) = state.string_maps[id as usize].as_mut() {
                *map_handle = foreign;
            }
        }
        Fault::ForeignSite => {
            state.project_string_map_entry(ctx, proof.element_site() as usize, id)?
        }
        Fault::EntryReplay => state.project_string_map_entry(ctx, site, id)?,
        Fault::ConsumedReplay => {
            let yielded = state.take_string_map_yield(ctx, id, proof.output())?;
            state.release_string_yield(yielded)?;
        }
        Fault::LeakedAtLatch => return Ok(true),
        Fault::ObserveAbsent => {
            let key = matches!(
                ctx.arena_copy_current(proof.element().get()),
                Err(Error::Value(NativeValueError::StaleHandle))
            );
            let output = matches!(
                ctx.arena_copy_current(proof.output().get()),
                Err(Error::Value(NativeValueError::StaleHandle))
            );
            ABSENT.with(|cell| cell.set(key && output));
        }
    }
    Ok(false)
}
