#![forbid(unsafe_code)]

use core::any::{Any, TypeId};
use core::fmt::Debug;

use crate::{LegacyValue, MResult, MechError, MechErrorKind, Ref};

impl LegacyValue {
    /// Infers a closed canonical schema and snapshots this compatibility value.
    pub fn to_canonical_value(&self) -> MResult<crate::Value> {
        let mut semantic = crate::legacy_adapter::value::InferredLegacySemanticContext;
        self.to_canonical_value_with_context(&mut semantic)
    }

    /// Canonicalizes this compatibility value using the caller's authoritative
    /// semantic resolver for every nominal identity and enum declaration.
    pub fn to_canonical_value_with_context(
        &self,
        semantic: &mut dyn crate::LegacySemanticContext,
    ) -> MResult<crate::Value> {
        // Validate the entire identity-bearing compatibility graph before kind
        // inference. Legacy `kind()` follows mutable references transparently
        // and therefore cannot itself be the cycle boundary.
        let value = self.try_deep_snapshot()?;
        let schema = crate::schema_from_legacy_value_kind(&value.kind(), semantic)?;
        let mut builder = crate::SchemaTableBuilder::new();
        let handle = builder.insert(schema)?;
        let build = builder.finish()?;
        let schema = build.resolve(handle)?;
        let schemas = build.table;
        let validation = crate::snapshot::SnapshotValidationContext::new(&schemas);
        let mut context = crate::LegacySnapshotContext::new(
            semantic,
            crate::LegacyEmptyPolicy::ResolveOptionAbsence,
            crate::LegacyReferencePolicy::SnapshotCurrentValue,
        );
        crate::snapshot_from_legacy(&value, schema, Box::new([]), &validation, &mut context)
            .map_err(Into::into)
    }

    /// Materializes a detached compatibility value from a canonical value.
    pub fn from_canonical_value(value: &crate::Value) -> MResult<Self> {
        let mut context = crate::legacy_adapter::value::InferredLegacyMaterializationContext;
        Self::from_canonical_value_with_context(value, &mut context)
    }

    /// Materializes a compatibility value with the caller's authoritative
    /// nominal dictionary. The convenience adapter deliberately rejects
    /// nominal values instead of fabricating identifiers or names.
    pub fn from_canonical_value_with_context(
        value: &crate::Value,
        context: &mut dyn crate::LegacyMaterializationContext,
    ) -> MResult<Self> {
        let schemas = value
            .schemas()
            .ok_or(crate::LegacySnapshotError::UnsupportedLegacyMaterialization)?;
        crate::legacy_from_snapshot(value, &schemas, context).map_err(Into::into)
    }

    /// Returns an exact legacy kind literal without formatting or reparsing.
    pub fn legacy_kind_literal(&self) -> Option<&crate::ValueKind> {
        match self {
            LegacyValue::Kind(kind) => Some(kind),
            _ => None,
        }
    }

    pub fn is_legacy_empty(&self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(&Self::legacy_absent_control())
    }

    pub(crate) fn legacy_absent_control() -> Self {
        LegacyValue::Empty
    }

    pub fn legacy_empty_kind(&self) -> Option<&crate::ValueKind> {
        match self {
            LegacyValue::EmptyKind(kind) => Some(kind),
            _ => None,
        }
    }

    pub fn is_legacy_index_all(&self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(&Self::legacy_matrix_all_control())
    }

    pub(crate) fn legacy_matrix_all_control() -> Self {
        LegacyValue::IndexAll
    }

    /// Returns the exact compatibility cell stored by an outer legacy mutable
    /// reference. Callers must keep the resulting borrow local.
    pub fn legacy_mutable_reference(&self) -> Option<&Ref<LegacyValue>> {
        match self {
            LegacyValue::MutableReference(reference) => Some(reference),
            _ => None,
        }
    }
}

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
use alloc::{boxed::Box, string::String, sync::Arc};

#[cfg(all(
    feature = "no_std",
    any(
        feature = "enum",
        feature = "map",
        feature = "matrix",
        feature = "record",
        feature = "set",
        feature = "table",
        feature = "tuple"
    )
))]
use alloc::vec::Vec;

#[cfg(not(feature = "no_std"))]
use std::sync::Arc;

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

#[cfg(any(feature = "map", feature = "set"))]
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

    fn validate_value(&mut self, value: &LegacyValue) -> MResult<()> {
        match value {
            LegacyValue::MutableReference(value) => {
                let key = ValueSnapshotKey::of_ref(value);
                self.visit_recursive_node(key, "mutable-reference", |detector| {
                    let value = try_clone_ref(value, "validate", "mutable-reference")?;
                    detector.validate_value(&value)
                })
            }
            LegacyValue::Typed(value, _) => self.validate_value(value),
            #[cfg(feature = "set")]
            LegacyValue::Set(value) => {
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
            LegacyValue::Map(value) => {
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
            LegacyValue::Record(value) => {
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
            LegacyValue::Table(value) => {
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
            LegacyValue::Tuple(value) => {
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
            LegacyValue::Enum(value) => {
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
            LegacyValue::MatrixValue(value) => self.validate_value_matrix(value),
            _ => Ok(()),
        }
    }

    #[cfg(feature = "matrix")]
    fn validate_value_matrix(&mut self, value: &Matrix<LegacyValue>) -> MResult<()> {
        let key = ValueSnapshotKey::of_matrix(value);
        self.visit_recursive_node(key, "matrix", |detector| {
            let (values, _, _) = value.try_clone_parts().map_err(|_| {
                snapshot_borrow_conflict::<Matrix<LegacyValue>>("validate", "matrix")
            })?;
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

    #[cfg(any(
        feature = "enum",
        feature = "map",
        feature = "record",
        feature = "set",
        feature = "table",
        feature = "tuple"
    ))]
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
        let snapshot = source.rebuild_with_same_storage(values, rows, cols);
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    #[cfg(feature = "matrix")]
    fn snapshot_value_matrix(
        &mut self,
        source: &Matrix<LegacyValue>,
    ) -> MResult<Matrix<LegacyValue>> {
        let key = ValueSnapshotKey::of_matrix(source);
        if let Some(snapshot) = self.memoized(key) {
            return Ok(snapshot);
        }

        let (values, rows, cols) = source
            .try_clone_parts()
            .map_err(|_| snapshot_borrow_conflict::<Matrix<LegacyValue>>("clone", "matrix"))?;
        let mut detached_values = Vec::new();
        for value in values {
            detached_values.push(self.snapshot_value(&value)?);
        }
        let snapshot = source.rebuild_with_same_storage(detached_values, rows, cols);
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    fn snapshot_mutable_reference(&mut self, source: &Ref<LegacyValue>) -> MResult<LegacyValue> {
        let key = ValueSnapshotKey::of_ref(source);
        if let Some(snapshot) = self.memoized::<LegacyValue>(key) {
            return Ok(snapshot);
        }

        let source_value = try_clone_ref(source, "clone", "mutable-reference")?;
        let snapshot = self.snapshot_value(&source_value)?;
        self.memoize(key, snapshot.clone());
        Ok(snapshot)
    }

    fn snapshot_value(&mut self, source: &LegacyValue) -> MResult<LegacyValue> {
        if let Some(reference) = source.legacy_mutable_reference() {
            return self.snapshot_mutable_reference(reference);
        }
        if let Some(kind) = source.legacy_kind_literal() {
            return Ok(LegacyValue::Kind(kind.clone()));
        }
        if source.is_legacy_index_all() {
            return Ok(LegacyValue::IndexAll);
        }
        if let Some(kind) = source.legacy_empty_kind() {
            return Ok(LegacyValue::EmptyKind(kind.clone()));
        }
        if source.is_legacy_empty() {
            return Ok(LegacyValue::Empty);
        }
        match source {
            #[cfg(feature = "u8")]
            LegacyValue::U8(value) => Ok(LegacyValue::U8(self.snapshot_leaf(value, "u8")?)),
            #[cfg(feature = "u16")]
            LegacyValue::U16(value) => Ok(LegacyValue::U16(self.snapshot_leaf(value, "u16")?)),
            #[cfg(feature = "u32")]
            LegacyValue::U32(value) => Ok(LegacyValue::U32(self.snapshot_leaf(value, "u32")?)),
            #[cfg(feature = "u64")]
            LegacyValue::U64(value) => Ok(LegacyValue::U64(self.snapshot_leaf(value, "u64")?)),
            #[cfg(feature = "u128")]
            LegacyValue::U128(value) => Ok(LegacyValue::U128(self.snapshot_leaf(value, "u128")?)),
            #[cfg(feature = "i8")]
            LegacyValue::I8(value) => Ok(LegacyValue::I8(self.snapshot_leaf(value, "i8")?)),
            #[cfg(feature = "i16")]
            LegacyValue::I16(value) => Ok(LegacyValue::I16(self.snapshot_leaf(value, "i16")?)),
            #[cfg(feature = "i32")]
            LegacyValue::I32(value) => Ok(LegacyValue::I32(self.snapshot_leaf(value, "i32")?)),
            #[cfg(feature = "i64")]
            LegacyValue::I64(value) => Ok(LegacyValue::I64(self.snapshot_leaf(value, "i64")?)),
            #[cfg(feature = "i128")]
            LegacyValue::I128(value) => Ok(LegacyValue::I128(self.snapshot_leaf(value, "i128")?)),
            #[cfg(feature = "f32")]
            LegacyValue::F32(value) => Ok(LegacyValue::F32(self.snapshot_leaf(value, "f32")?)),
            #[cfg(feature = "f64")]
            LegacyValue::F64(value) => Ok(LegacyValue::F64(self.snapshot_leaf(value, "f64")?)),
            #[cfg(feature = "complex")]
            LegacyValue::C64(value) => Ok(LegacyValue::C64(self.snapshot_leaf(value, "c64")?)),
            #[cfg(feature = "rational")]
            LegacyValue::R64(value) => Ok(LegacyValue::R64(self.snapshot_leaf(value, "r64")?)),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(value) => {
                Ok(LegacyValue::String(self.snapshot_leaf(value, "string")?))
            }
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(value) => Ok(LegacyValue::Bool(self.snapshot_leaf(value, "bool")?)),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(value) => Ok(LegacyValue::Atom(self.snapshot_atom(value)?)),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(value) => {
                Ok(LegacyValue::MatrixIndex(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(value) => {
                Ok(LegacyValue::MatrixBool(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(value) => Ok(LegacyValue::MatrixU8(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(value) => {
                Ok(LegacyValue::MatrixU16(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(value) => {
                Ok(LegacyValue::MatrixU32(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(value) => {
                Ok(LegacyValue::MatrixU64(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(value) => {
                Ok(LegacyValue::MatrixU128(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(value) => Ok(LegacyValue::MatrixI8(self.snapshot_matrix(value)?)),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(value) => {
                Ok(LegacyValue::MatrixI16(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(value) => {
                Ok(LegacyValue::MatrixI32(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(value) => {
                Ok(LegacyValue::MatrixI64(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(value) => {
                Ok(LegacyValue::MatrixI128(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(value) => {
                Ok(LegacyValue::MatrixF32(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(value) => {
                Ok(LegacyValue::MatrixF64(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(value) => {
                Ok(LegacyValue::MatrixString(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(value) => {
                Ok(LegacyValue::MatrixR64(self.snapshot_matrix(value)?))
            }
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(value) => {
                Ok(LegacyValue::MatrixC64(self.snapshot_matrix(value)?))
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(value) => {
                Ok(LegacyValue::MatrixValue(self.snapshot_value_matrix(value)?))
            }
            #[cfg(feature = "set")]
            LegacyValue::Set(value) => {
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
                Ok(LegacyValue::Set(snapshot))
            }
            #[cfg(feature = "map")]
            LegacyValue::Map(value) => {
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
                Ok(LegacyValue::Map(snapshot))
            }
            #[cfg(feature = "record")]
            LegacyValue::Record(value) => {
                let snapshot =
                    self.snapshot_recursive_ref(value, "record", |context, mut record| {
                        let mut detached = IndexMap::new();
                        for (id, value) in record.data {
                            detached.insert(id, context.snapshot_value(&value)?);
                        }
                        record.data = detached;
                        Ok(record)
                    })?;
                Ok(LegacyValue::Record(snapshot))
            }
            #[cfg(feature = "table")]
            LegacyValue::Table(value) => {
                let snapshot =
                    self.snapshot_recursive_ref(value, "table", |context, mut table| {
                        let mut detached = IndexMap::new();
                        for (id, (kind, values)) in table.data {
                            detached.insert(id, (kind, context.snapshot_value_matrix(&values)?));
                        }
                        table.data = detached;
                        Ok(table)
                    })?;
                Ok(LegacyValue::Table(snapshot))
            }
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(value) => {
                let snapshot = self.snapshot_recursive_ref(value, "tuple", |context, tuple| {
                    let mut detached = Vec::new();
                    for value in tuple.elements {
                        detached.push(context.snapshot_value(&value)?);
                    }
                    Ok(MechTuple::from_vec(detached))
                })?;
                Ok(LegacyValue::Tuple(snapshot))
            }
            #[cfg(feature = "enum")]
            LegacyValue::Enum(value) => {
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
                Ok(LegacyValue::Enum(snapshot))
            }
            LegacyValue::Id(value) => Ok(LegacyValue::Id(*value)),
            LegacyValue::Index(value) => {
                Ok(LegacyValue::Index(self.snapshot_leaf(value, "index")?))
            }
            LegacyValue::Typed(value, kind) => Ok(LegacyValue::Typed(
                Box::new(self.snapshot_value(value)?),
                kind.clone(),
            )),
            _ => unreachable!("legacy pseudo-values are normalized before snapshot matching"),
        }
    }
}

pub(crate) fn try_deep_snapshot(source: &LegacyValue) -> MResult<LegacyValue> {
    ValueSnapshotCycleDetector::default().validate_value(source)?;
    ValueSnapshotCloneContext::default().snapshot_value(source)
}

/// A thread-safe recipe for recreating detached copies of a [`LegacyValue`].
///
/// `LegacyValue` itself is deliberately single-threaded, so runtime planning cannot
/// retain one inside a `Send + Sync` function specializer. Constructing the
/// recipe here keeps the exhaustive match in the same feature domain as the
/// `LegacyValue` enum and prevents downstream Cargo feature unification from making
/// another crate's match incomplete.
#[derive(Clone)]
pub struct ValueSnapshotRecreator {
    recreate: Arc<dyn Fn() -> LegacyValue + Send + Sync>,
}

impl ValueSnapshotRecreator {
    fn new(recreate: impl Fn() -> LegacyValue + Send + Sync + 'static) -> Self {
        Self {
            recreate: Arc::new(recreate),
        }
    }

    pub fn capture(value: &LegacyValue) -> MResult<Self> {
        Self::capture_detached(value.try_deep_snapshot()?)
    }

    fn capture_detached(value: LegacyValue) -> MResult<Self> {
        macro_rules! scalar {
            ($value:expr, $variant:ident) => {{
                let value = $value.borrow().clone();
                Ok(Self::new(move || {
                    LegacyValue::$variant(Ref::new(value.clone()))
                }))
            }};
        }

        match value {
            #[cfg(feature = "u8")]
            LegacyValue::U8(value) => scalar!(value, U8),
            #[cfg(feature = "u16")]
            LegacyValue::U16(value) => scalar!(value, U16),
            #[cfg(feature = "u32")]
            LegacyValue::U32(value) => scalar!(value, U32),
            #[cfg(feature = "u64")]
            LegacyValue::U64(value) => scalar!(value, U64),
            #[cfg(feature = "u128")]
            LegacyValue::U128(value) => scalar!(value, U128),
            #[cfg(feature = "i8")]
            LegacyValue::I8(value) => scalar!(value, I8),
            #[cfg(feature = "i16")]
            LegacyValue::I16(value) => scalar!(value, I16),
            #[cfg(feature = "i32")]
            LegacyValue::I32(value) => scalar!(value, I32),
            #[cfg(feature = "i64")]
            LegacyValue::I64(value) => scalar!(value, I64),
            #[cfg(feature = "i128")]
            LegacyValue::I128(value) => scalar!(value, I128),
            #[cfg(feature = "f32")]
            LegacyValue::F32(value) => scalar!(value, F32),
            #[cfg(feature = "f64")]
            LegacyValue::F64(value) => scalar!(value, F64),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(value) => scalar!(value, String),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(value) => scalar!(value, Bool),
            #[cfg(feature = "complex")]
            LegacyValue::C64(value) => scalar!(value, C64),
            #[cfg(feature = "rational")]
            LegacyValue::R64(value) => scalar!(value, R64),
            LegacyValue::Index(value) => scalar!(value, Index),
            LegacyValue::Id(value) => Ok(Self::new(move || LegacyValue::Id(value))),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixBool))
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixIndex))
            }
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixU8))
            }
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixU16))
            }
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixU32))
            }
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixU64))
            }
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixU128))
            }
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixI8))
            }
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixI16))
            }
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixI32))
            }
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixI64))
            }
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixI128))
            }
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixF32))
            }
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixF64))
            }
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixString))
            }
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixR64))
            }
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(value) => {
                Ok(capture_snapshot_matrix(value, LegacyValue::MatrixC64))
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(value) => {
                let matrix = ValueSnapshotRecreatorMatrix::capture(value)?;
                Ok(Self::new(move || {
                    LegacyValue::MatrixValue(matrix.to_matrix())
                }))
            }
            #[cfg(feature = "atom")]
            LegacyValue::Atom(value) => {
                let value = value.borrow();
                let id = value.id();
                let dictionary = value.dictionary().borrow().clone();
                Ok(Self::new(move || {
                    LegacyValue::Atom(Ref::new(MechAtom((id, Ref::new(dictionary.clone())))))
                }))
            }
            #[cfg(feature = "set")]
            LegacyValue::Set(value) => {
                let value = value.borrow();
                let kind = value.kind.clone();
                let max_elements = value.max_elements;
                let num_elements = value.num_elements;
                let elements = value
                    .set
                    .iter()
                    .map(|value| Self::capture_detached(value.clone()))
                    .collect::<MResult<Vec<_>>>()?;
                Ok(Self::new(move || {
                    let mut value =
                        MechSet::from_vec(elements.iter().map(Self::to_value).collect());
                    value.kind = kind.clone();
                    value.max_elements = max_elements;
                    value.num_elements = num_elements;
                    LegacyValue::Set(Ref::new(value))
                }))
            }
            #[cfg(feature = "map")]
            LegacyValue::Map(value) => {
                let value = value.borrow();
                let key_kind = value.key_kind.clone();
                let value_kind = value.value_kind.clone();
                let num_elements = value.num_elements;
                let entries = value
                    .map
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            Self::capture_detached(key.clone())?,
                            Self::capture_detached(value.clone())?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(Self::new(move || {
                    LegacyValue::Map(Ref::new(MechMap::from_typed_vec(
                        key_kind.clone(),
                        value_kind.clone(),
                        num_elements,
                        entries
                            .iter()
                            .map(|(key, value)| (key.to_value(), value.to_value()))
                            .collect(),
                    )))
                }))
            }
            #[cfg(feature = "record")]
            LegacyValue::Record(value) => {
                let value = value.borrow();
                let cols = value.cols;
                let kinds = value.kinds.clone();
                let fields = value
                    .data
                    .iter()
                    .map(|(id, field)| {
                        Ok((
                            *id,
                            value.field_names.get(id).cloned().unwrap_or_default(),
                            Self::capture_detached(field.clone())?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(Self::new(move || {
                    LegacyValue::Record(Ref::new(MechRecord::from_parts(
                        cols,
                        kinds.clone(),
                        fields
                            .iter()
                            .map(|(id, name, value)| (*id, name.clone(), value.to_value()))
                            .collect(),
                    )))
                }))
            }
            #[cfg(feature = "table")]
            LegacyValue::Table(value) => {
                let value = value.borrow();
                let rows = value.rows;
                let cols = value.cols;
                let columns = value
                    .data
                    .iter()
                    .map(|(id, (kind, values))| {
                        Ok((
                            *id,
                            kind.clone(),
                            ValueSnapshotRecreatorMatrix::capture(values.clone())?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?;
                let column_names = value
                    .col_names
                    .iter()
                    .map(|(id, name)| (*id, name.clone()))
                    .collect::<Vec<_>>();
                Ok(Self::new(move || {
                    LegacyValue::Table(Ref::new(MechTable::from_parts(
                        rows,
                        cols,
                        columns
                            .iter()
                            .map(|(id, kind, values)| (*id, kind.clone(), values.to_matrix()))
                            .collect(),
                        column_names.clone(),
                    )))
                }))
            }
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(value) => {
                let elements = value
                    .borrow()
                    .elements
                    .iter()
                    .map(|value| Self::capture_detached((**value).clone()))
                    .collect::<MResult<Vec<_>>>()?;
                Ok(Self::new(move || {
                    LegacyValue::Tuple(Ref::new(MechTuple::from_vec(
                        elements.iter().map(Self::to_value).collect(),
                    )))
                }))
            }
            #[cfg(feature = "enum")]
            LegacyValue::Enum(value) => {
                let value = value.borrow();
                let id = value.id;
                let variants = value
                    .variants
                    .iter()
                    .map(|(id, payload)| {
                        Ok((
                            *id,
                            payload
                                .as_ref()
                                .map(|value| Self::capture_detached(value.clone()))
                                .transpose()?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?;
                let names = value.names.borrow().clone();
                Ok(Self::new(move || {
                    LegacyValue::Enum(Ref::new(MechEnum {
                        id,
                        variants: variants
                            .iter()
                            .map(|(id, payload)| (*id, payload.as_ref().map(Self::to_value)))
                            .collect(),
                        names: Ref::new(names.clone()),
                    }))
                }))
            }
            LegacyValue::MutableReference(value) => {
                let inner = Self::capture_detached(value.borrow().clone())?;
                Ok(Self::new(move || {
                    LegacyValue::MutableReference(Ref::new(inner.to_value()))
                }))
            }
            LegacyValue::Typed(value, kind) => {
                let inner = Self::capture_detached(*value)?;
                Ok(Self::new(move || {
                    LegacyValue::Typed(Box::new(inner.to_value()), kind.clone())
                }))
            }
            LegacyValue::Kind(kind) => Ok(Self::new(move || LegacyValue::Kind(kind.clone()))),
            LegacyValue::IndexAll => Ok(Self::new(|| LegacyValue::IndexAll)),
            LegacyValue::EmptyKind(kind) => {
                Ok(Self::new(move || LegacyValue::EmptyKind(kind.clone())))
            }
            LegacyValue::Empty => Ok(Self::new(|| LegacyValue::Empty)),
        }
    }

    pub fn to_value(&self) -> LegacyValue {
        (self.recreate)()
    }
}

#[cfg(feature = "matrix")]
#[derive(Clone)]
struct ValueSnapshotRecreatorMatrix {
    storage: crate::matrix::MatrixStorageForm,
    rows: usize,
    cols: usize,
    elements: Vec<ValueSnapshotRecreator>,
}

#[cfg(feature = "matrix")]
impl ValueSnapshotRecreatorMatrix {
    fn capture(matrix: Matrix<LegacyValue>) -> MResult<Self> {
        let storage = matrix.storage_form();
        let rows = matrix.rows();
        let cols = matrix.cols();
        let elements = matrix
            .as_vec()
            .into_iter()
            .map(ValueSnapshotRecreator::capture_detached)
            .collect::<MResult<Vec<_>>>()?;
        Ok(Self {
            storage,
            rows,
            cols,
            elements,
        })
    }

    fn to_matrix(&self) -> Matrix<LegacyValue> {
        Matrix::from_storage_form(
            self.storage,
            self.elements
                .iter()
                .map(ValueSnapshotRecreator::to_value)
                .collect(),
            self.rows,
            self.cols,
        )
    }
}

#[cfg(feature = "matrix")]
fn capture_snapshot_matrix<T>(
    matrix: Matrix<T>,
    wrap: fn(Matrix<T>) -> LegacyValue,
) -> ValueSnapshotRecreator
where
    T: Clone + Send + Sync + 'static,
{
    macro_rules! matrix {
        ($value:expr, $variant:ident) => {{
            let value = $value.borrow().clone();
            ValueSnapshotRecreator::new(move || wrap(Matrix::$variant(Ref::new(value.clone()))))
        }};
    }

    match matrix {
        #[cfg(feature = "matrix1")]
        Matrix::Matrix1(value) => matrix!(value, Matrix1),
        #[cfg(feature = "matrix2")]
        Matrix::Matrix2(value) => matrix!(value, Matrix2),
        #[cfg(feature = "matrix3")]
        Matrix::Matrix3(value) => matrix!(value, Matrix3),
        #[cfg(feature = "matrix4")]
        Matrix::Matrix4(value) => matrix!(value, Matrix4),
        #[cfg(feature = "matrix2x3")]
        Matrix::Matrix2x3(value) => matrix!(value, Matrix2x3),
        #[cfg(feature = "matrix3x2")]
        Matrix::Matrix3x2(value) => matrix!(value, Matrix3x2),
        #[cfg(feature = "row_vector2")]
        Matrix::RowVector2(value) => matrix!(value, RowVector2),
        #[cfg(feature = "row_vector3")]
        Matrix::RowVector3(value) => matrix!(value, RowVector3),
        #[cfg(feature = "row_vector4")]
        Matrix::RowVector4(value) => matrix!(value, RowVector4),
        #[cfg(feature = "vector2")]
        Matrix::Vector2(value) => matrix!(value, Vector2),
        #[cfg(feature = "vector3")]
        Matrix::Vector3(value) => matrix!(value, Vector3),
        #[cfg(feature = "vector4")]
        Matrix::Vector4(value) => matrix!(value, Vector4),
        #[cfg(feature = "row_vectord")]
        Matrix::RowDVector(value) => matrix!(value, RowDVector),
        #[cfg(feature = "vectord")]
        Matrix::DVector(value) => matrix!(value, DVector),
        #[cfg(feature = "matrixd")]
        Matrix::DMatrix(value) => matrix!(value, DMatrix),
    }
}
