//! Runtime frame lanes and compiler-only structural descriptions have separate
//! lifetimes. Genuine compiled modules borrow the latter from immutable Object
//! data; ordinary images still own and eagerly decode their complete fields.
use mech_engine::native_artifact::BuildOwnedImageRecord;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{marker::PhantomData, ops::Deref, sync::OnceLock};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.OwnedFrameWords")]
pub(in crate::native_artifact) struct OwnedFrameWords {
    pub value: u32,
    pub fields: Vec<Option<usize>>,
    pub arrays: Vec<Option<usize>>,
    pub array_bindings: Vec<(usize, usize)>,
    pub maximum_arity: usize,
    pub scalar_paths: Vec<(Vec<usize>, usize)>,
    pub scalar_node_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct CompilerFrameData {
    pub fields: Vec<Option<usize>>,
    pub scalar_paths: Vec<(Vec<usize>, usize)>,
}

#[derive(Clone, Debug)]
enum CompilerData {
    Owned(CompilerFrameData),
    Compiled {
        fields: BuildOwnedImageRecord<'static>,
        scalar_paths: BuildOwnedImageRecord<'static>,
        decoded: OnceLock<CompilerFrameData>,
    },
}
impl CompilerData {
    fn get(&self) -> &CompilerFrameData {
        match self {
            Self::Owned(data) => data,
            Self::Compiled {
                fields,
                scalar_paths,
                decoded,
            } => {
                decoded.get_or_init(|| CompilerFrameData {
                    // The zero-allocation structural readers below already
                    // checked these exact types, and the bytes are immutable
                    // process-lifetime data from the same checked Object.
                    fields: fields.decode().expect("validated compiled frame fields"),
                    scalar_paths: scalar_paths
                        .decode()
                        .expect("validated compiled frame paths"),
                })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::native_artifact) struct FrameWords {
    pub value: u32,
    field_count: usize,
    // These are actual runtime consumers, including parent segment exports.
    pub arrays: Vec<Option<usize>>,
    pub array_bindings: Vec<(usize, usize)>,
    pub maximum_arity: usize,
    pub scalar_node_count: usize,
    compiler: CompilerData,
}
impl From<OwnedFrameWords> for FrameWords {
    fn from(frame: OwnedFrameWords) -> Self {
        Self {
            value: frame.value,
            field_count: frame.fields.len(),
            arrays: frame.arrays,
            array_bindings: frame.array_bindings,
            maximum_arity: frame.maximum_arity,
            scalar_node_count: frame.scalar_node_count,
            compiler: CompilerData::Owned(CompilerFrameData {
                fields: frame.fields,
                scalar_paths: frame.scalar_paths,
            }),
        }
    }
}
impl Deref for FrameWords {
    type Target = CompilerFrameData;
    fn deref(&self) -> &Self::Target {
        self.compiler.get()
    }
}
impl PartialEq for FrameWords {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.arrays == other.arrays
            && self.array_bindings == other.array_bindings
            && self.maximum_arity == other.maximum_arity
            && self.scalar_node_count == other.scalar_node_count
            && self.deref() == other.deref()
    }
}
impl Eq for FrameWords {}
impl Serialize for FrameWords {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            value: u32,
            fields: &'a [Option<usize>],
            arrays: &'a [Option<usize>],
            array_bindings: &'a [(usize, usize)],
            maximum_arity: usize,
            scalar_paths: &'a [(Vec<usize>, usize)],
            scalar_node_count: usize,
        }
        let compiler = self.compiler.get();
        Wire {
            value: self.value,
            fields: &compiler.fields,
            arrays: &self.arrays,
            array_bindings: &self.array_bindings,
            maximum_arity: self.maximum_arity,
            scalar_paths: &compiler.scalar_paths,
            scalar_node_count: self.scalar_node_count,
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FrameWords {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        OwnedFrameWords::deserialize(reader).map(Into::into)
    }
}
impl FrameWords {
    pub fn field_count(&self) -> usize {
        self.field_count
    }
    pub fn scalar_nodes(&self) -> usize {
        self.scalar_node_count
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn scalar_slots(&self) -> impl Iterator<Item = usize> + '_ {
        self.scalar_paths.iter().map(|(_, slot)| *slot)
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn field_slots(&self, ordinal: usize) -> impl Iterator<Item = usize> + '_ {
        self.scalar_paths
            .iter()
            .filter(move |(path, _)| path.first() == Some(&ordinal))
            .map(|(_, slot)| *slot)
    }
    #[cfg(test)]
    pub fn compiler_data_is_borrowed_and_unmaterialized(&self) -> bool {
        matches!(&self.compiler, CompilerData::Compiled { decoded, .. } if decoded.get().is_none())
    }
}

// Validate the exact existing sequence vocabulary without retaining a Vec.
// This preserves eager rejection of malformed types/counts, including nested
// path tuples. Only their heap construction is removed from module startup.
struct CheckSequence<T>(usize, PhantomData<T>);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for CheckSequence<T> {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        struct Read<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Read<T> {
            type Value = CheckSequence<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a numeric frame metadata sequence")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut rows: A,
            ) -> Result<Self::Value, A::Error> {
                let mut count = 0usize;
                while rows.next_element::<T>()?.is_some() {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| serde::de::Error::custom("frame field count overflow"))?;
                }
                Ok(CheckSequence(count, PhantomData))
            }
        }
        reader.deserialize_seq(Read(PhantomData))
    }
}

/// This adapter is usable only with a process-lifetime input borrow. The
/// private compiled-module loader selects it from genuine Object relocations;
/// external/independent image bytes cannot choose it with a serialized flag.
pub(in crate::native_artifact) struct CompiledFrame(FrameWords);
impl From<CompiledFrame> for FrameWords {
    fn from(frame: CompiledFrame) -> Self {
        frame.0
    }
}
impl Deserialize<'static> for CompiledFrame {
    fn deserialize<D: Deserializer<'static>>(reader: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename = "mech.native.image.Wire")]
        struct Wire<'a> {
            value: u32,
            #[serde(borrow)]
            fields: BuildOwnedImageRecord<'a>,
            arrays: Vec<Option<usize>>,
            array_bindings: Vec<(usize, usize)>,
            maximum_arity: usize,
            #[serde(borrow)]
            scalar_paths: BuildOwnedImageRecord<'a>,
            scalar_node_count: usize,
        }
        let wire: Wire<'static> = Wire::deserialize(reader)?;
        let field_count = wire
            .fields
            .decode::<CheckSequence<Option<usize>>>()
            .map_err(serde::de::Error::custom)?
            .0;
        wire.scalar_paths
            .decode::<CheckSequence<(CheckSequence<usize>, usize)>>()
            .map_err(serde::de::Error::custom)?;
        Ok(Self(FrameWords {
            value: wire.value,
            field_count,
            arrays: wire.arrays,
            array_bindings: wire.array_bindings,
            maximum_arity: wire.maximum_arity,
            scalar_node_count: wire.scalar_node_count,
            compiler: CompilerData::Compiled {
                fields: wire.fields,
                scalar_paths: wire.scalar_paths,
                decoded: OnceLock::new(),
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_engine::native_artifact::{BuildOwnedImageReader, encode_native_metadata_image};
    const FRAME: &str = r#"{"value":17,"fields":[null,7,null],"arrays":[0,null,null],"array_bindings":[[3,0]],"maximum_arity":3,"scalar_paths":[[[1],7],[[2,0],8]],"scalar_node_count":3}"#;

    fn compiled(text: &str, native: bool) -> Result<FrameWords, String> {
        let bytes = if native {
            encode_native_metadata_image(text.as_bytes()).map_err(|e| e.to_string())?
        } else {
            text.as_bytes().to_vec()
        };
        // Private data-reader fixture only; no executable descriptor or native
        // entry authority is constructed from these process-lifetime bytes.
        let bytes = Box::leak(bytes.into_boxed_slice());
        let mut reader = BuildOwnedImageReader::from_image(bytes).map_err(|e| e.to_string())?;
        let frame = CompiledFrame::deserialize(&mut reader).map_err(|e| e.to_string())?;
        reader.end().map_err(|e| e.to_string())?;
        Ok(frame.into())
    }
    #[test]
    fn borrowed_frame_metadata_keeps_runtime_reads_without_materializing_compiler_vectors() {
        for native in [false, true] {
            let frame = compiled(FRAME, native).unwrap();
            assert!(frame.compiler_data_is_borrowed_and_unmaterialized());
            assert_eq!(frame.value, 17);
            assert_eq!(frame.maximum_arity, 3);
            assert_eq!(frame.field_count(), 3);
            assert_eq!(frame.arrays, [Some(0), None, None]);
            assert_eq!(frame.array_bindings, [(3, 0)]);
            assert_eq!(frame.scalar_nodes(), 3);
            assert!(frame.compiler_data_is_borrowed_and_unmaterialized());
        }
    }
    #[test]
    fn borrowed_frame_metadata_field_count_uses_the_exact_validated_field_sequence() {
        // Private data-reader fixtures only: no descriptor or native entry
        // authority is constructed from these independent record bytes.
        for count in [0, 1, 3, 17] {
            let mut row: serde_json::Value = serde_json::from_str(FRAME).unwrap();
            row["fields"] = serde_json::json!(vec![None::<usize>; count]);
            row["arrays"] = serde_json::json!([]);
            row["maximum_arity"] = serde_json::json!(99);
            let text = row.to_string();
            let owned: FrameWords = serde_json::from_str(&text).unwrap();
            assert_eq!(owned.field_count(), count);
            for native in [false, true] {
                let frame = compiled(&text, native).unwrap();
                assert_eq!(frame.field_count(), count);
                assert!(frame.compiler_data_is_borrowed_and_unmaterialized());
                assert_eq!(frame.field_count(), owned.fields.len());
                assert_eq!(serde_json::to_value(&frame).unwrap(), row);
            }
        }
    }
    #[test]
    fn borrowed_frame_metadata_serialization_and_clones_preserve_complete_descriptions() {
        let owned: FrameWords = serde_json::from_str(FRAME).unwrap();
        for native in [false, true] {
            let frame = compiled(FRAME, native).unwrap();
            let copied = frame.clone();
            assert!(copied.compiler_data_is_borrowed_and_unmaterialized());
            assert_eq!(serde_json::to_string(&frame).unwrap(), FRAME);
            assert!(!frame.compiler_data_is_borrowed_and_unmaterialized());
            assert!(copied.compiler_data_is_borrowed_and_unmaterialized());
            assert_eq!(frame, owned);
            assert_eq!(copied, owned);
            assert_eq!(frame.scalar_slots().collect::<Vec<_>>(), [7, 8]);
            assert_eq!(frame.field_slots(2).collect::<Vec<_>>(), [8]);
        }
    }
    #[test]
    fn borrowed_frame_metadata_rejects_malformed_rows_before_access() {
        let mut bad = Vec::new();
        for (field, value) in [
            ("fields", serde_json::json!(null)),
            ("fields", serde_json::json!([[0]])),
            ("fields", serde_json::json!([-1])),
            ("scalar_paths", serde_json::json!(null)),
            ("scalar_paths", serde_json::json!([[[], null]])),
            ("scalar_paths", serde_json::json!([[[-1], 0]])),
            ("scalar_paths", serde_json::json!([[[], 0, 1]])),
            ("scalar_paths", serde_json::json!([[]])),
            ("scalar_paths", serde_json::json!([["path", 0]])),
            ("arrays", serde_json::json!(["array"])),
            ("array_bindings", serde_json::json!([[1]])),
            ("scalar_node_count", serde_json::json!(-1)),
        ] {
            let mut row: serde_json::Value = serde_json::from_str(FRAME).unwrap();
            row[field] = value;
            bad.push(row.to_string());
        }
        bad.push(FRAME.replacen("\"value\":17", "\"value\":17,\"value\":18", 1));
        bad.push(FRAME.replacen("\"fields\":[null,7,null],", "", 1));
        bad.push(FRAME.replacen("\"value\":17", "\"foreign\":0,\"value\":17", 1));
        for row in bad {
            assert!(serde_json::from_str::<FrameWords>(&row).is_err(), "{row}");
            for native in [false, true] {
                assert!(compiled(&row, native).is_err(), "{row}");
            }
        }
    }
}
