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
            frames.retire_prepaid(recipes, slot as u32, &mut paid)?;
            arena.retire_current_prepaid(slot as u32, &mut paid)?;
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
        if !arena.native_retirement_available(slot, work).ok()? {
            return None;
        }
        total = total.checked_add(32)?;
        if let Some((charge, _)) = recipes.get(slot as usize) {
            if !frames.has_retirement_backing() {
                return None;
            }
            total = total.checked_add(charge)?;
            charges = charges.checked_add(1)?;
        }
    }
    Some((total, charges))
}

#[cfg(all(test, feature = "canonical-native-program"))]
thread_local! {
    static BATCHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) fn take_observation() -> u64 {
    BATCHES.with(|count| count.replace(0))
}
