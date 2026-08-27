use crate::{
    LegacyValue, MResult, MechError, MechErrorKind, MechFunction, ValueCell, ValueStateJournal,
};
use std::cell::Cell;

/// The value-state portion of one ephemeral reactive turn.
///
/// This journal deliberately contains only before-state. It is consumed by
/// higher-level rollback coordinators and never becomes durable history.
pub(crate) struct ReactiveTurnJournal {
    values: ValueStateJournal,
}

impl ReactiveTurnJournal {
    pub(crate) fn new() -> Self {
        Self {
            values: ValueStateJournal::new(),
        }
    }

    pub(crate) fn capture_value(&mut self, value: &LegacyValue) -> MResult<()> {
        self.values.capture_value(value)
    }

    pub(crate) fn capture_value_cell(&mut self, value: &ValueCell) -> MResult<()> {
        self.values.capture_value_cell(value)
    }

    pub(crate) fn capture_function_state(&mut self, function: &dyn MechFunction) -> MResult<()> {
        if let Some(ports) = function.transaction_state_ports()? {
            for port in ports {
                port.capture_into(&mut self.values)?;
            }
            return Ok(());
        }

        let mut values = function.transaction_state_values()?;
        if values.is_empty() {
            values.push(function.out());
        }
        for value in values {
            self.values.capture_value(&value)?;
        }
        Ok(())
    }

    pub(crate) fn preflight_restore_before(&self) -> MResult<()> {
        self.values.preflight_restore_before()
    }

    pub(crate) fn apply_restore_before(&self) {
        self.values.apply_restore_before();
    }

    pub(crate) fn restore_before(&self) -> MResult<()> {
        self.values.restore_before()
    }

    pub(crate) fn cell_count(&self) -> usize {
        self.values.cell_count()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Default for ReactiveTurnJournal {
    fn default() -> Self {
        Self::new()
    }
}

/// A lifetime-bound capability for participating in one coordinated reactive
/// operation.
///
/// The value journal and constructor are private. Callers can receive this
/// capability only inside [`with_reactive_journal_participant`], which restores
/// captured values if the operation exits without explicit finalization.
pub struct ReactiveJournalParticipant<'journal> {
    journal: &'journal mut ReactiveTurnJournal,
    finalization: &'journal Cell<ReactiveJournalFinalizationState>,
}

impl ReactiveJournalParticipant<'_> {
    pub fn capture_value(&mut self, value: &LegacyValue) -> MResult<()> {
        self.journal.capture_value(value)
    }

    pub fn capture_value_cell(&mut self, value: &ValueCell) -> MResult<()> {
        self.journal.capture_value_cell(value)
    }

    pub fn capture_function_state(&mut self, function: &dyn MechFunction) -> MResult<()> {
        self.journal.capture_function_state(function)
    }

    pub fn preflight_restore_before(&self) -> MResult<()> {
        self.journal.preflight_restore_before()
    }

    pub fn apply_restore_before(self) {
        self.journal.apply_restore_before();
        self.finalization
            .set(ReactiveJournalFinalizationState::RolledBack);
    }

    pub fn commit(self) {
        self.finalization
            .set(ReactiveJournalFinalizationState::Committed);
    }

    pub fn cell_count(&self) -> usize {
        self.journal.cell_count()
    }

    pub fn is_empty(&self) -> bool {
        self.journal.is_empty()
    }

    pub(crate) fn journal_mut(&mut self) -> &mut ReactiveTurnJournal {
        self.journal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReactiveJournalFinalizationState {
    Pending,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactiveJournalFinalizationMissing;

impl MechErrorKind for ReactiveJournalFinalizationMissing {
    fn name(&self) -> &str {
        "ReactiveJournalFinalizationMissing"
    }

    fn message(&self) -> String {
        "A reactive journal participant returned success without finalization; captured values were rolled back."
      .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactiveJournalAutomaticRollbackFailed {
    pub original_error: Option<String>,
    pub rollback_error: String,
}

impl MechErrorKind for ReactiveJournalAutomaticRollbackFailed {
    fn name(&self) -> &str {
        "ReactiveJournalAutomaticRollbackFailed"
    }

    fn message(&self) -> String {
        match &self.original_error {
            Some(original_error) => format!(
                "Reactive journal coordination failed with {}; automatic rollback also failed with {}.",
                original_error, self.rollback_error,
            ),
            None => format!(
                "A reactive journal participant returned without finalization and automatic rollback failed with {}.",
                self.rollback_error,
            ),
        }
    }
}

/// Runs one operation with an opaque reactive-journal participant.
///
/// The participant cannot escape this callback. An error path that has not
/// already been finalized is rolled back automatically. A successful path must
/// explicitly commit or apply rollback; otherwise it is rolled back and
/// reported as a coordination error.
pub fn with_reactive_journal_participant<T>(
    operation: impl FnOnce(ReactiveJournalParticipant<'_>) -> MResult<T>,
) -> MResult<T> {
    let mut journal = ReactiveTurnJournal::new();
    let finalization = Cell::new(ReactiveJournalFinalizationState::Pending);
    let participant = ReactiveJournalParticipant {
        journal: &mut journal,
        finalization: &finalization,
    };
    let result = operation(participant);
    match finalization.get() {
        ReactiveJournalFinalizationState::Committed
        | ReactiveJournalFinalizationState::RolledBack => result,
        ReactiveJournalFinalizationState::Pending => match journal.restore_before() {
            Ok(()) => match result {
                Ok(_) => Err(MechError::new(ReactiveJournalFinalizationMissing, None)),
                Err(error) => Err(error),
            },
            Err(rollback_error) => Err(MechError::new(
                ReactiveJournalAutomaticRollbackFailed {
                    original_error: result.as_ref().err().map(|error| format!("{:?}", error)),
                    rollback_error: format!("{:?}", rollback_error),
                },
                None,
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    fn deliberate_journal_error(message: &'static str) -> MechError {
        MechError::new(
            GenericError {
                msg: message.to_string(),
            },
            None,
        )
    }

    #[test]
    fn reactive_journal_pending_error_restores_and_returns_original_error() {
        let value = Ref::new(1usize);

        let error = with_reactive_journal_participant::<()>(|mut participant| {
            participant.capture_value(&LegacyValue::Index(value.clone()))?;
            *value.borrow_mut() = 2;
            Err(deliberate_journal_error("deliberate pending journal error"))
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert!(
            error
                .kind_message()
                .contains("deliberate pending journal error"),
        );
        assert_eq!(*value.borrow(), 1);
    }

    #[test]
    fn reactive_journal_pending_success_restores_and_reports_missing_finalization() {
        let value = Ref::new(1usize);

        let error = with_reactive_journal_participant(|mut participant| {
            participant.capture_value(&LegacyValue::Index(value.clone()))?;
            *value.borrow_mut() = 2;
            Ok(())
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "ReactiveJournalFinalizationMissing",);
        assert_eq!(*value.borrow(), 1);
    }

    #[test]
    fn reactive_journal_explicit_commit_retains_mutation() {
        let value = Ref::new(1usize);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_value(&LegacyValue::Index(value.clone()))?;
            *value.borrow_mut() = 2;
            participant.commit();
            Ok(())
        })
        .unwrap();

        assert_eq!(*value.borrow(), 2);
    }

    #[test]
    fn reactive_journal_commit_with_error_retains_mutation_and_error() {
        let value = Ref::new(1usize);

        let error = with_reactive_journal_participant::<()>(|mut participant| {
            participant.capture_value(&LegacyValue::Index(value.clone()))?;
            *value.borrow_mut() = 2;
            participant.commit();
            Err(deliberate_journal_error(
                "deliberate committed journal error",
            ))
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert!(
            error
                .kind_message()
                .contains("deliberate committed journal error"),
        );
        assert_eq!(*value.borrow(), 2);
    }

    #[test]
    fn reactive_journal_explicit_rollback_restores_and_returns_original_error() {
        let value = Ref::new(1usize);

        let error = with_reactive_journal_participant::<()>(|mut participant| {
            participant.capture_value(&LegacyValue::Index(value.clone()))?;
            *value.borrow_mut() = 2;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Err(deliberate_journal_error(
                "deliberate rolled-back journal error",
            ))
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert!(
            error
                .kind_message()
                .contains("deliberate rolled-back journal error"),
        );
        assert_eq!(*value.borrow(), 1);
    }

    #[test]
    fn reactive_journal_finalization_states_are_distinct() {
        assert_ne!(
            ReactiveJournalFinalizationState::Pending,
            ReactiveJournalFinalizationState::Committed,
        );
        assert_ne!(
            ReactiveJournalFinalizationState::Pending,
            ReactiveJournalFinalizationState::RolledBack,
        );
        assert_ne!(
            ReactiveJournalFinalizationState::Committed,
            ReactiveJournalFinalizationState::RolledBack,
        );
    }

    struct JournalFunction {
        name: &'static str,
        output: Ref<usize>,
        retained: Ref<usize>,
        events: Rc<RefCell<Vec<String>>>,
        capture_error: bool,
        solve_error: bool,
        sampled: bool,
    }

    impl MechFunctionImpl for JournalFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
            self.events
                .borrow_mut()
                .push(format!("solve {}", self.name));
            *self.output.borrow_mut() += 1;
            *self.retained.borrow_mut() += 10;
            if self.solve_error {
                return Err(MechError::new(
                    GenericError {
                        msg: format!("{} solve failed", self.name),
                    },
                    None,
                ));
            }
            Ok(ReactiveSolveStatus::Changed)
        }

        fn out(&self) -> LegacyValue {
            LegacyValue::Index(self.output.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            self.events
                .borrow_mut()
                .push(format!("capture {}", self.name));
            if self.capture_error {
                return Err(MechError::new(
                    TransactionStateUnsupportedError {
                        function: self.name.into(),
                        reason: "deliberate unsupported state".into(),
                    },
                    None,
                ));
            }
            Ok(vec![
                LegacyValue::Index(self.output.clone()),
                LegacyValue::Index(self.retained.clone()),
            ])
        }

        fn reactive_dependency_kinds(
            &self,
            argument_count: usize,
        ) -> Option<Vec<ReactiveDependencyKind>> {
            if self.sampled {
                Some(vec![ReactiveDependencyKind::Sampled; argument_count])
            } else {
                None
            }
        }

        fn to_string(&self) -> String {
            self.name.into()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for JournalFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn journal_function(
        name: &'static str,
        output: Ref<usize>,
        retained: Ref<usize>,
        events: Rc<RefCell<Vec<String>>>,
    ) -> JournalFunction {
        JournalFunction {
            name,
            output,
            retained,
            events,
            capture_error: false,
            solve_error: false,
            sampled: false,
        }
    }

    #[derive(Clone, Copy)]
    enum TypedStateMode {
        State,
        Empty,
        Error,
    }

    struct TypedJournalFunction {
        output: Ref<usize>,
        retained: Ref<usize>,
        mode: TypedStateMode,
        out_calls: Rc<Cell<usize>>,
        legacy_state_calls: Rc<Cell<usize>>,
        solve_calls: Rc<Cell<usize>>,
    }

    impl MechFunctionImpl for TypedJournalFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
            self.solve_calls.set(self.solve_calls.get() + 1);
            *self.output.borrow_mut() += 1;
            *self.retained.borrow_mut() += 10;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.output))
        }

        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
            match self.mode {
                TypedStateMode::State => {
                    let output = FunctionStatePort::from_ref(&self.output);
                    let retained = FunctionStatePort::from_ref(&self.retained);
                    Ok(Some(vec![output, retained, retained]))
                }
                TypedStateMode::Empty => Ok(Some(Vec::new())),
                TypedStateMode::Error => Err(MechError::new(
                    TransactionStateUnsupportedError {
                        function: "typed-journal".into(),
                        reason: "deliberate typed state error".into(),
                    },
                    None,
                )),
            }
        }

        fn out(&self) -> LegacyValue {
            self.out_calls.set(self.out_calls.get() + 1);
            LegacyValue::Index(self.output.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            self.legacy_state_calls
                .set(self.legacy_state_calls.get() + 1);
            Err(MechError::new(
                GenericError {
                    msg: "legacy typed-state fallback was invoked".into(),
                },
                None,
            ))
        }

        fn to_string(&self) -> String {
            "typed-journal".into()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for TypedJournalFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn typed_journal_function(
        mode: TypedStateMode,
    ) -> (
        TypedJournalFunction,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
    ) {
        let out_calls = Rc::new(Cell::new(0));
        let legacy_state_calls = Rc::new(Cell::new(0));
        let solve_calls = Rc::new(Cell::new(0));
        (
            TypedJournalFunction {
                output: Ref::new(1),
                retained: Ref::new(2),
                mode,
                out_calls: out_calls.clone(),
                legacy_state_calls: legacy_state_calls.clone(),
                solve_calls: solve_calls.clone(),
            },
            out_calls,
            legacy_state_calls,
            solve_calls,
        )
    }

    struct JournalRegisterCommit {
        sink: Ref<usize>,
        next: usize,
        outputs: Vec<ReactiveCellId>,
    }

    impl crate::function::reactive_register_sealed::Sealed for JournalRegisterCommit {}

    impl ReactiveRegisterCommit for JournalRegisterCommit {
        fn output_cells(&self) -> &[ReactiveCellId] {
            &self.outputs
        }

        fn commit(self: Box<Self>) {
            *self.sink.borrow_mut() = self.next;
        }
    }

    struct JournalRegister {
        name: &'static str,
        source: Ref<usize>,
        sink: Ref<usize>,
        retained: Ref<usize>,
        events: Rc<RefCell<Vec<String>>>,
        fail_stage: bool,
    }

    impl MechFunctionImpl for JournalRegister {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            LegacyValue::Index(self.sink.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            self.events
                .borrow_mut()
                .push(format!("capture {}", self.name));
            Ok(vec![
                LegacyValue::Index(self.sink.clone()),
                LegacyValue::Index(self.retained.clone()),
            ])
        }

        fn reactive_node_kind(&self) -> ReactiveNodeKind {
            ReactiveNodeKind::Register
        }

        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
            self.events
                .borrow_mut()
                .push(format!("stage {}", self.name));
            *self.retained.borrow_mut() += 1;
            if self.fail_stage {
                return Err(MechError::new(
                    GenericError {
                        msg: format!("{} stage failed", self.name),
                    },
                    None,
                ));
            }
            Ok(Box::new(JournalRegisterCommit {
                sink: self.sink.clone(),
                next: *self.source.borrow(),
                outputs: vec![ReactiveCellId::new(self.sink.id())],
            }))
        }

        fn to_string(&self) -> String {
            self.name.into()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for JournalRegister {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[test]
    fn reactive_transaction_captures_only_scheduled_combinational_nodes() {
        let scheduled_input = Ref::new(1usize);
        let unrelated_input = Ref::new(2usize);
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut plan = ReactivePlan::new();
        plan.register(
            Box::new(journal_function(
                "scheduled",
                Ref::new(0),
                Ref::new(0),
                events.clone(),
            )),
            &[LegacyValue::Index(scheduled_input.clone())],
        )
        .unwrap();
        plan.register(
            Box::new(journal_function(
                "unrelated",
                Ref::new(0),
                Ref::new(0),
                events.clone(),
            )),
            &[LegacyValue::Index(unrelated_input)],
        )
        .unwrap();

        let mut journal = ReactiveTurnJournal::new();
        plan.solve_dirty_cells_with_journal(
            &LegacyValue::Index(scheduled_input).reactive_root_cell_ids(),
            &mut journal,
        )
        .unwrap();

        assert_eq!(
            events.borrow().as_slice(),
            &["capture scheduled", "solve scheduled"],
        );
    }

    #[test]
    fn reactive_transaction_sampled_only_consumer_is_not_captured() {
        let input = Ref::new(1usize);
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut function = journal_function("sampled", Ref::new(0), Ref::new(0), events.clone());
        function.sampled = true;
        let mut plan = ReactivePlan::new();
        plan.register(Box::new(function), &[LegacyValue::Index(input.clone())])
            .unwrap();

        let mut journal = ReactiveTurnJournal::new();
        let outcome = plan
            .solve_dirty_cells_with_journal(
                &LegacyValue::Index(input).reactive_root_cell_ids(),
                &mut journal,
            )
            .unwrap();

        assert!(outcome.executed_nodes.is_empty());
        assert!(events.borrow().is_empty());
        assert!(journal.is_empty());
    }

    #[test]
    fn reactive_transaction_captures_function_state_before_solve() {
        let input = Ref::new(1usize);
        let output = Ref::new(5usize);
        let retained = Ref::new(7usize);
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut plan = ReactivePlan::new();
        plan.register(
            Box::new(journal_function(
                "node",
                output.clone(),
                retained.clone(),
                events.clone(),
            )),
            &[LegacyValue::Index(input.clone())],
        )
        .unwrap();

        let mut journal = ReactiveTurnJournal::new();
        plan.solve_dirty_cells_with_journal(
            &LegacyValue::Index(input).reactive_root_cell_ids(),
            &mut journal,
        )
        .unwrap();
        journal.restore_before().unwrap();

        assert_eq!(events.borrow().as_slice(), &["capture node", "solve node"]);
        assert_eq!((*output.borrow(), *retained.borrow()), (5, 7));
    }

    #[test]
    fn reactive_transaction_capture_failure_prevents_solve() {
        let input = Ref::new(1usize);
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut function =
            journal_function("unsupported", Ref::new(0), Ref::new(0), events.clone());
        function.capture_error = true;
        let mut plan = ReactivePlan::new();
        plan.register(Box::new(function), &[LegacyValue::Index(input.clone())])
            .unwrap();

        let error = plan
            .solve_dirty_cells_with_journal(
                &LegacyValue::Index(input).reactive_root_cell_ids(),
                &mut ReactiveTurnJournal::new(),
            )
            .unwrap_err();

        assert_eq!(error.kind_name(), "TransactionStateUnsupported");
        assert_eq!(events.borrow().as_slice(), &["capture unsupported"]);
    }

    #[test]
    fn reactive_transaction_deduplicates_shared_cells() {
        let shared = Ref::new(1usize);
        let mut journal = ReactiveTurnJournal::new();

        journal
            .capture_value(&LegacyValue::Index(shared.clone()))
            .unwrap();
        journal.capture_value(&LegacyValue::Index(shared)).unwrap();

        assert_eq!(journal.cell_count(), 1);
    }

    #[test]
    fn reactive_transaction_restores_nested_cell_identity() {
        let inner = Ref::new(3usize);
        let outer = ValueCell::new(LegacyValue::Index(inner.clone()));
        let mut journal = ReactiveTurnJournal::new();
        journal.capture_value_cell(&outer).unwrap();
        *inner.borrow_mut() = 9;
        *outer.borrow_mut() = LegacyValue::Index(Ref::new(10));

        journal.restore_before().unwrap();

        let restored = outer.borrow();
        let LegacyValue::Index(restored_inner) = &*restored else {
            panic!("expected restored nested index")
        };
        assert!(restored_inner.same_handle(&inner));
        assert_eq!(*restored_inner.borrow(), 3);
    }

    #[test]
    fn reactive_transaction_restores_hidden_function_state() {
        let output = Ref::new(1usize);
        let retained = Ref::new(2usize);
        let function = journal_function(
            "hidden",
            output,
            retained.clone(),
            Rc::new(RefCell::new(Vec::new())),
        );
        let mut journal = ReactiveTurnJournal::new();
        journal.capture_function_state(&function).unwrap();
        *retained.borrow_mut() = 99;

        journal.restore_before().unwrap();

        assert_eq!(*retained.borrow(), 2);
    }

    #[test]
    fn reactive_transaction_prefers_typed_ports_restores_hidden_state_and_deduplicates() {
        let (function, out_calls, legacy_state_calls, _) =
            typed_journal_function(TypedStateMode::State);
        let output = function.output.clone();
        let retained = function.retained.clone();
        let mut journal = ReactiveTurnJournal::new();

        journal.capture_function_state(&function).unwrap();
        assert_eq!(journal.cell_count(), 2);
        *output.borrow_mut() = 11;
        *retained.borrow_mut() = 22;
        journal.restore_before().unwrap();

        assert_eq!((*output.borrow(), *retained.borrow()), (1, 2));
        assert_eq!(out_calls.get(), 0);
        assert_eq!(legacy_state_calls.get(), 0);
    }

    #[test]
    fn empty_typed_state_is_authoritative_without_out_fallback() {
        let (function, out_calls, legacy_state_calls, _) =
            typed_journal_function(TypedStateMode::Empty);
        let mut journal = ReactiveTurnJournal::new();

        journal.capture_function_state(&function).unwrap();

        assert!(journal.is_empty());
        assert_eq!(out_calls.get(), 0);
        assert_eq!(legacy_state_calls.get(), 0);
    }

    #[test]
    fn typed_state_error_prevents_solve_without_compatibility_fallback() {
        let input = Ref::new(1usize);
        let (function, _out_calls, legacy_state_calls, solve_calls) =
            typed_journal_function(TypedStateMode::Error);
        let mut plan = ReactivePlan::new();
        plan.register(Box::new(function), &[LegacyValue::Index(input.clone())])
            .unwrap();

        let error = plan
            .solve_dirty_cells_with_journal(
                &LegacyValue::Index(input).reactive_root_cell_ids(),
                &mut ReactiveTurnJournal::new(),
            )
            .unwrap_err();

        assert_eq!(error.kind_name(), "TransactionStateUnsupported");
        assert_eq!(legacy_state_calls.get(), 0);
        assert_eq!(solve_calls.get(), 0);
    }

    #[test]
    fn typed_and_legacy_function_state_coexist_in_one_journal() {
        let (typed, _, _, _) = typed_journal_function(TypedStateMode::State);
        let typed_output = typed.output.clone();
        let legacy_output = Ref::new(3usize);
        let legacy_retained = Ref::new(4usize);
        let legacy = journal_function(
            "legacy",
            legacy_output.clone(),
            legacy_retained.clone(),
            Rc::new(RefCell::new(Vec::new())),
        );
        let mut journal = ReactiveTurnJournal::new();

        journal.capture_function_state(&typed).unwrap();
        journal.capture_function_state(&legacy).unwrap();
        *typed_output.borrow_mut() = 10;
        *legacy_output.borrow_mut() = 30;
        *legacy_retained.borrow_mut() = 40;
        journal.restore_before().unwrap();

        assert_eq!(*typed_output.borrow(), 1);
        assert_eq!((*legacy_output.borrow(), *legacy_retained.borrow()), (3, 4));
    }

    #[test]
    fn reactive_transaction_preserves_unsupported_state_error() {
        let mut function = journal_function(
            "unsupported",
            Ref::new(0),
            Ref::new(0),
            Rc::new(RefCell::new(Vec::new())),
        );
        function.capture_error = true;

        let error = ReactiveTurnJournal::new()
            .capture_function_state(&function)
            .unwrap_err();

        assert_eq!(error.kind_name(), "TransactionStateUnsupported");
        assert!(
            error
                .kind_message()
                .contains("deliberate unsupported state")
        );
    }

    fn register(
        plan: &mut ReactivePlan,
        name: &'static str,
        source: Ref<usize>,
        sink: Ref<usize>,
        retained: Ref<usize>,
        events: Rc<RefCell<Vec<String>>>,
        fail_stage: bool,
    ) -> ReactiveNodeId {
        plan.push(Box::new(JournalRegister {
            name,
            source,
            sink,
            retained,
            events,
            fail_stage,
        }))
    }

    #[test]
    fn reactive_transaction_captures_all_registers_before_staging() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut plan = ReactivePlan::new();
        let first = register(
            &mut plan,
            "a",
            Ref::new(1),
            Ref::new(0),
            Ref::new(0),
            events.clone(),
            false,
        );
        let second = register(
            &mut plan,
            "b",
            Ref::new(2),
            Ref::new(0),
            Ref::new(0),
            events.clone(),
            false,
        );

        plan.commit_pending_registers_with_journal(
            &[second, first],
            &mut ReactiveTurnJournal::new(),
        )
        .unwrap();

        assert_eq!(
            events.borrow().as_slice(),
            &["capture a", "capture b", "stage a", "stage b"],
        );
    }

    #[test]
    fn reactive_transaction_stage_failure_can_restore_every_register() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let first_state = Ref::new(10usize);
        let second_state = Ref::new(20usize);
        let mut plan = ReactivePlan::new();
        let first = register(
            &mut plan,
            "a",
            Ref::new(1),
            Ref::new(0),
            first_state.clone(),
            events,
            false,
        );
        let second = register(
            &mut plan,
            "b",
            Ref::new(2),
            Ref::new(0),
            second_state.clone(),
            Rc::new(RefCell::new(Vec::new())),
            true,
        );
        let mut journal = ReactiveTurnJournal::new();

        plan.commit_pending_registers_with_journal(&[first, second], &mut journal)
            .unwrap_err();
        journal.restore_before().unwrap();

        assert_eq!((*first_state.borrow(), *second_state.borrow()), (10, 20));
    }

    #[test]
    fn reactive_transaction_post_register_failure_restores_register_state() {
        let source = Ref::new(8usize);
        let sink = Ref::new(1usize);
        let register_events = Rc::new(RefCell::new(Vec::new()));
        let mut plan = ReactivePlan::new();
        plan.register(
            Box::new(JournalRegister {
                name: "register",
                source: source.clone(),
                sink: sink.clone(),
                retained: Ref::new(0),
                events: register_events,
                fail_stage: false,
            }),
            &[LegacyValue::Index(source.clone())],
        )
        .unwrap();
        let mut failing = journal_function(
            "downstream",
            Ref::new(0),
            Ref::new(0),
            Rc::new(RefCell::new(Vec::new())),
        );
        failing.solve_error = true;
        plan.register(Box::new(failing), &[LegacyValue::Index(sink.clone())])
            .unwrap();
        let mut journal = ReactiveTurnJournal::new();

        plan.advance_reactive_turn_with_journal(
            &mut ReactiveTurnState::default(),
            &LegacyValue::Index(source).reactive_root_cell_ids(),
            &mut journal,
        )
        .unwrap_err();
        assert_eq!(*sink.borrow(), 8);
        journal.restore_before().unwrap();

        assert_eq!(*sink.borrow(), 1);
    }

    #[test]
    fn reactive_transaction_success_retains_only_ephemeral_before_state() {
        let input = Ref::new(1usize);
        let mut plan = ReactivePlan::new();
        plan.register(
            Box::new(journal_function(
                "success",
                Ref::new(0),
                Ref::new(0),
                Rc::new(RefCell::new(Vec::new())),
            )),
            &[LegacyValue::Index(input.clone())],
        )
        .unwrap();
        let mut journal = ReactiveTurnJournal::new();

        plan.solve_dirty_cells_with_journal(
            &LegacyValue::Index(input).reactive_root_cell_ids(),
            &mut journal,
        )
        .unwrap();

        assert!(!journal.is_empty());
        assert!(journal.cell_count() >= 2);
    }
}
