use crate::*;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::cell::RefCell as StdRefCell;
#[cfg(feature = "invariant_define")]
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::ops::Deref;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
#[cfg(feature = "functions")]
use std::sync::Arc;
use std::time::Duration;
#[cfg(all(target_arch = "wasm32", target_os = "unknown",))]
use web_time::Instant;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown",)))]
use std::time::Instant;

mod bytecode;
use bytecode::{BytecodeRegisterFile, BytecodeRegisterFileCheckpoint};

// Interpreter
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeContextBinding {
    pub context_name: String,
    pub base_uri: String,
}

pub type InterpreterRef = Ref<Box<Interpreter>>;

/// Reactive bytecode pack for collections whose identity is hash-backed.
///
/// Tuple/record/table composites can safely retain live child cells. Maps and
/// sets cannot: a producer mutation would change a key's hash after insertion.
/// This node runs after those producers, rebuilds detached hashed children,
/// and replaces the contents of one stable outer output cell.
#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
#[derive(Debug)]
struct BytecodeHashedCompositePack {
    template: LegacyValue,
    children: Vec<LegacyValue>,
    output: LegacyValue,
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
impl MechFunctionImpl for BytecodeHashedCompositePack {
    fn solve_result(&self) -> MResult<()> {
        let rebuilt = mech_core::rebuild_bytecode_composite(&self.template, self.children.clone())?;
        match (&self.output, rebuilt) {
            #[cfg(feature = "map")]
            (LegacyValue::Map(output), LegacyValue::Map(rebuilt)) => {
                *output.borrow_mut() = rebuilt.borrow().clone();
            }
            #[cfg(feature = "set")]
            (LegacyValue::Set(output), LegacyValue::Set(rebuilt)) => {
                *output.borrow_mut() = rebuilt.borrow().clone();
            }
            _ => {
                return Err(MechError::new(
                    BytecodeValidationError {
                        reason: "hashed CompositePack output changed runtime kind".to_string(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        self.output.clone()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "BytecodeHashedCompositePack".to_string()
    }
}

#[cfg(all(
    feature = "semantic-compiler",
    feature = "program",
    feature = "functions",
    feature = "symbol_table"
))]
impl MechFunctionCompiler for BytecodeHashedCompositePack {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            BytecodeValidationError {
                reason: "an installed bytecode CompositePack cannot be recompiled".to_string(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

fn canonical_context_name_from_base_uri(base_uri: &str) -> String {
    base_uri
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(base_uri)
        .to_owned()
}

pub struct Interpreter {
    pub id: u64,
    checkpoint_owner: Rc<()>,
    #[cfg(feature = "functions")]
    function_catalog: Arc<FunctionCatalog>,
    pub profile: bool,
    pub max_steps: usize,
    #[cfg(feature = "trace")]
    pub trace: bool,
    #[cfg(feature = "trace")]
    pub trace_to_stdout: bool,
    #[cfg(feature = "trace")]
    pub trace_events: Ref<Vec<TraceEvent>>,
    ip: usize, // instruction pointer
    pub state: Ref<ProgramState>,
    #[cfg(feature = "functions")]
    reactive_turn_state: ReactiveTurnState,
    #[cfg(feature = "functions")]
    pub(crate) persistent_user_function_plan_depth: Ref<usize>,
    pub(crate) deferred_expression_solve_depth: Ref<usize>,
    #[cfg(feature = "functions")]
    pub stack: Vec<Frame>,
    bytecode_registers: BytecodeRegisterFile,
    constants: Vec<LegacyValue>,
    pub code: Vec<MechSourceCode>,
    pub out: LegacyValue,
    pub out_values: Ref<HashMap<u64, LegacyValue>>,
    #[cfg(feature = "subscript_formula")]
    pub string_access_live_values: Ref<std::collections::BTreeSet<usize>>,
    #[cfg(feature = "subscript_formula")]
    pub current_string_access_expression_live: Ref<bool>,
    pub inline_eval_counter: Ref<u64>,
    pub context_bindings: Ref<HashMap<u64, RuntimeContextBinding>>,
    pub module_manifests: Ref<ModuleManifestCatalog>,
    #[cfg(feature = "state_machines")]
    pub user_state_machines: Ref<HashMap<u64, FsmImplementation>>,
    #[cfg(feature = "state_machines")]
    pub user_state_machine_specs: Ref<HashMap<u64, FsmSpecification>>,
    pub sub_interpreters: Ref<HashMap<u64, InterpreterRef>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpreterBytecodeInstallRollbackFailed {
    pub interpreter_id: u64,
    pub original_error: String,
    pub rollback_error: String,
}

impl MechErrorKind for InterpreterBytecodeInstallRollbackFailed {
    fn name(&self) -> &str {
        "InterpreterBytecodeInstallRollbackFailed"
    }

    fn message(&self) -> String {
        format!(
            "interpreter {} could not roll back a failed bytecode installation after {}: {}",
            self.interpreter_id, self.original_error, self.rollback_error,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeResourceReadPlanningEvidenceError {
    pub instruction: u32,
    pub reason: String,
}

impl MechErrorKind for BytecodeResourceReadPlanningEvidenceError {
    fn name(&self) -> &str {
        "BytecodeResourceReadPlanningEvidence"
    }

    fn message(&self) -> String {
        format!(
            "bytecode ResourceRead planning evidence for instruction {} is invalid: {}",
            self.instruction, self.reason,
        )
    }
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
#[derive(Clone, Debug)]
struct PlannedResourceRead {
    request: ExecutionResourceRequest,
    representative: LegacyValue,
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
struct ExecutionServicesBytecodeContractResolver<'a> {
    services: &'a mut dyn MechExecutionServices,
    structural: StructuralExternalContractResolver,
    resource_reads: HashMap<u32, PlannedResourceRead>,
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
impl<'a> ExecutionServicesBytecodeContractResolver<'a> {
    fn new(services: &'a mut dyn MechExecutionServices) -> Self {
        Self {
            services,
            structural: StructuralExternalContractResolver,
            resource_reads: HashMap::new(),
        }
    }

    fn into_resource_reads(self) -> HashMap<u32, PlannedResourceRead> {
        self.resource_reads
    }
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
impl BytecodeExternalContractResolver for ExecutionServicesBytecodeContractResolver<'_> {
    fn validate_host_call(
        &mut self,
        contract: BytecodeHostCallContract<'_>,
    ) -> MResult<LegacyValue> {
        self.structural.validate_host_call(contract)
    }

    fn validate_resource_read(
        &mut self,
        contract: BytecodeResourceReadContract<'_>,
    ) -> MResult<LegacyValue> {
        let representative = self
            .services
            .plan_resource_read_output(contract.request)?
            .try_deep_snapshot()?;
        let validation_value = representative.try_deep_snapshot()?;
        let previous = self.resource_reads.insert(
            contract.instruction,
            PlannedResourceRead {
                request: contract.request.clone(),
                representative,
            },
        );
        if previous.is_some() {
            return Err(bytecode_resource_read_planning_evidence_error(
                contract.instruction,
                "duplicate evidence was supplied for one instruction",
            ));
        }
        Ok(validation_value)
    }

    fn validate_resource_write(
        &mut self,
        contract: BytecodeResourceWriteContract<'_>,
    ) -> MResult<()> {
        self.structural.validate_resource_write(contract)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointBorrowConflict {
    pub phase: &'static str,
    pub interpreter_id: u64,
    pub type_name: &'static str,
}

impl MechErrorKind for InterpreterCheckpointBorrowConflict {
    fn name(&self) -> &str {
        "InterpreterCheckpointBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Cannot borrow {} checkpoint target for interpreter {} during {}.",
            self.type_name, self.interpreter_id, self.phase,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointHierarchyAlias {
    pub interpreter_id: u64,
    pub child_id: u64,
}

impl MechErrorKind for InterpreterCheckpointHierarchyAlias {
    fn name(&self) -> &str {
        "InterpreterCheckpointHierarchyAlias"
    }

    fn message(&self) -> String {
        format!(
            "Interpreter {} contains a repeated or cyclic handle for child {}.",
            self.interpreter_id, self.child_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointOwnerMismatch {
    pub interpreter_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointChildKeyMismatch {
    pub parent_id: u64,
    pub key: u64,
    pub child_id: u64,
}

impl MechErrorKind for InterpreterCheckpointChildKeyMismatch {
    fn name(&self) -> &str {
        "InterpreterCheckpointChildKeyMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Interpreter {} stores child {} under mismatched key {}.",
            self.parent_id, self.child_id, self.key,
        )
    }
}

impl MechErrorKind for InterpreterCheckpointOwnerMismatch {
    fn name(&self) -> &str {
        "InterpreterCheckpointOwnerMismatch"
    }

    fn message(&self) -> String {
        format!(
            "The checkpoint belongs to a different interpreter than interpreter {}.",
            self.interpreter_id,
        )
    }
}

fn checkpoint_borrow_error<T: 'static>(
    phase: &'static str,
    interpreter_id: u64,
    _target: &Ref<T>,
) -> MechError {
    MechError::new(
        InterpreterCheckpointBorrowConflict {
            phase,
            interpreter_id,
            type_name: core::any::type_name::<T>(),
        },
        None,
    )
    .with_compiler_loc()
}

fn checkpoint_clone_ref<T: Clone + 'static>(target: &Ref<T>, interpreter_id: u64) -> MResult<T> {
    target
        .try_borrow()
        .map(|value| value.clone())
        .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, target))
}

fn checkpoint_preflight_ref<T: 'static>(target: &Ref<T>, interpreter_id: u64) -> MResult<()> {
    target
        .try_borrow_mut()
        .map(|_| ())
        .map_err(|_| checkpoint_borrow_error("checkpoint-restore", interpreter_id, target))
}

#[cfg(feature = "functions")]
fn checkpoint_preflight_plan_capture(plan: &Plan, interpreter_id: u64) -> MResult<()> {
    plan.0
        .try_borrow()
        .map(|_| ())
        .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, &plan.0))?;
    plan.1
        .try_borrow()
        .map(|_| ())
        .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, &plan.1))?;
    plan.validate_checkpoint_invariants()
}

struct RefPayloadCheckpoint<T: Clone + 'static> {
    target: Ref<T>,
    payload: T,
}

impl<T: Clone + 'static> RefPayloadCheckpoint<T> {
    fn capture(target: &Ref<T>, interpreter_id: u64) -> MResult<Self> {
        Ok(Self {
            target: target.clone(),
            payload: checkpoint_clone_ref(target, interpreter_id)?,
        })
    }

    fn preflight(&self, interpreter_id: u64) -> MResult<()> {
        checkpoint_preflight_ref(&self.target, interpreter_id)
    }

    fn apply(&self) {
        *self.target.borrow_mut() = self.payload.clone();
    }
}

#[cfg(feature = "symbol_table")]
struct SymbolTableCellCheckpoint {
    target: SymbolTableRef,
    snapshot: SymbolTableSnapshot,
}

#[cfg(feature = "symbol_table")]
impl SymbolTableCellCheckpoint {
    fn capture(
        target: &SymbolTableRef,
        interpreter_id: u64,
        journal: &mut ValueStateJournal,
    ) -> MResult<Self> {
        let snapshot = {
            let table = target.try_borrow().map_err(|_| {
                checkpoint_borrow_error("checkpoint-capture", interpreter_id, target)
            })?;
            table.dictionary.try_borrow().map_err(|_| {
                checkpoint_borrow_error("checkpoint-capture", interpreter_id, &table.dictionary)
            })?;
            let snapshot = table.snapshot();
            for value in table.symbols.values() {
                journal.capture_val_ref(value)?;
            }
            for value in table.mutable_variables.values() {
                journal.capture_val_ref(value)?;
            }
            snapshot
        };
        Ok(Self {
            target: target.clone(),
            snapshot,
        })
    }

    fn preflight(&self, interpreter_id: u64) -> MResult<()> {
        checkpoint_preflight_ref(&self.target, interpreter_id)?;
        let table = self.target.try_borrow().map_err(|_| {
            checkpoint_borrow_error("checkpoint-restore", interpreter_id, &self.target)
        })?;
        table.preflight_restore(&self.snapshot)
    }

    fn apply(&self) {
        self.target.borrow_mut().apply_restore(&self.snapshot);
    }
}

#[cfg(feature = "functions")]
struct FrameCellCheckpoint {
    locals: SymbolTableCellCheckpoint,
    plan: Plan,
    plan_checkpoint: PlanCheckpoint,
}

#[cfg(feature = "functions")]
impl FrameCellCheckpoint {
    fn capture(
        frame: &Frame,
        interpreter_id: u64,
        journal: &mut ValueStateJournal,
    ) -> MResult<Self> {
        let plan = frame.checkpoint_plan();
        checkpoint_preflight_plan_capture(&plan, interpreter_id)?;
        for value in plan.transaction_state_values()? {
            journal.capture_value(&value)?;
        }
        if let Some(value) = frame.checkpoint_out() {
            journal.capture_value(&value)?;
        }
        Ok(Self {
            locals: SymbolTableCellCheckpoint::capture(
                &frame.checkpoint_locals(),
                interpreter_id,
                journal,
            )?,
            plan_checkpoint: plan.checkpoint(),
            plan,
        })
    }

    fn preflight(&self, interpreter_id: u64) -> MResult<()> {
        self.locals.preflight(interpreter_id)?;
        self.plan.preflight_rollback(&self.plan_checkpoint)
    }

    fn apply_structure(&self) {
        self.locals.apply();
        self.plan.apply_rollback_structure(&self.plan_checkpoint);
    }

    fn rebuild_checkpoint_indexes(&self) {
        self.plan.rebuild_checkpoint_indexes();
        self.plan
            .validate_checkpoint_invariants()
            .expect("frame checkpoint preflight guarantees restored plan invariants");
    }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone)]
pub struct UserFunctionsCheckpointBorrowConflictError {
    pub phase: &'static str,
    pub component: &'static str,
}

#[cfg(feature = "functions")]
impl MechErrorKind for UserFunctionsCheckpointBorrowConflictError {
    fn name(&self) -> &str {
        "UserFunctionsCheckpointBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Cannot borrow user-function {} during {}.",
            self.component, self.phase,
        )
    }
}

#[cfg(feature = "functions")]
#[derive(Clone)]
struct UserFunctionDefinitionCheckpoint {
    symbols_target: SymbolTableRef,
    symbols: SymbolTableSnapshot,
    symbol_dictionary_target: Ref<Dictionary>,
    out_target: Ref<LegacyValue>,
    plan_target: Plan,
    plan: PlanCheckpoint,
}

#[cfg(feature = "functions")]
#[derive(Clone)]
struct UserFunctionsCheckpoint {
    table: UserFunctionTable,
    definitions: Vec<UserFunctionDefinitionCheckpoint>,
}

#[cfg(feature = "functions")]
impl UserFunctionsCheckpoint {
    fn borrow_conflict(phase: &'static str, component: &'static str) -> MechError {
        MechError::new(
            UserFunctionsCheckpointBorrowConflictError { phase, component },
            None,
        )
        .with_compiler_loc()
    }

    fn capture(table: &UserFunctionTable) -> MResult<Self> {
        let mut definitions = Vec::with_capacity(table.len());
        for definition in table.definitions() {
            let symbols = definition
                .symbols
                .try_borrow()
                .map_err(|_| Self::borrow_conflict("capture", "symbol table"))?;
            let symbol_dictionary_target = symbols.dictionary.clone();
            let symbol_dictionary = symbol_dictionary_target
                .try_borrow()
                .map_err(|_| Self::borrow_conflict("capture", "symbol dictionary"))?;
            let symbol_snapshot = symbols.snapshot();
            drop(symbol_dictionary);
            drop(symbols);

            let plan = definition.plan.try_checkpoint()?;

            definitions.push(UserFunctionDefinitionCheckpoint {
                symbols_target: definition.symbols.clone(),
                symbols: symbol_snapshot,
                symbol_dictionary_target,
                out_target: definition.out.clone(),
                plan_target: definition.plan.clone(),
                plan,
            });
        }

        Ok(Self {
            table: table.clone(),
            definitions,
        })
    }

    fn preflight_restore(&self) -> MResult<()> {
        for definition in &self.definitions {
            {
                let _symbols = definition
                    .symbols_target
                    .try_borrow_mut()
                    .map_err(|_| Self::borrow_conflict("restore", "symbol table"))?;
                let _dictionary = definition
                    .symbol_dictionary_target
                    .try_borrow_mut()
                    .map_err(|_| Self::borrow_conflict("restore", "symbol dictionary"))?;
            }
            definition
                .plan_target
                .preflight_rollback(&definition.plan)?;
        }
        Ok(())
    }

    fn apply_restore_structure(&self) {
        for definition in &self.definitions {
            {
                let mut symbols = definition.symbols_target.borrow_mut();
                symbols.dictionary = definition.symbol_dictionary_target.clone();
                symbols.restore(definition.symbols.clone());
            }
            definition
                .plan_target
                .apply_rollback_structure(&definition.plan);
        }
    }

    fn rebuild_checkpoint_indexes(&self) {
        for definition in &self.definitions {
            definition.plan_target.rebuild_checkpoint_indexes();
        }
    }

    fn validate_checkpoint_invariants(&self) -> MResult<()> {
        for definition in &self.definitions {
            definition.plan_target.validate_checkpoint_invariants()?;
        }
        Ok(())
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        let mut values = Vec::new();
        let mut seen_refs = HashSet::new();
        for definition in &self.definitions {
            if seen_refs.insert(definition.out_target.addr()) {
                values.push(LegacyValue::MutableReference(definition.out_target.clone()));
            }
            let symbols = definition
                .symbols_target
                .try_borrow()
                .map_err(|_| Self::borrow_conflict("transaction-state", "symbol table"))?;
            for value in symbols
                .symbols
                .values()
                .chain(symbols.mutable_variables.values())
            {
                if seen_refs.insert(value.addr()) {
                    values.push(LegacyValue::MutableReference(value.clone()));
                }
            }
            drop(symbols);
            values.extend(definition.plan_target.transaction_state_values()?);
        }
        Ok(values)
    }
}

struct ProgramStateCheckpoint {
    target: Ref<ProgramState>,
    #[cfg(feature = "symbol_table")]
    symbol_table: SymbolTableCellCheckpoint,
    #[cfg(feature = "symbol_table")]
    environment: Option<SymbolTableCellCheckpoint>,
    #[cfg(feature = "functions")]
    function_environment: FunctionEnvironment,
    #[cfg(feature = "functions")]
    function_extensions: FunctionExtensions,
    #[cfg(feature = "functions")]
    user_functions: UserFunctionsCheckpoint,
    #[cfg(feature = "functions")]
    plan: Plan,
    #[cfg(feature = "functions")]
    plan_checkpoint: PlanCheckpoint,
    kinds: KindTable,
    #[cfg(feature = "enum")]
    enums: EnumTable,
    #[cfg(feature = "enum")]
    enum_dictionaries: Vec<RefPayloadCheckpoint<Dictionary>>,
    #[cfg(feature = "invariant_define")]
    integrity_constraints: IntegrityConstraintTable,
    dictionary: RefPayloadCheckpoint<Dictionary>,
}

impl ProgramStateCheckpoint {
    fn capture(
        target: &Ref<ProgramState>,
        interpreter_id: u64,
        journal: &mut ValueStateJournal,
    ) -> MResult<Self> {
        let state = target
            .try_borrow()
            .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, target))?;

        #[cfg(feature = "symbol_table")]
        let symbol_table =
            SymbolTableCellCheckpoint::capture(&state.symbol_table, interpreter_id, journal)?;
        #[cfg(feature = "symbol_table")]
        let environment = match &state.environment {
            Some(environment) => Some(SymbolTableCellCheckpoint::capture(
                environment,
                interpreter_id,
                journal,
            )?),
            None => None,
        };

        #[cfg(feature = "functions")]
        let function_environment = state.function_environment.clone();
        #[cfg(feature = "functions")]
        let function_extensions = state.function_extensions.clone();
        #[cfg(feature = "functions")]
        let user_functions = UserFunctionsCheckpoint::capture(&state.user_functions)?;
        #[cfg(feature = "functions")]
        for value in user_functions.transaction_state_values()? {
            journal.capture_value(&value)?;
        }

        #[cfg(feature = "functions")]
        let plan = state.plan.clone();
        #[cfg(feature = "functions")]
        checkpoint_preflight_plan_capture(&plan, interpreter_id)?;
        #[cfg(feature = "functions")]
        for value in plan.transaction_state_values()? {
            journal.capture_value(&value)?;
        }
        #[cfg(feature = "functions")]
        let plan_checkpoint = plan.checkpoint();

        #[cfg(feature = "enum")]
        let enums = state.enums.clone();
        #[cfg(feature = "enum")]
        let mut enum_dictionaries = Vec::new();
        #[cfg(feature = "enum")]
        for enum_value in enums.values() {
            enum_dictionaries.push(RefPayloadCheckpoint::capture(
                &enum_value.names,
                interpreter_id,
            )?);
            for (_, payload) in &enum_value.variants {
                if let Some(payload) = payload {
                    journal.capture_value(payload)?;
                }
            }
        }

        #[cfg(feature = "invariant_define")]
        for constraint in state.integrity_constraints.values() {
            journal.capture_val_ref(&constraint.result)?;
            if let Some(lhs) = &constraint.lhs {
                journal.capture_val_ref(lhs)?;
            }
            if let Some(rhs) = &constraint.rhs {
                journal.capture_val_ref(rhs)?;
            }
        }

        Ok(Self {
            target: target.clone(),
            #[cfg(feature = "symbol_table")]
            symbol_table,
            #[cfg(feature = "symbol_table")]
            environment,
            #[cfg(feature = "functions")]
            function_environment,
            #[cfg(feature = "functions")]
            function_extensions,
            #[cfg(feature = "functions")]
            user_functions,
            #[cfg(feature = "functions")]
            plan,
            #[cfg(feature = "functions")]
            plan_checkpoint,
            kinds: state.kinds.clone(),
            #[cfg(feature = "enum")]
            enums,
            #[cfg(feature = "enum")]
            enum_dictionaries,
            #[cfg(feature = "invariant_define")]
            integrity_constraints: state.integrity_constraints.clone(),
            dictionary: RefPayloadCheckpoint::capture(&state.dictionary, interpreter_id)?,
        })
    }

    fn preflight(&self, interpreter_id: u64) -> MResult<()> {
        checkpoint_preflight_ref(&self.target, interpreter_id)?;
        #[cfg(feature = "symbol_table")]
        self.symbol_table.preflight(interpreter_id)?;
        #[cfg(feature = "symbol_table")]
        if let Some(environment) = &self.environment {
            environment.preflight(interpreter_id)?;
        }
        #[cfg(feature = "functions")]
        self.user_functions.preflight_restore()?;
        #[cfg(feature = "functions")]
        self.plan.preflight_rollback(&self.plan_checkpoint)?;
        #[cfg(feature = "enum")]
        for dictionary in &self.enum_dictionaries {
            dictionary.preflight(interpreter_id)?;
        }
        self.dictionary.preflight(interpreter_id)
    }

    fn apply_structure(&self) {
        {
            let mut state = self.target.borrow_mut();
            #[cfg(feature = "symbol_table")]
            {
                state.symbol_table = self.symbol_table.target.clone();
                state.environment = self
                    .environment
                    .as_ref()
                    .map(|environment| environment.target.clone());
            }
            #[cfg(feature = "functions")]
            {
                state.function_environment = self.function_environment.clone();
                state.function_extensions = self.function_extensions.clone();
                state.user_functions = self.user_functions.table.clone();
                state.plan = self.plan.clone();
            }
            state.kinds = self.kinds.clone();
            #[cfg(feature = "enum")]
            {
                state.enums = self.enums.clone();
            }
            #[cfg(feature = "invariant_define")]
            {
                state.integrity_constraints = self.integrity_constraints.clone();
            }
            state.dictionary = self.dictionary.target.clone();
        }

        #[cfg(feature = "symbol_table")]
        self.symbol_table.apply();
        #[cfg(feature = "symbol_table")]
        if let Some(environment) = &self.environment {
            environment.apply();
        }
        #[cfg(feature = "functions")]
        self.user_functions.apply_restore_structure();
        #[cfg(feature = "functions")]
        self.plan.apply_rollback_structure(&self.plan_checkpoint);
        #[cfg(feature = "enum")]
        for dictionary in &self.enum_dictionaries {
            dictionary.apply();
        }
        self.dictionary.apply();
    }

    #[cfg(feature = "functions")]
    fn rebuild_checkpoint_indexes(&self, turn_state: &ReactiveTurnState) {
        self.user_functions.rebuild_checkpoint_indexes();
        self.plan.rebuild_checkpoint_indexes();
        self.user_functions
            .validate_checkpoint_invariants()
            .expect("user-function checkpoint preflight guarantees restored plan invariants");
        self.plan
            .validate_checkpoint_invariants()
            .expect("program checkpoint preflight guarantees restored plan invariants");
        self.plan
            .validate_checkpoint_turn_state(turn_state)
            .expect("program checkpoint preflight guarantees restored turn invariants");
    }
}

struct ChildInterpreterCheckpoint {
    target: InterpreterRef,
    checkpoint: Box<InterpreterStructureCheckpoint>,
}

struct InterpreterStructureCheckpoint {
    owner: Rc<()>,
    id: u64,
    profile: bool,
    max_steps: usize,
    ip: usize,
    #[cfg(feature = "trace")]
    trace: bool,
    #[cfg(feature = "trace")]
    trace_to_stdout: bool,
    #[cfg(feature = "trace")]
    trace_events: RefPayloadCheckpoint<Vec<TraceEvent>>,
    state: ProgramStateCheckpoint,
    #[cfg(feature = "functions")]
    reactive_turn_state: ReactiveTurnState,
    #[cfg(feature = "functions")]
    persistent_user_function_plan_depth: RefPayloadCheckpoint<usize>,
    deferred_expression_solve_depth: RefPayloadCheckpoint<usize>,
    #[cfg(feature = "functions")]
    stack: Vec<Frame>,
    #[cfg(feature = "functions")]
    frame_checkpoints: Vec<FrameCellCheckpoint>,
    bytecode_registers: BytecodeRegisterFileCheckpoint,
    constants: Vec<LegacyValue>,
    code: Vec<MechSourceCode>,
    out: LegacyValue,
    out_values: RefPayloadCheckpoint<HashMap<u64, LegacyValue>>,
    #[cfg(feature = "subscript_formula")]
    string_access_live_values: RefPayloadCheckpoint<std::collections::BTreeSet<usize>>,
    #[cfg(feature = "subscript_formula")]
    current_string_access_expression_live: RefPayloadCheckpoint<bool>,
    inline_eval_counter: RefPayloadCheckpoint<u64>,
    context_bindings: RefPayloadCheckpoint<HashMap<u64, RuntimeContextBinding>>,
    module_manifests: RefPayloadCheckpoint<ModuleManifestCatalog>,
    #[cfg(feature = "state_machines")]
    user_state_machines: RefPayloadCheckpoint<HashMap<u64, FsmImplementation>>,
    #[cfg(feature = "state_machines")]
    user_state_machine_specs: RefPayloadCheckpoint<HashMap<u64, FsmSpecification>>,
    sub_interpreters: Ref<HashMap<u64, InterpreterRef>>,
    sub_interpreter_entries: HashMap<u64, InterpreterRef>,
    children: Vec<ChildInterpreterCheckpoint>,
}

impl InterpreterStructureCheckpoint {
    fn capture(
        interpreter: &Interpreter,
        journal: &mut ValueStateJournal,
        seen_children: &mut Vec<InterpreterRef>,
    ) -> MResult<Self> {
        let state = ProgramStateCheckpoint::capture(&interpreter.state, interpreter.id, journal)?;
        #[cfg(feature = "functions")]
        state
            .plan
            .validate_checkpoint_turn_state(&interpreter.reactive_turn_state)?;

        let bytecode_registers =
            BytecodeRegisterFileCheckpoint::capture(&interpreter.bytecode_registers, journal)?;
        for value in &interpreter.constants {
            journal.capture_value(value)?;
        }
        journal.capture_value(&interpreter.out)?;

        let out_values = RefPayloadCheckpoint::capture(&interpreter.out_values, interpreter.id)?;
        for value in out_values.payload.values() {
            journal.capture_value(value)?;
        }

        #[cfg(feature = "functions")]
        let mut frame_checkpoints = Vec::with_capacity(interpreter.stack.len());
        #[cfg(feature = "functions")]
        for frame in &interpreter.stack {
            frame_checkpoints.push(FrameCellCheckpoint::capture(
                frame,
                interpreter.id,
                journal,
            )?);
        }

        let sub_interpreter_entries =
            checkpoint_clone_ref(&interpreter.sub_interpreters, interpreter.id)?;
        let mut children = Vec::with_capacity(sub_interpreter_entries.len());
        for (child_id, child) in &sub_interpreter_entries {
            if seen_children.iter().any(|seen| seen.same_handle(child)) {
                return Err(MechError::new(
                    InterpreterCheckpointHierarchyAlias {
                        interpreter_id: interpreter.id,
                        child_id: *child_id,
                    },
                    None,
                )
                .with_compiler_loc());
            }
            seen_children.push(child.clone());
            let child_borrow = child.try_borrow().map_err(|_| {
                checkpoint_borrow_error("checkpoint-capture", interpreter.id, child)
            })?;
            if child_borrow.id != *child_id {
                return Err(MechError::new(
                    InterpreterCheckpointChildKeyMismatch {
                        parent_id: interpreter.id,
                        key: *child_id,
                        child_id: child_borrow.id,
                    },
                    None,
                )
                .with_compiler_loc());
            }
            let checkpoint = Self::capture(child_borrow.as_ref(), journal, seen_children)?;
            children.push(ChildInterpreterCheckpoint {
                target: child.clone(),
                checkpoint: Box::new(checkpoint),
            });
        }

        Ok(Self {
            owner: interpreter.checkpoint_owner.clone(),
            id: interpreter.id,
            profile: interpreter.profile,
            max_steps: interpreter.max_steps,
            ip: interpreter.ip,
            #[cfg(feature = "trace")]
            trace: interpreter.trace,
            #[cfg(feature = "trace")]
            trace_to_stdout: interpreter.trace_to_stdout,
            #[cfg(feature = "trace")]
            trace_events: RefPayloadCheckpoint::capture(&interpreter.trace_events, interpreter.id)?,
            state,
            #[cfg(feature = "functions")]
            reactive_turn_state: interpreter.reactive_turn_state.clone(),
            #[cfg(feature = "functions")]
            persistent_user_function_plan_depth: RefPayloadCheckpoint::capture(
                &interpreter.persistent_user_function_plan_depth,
                interpreter.id,
            )?,
            deferred_expression_solve_depth: RefPayloadCheckpoint::capture(
                &interpreter.deferred_expression_solve_depth,
                interpreter.id,
            )?,
            #[cfg(feature = "functions")]
            stack: interpreter.stack.clone(),
            #[cfg(feature = "functions")]
            frame_checkpoints,
            bytecode_registers,
            constants: interpreter.constants.clone(),
            code: interpreter.code.clone(),
            out: interpreter.out.clone(),
            out_values,
            #[cfg(feature = "subscript_formula")]
            string_access_live_values: RefPayloadCheckpoint::capture(
                &interpreter.string_access_live_values,
                interpreter.id,
            )?,
            #[cfg(feature = "subscript_formula")]
            current_string_access_expression_live: RefPayloadCheckpoint::capture(
                &interpreter.current_string_access_expression_live,
                interpreter.id,
            )?,
            inline_eval_counter: RefPayloadCheckpoint::capture(
                &interpreter.inline_eval_counter,
                interpreter.id,
            )?,
            context_bindings: RefPayloadCheckpoint::capture(
                &interpreter.context_bindings,
                interpreter.id,
            )?,
            module_manifests: RefPayloadCheckpoint::capture(
                &interpreter.module_manifests,
                interpreter.id,
            )?,
            #[cfg(feature = "state_machines")]
            user_state_machines: RefPayloadCheckpoint::capture(
                &interpreter.user_state_machines,
                interpreter.id,
            )?,
            #[cfg(feature = "state_machines")]
            user_state_machine_specs: RefPayloadCheckpoint::capture(
                &interpreter.user_state_machine_specs,
                interpreter.id,
            )?,
            sub_interpreters: interpreter.sub_interpreters.clone(),
            sub_interpreter_entries,
            children,
        })
    }

    fn preflight(&self) -> MResult<()> {
        self.state.preflight(self.id)?;
        #[cfg(feature = "trace")]
        self.trace_events.preflight(self.id)?;
        #[cfg(feature = "functions")]
        self.persistent_user_function_plan_depth
            .preflight(self.id)?;
        self.deferred_expression_solve_depth.preflight(self.id)?;
        self.out_values.preflight(self.id)?;
        #[cfg(feature = "subscript_formula")]
        self.string_access_live_values.preflight(self.id)?;
        #[cfg(feature = "subscript_formula")]
        self.current_string_access_expression_live
            .preflight(self.id)?;
        self.inline_eval_counter.preflight(self.id)?;
        self.context_bindings.preflight(self.id)?;
        self.module_manifests.preflight(self.id)?;
        #[cfg(feature = "state_machines")]
        self.user_state_machines.preflight(self.id)?;
        #[cfg(feature = "state_machines")]
        self.user_state_machine_specs.preflight(self.id)?;
        checkpoint_preflight_ref(&self.sub_interpreters, self.id)?;
        #[cfg(feature = "functions")]
        for frame in &self.frame_checkpoints {
            frame.preflight(self.id)?;
        }
        for child in &self.children {
            checkpoint_preflight_ref(&child.target, self.id)?;
            let child_borrow = child.target.try_borrow().map_err(|_| {
                checkpoint_borrow_error("checkpoint-restore", self.id, &child.target)
            })?;
            child.checkpoint.preflight_owner(child_borrow.as_ref())?;
            child.checkpoint.preflight()?;
        }
        Ok(())
    }

    fn preflight_owner(&self, interpreter: &Interpreter) -> MResult<()> {
        if Rc::ptr_eq(&self.owner, &interpreter.checkpoint_owner) {
            Ok(())
        } else {
            Err(MechError::new(
                InterpreterCheckpointOwnerMismatch {
                    interpreter_id: interpreter.id,
                },
                None,
            )
            .with_compiler_loc())
        }
    }

    fn apply_structure(&self, interpreter: &mut Interpreter) {
        interpreter.id = self.id;
        interpreter.profile = self.profile;
        interpreter.max_steps = self.max_steps;
        interpreter.ip = self.ip;
        #[cfg(feature = "trace")]
        {
            interpreter.trace = self.trace;
            interpreter.trace_to_stdout = self.trace_to_stdout;
            interpreter.trace_events = self.trace_events.target.clone();
        }
        interpreter.state = self.state.target.clone();
        #[cfg(feature = "functions")]
        {
            interpreter.reactive_turn_state = self.reactive_turn_state.clone();
            interpreter.persistent_user_function_plan_depth =
                self.persistent_user_function_plan_depth.target.clone();
            interpreter.stack = self.stack.clone();
        }
        interpreter.deferred_expression_solve_depth =
            self.deferred_expression_solve_depth.target.clone();
        interpreter.bytecode_registers = self.bytecode_registers.restore();
        interpreter.constants = self.constants.clone();
        interpreter.code = self.code.clone();
        interpreter.out = self.out.clone();
        interpreter.out_values = self.out_values.target.clone();
        #[cfg(feature = "subscript_formula")]
        {
            interpreter.string_access_live_values = self.string_access_live_values.target.clone();
            interpreter.current_string_access_expression_live =
                self.current_string_access_expression_live.target.clone();
        }
        interpreter.inline_eval_counter = self.inline_eval_counter.target.clone();
        interpreter.context_bindings = self.context_bindings.target.clone();
        interpreter.module_manifests = self.module_manifests.target.clone();
        #[cfg(feature = "state_machines")]
        {
            interpreter.user_state_machines = self.user_state_machines.target.clone();
            interpreter.user_state_machine_specs = self.user_state_machine_specs.target.clone();
        }
        interpreter.sub_interpreters = self.sub_interpreters.clone();

        self.state.apply_structure();
        #[cfg(feature = "trace")]
        self.trace_events.apply();
        #[cfg(feature = "functions")]
        self.persistent_user_function_plan_depth.apply();
        self.deferred_expression_solve_depth.apply();
        self.out_values.apply();
        #[cfg(feature = "subscript_formula")]
        self.string_access_live_values.apply();
        #[cfg(feature = "subscript_formula")]
        self.current_string_access_expression_live.apply();
        self.inline_eval_counter.apply();
        self.context_bindings.apply();
        self.module_manifests.apply();
        #[cfg(feature = "state_machines")]
        self.user_state_machines.apply();
        #[cfg(feature = "state_machines")]
        self.user_state_machine_specs.apply();
        #[cfg(feature = "functions")]
        for frame in &self.frame_checkpoints {
            frame.apply_structure();
        }

        *self.sub_interpreters.borrow_mut() = self.sub_interpreter_entries.clone();
        for child in &self.children {
            let mut child_borrow = child.target.borrow_mut();
            child.checkpoint.apply_structure(child_borrow.as_mut());
        }
    }

    fn rebuild_checkpoint_indexes(&self) {
        #[cfg(feature = "functions")]
        {
            self.state
                .rebuild_checkpoint_indexes(&self.reactive_turn_state);
            for frame in &self.frame_checkpoints {
                frame.rebuild_checkpoint_indexes();
            }
        }
        for child in &self.children {
            child.checkpoint.rebuild_checkpoint_indexes();
        }
    }
}

struct InterpreterCheckpointData {
    structure: InterpreterStructureCheckpoint,
    journal: ValueStateJournal,
}

/// A process-local explicit savepoint for retained interpreter state.
///
/// This type is intentionally opaque and is not a serializable history record
/// or a source of durable transaction identities.
#[derive(Clone)]
pub struct InterpreterCheckpoint {
    data: Rc<InterpreterCheckpointData>,
}

/// An opaque checkpoint of scheduler metadata for one reactive operation.
///
/// Function values are intentionally excluded and are captured lazily through
/// a reactive journal participant only when their functions execute.
#[cfg(feature = "functions")]
pub struct InterpreterReactiveTurnCheckpoint {
    owner: Rc<()>,
    interpreter_id: u64,
    plan: Plan,
    plan_node_len: usize,
    activation_registration_depth: usize,
    reactive_turn_state: ReactiveTurnState,
    #[cfg(feature = "trace")]
    trace_events: Ref<Vec<TraceEvent>>,
    #[cfg(feature = "trace")]
    trace_event_len: usize,
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnOwnerMismatch {
    pub interpreter_id: u64,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnOwnerMismatch {
    fn name(&self) -> &str {
        "InterpreterReactiveTurnOwnerMismatch"
    }

    fn message(&self) -> String {
        format!(
            "The reactive-turn checkpoint belongs to a different interpreter than interpreter {}.",
            self.interpreter_id,
        )
    }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnInvariant {
    pub interpreter_id: u64,
    pub reason: String,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnInvariant {
    fn name(&self) -> &str {
        "InterpreterReactiveTurnInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Interpreter {} cannot restore its reactive-turn scheduler state: {}.",
            self.interpreter_id, self.reason,
        )
    }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnBorrowConflict {
    pub phase: &'static str,
    pub interpreter_id: u64,
    pub component: &'static str,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnBorrowConflict {
    fn name(&self) -> &str {
        "InterpreterReactiveTurnBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Cannot borrow {} for interpreter {} during reactive-turn {}.",
            self.component, self.interpreter_id, self.phase,
        )
    }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnRollbackFailed {
    pub interpreter_id: u64,
    pub original_error: String,
    pub rollback_error: String,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnRollbackFailed {
    fn name(&self) -> &str {
        "InterpreterReactiveTurnRollbackFailed"
    }

    fn message(&self) -> String {
        format!(
            "Interpreter {} reactive-turn rollback failed after {}: {}.",
            self.interpreter_id, self.original_error, self.rollback_error,
        )
    }
}

impl Clone for Interpreter {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            checkpoint_owner: Rc::new(()),
            #[cfg(feature = "functions")]
            function_catalog: Arc::clone(&self.function_catalog),
            ip: self.ip,
            profile: false,
            max_steps: self.max_steps,
            #[cfg(feature = "trace")]
            trace: self.trace,
            #[cfg(feature = "trace")]
            trace_to_stdout: self.trace_to_stdout,
            #[cfg(feature = "trace")]
            trace_events: self.trace_events.clone(),
            state: Ref::new(self.state.borrow().clone()),
            #[cfg(feature = "functions")]
            reactive_turn_state: self.reactive_turn_state.clone(),
            #[cfg(feature = "functions")]
            persistent_user_function_plan_depth: self.persistent_user_function_plan_depth.clone(),
            deferred_expression_solve_depth: self.deferred_expression_solve_depth.clone(),
            #[cfg(feature = "functions")]
            stack: self.stack.clone(),
            bytecode_registers: self.bytecode_registers.clone(),
            constants: self.constants.clone(),
            code: self.code.clone(),
            out: self.out.clone(),
            out_values: self.out_values.clone(),
            #[cfg(feature = "subscript_formula")]
            string_access_live_values: Ref::new(self.string_access_live_values.borrow().clone()),
            #[cfg(feature = "subscript_formula")]
            current_string_access_expression_live: Ref::new(
                *self.current_string_access_expression_live.borrow(),
            ),
            inline_eval_counter: self.inline_eval_counter.clone(),
            context_bindings: self.context_bindings.clone(),
            module_manifests: self.module_manifests.clone(),
            #[cfg(feature = "state_machines")]
            user_state_machines: self.user_state_machines.clone(),
            #[cfg(feature = "state_machines")]
            user_state_machine_specs: self.user_state_machine_specs.clone(),
            sub_interpreters: self.sub_interpreters.clone(),
        }
    }
}

impl Interpreter {
    #[cfg(feature = "functions")]
    fn reactive_turn_invariant(&self, reason: impl Into<String>) -> MechError {
        MechError::new(
            InterpreterReactiveTurnInvariant {
                interpreter_id: self.id,
                reason: reason.into(),
            },
            None,
        )
        .with_compiler_loc()
    }

    #[cfg(feature = "functions")]
    fn reactive_turn_borrow_conflict(
        &self,
        phase: &'static str,
        component: &'static str,
    ) -> MechError {
        MechError::new(
            InterpreterReactiveTurnBorrowConflict {
                phase,
                interpreter_id: self.id,
                component,
            },
            None,
        )
        .with_compiler_loc()
    }

    #[cfg(feature = "functions")]
    pub fn reactive_turn_checkpoint(&self) -> MResult<InterpreterReactiveTurnCheckpoint> {
        let plan = self.plan();
        plan.validate_checkpoint_invariants()
            .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
        plan.validate_checkpoint_turn_state(&self.reactive_turn_state)
            .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
        #[cfg(feature = "trace")]
        let trace_event_len = self
            .trace_events
            .try_borrow()
            .map_err(|_| self.reactive_turn_borrow_conflict("checkpoint", "trace events"))?
            .len();
        Ok(InterpreterReactiveTurnCheckpoint {
            owner: self.checkpoint_owner.clone(),
            interpreter_id: self.id,
            plan_node_len: plan.len(),
            activation_registration_depth: plan.activation_registration_depth(),
            plan,
            reactive_turn_state: self.reactive_turn_state.clone(),
            #[cfg(feature = "trace")]
            trace_events: self.trace_events.clone(),
            #[cfg(feature = "trace")]
            trace_event_len,
        })
    }

    #[cfg(feature = "functions")]
    pub fn preflight_restore_reactive_turn(
        &self,
        checkpoint: &InterpreterReactiveTurnCheckpoint,
    ) -> MResult<()> {
        if !Rc::ptr_eq(&checkpoint.owner, &self.checkpoint_owner) {
            return Err(MechError::new(
                InterpreterReactiveTurnOwnerMismatch {
                    interpreter_id: self.id,
                },
                None,
            )
            .with_compiler_loc());
        }
        if checkpoint.interpreter_id != self.id {
            return Err(self.reactive_turn_invariant(format!(
                "checkpoint interpreter ID {} does not match {}",
                checkpoint.interpreter_id, self.id,
            )));
        }
        let plan = self.plan();
        if plan.0.addr() != checkpoint.plan.0.addr() || plan.1.addr() != checkpoint.plan.1.addr() {
            return Err(self
                .reactive_turn_invariant("the current plan handles do not match the checkpoint"));
        }
        if plan.len() != checkpoint.plan_node_len {
            return Err(self.reactive_turn_invariant(format!(
                "plan length {} does not match checkpoint length {}",
                plan.len(),
                checkpoint.plan_node_len,
            )));
        }
        if plan.activation_registration_depth() != checkpoint.activation_registration_depth {
            return Err(self.reactive_turn_invariant(format!(
                "activation registration depth {} does not match checkpoint depth {}",
                plan.activation_registration_depth(),
                checkpoint.activation_registration_depth,
            )));
        }
        plan.validate_checkpoint_invariants()
            .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
        plan.validate_checkpoint_turn_state(&checkpoint.reactive_turn_state)
            .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
        #[cfg(feature = "trace")]
        {
            if self.trace_events.addr() != checkpoint.trace_events.addr() {
                return Err(self.reactive_turn_invariant(
                    "the current trace target does not match the checkpoint",
                ));
            }
            let events = self
                .trace_events
                .try_borrow_mut()
                .map_err(|_| self.reactive_turn_borrow_conflict("restore", "trace events"))?;
            if events.len() < checkpoint.trace_event_len {
                return Err(self.reactive_turn_invariant(format!(
                    "trace length {} is shorter than checkpoint length {}",
                    events.len(),
                    checkpoint.trace_event_len,
                )));
            }
        }
        Ok(())
    }

    #[cfg(feature = "functions")]
    pub fn apply_restore_reactive_turn(&mut self, checkpoint: &InterpreterReactiveTurnCheckpoint) {
        self.reactive_turn_state = checkpoint.reactive_turn_state.clone();
        #[cfg(feature = "trace")]
        self.trace_events
            .borrow_mut()
            .truncate(checkpoint.trace_event_len);
    }

    #[cfg(feature = "functions")]
    fn finish_failed_reactive_operation<T>(
        &mut self,
        checkpoint: &InterpreterReactiveTurnCheckpoint,
        participant: ReactiveJournalParticipant<'_>,
        original_error: MechError,
    ) -> MResult<T> {
        let rollback_result = self
            .preflight_restore_reactive_turn(checkpoint)
            .and_then(|_| participant.preflight_restore_before());
        if let Err(rollback_error) = rollback_result {
            return Err(MechError::new(
                InterpreterReactiveTurnRollbackFailed {
                    interpreter_id: self.id,
                    original_error: format!("{:?}", original_error),
                    rollback_error: format!("{:?}", rollback_error),
                },
                None,
            )
            .with_compiler_loc());
        }
        participant.apply_restore_before();
        self.apply_restore_reactive_turn(checkpoint);
        Err(original_error)
    }

    pub fn checkpoint(&self) -> MResult<InterpreterCheckpoint> {
        let mut journal = ValueStateJournal::new();
        let mut seen_children = Vec::new();
        let structure =
            InterpreterStructureCheckpoint::capture(self, &mut journal, &mut seen_children)?;
        Ok(InterpreterCheckpoint {
            data: Rc::new(InterpreterCheckpointData { structure, journal }),
        })
    }

    pub fn restore(&mut self, checkpoint: InterpreterCheckpoint) -> MResult<()> {
        checkpoint.data.structure.preflight_owner(self)?;
        checkpoint.data.structure.preflight()?;
        checkpoint.data.journal.preflight_restore_before()?;
        checkpoint.data.structure.apply_structure(self);
        checkpoint.data.journal.apply_restore_before();
        checkpoint.data.structure.rebuild_checkpoint_indexes();
        Ok(())
    }

    pub fn new(id: u64, max_steps: usize) -> Self {
        #[cfg(feature = "functions")]
        {
            Self::with_function_catalog(id, max_steps, empty_function_catalog())
        }
        #[cfg(not(feature = "functions"))]
        {
            Self::initialize(id, max_steps, ProgramState::new())
        }
    }

    #[cfg(feature = "functions")]
    pub fn with_function_catalog(
        id: u64,
        max_steps: usize,
        function_catalog: Arc<FunctionCatalog>,
    ) -> Self {
        let mut state = ProgramState::new();
        state.function_environment = FunctionEnvironment::from_catalog_defaults(&function_catalog)
            .expect("validated function catalog defaults must initialize");
        Self::initialize(id, max_steps, state, function_catalog)
    }

    fn initialize(
        id: u64,
        max_steps: usize,
        mut state: ProgramState,
        #[cfg(feature = "functions")] function_catalog: Arc<FunctionCatalog>,
    ) -> Self {
        load_stdkinds(&mut state.kinds);
        #[cfg(feature = "symbol_table")]
        {
            let ans_id = hash_str("ans");
            state
                .symbol_table
                .borrow_mut()
                .insert(ans_id, LegacyValue::Empty, false);
            state
                .symbol_table
                .borrow_mut()
                .dictionary
                .borrow_mut()
                .insert(ans_id, "ans".to_string());
            state
                .dictionary
                .borrow_mut()
                .insert(ans_id, "ans".to_string());
        }
        Self {
            id,
            checkpoint_owner: Rc::new(()),
            #[cfg(feature = "functions")]
            function_catalog,
            ip: 0,
            profile: false,
            max_steps, // Default maximum steps
            #[cfg(feature = "trace")]
            trace: false,
            #[cfg(feature = "trace")]
            trace_to_stdout: true,
            #[cfg(feature = "trace")]
            trace_events: Ref::new(Vec::new()),
            state: Ref::new(state),
            #[cfg(feature = "functions")]
            reactive_turn_state: ReactiveTurnState::default(),
            #[cfg(feature = "functions")]
            persistent_user_function_plan_depth: Ref::new(0),
            deferred_expression_solve_depth: Ref::new(0),
            #[cfg(feature = "functions")]
            stack: Vec::new(),
            bytecode_registers: BytecodeRegisterFile::new(0),
            constants: Vec::new(),
            out: LegacyValue::Empty,
            sub_interpreters: Ref::new(HashMap::new()),
            out_values: Ref::new(HashMap::new()),
            #[cfg(feature = "subscript_formula")]
            string_access_live_values: Ref::new(std::collections::BTreeSet::new()),
            #[cfg(feature = "subscript_formula")]
            current_string_access_expression_live: Ref::new(false),
            inline_eval_counter: Ref::new(0),
            context_bindings: Ref::new(HashMap::new()),
            module_manifests: Ref::new(ModuleManifestCatalog::with_builtin_hosts()),
            #[cfg(feature = "state_machines")]
            user_state_machines: Ref::new(HashMap::new()),
            #[cfg(feature = "state_machines")]
            user_state_machine_specs: Ref::new(HashMap::new()),
            code: Vec::new(),
        }
    }

    #[cfg(feature = "functions")]
    pub fn function_catalog(&self) -> &Arc<FunctionCatalog> {
        &self.function_catalog
    }

    #[cfg(feature = "functions")]
    pub(crate) fn new_child_interpreter(&self, id: u64, max_steps: usize) -> Self {
        let child = Self::with_function_catalog(id, max_steps, Arc::clone(&self.function_catalog));
        {
            let parent_state = self.state.borrow();
            let mut child_state = child.state.borrow_mut();
            child_state.function_environment = parent_state.function_environment.clone();
            child_state.function_extensions = parent_state.function_extensions.clone();
            child_state.user_functions = parent_state.user_functions.clone();
        }
        child
    }

    pub fn default() -> Self {
        Self::new(0, 10_000)
    }

    pub fn bind_context(&self, name: &Identifier, base_uri: impl Into<String>) {
        let base_uri = base_uri.into();
        let context_name = canonical_context_name_from_base_uri(&base_uri);
        self.bind_context_with_name(name, context_name, base_uri);
    }

    pub(crate) fn bind_context_with_name(
        &self,
        alias: &Identifier,
        context_name: impl Into<String>,
        base_uri: impl Into<String>,
    ) {
        self.context_bindings.borrow_mut().insert(
            alias.hash(),
            RuntimeContextBinding {
                context_name: context_name.into(),
                base_uri: base_uri.into(),
            },
        );
    }

    pub fn context_binding(&self, name: &Identifier) -> Option<RuntimeContextBinding> {
        self.context_bindings.borrow().get(&name.hash()).cloned()
    }

    pub fn bind_context_export(&self, alias: &Identifier, module: &str, item: &str) -> MResult<()> {
        let (context_name, base_uri) = {
            let manifests = self.module_manifests.borrow();
            let export = manifests.context_export(module, item)?;
            (export.name.clone(), export.base_uri.clone())
        };
        self.bind_context_with_name(alias, context_name, base_uri);
        Ok(())
    }

    #[cfg(feature = "symbol_table")]
    pub fn set_environment(&mut self, env: SymbolTableRef) {
        self.state.borrow_mut().environment = Some(env);
    }

    #[cfg(feature = "functions")]
    pub fn clear_plan(&mut self) {
        self.state.borrow_mut().plan.borrow_mut().clear();
        self.reactive_turn_state = ReactiveTurnState::default();
    }

    #[cfg(feature = "pretty_print")]
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Interpreter ID: {}\n", self.id));
        // print state
        output.push_str(&self.state.borrow().pretty_print());

        output.push_str("Registers:\n");
        for register in 0..self.bytecode_registers.len() {
            let value = self
                .bytecode_registers
                .value(register as u32)
                .expect("pretty-print register index is in range");
            output.push_str(&format!("  R{}: {}\n", register, value));
        }
        output.push_str("Constants:\n");
        for (i, constant) in self.constants.iter().enumerate() {
            output.push_str(&format!("  C{}: {}\n", i, constant));
        }
        output.push_str(&format!("Output Value: {}\n", self.out));
        output.push_str(&format!(
            "Number of Sub-Interpreters: {}\n",
            self.sub_interpreters.borrow().len()
        ));
        output.push_str("Output Values:\n");
        for (key, value) in self.out_values.borrow().iter() {
            output.push_str(&format!("  {}: {}\n", key, value));
        }
        output.push_str(&format!("Code Length: {}\n", self.code.len()));
        output
    }

    pub fn clear(&mut self) {
        let id = self.id;
        let checkpoint_owner = self.checkpoint_owner.clone();
        #[cfg(feature = "functions")]
        {
            let function_catalog = Arc::clone(&self.function_catalog);
            *self = Interpreter::with_function_catalog(id, self.max_steps, function_catalog);
        }
        #[cfg(not(feature = "functions"))]
        {
            *self = Interpreter::new(id, self.max_steps);
        }
        self.checkpoint_owner = checkpoint_owner;
    }

    pub fn set_trace_enabled(&mut self, enabled: bool) {
        #[cfg(feature = "trace")]
        {
            self.trace = enabled;
        }
        #[cfg(not(feature = "trace"))]
        {
            let _ = enabled;
        }
    }

    #[cfg(feature = "trace")]
    pub fn set_trace_to_stdout(&mut self, enabled: bool) {
        self.trace_to_stdout = enabled;
    }

    #[cfg(feature = "trace")]
    pub fn clear_trace_events(&self) {
        self.trace_events.borrow_mut().clear();
    }

    #[cfg(feature = "trace")]
    pub fn trace_events(&self) -> Vec<TraceEvent> {
        self.trace_events.borrow().clone()
    }

    #[cfg(feature = "trace")]
    pub fn trace_events_to_json(&self) -> String {
        let trace_events = self.trace_events.borrow();
        trace_events_to_json(trace_events.as_slice())
    }

    #[cfg(feature = "trace")]
    pub fn push_trace_line(&self, rendered: String) {
        let (channel, label, message) = parse_trace_line(&rendered);
        let mut trace_events = self.trace_events.borrow_mut();
        let index = trace_events.len();
        trace_events.push(TraceEvent {
            index,
            channel,
            label,
            message,
            rendered,
        });
    }

    #[cfg(all(feature = "trace", feature = "state_machines"))]
    pub fn formatted_fsm_trace(&self) -> String {
        format_fsm_trace_report(&self.trace_events())
    }

    #[cfg(feature = "pretty_print")]
    pub fn pretty_print_symbols(&self) -> String {
        let state_brrw = self.state.borrow();
        let syms = state_brrw.symbol_table.borrow();
        syms.pretty_print()
    }

    #[cfg(feature = "pretty_print")]
    pub fn pretty_print_plan(&self) -> String {
        let state_brrw = self.state.borrow();
        let plan = state_brrw.plan.borrow();
        let mut result = String::new();
        for (i, step) in plan.iter().enumerate() {
            result.push_str(&format!("Step {}:\n", i));
            result.push_str(&format!("{}\n", step.to_string()));
        }
        result
    }

    #[cfg(feature = "functions")]
    pub fn plan(&self) -> Plan {
        self.state.borrow().plan.clone()
    }

    #[cfg(feature = "functions")]
    pub fn advance_reactive_turn(
        &mut self,
        dirty_cells: &[ReactiveCellId],
    ) -> MResult<ReactiveTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.advance_reactive_turn_with_services(dirty_cells, &mut services)
    }

    #[cfg(feature = "functions")]
    pub fn advance_reactive_turn_with_services(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        let checkpoint = self.reactive_turn_checkpoint()?;
        with_reactive_journal_participant(|mut participant| {
            let execution =
                self.advance_reactive_turn_participating(dirty_cells, &mut participant, services);
            match execution {
                Ok(outcome) => {
                    participant.commit();
                    Ok(outcome)
                }
                Err(error) => {
                    self.finish_failed_reactive_operation(&checkpoint, participant, error)
                }
            }
        })
    }

    /// Participates in one coordinator-backed reactive operation.
    ///
    /// The capability can only be received by value inside the auto-finalizing
    /// journal callback. Callers cannot construct, escape, clone, or reuse it
    /// after its owner commits or rolls back.
    #[cfg(feature = "functions")]
    pub fn advance_reactive_turn_participating(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        participant: &mut ReactiveJournalParticipant<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        let plan = self.plan();
        plan.advance_reactive_turn_participating(
            &mut self.reactive_turn_state,
            dirty_cells,
            participant,
            services,
        )
    }

    #[cfg(feature = "functions")]
    pub fn has_pending_reactive_registers(&self) -> bool {
        self.reactive_turn_state.has_pending_registers()
    }

    #[cfg(feature = "symbol_table")]
    pub fn symbols(&self) -> SymbolTableRef {
        self.state.borrow().symbol_table.clone()
    }

    pub fn dictionary(&self) -> Ref<Dictionary> {
        self.state.borrow().dictionary.clone()
    }

    #[cfg(feature = "functions")]
    pub fn plan_len(&self) -> usize {
        self.state.borrow().plan.len()
    }

    #[cfg(feature = "functions")]
    pub fn solve_plan(&mut self) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.solve_plan_with_services(&mut services)
    }

    #[cfg(feature = "functions")]
    pub fn solve_plan_with_services(
        &mut self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        self.step_with_services(0, 1, services)
    }

    #[cfg(feature = "functions")]
    pub fn step(&mut self, step_id: usize, step_count: u64) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.step_with_services(step_id, step_count, &mut services)
    }

    #[cfg(feature = "functions")]
    pub fn step_with_services(
        &mut self,
        step_id: usize,
        step_count: u64,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        let checkpoint = self.reactive_turn_checkpoint()?;
        with_reactive_journal_participant(|mut participant| {
            let execution = self.step_reactive_turn_participating(
                step_id,
                step_count,
                &mut participant,
                services,
            );
            match execution {
                Ok(value) => {
                    participant.commit();
                    Ok(value)
                }
                Err(error) => {
                    self.finish_failed_reactive_operation(&checkpoint, participant, error)
                }
            }
        })
    }

    /// Participates in one coordinator-backed stepping operation.
    ///
    /// The capability can only be received by value inside the auto-finalizing
    /// journal callback. Callers cannot construct, escape, clone, or reuse it
    /// after its owner commits or rolls back.
    #[cfg(feature = "functions")]
    pub fn step_reactive_turn_participating(
        &mut self,
        step_id: usize,
        step_count: u64,
        participant: &mut ReactiveJournalParticipant<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        let state_brrw = self.state.borrow();
        let mut plan_brrw = state_brrw
            .plan
            .0
            .try_borrow_mut()
            .map_err(|_| self.reactive_turn_borrow_conflict("execute", "plan"))?; // RefMut<Vec<Box<dyn MechFunction>>>

        if plan_brrw.is_empty() {
            return Err(MechError::new(NoStepsInPlanError, None).with_compiler_loc());
        }

        let len = plan_brrw.len();

        // Case 1: step_id == 0, run entire plan step_count times
        if step_id == 0 {
            if self.profile {
                // Initialize total durations per step
                let mut total_durations = vec![Duration::ZERO; len];
                for _ in 0..step_count {
                    for (idx, fxn) in plan_brrw.iter_mut().enumerate() {
                        let start = Instant::now();
                        participant.capture_function_state(fxn.as_ref())?;
                        fxn.solve_result_with(services)?;
                        total_durations[idx] += start.elapsed();
                    }
                }
                // Print histogram if profiling is enabled
                if self.profile {
                    println!("\nStep timing summary and histogram:");
                    print_histogram(&total_durations);
                }
                return Ok(plan_brrw[len - 1].out().clone());
            } else {
                for _ in 0..step_count {
                    for (idx, fxn) in plan_brrw.iter_mut().enumerate() {
                        trace_println!(self, "{}", {
                            let fxn_header = fxn
                                .to_string()
                                .lines()
                                .next()
                                .unwrap_or("<unknown-step>")
                                .to_string();
                            format!("[trace][plan] step[{idx}] {fxn_header}")
                        });
                        participant.capture_function_state(fxn.as_ref())?;
                        fxn.solve_result_with(services)?;
                        trace_println!(self, "{}", {
                            let output = fxn.out().to_string();
                            let output = if output.chars().count() > 96 {
                                format!("{}…", output.chars().take(96).collect::<String>())
                            } else {
                                output
                            };
                            format!("[trace][plan] step[{idx}] out={output}")
                        });
                    }
                }
                return Ok(plan_brrw[len - 1].out().clone());
            }
        }

        // Case 2: step a single function by index
        let idx = step_id as usize;
        if idx > len {
            return Err(MechError::new(
                StepIndexOutOfBoundsError {
                    step_id,
                    plan_length: len,
                },
                None,
            )
            .with_compiler_loc());
        }

        let fxn = &mut plan_brrw[idx - 1];

        let fxn_str = fxn.to_string();
        if fxn_str.lines().count() > 30 {
            let lines: Vec<&str> = fxn_str.lines().collect();
            println!("Stepping function:");
            for line in &lines[0..10] {
                println!("{}", line);
            }
            println!("…");
            for line in &lines[lines.len() - 10..] {
                println!("{}", line);
            }
        } else {
            println!("Stepping function:\n{}", fxn_str);
        }

        for _ in 0..step_count {
            participant.capture_function_state(fxn.as_ref())?;
            fxn.solve_result_with(services)?;
        }

        Ok(fxn.out().clone())
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn interpret(&mut self, tree: &Program) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.interpret_with_services(tree, &mut services)
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn interpret_with_services(
        &mut self,
        tree: &Program,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        self.code.push(MechSourceCode::Tree(tree.clone()));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let execution = InterpreterExecution::new(self, services);
            program(tree, &execution)
        }))
        .map_err(|err| match err.downcast_ref::<&'static str>() {
            Some(raw_msg) => {
                if raw_msg.contains("Index out of bounds") {
                    MechError::new(IndexOutOfBoundsError, None).with_compiler_loc()
                } else if raw_msg.contains("attempt to subtract with overflow") {
                    MechError::new(OverflowSubtractionError, None).with_compiler_loc()
                } else {
                    MechError::new(
                        UnknownPanicError {
                            details: raw_msg.to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                }
            }
            None => MechError::new(
                UnknownPanicError {
                    details: "Non-string panic".to_string(),
                },
                None,
            )
            .with_compiler_loc(),
        })??;
        // The interpreter output is the final source value, which may differ
        // from the last planned node (for example, `x := 1 + 2; 42`).
        self.out = result.clone();
        Ok(result)
    }

    #[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
    pub fn run_program(&mut self, program: &ParsedProgram) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_program_with_services(program, &mut services)
    }

    #[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
    pub fn run_program_with_services(
        &mut self,
        program: &ParsedProgram,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        let resource_read_evidence = {
            let mut resolver = ExecutionServicesBytecodeContractResolver::new(services);
            program.validate_runtime_contracts_with(&self.function_catalog, &mut resolver)?;
            resolver.into_resource_reads()
        };
        validate_bytecode_resource_read_evidence(program, &resource_read_evidence)?;
        let checkpoint = self.checkpoint()?;
        match self.run_validated_program_with_services(program, services, &resource_read_evidence) {
            Ok(value) => Ok(value),
            Err(original_error) => match self.restore(checkpoint) {
                Ok(()) => Err(original_error),
                Err(rollback_error) => Err(MechError::new(
                    InterpreterBytecodeInstallRollbackFailed {
                        interpreter_id: self.id,
                        original_error: original_error.display_message(),
                        rollback_error: rollback_error.display_message(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }

    #[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
    fn run_validated_program_with_services(
        &mut self,
        program: &ParsedProgram,
        services: &mut dyn MechExecutionServices,
        resource_read_evidence: &HashMap<u32, PlannedResourceRead>,
    ) -> MResult<LegacyValue> {
        // Reset the instruction pointer
        self.ip = 0;
        self.clear_plan();
        // Rebuild the register file with one stable outer cell per register.
        self.bytecode_registers = BytecodeRegisterFile::new(program.header.register_count as usize);
        // Load the constants
        self.constants = program.decode_constants()?;
        // Load the instructions
        {
            let function_catalog = Arc::clone(&self.function_catalog);
            let state_brrw = self.state.borrow();
            while self.ip < program.instructions.len() {
                let instr = &program.instructions[self.ip];
                match instr {
                    BytecodeInstruction::ConstLoad { dst, constant } => {
                        // Equal constants are deduplicated in the bytecode table, but
                        // distinct destination registers still represent distinct
                        // runtime cells. Detach each load so equal initial values do
                        // not accidentally alias after reconstruction.
                        let value = self.constants[*constant as usize].try_deep_snapshot()?;
                        self.bytecode_registers.load(*dst, value)?;
                    }
                    BytecodeInstruction::CompositePack {
                        dst,
                        template,
                        children,
                    } => {
                        let template = self.constants[*template as usize].clone();
                        let children = children
                            .iter()
                            .map(|register| self.bytecode_registers.function_argument(*register))
                            .collect::<MResult<Vec<_>>>()?;
                        let value =
                            mech_core::rebuild_bytecode_composite(&template, children.clone())?;
                        let hashed = match &value {
                            #[cfg(feature = "map")]
                            LegacyValue::Map(_) => true,
                            #[cfg(feature = "set")]
                            LegacyValue::Set(_) => true,
                            _ => false,
                        };
                        self.bytecode_registers.load(*dst, value)?;
                        if hashed {
                            let output = self.bytecode_registers.function_argument(*dst)?;
                            register_bytecode_node(
                                &state_brrw,
                                Box::new(BytecodeHashedCompositePack {
                                    template,
                                    children: children.clone(),
                                    output,
                                }),
                                &children,
                            )?;
                        }
                    }
                    BytecodeInstruction::RuntimeNullary { function, dst } => {
                        let runtime_id = RuntimeFunctionId::from_raw(*function);
                        match function_catalog.runtime_entry(runtime_id) {
                            Some(entry) => {
                                let out = self.bytecode_registers.function_argument(*dst)?;
                                let function_args = FunctionArgs::Nullary(out);
                                self.out =
                                    register_bytecode_function(&state_brrw, entry, function_args)?;
                            }
                            None => {
                                return Err(MechError::new(
                                    UnknownNullaryFunctionError { fxn_id: *function },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    BytecodeInstruction::RuntimeUnary { function, dst, src } => {
                        let runtime_id = RuntimeFunctionId::from_raw(*function);
                        match function_catalog.runtime_entry(runtime_id) {
                            Some(entry) => {
                                let out = self.bytecode_registers.function_argument(*dst)?;
                                let input = self.bytecode_registers.function_argument(*src)?;
                                let function_args = FunctionArgs::Unary(out, input);
                                self.out =
                                    register_bytecode_function(&state_brrw, entry, function_args)?;
                            }
                            None => {
                                return Err(MechError::new(
                                    UnknownUnaryFunctionError { fxn_id: *function },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    BytecodeInstruction::RuntimeBinary {
                        function,
                        dst,
                        lhs,
                        rhs,
                    } => {
                        let runtime_id = RuntimeFunctionId::from_raw(*function);
                        match function_catalog.runtime_entry(runtime_id) {
                            Some(entry) => {
                                let out = self.bytecode_registers.function_argument(*dst)?;
                                let lhs = self.bytecode_registers.function_argument(*lhs)?;
                                let rhs = self.bytecode_registers.function_argument(*rhs)?;
                                let function_args = FunctionArgs::Binary(out, lhs, rhs);
                                self.out =
                                    register_bytecode_function(&state_brrw, entry, function_args)?;
                            }
                            None => {
                                return Err(MechError::new(
                                    UnknownBinaryFunctionError { fxn_id: *function },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    BytecodeInstruction::RuntimeTernary {
                        function,
                        dst,
                        a,
                        b,
                        c,
                    } => {
                        let runtime_id = RuntimeFunctionId::from_raw(*function);
                        match function_catalog.runtime_entry(runtime_id) {
                            Some(entry) => {
                                let out = self.bytecode_registers.function_argument(*dst)?;
                                let arg_a = self.bytecode_registers.function_argument(*a)?;
                                let arg_b = self.bytecode_registers.function_argument(*b)?;
                                let arg_c = self.bytecode_registers.function_argument(*c)?;
                                let function_args = FunctionArgs::Ternary(out, arg_a, arg_b, arg_c);
                                self.out =
                                    register_bytecode_function(&state_brrw, entry, function_args)?;
                            }
                            None => {
                                return Err(MechError::new(
                                    UnknownTernaryFunctionError { fxn_id: *function },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    BytecodeInstruction::RuntimeQuaternary {
                        function,
                        dst,
                        a,
                        b,
                        c,
                        d,
                    } => {
                        let runtime_id = RuntimeFunctionId::from_raw(*function);
                        match function_catalog.runtime_entry(runtime_id) {
                            Some(entry) => {
                                let out = self.bytecode_registers.function_argument(*dst)?;
                                let arg_a = self.bytecode_registers.function_argument(*a)?;
                                let arg_b = self.bytecode_registers.function_argument(*b)?;
                                let arg_c = self.bytecode_registers.function_argument(*c)?;
                                let arg_d = self.bytecode_registers.function_argument(*d)?;
                                let function_args =
                                    FunctionArgs::Quaternary(out, arg_a, arg_b, arg_c, arg_d);
                                self.out =
                                    register_bytecode_function(&state_brrw, entry, function_args)?;
                            }
                            None => {
                                return Err(MechError::new(
                                    UnknownQuadFunctionError { fxn_id: *function },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    BytecodeInstruction::RuntimeVariadic {
                        function,
                        dst,
                        arguments,
                    } => {
                        let runtime_id = RuntimeFunctionId::from_raw(*function);
                        match function_catalog.runtime_entry(runtime_id) {
                            Some(entry) => {
                                let out = self.bytecode_registers.function_argument(*dst)?;
                                let argument_values = arguments
                                    .iter()
                                    .map(|register| {
                                        self.bytecode_registers.function_argument(*register)
                                    })
                                    .collect::<MResult<Vec<LegacyValue>>>()?;
                                let function_args = FunctionArgs::Variadic(out, argument_values);
                                self.out =
                                    register_bytecode_function(&state_brrw, entry, function_args)?;
                            }
                            None => {
                                return Err(MechError::new(
                                    UnknownVariadicFunctionError { fxn_id: *function },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }
                    }
                    BytecodeInstruction::HostCall {
                        requirement,
                        dst,
                        arguments,
                    } => {
                        let request =
                            bytecode_host_function_request(program, *requirement, "HostCall")?;
                        let argument_values = arguments
                            .iter()
                            .map(|register| self.bytecode_registers.external_input(*register))
                            .collect::<MResult<Vec<LegacyValue>>>()?;
                        let output = self.bytecode_registers.cell(*dst)?;
                        self.out = register_bytecode_node(
                            &state_brrw,
                            Box::new(ExternalHostCallFunction {
                                request,
                                arguments: argument_values.clone(),
                                output,
                                initial_solve_policy: InitialSolvePolicy::Solve,
                            }),
                            &argument_values,
                        )?;
                    }
                    BytecodeInstruction::ResourceRead { requirement, dst } => {
                        let instruction_index = u32::try_from(self.ip).unwrap_or(u32::MAX);
                        let request = bytecode_resource_request(
                            program,
                            *requirement,
                            "ResourceRead",
                            ResourceIntent::Read,
                        )?;
                        let planned =
                            resource_read_evidence
                                .get(&instruction_index)
                                .ok_or_else(|| {
                                    bytecode_resource_read_planning_evidence_error(
                                        instruction_index,
                                        "ResourceRead instruction has no planning evidence",
                                    )
                                })?;
                        if planned.request != request {
                            return Err(bytecode_resource_read_planning_evidence_error(
                                instruction_index,
                                format!(
                                    "evidence request {:?} differs from instruction request {:?}",
                                    planned.request, request,
                                ),
                            ));
                        }
                        self.bytecode_registers
                            .load(*dst, planned.representative.try_deep_snapshot()?)?;
                        let output = self.bytecode_registers.cell(*dst)?;
                        self.out = register_bytecode_node(
                            &state_brrw,
                            Box::new(ExternalResourceReadFunction {
                                interpreter_id: self.id,
                                request,
                                output,
                                initial_solve_policy: InitialSolvePolicy::Solve,
                                semantic_contract: None,
                            }),
                            &[],
                        )?;
                    }
                    BytecodeInstruction::ResourceWrite {
                        requirement,
                        dst,
                        src,
                    } => {
                        let request = bytecode_resource_request(
                            program,
                            *requirement,
                            "ResourceWrite",
                            ResourceIntent::Assign,
                        )?;
                        let input = self.bytecode_registers.external_input(*src)?;
                        let output = self.bytecode_registers.cell(*dst)?;
                        self.out = register_bytecode_node(
                            &state_brrw,
                            Box::new(ExternalResourceWriteFunction {
                                request,
                                input: input.clone(),
                                output,
                                initial_solve_policy: InitialSolvePolicy::Solve,
                                semantic_contract: None,
                            }),
                            &[input],
                        )?;
                    }
                    BytecodeInstruction::ResourceSend {
                        requirement,
                        dst,
                        src,
                    } => {
                        let request = bytecode_resource_request(
                            program,
                            *requirement,
                            "ResourceSend",
                            ResourceIntent::Send,
                        )?;
                        let input = self.bytecode_registers.external_input(*src)?;
                        let output = self.bytecode_registers.cell(*dst)?;
                        self.out = register_bytecode_node(
                            &state_brrw,
                            Box::new(ExternalResourceWriteFunction {
                                request,
                                input: input.clone(),
                                output,
                                initial_solve_policy: InitialSolvePolicy::Solve,
                                semantic_contract: None,
                            }),
                            &[input],
                        )?;
                    }
                    BytecodeInstruction::Return { src } => {
                        self.out = self.bytecode_registers.value(*src)?;
                    }
                }
                self.ip += 1;
            }
        }
        // Bind symbols to the decoded register values, then load the exact
        // checked dictionary. A bytecode install replaces these tables.
        {
            let mut state_brrw = self.state.borrow_mut();
            let mut symbol_table = state_brrw.symbol_table.borrow_mut();
            *symbol_table = SymbolTable::new();
            state_brrw.dictionary.borrow_mut().clear();
            for (id, register) in &program.symbols {
                let cell = self.bytecode_registers.cell(*register)?;
                symbol_table.insert_cell(*id, cell, program.mutable_symbols.contains(id));
            }
            for (id, name) in &program.dictionary {
                symbol_table
                    .dictionary
                    .borrow_mut()
                    .insert(*id, name.clone());
                state_brrw.dictionary.borrow_mut().insert(*id, name.clone());
            }
            drop(symbol_table);
            #[cfg(feature = "invariant_define")]
            {
                let constraints = &mut state_brrw.integrity_constraints;
                constraints.clear();
                let marker_id = hash_str("integrity/constraint");
                let mut markers = BTreeMap::new();
                for instruction in &program.instructions {
                    let BytecodeInstruction::RuntimeVariadic {
                        function,
                        arguments,
                        ..
                    } = instruction
                    else {
                        continue;
                    };
                    if *function != marker_id {
                        continue;
                    }
                    if arguments.len() != 6 {
                        return Err(MechError::new(
                            BytecodeValidationError {
                                reason: format!(
                                    "integrity constraint marker has {} metadata inputs, expected 6",
                                    arguments.len()
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                    let result = arguments[0];
                    if markers.insert(result, arguments.clone()).is_some() {
                        return Err(MechError::new(
                            BytecodeValidationError {
                                reason: format!(
                                    "integrity constraint register {result} has duplicate markers"
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                }
                for (id, register) in &program.symbols {
                    let Some(name) = program.dictionary.get(id) else {
                        continue;
                    };
                    if !name.ends_with('!') {
                        continue;
                    }
                    if program.mutable_symbols.contains(id) {
                        return Err(MechError::new(
                            BytecodeValidationError {
                                reason: format!(
                                    "integrity constraint symbol {name:?} must be immutable"
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                    let Some(metadata) = markers.remove(register) else {
                        return Err(MechError::new(
                            BytecodeValidationError {
                                reason: format!(
                                    "integrity constraint symbol {name:?} is missing its runtime marker"
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    };
                    let metadata_string = |index: usize, label: &str| -> MResult<String> {
                        match self.bytecode_registers.value(metadata[index])? {
                            LegacyValue::String(value) => Ok(value.borrow().clone()),
                            value => Err(MechError::new(
                                BytecodeValidationError {
                                    reason: format!(
                                        "integrity constraint {label} metadata must be String, found {:?}",
                                        value.kind()
                                    ),
                                },
                                None,
                            )
                            .with_compiler_loc()),
                        }
                    };
                    let encoded_name = metadata_string(1, "name")?;
                    if encoded_name != *name {
                        return Err(MechError::new(
                            BytecodeValidationError {
                                reason: format!(
                                    "integrity constraint marker name {encoded_name:?} does not match symbol {name:?}"
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                    let expression = metadata_string(2, "expression")?;
                    let operator = match self.bytecode_registers.value(metadata[3])? {
                        LegacyValue::Empty => None,
                        LegacyValue::String(value) => Some(match value.borrow().as_str() {
                            "eq" => FormulaOperator::Comparison(ComparisonOp::Equal),
                            "neq" => FormulaOperator::Comparison(ComparisonOp::NotEqual),
                            "lt" => FormulaOperator::Comparison(ComparisonOp::LessThan),
                            "lte" => FormulaOperator::Comparison(ComparisonOp::LessThanEqual),
                            "gt" => FormulaOperator::Comparison(ComparisonOp::GreaterThan),
                            "gte" => FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual),
                            code => {
                                return Err(MechError::new(
                                    BytecodeValidationError {
                                        reason: format!(
                                            "integrity constraint marker has unknown operator {code:?}"
                                        ),
                                    },
                                    None,
                                )
                                .with_compiler_loc());
                            }
                        }),
                        value => {
                            return Err(MechError::new(
                                BytecodeValidationError {
                                    reason: format!(
                                        "integrity constraint operator metadata must be Empty or String, found {:?}",
                                        value.kind()
                                    ),
                                },
                                None,
                            )
                            .with_compiler_loc());
                        }
                    };
                    let optional_operand = |index: usize| -> MResult<Option<ValRef>> {
                        match self.bytecode_registers.value(metadata[index])? {
                            LegacyValue::Empty => Ok(None),
                            value if operator.is_none() => Err(MechError::new(
                                BytecodeValidationError {
                                    reason: format!(
                                        "integrity constraint without an operator has {:?} operand metadata",
                                        value.kind()
                                    ),
                                },
                                None,
                            )
                            .with_compiler_loc()),
                            _ => Ok(Some(self.bytecode_registers.cell(metadata[index])?)),
                        }
                    };
                    let lhs = optional_operand(4)?;
                    let rhs = optional_operand(5)?;
                    constraints.insert(
                        *id,
                        IntegrityConstraint {
                            id: *id,
                            name: name.clone(),
                            expression,
                            result: self.bytecode_registers.cell(*register)?,
                            lhs,
                            operator,
                            rhs,
                            tokens: Vec::new(),
                        },
                    );
                }
                if let Some(register) = markers.keys().next() {
                    return Err(MechError::new(
                        BytecodeValidationError {
                            reason: format!(
                                "integrity constraint marker for register {register} has no immutable `!` symbol"
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            }
        }

        if self.plan_len() > 0 {
            self.solve_plan_with_services(services)?;
        }

        let Some(BytecodeInstruction::Return { src }) = program.instructions.last() else {
            return Err(MechError::new(
                UnknownInstructionError {
                    instr: "validated bytecode has no final Return".to_string(),
                },
                None,
            )
            .with_compiler_loc());
        };
        self.out = self.bytecode_registers.value(*src)?;
        Ok(self.out.clone())
    }
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn register_bytecode_function(
    state: &ProgramState,
    entry: &RuntimeFunctionEntry,
    function_args: FunctionArgs,
) -> MResult<LegacyValue> {
    let input_values = function_args.input_values();
    let function = entry.instantiate(function_args)?;
    register_bytecode_node(state, function, &input_values)
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn register_bytecode_node(
    state: &ProgramState,
    function: Box<dyn MechFunction>,
    input_values: &[LegacyValue],
) -> MResult<LegacyValue> {
    let output = function.out();
    state.plan.register_function(function, input_values)?;
    Ok(output)
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn bytecode_host_function_request(
    program: &ParsedProgram,
    requirement: u32,
    instruction: &'static str,
) -> MResult<ExecutionHostFunctionRequest> {
    match program.requirements.get(requirement as usize) {
        Some(ApplicationRequirement::HostFunction(request)) => Ok(request.clone()),
        actual => Err(bytecode_external_requirement_mismatch(
            instruction,
            requirement,
            "HostFunction",
            actual,
        )),
    }
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn bytecode_resource_request(
    program: &ParsedProgram,
    requirement: u32,
    instruction: &'static str,
    expected_intent: ResourceIntent,
) -> MResult<ExecutionResourceRequest> {
    match program.requirements.get(requirement as usize) {
        Some(ApplicationRequirement::Resource(request)) if request.intent == expected_intent => {
            Ok(request.clone())
        }
        actual => Err(bytecode_external_requirement_mismatch(
            instruction,
            requirement,
            match expected_intent {
                ResourceIntent::Read => "Resource(Read)",
                ResourceIntent::Assign => "Resource(Assign)",
                ResourceIntent::Send => "Resource(Send)",
            },
            actual,
        )),
    }
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn bytecode_resource_read_planning_evidence_error(
    instruction: u32,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        BytecodeResourceReadPlanningEvidenceError {
            instruction,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn validate_bytecode_resource_read_evidence(
    program: &ParsedProgram,
    evidence: &HashMap<u32, PlannedResourceRead>,
) -> MResult<()> {
    for (instruction, planned) in evidence {
        let Some(BytecodeInstruction::ResourceRead { requirement, .. }) = program
            .instructions
            .get(usize::try_from(*instruction).unwrap_or(usize::MAX))
        else {
            return Err(bytecode_resource_read_planning_evidence_error(
                *instruction,
                "planning evidence points at a non-ResourceRead instruction",
            ));
        };
        let request =
            bytecode_resource_request(program, *requirement, "ResourceRead", ResourceIntent::Read)?;
        if planned.request != request {
            return Err(bytecode_resource_read_planning_evidence_error(
                *instruction,
                format!(
                    "evidence request {:?} differs from instruction request {:?}",
                    planned.request, request,
                ),
            ));
        }
    }

    for (index, instruction) in program.instructions.iter().enumerate() {
        if matches!(instruction, BytecodeInstruction::ResourceRead { .. }) {
            let instruction = u32::try_from(index).unwrap_or(u32::MAX);
            if !evidence.contains_key(&instruction) {
                return Err(bytecode_resource_read_planning_evidence_error(
                    instruction,
                    "ResourceRead instruction has no planning evidence",
                ));
            }
        }
    }

    Ok(())
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn bytecode_external_requirement_mismatch(
    instruction: &'static str,
    requirement: u32,
    expected: &'static str,
    actual: Option<&ApplicationRequirement>,
) -> MechError {
    MechError::new(
        BytecodeExternalRequirementMismatch {
            instruction,
            requirement,
            expected,
            actual: actual.map(|requirement| format!("{requirement:?}")),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

// Interpreter Errors
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UnknownInstructionError {
    pub instr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeExternalRequirementMismatch {
    pub instruction: &'static str,
    pub requirement: u32,
    pub expected: &'static str,
    pub actual: Option<String>,
}

impl MechErrorKind for BytecodeExternalRequirementMismatch {
    fn name(&self) -> &str {
        "BytecodeExternalRequirementMismatch"
    }

    fn message(&self) -> String {
        format!(
            "{} requirement {} must be {}, found {}",
            self.instruction,
            self.requirement,
            self.expected,
            self.actual.as_deref().unwrap_or("no requirement"),
        )
    }
}
impl MechErrorKind for UnknownInstructionError {
    fn name(&self) -> &str {
        "UnknownInstruction"
    }

    fn message(&self) -> String {
        format!("Unknown instruction: {}", self.instr)
    }
}

#[derive(Debug, Clone)]
pub struct UnknownVariadicFunctionError {
    pub fxn_id: u64,
}

impl MechErrorKind for UnknownVariadicFunctionError {
    fn name(&self) -> &str {
        "UnknownVariadicFunction"
    }
    fn message(&self) -> String {
        format!("Unknown variadic function ID: {}", self.fxn_id)
    }
}

#[derive(Debug, Clone)]
pub struct UnknownQuadFunctionError {
    pub fxn_id: u64,
}
impl MechErrorKind for UnknownQuadFunctionError {
    fn name(&self) -> &str {
        "UnknownQuadFunction"
    }
    fn message(&self) -> String {
        format!("Unknown quad function ID: {}", self.fxn_id)
    }
}

#[derive(Debug, Clone)]
pub struct UnknownTernaryFunctionError {
    pub fxn_id: u64,
}
impl MechErrorKind for UnknownTernaryFunctionError {
    fn name(&self) -> &str {
        "UnknownTernaryFunction"
    }
    fn message(&self) -> String {
        format!("Unknown ternary function ID: {}", self.fxn_id)
    }
}

#[derive(Debug, Clone)]
pub struct UnknownBinaryFunctionError {
    pub fxn_id: u64,
}
impl MechErrorKind for UnknownBinaryFunctionError {
    fn name(&self) -> &str {
        "UnknownBinaryFunction"
    }
    fn message(&self) -> String {
        format!("Unknown binary function ID: {}", self.fxn_id)
    }
}

#[derive(Debug, Clone)]
pub struct UnknownUnaryFunctionError {
    pub fxn_id: u64,
}
impl MechErrorKind for UnknownUnaryFunctionError {
    fn name(&self) -> &str {
        "UnknownUnaryFunction"
    }
    fn message(&self) -> String {
        format!("Unknown unary function ID: {}", self.fxn_id)
    }
}

#[derive(Debug, Clone)]
pub struct UnknownNullaryFunctionError {
    pub fxn_id: u64,
}
impl MechErrorKind for UnknownNullaryFunctionError {
    fn name(&self) -> &str {
        "UnknownNullaryFunction"
    }
    fn message(&self) -> String {
        format!("Unknown nullary function ID: {}", self.fxn_id)
    }
}

#[derive(Debug, Clone)]
pub struct IndexOutOfBoundsError;
impl MechErrorKind for IndexOutOfBoundsError {
    fn name(&self) -> &str {
        "IndexOutOfBounds"
    }
    fn message(&self) -> String {
        "Index out of bounds".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct OverflowSubtractionError;
impl MechErrorKind for OverflowSubtractionError {
    fn name(&self) -> &str {
        "OverflowSubtraction"
    }
    fn message(&self) -> String {
        "Attempted subtraction overflow".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct UnknownPanicError {
    pub details: String,
}
impl MechErrorKind for UnknownPanicError {
    fn name(&self) -> &str {
        "UnknownPanic"
    }
    fn message(&self) -> String {
        self.details.clone()
    }
}

#[derive(Debug, Clone)]
struct StepIndexOutOfBoundsError {
    pub step_id: usize,
    pub plan_length: usize,
}
impl MechErrorKind for StepIndexOutOfBoundsError {
    fn name(&self) -> &str {
        "StepIndexOutOfBounds"
    }
    fn message(&self) -> String {
        format!(
            "Step id {} out of range (plan has {} steps)",
            self.step_id, self.plan_length
        )
    }
}

#[derive(Debug, Clone)]
struct NoStepsInPlanError;
impl MechErrorKind for NoStepsInPlanError {
    fn name(&self) -> &str {
        "NoStepsInPlan"
    }
    fn message(&self) -> String {
        "Plan contains no steps. This program doesn't do anything.".to_string()
    }
}

pub struct InterpreterExecution<'a> {
    interpreter: &'a Interpreter,
    // Mechdown renderers address the top-level document through the stable
    // source namespace `0`, while a retained program uses a distinct physical
    // interpreter ID derived from its configuration. Keep that presentation
    // namespace on the execution view so output keys stay compatible with the
    // formatter without changing runtime interpreter identity.
    presentation_namespace: u64,
    services: StdRefCell<&'a mut dyn MechExecutionServices>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionServicesBorrowConflict {
    pub operation: &'static str,
}

impl MechErrorKind for ExecutionServicesBorrowConflict {
    fn name(&self) -> &str {
        "ExecutionServicesBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Execution services are already borrowed while attempting `{}`.",
            self.operation,
        )
    }
}

impl<'a> InterpreterExecution<'a> {
    pub fn new(interpreter: &'a Interpreter, services: &'a mut dyn MechExecutionServices) -> Self {
        Self::with_presentation_namespace(interpreter, 0, services)
    }

    fn with_presentation_namespace(
        interpreter: &'a Interpreter,
        presentation_namespace: u64,
        services: &'a mut dyn MechExecutionServices,
    ) -> Self {
        Self {
            interpreter,
            presentation_namespace,
            services: StdRefCell::new(services),
        }
    }

    pub(crate) fn presentation_namespace(&self) -> u64 {
        self.presentation_namespace
    }

    #[cfg(feature = "functions")]
    pub fn specialize_visible_operation(
        &self,
        operation: OperationId,
        arguments: &[LegacyValue],
    ) -> MResult<Box<dyn MechFunction>> {
        self.specialize_visible_operation_named(operation, None, arguments)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn specialize_visible_operation_named(
        &self,
        operation: OperationId,
        canonical_name: Option<&str>,
        arguments: &[LegacyValue],
    ) -> MResult<Box<dyn MechFunction>> {
        let state = self.state.borrow();
        FunctionResolver::new(
            self.function_catalog(),
            &state.function_environment,
            &state.function_extensions,
            &state.user_functions,
        )
        .specialize_operation_named(operation, canonical_name, arguments)
    }

    pub fn with_services<T>(
        &self,
        operation: impl FnOnce(&mut dyn MechExecutionServices) -> MResult<T>,
    ) -> MResult<T> {
        let mut services = self.services.try_borrow_mut().map_err(|_| {
            MechError::new(
                ExecutionServicesBorrowConflict {
                    operation: "with_services",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        operation(&mut **services)
    }

    pub fn with_interpreter<T>(
        &self,
        interpreter: &Interpreter,
        operation: impl FnOnce(&InterpreterExecution<'_>) -> MResult<T>,
    ) -> MResult<T> {
        let mut services = self.services.try_borrow_mut().map_err(|_| {
            MechError::new(
                ExecutionServicesBorrowConflict {
                    operation: "with_interpreter",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let execution = InterpreterExecution::with_presentation_namespace(
            interpreter,
            interpreter.id,
            &mut **services,
        );
        operation(&execution)
    }
}

impl Deref for InterpreterExecution<'_> {
    type Target = Interpreter;

    fn deref(&self) -> &Self::Target {
        self.interpreter
    }
}

impl Interpreter {
    /// Whether executable source or bytecode state is retained. This is a
    /// defensive ownership signal; catalogs and empty environments do not
    /// count as installed programs.
    pub fn has_retained_execution_state(&self) -> bool {
        if !self.code.is_empty() || self.ip != 0 || !self.constants.is_empty() {
            return true;
        }
        #[cfg(feature = "functions")]
        if self.plan_len() != 0 {
            return true;
        }
        false
    }
}
