use super::sequence::SequenceStorage;
use super::{
    CanonicalKeyValue, MapEntryValue, ReifiedType, SnapshotCanonicalizationBudget, SnapshotPath,
    SnapshotValueError, Value, ValueData,
};
use crate::{FloatWidth, SchemaBody, SchemaTable};
use core::cmp::Ordering;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetValueRelation {
    Disjoint,
    Equal,
    NotEqual,
    ProperSubset,
    ProperSuperset,
    Subset,
    Superset,
}

impl Value {
    pub fn snapshot_eq(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<bool, SnapshotValueError> {
        let self_schema = self.validate_against(self_schemas)?;
        let other_schema = other.validate_against(other_schemas)?;
        if self.schema_key() != other.schema_key() {
            return Ok(false);
        }
        ensure_same_schema_definition(
            self.schema_key(),
            &self_schema.canonical_bytes(),
            &other_schema.canonical_bytes(),
        )?;
        if self.shape() != other.shape() {
            return Ok(false);
        }
        Ok(
            super::encoding::canonical_material(self_schema.body(), self.data())
                == super::encoding::canonical_material(other_schema.body(), other.data()),
        )
    }

    pub fn language_eq(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<bool, SnapshotValueError> {
        let self_schema = self.validate_against(self_schemas)?;
        let other_schema = other.validate_against(other_schemas)?;
        if self.schema_key() != other.schema_key() {
            return Ok(false);
        }
        ensure_same_schema_definition(
            self.schema_key(),
            &self_schema.canonical_bytes(),
            &other_schema.canonical_bytes(),
        )?;
        if self.shape() != other.shape() {
            return Ok(false);
        }
        Ok(language_data_eq(
            self_schema.body(),
            self.data(),
            other.data(),
        ))
    }

    pub fn key_cmp(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<Ordering, SnapshotValueError> {
        let self_schema = self.validate_against(self_schemas)?;
        let other_schema = other.validate_against(other_schemas)?;
        if !self_schema.is_keyable() || !other_schema.is_keyable() {
            return Err(SnapshotValueError::SchemaNotKeyableV1);
        }
        if self.schema_key() != other.schema_key() {
            if is_nominal_schema(self_schema.body()) && is_nominal_schema(other_schema.body()) {
                return Ok(self.schema_key().cmp(&other.schema_key()));
            }
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: self.schema_key(),
            });
        }
        ensure_same_schema_definition(
            self.schema_key(),
            &self_schema.canonical_bytes(),
            &other_schema.canonical_bytes(),
        )?;
        let shape_order = self
            .shape()
            .canonical_bytes()
            .cmp(&other.shape().canonical_bytes());
        if shape_order != Ordering::Equal {
            return Ok(shape_order);
        }
        let left = normalized_key_data(self_schema.body(), self.data().clone())?;
        let right = normalized_key_data(self_schema.body(), other.data().clone())?;
        compare_key_data(self_schema.body(), &left, &right)
    }

    pub fn set_contains(
        &self,
        self_schemas: &SchemaTable,
        candidate: &Value,
        candidate_schemas: &SchemaTable,
    ) -> Result<bool, SnapshotValueError> {
        let (element, elements, candidate) =
            self.validated_set_candidate(self_schemas, candidate, candidate_schemas)?;
        Ok(set_key_search(element, elements, &candidate)?.is_ok())
    }

    pub fn set_elements_after_insert(
        &self,
        self_schemas: &SchemaTable,
        candidate: &Value,
        candidate_schemas: &SchemaTable,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        let (element, elements, candidate) =
            self.validated_set_candidate(self_schemas, candidate, candidate_schemas)?;
        let mut next = elements
            .iter()
            .map(|value| value.data().clone())
            .collect::<Vec<_>>();
        if let Err(position) = set_key_search(element, elements, &candidate)? {
            next.insert(position, candidate);
        }
        Ok(next.into_boxed_slice())
    }

    pub fn set_elements_after_remove(
        &self,
        self_schemas: &SchemaTable,
        candidate: &Value,
        candidate_schemas: &SchemaTable,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        let (element, elements, candidate) =
            self.validated_set_candidate(self_schemas, candidate, candidate_schemas)?;
        let found = set_key_search(element, elements, &candidate)?.ok();
        let mut next =
            Vec::with_capacity(elements.len().saturating_sub(usize::from(found.is_some())));
        for (index, existing) in elements.iter().enumerate() {
            if Some(index) != found {
                next.push(existing.data().clone())
            }
        }
        Ok(next.into_boxed_slice())
    }

    pub fn set_union_elements(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(self_schemas, other, other_schemas, SetMerge::Union, None)
    }

    pub fn set_union_elements_with_budget(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        budget: &SnapshotCanonicalizationBudget,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::Union,
            Some(budget),
        )
    }

    pub fn set_intersection_elements(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::Intersection,
            None,
        )
    }

    pub fn set_intersection_elements_with_budget(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        budget: &SnapshotCanonicalizationBudget,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::Intersection,
            Some(budget),
        )
    }

    pub fn set_difference_elements(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::Difference,
            None,
        )
    }

    pub fn set_difference_elements_with_budget(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        budget: &SnapshotCanonicalizationBudget,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::Difference,
            Some(budget),
        )
    }

    pub fn set_symmetric_difference_elements(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::SymmetricDifference,
            None,
        )
    }

    pub fn set_symmetric_difference_elements_with_budget(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        budget: &SnapshotCanonicalizationBudget,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        self.merge_set_elements(
            self_schemas,
            other,
            other_schemas,
            SetMerge::SymmetricDifference,
            Some(budget),
        )
    }

    pub fn set_relation(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        relation: SetValueRelation,
    ) -> Result<bool, SnapshotValueError> {
        self.set_relation_with_optional_budget(self_schemas, other, other_schemas, relation, None)
    }

    /// Evaluates a set relation while charging every recursive ordered-key
    /// comparison against one caller-owned canonicalization allowance.
    pub fn set_relation_with_budget(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        relation: SetValueRelation,
        budget: &SnapshotCanonicalizationBudget,
    ) -> Result<bool, SnapshotValueError> {
        self.set_relation_with_optional_budget(
            self_schemas,
            other,
            other_schemas,
            relation,
            Some(budget),
        )
    }

    fn set_relation_with_optional_budget(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        relation: SetValueRelation,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Result<bool, SnapshotValueError> {
        let (element, left, right) = self.validated_set_pair(self_schemas, other, other_schemas)?;
        match relation {
            SetValueRelation::Disjoint => set_is_disjoint(element, left, right, budget),
            SetValueRelation::Equal => {
                Ok(left.len() == right.len() && set_is_subset(element, left, right, budget)?)
            }
            SetValueRelation::NotEqual => {
                Ok(left.len() != right.len() || !set_is_subset(element, left, right, budget)?)
            }
            SetValueRelation::ProperSubset => {
                Ok(left.len() < right.len() && set_is_subset(element, left, right, budget)?)
            }
            SetValueRelation::ProperSuperset => {
                Ok(left.len() > right.len() && set_is_subset(element, right, left, budget)?)
            }
            SetValueRelation::Subset => set_is_subset(element, left, right, budget),
            SetValueRelation::Superset => set_is_subset(element, right, left, budget),
        }
    }

    fn merge_set_elements(
        &self,
        self_schemas: &SchemaTable,
        other: &Value,
        other_schemas: &SchemaTable,
        merge: SetMerge,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Result<Box<[ValueData]>, SnapshotValueError> {
        let (element, left, right) = self.validated_set_pair(self_schemas, other, other_schemas)?;
        let mut next = Vec::with_capacity(left.len().saturating_add(right.len()));
        let (mut left_index, mut right_index) = (0, 0);
        while left_index < left.len() && right_index < right.len() {
            match compare_key_data_with_budget(
                element,
                left[left_index].data(),
                right[right_index].data(),
                budget,
            )? {
                Ordering::Less => {
                    if matches!(
                        merge,
                        SetMerge::Union | SetMerge::Difference | SetMerge::SymmetricDifference
                    ) {
                        next.push(left[left_index].data().clone());
                    }
                    left_index += 1;
                }
                Ordering::Equal => {
                    if matches!(merge, SetMerge::Union | SetMerge::Intersection) {
                        next.push(left[left_index].data().clone());
                    }
                    left_index += 1;
                    right_index += 1;
                }
                Ordering::Greater => {
                    if matches!(merge, SetMerge::Union | SetMerge::SymmetricDifference) {
                        next.push(right[right_index].data().clone());
                    }
                    right_index += 1;
                }
            }
        }
        if matches!(
            merge,
            SetMerge::Union | SetMerge::Difference | SetMerge::SymmetricDifference
        ) {
            next.extend(left[left_index..].iter().map(|value| value.data().clone()));
        }
        if matches!(merge, SetMerge::Union | SetMerge::SymmetricDifference) {
            next.extend(
                right[right_index..]
                    .iter()
                    .map(|value| value.data().clone()),
            );
        }
        Ok(next.into_boxed_slice())
    }

    fn validated_set_pair<'a>(
        &'a self,
        self_schemas: &'a SchemaTable,
        other: &'a Value,
        other_schemas: &'a SchemaTable,
    ) -> Result<
        (
            &'a SchemaBody,
            &'a [CanonicalKeyValue],
            &'a [CanonicalKeyValue],
        ),
        SnapshotValueError,
    > {
        let self_schema = self.validate_against(self_schemas)?;
        let other_schema = other.validate_against(other_schemas)?;
        let SchemaBody::Set {
            element: self_element,
            ..
        } = self_schema.body()
        else {
            return Err(SnapshotValueError::SnapshotDataSchemaMismatch {
                path: SnapshotPath::root(),
                expected: super::SchemaDataKind::Set,
                actual: self.data().kind(),
            });
        };
        let SchemaBody::Set {
            element: other_element,
            ..
        } = other_schema.body()
        else {
            return Err(SnapshotValueError::SnapshotDataSchemaMismatch {
                path: SnapshotPath::root(),
                expected: super::SchemaDataKind::Set,
                actual: other.data().kind(),
            });
        };
        if self_element != other_element {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: other.schema_key(),
            });
        }
        let (ValueData::Set(left), ValueData::Set(right)) = (self.data(), other.data()) else {
            unreachable!("validated set schemas have set data")
        };
        Ok((self_element, left.elements(), right.elements()))
    }

    fn validated_set_candidate<'a>(
        &'a self,
        self_schemas: &'a SchemaTable,
        candidate: &Value,
        candidate_schemas: &SchemaTable,
    ) -> Result<(&'a SchemaBody, &'a [CanonicalKeyValue], ValueData), SnapshotValueError> {
        let schema = self.validate_against(self_schemas)?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(SnapshotValueError::SnapshotDataSchemaMismatch {
                path: SnapshotPath::root(),
                expected: super::SchemaDataKind::Set,
                actual: self.data().kind(),
            });
        };
        let ValueData::Set(set) = self.data() else {
            unreachable!("validated set schema has set data")
        };
        let candidate_schema = candidate.validate_against(candidate_schemas)?;
        if candidate_schema.body() != element.as_ref() {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: candidate.schema_key(),
            });
        }
        let candidate = normalized_key_data(element, candidate.data().clone())?;
        Ok((element, set.elements(), candidate))
    }
}

#[derive(Clone, Copy)]
enum SetMerge {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

fn set_key_search(
    element: &SchemaBody,
    elements: &[CanonicalKeyValue],
    candidate: &ValueData,
) -> Result<core::result::Result<usize, usize>, SnapshotValueError> {
    let mut lower = 0usize;
    let mut upper = elements.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        match compare_key_data(element, elements[middle].data(), candidate)? {
            Ordering::Less => lower = middle + 1,
            Ordering::Greater => upper = middle,
            Ordering::Equal => return Ok(Ok(middle)),
        }
    }
    Ok(Err(lower))
}

fn set_is_subset(
    element: &SchemaBody,
    left: &[CanonicalKeyValue],
    right: &[CanonicalKeyValue],
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<bool, SnapshotValueError> {
    let (mut left_index, mut right_index) = (0, 0);
    while left_index < left.len() && right_index < right.len() {
        match compare_key_data_with_budget(
            element,
            left[left_index].data(),
            right[right_index].data(),
            budget,
        )? {
            Ordering::Less => return Ok(false),
            Ordering::Equal => {
                left_index += 1;
                right_index += 1;
            }
            Ordering::Greater => right_index += 1,
        }
    }
    Ok(left_index == left.len())
}

fn set_is_disjoint(
    element: &SchemaBody,
    left: &[CanonicalKeyValue],
    right: &[CanonicalKeyValue],
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<bool, SnapshotValueError> {
    let (mut left_index, mut right_index) = (0, 0);
    while left_index < left.len() && right_index < right.len() {
        match compare_key_data_with_budget(
            element,
            left[left_index].data(),
            right[right_index].data(),
            budget,
        )? {
            Ordering::Less => left_index += 1,
            Ordering::Equal => return Ok(false),
            Ordering::Greater => right_index += 1,
        }
    }
    Ok(true)
}

fn is_nominal_schema(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Atom(_) | SchemaBody::Enum { .. })
}

fn ensure_same_schema_definition(
    key: crate::SchemaKey,
    left: &[u8],
    right: &[u8],
) -> Result<(), SnapshotValueError> {
    if left == right {
        Ok(())
    } else {
        Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { key })
    }
}

pub(super) fn normalized_key_data(
    schema: &SchemaBody,
    data: ValueData,
) -> Result<ValueData, SnapshotValueError> {
    if !schema_is_keyable(schema) {
        return Err(SnapshotValueError::SchemaNotKeyableV1);
    }
    Ok(match (schema, data) {
        (SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(value)) => {
            ValueData::F32(super::F32Bits::from_bits(normalize_f32(value.bits())))
        }
        (SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(value)) => {
            ValueData::F64(super::F64Bits::from_bits(normalize_f64(value.bits())))
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => ValueData::Option(
            value
                .map(|value| normalized_key_data(element, *value).map(Box::new))
                .transpose()?,
        ),
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(mut value)) => {
            if let (Some(payload), Some(payload_schema)) = (
                value.payload.take(),
                variants[value.ordinal as usize].payload.as_ref(),
            ) {
                value.payload = Some(Box::new(normalized_key_data(payload_schema, *payload)?));
            }
            ValueData::Enum(value)
        }
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => ValueData::Tuple(
            elements
                .iter()
                .zip(values.into_vec())
                .map(|(schema, value)| normalized_key_data(schema, value))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        (SchemaBody::Record(fields), ValueData::Record(mut value)) => {
            value.fields = fields
                .iter()
                .zip(value.fields.into_vec())
                .map(|(field, value)| normalized_key_data(&field.schema, value))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice();
            ValueData::Record(value)
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(mut value)) => {
            value.elements = value
                .elements
                .into_vec()
                .into_iter()
                .map(|value| {
                    Ok(CanonicalKeyValue {
                        data: normalized_key_data(element, value.data)?,
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?
                .into_boxed_slice();
            ValueData::Set(value)
        }
        (_, data) => data,
    })
}

pub(super) fn insert_set_key(
    schema: &SchemaBody,
    elements: &mut Vec<CanonicalKeyValue>,
    data: ValueData,
    path: &SnapshotPath,
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<(), SnapshotValueError> {
    let key = CanonicalKeyValue {
        data: normalized_key_data(schema, data)?,
    };
    if let Some(last) = elements.last() {
        match compare_key_data_with_budget(schema, last.data(), key.data(), budget)? {
            Ordering::Less => {
                elements.push(key);
                return Ok(());
            }
            Ordering::Equal => {
                return Err(SnapshotValueError::DuplicateCanonicalKeyV1 { path: path.clone() });
            }
            Ordering::Greater => {}
        }
    }
    let mut insertion = elements.len();
    for (index, existing) in elements.iter().enumerate() {
        match compare_key_data_with_budget(schema, existing.data(), key.data(), budget)? {
            Ordering::Less => {}
            Ordering::Equal => {
                return Err(SnapshotValueError::DuplicateCanonicalKeyV1 { path: path.clone() });
            }
            Ordering::Greater => {
                insertion = index;
                break;
            }
        }
    }
    if let Some(budget) = budget {
        budget.charge(u64::try_from(elements.len() - insertion).unwrap_or(u64::MAX))?;
    }
    elements.insert(insertion, key);
    Ok(())
}

pub(super) fn insert_map_entry(
    schema: &SchemaBody,
    entries: &mut Vec<MapEntryValue>,
    key: ValueData,
    value: ValueData,
    path: &SnapshotPath,
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<(), SnapshotValueError> {
    let key = CanonicalKeyValue {
        data: normalized_key_data(schema, key)?,
    };
    if let Some(last) = entries.last() {
        match compare_key_data_with_budget(schema, last.key().data(), key.data(), budget)? {
            Ordering::Less => {
                entries.push(MapEntryValue { key, value });
                return Ok(());
            }
            Ordering::Equal => {
                return Err(SnapshotValueError::DuplicateCanonicalKeyV1 { path: path.clone() });
            }
            Ordering::Greater => {}
        }
    }
    let mut insertion = entries.len();
    for (index, existing) in entries.iter().enumerate() {
        match compare_key_data_with_budget(schema, existing.key().data(), key.data(), budget)? {
            Ordering::Less => {}
            Ordering::Equal => {
                return Err(SnapshotValueError::DuplicateCanonicalKeyV1 { path: path.clone() });
            }
            Ordering::Greater => {
                insertion = index;
                break;
            }
        }
    }
    if let Some(budget) = budget {
        budget.charge(u64::try_from(entries.len() - insertion).unwrap_or(u64::MAX))?;
    }
    entries.insert(insertion, MapEntryValue { key, value });
    Ok(())
}

pub fn compare_key_data(
    schema: &SchemaBody,
    left: &ValueData,
    right: &ValueData,
) -> Result<Ordering, SnapshotValueError> {
    compare_key_data_with_budget(schema, left, right, None)
}

fn compare_key_data_with_budget(
    schema: &SchemaBody,
    left: &ValueData,
    right: &ValueData,
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<Ordering, SnapshotValueError> {
    if !schema_is_keyable(schema) {
        return Err(SnapshotValueError::SchemaNotKeyableV1);
    }
    if let Some(budget) = budget {
        let work = match (schema, left, right) {
            (SchemaBody::String, ValueData::String(left), ValueData::String(right)) => {
                u64::try_from(left.len().max(right.len()).max(1)).unwrap_or(u64::MAX)
            }
            _ => 1,
        };
        budget.charge(work)?;
    }
    let order = match (schema, left, right) {
        (SchemaBody::Bool, ValueData::Bool(left), ValueData::Bool(right)) => left.cmp(right),
        (SchemaBody::UnsignedInteger(_), _, _) | (SchemaBody::SignedInteger(_), _, _) => {
            integer_cmp(left, right)
        }
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            ValueData::F32(left),
            ValueData::F32(right),
        ) => ordered_f32(left.bits()).cmp(&ordered_f32(right.bits())),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueData::F64(left),
            ValueData::F64(right),
        ) => ordered_f64(left.bits()).cmp(&ordered_f64(right.bits())),
        (SchemaBody::Rational64, ValueData::Rational64(left), ValueData::Rational64(right)) => {
            rational_cmp(left, right)
        }
        (SchemaBody::String, ValueData::String(left), ValueData::String(right)) => {
            left.as_bytes().cmp(right.as_bytes())
        }
        (SchemaBody::Id, ValueData::Id(left), ValueData::Id(right))
        | (SchemaBody::Index, ValueData::Index(left), ValueData::Index(right)) => left.cmp(right),
        (SchemaBody::Atom(_), ValueData::Atom, ValueData::Atom) => Ordering::Equal,
        (SchemaBody::Option(element), ValueData::Option(left), ValueData::Option(right)) => {
            match (left, right) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some(left), Some(right)) => {
                    compare_key_data_with_budget(element, left, right, budget)?
                }
            }
        }
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(left), ValueData::Enum(right)) => {
            let ordinal = left.ordinal.cmp(&right.ordinal);
            if ordinal != Ordering::Equal {
                ordinal
            } else {
                match (
                    variants[left.ordinal as usize].payload.as_ref(),
                    left.payload.as_deref(),
                    right.payload.as_deref(),
                ) {
                    (Some(schema), Some(left), Some(right)) => {
                        compare_key_data_with_budget(schema, left, right, budget)?
                    }
                    _ => Ordering::Equal,
                }
            }
        }
        (SchemaBody::Tuple(elements), ValueData::Tuple(left), ValueData::Tuple(right)) => {
            lexicographic(elements, left, right, budget)?
        }
        (SchemaBody::Record(fields), ValueData::Record(left), ValueData::Record(right)) => {
            let mut order = Ordering::Equal;
            for ((field, left), right) in fields.iter().zip(left.fields()).zip(right.fields()) {
                order = compare_key_data_with_budget(&field.schema, left, right, budget)?;
                if order != Ordering::Equal {
                    break;
                }
            }
            order
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(left), ValueData::Set(right)) => {
            let mut order = Ordering::Equal;
            for (left, right) in left.elements.iter().zip(right.elements.iter()) {
                order = compare_key_data_with_budget(element, left.data(), right.data(), budget)?;
                if order != Ordering::Equal {
                    break;
                }
            }
            order.then_with(|| left.elements.len().cmp(&right.elements.len()))
        }
        _ => unreachable!("validated key data changed representation"),
    };
    Ok(order)
}

fn rational_cmp(left: &super::Rational64Value, right: &super::Rational64Value) -> Ordering {
    (i128::from(left.numerator()) * i128::from(right.denominator()))
        .cmp(&(i128::from(right.numerator()) * i128::from(left.denominator())))
}

fn integer_cmp(left: &ValueData, right: &ValueData) -> Ordering {
    macro_rules! compare {
        ($variant:ident) => {
            if let (ValueData::$variant(left), ValueData::$variant(right)) = (left, right) {
                return left.cmp(right);
            }
        };
    }
    compare!(U8);
    compare!(U16);
    compare!(U32);
    compare!(U64);
    compare!(U128);
    compare!(I8);
    compare!(I16);
    compare!(I32);
    compare!(I64);
    compare!(I128);
    unreachable!("validated integer data changed width")
}

fn lexicographic(
    schemas: &[SchemaBody],
    left: &[ValueData],
    right: &[ValueData],
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<Ordering, SnapshotValueError> {
    for ((schema, left), right) in schemas.iter().zip(left).zip(right) {
        let order = compare_key_data_with_budget(schema, left, right, budget)?;
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(Ordering::Equal)
}

/// Applies the canonical language equality rules to two already-validated
/// payloads under one shared schema.
pub fn schema_data_language_eq(schema: &SchemaBody, left: &ValueData, right: &ValueData) -> bool {
    language_data_eq(schema, left, right)
}

/// Applies the source language's scalar numeric ordering to two already
/// validated payloads under one shared schema. `None` represents an unordered
/// floating-point comparison (for example, one involving NaN).
pub fn schema_data_partial_cmp(
    schema: &SchemaBody,
    left: &ValueData,
    right: &ValueData,
) -> Option<Ordering> {
    macro_rules! ordinary {
        ($variant:ident) => {
            if let (ValueData::$variant(left), ValueData::$variant(right)) = (left, right) {
                return left.partial_cmp(right);
            }
        };
    }
    match (schema, left, right) {
        (SchemaBody::UnsignedInteger(_), _, _) | (SchemaBody::SignedInteger(_), _, _) => {
            ordinary!(U8);
            ordinary!(U16);
            ordinary!(U32);
            ordinary!(U64);
            ordinary!(U128);
            ordinary!(I8);
            ordinary!(I16);
            ordinary!(I32);
            ordinary!(I64);
            ordinary!(I128);
            None
        }
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            ValueData::F32(left),
            ValueData::F32(right),
        ) => left.to_f32().partial_cmp(&right.to_f32()),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueData::F64(left),
            ValueData::F64(right),
        ) => left.to_f64().partial_cmp(&right.to_f64()),
        (
            SchemaBody::Complex(FloatWidth::W32),
            ValueData::Complex32(left),
            ValueData::Complex32(right),
        ) => left
            .real()
            .to_f32()
            .hypot(left.imaginary().to_f32())
            .partial_cmp(&right.real().to_f32().hypot(right.imaginary().to_f32())),
        (
            SchemaBody::Complex(FloatWidth::W64),
            ValueData::Complex64(left),
            ValueData::Complex64(right),
        ) => left
            .real()
            .to_f64()
            .hypot(left.imaginary().to_f64())
            .partial_cmp(&right.real().to_f64().hypot(right.imaginary().to_f64())),
        (SchemaBody::Rational64, ValueData::Rational64(left), ValueData::Rational64(right)) => {
            Some(rational_cmp(left, right))
        }
        _ => None,
    }
}

fn language_data_eq(schema: &SchemaBody, left: &ValueData, right: &ValueData) -> bool {
    match (schema, left, right) {
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            ValueData::F32(left),
            ValueData::F32(right),
        ) => left.to_f32() == right.to_f32(),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueData::F64(left),
            ValueData::F64(right),
        ) => left.to_f64() == right.to_f64(),
        (
            SchemaBody::Complex(FloatWidth::W32),
            ValueData::Complex32(left),
            ValueData::Complex32(right),
        ) => {
            left.real().to_f32() == right.real().to_f32()
                && left.imaginary().to_f32() == right.imaginary().to_f32()
        }
        (
            SchemaBody::Complex(FloatWidth::W64),
            ValueData::Complex64(left),
            ValueData::Complex64(right),
        ) => {
            left.real().to_f64() == right.real().to_f64()
                && left.imaginary().to_f64() == right.imaginary().to_f64()
        }
        (SchemaBody::Option(element), ValueData::Option(left), ValueData::Option(right)) => {
            match (left, right) {
                (None, None) => true,
                (Some(left), Some(right)) => language_data_eq(element, left, right),
                _ => false,
            }
        }
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(left), ValueData::Enum(right)) => {
            left.ordinal == right.ordinal
                && match (
                    variants[left.ordinal as usize].payload.as_ref(),
                    left.payload.as_deref(),
                    right.payload.as_deref(),
                ) {
                    (None, None, None) => true,
                    (Some(schema), Some(left), Some(right)) => {
                        language_data_eq(schema, left, right)
                    }
                    _ => false,
                }
        }
        (SchemaBody::Tuple(elements), ValueData::Tuple(left), ValueData::Tuple(right)) => elements
            .iter()
            .zip(left)
            .zip(right)
            .all(|((schema, left), right)| language_data_eq(schema, left, right)),
        (SchemaBody::Record(fields), ValueData::Record(left), ValueData::Record(right)) => fields
            .iter()
            .zip(left.fields())
            .zip(right.fields())
            .all(|((field, left), right)| language_data_eq(&field.schema, left, right)),
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(left), ValueData::Matrix(right)) => {
            language_sequence_eq(element, &left.elements, &right.elements)
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(left), ValueData::Table(right)) => {
            columns
                .iter()
                .zip(left.columns.iter().zip(right.columns.iter()))
                .all(|(column, (left, right))| language_sequence_eq(&column.schema, left, right))
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(left), ValueData::Set(right)) => {
            left.elements.len() == right.elements.len()
                && left
                    .elements
                    .iter()
                    .zip(right.elements.iter())
                    .all(|(left, right)| {
                        matches!(
                            compare_key_data(element, left.data(), right.data()),
                            Ok(Ordering::Equal)
                        )
                    })
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(left), ValueData::Map(right)) => {
            left.entries.len() == right.entries.len()
                && left
                    .entries
                    .iter()
                    .zip(right.entries.iter())
                    .all(|(left, right)| {
                        matches!(
                            compare_key_data(key, left.key().data(), right.key().data()),
                            Ok(Ordering::Equal)
                        ) && language_data_eq(value, left.value(), right.value())
                    })
        }
        _ => exact_leaf_eq(left, right),
    }
}

fn exact_leaf_eq(left: &ValueData, right: &ValueData) -> bool {
    match (left, right) {
        (ValueData::U8(left), ValueData::U8(right)) => left == right,
        (ValueData::U16(left), ValueData::U16(right)) => left == right,
        (ValueData::U32(left), ValueData::U32(right)) => left == right,
        (ValueData::U64(left), ValueData::U64(right)) => left == right,
        (ValueData::U128(left), ValueData::U128(right)) => left == right,
        (ValueData::I8(left), ValueData::I8(right)) => left == right,
        (ValueData::I16(left), ValueData::I16(right)) => left == right,
        (ValueData::I32(left), ValueData::I32(right)) => left == right,
        (ValueData::I64(left), ValueData::I64(right)) => left == right,
        (ValueData::I128(left), ValueData::I128(right)) => left == right,
        (ValueData::F32(left), ValueData::F32(right)) => left == right,
        (ValueData::F64(left), ValueData::F64(right)) => left == right,
        (ValueData::Complex32(left), ValueData::Complex32(right)) => left == right,
        (ValueData::Complex64(left), ValueData::Complex64(right)) => left == right,
        (ValueData::Rational64(left), ValueData::Rational64(right)) => {
            left.numerator() == right.numerator() && left.denominator() == right.denominator()
        }
        (ValueData::Bool(left), ValueData::Bool(right)) => left == right,
        (ValueData::String(left), ValueData::String(right)) => left == right,
        (ValueData::Id(left), ValueData::Id(right)) => left == right,
        (ValueData::Index(left), ValueData::Index(right)) => left == right,
        (ValueData::Atom, ValueData::Atom) => true,
        (
            ValueData::Type(ReifiedType::Schema(left)),
            ValueData::Type(ReifiedType::Schema(right)),
        ) => left == right,
        (ValueData::Type(ReifiedType::Kind(left)), ValueData::Type(ReifiedType::Kind(right))) => {
            left.canonical_bytes() == right.canonical_bytes()
        }
        _ => false,
    }
}

fn language_sequence_eq(
    schema: &SchemaBody,
    left: &SequenceStorage,
    right: &SequenceStorage,
) -> bool {
    match (left, right) {
        (SequenceStorage::F32(left), SequenceStorage::F32(right)) => left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| left.to_f32() == right.to_f32()),
        (SequenceStorage::F64(left), SequenceStorage::F64(right)) => left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| left.to_f64() == right.to_f64()),
        (SequenceStorage::Complex32(left), SequenceStorage::Complex32(right)) => {
            left.iter().zip(right.iter()).all(|(left, right)| {
                left.real().to_f32() == right.real().to_f32()
                    && left.imaginary().to_f32() == right.imaginary().to_f32()
            })
        }
        (SequenceStorage::Complex64(left), SequenceStorage::Complex64(right)) => {
            left.iter().zip(right.iter()).all(|(left, right)| {
                left.real().to_f64() == right.real().to_f64()
                    && left.imaginary().to_f64() == right.imaginary().to_f64()
            })
        }
        (SequenceStorage::Values(left), SequenceStorage::Values(right)) => left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| language_data_eq(schema, left, right)),
        _ => sequence_exact_eq(left, right),
    }
}

fn sequence_exact_eq(left: &SequenceStorage, right: &SequenceStorage) -> bool {
    macro_rules! slices {
        ($variant:ident) => {
            if let (SequenceStorage::$variant(left), SequenceStorage::$variant(right)) =
                (left, right)
            {
                return left == right;
            }
        };
    }
    slices!(U8);
    slices!(U16);
    slices!(U32);
    slices!(U64);
    slices!(U128);
    slices!(I8);
    slices!(I16);
    slices!(I32);
    slices!(I64);
    slices!(I128);
    slices!(F32);
    slices!(F64);
    slices!(Complex32);
    slices!(Complex64);
    slices!(Bool);
    slices!(String);
    slices!(Id);
    slices!(Index);
    match (left, right) {
        (SequenceStorage::Unit(left), SequenceStorage::Unit(right)) => left == right,
        (SequenceStorage::Rational64(left), SequenceStorage::Rational64(right)) => {
            left.iter().zip(right.iter()).all(|(left, right)| {
                left.numerator() == right.numerator() && left.denominator() == right.denominator()
            })
        }
        _ => false,
    }
}

pub(super) fn schema_is_keyable(schema: &SchemaBody) -> bool {
    match schema {
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_) => true,
        SchemaBody::Enum { variants, .. } => {
            let mut index = 0;
            while index < variants.len() {
                if let Some(payload) = &variants[index].payload
                    && !schema_is_keyable(payload)
                {
                    return false;
                }
                index += 1;
            }
            true
        }
        SchemaBody::Option(element) => schema_is_keyable(element),
        SchemaBody::Tuple(elements) => {
            let mut index = 0;
            while index < elements.len() {
                if !schema_is_keyable(&elements[index]) {
                    return false;
                }
                index += 1;
            }
            true
        }
        SchemaBody::Record(fields) => {
            let mut index = 0;
            while index < fields.len() {
                if !schema_is_keyable(&fields[index].schema) {
                    return false;
                }
                index += 1;
            }
            true
        }
        SchemaBody::Dynamic | SchemaBody::Complex(_) => false,
        SchemaBody::Matrix { element, .. } => schema_is_keyable(element),
        SchemaBody::Set { element, .. } => schema_is_keyable(element),
        SchemaBody::Table { .. } | SchemaBody::Map { .. } | SchemaBody::ReifiedType => false,
    }
}

const fn normalize_f32(bits: u32) -> u32 {
    let magnitude = bits & 0x7fff_ffff;
    if magnitude == 0 {
        0
    } else if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        0x7fc0_0000
    } else {
        bits
    }
}

const fn normalize_f64(bits: u64) -> u64 {
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;
    if magnitude == 0 {
        0
    } else if bits & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000
        && bits & 0x000f_ffff_ffff_ffff != 0
    {
        0x7ff8_0000_0000_0000
    } else {
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_schema_keys_still_require_equal_canonical_definitions() {
        let key = crate::SchemaKey::from_bytes([7; 32]);
        assert!(matches!(
            ensure_same_schema_definition(key, &[1], &[2]),
            Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: actual })
                if actual == key
        ));
    }
}

const fn ordered_f32(bits: u32) -> u32 {
    let bits = normalize_f32(bits);
    if bits & (1 << 31) != 0 {
        !bits
    } else {
        bits | (1 << 31)
    }
}

const fn ordered_f64(bits: u64) -> u64 {
    let bits = normalize_f64(bits);
    if bits & (1 << 63) != 0 {
        !bits
    } else {
        bits | (1 << 63)
    }
}
