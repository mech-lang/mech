//! Opt-in test counters emitted as native atomic instructions, never a callback.
use super::*;
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};
thread_local! {
    static ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static ENTRIES: AtomicU64 = AtomicU64::new(0);
static SERIAL: Mutex<()> = Mutex::new(());
pub(in crate::native_artifact) struct Observation {
    _guard: MutexGuard<'static, ()>,
}
impl Drop for Observation {
    fn drop(&mut self) {
        ENABLED.with(|enabled| enabled.set(false));
    }
}
pub(in crate::native_artifact) fn begin() -> Observation {
    let guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    ENABLED.with(|enabled| assert!(!enabled.replace(true)));
    ENTRIES.store(0, Ordering::SeqCst);
    Observation { _guard: guard }
}
pub(super) fn emit(b: &mut FunctionBuilder<'_>) {
    if !ENABLED.with(|enabled| enabled.get()) {
        return;
    }
    let address = b
        .ins()
        .iconst(types::I64, &ENTRIES as *const AtomicU64 as usize as i64);
    let one = b.ins().iconst(types::I64, 1);
    b.ins().atomic_rmw(
        types::I64,
        cranelift_codegen::ir::MemFlags::trusted(),
        cranelift_codegen::ir::AtomicRmwOp::Add,
        address,
        one,
    );
}
pub(in crate::native_artifact) fn take() -> usize {
    ENTRIES.swap(0, Ordering::SeqCst) as usize
}
