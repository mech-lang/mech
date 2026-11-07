//! Private decoding only. No source proofs are inferred from serialized fields.
use crate::native_artifact as native;
use crate::native_artifact::{NativeProgramDescriptor, NativeValueSlot};
use mech_core::{FunctionCatalog, SchemaId, SchemaKey};
use mech_engine::native_artifact::*;
use serde::{Deserialize, Deserializer};
#[cfg(test)]
use serde_json::Value as Json;

use native::program::prepare::readonly_numeric::{FrameWords, numeric_frame::CompiledFrame};
use std::sync::Arc;

// This trait is inaccessible outside the native implementation. In particular,
// the engine proof types still do NOT implement public Deserialize.
pub(in crate::native_artifact) trait ProofImage: Sized {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error>;
}
pub(in crate::native_artifact) fn proof<'de, T: ProofImage, D: Deserializer<'de>>(
    d: D,
) -> Result<T, D::Error> {
    T::read(d)
}
macro_rules! engine_proofs {
    ($($ty:ty),* $(,)?) => { $(impl ProofImage for $ty {
        fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
            // SAFETY: this private wire reader is reachable only through the
            // enclosing unsafe descriptor loader, which confines all results
            // until their unchanged checked Object provenance is established.
            unsafe { <$ty>::from_build_owned_image_reader(image) }
        }
    })* };
}
engine_proofs!(
    NativeMatchOrigin,
    NativeTailLoop,
    NativeWindowRegion,
    NativeStringMapRegion,
    NativeScalarAggregates,
    NativeMapOwnerWords,
    NativeBorrowedStringRegion,
    NativeSelectedTextJoin,
    NativeTextLineMatrixRegion,
    NativeTextLineRetentionFrontier,
);
// Only this private adapter implements Deserialize. Constructor-private engine
// proofs remain reachable exclusively through the confined unsafe image loader.
struct ReadProof<T>(T);
impl<'de, T: ProofImage> Deserialize<'de> for ReadProof<T> {
    fn deserialize<D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        T::read(image).map(Self)
    }
}
impl<T: ProofImage> ProofImage for Vec<T> {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        Vec::<ReadProof<T>>::deserialize(image)
            .map(|values| values.into_iter().map(|value| value.0).collect())
    }
}
impl<T: ProofImage> ProofImage for Box<[T]> {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        Vec::<T>::read(image).map(Vec::into_boxed_slice)
    }
}
impl<T: ProofImage> ProofImage for Option<T> {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        Option::<ReadProof<T>>::deserialize(image).map(|value| value.map(|value| value.0))
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Nested {
        #[serde(deserialize_with = "proof")]
        origins: Option<Box<[NativeMatchOrigin]>>,
    }

    #[test]
    fn nested_proof_readers_preserve_fields_and_reject_duplicate_wire_keys() {
        // Wire-only origins; none can reach an executable descriptor here.
        for text in [
            r#"{"origins":null}"#,
            r#"{"origins":[]}"#,
            r#"{"origins":[{"node":4294967295,"block":null,"local":3}]}"#,
        ] {
            let loaded: Nested = serde_json::from_str(text).unwrap();
            assert_eq!(
                serde_json::to_value(loaded.origins).unwrap(),
                serde_json::from_str::<Json>(text).unwrap()["origins"]
            );
        }
        for text in [
            r#"{"origins":[{"node":0,"node":1,"block":null,"local":null}]}"#,
            r#"{"origins":[{"node":4294967296,"block":null,"local":null}]}"#,
            r#"{"origins":[{"node":0,"block":null,"local":null,"extra":0}]}"#,
            r#"{"origins":[{"block":null,"local":null}]}"#,
            r#"{"origins":{}}"#,
        ] {
            assert!(serde_json::from_str::<Nested>(text).is_err(), "{text}");
        }
    }
}

pub(in crate::native_artifact::program::image::decode::wire) mod layouts;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Slot {
    schema: SchemaId,
    key: SchemaKey,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Visit {
    #[serde(deserialize_with = "proof")]
    origin: NativeMatchOrigin,
    successors: [Option<u32>; 2],
    iteration: Option<u32>,
    progress: native::iteration_abi::Progress,
    complete: bool,
    arithmetic: bool,
    reject: bool,
    integrity: bool,
    work: u64,
}
impl From<Visit> for native::iteration_abi::NativeIterationVisit {
    fn from(w: Visit) -> Self {
        Self {
            origin: w.origin,
            successors: w.successors,
            iteration: w.iteration,
            progress: w.progress,
            complete: w.complete,
            arithmetic: w.arithmetic,
            reject: w.reject,
            integrity: w.integrity,
            work: w.work,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor<'a, Frame> {
    #[serde(borrow)]
    plan: BuildOwnedImageRecord<'a>,
    #[serde(borrow)]
    schemas: BuildOwnedImageRecord<'a>,
    #[serde(borrow)]
    calls: Box<[Option<BuildOwnedImageRecord<'a>>]>,
    types: Box<[Slot]>,
    visits: Box<[Visit]>,
    retirements: native::program::prepare::retirement::RetirementTables,
    constant_bytes: Box<[u64]>,
    scalar_bytes: u64,
    observation_schema_bytes: u64,
    max_library_inputs: usize,
    #[serde(default)]
    word_collections: Box<[native::program::prepare::word_collections::Region]>,
    #[serde(deserialize_with = "proof")]
    window_regions: Box<[NativeWindowRegion]>,
    #[serde(deserialize_with = "proof")]
    string_map_regions: Box<[NativeStringMapRegion]>,
    #[serde(deserialize_with = "proof")]
    scalar_aggregates: NativeScalarAggregates,
    #[serde(borrow, default, deserialize_with = "present_raw")]
    buffer_frames: Option<BuildOwnedImageRecord<'a>>,
    #[serde(deserialize_with = "proof")]
    map_owner_words: Option<NativeMapOwnerWords>,
    borrowed_record_maps: native::program::prepare::borrowed_record_map::Layout,
    utf8_reads: native::program::prepare::utf8_reads::Layout,
    #[serde(default, deserialize_with = "present_numeric_maps")]
    numeric_maps: Option<native::program::prepare::numeric_maps::Layout>,
    frozen_maps: native::program::prepare::frozen_map::Layout,
    map_admission: native::program::prepare::map_admission::Layout,
    parallel: native::program::prepare::parallel::Layout,
    borrowed_strings: layouts::Borrowed,
    closed_tails: layouts::Tails<Frame>,
    selected_joins: layouts::SelectedJoins,
    line_matrices: layouts::LineMatrices,
    prefix: Option<layouts::Prefix>,
    stack: u64,
    code: u64,
    identity: [u8; 32],
    #[serde(borrow, default, deserialize_with = "present_raw")]
    static_layouts: Option<BuildOwnedImageRecord<'a>>,
    #[serde(borrow, default, deserialize_with = "present_raw")]
    schema_templates: Option<BuildOwnedImageRecord<'a>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image<'a, Frame> {
    format: String,
    version: u32,
    #[serde(borrow)]
    descriptor: Descriptor<'a, Frame>,
}

fn present_numeric_maps<'de, D: Deserializer<'de>>(
    reader: D,
) -> Result<Option<native::program::prepare::numeric_maps::Layout>, D::Error> {
    native::program::prepare::numeric_maps::Layout::deserialize(reader).map(Some)
}

fn present_raw<'de, D: Deserializer<'de>>(
    reader: D,
) -> Result<Option<BuildOwnedImageRecord<'de>>, D::Error> {
    <BuildOwnedImageRecord>::deserialize(reader).map(Some)
}

pub(in crate::native_artifact::program::image::decode) fn decode(
    bytes: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeProgramDescriptor, String> {
    decode_inner::<false, false, FrameWords>(bytes, catalog)
}

pub(in crate::native_artifact::program::image::decode) fn decode_verified(
    verified: &super::super::binding::VerifiedObjectImage<'_>,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramDescriptor, String> {
    decode_inner::<true, false, FrameWords>(verified.bytes(), catalog)
}

pub(in crate::native_artifact::program::image::decode) fn decode_compiled_verified(
    verified: &super::super::binding::VerifiedObjectImage<'static>,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramDescriptor, String> {
    decode_inner::<true, true, CompiledFrame>(verified.bytes(), catalog)
}

// Diagnostic-only synchronous observation. The executable supplies the hook;
// this checkout changes no validation, allocation or source/native operation.
unsafe extern "C" { fn mech_restore_phase_event(event:u64); }
fn restore_phase(id:u64,enter:bool) { unsafe { mech_restore_phase_event(if enter{id}else{id|(1u64<<63)}) }; }

fn decode_inner<
    'a,
    const OBJECT_BOUND: bool,
    const COMPILED: bool,
    Frame: Deserialize<'a> + Into<FrameWords>,
>(
    bytes: &'a [u8],
    catalog: &FunctionCatalog,
) -> Result<NativeProgramDescriptor, String> {
    restore_phase(10,true);
    let mut reader = BuildOwnedImageReader::from_image(bytes).map_err(|e| e.to_string())?;
    let image: Image<'a, Frame> = Image::deserialize(&mut reader).map_err(|e| e.to_string())?;
    reader.end().map_err(|e| e.to_string())?;
    restore_phase(10,false);
    if image.format != "mech.build-owned-native-descriptor"
        || (OBJECT_BOUND && !matches!(image.version, 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9))
        || (!OBJECT_BOUND && !matches!(image.version, 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9))
    {
        return Err("unsupported native descriptor image format".into());
    }
    let table_rows = image.version >= 3;
    let mut w = image.descriptor;
    if (image.version >= 2) != w.buffer_frames.is_some() {
        return Err("native descriptor version differs from its frame inventory".into());
    }
    if (image.version >= 4) != w.static_layouts.is_some() {
        return Err("native descriptor version differs from its static layout inventory".into());
    }
    if (image.version >= 8) != w.schema_templates.is_some() {
        return Err("native descriptor version differs from its type-template inventory".into());
    }
    // This is selected only by the private verified-byte borrow, never by an
    // untrusted field. V1 remains available through independent reconstruction;
    // new checked Objects bind V2 and have a distinct typed-IR identity.
    let frozen_layouts = OBJECT_BOUND;
    restore_phase(20,true);
    let schemas = Arc::new(decode_native_schema_record(w.schemas)?);
    restore_phase(20,false);
    // SAFETY: only the enclosing unsafe descriptor loader reaches this private
    // reader; it retains all metadata until exact checked-build provenance holds.
    restore_phase(30,true);
    let plan = if image.version >= 5 {
        // SAFETY: the explicit image version selects the shared table reader;
        // provenance and independent validation retain the same policy.
        unsafe { decode_build_owned_native_shared_table_plan_record(w.plan, &schemas, catalog)? }
    } else if table_rows {
        // SAFETY: same confined checked image; V3 changes wire representation,
        // never the Object-bound/independent proof-selection policy below.
        unsafe { decode_build_owned_native_table_plan_record(w.plan, &schemas, catalog)? }
    } else {
        unsafe { decode_build_owned_native_plan_record(w.plan, &schemas, catalog)? }
    };
    restore_phase(30,false);
    restore_phase(40,true);
    let mut descriptor_templates = match w.schema_templates {
        Some(record) if COMPILED => {
            // SAFETY: only decode_compiled_verified selects this monomorph;
            // its private witness binds these process-lifetime bytes to the
            // genuine checked compiled module. No image field selects it.
            unsafe {
                NativeDescriptorTemplatePool::from_build_owned_schema_templates(record, &schemas)?
            }
        }
        Some(record) => {
            NativeDescriptorTemplatePool::from_independent_schema_templates(record, &schemas)?
        }
        None => NativeDescriptorTemplatePool::default(),
    };
    restore_phase(40,false);
    restore_phase(50,true);
    let mut calls = w
        .calls
        .iter()
        .enumerate()
        .map(|(site, image)| {
            image
                .as_ref()
                .map(|image| {
                    if table_rows {
                        let template = plan.prepared_library_template(site)
                            .map_err(str::to_owned)?
                            .ok_or("table binding has no selected plan template")?;
                        // SAFETY: this exact source-site template was restored
                        // against these same schemas and actual catalog. The
                        // independent path still rederives every source layout.
                        return unsafe {
                            NativePreparedLibraryCall::from_build_owned_plan_binding_record_with_descriptors(
                                *image, template.clone(), schemas.clone(), catalog,
                                &mut descriptor_templates,
                            )
                        };
                    }
                    if OBJECT_BOUND {
                        // Only the private verified-byte path reaches this
                        // branch. Compilation and restoration share the exact
                        // source-site association, not a wire-supplied selector.
                        let template = plan
                            .prepared_library_template(site)
                            .map_err(str::to_owned)?
                            .ok_or("bound prepared call has no selected plan template")?;
                        // SAFETY: the complete image has genuine same-Object
                        // provenance; this is the exact plan site's selection,
                        // restored against these same schemas and catalog.
                        return unsafe {
                            NativePreparedLibraryCall::from_object_bound_binding_record_with_descriptors(
                                *image,
                                template.clone(),
                                schemas.clone(),
                                catalog,
                                &mut descriptor_templates,
                            )
                        };
                    }
                    // Do not retain every call's expanded JSON while binding
                    // the others: these bytes borrow the immutable build image.
                    // SAFETY: same confined build image; factory witnesses are checked too.
                    unsafe {
                        NativePreparedLibraryCall::from_build_owned_record_with_descriptors(
                            *image,
                            schemas.clone(),
                            catalog,
                            &mut descriptor_templates,
                        )
                    }
                })
                .transpose()
        })
        .collect::<Result<Box<[_]>, _>>()?;
    let types = w
        .types
        .into_vec()
        .into_iter()
        .map(|s| NativeValueSlot {
            schema: s.schema,
            key: s.key,
        })
        .collect::<Box<[_]>>();
    restore_phase(50,false);
    restore_phase(60,true);
    if w.stack == 0
        || w.code == 0
        || w.visits.len() != plan.blocks().len()
        || calls.len() != plan.blocks().len()
        || types.len() != plan.values().len() + plan.transport_output_count()
        || w.constant_bytes.len() != plan.constants().len()
    {
        return Err("descriptor image inventory differs from its plan".into());
    }
    native::program::prepare::validate_type_slots(&plan, &schemas, &types)
        .map_err(str::to_owned)?;
    if w.constant_bytes
        != native::program::prepare::constant_owners::admission_bounds(&plan.facts())?
    {
        return Err("constant ownership bounds differ from canonical runtime reads".into());
    }
    restore_phase(60,false);
    restore_phase(70,true);
    let closed_tails: native::program::prepare::tail::Layout = w.closed_tails.into();
    let image_frames = w
        .buffer_frames
        .take()
        .map(|image| {
            // SAFETY: the enclosing loader confines all recipes until unchanged
            // checked-build provenance holds. Independent loading compares these
            // records with fresh source derivation before use.
            let mut reader = image.reader();
            let frames = unsafe { NativeBufferFrames::from_build_owned_image_reader(&mut reader) }
                .map_err(|e| e.to_string())?;
            reader.end().map_err(|e| e.to_string())?;
            Ok::<_, String>(frames)
        })
        .transpose()?;
    let (buffer_frames, computed_map_owner_words) = if frozen_layouts {
        (image_frames.unwrap(), None)
    } else {
        let (frames, owners) = plan
            .facts()
            .buffer_frame_owner_layouts()
            .map_err(str::to_owned)?;
        if let Some(image_frames) = image_frames
            && serde_json::to_value(&frames).map_err(|e| e.to_string())?
                != serde_json::to_value(&image_frames).map_err(|e| e.to_string())?
        {
            return Err(
                "native buffer frames differ from independently derived source uses".into(),
            );
        }
        (frames, Some(owners))
    };
    native::program::prepare::prepare_frame_composition(&plan, &buffer_frames, &mut calls)?;
    if !OBJECT_BOUND {
        native::program::prepare::tail::validate_image(
            &plan,
            &buffer_frames,
            computed_map_owner_words.as_ref().unwrap(),
            &w.scalar_aggregates,
            &calls,
            &closed_tails,
        )?;
    }
    restore_phase(70,false);
    restore_phase(80,true);
    let borrowed_strings: native::program::prepare::borrowed::Layout = w.borrowed_strings.into();
    let map_owner_words = if frozen_layouts {
        w.map_owner_words.take()
    } else {
        let occupied = native::program::prepare::frame_prefix_words(
            &plan,
            &w.scalar_aggregates,
            &borrowed_strings,
            &closed_tails,
        )
        .ok_or("restored map owner frame prefix overflows")?;
        let map_owner_words = computed_map_owner_words
            .unwrap()
            .after_frame_words(occupied)
            .map_err(str::to_owned)?;
        let map_owner_words = (!map_owner_words.regions().is_empty()).then_some(map_owner_words);
        if serde_json::to_vec(&map_owner_words).map_err(|e| e.to_string())?
            != serde_json::to_vec(&w.map_owner_words).map_err(|e| e.to_string())?
        {
            return Err(
                "retained-map owner lanes differ from independently validated source uses".into(),
            );
        }
        map_owner_words
    };
    let borrowed_record_maps = if frozen_layouts {
        w.borrowed_record_maps
    } else {
        let derived = map_owner_words
            .as_ref()
            .map(|owners| {
                native::program::prepare::borrowed_record_map::Layout::derive(&plan, owners)
            })
            .transpose()
            .map_err(str::to_owned)?
            .unwrap_or_default();
        if derived != w.borrowed_record_maps {
            return Err(
                "borrowed record-map proof differs from exact canonical publication uses".into(),
            );
        }
        derived
    };
    let utf8_reads = if frozen_layouts {
        w.utf8_reads
    } else {
        let derived =
            native::program::prepare::utf8_reads::Layout::derive(&plan).map_err(str::to_owned)?;
        if derived != w.utf8_reads {
            return Err(
                "native UTF8 lookup proof differs from exact source collection uses".into(),
            );
        }
        derived
    };
    if (image.version >= 7) != w.numeric_maps.is_some() {
        return Err("native descriptor version differs from numeric map inventory".into());
    }
    let numeric_maps = match w.numeric_maps {
        Some(layout) if OBJECT_BOUND => layout,
        image => {
            let derived = native::program::prepare::numeric_maps::Layout::derive(&plan)
                .map_err(str::to_owned)?;
            if image.is_some_and(|image| image != derived) {
                return Err("numeric map proof differs from independent source derivation".into());
            }
            derived
        }
    };
    let visits: Box<[native::iteration_abi::NativeIterationVisit]> = w
        .visits
        .into_vec()
        .into_iter()
        .map(Into::into)
        .collect::<Box<[_]>>();
    let mut static_layouts = w
        .static_layouts
        .take()
        .map(|image| {
            let mut reader = image.reader();
            let layouts = super::super::static_layouts::Owned::deserialize(&mut reader)
                .map_err(|error| error.to_string())?;
            reader.end().map_err(|error| error.to_string())?;
            Ok::<_, String>(layouts)
        })
        .transpose()?;
    let image_uniform_collections = static_layouts
        .as_mut()
        .and_then(|layouts| layouts.uniform_word_collections.take());
    if (image.version >= 6) != image_uniform_collections.is_some() {
        return Err("native descriptor version differs from uniform collection inventory".into());
    }
    let uniform_word_collections = match image_uniform_collections {
        Some(regions) if OBJECT_BOUND => regions,
        image => {
            let derived = plan
                .facts()
                .uniform_unsigned_collections()
                .into_boxed_slice();
            if image.is_some_and(|image| image != derived) {
                return Err(
                    "uniform native collections differ from independent source derivation".into(),
                );
            }
            derived
        }
    };
    let base_visits = native::iteration_abi::control_visits(
        plan.blocks(),
        plan.iterations().iter().map(Into::into),
        0,
    )
    .map_err(str::to_owned)?;
    for region in &utf8_reads.regions {
        let site = region.header as usize;
        let required = base_visits
            .get(site)
            .and_then(|visit| {
                visit
                    .work
                    .checked_add(native::program::prepare::utf8_reads::HEADER_WORK)
            })
            .ok_or("native UTF8 lookup header price overflows")?;
        if visits.get(site).is_none_or(|visit| visit.work < required) {
            return Err("native UTF8 lookup header template is not funded".into());
        }
    }
    for region in &numeric_maps.regions {
        let required = base_visits
            .get(region.header as usize)
            .and_then(|visit| {
                visit
                    .work
                    .checked_add(native::program::prepare::numeric_maps::HEADER_WORK)
            })
            .ok_or("numeric map header price overflows")?;
        if visits
            .get(region.header as usize)
            .is_none_or(|visit| visit.work < required)
        {
            return Err("numeric map header template is not funded".into());
        }
    }
    let word_collections =
        native::program::prepare::word_collections::derive(&plan, &visits, &w.map_admission)
            .into_boxed_slice();
    if word_collections != w.word_collections
        || (!word_collections.is_empty() && w.max_library_inputs < 3)
    {
        return Err("word collection proof differs from its total scalar source CFG".into());
    }
    for region in &uniform_word_collections {
        let site = region.header() as usize;
        let required = base_visits
            .get(site)
            .ok_or("native fill header outside source visits")?
            .work;
        if visits.get(site).is_none_or(|visit| visit.work < required) {
            return Err("native fill header template is not funded".into());
        }
    }
    if !frozen_layouts {
        let admission = native::program::prepare::map_admission::Layout::derive(
            &plan,
            &w.scalar_aggregates,
            map_owner_words.as_ref(),
            &visits,
            &closed_tails,
        )
        .map_err(str::to_owned)?;
        if admission != w.map_admission {
            return Err("native path admission differs from its source frontiers".into());
        }
    }
    let frozen_maps = if frozen_layouts {
        w.frozen_maps
    } else {
        let derived = map_owner_words
            .as_ref()
            .map(|owner| {
                native::program::prepare::frozen_map::Layout::derive(&plan, owner, &closed_tails)
            })
            .transpose()
            .map_err(str::to_owned)?
            .unwrap_or_default();
        if derived != w.frozen_maps {
            return Err("frozen-map descriptor proof differs from its complete source uses".into());
        }
        derived
    };
    if !frozen_layouts {
        let selected_joins = native::program::prepare::selected_join::Layout::derive(
            &plan,
            &plan.string_map_regions(),
        )?;
        if selected_joins.maps != w.selected_joins.maps {
            return Err("selected-join map proof differs from its complete source uses".into());
        }
    }

    let (
        unsigned_buffer_calls,
        call_routes,
        frame_retirements,
        cleanup_effects,
        call_cache,
        compositions,
    ) = match static_layouts {
        Some(image) if OBJECT_BOUND => {
            // Only the private VerifiedObjectImage path can select this branch.
            // These are the exact immutable compiler results bound to the same
            // native code, not a version-controlled grant of proof authority.
            (
                image.unsigned_buffer_calls,
                image.call_routes,
                image.frame_retirements,
                image.cleanup_effects,
                image.call_cache,
                image.compositions.into(),
            )
        }
        image => {
            let unsigned_buffer_calls = plan.unsigned_buffer_calls().into_boxed_slice();
            let call_routes = native::program::prepare::call_routes::derive(
                &plan,
                &buffer_frames,
                &unsigned_buffer_calls,
                &calls,
            );
            let frame_retirements =
                native::program::prepare::frame_retirement::FrameRetirements::derive(
                    &buffer_frames,
                    plan.values().len(),
                )?;
            let cleanup_effects = native::program::prepare::cleanup_effects::Layout::derive(
                &plan,
                &w.retirements,
                &buffer_frames,
                &frame_retirements,
            )?;
            let call_cache = native::program::prepare::call_cache::Layout::derive(&plan, &calls)?;
            let compositions = native::program::prepare::composition::Layout::derive(
                &plan,
                &buffer_frames,
                &w.scalar_aggregates,
                &calls,
                &closed_tails,
                &visits,
                &frame_retirements,
                &w.map_admission,
            )?;
            if let Some(image) = image {
                let image_compositions: native::program::prepare::composition::Layout =
                    image.compositions.into();
                if image.unsigned_buffer_calls != unsigned_buffer_calls
                    || image.call_routes != call_routes
                    || image.frame_retirements != frame_retirements
                    || image.cleanup_effects != cleanup_effects
                    || image.call_cache != call_cache
                    || image_compositions != compositions
                {
                    return Err(
                        "static native layouts differ from independent source derivation".into(),
                    );
                }
            }
            (
                unsigned_buffer_calls,
                call_routes,
                frame_retirements,
                cleanup_effects,
                call_cache,
                compositions,
            )
        }
    };
    if w.max_library_inputs < compositions.argument_words() {
        return Err("restored parent native argument bank is too small".into());
    }
    // The managed prefix is not serialized permission. Reconstruct it from
    // source calls, checked parent pieces and source-derived window candidates.
    let max_call_values = native::program::prepare::call_value_bound(
        &plan,
        &compositions,
        !plan.window_regions().is_empty() || !plan.string_map_regions().is_empty(),
    );
    if max_call_values > w.max_library_inputs {
        return Err("restored canonical arguments exceed admitted backing".into());
    }
    restore_phase(80,false);
    restore_phase(90,true);
    let descriptor = NativeProgramDescriptor {
        metadata: native::program::prepare::ProgramMetadata {
            plan,
            schemas,
            calls,
            call_cache,
            compositions,
            call_routes,
            types,
            visits,
            retirements: w.retirements,
            constant_admission_bytes:
                crate::native_artifact::program::prepare::constant_admission_total(
                    &w.constant_bytes,
                )?,
            constant_bytes: w.constant_bytes,
            scalar_bytes: w.scalar_bytes,
            observation_schema_bytes: w.observation_schema_bytes,
            max_library_inputs: w.max_library_inputs,
            max_call_values,
            uniform_word_collections,
            word_collections,
            unsigned_buffer_calls,
            buffer_frames,
            frame_retirements,
            cleanup_effects,
            window_regions: w.window_regions,
            string_map_regions: w.string_map_regions,
            scalar_aggregates: w.scalar_aggregates,
            map_owner_words,
            borrowed_record_maps,
            utf8_reads,
            numeric_maps,
            frozen_maps,
            map_admission: w.map_admission,
            parallel: w.parallel,
            borrowed_strings,
            closed_tails,
            selected_joins: w.selected_joins.into(),
            line_matrices: w.line_matrices.into(),
            prefix: w.prefix.map(Into::into),
        },
        footprint: native::program::image::NativeCodeFootprint {
            stack_bytes: w.stack,
            code_bytes: w.code,
        },
        identity: w.identity,
    };
    restore_phase(90,false);
    // Wire shape, canonical schemas and actual factories were checked above.
    // Full round-trip fidelity is tested at build time; re-encoding here would
    // retain compiler-side serializers without proving source/Object provenance.
    Ok(descriptor)
}
