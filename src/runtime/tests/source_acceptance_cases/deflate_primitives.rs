//! Compression prerequisites execute through the public canonical compiler.
use super::{compile, roundtrip};
use mech_core::{ReactiveInstanceId, ValueDataDraft};
use mech_engine::resident::{ActivationFacts, activate};

#[test]
fn integer_arithmetic_and_byte_buffer_access_execute_source_and_decoded() {
    for (source, expected) in [
        ("1u8 + 2u8\n", ValueDataDraft::U8(3)),
        ("7u32 * 3u32\n", ValueDataDraft::U32(21)),
        (
            "bytes := [97u8 98u8 97u8]\nbytes[3u64]\n",
            ValueDataDraft::U8(97),
        ),
    ] {
        let original = compile(source);
        for artifact in [&original, &roundtrip(&original)] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x44464c54, 0),
                artifact,
                &mech_stdlib::runtime_catalog(),
                &ActivationFacts::default(),
            )
            .unwrap();
            instance.turn(&[]).unwrap();
            assert_eq!(
                instance
                    .copied_output(artifact.outputs().len() - 1)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected,
                "{source}",
            );
        }
    }
}

#[test]
fn bitwise_hash_word_is_available_to_source() {
    let source = "+> bits\nbits/xor(305419896u64, 4294967295u64)\n";
    let artifact = compile(source);
    let mut instance = activate(
        ReactiveInstanceId::new(0x44464c54, 1),
        &artifact,
        &mech_stdlib::runtime_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    instance.turn(&[]).unwrap();
    assert_eq!(
        instance
            .copied_output(artifact.outputs().len() - 1)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::U64(0xedcb_a987),
    );
}
