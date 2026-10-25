//! Private decoding only. No source proofs are inferred from serialized fields.
use crate::native_artifact::{self as native, NativeProgramDescriptor, NativeValueSlot};
use mech_core::{FunctionCatalog, SchemaId, SchemaKey};
use mech_engine::native_artifact::*;
use serde::{Deserialize, Deserializer};
use serde_json::{Value as Json, value::RawValue};
use std::sync::Arc;

// This trait is inaccessible outside the native implementation. In particular,
// the engine proof types still do NOT implement public Deserialize.
pub(in crate::native_artifact) trait ProofImage: Sized {
    fn read(image: Json) -> Result<Self, String>;
}
pub(in crate::native_artifact) fn proof<'de, T: ProofImage, D: Deserializer<'de>>(
    d: D,
) -> Result<T, D::Error> {
    T::read(Json::deserialize(d)?).map_err(serde::de::Error::custom)
}
macro_rules! engine_proofs {
    ($($ty:ty),* $(,)?) => { $(impl ProofImage for $ty {
        fn read(image: Json) -> Result<Self, String> {
            // SAFETY: this private wire reader is reachable only through the
            // enclosing unsafe descriptor loader, which confines all results
            // until their unchanged checked Object provenance is established.
            unsafe { <$ty>::from_build_owned_image(&image) }
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
impl<T: ProofImage> ProofImage for Vec<T> {
    fn read(image: Json) -> Result<Self, String> {
        let Json::Array(values) = image else {
            return Err("proof image expected an array".into());
        };
        values.into_iter().map(T::read).collect()
    }
}
impl<T: ProofImage> ProofImage for Box<[T]> {
    fn read(image: Json) -> Result<Self, String> {
        Vec::<T>::read(image).map(Vec::into_boxed_slice)
    }
}
impl<T: ProofImage> ProofImage for Option<T> {
    fn read(image: Json) -> Result<Self, String> {
        if image.is_null() {
            Ok(None)
        } else {
            T::read(image).map(Some)
        }
    }
}

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
            work: w.work,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor<'a> {
    #[serde(borrow)]
    plan: &'a RawValue,
    schemas: Json,
    #[serde(borrow)]
    calls: Box<[Option<&'a RawValue>]>,
    types: Box<[Slot]>,
    visits: Box<[Visit]>,
    retirements: native::program_descriptor::RetirementTables,
    constant_bytes: Box<[u64]>,
    scalar_bytes: u64,
    observation_schema_bytes: u64,
    max_library_inputs: usize,
    #[serde(deserialize_with = "proof")]
    window_regions: Box<[NativeWindowRegion]>,
    #[serde(deserialize_with = "proof")]
    string_map_regions: Box<[NativeStringMapRegion]>,
    #[serde(deserialize_with = "proof")]
    scalar_aggregates: NativeScalarAggregates,
    #[serde(deserialize_with = "proof")]
    map_owner_words: Option<NativeMapOwnerWords>,
    frozen_maps: native::program_frozen_map_layout::Layout,
    map_admission: native::program_map_admission::Layout,
    parallel: native::program_parallel_layout::Layout,
    borrowed_strings: native::program_borrowed_layout::Layout,
    closed_tails: native::program_tail_layout::Layout,
    selected_joins: native::program_selected_join_layout::Layout,
    line_matrices: native::program_line_matrix_layout::Layout,
    prefix: Option<native::program_prefix_layout::Region>,
    stack: u64,
    code: u64,
    identity: [u8; 32],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image<'a> {
    format: String,
    version: u32,
    #[serde(borrow)]
    descriptor: Descriptor<'a>,
}

pub(super) fn decode(
    bytes: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeProgramDescriptor, String> {
    let image: Image<'_> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if image.format != "mech.build-owned-native-descriptor" || image.version != 1 {
        return Err("unsupported native descriptor image format".into());
    }
    let w = image.descriptor;
    let schemas = Arc::new(decode_native_schema_image(&w.schemas)?);
    // SAFETY: only the enclosing unsafe descriptor loader reaches this private
    // reader; it retains all metadata until exact checked-build provenance holds.
    let plan =
        unsafe { decode_build_owned_native_plan_body(w.plan.get().as_bytes(), &schemas, catalog)? };
    let calls = w
        .calls
        .iter()
        .map(|image| {
            image
                .as_ref()
                .map(|image| {
                    // Do not retain every call's expanded JSON while binding
                    // the others: these bytes borrow the immutable build image.
                    let image: Json =
                        serde_json::from_str(image.get()).map_err(|e| e.to_string())?;
                    // SAFETY: same confined build image; factory witnesses are checked too.
                    unsafe {
                        NativePreparedLibraryCall::from_build_owned_image(
                            &image,
                            schemas.clone(),
                            catalog,
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
    if w.stack == 0
        || w.code == 0
        || w.visits.len() != plan.blocks().len()
        || calls.len() != plan.blocks().len()
        || types.len() != plan.values().len() + plan.transport_output_count()
        || w.constant_bytes.len() != plan.constants().len()
        || types
            .iter()
            .any(|t| schemas.get(t.schema).is_none_or(|s| s.key() != t.key))
    {
        return Err("descriptor image inventory differs from its plan".into());
    }
    let frozen_maps = w.map_owner_words.as_ref().map(|owner|
        native::program_frozen_map_layout::Layout::derive(&plan, owner, &w.closed_tails))
        .transpose().map_err(str::to_owned)?.unwrap_or_default();
    if frozen_maps != w.frozen_maps {
        return Err("frozen-map descriptor proof differs from its complete source uses".into());
    }
    let descriptor = NativeProgramDescriptor {
        plan,
        schemas,
        calls,
        types,
        visits: w.visits.into_vec().into_iter().map(Into::into).collect(),
        retirements: w.retirements,
        constant_admission_bytes: super::super::constant_admission_total(&w.constant_bytes)?,
        constant_bytes: w.constant_bytes,
        scalar_bytes: w.scalar_bytes,
        observation_schema_bytes: w.observation_schema_bytes,
        max_library_inputs: w.max_library_inputs,
        window_regions: w.window_regions,
        string_map_regions: w.string_map_regions,
        scalar_aggregates: w.scalar_aggregates,
        map_owner_words: w.map_owner_words,
        frozen_maps,
        map_admission: w.map_admission,
        parallel: w.parallel,
        borrowed_strings: w.borrowed_strings,
        closed_tails: w.closed_tails,
        selected_joins: w.selected_joins,
        line_matrices: w.line_matrices,
        prefix: w.prefix,
        stack: w.stack,
        code: w.code,
        identity: w.identity,
    };
    // Wire shape, canonical schemas and actual factories were checked above.
    // Full round-trip fidelity is tested at build time; re-encoding here would
    // retain compiler-side serializers without proving source/Object provenance.
    Ok(descriptor)
}
