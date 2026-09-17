use super::{CardinalitySpec, Schema, SchemaBody, SchemaField};
use crate::snapshot::SnapshotCanonicalizationBudget;
use crate::{
    DimensionExpr, DimensionParameter, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, SchemaId, SchemaKey, SemanticIdentityKind, SemanticModelError,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, vec, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SchemaHandle {
    ordinal: u32,
    key: SchemaKey,
}

#[derive(Clone, Debug, Default)]
pub struct SchemaTableBuilder {
    schemas: Vec<Schema>,
}

#[derive(Clone, Debug)]
pub struct SchemaTableBuild {
    pub table: SchemaTable,
    remap: Box<[SchemaId]>,
    handle_keys: Box<[SchemaKey]>,
}

#[derive(Clone, Debug)]
pub struct SchemaTable {
    entries: Box<[SchemaEntry]>,
}

#[derive(Clone, Debug)]
pub struct SchemaEntry {
    schema: Schema,
    key: SchemaKey,
    canonical_bytes: Box<[u8]>,
}

impl SchemaTableBuilder {
    pub const fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    pub fn insert(&mut self, schema: Schema) -> Result<SchemaHandle, SemanticModelError> {
        self.insert_with_limit(schema, usize::MAX)
    }

    fn insert_with_limit(
        &mut self,
        schema: Schema,
        handle_limit: usize,
    ) -> Result<SchemaHandle, SemanticModelError> {
        if self.schemas.len() >= handle_limit {
            return Err(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::SchemaHandle,
            });
        }
        let ordinal = u32::try_from(self.schemas.len()).map_err(|_| {
            SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::SchemaHandle,
            }
        })?;
        let handle = SchemaHandle {
            ordinal,
            key: schema.key(),
        };
        self.schemas.push(schema);
        Ok(handle)
    }

    pub fn finish(self) -> Result<SchemaTableBuild, SemanticModelError> {
        self.finish_with(u32::MAX as usize, Schema::key)
    }

    fn finish_with(
        self,
        unique_limit: usize,
        key_for: impl Fn(&Schema) -> SchemaKey,
    ) -> Result<SchemaTableBuild, SemanticModelError> {
        let handle_keys = self
            .schemas
            .iter()
            .map(Schema::key)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut pending = self
            .schemas
            .into_iter()
            .enumerate()
            .map(|(handle, schema)| (schema.canonical_bytes(), handle, schema))
            .collect::<Vec<_>>();
        pending.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

        let mut remap = vec![SchemaId::new(0); pending.len()];
        let mut entries: Vec<SchemaEntry> = Vec::new();
        let mut keys: BTreeMap<SchemaKey, Box<[u8]>> = BTreeMap::new();
        for (canonical_bytes, handle, schema) in pending {
            if entries
                .last()
                .is_some_and(|entry| entry.canonical_bytes == canonical_bytes)
            {
                remap[handle] = SchemaId::new((entries.len() - 1) as u32);
                continue;
            }
            if entries.len() >= unique_limit {
                return Err(SemanticModelError::SchemaIdExhausted);
            }
            let id = u32::try_from(entries.len())
                .map(SchemaId::new)
                .map_err(|_| SemanticModelError::SchemaIdExhausted)?;
            let key = key_for(&schema);
            if let Some(existing) = keys.get(&key) {
                if existing.as_ref() != canonical_bytes.as_ref() {
                    return Err(SemanticModelError::SchemaKeyCollision { key });
                }
            }
            keys.insert(key, canonical_bytes.clone());
            remap[handle] = id;
            entries.push(SchemaEntry {
                schema,
                key,
                canonical_bytes,
            });
        }
        Ok(SchemaTableBuild {
            table: SchemaTable {
                entries: entries.into_boxed_slice(),
            },
            remap: remap.into_boxed_slice(),
            handle_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CanonicalNominalPath, IntegerWidth, NominalKey, NominalKind, SchemaBody, SchemaDraft,
    };

    fn schema(body: SchemaBody) -> Schema {
        SchemaDraft {
            dimension_parameters: Vec::new().into_boxed_slice(),
            body,
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn test_only_limit_proves_schema_id_exhaustion() {
        let mut builder = SchemaTableBuilder::new();
        builder.insert(schema(SchemaBody::Bool)).unwrap();
        builder
            .insert(schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)))
            .unwrap();
        assert!(matches!(
            builder.finish_with(1, Schema::key),
            Err(SemanticModelError::SchemaIdExhausted)
        ));
    }

    #[test]
    fn test_only_limit_proves_schema_handle_identity_exhaustion() {
        let mut builder = SchemaTableBuilder::new();
        builder
            .insert_with_limit(schema(SchemaBody::Bool), 1)
            .unwrap();
        assert!(matches!(
            builder.insert_with_limit(schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)), 1,),
            Err(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::SchemaHandle,
            })
        ));
        assert_eq!(builder.schemas.len(), 1);
    }

    #[test]
    fn test_only_hash_hook_proves_collision_rejection() {
        let mut builder = SchemaTableBuilder::new();
        builder.insert(schema(SchemaBody::Bool)).unwrap();
        builder
            .insert(schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)))
            .unwrap();
        let forced = SchemaKey::from_bytes([7; 32]);
        assert!(matches!(
            builder.finish_with(u32::MAX as usize, |_| forced),
            Err(SemanticModelError::SchemaKeyCollision { key }) if key == forced
        ));
    }

    #[test]
    fn resolve_rejects_out_of_range_and_foreign_handles() {
        let bool_schema = schema(SchemaBody::Bool);
        let string_schema = schema(SchemaBody::String);

        let mut first = SchemaTableBuilder::new();
        let first_bool = first.insert(bool_schema.clone()).unwrap();
        let first = first.finish().unwrap();

        let mut second = SchemaTableBuilder::new();
        let second_string = second.insert(string_schema).unwrap();
        let second = second.finish().unwrap();

        assert!(first.resolve(first_bool).is_ok());
        assert!(matches!(
            first.resolve(second_string),
            Err(SemanticModelError::InvalidSchemaHandleV1)
        ));
        assert!(matches!(
            second.resolve(SchemaHandle {
                ordinal: u32::MAX,
                key: bool_schema.key(),
            }),
            Err(SemanticModelError::InvalidSchemaHandleV1)
        ));
    }

    #[test]
    fn component_closure_preserves_ids_and_bounds_its_runtime_construction() {
        let tuple_schema = schema(SchemaBody::Tuple(
            vec![SchemaBody::String, SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
        ));
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder.insert(tuple_schema.clone()).unwrap();
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let retained_bound = build
            .table
            .component_closure_allocation_bound_bytes()
            .unwrap();
        let construction_bound = build
            .table
            .component_closure_construction_bound_bytes()
            .unwrap();
        let entry_bound = build.table.component_closure_entry_count_bound().unwrap();
        let limited = SnapshotCanonicalizationBudget::new(1);
        assert!(
            build
                .table
                .component_closure_bounds_with_budget(&limited)
                .is_none(),
            "closure preflight stops while recursive work is still bounded",
        );
        let metered = SnapshotCanonicalizationBudget::new(u64::MAX);
        assert_eq!(
            build.table.component_closure_bounds_with_budget(&metered),
            Some((retained_bound, construction_bound, entry_bound)),
        );
        assert!(metered.consumed() > 0);
        let closed = build
            .table
            .extend_with_component_closure_preserving_ids()
            .unwrap();

        assert_eq!(closed.get(tuple), Some(&tuple_schema));
        assert!(
            closed
                .entries()
                .any(|entry| entry.schema().body() == &SchemaBody::String)
        );
        assert!(
            closed
                .entries()
                .any(|entry| entry.schema().body() == &SchemaBody::Bool)
        );
        assert_eq!(closed.len(), 3, "duplicate String children share one entry");
        assert!(closed.clone_allocation_bound_bytes().unwrap() <= retained_bound);
        assert!(retained_bound <= construction_bound);
        assert!(closed.len() as u64 <= entry_bound);
    }

    #[test]
    fn rooted_component_closure_excludes_unreachable_arena_entries() {
        let tuple_schema = schema(SchemaBody::Tuple(
            vec![SchemaBody::String, SchemaBody::Bool].into_boxed_slice(),
        ));
        let unrelated_schema = schema(SchemaBody::Atom(NominalKey::from_path(
            NominalKind::Atom,
            &CanonicalNominalPath::new(vec!["unrelated".to_owned()]).unwrap(),
        )));
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder.insert(tuple_schema.clone()).unwrap();
        builder.insert(unrelated_schema.clone()).unwrap();
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let budget = SnapshotCanonicalizationBudget::new(u64::MAX);
        let (retained, construction, nodes) = build
            .table
            .component_closure_bounds_for_roots_with_budget(&[tuple], &budget)
            .unwrap();
        let closed = build.table.component_closure_for_roots(&[tuple]).unwrap();

        assert_eq!(closed.len(), 3);
        assert!(
            closed
                .entries()
                .any(|entry| entry.schema() == &tuple_schema)
        );
        assert!(
            !closed
                .entries()
                .any(|entry| entry.schema() == &unrelated_schema)
        );
        assert!(closed.clone_allocation_bound_bytes().unwrap() <= retained);
        assert!(retained <= construction);
        assert!(closed.len() as u64 <= nodes);
    }

    #[test]
    fn clone_allocation_witness_counts_wide_tuple_nodes_not_encoding_multipliers() {
        const CHILDREN: usize = 2_048;
        let mut builder = SchemaTableBuilder::new();
        builder
            .insert(schema(SchemaBody::Tuple(
                vec![SchemaBody::Bool; CHILDREN].into_boxed_slice(),
            )))
            .unwrap();
        let table = builder.finish().unwrap().table;
        let entry = &table.entries[0];
        let expected = (core::mem::size_of::<SchemaTable>()
            + core::mem::size_of::<SchemaEntry>()
            + CHILDREN * core::mem::size_of::<SchemaBody>()
            + entry.canonical_bytes.len()) as u64;
        assert_eq!(table.clone_allocation_bound_bytes(), Some(expected));
        let encoding_multiplier = (core::mem::size_of::<SchemaTable>()
            + core::mem::size_of::<SchemaEntry>()
            + entry.canonical_bytes.len() * 4) as u64;
        assert!(
            expected > encoding_multiplier,
            "compact Bool tags do not bound the cloned SchemaBody slice"
        );
        assert_eq!(table.clone().clone_allocation_bound_bytes(), Some(expected));
    }

    #[test]
    fn clone_allocation_witness_counts_names_and_boxed_aggregate_children() {
        let record = SchemaBody::Record(
            vec![
                SchemaField {
                    name: "first".into(),
                    schema: SchemaBody::Option(Box::new(SchemaBody::Bool)),
                },
                SchemaField {
                    name: "second".into(),
                    schema: SchemaBody::Tuple(
                        vec![SchemaBody::String, SchemaBody::Bool].into_boxed_slice(),
                    ),
                },
            ]
            .into_boxed_slice(),
        );
        let record_bytes = (2 * core::mem::size_of::<SchemaField>()
            + "first".len()
            + "second".len()
            + 3 * core::mem::size_of::<SchemaBody>()) as u64;
        assert_eq!(body_clone_heap_bytes(&record), Some(record_bytes));
        let variants = SchemaBody::Enum {
            key: crate::NominalKey::from_bytes([7; 32]),
            variants: vec![
                super::super::EnumVariantSchema {
                    name: "Empty".into(),
                    payload: None,
                },
                super::super::EnumVariantSchema {
                    name: "Record".into(),
                    payload: Some(record.clone()),
                },
            ]
            .into_boxed_slice(),
        };
        assert_eq!(
            body_clone_heap_bytes(&variants),
            Some(
                record_bytes
                    + (2 * core::mem::size_of::<super::super::EnumVariantSchema>()
                        + "Empty".len()
                        + "Record".len()) as u64
            )
        );

        let dimension = DimensionExpr::Add(
            vec![
                DimensionExpr::Constant(2),
                DimensionExpr::Parameter(crate::DimensionParameterId::new(0)),
            ]
            .into_boxed_slice(),
        );
        let extent_bytes = (2 * core::mem::size_of::<DimensionExpr>()) as u64;
        let table = SchemaBody::Table {
            columns: vec![SchemaField {
                name: "column".into(),
                schema: SchemaBody::Bool,
            }]
            .into_boxed_slice(),
            rows: CardinalitySpec::Dynamic {
                upper_bound: Some(dimension.clone()),
            },
        };
        assert_eq!(
            body_clone_heap_bytes(&table),
            Some((core::mem::size_of::<SchemaField>() + "column".len()) as u64 + extent_bytes)
        );
        let set = SchemaBody::Set {
            element: Box::new(SchemaBody::Bool),
            cardinality: CardinalitySpec::Exact(dimension.clone()),
        };
        assert_eq!(
            body_clone_heap_bytes(&set),
            Some(core::mem::size_of::<SchemaBody>() as u64 + extent_bytes)
        );
        let map = SchemaBody::Map {
            key: Box::new(SchemaBody::Bool),
            value: Box::new(record),
            cardinality: CardinalitySpec::Exact(dimension.clone()),
        };
        assert_eq!(
            body_clone_heap_bytes(&map),
            Some(2 * core::mem::size_of::<SchemaBody>() as u64 + record_bytes + extent_bytes)
        );
        let matrix = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![dimension, DimensionExpr::Constant(3)].into_boxed_slice(),
        };
        assert_eq!(
            body_clone_heap_bytes(&matrix),
            Some(core::mem::size_of::<SchemaBody>() as u64 + 2 * extent_bytes)
        );
    }

    #[test]
    fn clone_allocation_witness_counts_recursive_dimension_storage_and_parameters() {
        use crate::{
            DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin,
        };
        let parameter = DimensionExpr::Parameter(DimensionParameterId::new(0));
        let expression = DimensionExpr::Max(
            vec![
                DimensionExpr::Min(
                    vec![DimensionExpr::Constant(1), parameter.clone()].into_boxed_slice(),
                ),
                DimensionExpr::Multiply(
                    vec![
                        parameter.clone(),
                        DimensionExpr::Add(
                            vec![parameter.clone(), DimensionExpr::Constant(2)].into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                ),
            ]
            .into_boxed_slice(),
        );
        assert_eq!(
            dimension_clone_heap_bytes(&expression),
            Some((8 * core::mem::size_of::<DimensionExpr>()) as u64)
        );
        let value = SchemaDraft {
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(1),
                upper_bound: Some(DimensionExpr::Constant(10)),
            }]
            .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![parameter, DimensionExpr::Constant(2)].into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        assert_eq!(
            schema_clone_heap_bytes(&value),
            Some(
                (core::mem::size_of::<DimensionParameter>()
                    + core::mem::size_of::<SchemaBody>()
                    + 2 * core::mem::size_of::<DimensionExpr>()) as u64
            )
        );
        let overflowing_count = u64::MAX / core::mem::size_of::<SchemaBody>() as u64 + 1;
        if let Ok(count) = usize::try_from(overflowing_count) {
            assert_eq!(clone_slice_bytes::<SchemaBody>(count), None);
        }
    }
}

impl SchemaTableBuild {
    pub fn resolve(&self, handle: SchemaHandle) -> Result<SchemaId, SemanticModelError> {
        let ordinal = handle.ordinal as usize;
        if self.handle_keys.get(ordinal) != Some(&handle.key) {
            return Err(SemanticModelError::InvalidSchemaHandleV1);
        }
        self.remap
            .get(ordinal)
            .copied()
            .ok_or(SemanticModelError::InvalidSchemaHandleV1)
    }

    pub fn into_parts(self) -> (SchemaTable, Box<[SchemaId]>) {
        (self.table, self.remap)
    }
}

impl SchemaTable {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: SchemaId) -> Option<&Schema> {
        self.entry(id).map(SchemaEntry::schema)
    }

    pub fn entry(&self, id: SchemaId) -> Option<&SchemaEntry> {
        self.entries.get(id.get() as usize)
    }

    pub fn find_by_key(&self, key: SchemaKey) -> Option<SchemaId> {
        self.entries
            .iter()
            .position(|entry| entry.key == key)
            .map(|index| SchemaId::new(index as u32))
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = &SchemaEntry> {
        self.entries.iter()
    }

    /// Extend this arena with schemas owned by a detached value while keeping
    /// every existing schema ID stable.
    ///
    /// Canonical source linking uses this before rebinding a dependency export
    /// into a dynamic input. Existing program and constant IDs remain valid;
    /// equivalent definitions are deduplicated by their canonical key.
    pub fn extend_preserving_ids(
        &self,
        additional: &SchemaTable,
    ) -> Result<Self, SemanticModelError> {
        let mut entries = self.entries.to_vec();
        let mut by_key = entries
            .iter()
            .map(|entry| (entry.key, entry.canonical_bytes.clone()))
            .collect::<BTreeMap<_, _>>();
        for entry in additional.entries.iter() {
            if let Some(existing) = by_key.get(&entry.key) {
                if existing.as_ref() != entry.canonical_bytes.as_ref() {
                    return Err(SemanticModelError::SchemaKeyCollision { key: entry.key });
                }
                continue;
            }
            u32::try_from(entries.len()).map_err(|_| SemanticModelError::SchemaIdExhausted)?;
            by_key.insert(entry.key, entry.canonical_bytes.clone());
            entries.push(entry.clone());
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
        })
    }

    /// Extends this arena with every canonical component schema reachable from
    /// its retained schemas while preserving all existing IDs. Detached
    /// Dynamic values can omit standalone component entries even though a
    /// structural projection needs one to publish the selected child.
    #[doc(hidden)]
    pub fn extend_with_component_closure_preserving_ids(&self) -> Result<Self, SemanticModelError> {
        let mut entries = self.entries.to_vec();
        for entry in self.entries.iter() {
            retain_component_children(&entry.schema, entry.schema.body(), &mut entries)?;
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
        })
    }

    /// Builds the canonical component closure reachable only from the given
    /// root schemas. The returned table is intended for key-based merging;
    /// its local IDs need not match the source arena.
    #[doc(hidden)]
    pub fn component_closure_for_roots(
        &self,
        roots: &[SchemaId],
    ) -> Result<Self, SemanticModelError> {
        let mut entries = Vec::new();
        for root in roots {
            let entry = self
                .entry(*root)
                .ok_or(SemanticModelError::InvalidSchemaHandleV1)?;
            if entries
                .iter()
                .any(|existing: &SchemaEntry| existing.key == entry.key)
            {
                continue;
            }
            entries.push(entry.clone());
        }
        let root_count = entries.len();
        for index in 0..root_count {
            let schema = entries[index].schema.clone();
            retain_component_children(&schema, schema.body(), &mut entries)?;
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
        })
    }

    /// Retained allocation upper bound for the component-closed arena.
    #[doc(hidden)]
    pub fn component_closure_allocation_bound_bytes(&self) -> Option<u64> {
        component_closure_cost(self).map(|cost| cost.retained_bytes)
    }

    /// Construction peak upper bound for the component-closed arena.
    #[doc(hidden)]
    pub fn component_closure_construction_bound_bytes(&self) -> Option<u64> {
        component_closure_cost(self).map(|cost| cost.construction_bytes)
    }

    /// Schema-entry population upper bound for the component-closed arena.
    #[doc(hidden)]
    pub fn component_closure_entry_count_bound(&self) -> Option<u64> {
        component_closure_cost(self).map(|cost| cost.entry_count)
    }

    /// Computes every component-closure bound in one traversal while
    /// enforcing the caller's recursive-work allowance as the traversal is
    /// performed. Resident consumers use this before constructing a foreign
    /// closure so a wide or deeply nested schema cannot consume unmetered
    /// work merely to discover that it exceeds the turn budget.
    #[doc(hidden)]
    pub fn component_closure_bounds_with_budget(
        &self,
        budget: &SnapshotCanonicalizationBudget,
    ) -> Option<(u64, u64, u64)> {
        component_closure_cost_with_budget(self, Some(budget)).map(|cost| {
            (
                cost.retained_bytes,
                cost.construction_bytes,
                cost.entry_count,
            )
        })
    }

    /// Bounds a component closure restricted to the supplied root schema IDs.
    #[doc(hidden)]
    pub fn component_closure_bounds_for_roots_with_budget(
        &self,
        roots: &[SchemaId],
        budget: &SnapshotCanonicalizationBudget,
    ) -> Option<(u64, u64, u64)> {
        component_closure_cost_for_roots_with_budget(self, Some(roots), Some(budget)).map(|cost| {
            (
                cost.retained_bytes,
                cost.construction_bytes,
                cost.entry_count,
            )
        })
    }

    /// Checked allocation witness for an independently owned clone of this
    /// canonical schema context. It includes the table header, entry slice,
    /// each concrete cloned schema allocation and the retained canonical byte
    /// buffer. Compact encodings are not a bound on Rust enum/node layouts.
    pub fn clone_allocation_bound_bytes(&self) -> Option<u64> {
        let initial =
            (core::mem::size_of::<Self>() as u64)
                .checked_add(clone_slice_bytes::<SchemaEntry>(self.entries.len())?)?;
        self.entries.iter().try_fold(initial, |total, entry| {
            total
                .checked_add(u64::try_from(entry.canonical_bytes.len()).ok()?)?
                .checked_add(schema_clone_heap_bytes(&entry.schema)?)
        })
    }
}

fn retain_component(
    parent: &Schema,
    child: &SchemaBody,
    entries: &mut Vec<SchemaEntry>,
) -> Result<(), SemanticModelError> {
    let schema = parent.canonical_component_schema(child)?;
    let key = schema.key();
    let canonical_bytes = schema.canonical_bytes();
    if let Some(existing) = entries.iter().find(|entry| entry.key == key) {
        if existing.canonical_bytes.as_ref() != canonical_bytes.as_ref() {
            return Err(SemanticModelError::SchemaKeyCollision { key });
        }
    } else {
        u32::try_from(entries.len()).map_err(|_| SemanticModelError::SchemaIdExhausted)?;
        entries.push(SchemaEntry {
            schema,
            key,
            canonical_bytes,
        });
    }
    retain_component_children(parent, child, entries)
}

fn retain_component_children(
    parent: &Schema,
    body: &SchemaBody,
    entries: &mut Vec<SchemaEntry>,
) -> Result<(), SemanticModelError> {
    match body {
        SchemaBody::Enum { variants, .. } => {
            for child in variants
                .iter()
                .filter_map(|variant| variant.payload.as_ref())
            {
                retain_component(parent, child, entries)?;
            }
        }
        SchemaBody::Option(child)
        | SchemaBody::Matrix { element: child, .. }
        | SchemaBody::Set { element: child, .. } => retain_component(parent, child, entries)?,
        SchemaBody::Tuple(children) => {
            for child in children {
                retain_component(parent, child, entries)?;
            }
        }
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => {
            for field in fields {
                retain_component(parent, &field.schema, entries)?;
            }
        }
        SchemaBody::Map { key, value, .. } => {
            retain_component(parent, key, entries)?;
            retain_component(parent, value, entries)?;
        }
        SchemaBody::Dynamic
        | SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => {}
    }
    Ok(())
}

impl Schema {
    /// Canonical standalone schema for one component body under this schema's
    /// dimension environment.
    #[doc(hidden)]
    pub fn canonical_component_schema(
        &self,
        body: &SchemaBody,
    ) -> Result<Self, SemanticModelError> {
        super::SchemaDraft {
            body: body.clone(),
            dimension_parameters: self
                .dimension_parameters()
                .iter()
                .enumerate()
                .map(|(id, parameter)| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(id as u32),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: parameter.lifetime(),
                    lower_bound: parameter.lower_bound().clone(),
                    upper_bound: parameter.upper_bound().cloned(),
                })
                .collect(),
        }
        .finalize()
    }
}

#[derive(Clone, Copy)]
struct ComponentClosureCost {
    retained_bytes: u64,
    construction_bytes: u64,
    entry_count: u64,
}

fn component_closure_cost(table: &SchemaTable) -> Option<ComponentClosureCost> {
    component_closure_cost_with_budget(table, None)
}

#[derive(Clone, Copy)]
struct CloneEncodingCost {
    clone_bytes: u64,
    encoding_bytes: u64,
}

fn component_closure_cost_with_budget(
    table: &SchemaTable,
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Option<ComponentClosureCost> {
    component_closure_cost_for_roots_with_budget(table, None, budget)
}

fn component_closure_cost_for_roots_with_budget(
    table: &SchemaTable,
    roots: Option<&[SchemaId]>,
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Option<ComponentClosureCost> {
    fn charge(budget: Option<&SnapshotCanonicalizationBudget>) -> Option<()> {
        match budget {
            Some(budget) => budget.charge(1).ok(),
            None => Some(()),
        }
    }

    fn dimension_cost(
        dimension: &DimensionExpr,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Option<CloneEncodingCost> {
        charge(budget)?;
        match dimension {
            DimensionExpr::Hole => None,
            DimensionExpr::Constant(_) => Some(CloneEncodingCost {
                clone_bytes: 0,
                encoding_bytes: 9,
            }),
            DimensionExpr::Parameter(_) => Some(CloneEncodingCost {
                clone_bytes: 0,
                encoding_bytes: 5,
            }),
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => {
                let mut clone_bytes = clone_slice_bytes::<DimensionExpr>(children.len())?;
                let mut encoding_bytes = 5_u64;
                for child in children {
                    let child = dimension_cost(child, budget)?;
                    clone_bytes = clone_bytes.checked_add(child.clone_bytes)?;
                    encoding_bytes = encoding_bytes
                        .checked_add(8)?
                        .checked_add(child.encoding_bytes)?;
                }
                Some(CloneEncodingCost {
                    clone_bytes,
                    encoding_bytes,
                })
            }
        }
    }

    fn parameters_cost(
        parameters: &[DimensionParameter],
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Option<CloneEncodingCost> {
        let mut clone_bytes = clone_slice_bytes::<DimensionParameter>(parameters.len())?;
        let mut encoding_bytes = 5_u64;
        for parameter in parameters {
            charge(budget)?;
            let lower = dimension_cost(parameter.lower_bound(), budget)?;
            clone_bytes = clone_bytes.checked_add(lower.clone_bytes)?;
            encoding_bytes = encoding_bytes
                .checked_add(1 + 8)?
                .checked_add(lower.encoding_bytes)?;
            match parameter.upper_bound() {
                Some(upper) => {
                    let upper = dimension_cost(upper, budget)?;
                    clone_bytes = clone_bytes.checked_add(upper.clone_bytes)?;
                    encoding_bytes = encoding_bytes
                        .checked_add(1 + 8)?
                        .checked_add(upper.encoding_bytes)?;
                }
                None => encoding_bytes = encoding_bytes.checked_add(1)?,
            }
        }
        Some(CloneEncodingCost {
            clone_bytes,
            encoding_bytes,
        })
    }

    fn extent_cost(
        extent: &CardinalitySpec,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Option<CloneEncodingCost> {
        match extent {
            CardinalitySpec::Exact(value) => dimension_cost(value, budget),
            CardinalitySpec::Dynamic { upper_bound: None } => {
                charge(budget)?;
                Some(CloneEncodingCost {
                    clone_bytes: 0,
                    encoding_bytes: 1,
                })
            }
            CardinalitySpec::Dynamic {
                upper_bound: Some(value),
            } => {
                charge(budget)?;
                let value = dimension_cost(value, budget)?;
                Some(CloneEncodingCost {
                    clone_bytes: value.clone_bytes,
                    encoding_bytes: value.encoding_bytes.checked_add(1 + 8)?,
                })
            }
        }
    }

    fn body_cost(
        body: &SchemaBody,
        parameters: CloneEncodingCost,
        retained: &mut u64,
        maximum_component: &mut u64,
        count: &mut u64,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Option<CloneEncodingCost> {
        fn child(
            body: &SchemaBody,
            parameters: CloneEncodingCost,
            retained: &mut u64,
            maximum_component: &mut u64,
            count: &mut u64,
            budget: Option<&SnapshotCanonicalizationBudget>,
        ) -> Option<CloneEncodingCost> {
            let cost = body_cost(body, parameters, retained, maximum_component, count, budget)?;
            let component = (core::mem::size_of::<SchemaEntry>() as u64)
                .checked_add(parameters.clone_bytes)?
                .checked_add(cost.clone_bytes)?
                .checked_add(parameters.encoding_bytes)?
                .checked_add(8)?
                .checked_add(cost.encoding_bytes)?;
            *retained = retained.checked_add(component)?;
            *maximum_component = (*maximum_component).max(component);
            *count = count.checked_add(1)?;
            Some(cost)
        }

        charge(budget)?;
        match body {
            SchemaBody::Dynamic
            | SchemaBody::Bool
            | SchemaBody::String
            | SchemaBody::Id
            | SchemaBody::Index
            | SchemaBody::ReifiedType => Some(CloneEncodingCost {
                clone_bytes: 0,
                encoding_bytes: 1,
            }),
            SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_) => Some(CloneEncodingCost {
                clone_bytes: 0,
                encoding_bytes: 3,
            }),
            SchemaBody::Complex(_) | SchemaBody::Rational64 => Some(CloneEncodingCost {
                clone_bytes: 0,
                encoding_bytes: 5,
            }),
            SchemaBody::Atom(_) => Some(CloneEncodingCost {
                clone_bytes: 0,
                encoding_bytes: 33,
            }),
            SchemaBody::Enum { variants, .. } => {
                let mut clone_bytes =
                    clone_slice_bytes::<super::EnumVariantSchema>(variants.len())?;
                let mut encoding_bytes = 37_u64;
                for variant in variants {
                    clone_bytes =
                        clone_bytes.checked_add(u64::try_from(variant.name.len()).ok()?)?;
                    encoding_bytes = encoding_bytes
                        .checked_add(8 + u64::try_from(variant.name.len()).ok()?)?
                        .checked_add(1)?;
                    if let Some(payload) = &variant.payload {
                        let payload = child(
                            payload,
                            parameters,
                            retained,
                            maximum_component,
                            count,
                            budget,
                        )?;
                        clone_bytes = clone_bytes.checked_add(payload.clone_bytes)?;
                        encoding_bytes = encoding_bytes
                            .checked_add(8)?
                            .checked_add(payload.encoding_bytes)?;
                    }
                }
                Some(CloneEncodingCost {
                    clone_bytes,
                    encoding_bytes,
                })
            }
            SchemaBody::Option(element) => {
                let element = child(
                    element,
                    parameters,
                    retained,
                    maximum_component,
                    count,
                    budget,
                )?;
                Some(CloneEncodingCost {
                    clone_bytes: (core::mem::size_of::<SchemaBody>() as u64)
                        .checked_add(element.clone_bytes)?,
                    encoding_bytes: element.encoding_bytes.checked_add(1 + 8)?,
                })
            }
            SchemaBody::Tuple(elements) => {
                let mut clone_bytes = clone_slice_bytes::<SchemaBody>(elements.len())?;
                let mut encoding_bytes = 5_u64;
                for element in elements {
                    let element = child(
                        element,
                        parameters,
                        retained,
                        maximum_component,
                        count,
                        budget,
                    )?;
                    clone_bytes = clone_bytes.checked_add(element.clone_bytes)?;
                    encoding_bytes = encoding_bytes
                        .checked_add(8)?
                        .checked_add(element.encoding_bytes)?;
                }
                Some(CloneEncodingCost {
                    clone_bytes,
                    encoding_bytes,
                })
            }
            SchemaBody::Record(fields) => fields_cost(
                fields,
                1,
                parameters,
                retained,
                maximum_component,
                count,
                budget,
            ),
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                let element = child(
                    element,
                    parameters,
                    retained,
                    maximum_component,
                    count,
                    budget,
                )?;
                let mut clone_bytes = (core::mem::size_of::<SchemaBody>() as u64)
                    .checked_add(element.clone_bytes)?
                    .checked_add(clone_slice_bytes::<DimensionExpr>(dimensions.len())?)?;
                let mut encoding_bytes = 1_u64
                    .checked_add(8)?
                    .checked_add(element.encoding_bytes)?
                    .checked_add(4)?;
                for dimension in dimensions {
                    let dimension = dimension_cost(dimension, budget)?;
                    clone_bytes = clone_bytes.checked_add(dimension.clone_bytes)?;
                    encoding_bytes = encoding_bytes
                        .checked_add(8)?
                        .checked_add(dimension.encoding_bytes)?;
                }
                Some(CloneEncodingCost {
                    clone_bytes,
                    encoding_bytes,
                })
            }
            SchemaBody::Table { columns, rows } => {
                let fields = fields_cost(
                    columns,
                    1,
                    parameters,
                    retained,
                    maximum_component,
                    count,
                    budget,
                )?;
                let rows = extent_cost(rows, budget)?;
                Some(CloneEncodingCost {
                    clone_bytes: fields.clone_bytes.checked_add(rows.clone_bytes)?,
                    encoding_bytes: fields
                        .encoding_bytes
                        .checked_add(8)?
                        .checked_add(rows.encoding_bytes)?,
                })
            }
            SchemaBody::Set {
                element,
                cardinality,
            } => {
                let element = child(
                    element,
                    parameters,
                    retained,
                    maximum_component,
                    count,
                    budget,
                )?;
                let cardinality = extent_cost(cardinality, budget)?;
                Some(CloneEncodingCost {
                    clone_bytes: (core::mem::size_of::<SchemaBody>() as u64)
                        .checked_add(element.clone_bytes)?
                        .checked_add(cardinality.clone_bytes)?,
                    encoding_bytes: 1_u64
                        .checked_add(8)?
                        .checked_add(element.encoding_bytes)?
                        .checked_add(8)?
                        .checked_add(cardinality.encoding_bytes)?,
                })
            }
            SchemaBody::Map {
                key,
                value,
                cardinality,
            } => {
                let key = child(key, parameters, retained, maximum_component, count, budget)?;
                let value = child(
                    value,
                    parameters,
                    retained,
                    maximum_component,
                    count,
                    budget,
                )?;
                let cardinality = extent_cost(cardinality, budget)?;
                Some(CloneEncodingCost {
                    clone_bytes: (2 * core::mem::size_of::<SchemaBody>() as u64)
                        .checked_add(key.clone_bytes)?
                        .checked_add(value.clone_bytes)?
                        .checked_add(cardinality.clone_bytes)?,
                    encoding_bytes: 1_u64
                        .checked_add(8)?
                        .checked_add(key.encoding_bytes)?
                        .checked_add(8)?
                        .checked_add(value.encoding_bytes)?
                        .checked_add(8)?
                        .checked_add(cardinality.encoding_bytes)?,
                })
            }
        }
    }

    fn fields_cost(
        fields: &[SchemaField],
        encoding_prefix: u64,
        parameters: CloneEncodingCost,
        retained: &mut u64,
        maximum_component: &mut u64,
        count: &mut u64,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Option<CloneEncodingCost> {
        let mut clone_bytes = clone_slice_bytes::<SchemaField>(fields.len())?;
        let mut encoding_bytes = encoding_prefix.checked_add(4)?;
        for field in fields {
            charge(budget)?;
            let name = u64::try_from(field.name.len()).ok()?;
            let body = body_cost(
                &field.schema,
                parameters,
                retained,
                maximum_component,
                count,
                budget,
            )?;
            let component = (core::mem::size_of::<SchemaEntry>() as u64)
                .checked_add(parameters.clone_bytes)?
                .checked_add(body.clone_bytes)?
                .checked_add(parameters.encoding_bytes)?
                .checked_add(8)?
                .checked_add(body.encoding_bytes)?;
            *retained = retained.checked_add(component)?;
            *maximum_component = (*maximum_component).max(component);
            *count = count.checked_add(1)?;
            clone_bytes = clone_bytes
                .checked_add(name)?
                .checked_add(body.clone_bytes)?;
            encoding_bytes = encoding_bytes
                .checked_add(8 + name)?
                .checked_add(8)?
                .checked_add(body.encoding_bytes)?;
        }
        Some(CloneEncodingCost {
            clone_bytes,
            encoding_bytes,
        })
    }

    let root_count = roots.map_or(table.entries.len(), <[SchemaId]>::len);
    let mut retained = (core::mem::size_of::<SchemaTable>() as u64)
        .checked_add(clone_slice_bytes::<SchemaEntry>(root_count)?)?;
    let mut maximum_component = 0_u64;
    let mut count = u64::try_from(root_count).ok()?;
    let mut measure_entry = |entry: &SchemaEntry| -> Option<()> {
        charge(budget)?;
        let parameters = parameters_cost(entry.schema().dimension_parameters(), budget)?;
        let body = body_cost(
            entry.schema().body(),
            parameters,
            &mut retained,
            &mut maximum_component,
            &mut count,
            budget,
        )?;
        retained = retained
            .checked_add(u64::try_from(entry.canonical_bytes().len()).ok()?)?
            .checked_add(parameters.clone_bytes)?
            .checked_add(body.clone_bytes)?;
        Some(())
    };
    match roots {
        Some(roots) => {
            for root in roots {
                measure_entry(table.entry(*root)?)?;
            }
        }
        None => {
            for entry in table.entries() {
                measure_entry(entry)?;
            }
        }
    }
    let construction = retained
        .checked_mul(2)?
        .checked_add(maximum_component.checked_mul(5)?)?;
    Some(ComponentClosureCost {
        retained_bytes: retained,
        construction_bytes: construction,
        entry_count: count,
    })
}

impl SchemaBody {
    /// Heap layout bound reused by canonical body closure. Closing dimensions
    /// only replaces expressions with constants, so it cannot exceed a clone.
    #[doc(hidden)]
    pub fn clone_allocation_bound_bytes(&self) -> Option<u64> {
        body_clone_heap_bytes(self)
    }
}

fn clone_slice_bytes<T>(len: usize) -> Option<u64> {
    u64::try_from(len)
        .ok()?
        .checked_mul(core::mem::size_of::<T>() as u64)
}

fn schema_clone_heap_bytes(schema: &Schema) -> Option<u64> {
    let parameters = clone_slice_bytes::<DimensionParameter>(schema.dimension_parameters.len())?;
    let parameters =
        schema
            .dimension_parameters
            .iter()
            .try_fold(parameters, |total, parameter| {
                total
                    .checked_add(dimension_clone_heap_bytes(parameter.lower_bound())?)?
                    .checked_add(match parameter.upper_bound() {
                        Some(bound) => dimension_clone_heap_bytes(bound)?,
                        None => 0,
                    })
            })?;
    parameters.checked_add(body_clone_heap_bytes(&schema.body)?)
}

fn dimension_clone_heap_bytes(dimension: &DimensionExpr) -> Option<u64> {
    match dimension {
        DimensionExpr::Hole | DimensionExpr::Constant(_) | DimensionExpr::Parameter(_) => Some(0),
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => {
            let initial = clone_slice_bytes::<DimensionExpr>(children.len())?;
            children.iter().try_fold(initial, |total, child| {
                total.checked_add(dimension_clone_heap_bytes(child)?)
            })
        }
    }
}

fn extent_clone_heap_bytes(extent: &CardinalitySpec) -> Option<u64> {
    match extent {
        CardinalitySpec::Exact(value)
        | CardinalitySpec::Dynamic {
            upper_bound: Some(value),
        } => dimension_clone_heap_bytes(value),
        CardinalitySpec::Dynamic { upper_bound: None } => Some(0),
    }
}

fn fields_clone_heap_bytes(fields: &[SchemaField]) -> Option<u64> {
    fields.iter().try_fold(
        clone_slice_bytes::<SchemaField>(fields.len())?,
        |total, field| {
            total
                .checked_add(u64::try_from(field.name.len()).ok()?)?
                .checked_add(body_clone_heap_bytes(&field.schema)?)
        },
    )
}

fn boxed_body_clone_bytes(body: &SchemaBody) -> Option<u64> {
    (core::mem::size_of::<SchemaBody>() as u64).checked_add(body_clone_heap_bytes(body)?)
}

fn body_clone_heap_bytes(body: &SchemaBody) -> Option<u64> {
    match body {
        SchemaBody::Dynamic
        | SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => Some(0),
        SchemaBody::Enum { variants, .. } => {
            let initial = clone_slice_bytes::<super::EnumVariantSchema>(variants.len())?;
            variants.iter().try_fold(initial, |total, variant| {
                total
                    .checked_add(u64::try_from(variant.name.len()).ok()?)?
                    .checked_add(match &variant.payload {
                        Some(payload) => body_clone_heap_bytes(payload)?,
                        None => 0,
                    })
            })
        }
        SchemaBody::Option(element) => boxed_body_clone_bytes(element),
        SchemaBody::Tuple(elements) => elements.iter().try_fold(
            clone_slice_bytes::<SchemaBody>(elements.len())?,
            |total, element| total.checked_add(body_clone_heap_bytes(element)?),
        ),
        SchemaBody::Record(fields) => fields_clone_heap_bytes(fields),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let initial =
                boxed_body_clone_bytes(element)?
                    .checked_add(clone_slice_bytes::<DimensionExpr>(dimensions.len())?)?;
            dimensions.iter().try_fold(initial, |total, dimension| {
                total.checked_add(dimension_clone_heap_bytes(dimension)?)
            })
        }
        SchemaBody::Table { columns, rows } => {
            fields_clone_heap_bytes(columns)?.checked_add(extent_clone_heap_bytes(rows)?)
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => boxed_body_clone_bytes(element)?.checked_add(extent_clone_heap_bytes(cardinality)?),
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => boxed_body_clone_bytes(key)?
            .checked_add(boxed_body_clone_bytes(value)?)?
            .checked_add(extent_clone_heap_bytes(cardinality)?),
    }
}

impl SchemaEntry {
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    pub const fn key(&self) -> SchemaKey {
        self.key
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}
