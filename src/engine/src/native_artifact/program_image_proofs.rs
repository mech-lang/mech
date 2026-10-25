//! Private wire assembly for immutable compiler proofs. Never implement public
//! Deserialize on these proof types: only an unsafe build-owned loader may let
//! reconstructed metadata escape to a descriptor used with native code.
use super::{NativeMatchOrigin, NativeMatchValue, NativeTailLoop};
use serde::{Deserialize, Deserializer};

pub(crate) trait ImageRead: Sized {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error>;
}
pub(crate) fn field<'de, T: ImageRead, D: Deserializer<'de>>(d: D) -> Result<T, D::Error> {
    T::read(d)
}

// A private adapter lets serde drive nested fields directly. The previous
// reader copied each nested proof through an owned JSON tree before decoding
// it; this wrapper neither publishes Deserialize on a proof nor copies JSON.
struct Image<T>(T);
impl<'de, T: ImageRead> Deserialize<'de> for Image<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::read(d).map(Self)
    }
}

// Implementations and all derived wire types remain crate-private. The public
// entry point on a proof is explicitly unsafe, with the same provenance and
// confinement contract as the enclosing plan/descriptor image reader.
macro_rules! image_struct {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        impl $crate::native_artifact::program_image_proofs::ImageRead for $name {
            fn read<'de, D: serde::Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Wire {
                    $(#[serde(deserialize_with = "crate::native_artifact::program_image_proofs::field")]
                    $field: $ty,)*
                }
                let wire = <Wire as serde::Deserialize>::deserialize(image)?;
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
                <Self as $crate::native_artifact::program_image_proofs::ImageRead>::read(image)
                    .map_err(|error| error.to_string())
            }
        }
    };
}
pub(crate) use image_struct;

macro_rules! image_enum {
    ($name:ident { $($variant:ident { $($field:ident: $ty:ty),* $(,)? }),* $(,)? }) => {
        impl $crate::native_artifact::program_image_proofs::ImageRead for $name {
            fn read<'de, D: serde::Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                enum Wire {
                    $($variant {
                        $(#[serde(deserialize_with = "crate::native_artifact::program_image_proofs::field")]
                        $field: $ty,)*
                    },)*
                }
                let wire = <Wire as serde::Deserialize>::deserialize(image)?;
                Ok(match wire { $(Wire::$variant { $($field,)* } => Self::$variant { $($field,)* },)* })
            }
        }
    };
}
pub(crate) use image_enum;

macro_rules! primitives {
    ($($ty:ty),* $(,)?) => { $(impl ImageRead for $ty {
        fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
            Self::deserialize(image)
        }
    })* };
}
primitives!(
    u32,
    u64,
    usize,
    bool,
    mech_core::NodeId,
    mech_core::ProgramRevision
);
impl ImageRead for NativeMatchValue {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        u32::read(image).map(Self)
    }
}
impl<T: ImageRead> ImageRead for Option<T> {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        Option::<Image<T>>::deserialize(image).map(|value| value.map(|image| image.0))
    }
}
impl<T: ImageRead> ImageRead for Box<[T]> {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        Vec::<Image<T>>::deserialize(image)
            .map(|items| items.into_iter().map(|image| image.0).collect())
    }
}
impl<A: ImageRead, B: ImageRead> ImageRead for (A, B) {
    fn read<'de, D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        <(Image<A>, Image<B>)>::deserialize(image).map(|(a, b)| (a.0, b.0))
    }
}

image_struct!(NativeMatchOrigin { node: mech_core::NodeId, block: Option<u32>, local: Option<u32> });
image_struct!(NativeTailLoop {
    origin: NativeMatchOrigin, parent: Option<u32>, seed: NativeMatchValue,
    carrier: NativeMatchValue, next: NativeMatchValue, returned: NativeMatchValue,
    result: NativeMatchValue, initialize: u32, header: u32, body: u32,
    latch: u32, finish: u32, exit: u32,
});

#[cfg(test)]
mod tests {
    use super::*;

    // Wire-only fixture: these records never reach a native descriptor or code.
    struct WireFixture {
        origin: NativeMatchOrigin,
        words: Box<[Option<(NativeMatchValue, u64)>]>,
    }
    image_struct!(WireFixture {
        origin: NativeMatchOrigin,
        words: Box<[Option<(NativeMatchValue, u64)>]>,
    });

    fn image() -> serde_json::Value {
        serde_json::json!({
            "origin": {"node": 7, "block": null, "local": 2},
            "words": [[3, u64::MAX], null, [4, 0]],
        })
    }

    #[test]
    fn direct_proof_fields_preserve_nested_options_tuples_and_full_width_words() {
        let bytes = serde_json::to_vec(&image()).unwrap();
        let mut reader = serde_json::Deserializer::from_slice(&bytes);
        let loaded = WireFixture::read(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(loaded.origin.node.get(), 7);
        assert_eq!(loaded.origin.block, None);
        assert_eq!(loaded.origin.local, Some(2));
        assert_eq!(
            loaded.words.as_ref(),
            &[
                Some((NativeMatchValue(3), u64::MAX)),
                None,
                Some((NativeMatchValue(4), 0))
            ]
        );
        // SAFETY: this is a wire-only fixture; no execution proof can escape.
        let borrowed = unsafe { WireFixture::from_build_owned_image(&image()) }.unwrap();
        assert_eq!(borrowed.words, loaded.words);
    }

    #[test]
    fn direct_proof_fields_reject_unknown_nested_fields_and_malformed_words() {
        let original = image();
        let mut corruptions = Vec::new();
        for (pointer, replacement) in [
            ("/origin/node", serde_json::json!(u64::from(u32::MAX) + 1)),
            ("/words/0", serde_json::json!([3])),
            ("/words/0", serde_json::json!([3, 4, 5])),
            ("/words/0/0", serde_json::json!(-1)),
            ("/words/0/1", serde_json::json!(true)),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = replacement;
            corruptions.push(changed);
        }
        let mut changed = original.clone();
        changed["unexpected"] = true.into();
        corruptions.push(changed);
        let mut changed = original;
        changed["origin"]["unexpected"] = true.into();
        corruptions.push(changed);
        for image in corruptions {
            let bytes = serde_json::to_vec(&image).unwrap();
            let mut reader = serde_json::Deserializer::from_slice(&bytes);
            assert!(WireFixture::read(&mut reader).is_err());
            // SAFETY: rejection probe on the same wire-only fixture.
            assert!(unsafe { WireFixture::from_build_owned_image(&image) }.is_err());
        }
    }
}
