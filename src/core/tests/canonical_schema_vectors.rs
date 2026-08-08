use mech_core::*;

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |value| match value {
                b'0'..=b'9' => value - b'0',
                b'a'..=b'f' => value - b'a' + 10,
                _ => panic!("invalid test hex"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn assert_vector(body: SchemaBody, bytes: &str, key: &str) {
    let schema = SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap();
    assert_eq!(schema.canonical_bytes().as_ref(), decode_hex(bytes));
    assert_eq!(schema.key().as_bytes().as_slice(), decode_hex(key));
}

#[test]
fn scalar_and_ordered_record_vectors_match_the_c0_reference() {
    assert_vector(
        SchemaBody::Bool,
        "0100000000010000000000000001",
        "ae4fd11fd6195a3547c6aa0a5dc7e31a491ae0a604d1ff6543ed6472e88cbadb",
    );
    assert_vector(
        SchemaBody::UnsignedInteger(IntegerWidth::W128),
        "01000000000300000000000000028000",
        "d7a42bd125b7d8730457b898e2697fe026852b65c3a8ae75db5f6ce86ef8dca4",
    );
    assert_vector(
        SchemaBody::Rational64,
        "010000000005000000000000000640004000",
        "0df89d0ebf468c6d0e29db5d6acce2ba19a91e143dcb27ec5c04c77952b88559",
    );
    assert_vector(
        SchemaBody::Record(
            vec![
                SchemaField {
                    name: "b".to_owned(),
                    schema: SchemaBody::Bool,
                },
                SchemaField {
                    name: "a".to_owned(),
                    schema: SchemaBody::UnsignedInteger(IntegerWidth::W16),
                },
            ]
            .into_boxed_slice(),
        ),
        "01000000002b000000000000000e020000000100000000000000620100000000000000010100000000000000610300000000000000021000",
        "f2aceea905a04a37501c92be87409061e2d25b85d8be13823efc5f62ed619cd7",
    );
}

#[test]
fn parameterized_matrix_vector_matches_the_c0_reference() {
    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(1),
            upper_bound: Some(DimensionExpr::Constant(8)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    assert_eq!(
        schema.canonical_bytes().as_ref(),
        decode_hex(
            "01010000000109000000000000000101000000000000000109000000000000000108000000000000001d000000000000000f03000000000000000440000100000005000000000000000200000000"
        )
    );
    assert_eq!(
        schema.key().as_bytes().as_slice(),
        decode_hex("d65fa2b13ee39e3af34c266530cac9afe6b8b27cc2653c63ee65474588839aae")
    );
}
