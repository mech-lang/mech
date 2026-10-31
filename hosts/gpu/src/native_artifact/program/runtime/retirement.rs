//! Original ordered lexical cleanup, shared by ordinary tail boundaries.
use super::*;
use crate::native_artifact::program::prepare::frame_retirement::FrameRetirements;

pub(super) fn run(
    slots: &[u64],
    recipes: &FrameRetirements,
    frames: &mut buffer_frames::BufferFrames,
    arena: &mut NativeValueArena<'_>,
    work: &mut NativeWorkMeter,
) -> Result<bool, NativeProgramExecutionError> {
    work.charge(NativeExecutionPhase::Setup, amount(slots.len(), 16, 64)?)?;
    // The original scan price is paid first. Preview only current validity and
    // immutable recipe costs; an unsupported or denied group stays ordered.
    if let Some((total, charges)) = preview(slots, recipes, frames, arena, work)
        && charges > 1
        && let Some(mut paid) = work.try_retirement_batch(total)
    {
        for &slot in slots {
            if frames.retire_prepaid(recipes, slot as u32, &mut paid)? {
                // The same batch preview checked this selected container's
                // canonical slot is empty. Only its frame can hold storage;
                // retain the original arena price on the existing paid loan.
                paid.pay(32)?;
            } else {
                arena.retire_current_prepaid(slot as u32, &mut paid)?;
            }
        }
        paid.finish()?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        BATCHES.with(|count| count.set(count.get() + 1));
        return Ok(true);
    }
    for &slot in slots {
        frames.retire(recipes, slot as u32, work)?;
        arena.retire_current(slot as u32, work)?;
    }
    Ok(false)
}

fn preview(
    slots: &[u64],
    recipes: &FrameRetirements,
    frames: &buffer_frames::BufferFrames,
    arena: &NativeValueArena<'_>,
    work: &NativeWorkMeter,
) -> Option<(u64, usize)> {
    let mut total = 0u64;
    let mut charges = slots.len();
    for &slot in slots {
        let slot = u32::try_from(slot).ok()?;
        if let Some((charge, _)) = recipes.get(slot as usize) {
            // This recipe is derived only for selected frame containers, not
            // their projected canonical fields. An unexpected canonical
            // definition conservatively keeps the entire original ordered path.
            if !frames.has_retirement_backing()
                || arena.native_entry_preview(slot, work).ok()?.is_some()
            {
                return None;
            }
            total = total.checked_add(charge)?;
            charges = charges.checked_add(1)?;
        } else if !arena.native_retirement_available(slot, work).ok()? {
            return None;
        }
        total = total.checked_add(32)?;
    }
    Some((total, charges))
}

#[cfg(all(test, feature = "canonical-native-program"))]
thread_local! {
    static BATCHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static CHILD_EFFECTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) fn take_observation() -> u64 {
    BATCHES.with(|count| count.replace(0))
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) fn take_child_observation() -> u64 {
    CHILD_EFFECTS.with(|count| count.replace(0))
}
