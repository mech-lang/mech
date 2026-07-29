#![forbid(unsafe_code)]

use core::any::{Any, TypeId};
use core::fmt::Debug;

use crate::{Ref, ValRef, Value};

#[cfg(feature = "atom")]
use crate::{Dictionary, MechAtom};
#[cfg(feature = "enum")]
use crate::MechEnum;
#[cfg(feature = "map")]
use crate::MechMap;
#[cfg(feature = "matrix")]
use crate::structures::matrix::Matrix;
#[cfg(feature = "record")]
use crate::MechRecord;
#[cfg(feature = "set")]
use crate::MechSet;
#[cfg(feature = "table")]
use crate::MechTable;
#[cfg(feature = "tuple")]
use crate::MechTuple;

#[cfg(any(feature = "map", feature = "record", feature = "table"))]
use indexmap::IndexMap;
#[cfg(feature = "set")]
use indexmap::IndexSet;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "no_std")]
use core::hash::BuildHasherDefault;

#[cfg(feature = "no_std")]
use fxhash::FxHasher;

#[cfg(feature = "no_std")]
use hashbrown::{
  HashMap as HashBrownMap,
  HashSet as HashBrownSet,
};

#[cfg(feature = "no_std")]
type SnapshotMap<K, V> =
  HashBrownMap<K, V, BuildHasherDefault<FxHasher>>;

#[cfg(feature = "no_std")]
type SnapshotSet<K> =
  HashBrownSet<K, BuildHasherDefault<FxHasher>>;

#[cfg(not(feature = "no_std"))]
type SnapshotMap<K, V> =
  std::collections::HashMap<K, V>;

#[cfg(not(feature = "no_std"))]
type SnapshotSet<K> =
  std::collections::HashSet<K>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ValueSnapshotKey {
  type_id: TypeId,
  address: usize,
}

impl ValueSnapshotKey {
  fn of_ref<T: 'static>(
    value: &Ref<T>,
  ) -> Self {
    Self {
      type_id: TypeId::of::<Ref<T>>(),
      address: value.addr(),
    }
  }

  #[cfg(feature = "matrix")]
  fn of_matrix<T: 'static>(
    value: &Matrix<T>,
  ) -> Self {
    Self {
      type_id: TypeId::of::<Matrix<T>>(),
      address: value.addr(),
    }
  }
}

struct SnapshotResult {
  value: Value,
  pure_reference_cycle: bool,
}

impl SnapshotResult {
  fn detached(value: Value) -> Self {
    Self {
      value,
      pure_reference_cycle: false,
    }
  }
}

struct ValueSnapshotContext {
  nodes: SnapshotMap<ValueSnapshotKey, Box<dyn Any>>,
  resolving_value_refs: SnapshotSet<ValueSnapshotKey>,
  cyclic_value_refs: SnapshotSet<ValueSnapshotKey>,
}

impl Default for ValueSnapshotContext {
  fn default() -> Self {
    Self {
      nodes: SnapshotMap::default(),
      resolving_value_refs: SnapshotSet::default(),
      cyclic_value_refs: SnapshotSet::default(),
    }
  }
}

impl ValueSnapshotContext {
  fn memoized<T>(
    &self,
    key: ValueSnapshotKey,
  ) -> Option<T>
  where
    T: Clone + 'static,
  {
    self.nodes.get(&key).map(|node| {
      node
        .downcast_ref::<T>()
        .expect(
          "value snapshot memo contained a mismatched node type",
        )
        .clone()
    })
  }

  fn memoize<T>(
    &mut self,
    key: ValueSnapshotKey,
    value: T,
  )
  where
    T: 'static,
  {
    self.nodes.insert(key, Box::new(value));
  }

  fn snapshot_leaf<T>(
    &mut self,
    source: &Ref<T>,
  ) -> Ref<T>
  where
    T: Clone + 'static,
  {
    let key = ValueSnapshotKey::of_ref(source);
    if let Some(snapshot) = self.memoized(key) {
      return snapshot;
    }
    let snapshot = Ref::new(source.borrow().clone());
    self.memoize(key, snapshot.clone());
    snapshot
  }

  fn snapshot_recursive_ref<T, P, B>(
    &mut self,
    source: &Ref<T>,
    placeholder: P,
    build: B,
  ) -> Ref<T>
  where
    T: Clone + 'static,
    P: FnOnce(&T) -> T,
    B: FnOnce(&mut Self, T) -> T,
  {
    let key = ValueSnapshotKey::of_ref(source);
    if let Some(snapshot) = self.memoized(key) {
      return snapshot;
    }

    let source_value = source.borrow().clone();
    let snapshot = Ref::new(placeholder(&source_value));
    self.memoize(key, snapshot.clone());
    let snapshot_value = build(self, source_value);
    *snapshot.borrow_mut() = snapshot_value;
    snapshot
  }

  #[cfg(feature = "atom")]
  fn snapshot_atom(
    &mut self,
    source: &Ref<MechAtom>,
  ) -> Ref<MechAtom> {
    let key = ValueSnapshotKey::of_ref(source);
    if let Some(snapshot) = self.memoized(key) {
      return snapshot;
    }

    let atom = source.borrow().clone();
    let dictionary: Ref<Dictionary> =
      self.snapshot_leaf(&atom.dictionary());
    let snapshot =
      Ref::new(MechAtom((atom.id(), dictionary)));
    self.memoize(key, snapshot.clone());
    snapshot
  }

  #[cfg(feature = "matrix")]
  fn snapshot_matrix<T>(
    &mut self,
    source: &Matrix<T>,
  ) -> Matrix<T>
  where
    T: Debug + Clone + PartialEq + 'static,
  {
    let key = ValueSnapshotKey::of_matrix(source);
    if let Some(snapshot) = self.memoized(key) {
      return snapshot;
    }

    let snapshot = Matrix::from_vec(
      source.as_vec(),
      source.rows(),
      source.cols(),
    );
    self.memoize(key, snapshot.clone());
    snapshot
  }

  #[cfg(feature = "matrix")]
  fn snapshot_value_matrix(
    &mut self,
    source: &Matrix<Value>,
  ) -> Matrix<Value> {
    let key = ValueSnapshotKey::of_matrix(source);
    if let Some(snapshot) = self.memoized(key) {
      return snapshot;
    }

    let snapshot = Matrix::from_element(
      source.rows(),
      source.cols(),
      Value::Empty,
    );
    self.memoize(key, snapshot.clone());
    let source_values = source.as_vec();
    for (index, value) in source_values.into_iter().enumerate() {
      snapshot.set_index1d(
        index,
        self.snapshot_value(&value).value,
      );
    }
    snapshot
  }

  fn snapshot_mutable_reference(
    &mut self,
    source: &ValRef,
  ) -> SnapshotResult {
    let key = ValueSnapshotKey::of_ref(source);
    if let Some(snapshot) = self.memoized::<ValRef>(key) {
      if self.resolving_value_refs.contains(&key)
        || self.cyclic_value_refs.contains(&key)
      {
        return SnapshotResult {
          value: Value::MutableReference(snapshot),
          pure_reference_cycle: true,
        };
      }
      return SnapshotResult::detached(
        snapshot.borrow().clone(),
      );
    }

    let snapshot = Ref::new(Value::Empty);
    self.memoize(key, snapshot.clone());
    self.resolving_value_refs.insert(key);
    let source_value = source.borrow().clone();
    let snapshot_value = self.snapshot_value(&source_value);
    let pure_reference_cycle =
      snapshot_value.pure_reference_cycle;
    *snapshot.borrow_mut() = snapshot_value.value;
    self.resolving_value_refs.remove(&key);

    if pure_reference_cycle {
      self.cyclic_value_refs.insert(key);
      SnapshotResult {
        value: Value::MutableReference(snapshot),
        pure_reference_cycle: true,
      }
    } else {
      SnapshotResult::detached(snapshot.borrow().clone())
    }
  }

  fn snapshot_value(
    &mut self,
    source: &Value,
  ) -> SnapshotResult {
    match source {
      #[cfg(feature = "u8")]
      Value::U8(value) => SnapshotResult::detached(
        Value::U8(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "u16")]
      Value::U16(value) => SnapshotResult::detached(
        Value::U16(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "u32")]
      Value::U32(value) => SnapshotResult::detached(
        Value::U32(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "u64")]
      Value::U64(value) => SnapshotResult::detached(
        Value::U64(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "u128")]
      Value::U128(value) => SnapshotResult::detached(
        Value::U128(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "i8")]
      Value::I8(value) => SnapshotResult::detached(
        Value::I8(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "i16")]
      Value::I16(value) => SnapshotResult::detached(
        Value::I16(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "i32")]
      Value::I32(value) => SnapshotResult::detached(
        Value::I32(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "i64")]
      Value::I64(value) => SnapshotResult::detached(
        Value::I64(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "i128")]
      Value::I128(value) => SnapshotResult::detached(
        Value::I128(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "f32")]
      Value::F32(value) => SnapshotResult::detached(
        Value::F32(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "f64")]
      Value::F64(value) => SnapshotResult::detached(
        Value::F64(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "complex")]
      Value::C64(value) => SnapshotResult::detached(
        Value::C64(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "rational")]
      Value::R64(value) => SnapshotResult::detached(
        Value::R64(self.snapshot_leaf(value)),
      ),
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(value) => SnapshotResult::detached(
        Value::String(self.snapshot_leaf(value)),
      ),
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(value) => SnapshotResult::detached(
        Value::Bool(self.snapshot_leaf(value)),
      ),
      #[cfg(feature = "atom")]
      Value::Atom(value) => SnapshotResult::detached(
        Value::Atom(self.snapshot_atom(value)),
      ),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(value) => SnapshotResult::detached(
        Value::MatrixIndex(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(value) => SnapshotResult::detached(
        Value::MatrixBool(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(value) => SnapshotResult::detached(
        Value::MatrixU8(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(value) => SnapshotResult::detached(
        Value::MatrixU16(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(value) => SnapshotResult::detached(
        Value::MatrixU32(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(value) => SnapshotResult::detached(
        Value::MatrixU64(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(value) => SnapshotResult::detached(
        Value::MatrixU128(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(value) => SnapshotResult::detached(
        Value::MatrixI8(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(value) => SnapshotResult::detached(
        Value::MatrixI16(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(value) => SnapshotResult::detached(
        Value::MatrixI32(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(value) => SnapshotResult::detached(
        Value::MatrixI64(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(value) => SnapshotResult::detached(
        Value::MatrixI128(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(value) => SnapshotResult::detached(
        Value::MatrixF32(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(value) => SnapshotResult::detached(
        Value::MatrixF64(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(value) => SnapshotResult::detached(
        Value::MatrixString(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(value) => SnapshotResult::detached(
        Value::MatrixR64(self.snapshot_matrix(value)),
      ),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(value) => SnapshotResult::detached(
        Value::MatrixC64(self.snapshot_matrix(value)),
      ),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(value) => SnapshotResult::detached(
        Value::MatrixValue(self.snapshot_value_matrix(value)),
      ),
      #[cfg(feature = "set")]
      Value::Set(value) => {
        let snapshot = self.snapshot_recursive_ref(
          value,
          |set| MechSet {
            kind: set.kind.clone(),
            num_elements: set.num_elements,
            set: IndexSet::new(),
          },
          |context, mut set| {
            set.set = set
              .set
              .into_iter()
              .map(|value| {
                context.snapshot_value(&value).value
              })
              .collect();
            set
          },
        );
        SnapshotResult::detached(Value::Set(snapshot))
      }
      #[cfg(feature = "map")]
      Value::Map(value) => {
        let snapshot = self.snapshot_recursive_ref(
          value,
          |map| MechMap {
            key_kind: map.key_kind.clone(),
            value_kind: map.value_kind.clone(),
            num_elements: map.num_elements,
            map: IndexMap::new(),
          },
          |context, mut map| {
            map.map = map
              .map
              .into_iter()
              .map(|(key, value)| {
                (
                  context.snapshot_value(&key).value,
                  context.snapshot_value(&value).value,
                )
              })
              .collect();
            map
          },
        );
        SnapshotResult::detached(Value::Map(snapshot))
      }
      #[cfg(feature = "record")]
      Value::Record(value) => {
        let snapshot = self.snapshot_recursive_ref(
          value,
          |record| MechRecord {
            cols: record.cols,
            kinds: record.kinds.clone(),
            data: IndexMap::new(),
            field_names: record.field_names.clone(),
          },
          |context, mut record| {
            record.data = record
              .data
              .into_iter()
              .map(|(id, value)| {
                (id, context.snapshot_value(&value).value)
              })
              .collect();
            record
          },
        );
        SnapshotResult::detached(Value::Record(snapshot))
      }
      #[cfg(feature = "table")]
      Value::Table(value) => {
        let snapshot = self.snapshot_recursive_ref(
          value,
          |table| MechTable {
            rows: table.rows,
            cols: table.cols,
            data: IndexMap::new(),
            col_names: table.col_names.clone(),
          },
          |context, mut table| {
            table.data = table
              .data
              .into_iter()
              .map(|(id, (kind, values))| {
                (
                  id,
                  (
                    kind,
                    context.snapshot_value_matrix(&values),
                  ),
                )
              })
              .collect();
            table
          },
        );
        SnapshotResult::detached(Value::Table(snapshot))
      }
      #[cfg(feature = "tuple")]
      Value::Tuple(value) => {
        let snapshot = self.snapshot_recursive_ref(
          value,
          |_| MechTuple::from_vec(Vec::new()),
          |context, tuple| {
            MechTuple::from_vec(
              tuple
                .elements
                .into_iter()
                .map(|value| {
                  context.snapshot_value(&value).value
                })
                .collect(),
            )
          },
        );
        SnapshotResult::detached(Value::Tuple(snapshot))
      }
      #[cfg(feature = "enum")]
      Value::Enum(value) => {
        let names = {
          let source_enum = value.borrow().clone();
          self.snapshot_leaf(&source_enum.names)
        };
        let placeholder_names = names.clone();
        let snapshot_names = names.clone();
        let snapshot = self.snapshot_recursive_ref(
          value,
          move |source_enum| MechEnum {
            id: source_enum.id,
            variants: Vec::new(),
            names: placeholder_names,
          },
          move |context, mut source_enum| {
            source_enum.names = snapshot_names;
            source_enum.variants = source_enum
              .variants
              .into_iter()
              .map(|(id, payload)| {
                (
                  id,
                  payload.map(|value| {
                    context.snapshot_value(&value).value
                  }),
                )
              })
              .collect();
            source_enum
          },
        );
        SnapshotResult::detached(Value::Enum(snapshot))
      }
      Value::Id(value) => {
        SnapshotResult::detached(Value::Id(*value))
      }
      Value::Index(value) => SnapshotResult::detached(
        Value::Index(self.snapshot_leaf(value)),
      ),
      Value::MutableReference(value) => {
        self.snapshot_mutable_reference(value)
      }
      Value::Typed(value, kind) => {
        let value = self.snapshot_value(value).value;
        SnapshotResult::detached(Value::Typed(
          Box::new(value),
          kind.clone(),
        ))
      }
      Value::Kind(kind) => {
        SnapshotResult::detached(Value::Kind(kind.clone()))
      }
      Value::IndexAll => {
        SnapshotResult::detached(Value::IndexAll)
      }
      Value::EmptyKind(kind) => {
        SnapshotResult::detached(Value::EmptyKind(kind.clone()))
      }
      Value::Empty => {
        SnapshotResult::detached(Value::Empty)
      }
    }
  }
}

pub(crate) fn deep_snapshot(source: &Value) -> Value {
  ValueSnapshotContext::default()
    .snapshot_value(source)
    .value
}
