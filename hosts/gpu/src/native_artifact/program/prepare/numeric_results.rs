//! Exact private return recipes. Field lanes remain owned by BufferFrames.
use super::*;
use mech_engine::native_artifact::NativeBufferFrameField;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum ResultRecipe {
    Scalar,
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
    carrier: &[NativeBufferFrameField],
    arrays: &[Array],
    provenance: &mut Provenance<'_, '_>,
) -> Result<ResultRecipe, &'static str> {
    let value = tail.returned();
    let ty = &plan.values()[value.get() as usize];
    if ty.scalar_kind().is_some() {
        return Ok(ResultRecipe::Scalar);
    }
    let mut root = |ordinal: Option<usize>, schema| {
        let field = provenance
            .array(value.get(), ordinal)
            .ok_or("leaf result lacks a numeric carrier root")?;
        if schema != carrier[field].schema() {
            return Err("returned array requires a different schema binding");
        }
        arrays
            .iter()
            .position(|a| a.field == field)
            .ok_or("returned array outside carrier")
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
    // terminal freshness proof; nested composites remain unsupported here.
    let fields = frames
        .fields(value)
        .ok_or("numeric result frame escapes or lacks exact private fields")?;
    if fields
        .iter()
        .any(|f| matches!(f.lane(), Lane::Composite { .. }))
    {
        return Err("numeric result requires flat fields");
    }
    let managed = fields
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.lane(), Lane::Managed { .. }))
        .collect::<Vec<_>>();
    if managed.is_empty() {
        return Err("numeric result frame requires an array owner");
    }
    let arrays = managed
        .into_iter()
        .map(|(ordinal, field)| Ok((ordinal, root(Some(ordinal), field.schema())?)))
        .collect::<Result<Vec<_>, &'static str>>()?;
    Ok(ResultRecipe::Frame {
        value: value.get(),
        arrays,
    })
}
