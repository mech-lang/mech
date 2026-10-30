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
    for &slot in slots {
        frames.retire(recipes, slot as u32, work)?;
        arena.retire_current(slot as u32, work)?;
    }
    Ok(false)
}

#[cfg(all(test, feature = "canonical-native-program"))]
thread_local! {
    static BATCHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) fn take_observation() -> u64 {
    BATCHES.with(|count| count.replace(0))
}
