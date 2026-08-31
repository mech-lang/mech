use crate::{
    CanonicalStateJournal, FunctionInstance, MResult, MechError, MechErrorKind, MechFunction,
    ValueCell,
};
#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::string::{String, ToString};
use core::cell::Cell;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::string::String;

/// The value-state portion of one ephemeral reactive turn.
///
/// This journal deliberately contains only before-state. It is consumed by
/// higher-level rollback coordinators and never becomes durable history.
pub(crate) struct CanonicalTurnJournal {
    values: CanonicalStateJournal,
}

impl CanonicalTurnJournal {
    pub(crate) fn new() -> Self {
        Self {
            values: CanonicalStateJournal::new(),
        }
    }

    pub(crate) fn capture_value_cell(&mut self, value: &ValueCell) -> MResult<()> {
        self.values.capture_value_cell(value)
    }

    pub(crate) fn capture_function_state(&mut self, function: &dyn MechFunction) -> MResult<()> {
        function.capture_retained_state(&mut self.values)
    }

    pub(crate) fn capture_primary_and_retained_function_state(
        &mut self,
        function: &dyn MechFunction,
    ) -> MResult<()> {
        function
            .primary_output_state_port()
            .ok_or_else(|| {
                MechError::new(
                    crate::TransactionStateUnsupportedError {
                        function: function.to_string(),
                        reason: "direct reactive functions must expose an exact primary state port"
                            .into(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?
            .capture_into(&mut self.values)?;
        self.capture_function_state(function)
    }

    pub(crate) fn capture_function_instance(&mut self, instance: &FunctionInstance) -> MResult<()> {
        instance.capture_state(&mut self.values)
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

impl Default for CanonicalTurnJournal {
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
    journal: &'journal mut CanonicalTurnJournal,
    finalization: &'journal Cell<ReactiveJournalFinalizationState>,
}

impl ReactiveJournalParticipant<'_> {
    pub fn capture_value_cell(&mut self, value: &ValueCell) -> MResult<()> {
        self.journal.capture_value_cell(value)
    }

    pub fn capture_function_state(&mut self, function: &dyn MechFunction) -> MResult<()> {
        self.journal.capture_function_state(function)
    }

    pub fn capture_function_instance(&mut self, instance: &FunctionInstance) -> MResult<()> {
        self.journal.capture_function_instance(instance)
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

    pub(crate) fn journal_mut(&mut self) -> &mut CanonicalTurnJournal {
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
    let mut journal = CanonicalTurnJournal::new();
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
    use crate::{GenericError, ValueData, ValueDataDraft};

    fn index_cell(value: usize) -> ValueCell {
        ValueCell::from_exact(value).unwrap()
    }

    fn index(cell: &ValueCell) -> u64 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::Index(value) = snapshot.data() else {
            panic!("expected an index cell")
        };
        *value
    }

    fn replace_index(cell: &ValueCell, value: u64) {
        let replacement = cell
            .rebuild_data_draft(ValueDataDraft::Index(value))
            .unwrap();
        cell.replace(&replacement).unwrap();
    }

    #[test]
    fn explicit_commit_keeps_canonical_mutation() {
        let cell = index_cell(1);
        with_reactive_journal_participant(|mut participant| {
            participant.capture_value_cell(&cell)?;
            replace_index(&cell, 2);
            participant.commit();
            Ok(())
        })
        .unwrap();

        assert_eq!(index(&cell), 2);
    }

    #[test]
    fn explicit_rollback_restores_canonical_state_and_identity() {
        let cell = index_cell(1);
        let alias = cell.clone();
        with_reactive_journal_participant(|mut participant| {
            participant.capture_value_cell(&cell)?;
            replace_index(&cell, 2);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(cell.same_cell(&alias));
        assert_eq!(index(&cell), 1);
    }

    #[test]
    fn error_path_rolls_back_automatically_and_preserves_the_original_error() {
        let cell = index_cell(1);
        let error = with_reactive_journal_participant::<()>(|mut participant| {
            participant.capture_value_cell(&cell)?;
            replace_index(&cell, 2);
            Err(MechError::new(
                GenericError {
                    msg: "intentional participant failure".into(),
                },
                None,
            ))
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert_eq!(index(&cell), 1);
    }

    #[test]
    fn successful_path_without_finalization_rolls_back_and_fails_closed() {
        let cell = index_cell(1);
        let error = with_reactive_journal_participant(|mut participant| {
            participant.capture_value_cell(&cell)?;
            replace_index(&cell, 2);
            Ok(())
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "ReactiveJournalFinalizationMissing");
        assert_eq!(index(&cell), 1);
    }
}
