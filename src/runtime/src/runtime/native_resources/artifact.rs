//! Canonical driverless observations and one explicit native-host transaction.
//!
//! This module selects no computation and evaluates no graph. A host must first
//! prepare its checked native entry against the SAME unmodified ProgramArtifact.
//! The invocation closure is privileged Rust host code, not a source-callable
//! function or evidence that a supplied implementation is native/AOT.
//!
//! Input text caps below apply after the provider returns and before retention
//! by this bridge. They do NOT bound a provider's stream acquisition or cache.
//! The caller still owns shutdown on every application exit path. Native input
//! rebinding/import and effect construction require the native caller's ordinary
//! finite memory/work admission; this bridge does not grant an unmetered import.

use std::collections::BTreeMap;

use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, CellSlotId, ExecutionResourceRequest,
    MResult, MechError, NodeId, ProgramRevision, ResourceDelivery, ResourceIntent, SchemaBody,
    SchemaId, SchemaKey, Value, ValueData,
};
use mech_engine::{ArtifactSource, ExecutableNodeBody, ProgramArtifact, SlotRole};

use crate::{
    ResourceSchemaPreflight, RuntimeCommitOutcome, RuntimeContext, RuntimeId, TransactionId,
    compute_effect_contract, preflight_artifact_resource_schemas, resource_observation_contract,
    validate_artifact_resource_schema_contract,
};

use super::{MechRuntime, NativeResourceBindingError};
use crate::runtime::transaction::RuntimeCommitResolution;

/// Descriptive action owned by a site selected from the original artifact.
/// Constructing a copy of an action grants neither I/O nor native-entry authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalNativeResourceAction {
    Observation {
        output: CellSlotId,
    },
    Effect {
        payload: ArtifactSource,
        ordinal: u32,
    },
}

/// Constructor-private port identity. There is no request replacement API.
#[derive(Clone, Debug)]
pub struct CanonicalNativeResourceSite {
    node: NodeId,
    requirement: ApplicationRequirementId,
    request: ExecutionResourceRequest,
    schema: SchemaId,
    schema_key: SchemaKey,
    action: CanonicalNativeResourceAction,
}
impl CanonicalNativeResourceSite {
    pub const fn node(&self) -> NodeId {
        self.node
    }
    pub const fn requirement(&self) -> ApplicationRequirementId {
        self.requirement
    }
    pub fn request(&self) -> &ExecutionResourceRequest {
        &self.request
    }
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }
    pub const fn schema_key(&self) -> SchemaKey {
        self.schema_key
    }
    pub const fn action(&self) -> CanonicalNativeResourceAction {
        self.action
    }
}

/// Host-side descriptor only, never another serialized or executable program.
/// The canonical revision commits to requirements, contracts and bindings.
#[derive(Clone, Debug)]
pub struct CanonicalNativeResourcePlan {
    revision: ProgramRevision,
    requirements: Box<[ExecutionResourceRequest]>,
    observations: Box<[CanonicalNativeResourceSite]>,
    effects: Box<[CanonicalNativeResourceSite]>,
}
impl CanonicalNativeResourcePlan {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn requirements(&self) -> &[ExecutionResourceRequest] {
        &self.requirements
    }
    pub fn observations(&self) -> &[CanonicalNativeResourceSite] {
        &self.observations
    }
    pub fn effects(&self) -> &[CanonicalNativeResourceSite] {
        &self.effects
    }

    /// Metadata-only selection. Actual provider and capability admission is
    /// mandatory again in with_canonical_native_resources_phased before any read.
    pub fn select(artifact: &ProgramArtifact, max_ports: usize) -> MResult<Self> {
        if artifact.requirements().len() > max_ports {
            return invalid("native resource requirement count exceeds its finite port cap");
        }
        if artifact
            .slots()
            .iter()
            .any(|slot| slot.role == SlotRole::State)
            || artifact.nodes().iter().any(|node| {
                matches!(
                    &node.body,
                    ExecutableNodeBody::Activation(_) | ExecutableNodeBody::Fsm(_)
                )
            })
        {
            return invalid("native resource bridge supports stateless snapshot turns only");
        }

        // Retain ALL original requirements, including unused declarations.
        // A missing grant cannot be concealed by pruning the execution view.
        let mut requirements = Vec::with_capacity(artifact.requirements().len());
        for (id, requirement) in artifact.requirements().iter() {
            if id.get() as usize != requirements.len() {
                return invalid("canonical requirement order is not contiguous");
            }
            let ApplicationRequirement::Resource(request) = requirement else {
                return invalid("native snapshot ports do not support host-function requirements");
            };
            let supported_delivery = match request.intent {
                ResourceIntent::Read => request.delivery == ResourceDelivery::Live,
                ResourceIntent::Send => request.delivery == ResourceDelivery::Snapshot,
                ResourceIntent::Assign => false,
            };
            if !supported_delivery {
                return invalid(
                    "canonical native ports require Live observations or Snapshot sends",
                );
            }
            requirements.push(request.clone());
        }

        let mut observations = Vec::new();
        let mut effects = Vec::new();
        preflight_artifact_resource_schemas(artifact, |site| {
            if observations
                .len()
                .checked_add(effects.len())
                .is_none_or(|count| count >= max_ports)
            {
                return invalid("native resource site count exceeds its finite port cap");
            }
            if !matches!(site.schema.body(), SchemaBody::String)
                || !site.schema.dimension_parameters().is_empty()
            {
                return invalid("native snapshot resource sites require plain String schemas");
            }
            let request = requirements
                .get(site.requirement.get() as usize)
                .ok_or_else(|| error("resource site requirement is absent"))?;
            if request != site.request {
                return invalid("resource site request differs from its original requirement");
            }
            let action = match site.request.intent {
                ResourceIntent::Read => {
                    validate_artifact_resource_schema_contract(
                        &site,
                        Some(resource_observation_contract()),
                    )?;
                    CanonicalNativeResourceAction::Observation {
                        output: site
                            .observation_output
                            .ok_or_else(|| error("resource observation has no output slot"))?,
                    }
                }
                ResourceIntent::Send => {
                    validate_artifact_resource_schema_contract(
                        &site,
                        Some(compute_effect_contract()),
                    )?;
                    CanonicalNativeResourceAction::Effect {
                        payload: site
                            .payload_source
                            .ok_or_else(|| error("resource effect has no payload binding"))?,
                        ordinal: u32::try_from(effects.len())
                            .map_err(|_| error("resource effect ordinal overflows"))?,
                    }
                }
                ResourceIntent::Assign => return invalid("native assignment is unsupported"),
            };
            let selected = CanonicalNativeResourceSite {
                node: site.node,
                requirement: site.requirement,
                request: request.clone(),
                schema: site.schema_id,
                schema_key: site.schema.key(),
                action,
            };
            match action {
                CanonicalNativeResourceAction::Observation { .. } => observations.push(selected),
                CanonicalNativeResourceAction::Effect { .. } => effects.push(selected),
            }
            Ok(ResourceSchemaPreflight::Validated)
        })?;

        // Resident capture order is input-slot order, not source-node order.
        // All supported String shapes are closed and identical here.
        observations.sort_by_key(|site| match site.action {
            CanonicalNativeResourceAction::Observation { output } => output.get(),
            CanonicalNativeResourceAction::Effect { .. } => unreachable!(),
        });
        // Same structural scheduler as resident execution: delivery follows
        // execution order, while stable effect ordinals remain node order.
        let order = mech_engine::resident::canonical_resource_effect_order(artifact)
            .map_err(|failure| error(format!("resource schedule: {failure:?}")))?;
        if order.len() != effects.len() || effects.iter().any(|site| !order.contains(&site.node)) {
            return invalid("canonical resource effect schedule differs from retained sites");
        }
        effects.sort_by_key(|site| order.iter().position(|node| *node == site.node).unwrap());
        Ok(Self {
            revision: artifact.revision(),
            requirements: requirements.into_boxed_slice(),
            observations: observations.into_boxed_slice(),
            effects: effects.into_boxed_slice(),
        })
    }
}

/// Explicit finite bridge-retention caps, not a provider stream/cache budget.
#[derive(Clone, Copy, Debug)]
pub struct CanonicalNativeResourceLimits {
    pub max_ports: usize,
    pub max_input_text_bytes: u64,
    pub max_output_text_bytes: u64,
}

/// Opaque identity tying pre-capture activation to the SAME admitted runtime
/// transaction. This is metadata, not authority; only opaque Admission/Turn
/// tokens can enter the hosted native execution boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalNativeResourceInvocation {
    runtime: RuntimeId,
    transaction: TransactionId,
}

/// Opaque pre-capture authority issued after ALL original requirements and
/// provider contracts pass canonical admission. It deliberately contains no
/// observations or I/O callback. The checked native preparer may run only its
/// generated closed activation phase, retaining the same state/work/memory for
/// resume; it must not publish values or effects.
pub struct CanonicalNativeResourceAdmission<'a> {
    plan: &'a CanonicalNativeResourcePlan,
    invocation: CanonicalNativeResourceInvocation,
    _private: (),
}
impl CanonicalNativeResourceAdmission<'_> {
    pub fn plan(&self) -> &CanonicalNativeResourcePlan {
        self.plan
    }
    pub fn program_revision(&self) -> ProgramRevision {
        self.plan.program_revision()
    }
    pub const fn invocation(&self) -> CanonicalNativeResourceInvocation {
        self.invocation
    }
}

/// Opaque authority/capture proof issued only inside the runtime's open turn.
/// A native hosted invocation consumes this token. Public metadata selection
/// cannot construct one; raw evaluate_into must reject hosted descriptors.
/// It carries no ambient I/O callback and cannot outlive the local capture.
pub struct CanonicalNativeResourceTurn<'a> {
    plan: &'a CanonicalNativeResourcePlan,
    observations: &'a [Value],
    invocation: CanonicalNativeResourceInvocation,
    _private: (),
}
impl CanonicalNativeResourceTurn<'_> {
    pub fn plan(&self) -> &CanonicalNativeResourcePlan {
        self.plan
    }
    pub fn observations(&self) -> &[Value] {
        self.observations
    }
    pub const fn invocation(&self) -> CanonicalNativeResourceInvocation {
        self.invocation
    }
    pub fn program_revision(&self) -> ProgramRevision {
        self.plan.program_revision()
    }
}

/// A native result stays local until all effect payloads pass validation and
/// the ordinary transaction commits. The checked native caller supplies one
/// payload per exact retained effect, in descriptor order. It must NOT publish
/// caller-visible results or perform external effects inside the closure.
pub struct CanonicalNativeResourceCompletion<R> {
    pub result: R,
    pub effects: Box<[Value]>,
}

/// Precommit failure preserves the original typed error and cleanup failure.
/// Postcommit failures are explicitly distinguishable: external delivery may
/// already have happened, so they are NOT an atomic denial or safe retry.
#[derive(Debug)]
pub enum CanonicalNativeResourceRunError {
    BeforeCommit {
        phase: &'static str,
        error: MechError,
        cleanup: Option<MechError>,
    },
    Commit {
        error: MechError,
        cleanup: Option<MechError>,
    },
    /// Runtime store publication is indeterminate. No rollback or blind retry
    /// is safe; the original typed store error remains available for recovery.
    CommitIndeterminate {
        error: MechError,
    },
    /// The runtime store committed, but a later participant/audit/local step
    /// failed. External state may also be indeterminate. This is NOT a denial
    /// that preserved the old logical state, and must never be retried blindly.
    CommittedWithError {
        error: MechError,
    },
    CommittedWithFailures(RuntimeCommitOutcome),
}
impl std::fmt::Display for CanonicalNativeResourceRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native resource execution: {self:?}")
    }
}
impl std::error::Error for CanonicalNativeResourceRunError {}

fn error(reason: impl Into<String>) -> MechError {
    MechError::new(
        NativeResourceBindingError {
            reason: reason.into(),
        },
        None,
    )
}
fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(error(reason))
}
fn preflight(error: MechError) -> CanonicalNativeResourceRunError {
    CanonicalNativeResourceRunError::BeforeCommit {
        phase: "admission",
        error,
        cleanup: None,
    }
}
fn committed_error(error: MechError) -> CanonicalNativeResourceRunError {
    if error
        .kind_as::<crate::RuntimeStoreCommitIndeterminate>()
        .is_some()
    {
        CanonicalNativeResourceRunError::CommitIndeterminate { error }
    } else {
        CanonicalNativeResourceRunError::CommittedWithError { error }
    }
}
fn admit_text(value: &Value, total: &mut u64, limit: u64) -> MResult<()> {
    let ValueData::String(text) = value.data() else {
        return invalid("native resource value is not String");
    };
    let bytes = u64::try_from(text.len()).map_err(|_| error("native text size overflows"))?;
    let next = total
        .checked_add(bytes)
        .ok_or_else(|| error("native text total overflows"))?;
    if next > limit {
        return invalid("native resource text exceeds its finite retention cap");
    }
    *total = next;
    Ok(())
}

/// Owns cleanup even when privileged host code unwinds. Normal failures expose
/// cleanup errors; unwinding cleanup still invokes ordinary runtime poisoning.
struct PendingNativeTransaction<'a> {
    runtime: &'a mut MechRuntime,
    context: RuntimeContext,
    active: bool,
}
impl PendingNativeTransaction<'_> {
    fn commit(&mut self) -> Result<RuntimeCommitOutcome, CanonicalNativeResourceRunError> {
        // Preserve the SAME public commit health/reentrancy/environment gate,
        // then retain the internal resolution that its public wrapper flattens.
        let resolution = self
            .runtime
            .ensure_runtime_mutation_allowed("commit_runtime_transaction")
            .and_then(|()| {
                self.runtime
                    .commit_runtime_transaction_internal(&mut self.context)
            });
        match resolution {
            Ok(RuntimeCommitResolution::Committed(outcome)) => {
                self.active = false;
                Ok(outcome)
            }
            Ok(RuntimeCommitResolution::CommittedWithError(error)) => {
                // The shared commit protocol has already closed the context
                // and poisoned/reconciled as appropriate. Do not abort it.
                self.active = false;
                Err(committed_error(error))
            }
            Err(error) if self.context.transaction_id().is_none() => {
                // A fallible persisted-event/context finalization step can
                // fail AFTER the shared protocol has published and cleared
                // the transaction. That authoritative boundary is not rolled
                // back merely because the final local step returned Err.
                self.active = false;
                Err(committed_error(error))
            }
            Err(error) => {
                let cleanup = self.abort();
                Err(CanonicalNativeResourceRunError::Commit { error, cleanup })
            }
        }
    }
    fn abort(&mut self) -> Option<MechError> {
        if !self.active {
            return None;
        }
        self.active = false;
        if self.context.transaction_id().is_none() {
            return None;
        }
        self.runtime
            .abort_runtime_transaction(
                &mut self.context,
                "canonical native execution did not commit",
            )
            .err()
    }
    fn reject(&mut self, phase: &'static str, error: MechError) -> CanonicalNativeResourceRunError {
        CanonicalNativeResourceRunError::BeforeCommit {
            phase,
            error,
            cleanup: self.abort(),
        }
    }
}
impl Drop for PendingNativeTransaction<'_> {
    fn drop(&mut self) {
        let _ = self.abort();
    }
}

impl MechRuntime {
    /// One fresh admitted native invocation, in canonical order:
    /// all authority/provider preflight -> generated closed activation ->
    /// provider capture -> SAME native state resumed -> effect stage/commit.
    ///
    /// Neither callback receives resource callbacks or the runtime itself.
    /// prepare must return an owning checked native execution session carrying
    /// its original entry provenance, invocation identity, current-visit state,
    /// cumulative work and memory accounts. invoke must consume THAT session
    /// with the matching Turn; it must not recreate an entry, ledger or limits.
    /// No resident evaluator, source parser, source-native compiler or Rust
    /// callee graph is used here. All computed candidates remain host-local
    /// until every effect validates and the transaction commits.
    ///
    /// Any prepare/capture/invoke failure drops the session, aborts the same
    /// ordinary transaction and publishes no candidate. Caller owns shutdown.
    pub fn with_canonical_native_resources_phased<S, R>(
        &mut self,
        artifact: &ProgramArtifact,
        limits: CanonicalNativeResourceLimits,
        prepare: impl FnOnce(CanonicalNativeResourceAdmission<'_>) -> MResult<S>,
        invoke: impl FnOnce(
            S,
            CanonicalNativeResourceTurn<'_>,
        ) -> MResult<CanonicalNativeResourceCompletion<R>>,
    ) -> Result<R, CanonicalNativeResourceRunError> {
        let plan =
            CanonicalNativeResourcePlan::select(artifact, limits.max_ports).map_err(preflight)?;
        // Reuse the SAME canonical session admission proof, including the
        // finite-use-capability rejection. This does not activate/evaluate a
        // resident instance or introduce another capability-policy authority.
        let authority = self.build_resident_authority(artifact).map_err(preflight)?;
        // Binding checks EVERY original requirement before observations or
        // provider preparation. Actual reads/sends recheck ordinary authority.
        let bindings = self
            .bind_native_resources_with_driverless_live_reads(&plan.requirements, true)
            .map_err(preflight)?;
        authority.revalidate(self).map_err(preflight)?;
        let mut context = self.runtime_context().map_err(preflight)?;
        let transaction_id = self.begin_transaction(&mut context).map_err(preflight)?;
        let invocation = CanonicalNativeResourceInvocation {
            runtime: context.runtime_id(),
            transaction: transaction_id,
        };
        let mut transaction = PendingNativeTransaction {
            runtime: self,
            context,
            active: true,
        };

        let prepared = prepare(CanonicalNativeResourceAdmission {
            plan: &plan,
            invocation,
            _private: (),
        })
        .map_err(|error| transaction.reject("native activation", error))?;
        authority
            .revalidate(transaction.runtime)
            .map_err(|error| transaction.reject("authorization before capture", error))?;

        let mut input_bytes = 0;
        let mut cache = BTreeMap::<ApplicationRequirementId, Value>::new();
        let mut inputs = Vec::with_capacity(plan.observations.len());
        for site in &plan.observations {
            let value = if let Some(value) = cache.get(&site.requirement) {
                value.clone()
            } else {
                let value = transaction
                    .runtime
                    .read_native_resource_with_context(
                        &bindings,
                        site.requirement.get() as usize,
                        &mut transaction.context,
                    )
                    .map_err(|error| transaction.reject("capture", error))?;
                cache.insert(site.requirement, value.clone());
                value
            };
            admit_text(&value, &mut input_bytes, limits.max_input_text_bytes)
                .map_err(|error| transaction.reject("capture", error))?;
            if value.schema_key() != site.schema_key {
                return Err(transaction.reject(
                    "capture",
                    error("provider String schema differs from canonical observation"),
                ));
            }
            inputs.push(value);
        }
        authority
            .revalidate(transaction.runtime)
            .map_err(|error| transaction.reject("authorization after capture", error))?;

        let completion = invoke(
            prepared,
            CanonicalNativeResourceTurn {
                plan: &plan,
                observations: &inputs,
                invocation,
                _private: (),
            },
        )
        .map_err(|error| transaction.reject("native execution", error))?;
        if completion.effects.len() != plan.effects.len() {
            return Err(transaction.reject(
                "effect materialization",
                error("native effect count differs from canonical plan"),
            ));
        }
        // Validate ALL materialized payloads before preparing even the first
        // provider effect. No arbitrary request or ordinal comes from native
        // words; each site is the immutable artifact-selected binding.
        let mut output_bytes = 0;
        for (value, site) in completion.effects.iter().zip(&plan.effects) {
            admit_text(value, &mut output_bytes, limits.max_output_text_bytes)
                .map_err(|error| transaction.reject("effect materialization", error))?;
            if value.schema() != site.schema || value.schema_key() != site.schema_key {
                return Err(transaction.reject(
                    "effect materialization",
                    error("native effect type differs from canonical plan"),
                ));
            }
            value
                .validate_against(artifact.schemas())
                .map_err(|error| {
                    transaction.reject(
                        "effect materialization",
                        self::error(format!("invalid native effect snapshot: {error:?}")),
                    )
                })?;
        }
        authority.revalidate(transaction.runtime).map_err(|error| {
            transaction.reject("authorization before effect preparation", error)
        })?;
        for (value, site) in completion.effects.into_vec().into_iter().zip(&plan.effects) {
            transaction
                .runtime
                .write_native_resource_with_context(
                    &bindings,
                    site.requirement.get() as usize,
                    value,
                    &mut transaction.context,
                )
                .map_err(|error| transaction.reject("effect preparation", error))?;
        }

        let outcome = transaction.commit()?;
        if !outcome.delivery_failures.is_empty() || !outcome.audit_failures.is_empty() {
            return Err(CanonicalNativeResourceRunError::CommittedWithFailures(
                outcome,
            ));
        }
        Ok(completion.result)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CanonicalNativeResourceRunError, MechRuntime, NativeResourceBindingError,
        PendingNativeTransaction, admit_text, invalid,
    };
    use mech_core::{MResult, Value};
    #[derive(Debug)]
    struct FailingParticipant {
        fail_prepare: bool,
        fail_commit: bool,
    }
    impl crate::RuntimeTransactionalEffect for FailingParticipant {
        fn metadata(&self) -> crate::RuntimeEffectMetadata {
            crate::RuntimeEffectMetadata::new(
                crate::RuntimeEffectSource::Custom {
                    name: "canonical-native-commit-test".into(),
                },
                "commit",
            )
        }
        fn prepare(&mut self) -> MResult<()> {
            if self.fail_prepare {
                invalid("participant prepare failed")
            } else {
                Ok(())
            }
        }
        fn commit(&mut self) -> MResult<()> {
            if self.fail_commit {
                invalid("participant commit failed")
            } else {
                Ok(())
            }
        }
        fn abort(&mut self) -> MResult<()> {
            Ok(())
        }
    }
    fn text(value: &str) -> Value {
        crate::RuntimeHostInputValue::String(value.into())
            .into_value()
            .unwrap()
    }
    #[test]
    fn native_commit_preserves_precommit_failure_and_aborts_its_active_transaction() {
        let mut runtime = MechRuntime::builder().build().unwrap();
        let mut context = runtime.runtime_context().unwrap();
        let id = runtime.begin_transaction(&mut context).unwrap();
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                crate::PreparedRuntimeEffect::Transactional(Box::new(FailingParticipant {
                    fail_prepare: true,
                    fail_commit: false,
                })),
            )
            .unwrap();
        let mut transaction = PendingNativeTransaction {
            runtime: &mut runtime,
            context,
            active: true,
        };
        let failure = transaction.commit().unwrap_err();
        let CanonicalNativeResourceRunError::Commit { error, cleanup } = failure else {
            panic!("{failure:?}")
        };
        assert!(error.kind_as::<NativeResourceBindingError>().is_some());
        assert!(cleanup.is_none());
        assert!(!transaction.active);
        assert!(transaction.context.transaction_id().is_none());
        assert!(!transaction.runtime.active_transactions.contains_key(&id));
        assert!(!transaction.runtime.is_poisoned());
    }

    #[test]
    fn native_commit_preserves_certain_store_commit_with_indeterminate_participant() {
        let mut runtime = MechRuntime::builder().build().unwrap();
        let mut context = runtime.runtime_context().unwrap();
        let id = runtime.begin_transaction(&mut context).unwrap();
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                crate::PreparedRuntimeEffect::Transactional(Box::new(FailingParticipant {
                    fail_prepare: false,
                    fail_commit: true,
                })),
            )
            .unwrap();
        let mut transaction = PendingNativeTransaction {
            runtime: &mut runtime,
            context,
            active: true,
        };
        let failure = transaction.commit().unwrap_err();
        let CanonicalNativeResourceRunError::CommittedWithError { error } = failure else {
            panic!("{failure:?}")
        };
        assert!(
            error
                .kind_as::<crate::RuntimeExternalCommitIndeterminate>()
                .is_some()
        );
        assert!(
            !transaction.active,
            "Drop must not attempt rollback after publication"
        );
        assert!(transaction.context.transaction_id().is_none());
        assert!(transaction.runtime.get_transaction(id).unwrap().is_some());
        assert!(!transaction.runtime.active_transactions.contains_key(&id));
        assert!(transaction.runtime.is_poisoned());
    }

    #[cfg(feature = "source")]
    #[test]
    fn native_commit_reports_the_existing_store_panic_hook_as_indeterminate_not_aborted() {
        let mut store = crate::InMemoryStore::new();
        store.panic_on_commit_runtime_for_test();
        let mut runtime = MechRuntime::builder().store(store).build().unwrap();
        let mut context = runtime.runtime_context().unwrap();
        let id = runtime.begin_transaction(&mut context).unwrap();
        let mut transaction = PendingNativeTransaction {
            runtime: &mut runtime,
            context,
            active: true,
        };
        let failure = transaction.commit().unwrap_err();
        let CanonicalNativeResourceRunError::CommitIndeterminate { error } = failure else {
            panic!("{failure:?}")
        };
        assert!(
            error
                .kind_as::<crate::RuntimeStoreCommitIndeterminate>()
                .is_some()
        );
        assert!(
            !transaction.active,
            "indeterminate store state is not abortable"
        );
        assert!(transaction.context.transaction_id().is_none());
        assert!(!transaction.runtime.active_transactions.contains_key(&id));
        assert!(transaction.runtime.is_poisoned());
    }

    #[test]
    fn text_retention_admission_is_exact_and_atomic_on_failure() {
        let mut total = 0;
        admit_text(&text(""), &mut total, 0).unwrap();
        admit_text(&text("é"), &mut total, 2).unwrap();
        assert_eq!(total, 2);
        assert!(admit_text(&text("x"), &mut total, 2).is_err());
        assert_eq!(total, 2);
        let mut overflow = u64::MAX;
        assert!(admit_text(&text("x"), &mut overflow, u64::MAX).is_err());
        assert_eq!(overflow, u64::MAX);
    }
}
