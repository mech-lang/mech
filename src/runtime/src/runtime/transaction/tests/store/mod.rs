use crate::MechRuntime;
use crate::runtime::test_support::events::event_count;

mod abort;
mod begin;
mod commit;
mod context_identity;
mod event_publication;
mod indeterminate;
mod store_failure;

fn new_runtime() -> MechRuntime {
    MechRuntime::builder().build().unwrap()
}
