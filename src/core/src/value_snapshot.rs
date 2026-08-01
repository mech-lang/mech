#![forbid(unsafe_code)]

use core::any::{Any, TypeId};
use core::fmt::Debug;

use crate::{MResult, MechError, MechErrorKind, Ref, ValRef, Value};

#[cfg(feature = "enum")]
use crate::MechEnum;
#[cfg(feature = "map")]
use crate::MechMap;
#[cfg(feature = "record")]
use crate::MechRecord;
#[cfg(feature = "set")]
use crate::MechSet;
#[cfg(feature = "table")]
use crate::MechTable;
#[cfg(feature = "tuple")]
use crate::MechTuple;
#[cfg(feature = "matrix")]
use crate::structures::matrix::Matrix;
#[cfg(feature = "atom")]
use crate::{Dictionary, MechAtom};

#[cfg(any(feature = "map", feature = "record", feature = "table"))]
use indexmap::IndexMap;
#[cfg(feature = "set")]
use indexmap::IndexSet;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, vec::Vec};

#[cfg(feature = "no_std")]
use core::hash::BuildHasherDefault;

#[cfg(feature = "no_std")]
use fxhash::FxHasher;

#[cfg(feature = "no_std")]
use hashbrown::{HashMap as HashBrownMap, HashSet as HashBrownSet};

#[cfg(feature = "no_std")]
type SnapshotMap<K, V> = HashBrownMap<K, V, BuildHasherDefault<FxHasher>>;

#[cfg(feature = "no_std")]
type SnapshotSet<K> = HashBrownSet<K, BuildHasherDefault<FxHasher>>;

#[cfg(not(feature = "no_std"))]
type SnapshotMap<K, V> = std::collections::HashMap<K, V>;

#[cfg(not(feature = "no_std"))]
type SnapshotSet<K> = std::collections::HashSet<K>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ValueSnapshotKey {
    type_id: TypeId,
    address: usize,
}

impl ValueSnapshotKey {
    fn of_ref<T: 'static>(value: &Ref<T>) -> Self {
        Self {
            type_id: TypeId::of::<Ref<T>>(),
            address: value.addr(),
        }
    }

    #[cfg(feature = "matrix")]
    fn of_matrix<T: 'static>(value: &Matrix<T>) -> Self {
        Self {
            type_id: TypeId::of::<Matrix<T>>(),
            address: value.addr(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValueSnapshotCycleUnsupported {
    pub node: &'static str,
}

impl MechErrorKind for ValueSnapshotCycleUnsupported {
    fn name(&self) -> &str {
        "ValueSnapshotCycleUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "detached value snapshots do not support cyclic `{}` graphs",
            self.node,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ValueSnapshotBorrowConflict {
    pub phase: &'static str,
    pub node: &'static str,
    pub type_name: &'static str,
}

impl MechErrorKind for ValueSnapshotBorrowConflict {
    fn name(&self) -> &str {
        "ValueSnapshotBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "detached value snapshot could not {} `{}` because `{}` is already mutably borrowed",
            self.phase, self.node, self.type_name,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ValueSnapshotCollectionCollision {
    pub collection: &'static str,
    pub first_index: usize,
    pub second_index: usize,
}

impl MechErrorKind for ValueSnapshotCollectionCollision {
    fn name(&self) -> &str {
        "ValueSnapshotCollectionCollision"
    }

    fn message(&self) -> String {
        format!(
            "detached value snapshot found duplicate-equal {} entries at indices {} and {}",
            self.collection, self.first_index, self.second_index,
        )
    }
}

fn snapshot_collection_collision(
    collection: &'static str,
    first_index: usize,
    second_index: usize,
) -> MechError {
    MechError::new(
        ValueSnapshotCollectionCollision {
            collection,
            first_index,
            second_index,
        },
        None,
    )
    .with_compiler_loc()
}

fn snapshot_borrow_conflict<T>(phase: &'static str, node: &'static str) -> MechError
where
    T: 'static,
{
    MechError::new(
        ValueSnapshotBorrowConflict {
            phase,
            node,
            type_name: core::any::type_name::<T>(),
        },
        None,
    )
    .with_compiler_loc()
}

fn try_clone_ref<T>(source: &Ref<T>, phase: &'static str, node: &'static str) -> MResult<T>
where
    T: Clone + 'static,
{
    source
        .try_borrow()
        .map(|value| value.clone())
        .map_err(|_| snapshot_borrow_conflict::<T>(phase, node))
}

#[derive(Default)]
struct ValueSnapshotCycleDetector {
    visiting: SnapshotSet<ValueSnapshotKey>,
    completed: SnapshotSet<ValueSnapshotKey>,
}

impl ValueSnapshotCycleDetector {
    fn visit_recursive_node(
        &mut self,
        key: ValueSnapshotKey,
        node: &'static str,
        descend: impl FnOnce(&mut Self) -> MResult<()>,
    ) -> MResult<()> {
        if self.completed.contains(&key) {
            return Ok(());
        }
        if !self.visiting.insert(key) {
            return Err(
                MechError::new(ValueSnapshotCycleUnsupported { node }, None).with_compiler_loc()
            );
        }

        let result = descend(self);
        self.visiting.remove(&key);
        if result.is_ok() {
            self.completed.insert(key);
        }
        result
    }

    fn validate_value(&mut self, value: &Value) -> MResult<()> {
        match value {
            Value::MutableReference(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "mutable-reference", |detector| {
                    let value = try_clone_ref(value, "validate", "mutable-reference")?;
                    detector.validate_value(&value)
                })
            }
            Value::Typed(value, _) => self.validate_value(value),
            #[cfg(feature = "set")]
            Value::Set(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "set", |detector| {
                    let set = try_clone_ref(value, "validate", "set")?;
                    for value in set.set {
                        detector.validate_value(&value)?;
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "map")]
            Value::Map(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "map", |detector| {
                    let map = try_clone_ref(value, "validate", "map")?;
                    for (key, value) in map.map {
                        detector.validate_value(&key)?;
                        detector.validate_value(&value)?;
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "record")]
            Value::Record(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "record", |detector| {
                    let record = try_clone_ref(value, "validate", "record")?;
                    for value in record.data.into_values() {
                        detector.validate_value(&value)?;
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "table")]
            Value::Table(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "table", |detector| {
                    let table = try_clone_ref(value, "validate", "table")?;
                    for (_, matrix) in table.data.into_values() {
                        detector.validate_value_matrix(&matrix)?;
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "tuple")]
            Value::Tuple(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "tuple", |detector| {
                    let tuple = try_clone_ref(value, "validate", "tuple")?;
                    for value in tuple.elements {
                        detector.validate_value(&value)?;
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "enum")]
            Value::Enum(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "enum", |detector| {
                    let value = try_clone_ref(value, "validate", "enum")?;
                    for (_, payload) in value.variants {
                        if let Some(payload) = payload {
                            detector.validate_value(&payload)?;
                        }
                    }
                    Ok(())
                })
            }
            #[cfg(feature = "matrix")]
            Value::MatrixValue(value) => self.validate_value_matrix(value),
            _ => Ok(()),
        }
    }

    #[cfg(feature = "matrix")]
    fn validate_value_matrix(&mut self, value: &Matrix<Value>) -> MResult<()> {
        let key = ValueSnapshotKey::of_matrix(value);
        self.visit_recursive_node(key, "matrix", |detector| {
            let (values, _, _) = value
                .try_clone_parts()
                .map_err(|_| snapshot_borrow_conflict::<Matrix<Value>>("validate", "matrix"))?;
            for value in values {
                detector.validate_value(&value)?;
            }
            Ok(())
        })
    }
}

#[derive(Default)]
struct ValueSnapshotCloneContext {
    nodes: SnapshotMap<ValueSnapshotKey, Box<dyn Any>>,
}

impl ValueSnapshotCloneContext {
    fn memoized<T>(&self, key: ValueSnapshotKey) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.nodes.get(&key).map(|node| {
            node.downcast_ref::<T>()
                .expect("value snapshot memo contained a mismatched node type")
                .clone()
        })
    }

    fn memoize<T>(&mut self, key: ValueSnapshotKey, value: T)
    where
        T: 'static,
    {
        self.nodes.insert(key, Box::new(value));
    }

    fn snapshot_leaf<T>(&mut self, source: &Ref<T>, node: &'static str) -> MResult<Ref<T>>
    where
        T: Clone + 'static,
    {
        let key = ValueSnapshotKey::of_ref(source);
        if let Some(snapshot) = self.memoized(key) {
            return Ok(snapshot);
        }
        let value = try_clone_ref(source, "clone", node)?;
        let snapshot = Ref::new(value);
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    fn snapshot_recursive_ref<T, B>(
        &mut self,
        source: &Ref<T>,
        node: &'static str,
        build: B,
    ) -> MResult<Ref<T>>
    where
        T: Clone + 'static,
        B: FnOnce(&mut Self, T) -> MResult<T>,
    {
        let key = ValueSnapshotKey::of_ref(source);
        if let Some(snapshot) = self.memoized(key) {
            return Ok(snapshot);
        }

        let source_value = try_clone_ref(source, "clone", node)?;
        let snapshot_value = build(self, source_value)?;
        let snapshot = Ref::new(snapshot_value);
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    #[cfg(feature = "atom")]
    fn snapshot_atom(&mut self, source: &Ref<MechAtom>) -> MResult<Ref<MechAtom>> {
        let key = ValueSnapshotKey::of_ref(source);
        if let Some(snapshot) = self.memoized(key) {
            return Ok(snapshot);
        }

        let atom = try_clone_ref(source, "clone", "atom")?;
        let dictionary: Ref<Dictionary> =
            self.snapshot_leaf(&atom.dictionary(), "atom-dictionary")?;
        let snapshot = Ref::new(MechAtom((atom.id(), dictionary)));
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    #[cfg(feature = "matrix")]
    fn snapshot_matrix<T>(&mut self, source: &Matrix<T>) -> MResult<Matrix<T>>
    where
        T: Debug + Clone + PartialEq + 'static,
    {
        let key = ValueSnapshotKey::of_matrix(source);
        if let Some(snapshot) = self.memoized(key) {
            return Ok(snapshot);
        }

        let (values, rows, cols) = source
            .try_clone_parts()
            .map_err(|_| snapshot_borrow_conflict::<Matrix<T>>("clone", "matrix"))?;
        let snapshot = Matrix::from_vec(values, rows, cols);
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    #[cfg(feature = "matrix")]
    fn snapshot_value_matrix(&mut self, source: &Matrix<Value>) -> MResult<Matrix<Value>> {
        let key = ValueSnapshotKey::of_matrix(source);
        if let Some(snapshot) = self.memoized(key) {
            return Ok(snapshot);
        }

        let (values, rows, cols) = source
            .try_clone_parts()
            .map_err(|_| snapshot_borrow_conflict::<Matrix<Value>>("clone", "matrix"))?;
        let mut detached_values = Vec::new();
        for value in values {
            detached_values.push(self.snapshot_value(&value)?);
        }
        let snapshot = Matrix::from_vec(detached_values, rows, cols);
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    fn snapshot_mutable_reference(&mut self, source: &ValRef) -> MResult<Value> {
        let key = ValueSnapshotKey::of_ref(source);
        if let Some(snapshot) = self.memoized::<Value>(key) {
            return Ok(snapshot);
        }

        let source_value = try_clone_ref(source, "clone", "mutable-reference")?;
        let snapshot = self.snapshot_value(&source_value)?;
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    fn snapshot_value(&mut self, source: &Value) -> MResult<Value> {
        match source {
            #[cfg(feature = "u8")]
            Value::U8(value) => Ok(Value::U8(self.snapshot_leaf(value, "u8")?)),
            #[cfg(feature = "u16")]
            Value::U16(value) => Ok(Value::U16(self.snapshot_leaf(value, "u16")?)),
            #[cfg(feature = "u32")]
            Value::U32(value) => Ok(Value::U32(self.snapshot_leaf(value, "u32")?)),
            #[cfg(feature = "u64")]
            Value::U64(value) => Ok(Value::U64(self.snapshot_leaf(value, "u64")?)),
            #[cfg(feature = "u128")]
            Value::U128(value) => Ok(Value::U128(self.snapshot_leaf(value, "u128")?)),
            #[cfg(feature = "i8")]
            Value::I8(value) => Ok(Value::I8(self.snapshot_leaf(value, "i8")?)),
            #[cfg(feature = "i16")]
            Value::I16(value) => Ok(Value::I16(self.snapshot_leaf(value, "i16")?)),
            #[cfg(feature = "i32")]
            Value::I32(value) => Ok(Value::I32(self.snapshot_leaf(value, "i32")?)),
            #[cfg(feature = "i64")]
            Value::I64(value) => Ok(Value::I64(self.snapshot_leaf(value, "i64")?)),
            #[cfg(feature = "i128")]
            Value::I128(value) => Ok(Value::I128(self.snapshot_leaf(value, "i128")?)),
            #[cfg(feature = "f32")]
            Value::F32(value) => Ok(Value::F32(self.snapshot_leaf(value, "f32")?)),
            #[cfg(feature = "f64")]
            Value::F64(value) => Ok(Value::F64(self.snapshot_leaf(value, "f64")?)),
            #[cfg(feature = "complex")]
            Value::C64(value) => Ok(Value::C64(self.snapshot_leaf(value, "c64")?)),
            #[cfg(feature = "rational")]
            Value::R64(value) => Ok(Value::R64(self.snapshot_leaf(value, "r64")?)),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            Value::String(value) => Ok(Value::String(self.snapshot_leaf(value, "string")?)),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            Value::Bool(value) => Ok(Value::Bool(self.snapshot_leaf(value, "bool")?)),
            #[cfg(feature = "atom")]
            Value::Atom(value) => Ok(Value::Atom(self.snapshot_atom(value)?)),
            #[cfg(feature = "matrix")]
            Value::MatrixIndex(value) => Ok(Value::MatrixIndex(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            Value::MatrixBool(value) => Ok(Value::MatrixBool(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            Value::MatrixU8(value) => Ok(Value::MatrixU8(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            Value::MatrixU16(value) => Ok(Value::MatrixU16(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            Value::MatrixU32(value) => Ok(Value::MatrixU32(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            Value::MatrixU64(value) => Ok(Value::MatrixU64(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            Value::MatrixU128(value) => Ok(Value::MatrixU128(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            Value::MatrixI8(value) => Ok(Value::MatrixI8(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            Value::MatrixI16(value) => Ok(Value::MatrixI16(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            Value::MatrixI32(value) => Ok(Value::MatrixI32(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            Value::MatrixI64(value) => Ok(Value::MatrixI64(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            Value::MatrixI128(value) => Ok(Value::MatrixI128(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            Value::MatrixF32(value) => Ok(Value::MatrixF32(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            Value::MatrixF64(value) => Ok(Value::MatrixF64(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "string"))]
            Value::MatrixString(value) => Ok(Value::MatrixString(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            Value::MatrixR64(value) => Ok(Value::MatrixR64(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            Value::MatrixC64(value) => Ok(Value::MatrixC64(self.snapshot_matrix(value)?)),
            #[cfg(feature = "matrix")]
            Value::MatrixValue(value) => Ok(Value::MatrixValue(self.snapshot_value_matrix(value)?)),
            #[cfg(feature = "set")]
            Value::Set(value) => {
                let snapshot = self.snapshot_recursive_ref(value, "set", |context, mut set| {
                    let mut detached = IndexSet::with_capacity(set.set.len());
                    for (second_index, value) in set.set.into_iter().enumerate() {
                        let element = context.snapshot_value(&value)?;
                        if let Some(first_index) =
                            detached.iter().position(|existing| existing == &element)
                        {
                            return Err(snapshot_collection_collision(
                                "set element",
                                first_index,
                                second_index,
                            ));
                        }
                        detached.insert(element);
                    }
                    set.set = detached;
                    Ok(set)
                })?;
                Ok(Value::Set(snapshot))
            }
            #[cfg(feature = "map")]
            Value::Map(value) => {
                let snapshot = self.snapshot_recursive_ref(value, "map", |context, mut map| {
                    let mut detached = IndexMap::with_capacity(map.map.len());
                    for (second_index, (source_key, source_value)) in
                        map.map.into_iter().enumerate()
                    {
                        let key = context.snapshot_value(&source_key)?;
                        if let Some(first_index) =
                            detached.keys().position(|existing| existing == &key)
                        {
                            return Err(snapshot_collection_collision(
                                "map key",
                                first_index,
                                second_index,
                            ));
                        }
                        let value = context.snapshot_value(&source_value)?;
                        detached.insert(key, value);
                    }
                    map.map = detached;
                    Ok(map)
                })?;
                Ok(Value::Map(snapshot))
            }
            #[cfg(feature = "record")]
            Value::Record(value) => {
                let snapshot =
                    self.snapshot_recursive_ref(value, "record", |context, mut record| {
                        let mut detached = IndexMap::new();
                        for (id, value) in record.data {
                            detached.insert(id, context.snapshot_value(&value)?);
                        }
                        record.data = detached;
                        Ok(record)
                    })?;
                Ok(Value::Record(snapshot))
            }
            #[cfg(feature = "table")]
            Value::Table(value) => {
                let snapshot =
                    self.snapshot_recursive_ref(value, "table", |context, mut table| {
                        let mut detached = IndexMap::new();
                        for (id, (kind, values)) in table.data {
                            detached.insert(id, (kind, context.snapshot_value_matrix(&values)?));
                        }
                        table.data = detached;
                        Ok(table)
                    })?;
                Ok(Value::Table(snapshot))
            }
            #[cfg(feature = "tuple")]
            Value::Tuple(value) => {
                let snapshot = self.snapshot_recursive_ref(value, "tuple", |context, tuple| {
                    let mut detached = Vec::new();
                    for value in tuple.elements {
                        detached.push(context.snapshot_value(&value)?);
                    }
                    Ok(MechTuple::from_vec(detached))
                })?;
                Ok(Value::Tuple(snapshot))
            }
            #[cfg(feature = "enum")]
            Value::Enum(value) => {
                let snapshot =
                    self.snapshot_recursive_ref(value, "enum", |context, mut value| {
                        value.names = context.snapshot_leaf(&value.names, "enum-dictionary")?;
                        let mut detached = Vec::new();
                        for (id, payload) in value.variants {
                            detached.push((
                                id,
                                match payload {
                                    Some(payload) => Some(context.snapshot_value(&payload)?),
                                    None => None,
                                },
                            ));
                        }
                        value.variants = detached;
                        Ok(value)
                    })?;
                Ok(Value::Enum(snapshot))
            }
            Value::Id(value) => Ok(Value::Id(*value)),
            Value::Index(value) => Ok(Value::Index(self.snapshot_leaf(value, "index")?)),
            Value::MutableReference(value) => self.snapshot_mutable_reference(value),
            Value::Typed(value, kind) => Ok(Value::Typed(
                Box::new(self.snapshot_value(value)?),
                kind.clone(),
            )),
            Value::Kind(kind) => Ok(Value::Kind(kind.clone())),
            Value::IndexAll => Ok(Value::IndexAll),
            Value::EmptyKind(kind) => Ok(Value::EmptyKind(kind.clone())),
            Value::Empty => Ok(Value::Empty),
        }
    }
}

pub(crate) fn try_deep_snapshot(source: &Value) -> MResult<Value> {
    ValueSnapshotCycleDetector::default().validate_value(source)?;
    ValueSnapshotCloneContext::default().snapshot_value(source)
}
