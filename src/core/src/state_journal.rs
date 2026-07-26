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
    fn restore_before_unchecked(&self);
    fn restore_after_unchecked(&self);
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
                address: self.key.address,
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

    fn restore_before_unchecked(&self) {
        if let Some(before) = &self.before {
            *self.target.borrow_mut() = before.clone();
        }
    }

    fn restore_after_unchecked(&self) {
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
            entry.restore_before_unchecked();
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
                    address: key.address,
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let snapshot = snapshot(&payload, side.phase())?;
        descend(self, &snapshot, seen)
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
                    address: key.address,
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
                            address: key.address,
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
            entry.restore_before_unchecked();
        }
        Ok(())
    }

    /// Restores the recorded after-state into the original cells.
    pub fn replay(&self) -> MResult<()> {
        for entry in &self.entries {
            entry.preflight_restore_after()?;
        }
        for entry in &self.entries {
            entry.restore_after_unchecked();
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
    pub address: usize,
}

impl MechErrorKind for ValueStateBorrowConflict {
    fn name(&self) -> &str {
        "ValueStateBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "Cannot borrow {} cell at 0x{:x} during {}.",
            self.type_name, self.address, self.phase
        )
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
    pub address: usize,
}

impl MechErrorKind for ValueStateEntryTypeMismatch {
    fn name(&self) -> &str {
        "ValueStateEntryTypeMismatch"
    }

    fn message(&self) -> String {
        format!(
            "Journal entry for {} cell at 0x{:x} has an incompatible erased type.",
            self.type_name, self.address
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

#[cfg(all(
    test,
    feature = "f64",
    feature = "record",
    feature = "tuple",
    feature = "map",
    feature = "set",
    feature = "table",
    feature = "enum",
    feature = "matrixd",
    feature = "vectord"
))]
mod tests {
    use super::*;
    use crate::structures::matrix::Matrix as ValueMatrix;
    use indexmap::{IndexMap, IndexSet};
    use nalgebra::{DMatrix, DVector};
    use std::collections::HashMap;

    fn scalar(value: f64) -> Ref<f64> {
        Ref::new(value)
    }

    fn scalar_value(value: &Ref<f64>) -> Value {
        Value::F64(value.clone())
    }

    fn as_scalar(value: &Value) -> Ref<f64> {
        match value {
            Value::F64(value) => value.clone(),
            _ => panic!("expected f64 value"),
        }
    }

    fn scalar_payload(value: &Value) -> f64 {
        *as_scalar(value).borrow()
    }

    fn set_contains_scalar(set: &Ref<MechSet>, value: f64) -> bool {
        let probe = scalar_value(&scalar(value));
        set.borrow().set.contains(&probe)
    }

    fn map_get_scalar(map: &Ref<MechMap>, key: f64) -> Option<Ref<f64>> {
        let probe = scalar_value(&scalar(key));
        map.borrow().map.get(&probe).map(as_scalar)
    }

    fn scalar_set_value(value: f64) -> Value {
        let member = scalar(value);
        Value::Set(Ref::new(MechSet::from_vec(vec![scalar_value(&member)])))
    }

    fn map_contains_scalar_set_key(map: &Ref<MechMap>, value: f64) -> bool {
        map.borrow().map.contains_key(&scalar_set_value(value))
    }

    fn record_value(fields: Vec<(&str, Value)>) -> (Ref<MechRecord>, Value) {
        let record = Ref::new(MechRecord::new(fields));
        let value = Value::Record(record.clone());
        (record, value)
    }

    #[test]
    fn state_journal_scalar_restore_preserves_address_and_reactive_identity() {
        let cell = scalar(1.0);
        let value = scalar_value(&cell);
        let address = cell.addr();
        let identity = ReactiveCellId::new(cell.id());

        let mut journal = ValueStateJournal::new();
        assert!(journal.is_empty());
        journal.capture_value(&value).unwrap();
        assert_eq!(journal.cell_count(), 1);

        *cell.borrow_mut() = 9.0;
        journal.restore_before().unwrap();

        assert_eq!(*cell.borrow(), 1.0);
        assert_eq!(cell.addr(), address);
        assert_eq!(value.reactive_root_cell_ids(), vec![identity]);
    }

    #[test]
    fn state_journal_split_restore_preflights_before_apply() {
        let cell = scalar(1.0);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&cell)).unwrap();
        *cell.borrow_mut() = 2.0;

        let held_borrow = cell.borrow();
        let error = journal.preflight_restore_before().unwrap_err();
        assert_eq!(error.kind_name(), "ValueStateBorrowConflict");
        assert_eq!(*held_borrow, 2.0);
        drop(held_borrow);

        journal.preflight_restore_before().unwrap();
        journal.apply_restore_before();
        assert_eq!(*cell.borrow(), 1.0);
    }

    #[test]
    fn state_journal_delta_rewinds_and_replays_repeatedly() {
        let cell = scalar(1.0);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&cell)).unwrap();
        *cell.borrow_mut() = 2.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();

        delta.rewind().unwrap();
        assert_eq!(*cell.borrow(), 1.0);
        delta.replay().unwrap();
        assert_eq!(*cell.borrow(), 2.0);
        delta.rewind().unwrap();
        assert_eq!(*cell.borrow(), 1.0);

        *cell.borrow_mut() = 99.0;
        delta.replay().unwrap();
        assert_eq!(*cell.borrow(), 2.0);
    }

    #[test]
    fn state_journal_deduplicates_shared_cells_and_restores_aliases() {
        let shared = scalar(3.0);
        let (record, root) = record_value(vec![
            ("left", scalar_value(&shared)),
            ("right", scalar_value(&shared)),
        ]);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();

        assert_eq!(journal.cell_count(), 2);
        *shared.borrow_mut() = 8.0;
        journal.restore_before().unwrap();

        let record = record.borrow();
        let left = as_scalar(record.data.get(&hash_str("left")).unwrap());
        let right = as_scalar(record.data.get(&hash_str("right")).unwrap());
        assert_eq!(*left.borrow(), 3.0);
        assert_eq!(left.addr(), shared.addr());
        assert_eq!(right.addr(), shared.addr());
    }

    #[test]
    fn state_journal_self_reference_terminates_in_both_phases() {
        let cell = Ref::new(Value::Empty);
        *cell.borrow_mut() = Value::MutableReference(cell.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_val_ref(&cell).unwrap();
        assert_eq!(journal.cell_count(), 1);
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();

        delta.rewind().unwrap();
        match &*cell.borrow() {
            Value::MutableReference(inner) => assert_eq!(inner.addr(), cell.addr()),
            _ => panic!("self-reference was not restored"),
        }
        delta.replay().unwrap();
        match &*cell.borrow() {
            Value::MutableReference(inner) => assert_eq!(inner.addr(), cell.addr()),
            _ => panic!("self-reference was not replayed"),
        }
    }

    #[test]
    fn state_journal_val_ref_tracks_replaced_root_and_after_only_cell() {
        let before = scalar(1.0);
        let after = scalar(2.0);
        let root = Ref::new(scalar_value(&before));
        let root_address = root.addr();

        let mut journal = ValueStateJournal::new();
        journal.capture_val_ref(&root).unwrap();
        *root.borrow_mut() = scalar_value(&after);
        journal.record_after().unwrap();
        assert_eq!(journal.cell_count(), 3);
        let delta = journal.into_delta().unwrap();

        delta.rewind().unwrap();
        assert_eq!(root.addr(), root_address);
        assert_eq!(as_scalar(&root.borrow()).addr(), before.addr());

        *after.borrow_mut() = 22.0;
        delta.replay().unwrap();
        assert_eq!(as_scalar(&root.borrow()).addr(), after.addr());
        assert_eq!(*after.borrow(), 2.0);

        *before.borrow_mut() = 11.0;
        delta.rewind().unwrap();
        assert_eq!(as_scalar(&root.borrow()).addr(), before.addr());
        assert_eq!(*before.borrow(), 1.0);
    }

    #[test]
    fn state_journal_record_delta_restores_order_and_removed_or_new_cells() {
        let old = scalar(1.0);
        let retained = scalar(2.0);
        let new = scalar(3.0);
        let old_id = hash_str("old");
        let retained_id = hash_str("retained");
        let new_id = hash_str("new");
        let (record, root) = record_value(vec![
            ("old", scalar_value(&old)),
            ("retained", scalar_value(&retained)),
        ]);

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        {
            let mut record = record.borrow_mut();
            record.data.shift_remove(&old_id);
            record.field_names.remove(&old_id);
            record.kinds.remove(0);
            record.data.insert(new_id, scalar_value(&new));
            record.field_names.insert(new_id, "new".to_string());
            record.kinds.push(ValueKind::F64);
            record.cols = 2;
        }
        *retained.borrow_mut() = 20.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();

        delta.rewind().unwrap();
        {
            let record = record.borrow();
            assert_eq!(
                record.data.keys().copied().collect::<Vec<_>>(),
                vec![old_id, retained_id]
            );
            assert_eq!(record.cols, 2);
            assert_eq!(record.kinds, vec![ValueKind::F64, ValueKind::F64]);
            assert_eq!(record.field_names.get(&old_id).unwrap(), "old");
            assert_eq!(
                as_scalar(record.data.get(&old_id).unwrap()).addr(),
                old.addr()
            );
            assert!(!record.data.contains_key(&new_id));
        }
        assert_eq!(*old.borrow(), 1.0);
        assert_eq!(*retained.borrow(), 2.0);

        *new.borrow_mut() = 30.0;
        delta.replay().unwrap();
        {
            let record = record.borrow();
            assert_eq!(
                record.data.keys().copied().collect::<Vec<_>>(),
                vec![retained_id, new_id]
            );
            assert!(!record.data.contains_key(&old_id));
            assert_eq!(
                as_scalar(record.data.get(&new_id).unwrap()).addr(),
                new.addr()
            );
        }
        assert_eq!(*retained.borrow(), 20.0);
        assert_eq!(*new.borrow(), 3.0);
    }

    #[test]
    fn state_journal_tuple_restores_structure_order_and_child_identity() {
        let first = scalar(1.0);
        let retained = scalar(2.0);
        let replacement = scalar(3.0);
        let tuple = Ref::new(MechTuple::from_vec(vec![
            scalar_value(&first),
            scalar_value(&retained),
        ]));
        let root = Value::Tuple(tuple.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        {
            let mut tuple = tuple.borrow_mut();
            tuple.elements.remove(0);
            tuple
                .elements
                .insert(0, Box::new(scalar_value(&replacement)));
            tuple.elements.swap(0, 1);
        }
        *retained.borrow_mut() = 22.0;
        journal.restore_before().unwrap();

        let tuple = tuple.borrow();
        assert_eq!(tuple.elements.len(), 2);
        assert_eq!(as_scalar(&tuple.elements[0]).addr(), first.addr());
        assert_eq!(as_scalar(&tuple.elements[1]).addr(), retained.addr());
        assert_eq!(scalar_payload(&tuple.elements[0]), 1.0);
        assert_eq!(scalar_payload(&tuple.elements[1]), 2.0);
    }

    #[test]
    fn state_journal_map_restores_metadata_order_and_retained_value() {
        let removed = scalar(1.0);
        let retained = scalar(2.0);
        let added = scalar(3.0);
        let mut data = IndexMap::new();
        data.insert(Value::Id(1), scalar_value(&removed));
        data.insert(Value::Id(2), scalar_value(&retained));
        let map = Ref::new(MechMap {
            key_kind: ValueKind::Id,
            value_kind: ValueKind::F64,
            num_elements: 2,
            map: data,
        });
        let root = Value::Map(map.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        {
            let mut map = map.borrow_mut();
            map.map.shift_remove(&Value::Id(1));
            map.map.insert(Value::Id(3), scalar_value(&added));
            map.key_kind = ValueKind::Any;
            map.value_kind = ValueKind::Any;
            map.num_elements = 9;
        }
        *retained.borrow_mut() = 20.0;
        journal.restore_before().unwrap();

        let map = map.borrow();
        assert_eq!(map.key_kind, ValueKind::Id);
        assert_eq!(map.value_kind, ValueKind::F64);
        assert_eq!(map.num_elements, 2);
        assert_eq!(
            map.map.keys().cloned().collect::<Vec<_>>(),
            vec![Value::Id(1), Value::Id(2)]
        );
        assert_eq!(
            as_scalar(map.map.get(&Value::Id(1)).unwrap()).addr(),
            removed.addr()
        );
        assert_eq!(
            as_scalar(map.map.get(&Value::Id(2)).unwrap()).addr(),
            retained.addr()
        );
        assert_eq!(*retained.borrow(), 2.0);
    }

    #[test]
    fn state_journal_set_restores_metadata_order_and_retained_cell() {
        let removed = scalar(1.0);
        let retained = scalar(2.0);
        let added = scalar(3.0);
        let mut members = IndexSet::new();
        members.insert(scalar_value(&removed));
        members.insert(scalar_value(&retained));
        let set = Ref::new(MechSet {
            kind: ValueKind::F64,
            num_elements: 2,
            set: members,
        });
        let root = Value::Set(set.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        {
            let mut set = set.borrow_mut();
            set.set.shift_remove(&scalar_value(&removed));
            set.set.insert(scalar_value(&added));
            set.kind = ValueKind::Any;
            set.num_elements = 9;
        }
        // Perform this after structural edits so the temporarily stale hash does
        // not participate in another set operation.
        *retained.borrow_mut() = 20.0;
        journal.restore_before().unwrap();

        let set = set.borrow();
        assert_eq!(set.kind, ValueKind::F64);
        assert_eq!(set.num_elements, 2);
        let members = set.set.iter().map(as_scalar).collect::<Vec<_>>();
        assert_eq!(
            members.iter().map(Ref::addr).collect::<Vec<_>>(),
            vec![removed.addr(), retained.addr()]
        );
        assert_eq!(*members[0].borrow(), 1.0);
        assert_eq!(*members[1].borrow(), 2.0);
    }

    #[test]
    fn state_journal_set_membership_survives_repeated_rewind_and_replay() {
        let member = scalar(1.0);
        let set = Ref::new(MechSet::from_vec(vec![scalar_value(&member)]));
        let member_address = member.addr();
        let set_address = set.addr();
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&Value::Set(set.clone())).unwrap();
        assert_eq!(journal.cell_count(), 2);

        *member.borrow_mut() = 2.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        assert_eq!(delta.cell_count(), 2);

        for _ in 0..2 {
            delta.rewind().unwrap();
            assert_eq!(set.addr(), set_address);
            assert_eq!(member.addr(), member_address);
            assert_eq!(*member.borrow(), 1.0);
            assert!(set_contains_scalar(&set, 1.0));
            assert!(!set_contains_scalar(&set, 2.0));
            assert_eq!(
                set.borrow()
                    .set
                    .iter()
                    .next()
                    .map(as_scalar)
                    .unwrap()
                    .addr(),
                member_address
            );

            delta.replay().unwrap();
            assert_eq!(set.addr(), set_address);
            assert_eq!(member.addr(), member_address);
            assert_eq!(*member.borrow(), 2.0);
            assert!(!set_contains_scalar(&set, 1.0));
            assert!(set_contains_scalar(&set, 2.0));
            assert_eq!(
                set.borrow()
                    .set
                    .iter()
                    .next()
                    .map(as_scalar)
                    .unwrap()
                    .addr(),
                member_address
            );
        }
    }

    #[test]
    fn state_journal_map_key_lookup_survives_repeated_rewind_and_replay() {
        let key = scalar(1.0);
        let value = scalar(10.0);
        let map = Ref::new(MechMap::from_vec(vec![(
            scalar_value(&key),
            scalar_value(&value),
        )]));
        let key_address = key.addr();
        let map_address = map.addr();
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&Value::Map(map.clone())).unwrap();
        assert_eq!(journal.cell_count(), 3);

        *key.borrow_mut() = 2.0;
        *value.borrow_mut() = 20.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        assert_eq!(delta.cell_count(), 3);

        for _ in 0..2 {
            delta.rewind().unwrap();
            assert_eq!(map.addr(), map_address);
            assert_eq!(key.addr(), key_address);
            assert!(map_get_scalar(&map, 2.0).is_none());
            let found = map_get_scalar(&map, 1.0).unwrap();
            assert_eq!(found.addr(), value.addr());
            assert_eq!(*found.borrow(), 10.0);
            assert_eq!(
                map.borrow()
                    .map
                    .keys()
                    .next()
                    .map(as_scalar)
                    .unwrap()
                    .addr(),
                key_address
            );

            delta.replay().unwrap();
            assert_eq!(map.addr(), map_address);
            assert_eq!(key.addr(), key_address);
            assert!(map_get_scalar(&map, 1.0).is_none());
            let found = map_get_scalar(&map, 2.0).unwrap();
            assert_eq!(found.addr(), value.addr());
            assert_eq!(*found.borrow(), 20.0);
            assert_eq!(
                map.borrow()
                    .map
                    .keys()
                    .next()
                    .map(as_scalar)
                    .unwrap()
                    .addr(),
                key_address
            );
        }
    }

    #[test]
    fn state_journal_set_collision_is_atomic_and_retryable() {
        let safe = scalar(10.0);
        let left = scalar(1.0);
        let right = scalar(2.0);
        let set = Ref::new(MechSet::from_vec(vec![
            scalar_value(&left),
            scalar_value(&right),
        ]));
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&safe)).unwrap();
        journal.capture_value(&Value::Set(set.clone())).unwrap();
        assert_eq!(journal.cell_count(), 4);

        *safe.borrow_mut() = 11.0;
        *right.borrow_mut() = 1.0;
        let error = journal.record_after().unwrap_err();
        let collision = error.kind_as::<ValueStateCollectionCollision>().unwrap();
        assert_eq!(collision.phase, "capture-after");
        assert_eq!(collision.collection, "set element");
        assert_eq!(collision.first_index, 0);
        assert_eq!(collision.second_index, 1);
        assert!(!journal.after_recorded);
        assert!(!journal.sealed);
        assert_eq!(journal.cell_count(), 4);
        assert!(journal.entries.iter().all(|entry| !entry.has_after()));
        assert_eq!(*safe.borrow(), 11.0);
        assert_eq!(set.borrow().kind, ValueKind::F64);
        assert_eq!(set.borrow().num_elements, 2);
        assert_eq!(
            set.borrow()
                .set
                .iter()
                .map(as_scalar)
                .map(|value| value.addr())
                .collect::<Vec<_>>(),
            vec![left.addr(), right.addr()]
        );

        journal.restore_before().unwrap();
        assert_eq!(*safe.borrow(), 10.0);
        assert_eq!((*left.borrow(), *right.borrow()), (1.0, 2.0));
        assert!(set_contains_scalar(&set, 1.0));
        assert!(set_contains_scalar(&set, 2.0));

        *safe.borrow_mut() = 12.0;
        *right.borrow_mut() = 3.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        assert_eq!(delta.cell_count(), 4);
        delta.rewind().unwrap();
        assert_eq!(*safe.borrow(), 10.0);
        assert!(set_contains_scalar(&set, 1.0));
        assert!(set_contains_scalar(&set, 2.0));
        delta.replay().unwrap();
        assert_eq!(*safe.borrow(), 12.0);
        assert!(set_contains_scalar(&set, 1.0));
        assert!(!set_contains_scalar(&set, 2.0));
        assert!(set_contains_scalar(&set, 3.0));
    }

    #[test]
    fn state_journal_collection_collision_uses_payload_equality_not_hash_match() {
        let left = scalar(-0.0);
        let right = scalar(1.0);
        let set = Ref::new(MechSet::from_vec(vec![
            scalar_value(&left),
            scalar_value(&right),
        ]));
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&Value::Set(set.clone())).unwrap();

        *right.borrow_mut() = 0.0;
        let error = journal.record_after().unwrap_err();
        let collision = error.kind_as::<ValueStateCollectionCollision>().unwrap();
        assert_eq!(collision.collection, "set element");
        assert_eq!(collision.first_index, 0);
        assert_eq!(collision.second_index, 1);
        assert!(!journal.after_recorded);
        assert!(!journal.sealed);
        assert!(journal.entries.iter().all(|entry| !entry.has_after()));

        journal.restore_before().unwrap();
        assert_eq!(left.borrow().to_bits(), (-0.0f64).to_bits());
        assert_eq!(*right.borrow(), 1.0);
        assert!(set_contains_scalar(&set, -0.0));
        assert!(set_contains_scalar(&set, 1.0));
    }

    #[test]
    fn state_journal_map_collision_is_atomic_and_retryable() {
        let safe = scalar(10.0);
        let left_key = scalar(1.0);
        let right_key = scalar(2.0);
        let left_value = scalar(10.0);
        let right_value = scalar(20.0);
        let map = Ref::new(MechMap::from_vec(vec![
            (scalar_value(&left_key), scalar_value(&left_value)),
            (scalar_value(&right_key), scalar_value(&right_value)),
        ]));
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&safe)).unwrap();
        journal.capture_value(&Value::Map(map.clone())).unwrap();
        assert_eq!(journal.cell_count(), 6);

        *safe.borrow_mut() = 11.0;
        *right_key.borrow_mut() = 1.0;
        let error = journal.record_after().unwrap_err();
        let collision = error.kind_as::<ValueStateCollectionCollision>().unwrap();
        assert_eq!(collision.phase, "capture-after");
        assert_eq!(collision.collection, "map key");
        assert_eq!(collision.first_index, 0);
        assert_eq!(collision.second_index, 1);
        assert!(!journal.after_recorded);
        assert!(!journal.sealed);
        assert_eq!(journal.cell_count(), 6);
        assert!(journal.entries.iter().all(|entry| !entry.has_after()));
        assert_eq!(*safe.borrow(), 11.0);
        assert_eq!(map.borrow().key_kind, ValueKind::F64);
        assert_eq!(map.borrow().value_kind, ValueKind::F64);
        assert_eq!(map.borrow().num_elements, 2);
        assert_eq!(map.borrow().map.len(), 2);
        assert_eq!(
            map.borrow()
                .map
                .keys()
                .map(as_scalar)
                .map(|value| value.addr())
                .collect::<Vec<_>>(),
            vec![left_key.addr(), right_key.addr()]
        );
        assert_eq!(
            map.borrow()
                .map
                .values()
                .map(as_scalar)
                .map(|value| value.addr())
                .collect::<Vec<_>>(),
            vec![left_value.addr(), right_value.addr()]
        );

        journal.restore_before().unwrap();
        assert_eq!(*safe.borrow(), 10.0);
        assert_eq!((*left_key.borrow(), *right_key.borrow()), (1.0, 2.0));
        assert_eq!(map_get_scalar(&map, 1.0).unwrap().addr(), left_value.addr());
        assert_eq!(
            map_get_scalar(&map, 2.0).unwrap().addr(),
            right_value.addr()
        );

        *safe.borrow_mut() = 12.0;
        *right_key.borrow_mut() = 3.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        assert_eq!(delta.cell_count(), 6);
        delta.rewind().unwrap();
        assert_eq!(*safe.borrow(), 10.0);
        assert_eq!(map_get_scalar(&map, 1.0).unwrap().addr(), left_value.addr());
        assert_eq!(
            map_get_scalar(&map, 2.0).unwrap().addr(),
            right_value.addr()
        );
        delta.replay().unwrap();
        assert_eq!(*safe.borrow(), 12.0);
        assert_eq!(map_get_scalar(&map, 1.0).unwrap().addr(), left_value.addr());
        assert!(map_get_scalar(&map, 2.0).is_none());
        assert_eq!(
            map_get_scalar(&map, 3.0).unwrap().addr(),
            right_value.addr()
        );
    }

    #[test]
    fn state_journal_nested_hashed_collection_lookups_survive_delta() {
        let member = scalar(1.0);
        let inner_set = Ref::new(MechSet::from_vec(vec![scalar_value(&member)]));
        let outer_map = Ref::new(MechMap::from_vec(vec![(
            Value::Set(inner_set.clone()),
            Value::Id(7),
        )]));
        let outer_address = outer_map.addr();
        let inner_address = inner_set.addr();
        let member_address = member.addr();
        let mut journal = ValueStateJournal::new();
        journal
            .capture_value(&Value::Map(outer_map.clone()))
            .unwrap();
        assert_eq!(journal.cell_count(), 3);

        *member.borrow_mut() = 2.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        assert_eq!(delta.cell_count(), 3);

        for _ in 0..2 {
            delta.rewind().unwrap();
            assert_eq!(outer_map.addr(), outer_address);
            assert_eq!(inner_set.addr(), inner_address);
            assert_eq!(member.addr(), member_address);
            assert!(set_contains_scalar(&inner_set, 1.0));
            assert!(!set_contains_scalar(&inner_set, 2.0));
            assert!(map_contains_scalar_set_key(&outer_map, 1.0));
            assert!(!map_contains_scalar_set_key(&outer_map, 2.0));

            assert_eq!(outer_map.borrow().map.len(), 1);
            assert!(matches!(
                outer_map.borrow().map.values().next(),
                Some(Value::Id(7))
            ));
            let restored_key = outer_map.borrow().map.keys().next().cloned().unwrap();
            match restored_key {
                Value::Set(restored) => assert_eq!(restored.addr(), inner_address),
                _ => panic!("expected nested set key"),
            }
            assert_eq!(
                inner_set
                    .borrow()
                    .set
                    .iter()
                    .next()
                    .map(as_scalar)
                    .unwrap()
                    .addr(),
                member_address
            );

            delta.replay().unwrap();
            assert_eq!(outer_map.addr(), outer_address);
            assert_eq!(inner_set.addr(), inner_address);
            assert_eq!(member.addr(), member_address);
            assert!(!set_contains_scalar(&inner_set, 1.0));
            assert!(set_contains_scalar(&inner_set, 2.0));
            assert!(!map_contains_scalar_set_key(&outer_map, 1.0));
            assert!(map_contains_scalar_set_key(&outer_map, 2.0));

            assert_eq!(outer_map.borrow().map.len(), 1);
            assert!(matches!(
                outer_map.borrow().map.values().next(),
                Some(Value::Id(7))
            ));
            let restored_key = outer_map.borrow().map.keys().next().cloned().unwrap();
            match restored_key {
                Value::Set(restored) => assert_eq!(restored.addr(), inner_address),
                _ => panic!("expected nested set key"),
            }
            assert_eq!(
                inner_set
                    .borrow()
                    .set
                    .iter()
                    .next()
                    .map(as_scalar)
                    .unwrap()
                    .addr(),
                member_address
            );
        }
    }

    #[test]
    fn state_journal_table_restores_columns_backings_and_nested_cells() {
        let first = scalar(1.0);
        let retained = scalar(2.0);
        let added = scalar(3.0);
        let original_backing = Ref::new(DVector::from_vec(vec![
            scalar_value(&first),
            scalar_value(&retained),
        ]));
        let original_matrix = ValueMatrix::DVector(original_backing.clone());
        let added_backing = Ref::new(DVector::from_vec(vec![scalar_value(&added)]));
        let added_matrix = ValueMatrix::DVector(added_backing);
        let original_id = hash_str("original");
        let added_id = hash_str("added");
        let mut data = IndexMap::new();
        data.insert(original_id, (ValueKind::F64, original_matrix));
        let mut names = HashMap::new();
        names.insert(original_id, "original".to_string());
        let table = Ref::new(MechTable {
            rows: 2,
            cols: 1,
            data,
            col_names: names,
        });
        let root = Value::Table(table.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        {
            let mut table = table.borrow_mut();
            let original = table.data.shift_remove(&original_id).unwrap();
            table.data.insert(added_id, (ValueKind::F64, added_matrix));
            table.data.insert(original_id, original);
            table.col_names.insert(added_id, "added".to_string());
            table.rows = 7;
            table.cols = 2;
        }
        *retained.borrow_mut() = 20.0;
        journal.restore_before().unwrap();

        let table = table.borrow();
        assert_eq!(table.rows, 2);
        assert_eq!(table.cols, 1);
        assert_eq!(
            table.data.keys().copied().collect::<Vec<_>>(),
            vec![original_id]
        );
        assert_eq!(table.col_names.len(), 1);
        assert_eq!(table.col_names.get(&original_id).unwrap(), "original");
        let (kind, matrix) = table.data.get(&original_id).unwrap();
        assert_eq!(*kind, ValueKind::F64);
        assert_eq!(matrix.addr(), original_backing.addr());
        assert_eq!(
            matrix
                .as_vec()
                .iter()
                .map(scalar_payload)
                .collect::<Vec<_>>(),
            vec![1.0, 2.0]
        );
        assert_eq!(as_scalar(&matrix.as_vec()[1]).addr(), retained.addr());
    }

    #[test]
    fn state_journal_enum_restores_names_variants_and_payload_identities() {
        let removed = scalar(1.0);
        let retained = scalar(2.0);
        let added = scalar(3.0);
        let enum_id = hash_str("choice");
        let removed_id = hash_str("removed");
        let retained_id = hash_str("retained");
        let added_id = hash_str("added");
        let mut dictionary = HashMap::new();
        dictionary.insert(enum_id, "choice".to_string());
        dictionary.insert(removed_id, "choice/removed".to_string());
        dictionary.insert(retained_id, "choice/retained".to_string());
        let names = Ref::new(dictionary);
        let names_address = names.addr();
        let enum_value = Ref::new(MechEnum {
            id: enum_id,
            variants: vec![
                (removed_id, Some(scalar_value(&removed))),
                (retained_id, Some(scalar_value(&retained))),
            ],
            names: names.clone(),
        });
        let root = Value::Enum(enum_value.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        {
            let mut value = enum_value.borrow_mut();
            value.id = hash_str("changed");
            value.variants.remove(0);
            value
                .variants
                .insert(0, (added_id, Some(scalar_value(&added))));
        }
        names.borrow_mut().clear();
        *retained.borrow_mut() = 20.0;
        journal.restore_before().unwrap();

        let value = enum_value.borrow();
        assert_eq!(value.id, enum_id);
        assert_eq!(value.names.addr(), names_address);
        assert_eq!(
            value.variants.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![removed_id, retained_id]
        );
        assert_eq!(
            as_scalar(value.variants[0].1.as_ref().unwrap()).addr(),
            removed.addr()
        );
        assert_eq!(
            as_scalar(value.variants[1].1.as_ref().unwrap()).addr(),
            retained.addr()
        );
        assert_eq!(
            value.names.borrow().get(&retained_id).unwrap(),
            "choice/retained"
        );
        assert_eq!(*retained.borrow(), 2.0);
    }

    #[cfg(feature = "atom")]
    #[test]
    fn state_journal_atom_restores_its_reachable_dictionary_cell() {
        let atom_id = hash_str("ready");
        let mut dictionary = HashMap::new();
        dictionary.insert(atom_id, "ready".to_string());
        let names = Ref::new(dictionary);
        let names_address = names.addr();
        let atom = Ref::new(MechAtom((atom_id, names.clone())));
        let root = Value::Atom(atom.clone());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        assert_eq!(journal.cell_count(), 2);
        atom.borrow_mut().0.0 = hash_str("changed");
        names.borrow_mut().clear();
        journal.restore_before().unwrap();

        assert_eq!(atom.borrow().id(), atom_id);
        assert_eq!(atom.borrow().dictionary().addr(), names_address);
        assert_eq!(names.borrow().get(&atom_id).unwrap(), "ready");
    }

    #[test]
    fn state_journal_dynamic_matrix_restores_shape_contents_and_backing() {
        let backing = Ref::new(DMatrix::from_vec(2, 2, vec![1.0, 2.0, 3.0, 4.0]));
        let root = Value::MatrixF64(ValueMatrix::DMatrix(backing.clone()));
        let address = backing.addr();
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();

        *backing.borrow_mut() = DMatrix::from_vec(1, 3, vec![8.0, 9.0, 10.0]);
        journal.restore_before().unwrap();

        let matrix = backing.borrow();
        assert_eq!(backing.addr(), address);
        assert_eq!(matrix.shape(), (2, 2));
        assert_eq!(matrix.as_slice(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn state_journal_dynamic_vector_restores_contents_and_backing() {
        let backing = Ref::new(DVector::from_vec(vec![1.0, 2.0, 3.0]));
        let root = Value::MatrixF64(ValueMatrix::DVector(backing.clone()));
        let address = backing.addr();
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();

        *backing.borrow_mut() = DVector::from_vec(vec![9.0]);
        journal.restore_before().unwrap();

        assert_eq!(backing.addr(), address);
        assert_eq!(backing.borrow().as_slice(), &[1.0, 2.0, 3.0]);
    }

    #[cfg(feature = "matrix2")]
    #[test]
    fn state_journal_fixed_matrix_restores_contents_and_backing() {
        let backing = Ref::new(nalgebra::Matrix2::from_vec(vec![1.0, 2.0, 3.0, 4.0]));
        let root = Value::MatrixF64(ValueMatrix::Matrix2(backing.clone()));
        let address = backing.addr();
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();

        *backing.borrow_mut() = nalgebra::Matrix2::from_element(9.0);
        journal.restore_before().unwrap();

        assert_eq!(backing.addr(), address);
        assert_eq!(backing.borrow().as_slice(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn state_journal_value_matrix_restores_topology_and_nested_cells() {
        let first = scalar(1.0);
        let retained = scalar(2.0);
        let backing = Ref::new(DMatrix::from_vec(
            2,
            1,
            vec![scalar_value(&first), scalar_value(&retained)],
        ));
        let root = Value::MatrixValue(ValueMatrix::DMatrix(backing.clone()));
        let address = backing.addr();
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();

        *backing.borrow_mut() = DMatrix::from_vec(1, 1, vec![scalar_value(&retained)]);
        *retained.borrow_mut() = 20.0;
        journal.restore_before().unwrap();

        let matrix = backing.borrow();
        assert_eq!(backing.addr(), address);
        assert_eq!(matrix.shape(), (2, 1));
        assert_eq!(as_scalar(&matrix[0]).addr(), first.addr());
        assert_eq!(as_scalar(&matrix[1]).addr(), retained.addr());
        assert_eq!(scalar_payload(&matrix[0]), 1.0);
        assert_eq!(scalar_payload(&matrix[1]), 2.0);
    }

    #[test]
    fn state_journal_typed_value_rewinds_nested_state_without_changing_kind() {
        let cell = scalar(1.0);
        let root = Value::Typed(Box::new(scalar_value(&cell)), ValueKind::F64);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&root).unwrap();
        *cell.borrow_mut() = 2.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();

        delta.rewind().unwrap();
        assert_eq!(*cell.borrow(), 1.0);
        delta.replay().unwrap();
        assert_eq!(*cell.borrow(), 2.0);
        match root {
            Value::Typed(_, kind) => assert_eq!(kind, ValueKind::F64),
            _ => panic!("typed wrapper changed"),
        }
    }

    #[test]
    fn state_journal_capture_conflict_is_structured_and_adds_nothing() {
        let cell = scalar(1.0);
        let root = scalar_value(&cell);
        let held = cell.borrow_mut();
        let mut journal = ValueStateJournal::new();
        let error = journal.capture_value(&root).unwrap_err();

        let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
        assert_eq!(conflict.phase, "capture-before");
        assert_eq!(conflict.address, cell.addr());
        assert_eq!(conflict.type_name, type_name::<f64>());
        assert!(journal.is_empty());
        assert!(journal.roots.is_empty());
        drop(held);

        journal.capture_value(&root).unwrap();
        assert_eq!(journal.cell_count(), 1);
    }

    #[test]
    fn state_journal_nested_capture_preflight_adds_no_partial_entries() {
        let first = scalar(1.0);
        let second = scalar(2.0);
        let (_, root) = record_value(vec![
            ("first", scalar_value(&first)),
            ("second", scalar_value(&second)),
        ]);
        let held = second.borrow_mut();
        let mut journal = ValueStateJournal::new();
        let error = journal.capture_value(&root).unwrap_err();

        let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
        assert_eq!(conflict.phase, "capture-before");
        assert_eq!(conflict.address, second.addr());
        assert!(journal.entries.is_empty());
        assert!(journal.entry_indices.is_empty());
        assert!(journal.roots.is_empty());
        drop(held);

        journal.capture_value(&root).unwrap();
        assert_eq!(journal.cell_count(), 3);
    }

    #[test]
    fn state_journal_restore_preflight_is_atomic() {
        let first = scalar(1.0);
        let second = scalar(2.0);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&first)).unwrap();
        journal.capture_value(&scalar_value(&second)).unwrap();
        *first.borrow_mut() = 10.0;
        *second.borrow_mut() = 20.0;

        let held = second.borrow();
        let error = journal.restore_before().unwrap_err();
        let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
        assert_eq!(conflict.phase, "restore-before");
        assert_eq!(conflict.address, second.addr());
        assert_eq!(*first.borrow(), 10.0);
        assert_eq!(*held, 20.0);
        drop(held);

        journal.restore_before().unwrap();
        assert_eq!(*first.borrow(), 1.0);
        assert_eq!(*second.borrow(), 2.0);
    }

    #[test]
    fn state_journal_record_after_conflict_is_retryable() {
        let first = scalar(1.0);
        let second = scalar(2.0);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&first)).unwrap();
        journal.capture_value(&scalar_value(&second)).unwrap();
        *first.borrow_mut() = 10.0;
        *second.borrow_mut() = 20.0;

        let held = second.borrow_mut();
        let error = journal.record_after().unwrap_err();
        assert_eq!(
            error.kind_as::<ValueStateBorrowConflict>().unwrap().phase,
            "capture-after"
        );
        assert!(!journal.after_recorded);
        assert!(!journal.sealed);
        assert!(journal.entries.iter().all(|entry| !entry.has_after()));
        drop(held);

        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        delta.rewind().unwrap();
        assert_eq!((*first.borrow(), *second.borrow()), (1.0, 2.0));
        delta.replay().unwrap();
        assert_eq!((*first.borrow(), *second.borrow()), (10.0, 20.0));
    }

    #[test]
    fn state_journal_replay_preflight_is_atomic() {
        let first = scalar(1.0);
        let second = scalar(2.0);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&first)).unwrap();
        journal.capture_value(&scalar_value(&second)).unwrap();
        *first.borrow_mut() = 10.0;
        *second.borrow_mut() = 20.0;
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();
        delta.rewind().unwrap();

        let held = second.borrow();
        let error = delta.replay().unwrap_err();
        let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
        assert_eq!(conflict.phase, "restore-after");
        assert_eq!(conflict.address, second.addr());
        assert_eq!(*first.borrow(), 1.0);
        assert_eq!(*held, 2.0);
        drop(held);

        delta.replay().unwrap();
        assert_eq!((*first.borrow(), *second.borrow()), (10.0, 20.0));
    }

    #[test]
    fn state_journal_lifecycle_errors_are_structured() {
        let cell = scalar(1.0);
        let mut unrecorded = ValueStateJournal::new();
        unrecorded.capture_value(&scalar_value(&cell)).unwrap();
        let error = match unrecorded.into_delta() {
            Ok(_) => panic!("unrecorded journal unexpectedly produced a delta"),
            Err(error) => error,
        };
        assert!(error.kind_as::<ValueStateAfterNotRecorded>().is_some());

        let mut journal = ValueStateJournal::new();
        journal.capture_value(&scalar_value(&cell)).unwrap();
        journal.record_after().unwrap();
        assert!(
            journal
                .record_after()
                .unwrap_err()
                .kind_as::<ValueStateAfterAlreadyRecorded>()
                .is_some()
        );
        assert!(
            journal
                .capture_value(&scalar_value(&cell))
                .unwrap_err()
                .kind_as::<ValueStateJournalSealed>()
                .is_some()
        );
    }

    #[test]
    fn state_journal_non_cell_roots_remain_empty() {
        let roots = vec![
            Value::Id(1),
            Value::Kind(ValueKind::F64),
            Value::IndexAll,
            Value::EmptyKind(ValueKind::F64),
            Value::Empty,
            Value::Typed(Box::new(Value::Empty), ValueKind::F64),
        ];
        let mut journal = ValueStateJournal::new();
        for root in roots {
            journal.capture_value(&root).unwrap();
        }
        assert!(journal.is_empty());
        journal.record_after().unwrap();
        assert_eq!(journal.into_delta().unwrap().cell_count(), 0);
    }

    #[test]
    fn state_journal_erased_type_mismatch_is_structured() {
        let index = Ref::new(1usize);
        let float = scalar(1.0);
        let mut journal = ValueStateJournal::new();
        journal.capture_value(&Value::Index(index)).unwrap();

        let float_key = ValueStateKey::of(&float);
        journal.entry_indices.insert(float_key, 0);
        let error = journal.capture_value(&scalar_value(&float)).unwrap_err();
        let mismatch = error.kind_as::<ValueStateEntryTypeMismatch>().unwrap();
        assert_eq!(mismatch.type_name, type_name::<f64>());
        assert_eq!(mismatch.address, float.addr());
    }
}
