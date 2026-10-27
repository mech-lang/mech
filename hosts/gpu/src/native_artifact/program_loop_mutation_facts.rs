//! Test controls and actual-emission observations for the source-derived
//! single-insertion/index relation. The range proof owns the CFG certificate;
//! the emitter additionally checks the initial runtime tally/index equality.
#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}
pub(super) fn enabled() -> bool {
    #[cfg(test)]
    {
        !DISABLED.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        true
    }
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_mutation_facts<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            DISABLED.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(DISABLED.with(|v| v.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_mutation_emissions() -> (usize, usize, usize) {
    EMISSIONS.with(|v| v.replace((0, 0, 0)))
}
pub(super) fn note(headers: usize, counters: usize, probes: usize) {
    #[cfg(test)]
    EMISSIONS.with(|v| {
        let old = v.get();
        v.set((old.0 + headers, old.1 + counters, old.2 + probes));
    });
    #[cfg(not(test))]
    let _ = (headers, counters, probes);
}
