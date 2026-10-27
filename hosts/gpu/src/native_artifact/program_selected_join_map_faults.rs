//! Test-only mutations at actual native boundaries; production guards decide.
use super::*;
use std::cell::Cell;
#[derive(Clone, Copy, Debug)]
pub(in crate::native_artifact::program_runtime) enum Fault {
    ReplayPrepare,
    FinishWithoutLoan,
    WrongSite,
    ExcessCompletion,
    SkipCompletion,
    WrongMapGeneration,
    WrongCopySource,
    MissingOwner,
    PrivateWrongAccount,
    PrivateShortCapacity,
    PrivateWrongFactory,
    UnpublishedExtent,
    ExcessInitializedExtent,
    CorruptSourceGeometry,
    MovedInitializedBacking,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::native_artifact::program_runtime) enum Limit {
    Output,
    Cloned,
    Temporary,
    Nodes,
    Compute,
}
#[derive(Clone, Copy, Default)]
pub(in crate::native_artifact::program_runtime) struct Control {
    pub disable: bool,
    pub fault: Option<Fault>,
    pub limit: Option<(Limit, u64)>,
    pub global_bytes: Option<u64>,
    pub global_nodes: Option<u64>,
}
thread_local! {
    static CONTROL: Cell<Control> = Cell::new(Control::default());
    static FIRED: Cell<bool> = const { Cell::new(false) };
    static STARTED: Cell<bool> = const { Cell::new(false) };
}
pub(in crate::native_artifact::program_runtime) fn with_control<T>(
    control: Control,
    f: impl FnOnce() -> T,
) -> T {
    struct Restore(Control, bool, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            CONTROL.with(|v| v.set(self.0));
            FIRED.with(|v| v.set(self.1));
            STARTED.with(|v| v.set(self.2));
        }
    }
    let guard = Restore(
        CONTROL.with(|v| v.replace(control)),
        FIRED.with(|v| v.replace(false)),
        STARTED.with(|v| v.replace(false)),
    );
    let result = f();
    drop(guard);
    result
}
pub(in crate::native_artifact::program_runtime) fn fired() -> bool {
    FIRED.with(Cell::get)
}
pub(in crate::native_artifact::program_runtime) fn started() -> bool {
    STARTED.with(Cell::get)
}
pub(super) fn disabled() -> bool {
    CONTROL.with(|v| v.get().disable)
}
pub(in crate::native_artifact::program_runtime) fn demand() -> [u64; 6] {
    let d = super::super::tests::take();
    [
        d.output,
        d.cloned,
        d.temporary,
        d.nodes,
        d.compute,
        d.envelope,
    ]
}
pub(in crate::native_artifact::program_runtime) fn pressure(
    ctx: &Context<'_, '_>,
) -> Result<
    (
        Option<ManagedMemoryReservation>,
        Option<ManagedNodeReservation>,
    ),
    Error,
> {
    let control = CONTROL.with(Cell::get);
    let memory = ctx.library.memory_budget();
    let bytes = if let Some(remaining) = control.global_bytes {
        fire();
        let count = memory
            .limit_bytes()
            .checked_sub(memory.used_bytes())
            .and_then(|n| n.checked_sub(remaining))
            .ok_or_else(|| invalid("test byte pressure exceeds free memory"))?;
        Some(memory.reserve_capacity(count)?)
    } else {
        None
    };
    let nodes = if let Some(remaining) = control.global_nodes {
        fire();
        let count = memory
            .limit_nodes()
            .checked_sub(memory.retained_nodes())
            .and_then(|n| n.checked_sub(remaining))
            .ok_or_else(|| invalid("test node pressure exceeds free nodes"))?;
        Some(memory.reserve_nodes(count)?)
    } else {
        None
    };
    Ok((bytes, nodes))
}
pub(in crate::native_artifact::program_runtime) fn before_join(
    ctx: &mut Context<'_, '_>,
    role: Site,
) {
    if !matches!(role, Site::Join(_)) || FIRED.with(Cell::get) {
        return;
    }
    if let Some((dimension, limit)) = CONTROL.with(|v| v.get().limit) {
        FIRED.with(|v| v.set(true));
        match dimension {
            Limit::Output => ctx.limits.max_output_bytes = Some(limit),
            Limit::Cloned => ctx.limits.max_cloned_bytes = Some(limit),
            Limit::Temporary => ctx.limits.max_temporary_bytes = Some(limit),
            Limit::Nodes => ctx.limits.max_retained_nodes = Some(limit),
            Limit::Compute => ctx.limits.max_compute_work = Some(limit),
        }
    }
}
fn fault() -> Option<Fault> {
    if FIRED.with(Cell::get) {
        None
    } else {
        CONTROL.with(|v| v.get().fault)
    }
}
fn fire() {
    FIRED.with(|v| v.set(true));
}
pub(super) fn before_prepare(ctx: &mut Context<'_, '_>, site: usize) -> Result<(), Error> {
    if !ctx.join_map_is_fusing() {
        return Ok(());
    }
    STARTED.with(|v| v.set(true));
    match fault() {
        Some(Fault::FinishWithoutLoan) => {
            fire();
            ctx.finish_join_map_chunk(site, [0; 3])?;
        }
        Some(Fault::WrongSite) => {
            fire();
            ctx.prepare_join_map_chunk(site + 1, [0; 3])?;
        }
        _ => {}
    }
    Ok(())
}
pub(super) fn after_prepare(ctx: &mut Context<'_, '_>, site: usize) -> Result<(), Error> {
    if matches!(fault(), Some(Fault::ReplayPrepare)) {
        fire();
        ctx.prepare_join_map_chunk(site, [0; 3])?;
    }
    Ok(())
}
pub(super) fn before_finish(ctx: &mut Context<'_, '_>, site: usize) -> Result<bool, Error> {
    match fault() {
        Some(Fault::ExcessCompletion) => {
            fire();
            let Some(State {
                phase: Phase::Fusing(f),
                ..
            }) = ctx.selected_join.as_ref()
            else {
                return Err(invalid("test missing chunk owner"));
            };
            let count = f
                .loan
                .as_ref()
                .ok_or_else(|| invalid("test missing chunk loan"))?
                .count;
            ctx.finish_join_map_chunk(site, [count as u64 + 1, 0, 0])?;
        }
        Some(
            test @ (Fault::UnpublishedExtent
            | Fault::ExcessInitializedExtent
            | Fault::CorruptSourceGeometry),
        ) => {
            fire();
            let Some(State {
                phase: Phase::Fusing(f),
                ..
            }) = ctx.selected_join.as_mut()
            else {
                return Err(invalid("test missing initialized source loan"));
            };
            let loan = f
                .loan
                .as_mut()
                .ok_or_else(|| invalid("test missing initialized extent"))?;
            match test {
                Fault::UnpublishedExtent => {
                    loan.words[6] = if loan.count == 0 {
                        1
                    } else {
                        loan.count as u64 - 1
                    }
                }
                Fault::ExcessInitializedExtent => loan.words[6] = loan.count as u64 + 1,
                Fault::CorruptSourceGeometry => loan.words[0] ^= 8,
                _ => unreachable!(),
            }
        }
        Some(Fault::MovedInitializedBacking) => {
            fire();
            let Some(State {
                phase: Phase::Fusing(f),
                ..
            }) = ctx.selected_join.as_mut()
            else {
                return Err(invalid("test missing completed backing"));
            };
            let mut replacement = Vec::new();
            replacement.try_reserve_exact(f.bytes.capacity()).unwrap();
            replacement.extend_from_slice(&f.bytes);
            // Allocate before dropping the old buffer: a nonempty loan cannot
            // accidentally receive the same allocator address in this probe.
            f.bytes = replacement;
        }
        Some(Fault::SkipCompletion) => {
            fire();
            return Ok(true);
        }
        _ => {}
    }
    Ok(false)
}
pub(super) fn after_mapped(ctx: &mut Context<'_, '_>) -> Result<(), Error> {
    match fault() {
        Some(Fault::WrongMapGeneration) => {
            fire();
            let Some(State {
                phase: Phase::Mapped(mapped),
                region,
                ..
            }) = ctx.selected_join.as_ref()
            else {
                return Err(invalid("test missing mapped owner"));
            };
            let map = mapped.map.clone();
            let slot = ctx.control.descriptor.selected_joins.maps[*region]
                .as_ref()
                .unwrap()
                .map;
            ctx.arena.retire_current(slot, &mut ctx.control.work)?;
            ctx.arena.put(slot, &map, &mut ctx.control.work)?;
        }
        Some(Fault::WrongCopySource) => {
            fire();
            if let Some(State {
                phase: Phase::Mapped(mapped),
                ..
            }) = ctx.selected_join.as_mut()
            {
                mapped.slot = u32::MAX;
            }
        }
        Some(Fault::MissingOwner) => {
            fire();
            ctx.selected_join = None;
        }
        Some(
            test @ (Fault::PrivateWrongAccount
            | Fault::PrivateShortCapacity
            | Fault::PrivateWrongFactory),
        ) => {
            fire();
            let Some(State {
                phase: Phase::Mapped(mapped),
                region,
                ..
            }) = ctx.selected_join.as_ref()
            else {
                return Err(invalid("test missing private String buffer"));
            };
            let mut text = String::new();
            text.try_reserve_exact(mapped.owner.as_bytes().len())
                .unwrap();
            text.push_str(core::str::from_utf8(mapped.owner.as_bytes()).unwrap());
            let memory = if matches!(test, Fault::PrivateWrongAccount) {
                mech_core::ManagedMemoryBudget::new(1024 * 1024)
            } else {
                ctx.library.memory_budget()
            };
            let capacity =
                text.capacity() as u64 - u64::from(matches!(test, Fault::PrivateShortCapacity));
            let claim = memory.reserve_capacity(capacity)?;
            let join = &ctx.control.descriptor.selected_joins.regions[*region];
            let site = if matches!(test, Fault::PrivateWrongFactory) {
                ctx.control.descriptor.string_map_regions[0].selection_site() as usize
            } else {
                join.consumer() as usize
            };
            let call = ctx.control.descriptor.calls[site].as_ref().unwrap();
            call.finish_private_string_buffer(text, claim, &ctx.library)
                .map_err(|error| library(ctx, site, error))?;
            return Err(invalid(
                "private String owner accepted an invalid factory/account/capacity",
            ));
        }
        _ => {}
    }
    Ok(())
}
