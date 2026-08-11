pub mod argument;
pub mod catalog;
pub mod contract;
pub mod resident;
pub mod signature;
pub use argument::*;
pub use catalog::*;
pub use contract::*;
pub use resident::*;
pub use signature::*;

use crate::legacy_value::*;
use crate::nodes::*;
use crate::types::*;
use crate::*;

#[cfg(feature = "functions")]
use indexmap::map::IndexMap;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
#[cfg(feature = "pretty_print")]
use tabled::{
    Tabled,
    builder::Builder,
    settings::{Alignment, Modify, Panel, Span, Style, object::Rows},
};

// Functions ------------------------------------------------------------------

/// Program-local user-function definitions keyed by their stable name hash.
///
/// The backing map is intentionally opaque so callers cannot accidentally
/// replace a different name that happens to share the same stable ID.
#[derive(Clone, Default)]
pub struct UserFunctionTable {
    definitions: HashMap<u64, FunctionDefinition>,
}

impl UserFunctionTable {
    /// Resolves one exact source-visible name.
    pub fn resolve_name(&self, name: &str) -> Option<&FunctionDefinition> {
        let id = hash_str(name);
        self.definitions
            .get(&id)
            .filter(|definition| definition.name == name)
    }

    /// Inserts a definition, replacing an existing definition only when both
    /// definitions have the exact same name.
    pub fn insert_or_replace(
        &mut self,
        definition: FunctionDefinition,
    ) -> MResult<Option<FunctionDefinition>> {
        validate_user_function_definition(&definition)?;

        if let Some(existing) = self.definitions.get(&definition.id)
            && existing.name != definition.name
        {
            return Err(MechError::new(
                UserFunctionIdCollision {
                    id: definition.id,
                    existing_name: existing.name.clone(),
                    incoming_name: definition.name,
                },
                None,
            )
            .with_compiler_loc());
        }

        Ok(self.definitions.insert(definition.id, definition))
    }

    pub fn definitions(&self) -> impl ExactSizeIterator<Item = &FunctionDefinition> + '_ {
        self.definitions.values()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn clear(&mut self) {
        self.definitions.clear();
    }
}

fn validate_user_function_definition(definition: &FunctionDefinition) -> MResult<()> {
    if definition.name.is_empty() {
        return Err(invalid_user_function_definition(
            definition,
            "name must not be empty",
        ));
    }

    let expected = hash_str(&definition.name);
    if expected != definition.id {
        return Err(invalid_user_function_definition(
            definition,
            format!(
                "name hashes to 0x{expected:016x}, not 0x{:016x}",
                definition.id,
            ),
        ));
    }

    Ok(())
}

fn invalid_user_function_definition(
    definition: &FunctionDefinition,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        UserFunctionInvalidDefinition {
            id: definition.id,
            name: definition.name.clone(),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserFunctionInvalidDefinition {
    pub id: u64,
    pub name: String,
    pub reason: String,
}

impl MechErrorKind for UserFunctionInvalidDefinition {
    fn name(&self) -> &str {
        "UserFunctionInvalidDefinition"
    }

    fn message(&self) -> String {
        format!(
            "invalid user function {:?} at ID 0x{:016x}: {}",
            self.name, self.id, self.reason,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserFunctionIdCollision {
    pub id: u64,
    pub existing_name: String,
    pub incoming_name: String,
}

impl MechErrorKind for UserFunctionIdCollision {
    fn name(&self) -> &str {
        "UserFunctionIdCollision"
    }

    fn message(&self) -> String {
        format!(
            "user function names {:?} and {:?} collide at ID 0x{:016x}",
            self.existing_name, self.incoming_name, self.id,
        )
    }
}

#[derive(Clone, Debug)]
pub enum FunctionArgs {
    Nullary(LegacyValue),
    Unary(LegacyValue, LegacyValue),
    Binary(LegacyValue, LegacyValue, LegacyValue),
    Ternary(LegacyValue, LegacyValue, LegacyValue, LegacyValue),
    Quaternary(
        LegacyValue,
        LegacyValue,
        LegacyValue,
        LegacyValue,
        LegacyValue,
    ),
    Variadic(LegacyValue, Vec<LegacyValue>),
}

impl FunctionArgs {
    pub(crate) fn normalize_for_signature(self, signature: RuntimeFunctionSignature) -> Self {
        if !matches!(signature.inputs, RuntimeFunctionInputs::Variadic { .. }) {
            return self;
        }
        match self {
            FunctionArgs::Nullary(output) => FunctionArgs::Variadic(output, Vec::new()),
            FunctionArgs::Unary(output, a) => FunctionArgs::Variadic(output, vec![a]),
            FunctionArgs::Binary(output, a, b) => FunctionArgs::Variadic(output, vec![a, b]),
            FunctionArgs::Ternary(output, a, b, c) => FunctionArgs::Variadic(output, vec![a, b, c]),
            FunctionArgs::Quaternary(output, a, b, c, d) => {
                FunctionArgs::Variadic(output, vec![a, b, c, d])
            }
            args @ FunctionArgs::Variadic(_, _) => args,
        }
    }

    pub fn output_value(&self) -> &LegacyValue {
        match self {
            FunctionArgs::Nullary(output)
            | FunctionArgs::Unary(output, _)
            | FunctionArgs::Binary(output, _, _)
            | FunctionArgs::Ternary(output, _, _, _)
            | FunctionArgs::Quaternary(output, _, _, _, _)
            | FunctionArgs::Variadic(output, _) => output,
        }
    }

    pub fn input_value(&self, index: usize) -> Option<&LegacyValue> {
        match (self, index) {
            (FunctionArgs::Unary(_, a), 0) => Some(a),
            (FunctionArgs::Binary(_, a, _), 0) => Some(a),
            (FunctionArgs::Binary(_, _, b), 1) => Some(b),
            (FunctionArgs::Ternary(_, a, _, _), 0) => Some(a),
            (FunctionArgs::Ternary(_, _, b, _), 1) => Some(b),
            (FunctionArgs::Ternary(_, _, _, c), 2) => Some(c),
            (FunctionArgs::Quaternary(_, a, _, _, _), 0) => Some(a),
            (FunctionArgs::Quaternary(_, _, b, _, _), 1) => Some(b),
            (FunctionArgs::Quaternary(_, _, _, c, _), 2) => Some(c),
            (FunctionArgs::Quaternary(_, _, _, _, d), 3) => Some(d),
            (FunctionArgs::Variadic(_, arguments), index) => arguments.get(index),
            _ => None,
        }
    }

    pub fn input_count(&self) -> usize {
        self.len()
    }

    pub fn validate_contract(&self, contract: RuntimeFunctionContract) -> MResult<()> {
        if contract.output_alias == RuntimeOutputAliasPolicy::DisallowInputAlias {
            let output_roots = self.output_value().reactive_root_cell_ids();
            for index in 0..self.input_count() {
                let Some(input) = self.input_value(index) else {
                    continue;
                };
                for cell in input.reactive_root_cell_ids() {
                    if output_roots.contains(&cell) {
                        return Err(MechError::new(
                            FunctionArgumentAliasViolation { input: index, cell },
                            None,
                        )
                        .with_compiler_loc());
                    }
                }
            }
        }
        (contract.validate_shapes)(self)
    }

    pub fn validate_signature(&self, signature: RuntimeFunctionSignature) -> MResult<()> {
        let arity_kind_matches = matches!(
            (self, signature.inputs),
            (FunctionArgs::Nullary(_), RuntimeFunctionInputs::Nullary)
                | (FunctionArgs::Unary(_, _), RuntimeFunctionInputs::Unary(_))
                | (
                    FunctionArgs::Binary(_, _, _),
                    RuntimeFunctionInputs::Binary(_, _)
                )
                | (
                    FunctionArgs::Ternary(_, _, _, _),
                    RuntimeFunctionInputs::Ternary(_, _, _)
                )
                | (
                    FunctionArgs::Quaternary(_, _, _, _, _),
                    RuntimeFunctionInputs::Quaternary(_, _, _, _)
                )
                | (
                    FunctionArgs::Variadic(_, _),
                    RuntimeFunctionInputs::Variadic { .. }
                )
        );
        let expected_inputs: Vec<FunctionValueRepresentation> = match signature.inputs {
            RuntimeFunctionInputs::Nullary => Vec::new(),
            RuntimeFunctionInputs::Unary(argument) => vec![argument],
            RuntimeFunctionInputs::Binary(lhs, rhs) => vec![lhs, rhs],
            RuntimeFunctionInputs::Ternary(first, second, third) => {
                vec![first, second, third]
            }
            RuntimeFunctionInputs::Quaternary(first, second, third, fourth) => {
                vec![first, second, third, fourth]
            }
            RuntimeFunctionInputs::Variadic { element } => vec![element; self.input_count()],
        };

        if !arity_kind_matches || expected_inputs.len() != self.input_count() {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: expected_inputs.len(),
                    found: self.input_count(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let found_output = FunctionValueRepresentation::from_value(self.output_value());
        if !signature.output.matches(found_output) {
            return Err(signature_violation(
                FunctionArgumentRole::Output,
                signature.output,
                self.output_value(),
            ));
        }

        for (index, expected) in expected_inputs.into_iter().enumerate() {
            let input = self.input_value(index).expect("validated function arity");
            let found = FunctionValueRepresentation::from_value(input);
            if !expected.matches(found) {
                return Err(signature_violation(
                    FunctionArgumentRole::Input(index),
                    expected,
                    input,
                ));
            }
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        match self {
            FunctionArgs::Nullary(_) => 0,
            FunctionArgs::Unary(_, _) => 1,
            FunctionArgs::Binary(_, _, _) => 2,
            FunctionArgs::Ternary(_, _, _, _) => 3,
            FunctionArgs::Quaternary(_, _, _, _, _) => 4,
            FunctionArgs::Variadic(_, args) => args.len(),
        }
    }

    pub fn input_values(&self) -> Vec<LegacyValue> {
        match self {
            FunctionArgs::Nullary(_) => Vec::new(),

            FunctionArgs::Unary(_, a) => vec![a.clone()],

            FunctionArgs::Binary(_, a, b) => vec![a.clone(), b.clone()],

            FunctionArgs::Ternary(_, a, b, c) => vec![a.clone(), b.clone(), c.clone()],

            FunctionArgs::Quaternary(_, a, b, c, d) => {
                vec![a.clone(), b.clone(), c.clone(), d.clone()]
            }

            FunctionArgs::Variadic(_, arguments) => arguments.clone(),
        }
    }
}

pub trait MechFunctionFactory {
    const SIGNATURE: RuntimeFunctionSignature;

    /// Constructs a runtime function from its authoritative argument contract.
    ///
    /// Implementations must be deterministic and side-effect-free, safely
    /// reject arbitrary incompatible [`FunctionArgs`], validate every exact
    /// backing extraction, and must not execute or solve the function.
    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialSolvePolicy {
    Solve,
    PreserveSpecializedOutput,
}

pub trait MechFunctionImpl {
    fn solve_result(&self) -> MResult<()>;
    fn solve_result_with(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        let _ = services;
        self.solve_result()
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        self.solve_result()?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn solve_reactive_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let _ = services;
        self.solve_reactive()
    }
    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        InitialSolvePolicy::Solve
    }
    /// Performs service-aware initialization that is required even when the
    /// specialized output itself was produced during deterministic planning.
    ///
    /// This hook must not recompute or replace that planned output. Most
    /// functions need no extra initialization, so the default is a no-op.
    fn initialize_preserved_output_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        let _ = services;
        Ok(())
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        Err(MechError::new(
            ReactiveRegisterStagingUnsupportedError {
                function: self.to_string(),
            },
            None,
        )
        .with_compiler_loc())
    }
    fn out(&self) -> LegacyValue;
    fn reactive_dependency_kinds(
        &self,
        _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyKind>> {
        None
    }
    fn reactive_dependency_scopes(
        &self,
        _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        None
    }
    fn reactive_output_values(&self) -> Vec<LegacyValue> {
        vec![self.out()]
    }
    /// Returns every `Value`-backed cell that contains retained mutable state
    /// owned by this function.
    ///
    /// Reactive outputs cover the common case. Functions with hidden retained
    /// cells must return those cells, while functions whose retained state
    /// cannot be represented by `Value` must return
    /// [`TransactionStateUnsupportedError`].
    ///
    /// The function implementation itself owns this checkpoint contract.
    /// Callers must not infer checkpoint support from [`Self::to_string`],
    /// [`Debug`] output, type names, module names, or other display metadata. A
    /// function that requires participation from a higher-level transaction
    /// coordinator must return [`TransactionStateUnsupportedError`] directly.
    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>>;
    fn reactive_output_cell_ids(&self) -> Vec<ReactiveCellId> {
        let mut cells = Vec::new();

        for output in self.reactive_output_values() {
            for cell in output.reactive_root_cell_ids() {
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        }

        cells
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }
    /// Portable semantic metadata for the compiled operation represented by
    /// this specialized function. Current execution continues to use
    /// `RuntimeFunctionContract`; this declaration is consumed only while
    /// constructing a `ProgramArtifact`.
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        None
    }
    fn to_string(&self) -> String;
}

/// An already validated register write. Implementations must not fail or run
/// arbitrary reactive work when they are committed.
pub(crate) mod reactive_register_sealed {
    pub trait Sealed {}
}

pub trait ReactiveRegisterCommit: reactive_register_sealed::Sealed {
    fn output_cells(&self) -> &[ReactiveCellId];
    fn commit(self: Box<Self>);
}

pub struct ReactiveRegisterWrite<T> {
    sink: Ref<T>,
    next: T,
    output_cells: Vec<ReactiveCellId>,
}

impl<T> ReactiveRegisterWrite<T> {
    pub fn new(sink: Ref<T>, next: T, output_cells: Vec<ReactiveCellId>) -> Self {
        Self {
            sink,
            next,
            output_cells,
        }
    }
}

impl<T> reactive_register_sealed::Sealed for ReactiveRegisterWrite<T> {}

impl<T: 'static> ReactiveRegisterCommit for ReactiveRegisterWrite<T> {
    fn output_cells(&self) -> &[ReactiveCellId] {
        self.output_cells.as_slice()
    }
    fn commit(self: Box<Self>) {
        let ReactiveRegisterWrite {
            sink,
            next,
            output_cells: _,
        } = *self;
        *sink.borrow_mut() = next;
    }
}

/// A pre-staged collection of register writes that commits as one infallible
/// unit. Composite register nodes use this to preserve every nested reactive
/// cell while still reporting the outer register cell as their owned output.
pub struct ReactiveRegisterCommitBatch {
    commits: Vec<Box<dyn ReactiveRegisterCommit>>,
    output_cells: Vec<ReactiveCellId>,
}

impl ReactiveRegisterCommitBatch {
    pub fn new(
        commits: Vec<Box<dyn ReactiveRegisterCommit>>,
        output_cells: Vec<ReactiveCellId>,
    ) -> Self {
        Self {
            commits,
            output_cells,
        }
    }
}

impl reactive_register_sealed::Sealed for ReactiveRegisterCommitBatch {}

impl ReactiveRegisterCommit for ReactiveRegisterCommitBatch {
    fn output_cells(&self) -> &[ReactiveCellId] {
        self.output_cells.as_slice()
    }

    fn commit(self: Box<Self>) {
        for commit in self.commits {
            commit.commit();
        }
    }
}

pub struct ReactiveRegisterNoopCommit {
    output_cells: Vec<ReactiveCellId>,
}
impl ReactiveRegisterNoopCommit {
    pub fn new(output_cells: Vec<ReactiveCellId>) -> Self {
        Self { output_cells }
    }
}
impl reactive_register_sealed::Sealed for ReactiveRegisterNoopCommit {}
impl ReactiveRegisterCommit for ReactiveRegisterNoopCommit {
    fn output_cells(&self) -> &[ReactiveCellId] {
        self.output_cells.as_slice()
    }
    fn commit(self: Box<Self>) {}
}

#[cfg(feature = "semantic-compiler")]
pub trait MechFunctionCompiler {
    /// Reserves registers whose initializer must come from declaration-time
    /// state before other plan nodes observe their live reactive values.
    fn reserve_bytecode_registers(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        Ok(())
    }

    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register>;
}

#[cfg(feature = "semantic-compiler")]
pub trait MechFunction: MechFunctionImpl + MechFunctionCompiler {}
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunction for T where T: MechFunctionImpl + MechFunctionCompiler {}

#[cfg(not(feature = "semantic-compiler"))]
pub trait MechFunction: MechFunctionImpl {}
#[cfg(not(feature = "semantic-compiler"))]
impl<T> MechFunction for T where T: MechFunctionImpl {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardFunctionSafety {
    /// Compilation constructs only the static graph (including kind and shape
    /// selection) without executing function behavior or reading live contents;
    /// reactive solving is deferred to the guard pulse.
    PureStatic,
    /// Compilation may execute work or lacks an explicit purity contract.
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct TransactionStateUnsupportedError {
    pub function: String,
    pub reason: String,
}

impl MechErrorKind for TransactionStateUnsupportedError {
    fn name(&self) -> &str {
        "TransactionStateUnsupported"
    }
    fn message(&self) -> String {
        format!(
            "Cannot checkpoint retained transaction state for function '{}': {}.",
            self.function, self.reason,
        )
    }
}

#[derive(Debug, Clone)]
pub struct TransactionStateBorrowConflictError {
    pub function: String,
    pub component: &'static str,
}

impl MechErrorKind for TransactionStateBorrowConflictError {
    fn name(&self) -> &str {
        "TransactionStateBorrowConflict"
    }
    fn message(&self) -> String {
        format!(
            "Cannot inspect retained transaction state for function '{}' because {} is already borrowed.",
            self.function, self.component,
        )
    }
}

#[derive(Clone)]
pub struct FunctionDefinition {
    pub code: FunctionDefine,
    pub id: u64,
    pub name: String,
    pub input: IndexMap<u64, KindAnnotation>,
    pub output: IndexMap<u64, KindAnnotation>,
    pub symbols: SymbolTableRef,
    pub out: Ref<LegacyValue>,
    pub plan: Plan,
}

impl fmt::Debug for FunctionDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(feature = "pretty_print") {
            #[cfg(feature = "pretty_print")]
            return fmt::Display::fmt(&self.pretty_print(), f);
            fmt::Display::fmt(&"".to_string(), f)
        } else {
            write!(
                f,
                "FunctionDefinition {{ id: {}, name: {}, input: {:?}, output: {:?}, symbols: {:?} }}",
                self.id,
                self.name,
                self.input,
                self.output,
                self.symbols.borrow()
            )
        }
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for FunctionDefinition {
    fn pretty_print(&self) -> String {
        let input_str = format!("{:#?}", self.input);
        let output_str = format!("{:#?}", self.output);
        let symbols_str = format!("{:#?}", self.symbols);
        let mut plan_str = "".to_string();
        for step in self.plan.borrow().iter() {
            plan_str = format!("{}  - {}\n", plan_str, step.to_string());
        }
        let data = vec![
            "📥 Input",
            &input_str,
            "📤 Output",
            &output_str,
            "🔣 Symbols",
            &symbols_str,
            "📋 Plan",
            &plan_str,
        ];
        let mut table = tabled::Table::new(data);
        table
            .with(Style::modern_rounded())
            .with(Panel::header(format!(
                "📈 UserFxn::{}\n({})",
                self.name,
                humanize(&self.id)
            )))
            .with(Alignment::left());
        format!("{table}")
    }
}

impl FunctionDefinition {
    pub fn new(id: u64, name: String, code: FunctionDefine) -> Self {
        Self {
            id,
            name,
            code,
            input: IndexMap::new(),
            output: IndexMap::new(),
            out: Ref::new(LegacyValue::Empty),
            symbols: Ref::new(SymbolTable::new()),
            plan: Plan::new(),
        }
    }

    pub fn solve_result(&self) -> MResult<ValRef> {
        let plan_brrw = self.plan.borrow();
        for step in plan_brrw.iter() {
            step.solve_result()?;
        }
        Ok(self.out.clone())
    }

    pub fn out(&self) -> ValRef {
        self.out.clone()
    }
}

// User Function --------------------------------------------------------------

pub struct UserFunction {
    pub fxn: FunctionDefinition,
}

impl MechFunctionImpl for UserFunction {
    fn solve_result(&self) -> MResult<()> {
        self.fxn.solve_result()?;
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.fxn.out.borrow().clone()
    }
    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        let mut values = vec![LegacyValue::MutableReference(self.fxn.out.clone())];
        let mut seen_refs = HashSet::new();
        seen_refs.insert(self.fxn.out.addr());
        let symbols = self.fxn.symbols.try_borrow().map_err(|_| {
            MechError::new(
                TransactionStateBorrowConflictError {
                    function: self.to_string(),
                    component: "user symbol table",
                },
                None,
            )
            .with_compiler_loc()
        })?;
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
        values.extend(self.fxn.plan.transaction_state_values()?);
        Ok(values)
    }
    fn to_string(&self) -> String {
        format!("UserFxn::{:?}", self.fxn.name)
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for UserFunction {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        todo!();
    }
}

// Reactive Plan
// ----------------------------------------------------------------------------

pub type ReactiveNodeId = usize;

/// Read-only registration information for a patterned activation.
///
/// This belongs to the plan, rather than to a turn, so consumers can inspect
/// the statically registered dispatch graph without relying on transient
/// scheduler state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationRegistration {
    pub scope_pulse_node: ReactiveNodeId,
    pub selector_node: ReactiveNodeId,
    pub arms: Vec<PatternActivationArmRegistration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationArmRegistration {
    pub matcher_node: ReactiveNodeId,
    /// The structural-only finalizer for an unguarded arm, or the unmatched
    /// finalization path for a guarded arm.
    pub finalizer_node: ReactiveNodeId,
    pub guard: Option<PatternActivationGuardRegistration>,
    pub gate_node: ReactiveNodeId,
    pub pulse_cell: ReactiveCellId,
    /// Half-open range of plan nodes registered for this arm's body.
    pub body_node_start: usize,
    pub body_node_end: usize,
    pub captures: Vec<PatternActivationCaptureRegistration>,
}

/// Static graph information for one guarded patterned-activation arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationGuardRegistration {
    pub match_gate_node: ReactiveNodeId,
    pub guard_finalizer_node: ReactiveNodeId,
    /// Half-open range containing the ordinary guard-expression graph followed
    /// by its guard finalizer.
    pub guard_node_start: usize,
    pub guard_node_end: usize,
}

/// Stable storage made available to a single patterned activation arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternActivationCaptureRegistration {
    pub id: u64,
    pub kind: ValueKind,
    pub cell: ReactiveCellId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveDependencyKind {
    Reactive,
    Sampled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveDependencyScope {
    Recursive,
    Logical,
    Root,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReactiveNodeKind {
    Combinational,
    Register,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveSolveStatus {
    Changed,
    Unchanged,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactivePlanSolveOutcome {
    pub executed_nodes: Vec<ReactiveNodeId>,
    pub changed_nodes: Vec<ReactiveNodeId>,
    pub unchanged_nodes: Vec<ReactiveNodeId>,
    pub pending_register_nodes: Vec<ReactiveNodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveRegisterCommitOutcome {
    pub staged_nodes: Vec<ReactiveNodeId>,
    pub committed_nodes: Vec<ReactiveNodeId>,
    pub dirty_cells: Vec<ReactiveCellId>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveTurnState {
    pub pending_register_nodes: Vec<ReactiveNodeId>,
}

impl ReactiveTurnState {
    pub fn has_pending_registers(&self) -> bool {
        !self.pending_register_nodes.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveTurnOutcome {
    pub before_commit: ReactivePlanSolveOutcome,
    pub register_commit: ReactiveRegisterCommitOutcome,
    pub after_commit: ReactivePlanSolveOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationRegistrationScope {
    pub trigger_cells: Vec<ReactiveCellId>,
    pub local_combinational_cells: Vec<ReactiveCellId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactiveDependency {
    pub cell: ReactiveCellId,
    pub kind: ReactiveDependencyKind,
}

pub struct ReactivePlanFunction {
    function: Box<dyn MechFunction>,
    identity: Rc<()>,
}

impl ReactivePlanFunction {
    fn new(function: Box<dyn MechFunction>) -> Self {
        Self {
            function,
            identity: Rc::new(()),
        }
    }

    pub fn as_ref(&self) -> &dyn MechFunction {
        self.function.as_ref()
    }
}

impl core::ops::Deref for ReactivePlanFunction {
    type Target = dyn MechFunction;

    fn deref(&self) -> &Self::Target {
        self.function.as_ref()
    }
}

impl core::ops::DerefMut for ReactivePlanFunction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.function.as_mut()
    }
}

pub struct ReactivePlanNode {
    pub id: ReactiveNodeId,
    pub plan_index: usize,
    pub function: ReactivePlanFunction,
    pub inputs: Vec<ReactiveDependency>,
    pub outputs: Vec<ReactiveCellId>,
    pub kind: ReactiveNodeKind,
}

pub struct ReactivePlan {
    pub nodes: Vec<ReactivePlanNode>,
    pub reactive_consumers: HashMap<ReactiveCellId, Vec<ReactiveNodeId>>,
    pub sampled_consumers: HashMap<ReactiveCellId, Vec<ReactiveNodeId>>,
    pattern_activation_registrations: Vec<PatternActivationRegistration>,
    activation_sampled_cells: Vec<Vec<ReactiveCellId>>,
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyArityMismatchError {
    pub function: String,
    pub expected: usize,
    pub found: usize,
}

impl MechErrorKind for ReactiveDependencyArityMismatchError {
    fn name(&self) -> &str {
        "ReactiveDependencyArityMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Reactive dependency arity mismatch for function '{}': expected {} dependency kinds, found {}.",
            self.function, self.expected, self.found,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyScopeArityMismatchError {
    pub function: String,
    pub expected: usize,
    pub found: usize,
}

impl MechErrorKind for ReactiveDependencyScopeArityMismatchError {
    fn name(&self) -> &str {
        "ReactiveDependencyScopeArityMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Reactive dependency scope arity mismatch for function '{}': expected argument count {}, provided scope count {}.",
            self.function, self.expected, self.found,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyKindConflictError {
    pub function: String,
    pub cell: ReactiveCellId,
}

#[derive(Debug, Clone)]
pub struct ReactiveRegisterStagingUnsupportedError {
    pub function: String,
}
impl MechErrorKind for ReactiveRegisterStagingUnsupportedError {
    fn name(&self) -> &str {
        "ReactiveRegisterStagingUnsupported"
    }
    fn message(&self) -> String {
        format!(
            "Reactive register staging is not implemented for function '{}'.",
            self.function
        )
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterNodeNotFoundError {
    pub node_id: ReactiveNodeId,
}
impl MechErrorKind for ReactiveRegisterNodeNotFoundError {
    fn name(&self) -> &str {
        "ReactiveRegisterNodeNotFound"
    }
    fn message(&self) -> String {
        format!("Reactive register node {} does not exist.", self.node_id)
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterNodeKindError {
    pub node_id: ReactiveNodeId,
    pub actual: ReactiveNodeKind,
}
impl MechErrorKind for ReactiveRegisterNodeKindError {
    fn name(&self) -> &str {
        "ReactiveRegisterNodeKind"
    }
    fn message(&self) -> String {
        format!(
            "Reactive node {} must be a register for commit, but its kind is {:?}.",
            self.node_id, self.actual
        )
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterOutputConflictError {
    pub cell: ReactiveCellId,
    pub first_node: ReactiveNodeId,
    pub second_node: ReactiveNodeId,
}
impl MechErrorKind for ReactiveRegisterOutputConflictError {
    fn name(&self) -> &str {
        "ReactiveRegisterOutputConflict"
    }
    fn message(&self) -> String {
        format!(
            "Reactive register nodes {} and {} both write output cell {:?}.",
            self.first_node, self.second_node, self.cell
        )
    }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterStagedOutputMismatchError {
    pub node_id: ReactiveNodeId,
    pub expected: Vec<ReactiveCellId>,
    pub found: Vec<ReactiveCellId>,
}
impl MechErrorKind for ReactiveRegisterStagedOutputMismatchError {
    fn name(&self) -> &str {
        "ReactiveRegisterStagedOutputMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Reactive register node {} staged outputs {:?}, but its registered outputs are {:?}.",
            self.node_id, self.found, self.expected
        )
    }
}

impl MechErrorKind for ReactiveDependencyKindConflictError {
    fn name(&self) -> &str {
        "ReactiveDependencyKindConflict"
    }

    fn message(&self) -> String {
        format!(
            "Reactive dependency kind conflict for function '{}': one node classified cell {:?} as both reactive and sampled.",
            self.function, self.cell,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReactivePlanNodeCheckpoint {
    id: ReactiveNodeId,
    plan_index: usize,
    inputs: Vec<ReactiveDependency>,
    outputs: Vec<ReactiveCellId>,
    kind: ReactiveNodeKind,
    function_identity: ReactiveFunctionIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactivePlanCheckpoint {
    nodes: Vec<ReactivePlanNodeCheckpoint>,
    pattern_activation_registrations: Vec<PatternActivationRegistration>,
    activation_sampled_cells: Vec<Vec<ReactiveCellId>>,
}

impl ReactivePlanCheckpoint {
    pub fn node_len(&self) -> usize {
        self.nodes.len()
    }
}

/// A process-local structural checkpoint for a [`Plan`].
///
/// This checkpoint supports append-only plan elaboration. Removing or
/// replacing a function object that existed at capture time invalidates
/// restoration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCheckpoint {
    reactive: ReactivePlanCheckpoint,
    activation_registration_scopes: Vec<ActivationRegistrationScope>,
}

impl PlanCheckpoint {
    pub fn node_len(&self) -> usize {
        self.reactive.node_len()
    }
}

#[derive(Debug, Clone)]
pub struct ReactivePlanRollbackInvariantError {
    pub checkpoint_nodes: usize,
    pub current_nodes: usize,
    pub checkpoint_registrations: usize,
    pub current_registrations: usize,
}

impl MechErrorKind for ReactivePlanRollbackInvariantError {
    fn name(&self) -> &str {
        "ReactivePlanRollbackInvariant"
    }
    fn message(&self) -> String {
        format!(
            "Cannot roll the reactive plan back from {} nodes and {} patterned registrations to {} nodes and {} patterned registrations.",
            self.current_nodes,
            self.current_registrations,
            self.checkpoint_nodes,
            self.checkpoint_registrations
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactivePlanFunctionIdentityError {
    pub node_id: ReactiveNodeId,
}

impl MechErrorKind for ReactivePlanFunctionIdentityError {
    fn name(&self) -> &str {
        "ReactivePlanFunctionIdentity"
    }
    fn message(&self) -> String {
        format!(
            "Cannot restore reactive node {} because its function identity changed.",
            self.node_id,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ActivationRegistrationRollbackInvariantError {
    pub checkpoint_depth: usize,
    pub current_depth: usize,
}

impl MechErrorKind for ActivationRegistrationRollbackInvariantError {
    fn name(&self) -> &str {
        "ActivationRegistrationRollbackInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Cannot roll the activation registration stack back from depth {} to future depth {}.",
            self.current_depth, self.checkpoint_depth,
        )
    }
}

#[derive(Debug, Clone)]
pub struct PlanCheckpointBorrowConflictError {
    pub phase: &'static str,
    pub component: &'static str,
}

impl MechErrorKind for PlanCheckpointBorrowConflictError {
    fn name(&self) -> &str {
        "PlanCheckpointBorrowConflict"
    }
    fn message(&self) -> String {
        format!(
            "Cannot borrow plan {} during {}.",
            self.component, self.phase,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReactivePlanCheckpointInvariantError {
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ReactiveTurnCheckpointInvariantError {
    pub node_id: ReactiveNodeId,
    pub reason: String,
}

impl MechErrorKind for ReactiveTurnCheckpointInvariantError {
    fn name(&self) -> &str {
        "ReactiveTurnCheckpointInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Cannot checkpoint pending reactive node {}: {}.",
            self.node_id, self.reason,
        )
    }
}

impl MechErrorKind for ReactivePlanCheckpointInvariantError {
    fn name(&self) -> &str {
        "ReactivePlanCheckpointInvariant"
    }

    fn message(&self) -> String {
        format!(
            "Cannot checkpoint an invalid reactive plan: {}.",
            self.reason
        )
    }
}

#[derive(Clone)]
struct ReactiveFunctionIdentity {
    owner: Rc<()>,
}

impl Debug for ReactiveFunctionIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveFunctionIdentity")
            .finish_non_exhaustive()
    }
}

impl PartialEq for ReactiveFunctionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
    }
}

impl Eq for ReactiveFunctionIdentity {}

fn reactive_function_identity(function: &ReactivePlanFunction) -> ReactiveFunctionIdentity {
    ReactiveFunctionIdentity {
        owner: function.identity.clone(),
    }
}

impl ReactivePlan {
    fn rebuild_consumer_indexes(&mut self) {
        self.reactive_consumers.clear();
        self.sampled_consumers.clear();

        for node in &self.nodes {
            for dependency in &node.inputs {
                let consumers = match dependency.kind {
                    ReactiveDependencyKind::Reactive => &mut self.reactive_consumers,
                    ReactiveDependencyKind::Sampled => &mut self.sampled_consumers,
                };

                let consumers = consumers.entry(dependency.cell).or_default();

                if !consumers.contains(&node.id) {
                    consumers.push(node.id);
                }
            }
        }
    }

    pub fn validate_checkpoint_invariants(&self, activation_scope_count: usize) -> MResult<()> {
        let invalid = |reason: String| {
            MechError::new(ReactivePlanCheckpointInvariantError { reason }, None)
                .with_compiler_loc()
        };
        let node_len = self.nodes.len();

        for (index, node) in self.nodes.iter().enumerate() {
            if node.id != index {
                return Err(invalid(format!(
                    "node at index {} has id {}",
                    index, node.id,
                )));
            }
            if node.plan_index != index {
                return Err(invalid(format!(
                    "node {} has plan index {}",
                    node.id, node.plan_index,
                )));
            }
        }

        let mut reactive_consumers: HashMap<ReactiveCellId, Vec<ReactiveNodeId>> = HashMap::new();
        let mut sampled_consumers: HashMap<ReactiveCellId, Vec<ReactiveNodeId>> = HashMap::new();
        for node in &self.nodes {
            for dependency in &node.inputs {
                let consumers = match dependency.kind {
                    ReactiveDependencyKind::Reactive => &mut reactive_consumers,
                    ReactiveDependencyKind::Sampled => &mut sampled_consumers,
                };
                let consumers = consumers.entry(dependency.cell).or_insert_with(Vec::new);
                if !consumers.contains(&node.id) {
                    consumers.push(node.id);
                }
            }
        }
        for consumers in reactive_consumers.values_mut() {
            consumers.sort_unstable();
        }
        for consumers in sampled_consumers.values_mut() {
            consumers.sort_unstable();
        }
        let mut indexed_reactive_consumers = self.reactive_consumers.clone();
        let mut indexed_sampled_consumers = self.sampled_consumers.clone();
        for consumers in indexed_reactive_consumers.values_mut() {
            consumers.sort_unstable();
        }
        for consumers in indexed_sampled_consumers.values_mut() {
            consumers.sort_unstable();
        }
        if reactive_consumers != indexed_reactive_consumers
            || sampled_consumers != indexed_sampled_consumers
        {
            return Err(invalid(
                "consumer indexes do not match node dependencies".into(),
            ));
        }

        let valid_node = |node: ReactiveNodeId| node < node_len;
        for registration in &self.pattern_activation_registrations {
            if !valid_node(registration.scope_pulse_node) || !valid_node(registration.selector_node)
            {
                return Err(invalid(
                    "pattern activation references a missing root node".into(),
                ));
            }
            for arm in &registration.arms {
                if !valid_node(arm.matcher_node)
                    || !valid_node(arm.finalizer_node)
                    || !valid_node(arm.gate_node)
                    || arm.body_node_start > arm.body_node_end
                    || arm.body_node_end > node_len
                {
                    return Err(invalid("pattern activation arm topology is invalid".into()));
                }
                if let Some(guard) = &arm.guard {
                    if !valid_node(guard.match_gate_node)
                        || !valid_node(guard.guard_finalizer_node)
                        || guard.guard_node_start > guard.guard_node_end
                        || guard.guard_node_end > node_len
                    {
                        return Err(invalid(
                            "pattern activation guard topology is invalid".into(),
                        ));
                    }
                }
            }
        }

        if self.activation_sampled_cells.len() != activation_scope_count {
            return Err(invalid(format!(
                "sampled-cell stack depth {} differs from activation-scope depth {}",
                self.activation_sampled_cells.len(),
                activation_scope_count,
            )));
        }

        Ok(())
    }

    pub fn preflight_rollback(&self, checkpoint: &ReactivePlanCheckpoint) -> MResult<()> {
        if checkpoint.nodes.len() > self.nodes.len() {
            return Err(MechError::new(
                ReactivePlanRollbackInvariantError {
                    checkpoint_nodes: checkpoint.nodes.len(),
                    current_nodes: self.nodes.len(),
                    checkpoint_registrations: checkpoint.pattern_activation_registrations.len(),
                    current_registrations: self.pattern_activation_registrations.len(),
                },
                None,
            ));
        }

        for (node, saved) in self.nodes.iter().zip(checkpoint.nodes.iter()) {
            let current_identity = reactive_function_identity(&node.function);
            if current_identity != saved.function_identity {
                return Err(MechError::new(
                    ReactivePlanFunctionIdentityError { node_id: saved.id },
                    None,
                ));
            }
        }

        Ok(())
    }

    pub fn apply_rollback_structure(&mut self, checkpoint: &ReactivePlanCheckpoint) {
        self.nodes.truncate(checkpoint.nodes.len());
        for (node, saved) in self.nodes.iter_mut().zip(checkpoint.nodes.iter()) {
            node.id = saved.id;
            node.plan_index = saved.plan_index;
            node.inputs = saved.inputs.clone();
            node.outputs = saved.outputs.clone();
            node.kind = saved.kind;
        }
        self.pattern_activation_registrations = checkpoint.pattern_activation_registrations.clone();
        self.activation_sampled_cells = checkpoint.activation_sampled_cells.clone();
    }

    pub fn rebuild_checkpoint_indexes(&mut self) {
        self.rebuild_consumer_indexes();
    }

    pub fn apply_rollback(&mut self, checkpoint: &ReactivePlanCheckpoint) {
        self.apply_rollback_structure(checkpoint);
        self.rebuild_checkpoint_indexes();
    }

    pub fn checkpoint(&self) -> ReactivePlanCheckpoint {
        ReactivePlanCheckpoint {
            nodes: self
                .nodes
                .iter()
                .map(|node| ReactivePlanNodeCheckpoint {
                    id: node.id,
                    plan_index: node.plan_index,
                    inputs: node.inputs.clone(),
                    outputs: node.outputs.clone(),
                    kind: node.kind,
                    function_identity: reactive_function_identity(&node.function),
                })
                .collect(),
            pattern_activation_registrations: self.pattern_activation_registrations.clone(),
            activation_sampled_cells: self.activation_sampled_cells.clone(),
        }
    }

    pub fn rollback(&mut self, checkpoint: ReactivePlanCheckpoint) -> MResult<()> {
        self.preflight_rollback(&checkpoint)?;
        self.apply_rollback(&checkpoint);
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            reactive_consumers: HashMap::new(),
            sampled_consumers: HashMap::new(),
            pattern_activation_registrations: Vec::new(),
            activation_sampled_cells: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.reactive_consumers.clear();
        self.sampled_consumers.clear();
        self.pattern_activation_registrations.clear();
        self.activation_sampled_cells.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &ReactivePlanFunction> {
        self.nodes.iter().map(|node| &node.function)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ReactivePlanFunction> {
        self.nodes.iter_mut().map(|node| &mut node.function)
    }

    pub fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        let mut values = Vec::new();
        for node in &self.nodes {
            let mut function_values = node.function.transaction_state_values()?;
            if function_values.is_empty() {
                function_values.push(node.function.out());
            }
            values.extend(function_values);
        }
        Ok(values)
    }

    pub fn last(&self) -> Option<&ReactivePlanFunction> {
        self.nodes.last().map(|node| &node.function)
    }

    pub fn append(&mut self, functions: &mut Vec<Box<dyn MechFunction>>) {
        for function in functions.drain(..) {
            self.push(function);
        }
    }

    pub fn push(&mut self, function: Box<dyn MechFunction>) -> ReactiveNodeId {
        let node_id = self.nodes.len();
        let outputs = function.reactive_output_cell_ids();
        let node = ReactivePlanNode {
            id: node_id,
            plan_index: node_id,
            inputs: Vec::new(),
            outputs,
            kind: function.reactive_node_kind(),
            function: ReactivePlanFunction::new(function),
        };

        self.nodes.push(node);
        node_id
    }

    pub fn register(
        &mut self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
    ) -> MResult<ReactiveNodeId> {
        self.register_with_activation(function, arguments, None)
    }

    pub fn register_with_activation(
        &mut self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
        activation: Option<&ActivationRegistrationScope>,
    ) -> MResult<ReactiveNodeId> {
        let node_id = self.nodes.len();
        let plan_index = node_id;
        let function_description = function.to_string();

        let dependency_kinds = match function.reactive_dependency_kinds(arguments.len()) {
            Some(kinds) => {
                if kinds.len() != arguments.len() {
                    return Err(MechError::new(
                        ReactiveDependencyArityMismatchError {
                            function: function_description,
                            expected: arguments.len(),
                            found: kinds.len(),
                        },
                        None,
                    ));
                }
                kinds
            }
            None => vec![ReactiveDependencyKind::Reactive; arguments.len()],
        };

        let dependency_scopes = match function.reactive_dependency_scopes(arguments.len()) {
            Some(scopes) => {
                if scopes.len() != arguments.len() {
                    return Err(MechError::new(
                        ReactiveDependencyScopeArityMismatchError {
                            function: function_description,
                            expected: arguments.len(),
                            found: scopes.len(),
                        },
                        None,
                    ));
                }
                scopes
            }
            None => vec![ReactiveDependencyScope::Recursive; arguments.len()],
        };

        let node_kind = function.reactive_node_kind();
        let outputs = function.reactive_output_cell_ids();
        let mut inputs = Vec::<ReactiveDependency>::new();

        if node_kind == ReactiveNodeKind::Register {
            for cell in &outputs {
                inputs.push(ReactiveDependency {
                    cell: *cell,
                    kind: ReactiveDependencyKind::Sampled,
                });
            }
        }

        for ((argument, kind), scope) in arguments
            .iter()
            .zip(dependency_kinds.iter())
            .zip(dependency_scopes.iter())
        {
            let cells = match scope {
                ReactiveDependencyScope::Recursive => argument.reactive_cell_ids(),
                ReactiveDependencyScope::Logical => argument.logical_reactive_cell_ids(),
                ReactiveDependencyScope::Root => argument.reactive_root_cell_ids(),
                ReactiveDependencyScope::None => Vec::new(),
            };

            for cell in cells {
                let kind = activation.map_or(*kind, |scope| {
                    if scope.trigger_cells.contains(&cell)
                        || scope.local_combinational_cells.contains(&cell)
                    {
                        ReactiveDependencyKind::Reactive
                    } else {
                        ReactiveDependencyKind::Sampled
                    }
                });
                match inputs.iter().find(|dependency| dependency.cell == cell) {
                    Some(dependency) if dependency.kind == kind => {}
                    Some(dependency)
                        if node_kind == ReactiveNodeKind::Register
                            && outputs.contains(&cell)
                            && (dependency.kind == ReactiveDependencyKind::Sampled
                                || kind == ReactiveDependencyKind::Sampled) => {}
                    Some(_) => {
                        return Err(MechError::new(
                            ReactiveDependencyKindConflictError {
                                function: function_description,
                                cell,
                            },
                            None,
                        ));
                    }
                    None => inputs.push(ReactiveDependency { cell, kind }),
                }
            }
        }

        if let Some(scope) = activation {
            for cell in &scope.trigger_cells {
                if !inputs.iter().any(|dependency| dependency.cell == *cell) {
                    inputs.push(ReactiveDependency {
                        cell: *cell,
                        kind: ReactiveDependencyKind::Reactive,
                    });
                }
            }
        }

        let node = ReactivePlanNode {
            id: node_id,
            plan_index,
            inputs,
            outputs,
            kind: node_kind,
            function: ReactivePlanFunction::new(function),
        };

        self.nodes.push(node);

        for dependency in &self.nodes[node_id].inputs {
            let consumers = match dependency.kind {
                ReactiveDependencyKind::Reactive => {
                    self.reactive_consumers.entry(dependency.cell).or_default()
                }
                ReactiveDependencyKind::Sampled => {
                    self.sampled_consumers.entry(dependency.cell).or_default()
                }
            };

            if !consumers.contains(&node_id) {
                consumers.push(node_id);
            }
        }

        Ok(node_id)
    }

    pub fn node(&self, node_id: ReactiveNodeId) -> Option<&ReactivePlanNode> {
        self.nodes.get(node_id)
    }

    pub fn pattern_activation_registrations(&self) -> &[PatternActivationRegistration] {
        &self.pattern_activation_registrations
    }

    pub fn register_pattern_activation(&mut self, registration: PatternActivationRegistration) {
        self.pattern_activation_registrations.push(registration);
    }

    /// Records an input that is read when another reactive cause schedules the
    /// node. Updating this cell alone must not schedule the node.
    pub fn add_sampled_dependency(
        &mut self,
        node_id: ReactiveNodeId,
        cell: ReactiveCellId,
    ) -> bool {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return false;
        };
        if let Some(existing) = node
            .inputs
            .iter()
            .find(|dependency| dependency.cell == cell)
        {
            return existing.kind == ReactiveDependencyKind::Sampled
                || existing.kind == ReactiveDependencyKind::Reactive;
        }
        node.inputs.push(ReactiveDependency {
            cell,
            kind: ReactiveDependencyKind::Sampled,
        });
        let consumers = self.sampled_consumers.entry(cell).or_default();
        if !consumers.contains(&node_id) {
            consumers.push(node_id);
        }
        true
    }

    /// Records a cell that schedules this node. This is also used to repair
    /// legacy combinational expression nodes that were appended directly while
    /// an activation-registration scope was active.
    pub fn add_reactive_dependency(
        &mut self,
        node_id: ReactiveNodeId,
        cell: ReactiveCellId,
    ) -> bool {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return false;
        };
        if let Some(existing) = node
            .inputs
            .iter()
            .find(|dependency| dependency.cell == cell)
        {
            return existing.kind == ReactiveDependencyKind::Reactive;
        }
        node.inputs.push(ReactiveDependency {
            cell,
            kind: ReactiveDependencyKind::Reactive,
        });
        let consumers = self.reactive_consumers.entry(cell).or_default();
        if !consumers.contains(&node_id) {
            consumers.push(node_id);
        }
        true
    }

    pub fn reactive_consumers_for(&self, cell: ReactiveCellId) -> &[ReactiveNodeId] {
        self.reactive_consumers
            .get(&cell)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn sampled_consumers_for(&self, cell: ReactiveCellId) -> &[ReactiveNodeId] {
        self.sampled_consumers
            .get(&cell)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn solve_dirty_cells(
        &mut self,
        dirty_cells: &[ReactiveCellId],
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut services = NoMechExecutionServices;
        self.solve_dirty_cells_with_services(dirty_cells, &mut services)
    }

    pub fn solve_dirty_cells_with_services(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut outcome = ReactivePlanSolveOutcome::default();
        self.solve_dirty_cells_into_impl(dirty_cells, &mut outcome, None, services)?;
        Ok(outcome)
    }

    pub(crate) fn solve_dirty_cells_with_journal(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut services = NoMechExecutionServices;
        self.solve_dirty_cells_with_journal_and_services(dirty_cells, journal, &mut services)
    }

    pub(crate) fn solve_dirty_cells_with_journal_and_services(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        let mut outcome = ReactivePlanSolveOutcome::default();
        self.solve_dirty_cells_into_with_journal_and_services(
            dirty_cells,
            &mut outcome,
            journal,
            services,
        )?;
        Ok(outcome)
    }

    fn solve_dirty_cells_into(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        outcome: &mut ReactivePlanSolveOutcome,
    ) -> MResult<()> {
        let mut services = NoMechExecutionServices;
        self.solve_dirty_cells_into_impl(dirty_cells, outcome, None, &mut services)
    }

    fn solve_dirty_cells_into_with_journal(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        outcome: &mut ReactivePlanSolveOutcome,
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<()> {
        let mut services = NoMechExecutionServices;
        self.solve_dirty_cells_into_with_journal_and_services(
            dirty_cells,
            outcome,
            journal,
            &mut services,
        )
    }

    fn solve_dirty_cells_into_with_journal_and_services(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        outcome: &mut ReactivePlanSolveOutcome,
        journal: &mut ReactiveTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        self.solve_dirty_cells_into_impl(dirty_cells, outcome, Some(journal), services)
    }

    fn solve_dirty_cells_into_impl(
        &mut self,
        dirty_cells: &[ReactiveCellId],
        outcome: &mut ReactivePlanSolveOutcome,
        mut journal: Option<&mut ReactiveTurnJournal>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        let dirty_cells = dirty_cells.iter().copied().collect::<HashSet<_>>();
        let mut work = BTreeSet::new();
        let mut processed = BTreeSet::new();

        for cell in dirty_cells.iter().copied() {
            for node_id in self.reactive_consumers_for(cell) {
                let node = &self.nodes[*node_id];
                work.insert((node.plan_index, node.id));
            }
        }

        while let Some((_, node_id)) = work.pop_first() {
            if !processed.insert(node_id) {
                continue;
            }

            let node = &self.nodes[node_id];
            if node.kind == ReactiveNodeKind::Register {
                outcome.pending_register_nodes.push(node.id);
                continue;
            }

            if let Some(journal) = journal.as_deref_mut() {
                journal.capture_function_state(node.function.as_ref())?;
            }
            let status = node.function.solve_reactive_with(services)?;
            outcome.executed_nodes.push(node.id);
            match status {
                ReactiveSolveStatus::Changed => {
                    outcome.changed_nodes.push(node.id);
                    let outputs = node.outputs.clone();
                    for cell in outputs {
                        for consumer_id in self.reactive_consumers_for(cell) {
                            let consumer = &self.nodes[*consumer_id];
                            work.insert((consumer.plan_index, consumer.id));
                        }
                    }
                }
                ReactiveSolveStatus::Unchanged => outcome.unchanged_nodes.push(node.id),
            }
        }

        Ok(())
    }

    pub fn commit_pending_registers(
        &mut self,
        pending_nodes: &[ReactiveNodeId],
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.commit_pending_registers_impl(pending_nodes, None)
    }

    pub(crate) fn commit_pending_registers_with_journal(
        &mut self,
        pending_nodes: &[ReactiveNodeId],
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.commit_pending_registers_impl(pending_nodes, Some(journal))
    }

    fn commit_pending_registers_impl(
        &mut self,
        pending_nodes: &[ReactiveNodeId],
        mut journal: Option<&mut ReactiveTurnJournal>,
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        let mut unique = HashSet::new();
        let mut ordered = BTreeSet::new();
        for node_id in pending_nodes.iter().copied() {
            if !unique.insert(node_id) {
                continue;
            }
            let node = self.nodes.get(node_id).ok_or_else(|| {
                MechError::new(ReactiveRegisterNodeNotFoundError { node_id }, None)
            })?;
            if node.kind != ReactiveNodeKind::Register {
                return Err(MechError::new(
                    ReactiveRegisterNodeKindError {
                        node_id,
                        actual: node.kind,
                    },
                    None,
                ));
            }
            ordered.insert((node.plan_index, node.id));
        }

        let mut owners = HashMap::new();
        for (_, node_id) in &ordered {
            let node = &self.nodes[*node_id];
            for cell in &node.outputs {
                if let Some(first_node) = owners.insert(*cell, node.id) {
                    return Err(MechError::new(
                        ReactiveRegisterOutputConflictError {
                            cell: *cell,
                            first_node,
                            second_node: node.id,
                        },
                        None,
                    ));
                }
            }
        }

        if let Some(journal) = journal.as_deref_mut() {
            for (_, node_id) in &ordered {
                journal.capture_function_state(self.nodes[*node_id].function.as_ref())?;
            }
        }

        let mut staged: Vec<(ReactiveNodeId, Box<dyn ReactiveRegisterCommit>)> = Vec::new();
        for (_, node_id) in &ordered {
            let node = &self.nodes[*node_id];
            let commit = node.function.stage_register()?;
            let found = commit.output_cells().to_vec();
            if found != node.outputs {
                return Err(MechError::new(
                    ReactiveRegisterStagedOutputMismatchError {
                        node_id: node.id,
                        expected: node.outputs.clone(),
                        found,
                    },
                    None,
                ));
            }
            staged.push((node.id, commit));
        }

        let staged_nodes = staged.iter().map(|(id, _)| *id).collect();
        let mut outcome = ReactiveRegisterCommitOutcome {
            staged_nodes,
            ..Default::default()
        };
        for (node_id, commit) in staged {
            let outputs = commit.output_cells().to_vec();
            commit.commit();
            outcome.committed_nodes.push(node_id);
            for cell in outputs {
                if !outcome.dirty_cells.contains(&cell) {
                    outcome.dirty_cells.push(cell);
                }
            }
        }
        Ok(outcome)
    }
    /// Advances one synchronous reactive turn using this existing plan.
    ///
    /// Pre-commit and staging failures occur before register mutation. Post-commit
    /// propagation failures occur after the atomic register batch has been committed
    /// and are therefore not rolled back.
    pub fn advance_reactive_turn(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
    ) -> MResult<ReactiveTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.advance_reactive_turn_with_services(state, dirty_cells, &mut services)
    }

    pub fn advance_reactive_turn_with_services(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        let before_commit = self.solve_dirty_cells_with_services(dirty_cells, services)?;
        let mut pending_register_nodes = std::mem::take(&mut state.pending_register_nodes);
        pending_register_nodes.extend(before_commit.pending_register_nodes.iter().copied());
        let register_commit = match self.commit_pending_registers(&pending_register_nodes) {
            Ok(outcome) => outcome,
            Err(error) => {
                state.pending_register_nodes = pending_register_nodes;
                return Err(error);
            }
        };
        state.pending_register_nodes.clear();
        let mut after_commit = ReactivePlanSolveOutcome::default();
        if let Err(error) = self.solve_dirty_cells_into_impl(
            &register_commit.dirty_cells,
            &mut after_commit,
            None,
            services,
        ) {
            state.pending_register_nodes = after_commit.pending_register_nodes;
            return Err(error);
        }
        state.pending_register_nodes = after_commit.pending_register_nodes.clone();
        Ok(ReactiveTurnOutcome {
            before_commit,
            register_commit,
            after_commit,
        })
    }

    pub(crate) fn advance_reactive_turn_with_journal(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<ReactiveTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.advance_reactive_turn_with_journal_and_services(
            state,
            dirty_cells,
            journal,
            &mut services,
        )
    }

    pub(crate) fn advance_reactive_turn_with_journal_and_services(
        &mut self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        let before_commit =
            self.solve_dirty_cells_with_journal_and_services(dirty_cells, journal, services)?;
        let mut pending_register_nodes = std::mem::take(&mut state.pending_register_nodes);
        pending_register_nodes.extend(before_commit.pending_register_nodes.iter().copied());
        let register_commit =
            match self.commit_pending_registers_with_journal(&pending_register_nodes, journal) {
                Ok(outcome) => outcome,
                Err(error) => {
                    state.pending_register_nodes = pending_register_nodes;
                    return Err(error);
                }
            };
        state.pending_register_nodes.clear();
        let mut after_commit = ReactivePlanSolveOutcome::default();
        if let Err(error) = self.solve_dirty_cells_into_with_journal_and_services(
            &register_commit.dirty_cells,
            &mut after_commit,
            journal,
            services,
        ) {
            state.pending_register_nodes = after_commit.pending_register_nodes;
            return Err(error);
        }
        state.pending_register_nodes = after_commit.pending_register_nodes.clone();
        Ok(ReactiveTurnOutcome {
            before_commit,
            register_commit,
            after_commit,
        })
    }
}

impl core::ops::Index<usize> for ReactivePlan {
    type Output = ReactivePlanFunction;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index].function
    }
}

impl core::ops::IndexMut<usize> for ReactivePlan {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.nodes[index].function
    }
}

pub struct Plan(
    pub Ref<ReactivePlan>,
    pub Ref<Vec<ActivationRegistrationScope>>,
);

impl Clone for Plan {
    fn clone(&self) -> Self {
        Plan(self.0.clone(), self.1.clone())
    }
}

impl fmt::Debug for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for p in self.0.borrow().iter() {
            writeln!(f, "{}", p.to_string())?;
        }
        Ok(())
    }
}

impl Plan {
    fn checkpoint_borrow_conflict(phase: &'static str, component: &'static str) -> MechError {
        MechError::new(PlanCheckpointBorrowConflictError { phase, component }, None)
            .with_compiler_loc()
    }

    pub fn checkpoint(&self) -> PlanCheckpoint {
        PlanCheckpoint {
            reactive: self.0.borrow().checkpoint(),
            activation_registration_scopes: self.1.borrow().clone(),
        }
    }

    /// Fallibly captures a structurally valid checkpoint without panicking on
    /// outstanding plan borrows.
    pub fn try_checkpoint(&self) -> MResult<PlanCheckpoint> {
        let reactive = self
            .0
            .try_borrow()
            .map_err(|_| Self::checkpoint_borrow_conflict("capture", "reactive graph"))?;
        let scopes = self
            .1
            .try_borrow()
            .map_err(|_| Self::checkpoint_borrow_conflict("capture", "activation scopes"))?;
        reactive.validate_checkpoint_invariants(scopes.len())?;
        Ok(PlanCheckpoint {
            reactive: reactive.checkpoint(),
            activation_registration_scopes: scopes.clone(),
        })
    }

    pub fn validate_checkpoint_invariants(&self) -> MResult<()> {
        let reactive = self.0.try_borrow().map_err(|_| {
            Self::checkpoint_borrow_conflict("checkpoint-validation", "reactive graph")
        })?;
        let scopes = self.1.try_borrow().map_err(|_| {
            Self::checkpoint_borrow_conflict("checkpoint-validation", "activation scopes")
        })?;
        reactive.validate_checkpoint_invariants(scopes.len())
    }

    pub fn validate_checkpoint_turn_state(&self, state: &ReactiveTurnState) -> MResult<()> {
        let reactive = self.0.try_borrow().map_err(|_| {
            Self::checkpoint_borrow_conflict("checkpoint-validation", "reactive graph")
        })?;
        for node_id in &state.pending_register_nodes {
            let Some(node) = reactive.nodes.get(*node_id) else {
                return Err(MechError::new(
                    ReactiveTurnCheckpointInvariantError {
                        node_id: *node_id,
                        reason: "the node does not exist".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            if node.kind != ReactiveNodeKind::Register {
                return Err(MechError::new(
                    ReactiveTurnCheckpointInvariantError {
                        node_id: *node_id,
                        reason: "the node is not a register".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
        Ok(())
    }

    pub fn preflight_rollback(&self, checkpoint: &PlanCheckpoint) -> MResult<()> {
        let reactive = self
            .0
            .try_borrow_mut()
            .map_err(|_| Self::checkpoint_borrow_conflict("restore", "reactive graph"))?;
        let _scopes = self
            .1
            .try_borrow_mut()
            .map_err(|_| Self::checkpoint_borrow_conflict("restore", "activation scopes"))?;
        reactive.preflight_rollback(&checkpoint.reactive)
    }

    pub fn apply_rollback_structure(&self, checkpoint: &PlanCheckpoint) {
        let mut reactive = self.0.borrow_mut();
        let mut scopes = self.1.borrow_mut();
        reactive.apply_rollback_structure(&checkpoint.reactive);
        *scopes = checkpoint.activation_registration_scopes.clone();
    }

    pub fn rebuild_checkpoint_indexes(&self) {
        self.0.borrow_mut().rebuild_checkpoint_indexes();
    }

    pub fn apply_rollback(&self, checkpoint: &PlanCheckpoint) {
        self.apply_rollback_structure(checkpoint);
        self.rebuild_checkpoint_indexes();
    }

    pub fn rollback(&self, checkpoint: PlanCheckpoint) -> MResult<()> {
        self.preflight_rollback(&checkpoint)?;
        self.apply_rollback(&checkpoint);
        Ok(())
    }

    pub fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        let reactive = self
            .0
            .try_borrow()
            .map_err(|_| Self::checkpoint_borrow_conflict("transaction-state", "reactive graph"))?;
        reactive.transaction_state_values()
    }

    pub fn activation_registration_depth(&self) -> usize {
        self.1.borrow().len()
    }

    pub fn new() -> Self {
        Self(Ref::new(ReactivePlan::new()), Ref::new(Vec::new()))
    }

    pub fn borrow(&self) -> std::cell::Ref<'_, ReactivePlan> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, ReactivePlan> {
        self.0.borrow_mut()
    }

    pub fn add_function(&self, function: Box<dyn MechFunction>) -> ReactiveNodeId {
        self.0.borrow_mut().push(function)
    }

    pub fn activation_registration_active(&self) -> bool {
        !self.1.borrow().is_empty()
    }
    pub fn push_activation_registration_scope(&self, trigger_cells: Vec<ReactiveCellId>) {
        self.push_activation_registration_scope_with_sampled_cells(trigger_cells, Vec::new());
    }
    pub fn push_activation_registration_scope_with_sampled_cells(
        &self,
        trigger_cells: Vec<ReactiveCellId>,
        sampled_cells: Vec<ReactiveCellId>,
    ) {
        self.1.borrow_mut().push(ActivationRegistrationScope {
            trigger_cells,
            local_combinational_cells: Vec::new(),
        });
        self.0
            .borrow_mut()
            .activation_sampled_cells
            .push(sampled_cells);
    }
    pub fn pop_activation_registration_scope(&self) {
        self.1.borrow_mut().pop();
        self.0.borrow_mut().activation_sampled_cells.pop();
    }
    pub fn register_function(
        &self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
    ) -> MResult<ReactiveNodeId> {
        let scope = self.1.borrow().last().cloned();
        let kind = function.reactive_node_kind();
        let outputs = function.reactive_output_cell_ids();
        let sampled_cells = self
            .0
            .borrow()
            .activation_sampled_cells
            .last()
            .cloned()
            .unwrap_or_default();
        let node =
            self.0
                .borrow_mut()
                .register_with_activation(function, arguments, scope.as_ref())?;
        if scope.is_some() && kind == ReactiveNodeKind::Combinational {
            if let Some(active) = self.1.borrow_mut().last_mut() {
                for cell in outputs {
                    if !sampled_cells.contains(&cell)
                        && !active.local_combinational_cells.contains(&cell)
                    {
                        active.local_combinational_cells.push(cell);
                    }
                }
            }
        }
        Ok(node)
    }

    pub fn solve_dirty_cells(
        &self,
        dirty_cells: &[ReactiveCellId],
    ) -> MResult<ReactivePlanSolveOutcome> {
        self.0.borrow_mut().solve_dirty_cells(dirty_cells)
    }
    pub fn solve_dirty_cells_with_services(
        &self,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        self.0
            .borrow_mut()
            .solve_dirty_cells_with_services(dirty_cells, services)
    }

    pub(crate) fn solve_dirty_cells_with_journal(
        &self,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<ReactivePlanSolveOutcome> {
        self.0
            .borrow_mut()
            .solve_dirty_cells_with_journal(dirty_cells, journal)
    }
    pub(crate) fn solve_dirty_cells_with_journal_and_services(
        &self,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactivePlanSolveOutcome> {
        self.0
            .borrow_mut()
            .solve_dirty_cells_with_journal_and_services(dirty_cells, journal, services)
    }

    pub fn commit_pending_registers(
        &self,
        pending_nodes: &[ReactiveNodeId],
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.0.borrow_mut().commit_pending_registers(pending_nodes)
    }
    pub(crate) fn commit_pending_registers_with_journal(
        &self,
        pending_nodes: &[ReactiveNodeId],
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<ReactiveRegisterCommitOutcome> {
        self.0
            .borrow_mut()
            .commit_pending_registers_with_journal(pending_nodes, journal)
    }
    pub fn advance_reactive_turn(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn(state, dirty_cells)
    }
    pub fn advance_reactive_turn_with_services(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn_with_services(state, dirty_cells, services)
    }
    pub(crate) fn advance_reactive_turn_with_journal(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn_with_journal(state, dirty_cells, journal)
    }
    pub(crate) fn advance_reactive_turn_with_journal_and_services(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ReactiveTurnJournal,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn_with_journal_and_services(state, dirty_cells, journal, services)
    }

    pub fn advance_reactive_turn_participating(
        &self,
        state: &mut ReactiveTurnState,
        dirty_cells: &[ReactiveCellId],
        participant: &mut ReactiveJournalParticipant<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        self.0
            .borrow_mut()
            .advance_reactive_turn_with_journal_and_services(
                state,
                dirty_cells,
                participant.journal_mut(),
                services,
            )
    }

    pub fn get_functions(&self) -> std::cell::Ref<'_, ReactivePlan> {
        self.0.borrow()
    }

    pub fn pattern_activation_registrations(
        &self,
    ) -> std::cell::Ref<'_, Vec<PatternActivationRegistration>> {
        std::cell::Ref::map(self.0.borrow(), |plan| {
            &plan.pattern_activation_registrations
        })
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Plan {
    fn pretty_print(&self) -> String {
        let mut builder = Builder::default();
        let plan_brrw = self.0.borrow();

        if self.is_empty() {
            builder.push_record(vec!["".to_string()]);
        } else {
            let total = plan_brrw.len();
            let mut display_fxns: Vec<String> = Vec::new();

            let indices: Vec<usize> = if total > 30 {
                (0..10).chain((total - 10)..total).collect()
            } else {
                (0..total).collect()
            };

            for &ix in &indices {
                let fxn_str = plan_brrw[ix].to_string();
                let lines: Vec<&str> = fxn_str.lines().collect();

                let truncated = if lines.len() > 20 {
                    let mut t = Vec::new();
                    t.extend_from_slice(&lines[..10]);
                    t.push("…");
                    t.extend_from_slice(&lines[lines.len() - 10..]);
                    t.join("\n")
                } else {
                    lines.join("\n")
                };

                display_fxns.push(format!("{}. {}", ix + 1, truncated));
            }

            if total > 30 {
                display_fxns.insert(10, "…".to_string());
            }

            let mut row: Vec<String> = Vec::new();
            for plan_str in display_fxns {
                row.push(plan_str);
                if row.len() == 4 {
                    builder.push_record(row.clone());
                    row.clear();
                }
            }
            if !row.is_empty() {
                while row.len() < 4 {
                    row.push("".to_string());
                }
                builder.push_record(row);
            }
        }

        let mut table = builder.build();
        table
            .with(Style::modern_rounded())
            .with(Panel::header("📋 Plan"));

        format!("{table}")
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind1 {
    pub arg: ValueKind,
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind1 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind1"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kind for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind2 {
    pub arg: (ValueKind, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind2 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind2"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind3 {
    pub arg: (ValueKind, ValueKind, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind3 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind3"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind4 {
    pub arg: (ValueKind, ValueKind, ValueKind, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind4 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind4"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKindVarg {
    pub arg: Vec<ValueKind>,
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKindVarg {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKindVarg"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentIxes {
    pub arg: (ValueKind, Vec<ValueKind>, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentIxes {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentIxes"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentIxesMono {
    pub arg: (ValueKind, Vec<ValueKind>),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentIxesMono {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentIxesMono"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct IncorrectNumberOfArguments {
    pub expected: usize,
    pub found: usize,
}
impl MechErrorKind for IncorrectNumberOfArguments {
    fn name(&self) -> &str {
        "IncorrectNumberOfArguments"
    }

    fn message(&self) -> String {
        format!(
            "Expected {} arguments, but found {}",
            self.expected, self.found
        )
    }
}
