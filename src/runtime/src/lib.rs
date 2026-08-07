#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]
#![forbid(unsafe_code)]

pub mod config;
pub mod effect;
mod extension;
pub mod id;
#[cfg(feature = "runtime")]
pub mod input;
pub mod operation;
mod resource;
#[cfg(feature = "runtime")]
mod snapshot;

#[cfg(feature = "runtime")]
pub mod actor;
#[cfg(feature = "runtime")]
pub mod actor_behavior;
#[cfg(feature = "runtime")]
pub mod capability;
#[cfg(feature = "runtime")]
pub mod context;
#[cfg(feature = "runtime")]
mod context_events;
#[cfg(feature = "runtime")]
pub mod event;
pub mod host;
#[cfg(feature = "runtime")]
pub mod module;
#[cfg(feature = "runtime")]
pub mod resolver;
#[cfg(feature = "runtime")]
pub mod runtime;
#[cfg(feature = "runtime")]
pub mod scheduler;
#[cfg(feature = "runtime")]
pub mod service;
#[cfg(feature = "runtime")]
pub mod store;
#[cfg(feature = "runtime")]
pub mod transaction;
#[cfg(all(feature = "watcher", feature = "source"))]
mod workspace;

pub use self::config::*;
pub use self::effect::*;
pub use self::extension::{RuntimeExtensionPanicked, RuntimeStoreCommitIndeterminate};
pub use self::id::*;
#[cfg(feature = "runtime")]
pub use self::input::*;
pub use self::operation::*;
pub use self::resource::*;
#[cfg(feature = "runtime")]
pub use self::snapshot::*;

#[cfg(feature = "runtime")]
pub use self::actor::*;
#[cfg(feature = "runtime")]
pub use self::actor_behavior::*;
#[cfg(feature = "runtime")]
pub use self::capability::*;
#[cfg(feature = "runtime")]
pub use self::context::*;
#[cfg(feature = "runtime")]
pub use self::event::*;
pub use self::host::*;
#[cfg(feature = "runtime")]
pub use self::module::*;
#[cfg(feature = "runtime")]
pub use self::resolver::*;
#[cfg(feature = "runtime")]
pub use self::runtime::*;
#[cfg(feature = "runtime")]
pub use self::scheduler::*;
#[cfg(feature = "runtime")]
pub use self::service::*;
#[cfg(feature = "runtime")]
pub use self::store::*;
#[cfg(feature = "runtime")]
pub use self::transaction::*;
#[cfg(all(feature = "watcher", feature = "source"))]
pub use self::workspace::*;

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    use std::sync::Arc;

    use mech_core::MResult;

    use crate::{
        ActorMessageKindHostFunction, ActorMessagePayloadHostFunction, ActorStateGetHostFunction,
        ActorStateIdHostFunction, ActorStatePutHostFunction, RegisteredHostFunction,
        RuntimeBuilder,
    };

    pub fn install_actor_message_kind(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::Pure(Arc::new(
            ActorMessageKindHostFunction::new(),
        )))
    }

    pub fn install_actor_message_payload(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::Pure(Arc::new(
            ActorMessagePayloadHostFunction::new(),
        )))
    }

    pub fn install_actor_state_id(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::Pure(Arc::new(
            ActorStateIdHostFunction::new(),
        )))
    }

    pub fn install_actor_state_get(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::RuntimeManaged(Arc::new(
            ActorStateGetHostFunction::new(),
        )))
    }

    pub fn install_actor_state_put(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::RuntimeManaged(Arc::new(
            ActorStatePutHostFunction::new(),
        )))
    }
}
