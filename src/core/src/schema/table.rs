use super::{CardinalitySpec, Schema, SchemaBody, SchemaField};
use crate::{
    DimensionExpr, DimensionParameter, SchemaId, SchemaKey, SemanticIdentityKind,
    SemanticModelError,
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
    use crate::{IntegerWidth, SchemaBody, SchemaDraft};

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
