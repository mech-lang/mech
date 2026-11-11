//! Confined compiler output, never a public semantic proof deserializer.
use super::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    dimensions: Box<[DimensionParameterDeclaration]>,
    kind: KindExpr,
    evidence: Box<[(KindExpr, u16)]>,
}

impl Serialize for ResolvedType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Image {
            dimensions: self.dimension_parameters.clone(),
            kind: self.kind.clone(),
            evidence: self
                .predicate_evidence
                .iter()
                .map(|e| (e.kind.clone(), e.predicates.image_bits()))
                .collect(),
        }
        .serialize(serializer)
    }
}

impl ResolvedType {
    /// Restore an already normalized compiler-produced type without resolving
    /// its schema again. This type intentionally has no public Deserialize.
    ///
    /// # Safety
    /// Confine the returned metadata until every byte, including complete
    /// predicate evidence, is established as unchanged output of the same
    /// checked compiler and genuinely bound Object. Alternatively, independently
    /// derive the type from its canonical schema and compare it with
    /// `has_same_normalized_evidence` before retaining it. Serialized fields,
    /// IDs and digests alone establish no semantic or execution authority.
    pub unsafe fn from_build_owned_image_reader<'de, D: Deserializer<'de>>(
        reader: D,
    ) -> Result<Self, D::Error> {
        let image = Image::deserialize(reader)?;
        let predicate_evidence = image
            .evidence
            .into_vec()
            .into_iter()
            .map(|(kind, bits)| {
                BuiltinKindPredicateSet::from_image_bits(bits)
                    .map(|predicates| KindPredicateEvidence { kind, predicates })
                    .ok_or_else(|| D::Error::custom("unknown normalized kind predicate bit"))
            })
            .collect::<Result<Box<[_]>, _>>()?;
        Ok(Self {
            dimension_parameters: image.dimensions,
            kind: image.kind,
            predicate_evidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_resolved_type_roundtrip_compares_evidence_that_type_equality_omits() {
        for scalar in BuiltinScalarKind::ALL {
            let resolved = ResolvedType::new(scalar.kind_expr(), Box::new([])).unwrap();
            let bytes = serde_json::to_vec(&resolved).unwrap();
            let mut reader = serde_json::Deserializer::from_slice(&bytes);
            // SAFETY: exact compiler serialization, confined comparison only.
            let loaded =
                unsafe { ResolvedType::from_build_owned_image_reader(&mut reader) }.unwrap();
            reader.end().unwrap();
            assert!(resolved.has_same_normalized_evidence(&loaded));
            let mut changed = serde_json::to_value(&resolved).unwrap();
            let bits = changed["evidence"][0][1].as_u64().unwrap();
            changed["evidence"][0][1] = (bits ^ 1).into();
            // SAFETY: isolated negative comparison; no authority escapes.
            let loaded = unsafe { ResolvedType::from_build_owned_image_reader(&changed) }.unwrap();
            assert_eq!(resolved, loaded);
            assert!(!resolved.has_same_normalized_evidence(&loaded));
        }
    }

    #[test]
    fn frozen_resolved_type_wire_rejects_missing_extra_and_unknown_predicates() {
        let resolved = ResolvedType::new(BuiltinScalarKind::U64.kind_expr(), Box::new([])).unwrap();
        for fault in 0..5 {
            let mut image = serde_json::to_value(&resolved).unwrap();
            match fault {
                0 => {
                    image.as_object_mut().unwrap().remove("dimensions");
                }
                1 => {
                    image.as_object_mut().unwrap().remove("evidence");
                }
                2 => image["evidence"][0][1] = 65535.into(),
                3 => image["extra"] = true.into(),
                _ => image["kind"] = "unrecognized kind".into(),
            }
            // SAFETY: negative grammar test; no returned type is retained.
            assert!(unsafe { ResolvedType::from_build_owned_image_reader(&image) }.is_err());
        }
    }
}
