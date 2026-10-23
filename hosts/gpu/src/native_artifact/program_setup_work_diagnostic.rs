//! Test-only fixed observation at the existing callback error boundary.
//! No replacement entry, callback, execution authority, price or policy.
use super::{Context, NativeProgramExecutionError};

#[derive(Clone, Copy, Debug)]
pub(super) enum Callback {
    Admit,
    Helper,
}

#[cfg(feature = "canonical-native-program-host")]
#[path = "program_hosted_work_diagnostic.rs"]
mod hosted;
#[cfg(feature = "canonical-native-program-host")]
pub(super) use hosted::{begin_callback, checkpoint, end_callback, record};

// The generic test hook compiles in runtime-bridge profiles without pulling
// terminal/compiler fixture dependencies into those narrower feature sets.
#[cfg(not(feature = "canonical-native-program-host"))]
pub(super) fn record(_: &Context<'_, '_>, _: u64, _: Callback, _: &NativeProgramExecutionError) {}
#[cfg(not(feature = "canonical-native-program-host"))]
pub(super) fn begin_callback(_: &Context<'_, '_>, _: u64, _: Callback) -> Option<u64> {
    None
}
#[cfg(not(feature = "canonical-native-program-host"))]
pub(super) fn end_callback(
    _: &Context<'_, '_>,
    _: u64,
    _: Callback,
    _: Option<u64>,
    _: Option<&NativeProgramExecutionError>,
) {
}
#[cfg(not(feature = "canonical-native-program-host"))]
pub(super) fn checkpoint(_: u64) {}
