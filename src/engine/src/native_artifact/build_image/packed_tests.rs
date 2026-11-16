use super::*;

fn compatible<T: for<'de> Deserialize<'de> + std::fmt::Debug + PartialEq>(json: &[u8]) {
    let legacy = encode::legacy_image(json).unwrap();
    let packed = encode_native_metadata_image(json).unwrap();
    let old = decode::<T>(&legacy);
    let new = decode::<T>(&packed);
    let expected = serde_json::from_slice::<T>(json);
    assert_eq!(old.is_ok(), expected.is_ok());
    assert_eq!(new.is_ok(), expected.is_ok());
    if let (Ok(old), Ok(new), Ok(expected)) = (old, new, expected) {
        assert_eq!(old, expected);
        assert_eq!(new, expected);
    }
}

#[test]
fn packed_unsigned_arrays_preserve_numeric_visitor_types_and_boundary_values() {
    #[derive(Debug, PartialEq)]
    struct OnlyU64(u64);
    impl<'de> Deserialize<'de> for OnlyU64 {
        fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
            struct Read;
            impl<'de> Visitor<'de> for Read {
                type Value = OnlyU64;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("an unsigned64 wire value")
                }
                fn visit_u64<E: de::Error>(self, value: u64) -> Result<OnlyU64, E> {
                    Ok(OnlyU64(value))
                }
            }
            reader.deserialize_any(Read)
        }
    }
    for count in [0, 1, 3, 4, 31, 32, 33, 256, 1024] {
        let values: Vec<_> = (0..count).map(|i| (i % 256) as u8).collect();
        let json = serde_json::to_vec(&values).unwrap();
        let image = encode_native_metadata_image(&json).unwrap();
        assert!(image.starts_with(NATIVE_METADATA_IMAGE_MAGIC));
        assert_eq!(image[8], if count >= 4 { BYTE_ARRAY } else { ARRAY });
        if count >= 4 {
            assert_eq!(image.len(), 17 + count);
        }
        compatible::<Vec<u8>>(&json);
        compatible::<Vec<u16>>(&json);
        compatible::<Vec<u32>>(&json);
        compatible::<Vec<u64>>(&json);
        compatible::<Vec<u128>>(&json);
        compatible::<Vec<i8>>(&json);
        compatible::<Vec<i16>>(&json);
        compatible::<Vec<i64>>(&json);
        compatible::<Vec<i128>>(&json);
        compatible::<Vec<f32>>(&json);
        compatible::<Vec<f64>>(&json);
        compatible::<Vec<bool>>(&json);
        compatible::<Vec<OnlyU64>>(&json);
    }
}

#[test]
fn packed_raw_children_borrow_original_bytes_and_preserve_tuple_option_and_newtype_reads() {
    #[derive(Deserialize)]
    struct Outer<'a> {
        #[serde(borrow)]
        values: Vec<BuildOwnedImageRecord<'a>>,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrap(u64);
    let image = encode_native_metadata_image(br#"{"values":[0,1,127,255]}"#).unwrap();
    let outer: Outer<'_> = decode(&image).unwrap();
    for (record, expected) in outer.values.iter().zip([0, 1, 127, 255]) {
        let BuildOwnedImageRecord::UnsignedByte(byte) = record else {
            panic!("packed raw child lost its borrowed byte")
        };
        let address = *byte as *const u8 as usize;
        assert!(
            address >= image.as_ptr() as usize && address < image.as_ptr() as usize + image.len()
        );
        assert_eq!(record.decode::<u128>().unwrap(), expected as u128);
        assert_eq!(
            record.decode::<Option<Wrap>>().unwrap(),
            Some(Wrap(expected))
        );
        assert_eq!(
            record.decode::<Option<Option<Wrap>>>().unwrap(),
            Some(Some(Wrap(expected)))
        );
        let mut reader = record.reader();
        assert!(reader.end().is_err());
        assert_eq!(u64::deserialize(&mut reader).unwrap(), expected);
        reader.end().unwrap();
        assert!(u64::deserialize(&mut reader).is_err());
    }
    let image = encode_native_metadata_image(b"[0,1,127,255]").unwrap();
    assert_eq!(decode::<[u16; 4]>(&image).unwrap(), [0, 1, 127, 255]);
    assert_eq!(
        decode::<(u64, Option<Wrap>, u128, i16)>(&image).unwrap(),
        (0, Some(Wrap(1)), 127, 255)
    );
    assert!(decode::<[u8; 3]>(&image).is_err());
    assert!(decode::<[u8; 5]>(&image).is_err());
    assert!(BuildOwnedImageRecord::Native(&[1]).decode::<u64>().is_err());
}

#[test]
fn mixed_full_width_and_noninteger_arrays_keep_the_legacy_node_representation() {
    for json in [
        "[0,1,2,256]",
        "[-1,1,2,3]",
        "[0,1,2,18446744073709551615]",
        "[0,1,2,340282366920938463463374607431768211455]",
        "[1,2,3,1.0]",
        "[1,2,3,1e0]",
        "[1,2,3,-0.0]",
        "[1,2,3,true]",
        "[1,2,3,null]",
        "[1,2,3,\"4\"]",
        "[1,2,3,[4]]",
    ] {
        let legacy = encode::legacy_image(json.as_bytes()).unwrap();
        let packed = encode_native_metadata_image(json.as_bytes()).unwrap();
        assert_eq!(packed[8], ARRAY, "{json}");
        assert_eq!(&legacy[8..], &packed[8..], "{json}");
    }
    compatible::<Vec<u128>>(b"[0,1,2,340282366920938463463374607431768211455]");
    compatible::<Vec<i128>>(b"[-170141183460469231731687303715884105728,-1,0,1]");
    let image = encode_native_metadata_image(b"[1,2,3,-0.0]").unwrap();
    let values: Vec<f64> = decode(&image).unwrap();
    assert_eq!(values[3].to_bits(), (-0.0f64).to_bits());
}

#[test]
fn packed_arrays_reject_truncation_count_length_kind_and_target_type_errors() {
    let original = encode_native_metadata_image(b"[0,1,254,255]").unwrap();
    assert_eq!(original[8], BYTE_ARRAY);
    for end in 0..original.len() {
        assert!(decode::<Vec<u64>>(&original[..end]).is_err(), "end={end}");
    }
    for count in [0u32, 3, 5, u32::MAX] {
        let mut changed = original.clone();
        changed[13..17].copy_from_slice(&count.to_le_bytes());
        assert!(decode::<Vec<u64>>(&changed).is_err());
    }
    let mut changed = original.clone();
    changed.push(0);
    assert!(decode::<Vec<u64>>(&changed).is_err());
    let mut changed = original.clone();
    changed[8] = 255;
    assert!(decode::<Vec<u64>>(&changed).is_err());
    let mut changed = original.clone();
    changed[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode::<Vec<u64>>(&changed).is_err());
    assert!(decode::<Vec<bool>>(&original).is_err());
    assert!(decode::<Vec<()>>(&original).is_err());
    assert!(decode::<Vec<Vec<u8>>>(&original).is_err());
    assert!(decode::<Vec<Item>>(&original).is_err());
}

#[test]
fn packed_arrays_and_raw_children_reject_visitors_that_do_not_consume_their_values() {
    struct Ignored;
    impl<'de> Deserialize<'de> for Ignored {
        fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
            Ok(Self)
        }
    }
    #[derive(Deserialize)]
    struct Wrap(#[allow(dead_code)] Ignored);
    struct Early;
    impl<'de> Deserialize<'de> for Early {
        fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
            struct Read;
            impl<'de> Visitor<'de> for Read {
                type Value = Early;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("an incomplete array visitor")
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut values: A) -> Result<Early, A::Error> {
                    let _ = values.next_element::<u64>()?;
                    Ok(Early)
                }
            }
            reader.deserialize_seq(Read)
        }
    }
    let image = encode_native_metadata_image(b"[0,1,127,255]").unwrap();
    assert!(decode::<Vec<Ignored>>(&image).is_err());
    assert!(decode::<Vec<Option<Ignored>>>(&image).is_err());
    assert!(decode::<Vec<Wrap>>(&image).is_err());
    assert!(decode::<Early>(&image).is_err());
    let raw: Vec<BuildOwnedImageRecord<'_>> = decode(&image).unwrap();
    for record in raw {
        assert!(record.decode::<Ignored>().is_err());
        assert!(record.decode::<Option<Ignored>>().is_err());
        assert!(record.decode::<Wrap>().is_err());
    }
}

#[test]
fn packed_array_elements_retain_legacy_nesting_limits() {
    fn nested(image: Vec<u8>, levels: usize) -> Vec<u8> {
        let mut body = image[8..].to_vec();
        for _ in 0..levels {
            let mut next = vec![ARRAY];
            next.extend_from_slice(&(4u32 + body.len() as u32).to_le_bytes());
            next.extend_from_slice(&1u32.to_le_bytes());
            next.extend_from_slice(&body);
            body = next;
        }
        let mut result = image[..8].to_vec();
        result.extend_from_slice(&body);
        result
    }
    for levels in [0, MAX_DEPTH - 1, MAX_DEPTH, MAX_DEPTH + 1] {
        let old = nested(encode::legacy_image(b"[0,1,127,255]").unwrap(), levels);
        let new = nested(
            encode_native_metadata_image(b"[0,1,127,255]").unwrap(),
            levels,
        );
        assert_eq!(
            decode::<serde_json::Value>(&old).is_ok(),
            levels < MAX_DEPTH
        );
        assert_eq!(
            decode::<serde_json::Value>(&new).is_ok(),
            levels < MAX_DEPTH
        );
        let json = "[".repeat(levels) + "[0,1,127,255]" + &"]".repeat(levels);
        let a = encode::legacy_image(json.as_bytes());
        let b = encode_native_metadata_image(json.as_bytes());
        assert_eq!(a.is_ok(), b.is_ok(), "levels={levels}");
        if b.is_ok() {
            assert!(levels < MAX_DEPTH);
        }
    }
}

#[test]
fn legacy_and_packed_images_preserve_complete_records_and_full_width_enum_payloads() {
    for item in [
        Item::Unit,
        Item::One(u128::MAX),
        Item::Pair(i128::MIN, true),
        Item::Named {
            text: "é😀".into(),
            optional: None,
        },
    ] {
        let expected = Record {
            item,
            bytes: (0..32).map(|n| n * 7).collect(),
            signed: i64::MIN,
            unsigned: u64::MAX,
            float: -0.0,
        };
        let json = serde_json::to_vec(&expected).unwrap();
        let old = encode::legacy_image(&json).unwrap();
        let new = encode_native_metadata_image(&json).unwrap();
        assert!(old.starts_with(LEGACY_NATIVE_METADATA_IMAGE_MAGIC));
        assert!(new.starts_with(NATIVE_METADATA_IMAGE_MAGIC));
        for image in [old, new] {
            let actual: Record = decode(&image).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual.float.to_bits(), expected.float.to_bits());
            assert_eq!(serde_json::to_vec(&actual).unwrap(), json);
        }
    }
}
