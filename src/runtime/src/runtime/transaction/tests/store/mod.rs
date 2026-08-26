use crate::MechRuntime;
use crate::runtime::test_support::events::event_count;

#[cfg(feature = "source")]
mod abort;
mod begin;
#[cfg(feature = "source")]
mod commit;
mod context_identity;
mod event_publication;
#[cfg(feature = "source")]
mod indeterminate;
#[cfg(feature = "resident-external")]
mod projection;
mod store_failure;

fn new_runtime() -> MechRuntime {
    MechRuntime::builder().build().unwrap()
}
