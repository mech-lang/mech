//! Identity-preserving snapshots for the live cells reachable from [`Value`]
//! roots.
//!
//! This module deliberately has no integration with execution or reactive
//! scheduling. Callers pay for graph traversal only when they explicitly
//! construct and populate a [`ValueStateJournal`].

#[cfg(feature = "matrix")]
use crate::structures::matrix::Matrix;
use crate::*;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "no_std")]
use core::any::{Any, TypeId, type_name};
#[cfg(feature = "no_std")]
use core::hash::BuildHasherDefault;
#[cfg(feature = "no_std")]
use fxhash::FxHasher;
#[cfg(feature = "no_std")]
use hashbrown::{HashMap as HashBrownMap, HashSet as HashBrownSet};
#[cfg(feature = "map")]
use indexmap::IndexMap;
#[cfg(feature = "set")]
use indexmap::IndexSet;

#[cfg(not(feature = "no_std"))]
use std::any::{Any, TypeId, type_name};
#[cfg(not(feature = "no_std"))]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "no_std")]
type HashMap<K, V> = HashBrownMap<K, V, BuildHasherDefault<FxHasher>>;
#[cfg(feature = "no_std")]
type HashSet<K> = HashBrownSet<K, BuildHasherDefault<FxHasher>>;

#[cfg(feature = "set")]
fn canonical_set_snapshot(set: &MechSet, phase: &'static str) -> MResult<MechSet> {
    let mut elements = IndexSet::with_capacity(set.set.len());
    for (second_index, element) in set.set.iter().enumerate() {
        let element = element.clone();
        if let Some(first_index) = elements.iter().position(|existing| existing == &element) {
            return Err(MechError::new(
                ValueStateCollectionCollision {
                    phase,
                    collection: "set element",
                    first_index,
                    second_index,
                },
                None,
            )
            .with_compiler_loc());
        }
        elements.insert(element);
    }
    Ok(MechSet {
        kind: set.kind.clone(),
        num_elements: set.num_elements,
        set: elements,
    })
}

#[cfg(feature = "map")]
fn canonical_map_snapshot(map: &MechMap, phase: &'static str) -> MResult<MechMap> {
    let mut entries = IndexMap::with_capacity(map.map.len());
    for (second_index, (key, value)) in map.map.iter().enumerate() {
        let key = key.clone();
        let value = value.clone();
        if let Some(first_index) = entries.keys().position(|existing| existing == &key) {
            return Err(MechError::new(
                ValueStateCollectionCollision {
                    phase,
                    collection: "map key",
                    first_index,
                    second_index,
                },
                None,
            )
            .with_compiler_loc());
        }
        entries.insert(key, value);
    }
    Ok(MechMap {
        key_kind: map.key_kind.clone(),
        value_kind: map.value_kind.clone(),
        num_elements: map.num_elements,
        map: entries,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ValueStateKey {
    type_id: TypeId,
    address: usize,
}

impl ValueStateKey {
    fn of<T: 'static>(target: &Ref<T>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            address: target.addr(),
        }
    }
}

#[derive(Clone, Copy)]
enum CaptureSide {
    Before,
    After,
}

impl CaptureSide {
    fn phase(self) -> &'static str {
        match self {
            Self::Before => "capture-before",
            Self::After => "capture-after",
        }
    }
}

#[derive(Clone)]
enum ValueStateRoot {
    Value(Value),
    ValRef(ValRef),
}

trait ErasedValueStateEntry {
    fn key(&self) -> ValueStateKey;
    fn has_before(&self) -> bool;
    fn has_after(&self) -> bool;
    fn preflight_restore_before(&self) -> MResult<()>;
    fn preflight_restore_after(&self) -> MResult<()>;
    fn apply_restore_before(&self);
    fn apply_restore_after(&self);
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct RefValueStateEntry<T>
where
    T: Clone + 'static,
{
    key: ValueStateKey,
    target: Ref<T>,
    before: Option<T>,
    after: Option<T>,
}

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

impl<T> ErasedValueStateEntry for RefValueStateEntry<T>
where
    T: Clone + 'static,
{
    fn key(&self) -> ValueStateKey {
        self.key
    }

    fn has_before(&self) -> bool {
        self.before.is_some()
    }

    fn has_after(&self) -> bool {
        self.after.is_some()
    }

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
}

/// A reversible snapshot of all live cells reachable from explicitly captured
/// roots.
///
/// The journal stores shallow payload clones alongside the original cell
/// handles. Nested cells are journaled separately, preserving addresses,
/// sharing, cycles, and reactive identities when state is restored.
///
/// This is the only checkpoint layer that knows how physical cell payloads are
/// captured and restored. Higher-level checkpoints coordinate its opaque
/// preflight and apply operations without reaching into cell storage.
pub struct ValueStateJournal {
    entries: Vec<Box<dyn ErasedValueStateEntry>>,
    entry_indices: HashMap<ValueStateKey, usize>,
    roots: Vec<ValueStateRoot>,
    after_recorded: bool,
    sealed: bool,
}

impl ValueStateJournal {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            entry_indices: HashMap::default(),
            roots: Vec::new(),
            after_recorded: false,
            sealed: false,
        }
    }

    /// Adds a root [`Value`] and captures every currently reachable live cell.
    pub fn capture_value(&mut self, value: &Value) -> MResult<()> {
        self.ensure_open()?;

        let mut preflight_seen = HashSet::default();
        self.preflight_value(value, CaptureSide::Before, &mut preflight_seen)?;

        let mut capture_seen = HashSet::default();
        self.visit_value(value, CaptureSide::Before, &mut capture_seen)?;
        self.roots.push(ValueStateRoot::Value(value.clone()));
        Ok(())
    }

    /// Captures the [`ValRef`] cell itself and every cell reachable from its
    /// current [`Value`].
    pub fn capture_val_ref(&mut self, value: &ValRef) -> MResult<()> {
        self.ensure_open()?;

        let mut preflight_seen = HashSet::default();
        self.preflight_ref(
            value,
            CaptureSide::Before,
            &mut preflight_seen,
            |journal, value, seen| journal.preflight_value(value, CaptureSide::Before, seen),
        )?;

        let mut capture_seen = HashSet::default();
        self.visit_ref(
            value,
            CaptureSide::Before,
            &mut capture_seen,
            |journal, value, seen| journal.visit_value(value, CaptureSide::Before, seen),
        )?;
        self.roots.push(ValueStateRoot::ValRef(value.clone()));
        Ok(())
    }

    /// Restores all captured before-state into the original cells.
    ///
    /// Every target is preflighted before the first payload is changed.
    pub fn restore_before(&self) -> MResult<()> {
        self.preflight_restore_before()?;
        self.apply_restore_before();
        Ok(())
    }

    /// Verifies that every captured before-state target can be mutably borrowed.
    ///
    /// Transaction coordinators can call this before mutating any other
    /// checkpointed subsystem, then call [`Self::apply_restore_before`] after
    /// all participating checkpoints have passed their own preflight.
    pub fn preflight_restore_before(&self) -> MResult<()> {
        for entry in &self.entries {
            entry.preflight_restore_before()?;
        }
        Ok(())
    }

    /// Restores all captured before-state after a successful preflight.
    ///
    /// This method deliberately performs no fallible work. Callers must not
    /// retain or acquire borrows of captured cells between
    /// [`Self::preflight_restore_before`] and this method.
    pub fn apply_restore_before(&self) {
        for entry in &self.entries {
            entry.apply_restore_before();
        }
    }

    /// Records the complete state currently reachable from all saved roots.
    pub fn record_after(&mut self) -> MResult<()> {
        if self.after_recorded {
            return Err(MechError::new(ValueStateAfterAlreadyRecorded, None).with_compiler_loc());
        }
        if self.sealed {
            return Err(MechError::new(ValueStateJournalSealed, None).with_compiler_loc());
        }

        // Clone only the root handles so mutable traversal of the entries does not
        // alias the roots field. This does not clone any cell payload.
        let roots = self.roots.clone();

        let mut preflight_seen = HashSet::default();
        for root in &roots {
            self.preflight_root(root, CaptureSide::After, &mut preflight_seen)?;
        }

        let mut capture_seen = HashSet::default();
        for root in &roots {
            self.visit_root(root, CaptureSide::After, &mut capture_seen)?;
        }

        self.after_recorded = true;
        self.sealed = true;
        Ok(())
    }

    /// Converts a journal with a recorded after-state into a reusable delta.
    pub fn into_delta(self) -> MResult<CommittedValueStateDelta> {
        if !self.after_recorded {
            return Err(MechError::new(ValueStateAfterNotRecorded, None).with_compiler_loc());
        }
        Ok(CommittedValueStateDelta {
            entries: self.entries,
        })
    }

    pub fn cell_count(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn ensure_open(&self) -> MResult<()> {
        if self.sealed {
            Err(MechError::new(ValueStateJournalSealed, None).with_compiler_loc())
        } else {
            Ok(())
        }
    }

    fn preflight_root(
        &self,
        root: &ValueStateRoot,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()> {
        match root {
            ValueStateRoot::Value(value) => self.preflight_value(value, side, seen),
            ValueStateRoot::ValRef(value) => {
                self.preflight_ref(value, side, seen, |journal, value, seen| {
                    journal.preflight_value(value, side, seen)
                })
            }
        }
    }

    fn visit_root(
        &mut self,
        root: &ValueStateRoot,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()> {
        match root {
            ValueStateRoot::Value(value) => self.visit_value(value, side, seen),
            ValueStateRoot::ValRef(value) => {
                self.visit_ref(value, side, seen, |journal, value, seen| {
                    journal.visit_value(value, side, seen)
                })
            }
        }
    }

    fn preflight_ref<T, F>(
        &self,
        target: &Ref<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
        descend: F,
    ) -> MResult<()>
    where
        T: Clone + 'static,
        F: FnOnce(&Self, &T, &mut HashSet<ValueStateKey>) -> MResult<()>,
    {
        self.preflight_ref_with_snapshot(
            target,
            side,
            seen,
            |value, _phase| Ok(value.clone()),
            descend,
        )
    }

    fn preflight_ref_with_snapshot<T, F, S>(
        &self,
        target: &Ref<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
        snapshot: S,
        descend: F,
    ) -> MResult<()>
    where
        T: Clone + 'static,
        S: FnOnce(&T, &'static str) -> MResult<T>,
        F: FnOnce(&Self, &T, &mut HashSet<ValueStateKey>) -> MResult<()>,
    {
        let key = ValueStateKey::of(target);
        if !seen.insert(key) {
            return Ok(());
        }
        let payload = target.try_borrow().map_err(|_| {
            MechError::new(
                ValueStateBorrowConflict {
                    phase: side.phase(),
                    type_name: type_name::<T>(),
                },
                None,
            )
            .with_compiler_loc()
        })?;

        // Map/set snapshot construction invokes Value equality and hashing. Every
        // nested Ref must be preflighted before those operations can run.
        descend(self, &payload, seen)?;

        let _ = snapshot(&payload, side.phase())?;
        Ok(())
    }

    fn visit_ref<T, F>(
        &mut self,
        target: &Ref<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
        descend: F,
    ) -> MResult<()>
    where
        T: Clone + 'static,
        F: FnOnce(&mut Self, &T, &mut HashSet<ValueStateKey>) -> MResult<()>,
    {
        self.visit_ref_with_snapshot(
            target,
            side,
            seen,
            |value, _phase| Ok(value.clone()),
            descend,
        )
    }

    fn visit_ref_with_snapshot<T, F, S>(
        &mut self,
        target: &Ref<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
        snapshot: S,
        descend: F,
    ) -> MResult<()>
    where
        T: Clone + 'static,
        S: FnOnce(&T, &'static str) -> MResult<T>,
        F: FnOnce(&mut Self, &T, &mut HashSet<ValueStateKey>) -> MResult<()>,
    {
        let key = ValueStateKey::of(target);
        if !seen.insert(key) {
            return Ok(());
        }
        let payload = target.try_borrow().map_err(|_| {
            MechError::new(
                ValueStateBorrowConflict {
                    phase: side.phase(),
                    type_name: type_name::<T>(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let snapshot = snapshot(&payload, side.phase())?;

        descend(self, &snapshot, seen)?;

        if let Some(index) = self.entry_indices.get(&key).copied() {
            let entry = self.entries[index]
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
                CaptureSide::Before => {
                    if entry.before.is_none() {
                        entry.before = Some(snapshot);
                    }
                }
                CaptureSide::After => {
                    entry.after = Some(snapshot);
                }
            }
        } else {
            let (before, after) = match side {
                CaptureSide::Before => (Some(snapshot), None),
                CaptureSide::After => (None, Some(snapshot)),
            };
            let index = self.entries.len();
            self.entries.push(Box::new(RefValueStateEntry {
                key,
                target: target.clone(),
                before,
                after,
            }));
            self.entry_indices.insert(key, index);
        }

        Ok(())
    }

    fn preflight_value(
        &self,
        value: &Value,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()> {
        match value {
            #[cfg(feature = "u8")]
            Value::U8(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "u16")]
            Value::U16(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "u32")]
            Value::U32(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "u64")]
            Value::U64(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "u128")]
            Value::U128(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "i8")]
            Value::I8(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "i16")]
            Value::I16(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "i32")]
            Value::I32(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "i64")]
            Value::I64(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "i128")]
            Value::I128(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "f32")]
            Value::F32(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "f64")]
            Value::F64(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "complex")]
            Value::C64(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "rational")]
            Value::R64(value) => self.preflight_leaf(value, side, seen),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            Value::String(value) => self.preflight_leaf(value, side, seen),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            Value::Bool(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "atom")]
            Value::Atom(value) => self.preflight_ref(value, side, seen, |journal, atom, seen| {
                journal.preflight_leaf(&atom.0.1, side, seen)
            }),
            Value::Index(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrix")]
            Value::MatrixIndex(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            Value::MatrixBool(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            Value::MatrixU8(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            Value::MatrixU16(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            Value::MatrixU32(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            Value::MatrixU64(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            Value::MatrixU128(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            Value::MatrixI8(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            Value::MatrixI16(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            Value::MatrixI32(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            Value::MatrixI64(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            Value::MatrixI128(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            Value::MatrixF32(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            Value::MatrixF64(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "string"))]
            Value::MatrixString(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            Value::MatrixR64(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            Value::MatrixC64(matrix) => self.preflight_matrix(matrix, side, seen),
            #[cfg(feature = "matrix")]
            Value::MatrixValue(matrix) => self.preflight_value_matrix(matrix, side, seen),
            #[cfg(feature = "set")]
            Value::Set(value) => self.preflight_ref_with_snapshot(
                value,
                side,
                seen,
                canonical_set_snapshot,
                |journal, set, seen| {
                    for value in &set.set {
                        journal.preflight_value(value, side, seen)?;
                    }
                    Ok(())
                },
            ),
            #[cfg(feature = "map")]
            Value::Map(value) => self.preflight_ref_with_snapshot(
                value,
                side,
                seen,
                canonical_map_snapshot,
                |journal, map, seen| {
                    for (key, value) in &map.map {
                        journal.preflight_value(key, side, seen)?;
                        journal.preflight_value(value, side, seen)?;
                    }
                    Ok(())
                },
            ),
            #[cfg(feature = "record")]
            Value::Record(value) => {
                self.preflight_ref(value, side, seen, |journal, record, seen| {
                    for value in record.data.values() {
                        journal.preflight_value(value, side, seen)?;
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "table")]
            Value::Table(value) => self.preflight_ref(value, side, seen, |journal, table, seen| {
                for (_, matrix) in table.data.values() {
                    journal.preflight_value_matrix(matrix, side, seen)?;
                }
                Ok(())
            }),
            #[cfg(feature = "tuple")]
            Value::Tuple(value) => self.preflight_ref(value, side, seen, |journal, tuple, seen| {
                for value in &tuple.elements {
                    journal.preflight_value(value, side, seen)?;
                }
                Ok(())
            }),
            #[cfg(feature = "enum")]
            Value::Enum(value) => {
                self.preflight_ref(value, side, seen, |journal, enum_value, seen| {
                    journal.preflight_leaf(&enum_value.names, side, seen)?;
                    for (_, payload) in &enum_value.variants {
                        if let Some(payload) = payload {
                            journal.preflight_value(payload, side, seen)?;
                        }
                    }
                    Ok(())
                })
            }
            Value::MutableReference(value) => {
                self.preflight_ref(value, side, seen, |journal, value, seen| {
                    journal.preflight_value(value, side, seen)
                })
            }
            Value::Typed(value, _) => self.preflight_value(value, side, seen),
            Value::Id(_)
            | Value::Kind(_)
            | Value::IndexAll
            | Value::EmptyKind(_)
            | Value::Empty => Ok(()),
        }
    }

    fn visit_value(
        &mut self,
        value: &Value,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()> {
        match value {
            #[cfg(feature = "u8")]
            Value::U8(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "u16")]
            Value::U16(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "u32")]
            Value::U32(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "u64")]
            Value::U64(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "u128")]
            Value::U128(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "i8")]
            Value::I8(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "i16")]
            Value::I16(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "i32")]
            Value::I32(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "i64")]
            Value::I64(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "i128")]
            Value::I128(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "f32")]
            Value::F32(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "f64")]
            Value::F64(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "complex")]
            Value::C64(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "rational")]
            Value::R64(value) => self.visit_leaf(value, side, seen),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            Value::String(value) => self.visit_leaf(value, side, seen),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            Value::Bool(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "atom")]
            Value::Atom(value) => self.visit_ref(value, side, seen, |journal, atom, seen| {
                journal.visit_leaf(&atom.0.1, side, seen)
            }),
            Value::Index(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrix")]
            Value::MatrixIndex(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            Value::MatrixBool(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            Value::MatrixU8(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            Value::MatrixU16(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            Value::MatrixU32(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            Value::MatrixU64(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            Value::MatrixU128(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            Value::MatrixI8(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            Value::MatrixI16(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            Value::MatrixI32(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            Value::MatrixI64(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            Value::MatrixI128(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            Value::MatrixF32(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            Value::MatrixF64(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "string"))]
            Value::MatrixString(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            Value::MatrixR64(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            Value::MatrixC64(matrix) => self.visit_matrix(matrix, side, seen),
            #[cfg(feature = "matrix")]
            Value::MatrixValue(matrix) => self.visit_value_matrix(matrix, side, seen),
            #[cfg(feature = "set")]
            Value::Set(value) => self.visit_ref_with_snapshot(
                value,
                side,
                seen,
                canonical_set_snapshot,
                |journal, set, seen| {
                    for value in &set.set {
                        journal.visit_value(value, side, seen)?;
                    }
                    Ok(())
                },
            ),
            #[cfg(feature = "map")]
            Value::Map(value) => self.visit_ref_with_snapshot(
                value,
                side,
                seen,
                canonical_map_snapshot,
                |journal, map, seen| {
                    for (key, value) in &map.map {
                        journal.visit_value(key, side, seen)?;
                        journal.visit_value(value, side, seen)?;
                    }
                    Ok(())
                },
            ),
            #[cfg(feature = "record")]
            Value::Record(value) => self.visit_ref(value, side, seen, |journal, record, seen| {
                for value in record.data.values() {
                    journal.visit_value(value, side, seen)?;
                }
                Ok(())
            }),
            #[cfg(feature = "table")]
            Value::Table(value) => self.visit_ref(value, side, seen, |journal, table, seen| {
                for (_, matrix) in table.data.values() {
                    journal.visit_value_matrix(matrix, side, seen)?;
                }
                Ok(())
            }),
            #[cfg(feature = "tuple")]
            Value::Tuple(value) => self.visit_ref(value, side, seen, |journal, tuple, seen| {
                for value in &tuple.elements {
                    journal.visit_value(value, side, seen)?;
                }
                Ok(())
            }),
            #[cfg(feature = "enum")]
            Value::Enum(value) => self.visit_ref(value, side, seen, |journal, enum_value, seen| {
                journal.visit_leaf(&enum_value.names, side, seen)?;
                for (_, payload) in &enum_value.variants {
                    if let Some(payload) = payload {
                        journal.visit_value(payload, side, seen)?;
                    }
                }
                Ok(())
            }),
            Value::MutableReference(value) => {
                self.visit_ref(value, side, seen, |journal, value, seen| {
                    journal.visit_value(value, side, seen)
                })
            }
            Value::Typed(value, _) => self.visit_value(value, side, seen),
            Value::Id(_)
            | Value::Kind(_)
            | Value::IndexAll
            | Value::EmptyKind(_)
            | Value::Empty => Ok(()),
        }
    }

    fn preflight_leaf<T>(
        &self,
        value: &Ref<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()>
    where
        T: Clone + 'static,
    {
        self.preflight_ref(value, side, seen, |_, _, _| Ok(()))
    }

    fn visit_leaf<T>(
        &mut self,
        value: &Ref<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()>
    where
        T: Clone + 'static,
    {
        self.visit_ref(value, side, seen, |_, _, _| Ok(()))
    }

    #[cfg(feature = "matrix")]
    fn preflight_matrix<T>(
        &self,
        matrix: &Matrix<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()>
    where
        T: Clone + 'static,
    {
        match matrix {
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "vectord")]
            Matrix::DVector(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(value) => self.preflight_leaf(value, side, seen),
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(value) => self.preflight_leaf(value, side, seen),
        }
    }

    #[cfg(feature = "matrix")]
    fn visit_matrix<T>(
        &mut self,
        matrix: &Matrix<T>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()>
    where
        T: Clone + 'static,
    {
        match matrix {
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "vectord")]
            Matrix::DVector(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(value) => self.visit_leaf(value, side, seen),
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(value) => self.visit_leaf(value, side, seen),
        }
    }

    #[cfg(feature = "matrix")]
    fn preflight_value_matrix(
        &self,
        matrix: &Matrix<Value>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()> {
        macro_rules! preflight_backing {
            ($value:expr) => {
                self.preflight_ref($value, side, seen, |journal, values, seen| {
                    for value in values.iter() {
                        journal.preflight_value(value, side, seen)?;
                    }
                    Ok(())
                })
            };
        }
        match matrix {
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(value) => preflight_backing!(value),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(value) => preflight_backing!(value),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(value) => preflight_backing!(value),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(value) => preflight_backing!(value),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(value) => preflight_backing!(value),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(value) => preflight_backing!(value),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(value) => preflight_backing!(value),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(value) => preflight_backing!(value),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(value) => preflight_backing!(value),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(value) => preflight_backing!(value),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(value) => preflight_backing!(value),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(value) => preflight_backing!(value),
            #[cfg(feature = "vectord")]
            Matrix::DVector(value) => preflight_backing!(value),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(value) => preflight_backing!(value),
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(value) => preflight_backing!(value),
        }
    }

    #[cfg(feature = "matrix")]
    fn visit_value_matrix(
        &mut self,
        matrix: &Matrix<Value>,
        side: CaptureSide,
        seen: &mut HashSet<ValueStateKey>,
    ) -> MResult<()> {
        macro_rules! visit_backing {
            ($value:expr) => {
                self.visit_ref($value, side, seen, |journal, values, seen| {
                    for value in values.iter() {
                        journal.visit_value(value, side, seen)?;
                    }
                    Ok(())
                })
            };
        }
        match matrix {
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(value) => visit_backing!(value),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(value) => visit_backing!(value),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(value) => visit_backing!(value),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(value) => visit_backing!(value),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(value) => visit_backing!(value),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(value) => visit_backing!(value),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(value) => visit_backing!(value),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(value) => visit_backing!(value),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(value) => visit_backing!(value),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(value) => visit_backing!(value),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(value) => visit_backing!(value),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(value) => visit_backing!(value),
            #[cfg(feature = "vectord")]
            Matrix::DVector(value) => visit_backing!(value),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(value) => visit_backing!(value),
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(value) => visit_backing!(value),
        }
    }
}

impl Default for ValueStateJournal {
    fn default() -> Self {
        Self::new()
    }
}

/// A reusable, process-local before/after delta over the original live cells.
pub struct CommittedValueStateDelta {
    entries: Vec<Box<dyn ErasedValueStateEntry>>,
}

impl CommittedValueStateDelta {
    /// Restores the before-state into the original cells.
    pub fn rewind(&self) -> MResult<()> {
        for entry in &self.entries {
            entry.preflight_restore_before()?;
        }
        for entry in &self.entries {
            entry.apply_restore_before();
        }
        Ok(())
    }

    /// Restores the recorded after-state into the original cells.
    pub fn replay(&self) -> MResult<()> {
        for entry in &self.entries {
            entry.preflight_restore_after()?;
        }
        for entry in &self.entries {
            entry.apply_restore_after();
        }
        Ok(())
    }

    pub fn cell_count(&self) -> usize {
        self.entries.len()
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
        format!("Cannot borrow {} cell during {}.", self.type_name, self.phase)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueStateCollectionCollision {
    pub phase: &'static str,
    pub collection: &'static str,
    pub first_index: usize,
    pub second_index: usize,
}

impl MechErrorKind for ValueStateCollectionCollision {
    fn name(&self) -> &str {
        "ValueStateCollectionCollision"
    }

    fn message(&self) -> String {
        format!(
            "Canonical {} state capture during {} found entries at indexes {} and {} that are equal under their current payloads.",
            self.collection, self.phase, self.first_index, self.second_index
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

#[cfg(test)]
#[path = "state_journal/tests/mod.rs"]
mod tests;
