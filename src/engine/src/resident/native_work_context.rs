//! A synchronous borrow of the current ledger cells, never saved authority.
use super::*;

/// The current thread's work context, borrowed only for an enclosing call.
/// Every debit still reads the live account, relative limits and worker ledger.
#[derive(Clone, Copy)]
pub struct NativeLibraryWorkContext<'scope>(&'scope ControlWorkContext);

/// Borrow the existing context without holding any RefCell borrow over `call`.
/// The higher-ranked callback prevents the reference from escaping its scope.
pub fn with_native_work_context<T>(
    call: impl for<'scope> FnOnce(NativeLibraryWorkContext<'scope>) -> T,
) -> T {
    CONTROL_WORK_CONTEXT.with(|context| call(NativeLibraryWorkContext(context)))
}

impl NativeLibraryWorkContext<'_> {
    /// Query the current cells; a nested call may have changed this since entry.
    pub fn has_active_scope(self) -> bool {
        self.0.active.borrow().is_some()
    }
}

impl NativeLibraryWorkBudget {
    /// Same ordinary debit using an already borrowed context address.
    /// Caller ceiling/overflow still precedes current-account rejection.
    pub fn charge_compute_with_limit_scoped(
        &self,
        context: NativeLibraryWorkContext<'_>,
        compute: u64,
        limit: u64,
    ) -> Result<(), ResidentKernelError> {
        self.check_compute_ceiling(compute, limit)?;
        self.charge_in_context(context.0, compute, 0)
    }
}

#[cfg(test)]
#[path = "native_work_context_tests.rs"]
mod tests;
