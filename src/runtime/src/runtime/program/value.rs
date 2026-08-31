use mech_core::{GenericError, MResult};
use mech_engine::resident::{ReactiveInstance, ResidentValueBorrow};

use crate::RuntimeValueSnapshot;

pub(crate) fn initial_value(
    instance: &ReactiveInstance,
    output_index: Option<usize>,
) -> MResult<RuntimeValueSnapshot> {
    let Some(output_index) = output_index else {
        return Ok(RuntimeValueSnapshot::empty());
    };
    output_value(instance, output_index)
        .map(|value| value.unwrap_or_else(RuntimeValueSnapshot::empty))
}

pub(crate) fn output_value(
    instance: &ReactiveInstance,
    output_index: usize,
) -> MResult<Option<RuntimeValueSnapshot>> {
    let Some(output) = instance.output_borrow(output_index) else {
        return Ok(None);
    };
    if matches!(output, ResidentValueBorrow::Snapshot { values: [None], .. }) {
        return Ok(None);
    }
    let value = instance.copied_output(output_index).map_err(|error| {
        mech_core::MechError::new(
            GenericError {
                msg: format!("resident output snapshot failed: {error:?}"),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    RuntimeValueSnapshot::from_value(value).map(Some)
}
