//! Restore build-time binding bounds; never prepare or admit a runtime call.
use super::{NativePreparedLibraryCall, NativeProgramLibraryTemplate};
use mech_core::{FunctionCatalog, SchemaKey, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call<T> {
    template: T,
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
    /// # Safety
    /// All unchanged checked-build, confinement and runtime admission duties of
    /// `from_build_owned_image` apply. This path retains independent selection
    /// and actual factory checks without materializing an enclosing JSON tree.
    pub unsafe fn from_build_owned_record(
        image: crate::native_artifact::BuildOwnedImageRecord<'_>,
        schemas: Arc<SchemaTable>,
        catalog: &FunctionCatalog,
    ) -> Result<Self, String> {
        let wire: Call<crate::native_artifact::BuildOwnedImageRecord<'_>> =
            image.decode().map_err(|e| e.to_string())?;
        let template = unsafe {
            crate::native_artifact::program_image_read::library::template_record(
                wire.template,
                catalog,
                &schemas,
            )?
        };
        Self::restore_binding(wire, template, schemas, catalog)
    }
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
        let wire = Call::<Json>::deserialize(image).map_err(|e| e.to_string())?;
        // SAFETY: the caller inherits the enclosing unchanged-build confinement
        // obligation; this helper also checks the actual maintained factory.
        let template = unsafe {
            crate::native_artifact::program_image_read::library::template(
                &wire.template,
                catalog,
                &schemas,
            )?
        };
        Self::restore_binding(wire, template, schemas, catalog)
    }

    /// Reuse the exact plan selection from the same verified Object image.
    /// The redundant serialized template is borrowed, never expanded or used
    /// as an authority. Binding bounds and fresh invocation caches are restored
    /// normally; actual linked factory and schema-key checks still run.
    ///
    /// # Safety
    /// In addition to `from_build_owned_image`'s confinement obligations, the
    /// caller must establish genuine checked-Object provenance for every image
    /// byte and obtain `template` from this exact plan/source site's selected
    /// call, already restored against the SAME schemas and catalog. A supplied
    /// digest or arbitrary caller template does not establish that association.
    pub unsafe fn from_object_bound_binding_image(
        bytes: &[u8],
        template: NativeProgramLibraryTemplate,
        schemas: Arc<SchemaTable>,
        catalog: &FunctionCatalog,
    ) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        // SAFETY: identical unchanged-build and exact source-site association.
        unsafe {
            Self::from_object_bound_binding_record(
                crate::native_artifact::BuildOwnedImageRecord::Json(text),
                template,
                schemas,
                catalog,
            )
        }
    }

    /// Bind the same compiler-selected template to borrowed typed Object data.
    ///
    /// # Safety
    /// All genuine checked-Object provenance, exact source-site association,
    /// confinement and runtime admission duties of
    /// `from_object_bound_binding_image` apply. A record is not authority.
    pub unsafe fn from_object_bound_binding_record(
        image: crate::native_artifact::BuildOwnedImageRecord<'_>,
        template: NativeProgramLibraryTemplate,
        schemas: Arc<SchemaTable>,
        catalog: &FunctionCatalog,
    ) -> Result<Self, String> {
        let wire: Call<crate::native_artifact::BuildOwnedImageRecord<'_>> =
            image.decode().map_err(|e| e.to_string())?;
        Self::restore_binding(wire, template, schemas, catalog)
    }

    fn restore_binding<T>(
        wire: Call<T>,
        template: NativeProgramLibraryTemplate,
        schemas: Arc<SchemaTable>,
        catalog: &FunctionCatalog,
    ) -> Result<Self, String> {
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
        let constructor_normalization =
            super::map_owner_constructors::normalization(&template, &schemas);
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
            constructor_normalization,
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
