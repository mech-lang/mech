//! Test controls for the optional source-derived count-value induction. The
//! bounded source walk owns its proof; emission also requires the existing
//! empty packed owner and guarded source-index/update-tally relation.
#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(super) fn enabled() -> bool {
    #[cfg(test)] { !DISABLED.with(std::cell::Cell::get) }
    #[cfg(not(test))] { true }
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_count_bounds<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore { fn drop(&mut self) { DISABLED.with(|v| v.set(self.0)); } }
    let _restore = Restore(DISABLED.with(|v| v.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_count_bound_emissions() -> usize {
    EMISSIONS.with(|v| v.replace(0))
}
pub(super) fn note(count: usize) {
    #[cfg(test)]
    EMISSIONS.with(|v| v.set(v.get() + count));
    #[cfg(not(test))]
    let _ = count;
}
