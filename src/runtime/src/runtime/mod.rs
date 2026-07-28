//! Runtime shell for Mech.
//!
//! `MechRuntime` is the host-facing runtime object. It wraps the current
//! program/interpreter layer and owns the system-level components:
//!
//! - ID generator
//! - store
//! - capability kernel
//! - source resolver
//! - host registry
//! - host call policy
//! - scheduler
//! - runtime config
//!
//! RuntimeContext is used as the per-operation execution envelope. It carries
//! subject/task/actor/module/transaction identity, resource budget, capabilities,
//! and accumulated events.

mod actor;
mod builder;
mod components;
mod errors;
mod events;
mod execution;
mod execution_session;
pub(crate) mod extension;
mod host;
mod id;
mod live_state;
mod module;
mod object;
mod resources;
mod runtime_context;
mod schedule;
mod state;
mod task;
mod transaction;

#[cfg(test)]
mod input_tests;

#[cfg(test)]
pub(crate) mod test_support;

pub use self::builder::RuntimeBuilder;
pub use self::errors::*;
pub(crate) use self::live_state::{LiveRegistrationMode, RuntimePersistentSendSchedule};
use self::live_state::{
  RuntimeLiveContextTemplate, RuntimeLiveStateSnapshot, RuntimePersistentSend,
};
pub(in crate::runtime) use self::resources::{
  runtime_resource_binding_error, validate_resource_binding_name,
};
pub use self::resources::{RuntimeResourceBinding, RuntimeResourceBindingError};
pub use self::state::MechRuntime;
pub(in crate::runtime) use self::state::{
  validate_module_import_edges, ModuleInstance, ScopedRuntimeState,
};
use self::transaction::ActiveRuntimeProgramOperation;
use self::transaction::RuntimeCommitResolution;
use self::transaction::{RuntimeCapabilityOverlay, RuntimeEffectJournal, RuntimeModuleJournal};
use self::transaction::{
  RuntimeContextCheckpoint, RuntimeExecutionTransaction, RuntimeExecutionTransactionMode,
  RuntimeExecutionTransactionState, RuntimeTransactionContextIdentity,
};
pub use self::transaction::{RuntimeHealth, RuntimePoisonRecord};
use crate::{ActiveRuntimeEffectPhase, RuntimeEffectId};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
#[cfg(all(target_arch = "wasm32", target_os = "unknown",))]
use web_time::Instant;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown",)))]
use std::time::Instant;

#[cfg(test)]
use mech_core::hash_str;
use mech_core::{
  CompileCtx, MResult, MechError, MechErrorKind, MechFunctionCompiler, MechFunctionImpl,
  MechSourceCode, ModuleManifestCatalog, ModuleManifestConfig, NativeFunctionCompiler, Register,
  ValRef, Value,
};

use mech_program::{
  MechProgram, MechProgramCheckpoint, MechProgramConfig, MechProgramEnvironment, ProgramInputId,
};

use crate::capability::{
  BasicCapabilityKernel, Capability, CapabilityGrant, CapabilityKernel, CapabilityRequest,
  CapabilityRevocation,
};

use crate::config::RuntimeConfig;

use crate::context::{
  ResourceBudget, ResourceBudgetExceededError, RuntimeAuthorityScope, RuntimeContext,
  RuntimeContextBinding, RuntimeContextBuilder, RuntimeTurnOutcome,
};

use crate::event::{RuntimeEvent, RuntimeEventKind};

use crate::host::{
  default_host_capability_request, DefaultHostCallPolicy, HostCall, HostCallPolicy,
  HostFunctionNotFoundError, HostRegistry, InMemoryHostRegistry,
};

use crate::id::{
  module_id, ActorId, CapabilityId, DefaultIdGenerator, EventId, IdGenerator, MessageId, ModuleId,
  ModuleVersionId, ObjectId, RuntimeId, TaskId, TransactionId,
};

use crate::resolver::{
  InMemorySourceResolver, ResolvedSource, SourceImportAlias, SourceRequest, SourceResolver,
  SourceScope,
};

use crate::scheduler::{
  collect_tick, InMemoryScheduler, ScheduledWork, Scheduler, SchedulerPolicy, SchedulerTick,
};

use crate::store::{
  ActorRecord, InMemoryStore, MechStore, MessageRecord, ModuleImportEdge, ModuleRecord,
  ModuleVersionRecord, ObjectRecord, RuntimeStoreCommit, TaskRecord, TaskStatus, TransactionRecord,
};

use crate::transaction::{RuntimeTransaction, RuntimeTransactionNotFoundError};

use crate::actor::ActorTurn;

use crate::input::RuntimeHostInputQueueState;

use crate::actor_behavior::{ActorBehaviorDriver, ActorBehaviorRuntime, NoActorBehaviorDriver};

use crate::module::{ModuleBuildOptions, ModuleBuilder, ModuleDependencyGraph};

use crate::{
  materialize_config_spec_grants, register_config_spec_resources, HostInstanceConfig,
  HostInterfaceCatalog, InMemoryDocsProvider, RegisteredHostFunction, ResourcePathCapability,
  RunResourceGrantConfig, RuntimeCapabilityGrantSpec, RuntimeCapabilityOperation,
  RuntimeConfigSpec, RuntimeHostFactory, RuntimeHostFactoryRegistry, RuntimeHostInputDriver,
  RuntimeHostInputQueue, RuntimeModuleResult, RuntimeResourceKey, RuntimeResourceProvider,
  RuntimeResourceReadRequest, RuntimeResourceRegistry, RuntimeResourceWriteIntent,
  RuntimeResourceWriteRequest, RuntimeValueSnapshot, DEFAULT_HOST_INPUT_CAPACITY,
};
