//! Original row/sum/extraction costs shared by standalone and phase folds.
//! A bound grants neither a source visit nor ownership/publication authority.
use super::numeric_maps::Region;
use core::mem::size_of;
use mech_core::ValueDataDraft;

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Bound {
    pub temporary: u64,
    pub nodes: u64,
    pub output: u64,
    pub elements: u64,
    pub comparison: u64,
    pub setup: u64,
    pub replay: u64,
    pub commit: u64,
    pub cloned: u64,
}
pub(in crate::native_artifact) fn bound(
    calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
    schema_bytes: u64,
    region: &Region,
    count: u64,
) -> Option<Bound> {
    let Some(reduction) = &region.reduction else {
        return Some(Bound::default());
    };
    let sum = calls.get(reduction.sum as usize)?.as_ref()?;
    let select = calls.get(reduction.select as usize)?.as_ref()?;
    if !sum.supports_f64_row_sum() || !select.supports_f64_matrix_element() {
        return None;
    }
    let bytes = sum
        .binding_metadata_bytes()
        .checked_add(select.binding_metadata_bytes())?;
    let work = sum.binding_work().checked_add(select.binding_work())?;
    let cells = count.checked_add(4)?.checked_mul(
        (size_of::<ValueDataDraft>() as u64)
            .checked_mul(16)?
            .checked_add(512)?,
    )?;
    let temporary = bytes
        .checked_mul(128)?
        .checked_add(schema_bytes.checked_mul(32)?)?
        .checked_add(cells)?
        .checked_add(65536)?;
    let output = count
        .checked_add(4)?
        .checked_mul(size_of::<ValueDataDraft>() as u64)?
        .checked_add(bytes)?
        .checked_add(4096)?;
    let commit = work
        .checked_mul(16)?
        .checked_add(temporary.checked_mul(16)?)?
        .checked_add(8192)?;
    let replay = work
        .checked_mul(1024)?
        .checked_add(temporary.checked_mul(128)?)?
        .checked_add(65536)?;
    Some(Bound {
        temporary,
        nodes: count.checked_mul(4)?.checked_add(bytes)?.checked_add(256)?,
        output,
        // Even an empty producer has a 1x1 sum and scalar selection.
        elements: count.max(1),
        comparison: output,
        setup: bytes.checked_mul(128)?.checked_add(4096)?,
        replay,
        commit,
        cloned: cells.checked_add(bytes)?,
    })
}
