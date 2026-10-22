//! Private wire assembly for immutable compiler proofs. Never implement public
//! Deserialize on these proof types: only an unsafe build-owned loader may let
//! reconstructed metadata escape to a descriptor used with native code.
use super::{NativeMatchOrigin, NativeMatchValue, NativeTailLoop};
use serde::{Deserialize, Deserializer};
use serde_json::Value as Json;

pub(crate) trait ImageRead: Sized {
    fn read(image: Json) -> Result<Self, String>;
}
pub(crate) fn field<'de, T: ImageRead, D: Deserializer<'de>>(d: D) -> Result<T, D::Error> {
    T::read(Json::deserialize(d)?).map_err(serde::de::Error::custom)
}

// Implementations and all derived wire types remain crate-private. The public
// entry point on a proof is explicitly unsafe, with the same provenance and
// confinement contract as the enclosing plan/descriptor image reader.
macro_rules! image_struct {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        impl $crate::native_artifact::program_image_proofs::ImageRead for $name {
            fn read(image: serde_json::Value) -> Result<Self, String> {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Wire {
                    $(#[serde(deserialize_with = "crate::native_artifact::program_image_proofs::field")]
                    $field: $ty,)*
                }
                let wire: Wire = serde_json::from_value(image).map_err(|e| e.to_string())?;
                Ok(Self { $($field: wire.$field,)* })
            }
        }
        impl $name {
            /// Restore immutable compiler output, not a newly established proof.
            ///
            /// # Safety
            /// Keep this metadata confined until the enclosing loader establishes
            /// unchanged provenance from the SAME checked compiler build and
            /// linked Object. Wire decoding does not prove semantic validity or
            /// authenticate supplied hashes. Runtime admission, current-visit
            /// checks and resource policy remain mandatory before execution.
            pub unsafe fn from_build_owned_image(image: &serde_json::Value) -> Result<Self, String> {
                <Self as $crate::native_artifact::program_image_proofs::ImageRead>::read(image.clone())
            }
        }
    };
}
pub(crate) use image_struct;

macro_rules! image_enum {
    ($name:ident { $($variant:ident { $($field:ident: $ty:ty),* $(,)? }),* $(,)? }) => {
        impl $crate::native_artifact::program_image_proofs::ImageRead for $name {
            fn read(image: serde_json::Value) -> Result<Self, String> {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                enum Wire {
                    $($variant {
                        $(#[serde(deserialize_with = "crate::native_artifact::program_image_proofs::field")]
                        $field: $ty,)*
                    },)*
                }
                let wire: Wire = serde_json::from_value(image).map_err(|e| e.to_string())?;
                Ok(match wire { $(Wire::$variant { $($field,)* } => Self::$variant { $($field,)* },)* })
            }
        }
    };
}
pub(crate) use image_enum;

macro_rules! primitives {
    ($($ty:ty),* $(,)?) => { $(impl ImageRead for $ty {
        fn read(image: Json) -> Result<Self, String> {
            serde_json::from_value(image).map_err(|e| e.to_string())
        }
    })* };
}
primitives!(u32, u64, usize, bool, mech_core::NodeId);
impl ImageRead for NativeMatchValue {
    fn read(image: Json) -> Result<Self, String> {
        u32::read(image).map(Self)
    }
}
impl<T: ImageRead> ImageRead for Option<T> {
    fn read(image: Json) -> Result<Self, String> {
        if image.is_null() {
            Ok(None)
        } else {
            T::read(image).map(Some)
        }
    }
}
impl<T: ImageRead> ImageRead for Box<[T]> {
    fn read(image: Json) -> Result<Self, String> {
        let Json::Array(items) = image else {
            return Err("image expected an array".into());
        };
        items.into_iter().map(T::read).collect()
    }
}
impl<A: ImageRead, B: ImageRead> ImageRead for (A, B) {
    fn read(image: Json) -> Result<Self, String> {
        let [a, b]: [Json; 2] = serde_json::from_value(image).map_err(|e| e.to_string())?;
        Ok((A::read(a)?, B::read(b)?))
    }
}

image_struct!(NativeMatchOrigin { node: mech_core::NodeId, block: Option<u32>, local: Option<u32> });
image_struct!(NativeTailLoop {
    origin: NativeMatchOrigin, parent: Option<u32>, seed: NativeMatchValue,
    carrier: NativeMatchValue, next: NativeMatchValue, returned: NativeMatchValue,
    result: NativeMatchValue, initialize: u32, header: u32, body: u32,
    latch: u32, finish: u32, exit: u32,
});
