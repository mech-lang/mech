//! Copy-on-write schema storage for one frontend's pending values.
//! Canonicalization is pure schema data; no shape, factory or visit proof lives here.
use mech_core::{Schema, SchemaDraft, SemanticModelError};
use std::borrow::Borrow;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, OnceLock};

#[derive(Clone)]
pub(super) struct PendingSchema(Arc<Data>);

struct Data {
    draft: SchemaDraft,
    canonical: OnceLock<Result<Schema, SemanticModelError>>,
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Self {
            draft: self.draft.clone(),
            canonical: OnceLock::new(),
        }
    }
}

impl From<SchemaDraft> for PendingSchema {
    fn from(draft: SchemaDraft) -> Self {
        Self(Arc::new(Data {
            draft,
            canonical: OnceLock::new(),
        }))
    }
}
impl From<PendingSchema> for SchemaDraft {
    fn from(schema: PendingSchema) -> Self {
        schema.into_owned()
    }
}

impl PendingSchema {
    pub(super) fn into_owned(self) -> SchemaDraft {
        match Arc::try_unwrap(self.0) {
            Ok(data) => data.draft,
            Err(data) => data.draft.clone(),
        }
    }
    pub(super) fn finalize_ref(&self) -> Result<Schema, SemanticModelError> {
        self.0
            .canonical
            .get_or_init(|| self.0.draft.finalize_ref())
            .clone()
    }
    pub(super) fn finalize(self) -> Result<Schema, SemanticModelError> {
        self.finalize_ref()
    }
}

impl Deref for PendingSchema {
    type Target = SchemaDraft;
    fn deref(&self) -> &SchemaDraft {
        &self.0.draft
    }
}
impl DerefMut for PendingSchema {
    fn deref_mut(&mut self) -> &mut SchemaDraft {
        let data = Arc::make_mut(&mut self.0);
        // Even a unique owner may have been finalized before this mutation.
        data.canonical.take();
        &mut data.draft
    }
}
impl AsRef<SchemaDraft> for PendingSchema {
    fn as_ref(&self) -> &SchemaDraft {
        self
    }
}
impl Borrow<SchemaDraft> for PendingSchema {
    fn borrow(&self) -> &SchemaDraft {
        self
    }
}
impl fmt::Debug for PendingSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}
impl PartialEq for PendingSchema {
    fn eq(&self, rhs: &Self) -> bool {
        self.deref() == rhs.deref()
    }
}
impl Eq for PendingSchema {}
impl PartialEq<SchemaDraft> for PendingSchema {
    fn eq(&self, rhs: &SchemaDraft) -> bool {
        self.deref() == rhs
    }
}
impl PartialEq<PendingSchema> for SchemaDraft {
    fn eq(&self, rhs: &PendingSchema) -> bool {
        self == rhs.deref()
    }
}

pub(super) trait FinalizeSourceSchema {
    fn finalize_source_schema(&self) -> Result<Schema, SemanticModelError>;
}

/// Preserve a pending owner's pure canonicalization cache through mixed visitors.
/// Schemas without a pending owner retain their uncached finalization path.
pub(super) enum SourceSchemaRef<'a> {
    Pending(&'a PendingSchema),
    Draft(&'a SchemaDraft),
}

impl<'a> From<&'a PendingSchema> for SourceSchemaRef<'a> {
    fn from(schema: &'a PendingSchema) -> Self {
        Self::Pending(schema)
    }
}
impl<'a> From<&'a SchemaDraft> for SourceSchemaRef<'a> {
    fn from(schema: &'a SchemaDraft) -> Self {
        Self::Draft(schema)
    }
}
impl Deref for SourceSchemaRef<'_> {
    type Target = SchemaDraft;
    fn deref(&self) -> &SchemaDraft {
        match self {
            Self::Pending(schema) => schema,
            Self::Draft(schema) => schema,
        }
    }
}
impl FinalizeSourceSchema for SourceSchemaRef<'_> {
    fn finalize_source_schema(&self) -> Result<Schema, SemanticModelError> {
        match self {
            Self::Pending(schema) => schema.finalize_source_schema(),
            Self::Draft(schema) => schema.finalize_source_schema(),
        }
    }
}

impl FinalizeSourceSchema for PendingSchema {
    fn finalize_source_schema(&self) -> Result<Schema, SemanticModelError> {
        self.finalize_ref()
    }
}
impl FinalizeSourceSchema for SchemaDraft {
    fn finalize_source_schema(&self) -> Result<Schema, SemanticModelError> {
        self.finalize_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, IntegerWidth, SchemaBody,
    };

    fn draft() -> SchemaDraft {
        SchemaDraft {
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: Box::new([
                    DimensionExpr::Constant(1),
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                ]),
            },
        }
    }

    #[test]
    fn cloned_owners_keep_independent_mutations_and_error_caches() {
        let raw = draft();
        let base = PendingSchema::from(raw.clone());
        let expected = raw.finalize_ref();
        assert_eq!(base.finalize_ref(), expected);
        let mut branch = base.clone();
        assert!(Arc::ptr_eq(&base.0, &branch.0));
        // Even unused declarations must retain their errors after a mutation.
        branch.body = SchemaBody::Bool;
        branch.dimension_parameters[0].lower_bound =
            DimensionExpr::Parameter(DimensionParameterId::new(7));
        assert!(branch.finalize_ref().is_err());
        assert_eq!(branch.finalize_ref(), branch.as_ref().finalize_ref());
        assert_eq!(base.finalize_ref(), expected);
        assert!(!Arc::ptr_eq(&base.0, &branch.0));
        // Repair after caching the error, including the unique-owner path.
        branch.dimension_parameters[0].lower_bound = DimensionExpr::Constant(0);
        assert!(branch.finalize_ref().is_ok());
        assert_eq!(branch.finalize_ref(), branch.as_ref().finalize_ref());
        assert_eq!(format!("{branch:?}"), format!("{:?}", branch.as_ref()));
    }

    #[test]
    fn repeated_queries_and_mutations_match_uncached_finalization() {
        for seed in 0..512u32 {
            let mut raw = draft();
            raw.dimension_parameters[0].origin = if seed & 1 == 0 {
                DimensionParameterOrigin::Explicit
            } else {
                DimensionParameterOrigin::Inferred
            };
            raw.dimension_parameters[0].upper_bound = Some(DimensionExpr::Constant(seed as u64));
            let base = PendingSchema::from(raw);
            for mutation in 0..4 {
                let mut branch = base.clone();
                assert_eq!(branch.finalize_ref(), branch.as_ref().finalize_ref());
                match mutation {
                    0 => branch.body = SchemaBody::Bool,
                    1 => branch.dimension_parameters[0].lifetime = DimensionLifetime::CompileTime,
                    2 => branch.dimension_parameters[0].id = DimensionParameterId::new(seed + 1),
                    _ => {
                        branch.dimension_parameters[0].lower_bound =
                            DimensionExpr::Parameter(DimensionParameterId::new(seed % 3))
                    }
                }
                for _ in 0..2 {
                    assert_eq!(branch.finalize_ref(), branch.as_ref().finalize_ref());
                    assert_eq!(base.finalize_ref(), base.as_ref().finalize_ref());
                }
            }
        }
    }

    #[test]
    fn materialized_drafts_do_not_mutate_remaining_shared_owners() {
        let base = PendingSchema::from(draft());
        let expected = base.finalize_ref();
        assert_eq!(base.finalize_source_schema(), expected);
        assert_eq!(base.clone().finalize(), expected);
        let mut owned = base.clone().into_owned();
        owned.body = SchemaBody::Bool;
        assert_eq!(base.finalize_ref(), expected);
        let unique = PendingSchema::from(owned.clone()).into_owned();
        assert_eq!(unique, owned);
    }

    #[test]
    fn unique_owner_mutation_invalidates_a_successful_borrowed_finalization() {
        let mut schema = PendingSchema::from(draft());
        let expected = schema.as_ref().finalize_ref();
        assert_eq!(
            SourceSchemaRef::from(&schema).finalize_source_schema(),
            expected
        );
        assert!(schema.0.canonical.get().is_some());
        assert_eq!(Arc::strong_count(&schema.0), 1);
        schema.body = SchemaBody::Bool;
        assert!(schema.0.canonical.get().is_none());
        let changed = schema.as_ref().finalize_ref();
        assert_ne!(changed, expected);
        assert_eq!(
            SourceSchemaRef::from(&schema).finalize_source_schema(),
            changed
        );
        let raw = schema.as_ref();
        assert_eq!(SourceSchemaRef::from(raw).finalize_source_schema(), changed);
        assert_eq!(SourceSchemaRef::from(&schema).body, raw.body);
    }

    #[test]
    fn immutable_canonicalization_is_safe_across_concurrent_readers() {
        let base = PendingSchema::from(draft());
        let expected = base.as_ref().finalize_ref();
        let threads = (0..8)
            .map(|_| {
                let schema = base.clone();
                std::thread::spawn(move || {
                    (0..16).map(|_| schema.finalize_ref()).collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            for result in thread.join().unwrap() {
                assert_eq!(result, expected);
            }
        }
    }
}
