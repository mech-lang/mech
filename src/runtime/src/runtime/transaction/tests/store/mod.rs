use crate::MechRuntime;
use crate::runtime::test_support::events::event_count;

#[cfg(feature = "source")]
mod abort;
mod begin;
mod commit;
mod context_identity;
mod event_publication;
#[cfg(feature = "source")]
mod indeterminate;
mod projection;
mod store_failure;

fn new_runtime() -> MechRuntime {
    MechRuntime::builder().build().unwrap()
}
