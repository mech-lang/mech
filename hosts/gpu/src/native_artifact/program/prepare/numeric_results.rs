//! Exact private return recipes in their existing scalar/owner word banks.
use super::*;
use mech_engine::native_artifact::{
    NativeBufferFrameField, NativeMatchValue, NativeScalarAggregates,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum ResultRecipe {
    Scalar,
    Aggregate {
        value: u32,
    },
    Array {
        array: usize,
    },
    Frame {
        value: u32,
        /// Each returned managed field and its exact carrier-array root.
        /// Repeated roots are valid aliases, but each field must be current.
        arrays: Vec<(usize, usize)>,
    },
}

/// Reuse the connected private-use proof and its exact scalar word count for
/// flat scalar aggregates. Nested scalar spans use the separate buffer-frame
/// recipe; the ordinary shared aggregate emitter still owns bit semantics here.
pub(in crate::native_artifact::program::prepare) fn scalar_aggregate(
    plan: &Plan,
    aggregates: &NativeScalarAggregates,
    value: NativeMatchValue,
) -> bool {
    let ty = &plan.values()[value.get() as usize];
    if !ty.definition().dimension_parameters().is_empty() {
        return false;
    }
    let count = match ty.definition().body() {
        SchemaBody::Tuple(fields)
            if fields
                .iter()
                .all(|s| NativeScalarKind::from_schema_body(s).is_some()) =>
        {
            fields.len()
        }
        SchemaBody::Record(fields)
            if fields
                .iter()
                .all(|f| NativeScalarKind::from_schema_body(&f.schema).is_some()) =>
        {
            fields.len()
        }
        _ => return false,
    };
    aggregates
        .slot(value)
        .is_some_and(|(_, words)| words == count)
}
impl ResultRecipe {
    pub fn direct_array(&self) -> Option<usize> {
        match self {
            Self::Array { array } => Some(*array),
            _ => None,
        }
    }
    pub fn frame(&self) -> Option<u32> {
        match self {
            Self::Frame { value, .. } => Some(*value),
            _ => None,
        }
    }
}

pub(super) fn derive(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    _carrier: &[NativeBufferFrameField],
    arrays: &[Array],
    provenance: &mut Provenance<'_, '_>,
) -> Result<ResultRecipe, &'static str> {
    let value = tail.returned();
    let ty = &plan.values()[value.get() as usize];
    if ty.scalar_kind().is_some() {
        return Ok(ResultRecipe::Scalar);
    }
    if scalar_aggregate(plan, aggregates, value) {
        return Ok(ResultRecipe::Aggregate { value: value.get() });
    }
    let mut root = |ordinal: Option<usize>, schema| {
        let field = provenance
            .array(value.get(), ordinal)
            .ok_or("leaf result lacks a numeric carrier root")?;
        let at = arrays
            .iter()
            .position(|a| a.root == field)
            .ok_or("returned array outside proved roots")?;
        if schema != arrays[at].schema {
            return Err("returned array requires a different schema binding");
        }
        Ok(at)
    };
    if matches!(ty.definition().body(), SchemaBody::Matrix { .. }) {
        return Ok(ResultRecipe::Array {
            array: root(None, ty.schema())?,
        });
    }
    if !matches!(
        ty.definition().body(),
        SchemaBody::Tuple(_) | SchemaBody::Record(_)
    ) {
        return Err("numeric result is not a scalar, array, or private flat frame");
    }
    // The connected private-use proof supplies the exact tuple/record field
    // order and rejects public escape. Both use the same per-field roots and
    // terminal freshness proof. Nested spans must contain scalar leaves only.
    let fields = numeric_fields::at(frames, value.get())
        .ok_or("numeric result frame escapes or lacks exact untagged leaves")?;
    let managed = fields
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.lane(), Lane::Managed { .. }))
        .collect::<Vec<_>>();
    // A lifted immutable array may leave only nested scalar spans in the
    // result. The exact private-frame recipe also represents that empty set
    // of owner transfers; scalar lanes still use their ordinary exit stores.
    let arrays = managed
        .into_iter()
        .map(|(ordinal, field)| Ok((ordinal, root(Some(ordinal), field.schema())?)))
        .collect::<Result<Vec<_>, &'static str>>()?;
    Ok(ResultRecipe::Frame {
        value: value.get(),
        arrays,
    })
}
