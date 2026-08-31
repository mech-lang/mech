//! Identity-preserving checkpoints for canonical cells and exact typed state.

use crate::*;

#[cfg(all(feature = "no_std", feature = "functions"))]
use alloc::boxed::Box;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(all(feature = "no_std", feature = "functions"))]
use core::any::{Any, type_name};
#[cfg(all(not(feature = "no_std"), feature = "functions"))]
use std::any::{Any, type_name};

#[cfg(feature = "functions")]
#[derive(Clone, Copy)]
enum CaptureSide {
    Before,
    After,
}

#[cfg(feature = "functions")]
impl CaptureSide {
    const fn phase(self) -> &'static str {
        match self {
            Self::Before => "capture-before",
            Self::After => "capture-after",
        }
    }
}

#[cfg(feature = "functions")]
trait ErasedValueStateRoot {
    fn clone_box(&self) -> Box<dyn ErasedValueStateRoot>;
    fn preflight(&self, journal: &CanonicalStateJournal, side: CaptureSide) -> MResult<()>;
    fn visit(&self, journal: &mut CanonicalStateJournal, side: CaptureSide) -> MResult<()>;
}

#[cfg(feature = "functions")]
struct RefValueStateRoot<T>
where
    T: Clone + 'static,
{
    target: Ref<T>,
}

#[cfg(feature = "functions")]
impl<T> ErasedValueStateRoot for RefValueStateRoot<T>
where
    T: Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn ErasedValueStateRoot> {
        Box::new(Self {
            target: self.target.clone(),
        })
    }

    fn preflight(&self, journal: &CanonicalStateJournal, side: CaptureSide) -> MResult<()> {
        journal.preflight_ref(&self.target, side)
    }

    fn visit(&self, journal: &mut CanonicalStateJournal, side: CaptureSide) -> MResult<()> {
        journal.visit_ref(&self.target, side)
    }
}

#[cfg(feature = "functions")]
impl Clone for Box<dyn ErasedValueStateRoot> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(feature = "functions")]
trait ErasedValueStateEntry {
    fn preflight_restore_before(&self) -> MResult<()>;
    fn preflight_restore_after(&self) -> MResult<()>;
    fn apply_restore_before(&self);
    fn apply_restore_after(&self);
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn matches_ref(&self, target: &dyn Any) -> bool;
    fn matches_cell(&self, cell: &ValueCell) -> bool;
}

#[cfg(feature = "functions")]
struct RefValueStateEntry<T>
where
    T: Clone + 'static,
{
    target: Ref<T>,
    before: Option<T>,
    after: Option<T>,
}

#[cfg(feature = "functions")]
impl<T> RefValueStateEntry<T>
where
    T: Clone + 'static,
{
    fn borrow_conflict(&self, phase: &'static str) -> MechError {
        MechError::new(
            ValueStateBorrowConflict {
                phase,
                type_name: type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    }
}

#[cfg(feature = "functions")]
impl<T> ErasedValueStateEntry for RefValueStateEntry<T>
where
    T: Clone + 'static,
{
    fn preflight_restore_before(&self) -> MResult<()> {
        if self.before.is_none() {
            return Ok(());
        }
        self.target
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| self.borrow_conflict("restore-before"))
    }

    fn preflight_restore_after(&self) -> MResult<()> {
        if self.after.is_none() {
            return Ok(());
        }
        self.target
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| self.borrow_conflict("restore-after"))
    }

    fn apply_restore_before(&self) {
        if let Some(before) = &self.before {
            *self.target.borrow_mut() = before.clone();
        }
    }

    fn apply_restore_after(&self) {
        if let Some(after) = &self.after {
            *self.target.borrow_mut() = after.clone();
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn matches_ref(&self, target: &dyn Any) -> bool {
        target
            .downcast_ref::<Ref<T>>()
            .is_some_and(|target| self.target.same_handle(target))
    }

    fn matches_cell(&self, cell: &ValueCell) -> bool {
        cell.same_exact_ref(&self.target)
    }
}

struct CanonicalValueCellStateEntry {
    target: ValueCell,
    before: Value,
    after: Option<Value>,
}

impl CanonicalValueCellStateEntry {
    fn preflight_restore_before(&self) -> MResult<()> {
        self.target.preflight_replace()
    }

    fn preflight_restore_after(&self) -> MResult<()> {
        if self.after.is_some() {
            self.target.preflight_replace()
        } else {
            Ok(())
        }
    }

    fn apply_restore_before(&self) {
        self.target
            .replace(&self.before)
            .expect("canonical value-cell restore was preflighted");
    }

    fn apply_restore_after(&self) {
        if let Some(after) = &self.after {
            self.target
                .replace(after)
                .expect("canonical value-cell replay was preflighted");
        }
    }
}

/// A reversible checkpoint over explicitly retained canonical and exact cells.
pub struct CanonicalStateJournal {
    #[cfg(feature = "functions")]
    entries: Vec<Box<dyn ErasedValueStateEntry>>,
    canonical_entries: Vec<CanonicalValueCellStateEntry>,
    #[cfg(feature = "functions")]
    exact_roots: Vec<Box<dyn ErasedValueStateRoot>>,
    after_recorded: bool,
    sealed: bool,
}

impl CanonicalStateJournal {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "functions")]
            entries: Vec::new(),
            canonical_entries: Vec::new(),
            #[cfg(feature = "functions")]
            exact_roots: Vec::new(),
            after_recorded: false,
            sealed: false,
        }
    }

    /// Captures one schema-aware cell without projecting its payload.
    pub fn capture_value_cell(&mut self, cell: &ValueCell) -> MResult<()> {
        self.ensure_open()?;
        if self
            .canonical_entries
            .iter()
            .any(|entry| entry.target.same_cell(cell))
        {
            return Ok(());
        }
        #[cfg(feature = "functions")]
        if self.entries.iter().any(|entry| entry.matches_cell(cell)) {
            return Ok(());
        }
        self.canonical_entries.push(CanonicalValueCellStateEntry {
            target: cell.clone(),
            before: cell.snapshot()?,
            after: None,
        });
        Ok(())
    }

    /// Captures one exact typed cell without a universal-value projection.
    #[cfg(feature = "functions")]
    pub(crate) fn capture_exact_ref<T>(&mut self, target: &Ref<T>) -> MResult<()>
    where
        T: Clone + 'static,
    {
        self.ensure_open()?;
        if self
            .canonical_entries
            .iter()
            .any(|entry| entry.target.same_exact_ref(target))
            || self.entries.iter().any(|entry| entry.matches_ref(target))
        {
            return Ok(());
        }
        self.preflight_ref(target, CaptureSide::Before)?;
        self.visit_ref(target, CaptureSide::Before)?;
        self.exact_roots.push(Box::new(RefValueStateRoot {
            target: target.clone(),
        }));
        Ok(())
    }

    pub fn restore_before(&self) -> MResult<()> {
        self.preflight_restore_before()?;
        self.apply_restore_before();
        Ok(())
    }

    pub fn preflight_restore_before(&self) -> MResult<()> {
        #[cfg(feature = "functions")]
        for entry in &self.entries {
            entry.preflight_restore_before()?;
        }
        for entry in &self.canonical_entries {
            entry.preflight_restore_before()?;
        }
        Ok(())
    }

    pub fn apply_restore_before(&self) {
        #[cfg(feature = "functions")]
        for entry in &self.entries {
            entry.apply_restore_before();
        }
        for entry in &self.canonical_entries {
            entry.apply_restore_before();
        }
    }

    pub fn record_after(&mut self) -> MResult<()> {
        if self.after_recorded {
            return Err(MechError::new(ValueStateAfterAlreadyRecorded, None).with_compiler_loc());
        }
        self.ensure_open()?;
        #[cfg(feature = "functions")]
        let roots = self.exact_roots.clone();
        #[cfg(feature = "functions")]
        for root in &roots {
            root.preflight(self, CaptureSide::After)?;
        }
        #[cfg(feature = "functions")]
        for root in &roots {
            root.visit(self, CaptureSide::After)?;
        }
        for entry in &mut self.canonical_entries {
            entry.after = Some(entry.target.snapshot()?);
        }
        self.after_recorded = true;
        self.sealed = true;
        Ok(())
    }

    pub fn into_delta(self) -> MResult<CommittedValueStateDelta> {
        if !self.after_recorded {
            return Err(MechError::new(ValueStateAfterNotRecorded, None).with_compiler_loc());
        }
        Ok(CommittedValueStateDelta {
            #[cfg(feature = "functions")]
            entries: self.entries,
            canonical_entries: self.canonical_entries,
        })
    }

    pub fn cell_count(&self) -> usize {
        let count = self.canonical_entries.len();
        #[cfg(feature = "functions")]
        let count = count + self.entries.len();
        count
    }

    pub fn is_empty(&self) -> bool {
        let empty = self.canonical_entries.is_empty();
        #[cfg(feature = "functions")]
        let empty = empty && self.entries.is_empty();
        empty
    }

    fn ensure_open(&self) -> MResult<()> {
        if self.sealed {
            Err(MechError::new(ValueStateJournalSealed, None).with_compiler_loc())
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "functions")]
    fn preflight_ref<T>(&self, target: &Ref<T>, side: CaptureSide) -> MResult<()>
    where
        T: Clone + 'static,
    {
        target.try_borrow().map(|_| ()).map_err(|_| {
            MechError::new(
                ValueStateBorrowConflict {
                    phase: side.phase(),
                    type_name: type_name::<T>(),
                },
                None,
            )
            .with_compiler_loc()
        })
    }

    #[cfg(feature = "functions")]
    fn visit_ref<T>(&mut self, target: &Ref<T>, side: CaptureSide) -> MResult<()>
    where
        T: Clone + 'static,
    {
        let snapshot = target
            .try_borrow()
            .map(|value| value.clone())
            .map_err(|_| {
                MechError::new(
                    ValueStateBorrowConflict {
                        phase: side.phase(),
                        type_name: type_name::<T>(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.matches_ref(target))
        {
            let entry = entry
                .as_any_mut()
                .downcast_mut::<RefValueStateEntry<T>>()
                .ok_or_else(|| {
                    MechError::new(
                        ValueStateEntryTypeMismatch {
                            type_name: type_name::<T>(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
            match side {
                CaptureSide::Before if entry.before.is_none() => entry.before = Some(snapshot),
                CaptureSide::After => entry.after = Some(snapshot),
                CaptureSide::Before => {}
            }
        } else {
            let (before, after) = match side {
                CaptureSide::Before => (Some(snapshot), None),
                CaptureSide::After => (None, Some(snapshot)),
            };
            self.entries.push(Box::new(RefValueStateEntry {
                target: target.clone(),
                before,
                after,
            }));
        }
        Ok(())
    }
}

impl Default for CanonicalStateJournal {
    fn default() -> Self {
        Self::new()
    }
}

/// A reusable, process-local before/after delta over the original cells.
pub struct CommittedValueStateDelta {
    #[cfg(feature = "functions")]
    entries: Vec<Box<dyn ErasedValueStateEntry>>,
    canonical_entries: Vec<CanonicalValueCellStateEntry>,
}

impl CommittedValueStateDelta {
    pub fn rewind(&self) -> MResult<()> {
        #[cfg(feature = "functions")]
        for entry in &self.entries {
            entry.preflight_restore_before()?;
        }
        for entry in &self.canonical_entries {
            entry.preflight_restore_before()?;
        }
        #[cfg(feature = "functions")]
        for entry in &self.entries {
            entry.apply_restore_before();
        }
        for entry in &self.canonical_entries {
            entry.apply_restore_before();
        }
        Ok(())
    }

    pub fn replay(&self) -> MResult<()> {
        #[cfg(feature = "functions")]
        for entry in &self.entries {
            entry.preflight_restore_after()?;
        }
        for entry in &self.canonical_entries {
            entry.preflight_restore_after()?;
        }
        #[cfg(feature = "functions")]
        for entry in &self.entries {
            entry.apply_restore_after();
        }
        for entry in &self.canonical_entries {
            entry.apply_restore_after();
        }
        Ok(())
    }

    pub fn cell_count(&self) -> usize {
        let count = self.canonical_entries.len();
        #[cfg(feature = "functions")]
        let count = count + self.entries.len();
        count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueStateBorrowConflict {
    pub phase: &'static str,
    pub type_name: &'static str,
}

impl MechErrorKind for ValueStateBorrowConflict {
    fn name(&self) -> &str {
        "ValueStateBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Cannot borrow {} cell during {}.",
            self.type_name, self.phase
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueStateEntryTypeMismatch {
    pub type_name: &'static str,
}

impl MechErrorKind for ValueStateEntryTypeMismatch {
    fn name(&self) -> &str {
        "ValueStateEntryTypeMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Journal entry for {} cell has an incompatible erased type.",
            self.type_name
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueStateJournalSealed;

impl MechErrorKind for ValueStateJournalSealed {
    fn name(&self) -> &str {
        "ValueStateJournalSealed"
    }

    fn message(&self) -> String {
        "The value-state journal is sealed and cannot capture more roots.".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueStateAfterAlreadyRecorded;

impl MechErrorKind for ValueStateAfterAlreadyRecorded {
    fn name(&self) -> &str {
        "ValueStateAfterAlreadyRecorded"
    }

    fn message(&self) -> String {
        "The value-state journal already contains an after-state.".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueStateAfterNotRecorded;

impl MechErrorKind for ValueStateAfterNotRecorded {
    fn name(&self) -> &str {
        "ValueStateAfterNotRecorded"
    }

    fn message(&self) -> String {
        "The value-state journal has no recorded after-state.".to_string()
    }
}

#[cfg(all(test, feature = "functions"))]
#[path = "tests/exact_minimal.rs"]
mod exact_ref_feature_smoke_tests;

#[cfg(test)]
mod tests;
