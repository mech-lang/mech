//! Restore build-time binding bounds; never prepare or admit a runtime call.
use super::NativePreparedLibraryCall;
use mech_core::{FunctionCatalog, SchemaKey, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    template: Json,
    binding_bytes: u64,
    binding_work: u64,
    binding_nodes: u64,
    output_parameters: usize,
    input_keys: Box<[SchemaKey]>,
    output_key: SchemaKey,
    lookup_work: u64,
    output_follows_input: bool,
    span_selector_metadata_bytes: u64,
    span_selector_work: u64,
    #[cfg(feature = "pure_text")]
    selection_shape_bounds: (u64, u64),
    #[cfg(feature = "pure_text")]
    selection_range_shape_bounds: (u64, u64),
}

impl NativePreparedLibraryCall {
    /// Restore this build's frozen selection, with a fresh invocation-cache
    /// identity and the actual linked factory. No binding or execution occurs.
    ///
    /// # Safety
    /// Keep the result confined until the enclosing descriptor loader establishes
    /// that the image, schema table and linked Object are unchanged products of
    /// the SAME checked compiler build. The image's budget bounds and source-site
    /// proofs are compiler output, not independently proved here. A supplied hash
    /// is not authentication. Normal runtime admission and current-site checks
    /// remain mandatory before use; this constructor grants no visit permission.
    pub unsafe fn from_build_owned_image(
        image: &Json,
        schemas: Arc<SchemaTable>,
        catalog: &FunctionCatalog,
    ) -> Result<Self, String> {
        let wire = Call::deserialize(image).map_err(|e| e.to_string())?;
        // SAFETY: the caller inherits the enclosing unchanged-build confinement
        // obligation; this helper also checks the actual maintained factory.
        let template = unsafe {
            crate::native_artifact::program_image_read::library::template(
                &wire.template,
                catalog,
                &schemas,
            )?
        };
        if wire.output_parameters != template.output().dimension_parameters().len()
            || wire.input_keys.len() != template.inputs().len()
            || wire
                .input_keys
                .iter()
                .zip(template.inputs())
                .any(|(key, schema)| *key != schema.key())
            || wire.output_key != template.output().key()
        {
            return Err("prepared image schema keys or parameter count differ".into());
        }
        let key = template.selected_key();
        let factory = catalog
            .resident_factory(&key.module_path, &key.operation_name)
            .ok_or("prepared image factory is not linked")?
            .factory();
        #[cfg(feature = "sequence_buffers")]
        let frame_geometry =
            super::buffer_frames::FrameGeometry::derive(&template, &schemas, wire.binding_bytes);
        #[cfg(feature = "sequence_buffers")]
        let record_selector = super::record_selector::RecordSelector::derive(&template);
        let descriptor_templates = super::descriptor_templates(&template);
        let call = Self {
            template,
            schemas,
            factory,
            descriptor_templates,
            binding_bytes: wire.binding_bytes,
            binding_work: wire.binding_work,
            binding_nodes: wire.binding_nodes,
            output_parameters: wire.output_parameters,
            input_keys: wire.input_keys,
            output_key: wire.output_key,
            lookup_work: wire.lookup_work,
            output_follows_input: wire.output_follows_input,
            cache_identity: Arc::new(()),
            #[cfg(feature = "sequence_buffers")]
            frame_geometry,
            #[cfg(feature = "sequence_buffers")]
            record_selector,
            #[cfg(feature = "sequence_buffers")]
            frame_composition: None,
            span_selector_metadata_bytes: wire.span_selector_metadata_bytes,
            span_selector_work: wire.span_selector_work,
            #[cfg(feature = "pure_text")]
            selection_shape_bounds: wire.selection_shape_bounds,
            #[cfg(feature = "pure_text")]
            selection_range_shape_bounds: wire.selection_range_shape_bounds,
        };
        Ok(call)
    }
}
