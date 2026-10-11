//! Reusable sequence, map, text and numeric operations for native dataflows.
//! This catalog is opt-in; it does not modify the frozen resident distribution.
use mech_engine::native_sequence::{Catalog, Kind::*, OperationSignature, OutputKind::*};

macro_rules! operation {
    ($name:literal, $id:literal, [$([$($kind:ident),+]),*], $output:expr) => {
        OperationSignature { name: $name, op: $id, inputs: &[$(&[$($kind),+]),*], output: $output }
    };
}
const OPERATIONS: Catalog = &[
    operation!("io/read", 1, [], Fixed(Text)),
    operation!("text/record", 2, [[Text], [Text], [Text]], Fixed(Text)),
    operation!("bytes/encode", 3, [[Text], [Text], [Text]], Fixed(Bytes)),
    operation!(
        "sequence/packed-windows",
        4,
        [[Bytes], [Integer], [Integer]],
        Fixed(Windows)
    ),
    operation!("map/histogram", 5, [[Windows]], Fixed(Histogram)),
    operation!("io/write", 8, [[Text]], Fixed(Unit)),
    operation!("map/keys", 13, [[Histogram]], Fixed(Integers)),
    operation!("map/values", 14, [[Histogram]], Fixed(Integers)),
    operation!(
        "sequence/decode",
        15,
        [[Integers], [Integer], [Integer], [Text]],
        Fixed(Texts)
    ),
    operation!(
        "sequence/sort-indices",
        16,
        [
            [Integers, Numbers, Texts],
            [Text],
            [Integers, Numbers, Texts],
            [Text]
        ],
        Fixed(Indices)
    ),
    operation!(
        "sequence/select",
        17,
        [[Integers, Numbers, Texts], [Indices]],
        Input(0)
    ),
    operation!("stats/sum", 18, [[Integers]], Fixed(Integer)),
    operation!("number/to-f64", 19, [[Integer, Integers]], Numeric),
    operation!(
        "number/format",
        24,
        [[Integer, Integers, Float, Numbers], [Integer]],
        Formatted
    ),
    operation!("text/zip", 25, [[Texts], [Texts], [Text]], Fixed(Texts)),
    operation!("text/join", 26, [[Texts], [Text]], Fixed(Text)),
    operation!("string/concat", 27, [[Text], [Text]], Fixed(Text)),
    operation!("text/length", 28, [[Text]], Fixed(Integer)),
    operation!("sequence/pack", 29, [[Bytes], [Integer]], Fixed(Integer)),
    operation!("map/get", 30, [[Histogram], [Integer]], Fixed(Integer)),
    operation!("text/join-terminated", 31, [[Texts], [Text]], Fixed(Text)),
    operation!("io/argument-u64", 32, [[Integer]], Fixed(Integer)),
    operation!("sequence/fill", 33, [[Integer], [Float]], Fixed(Numbers)),
    operation!(
        "sequence/length",
        34,
        [[Numbers, Integers, Texts]],
        Fixed(Integer)
    ),
    operation!(
        "matrix/apply",
        35,
        [[Numbers], [Integer], [Function], [Text]],
        Fixed(Numbers)
    ),
    operation!(
        "matrix/apply-transpose",
        36,
        [[Numbers], [Integer], [Function], [Text]],
        Fixed(Numbers)
    ),
    operation!(
        "matrix/require-domain",
        73,
        [[Integer], [Integer], [Function]],
        Fixed(Unit)
    ),
    operation!(
        "sequence/require-length",
        72,
        [[Numbers], [Integer]],
        Fixed(Numbers)
    ),
    operation!("stats/dot", 37, [[Numbers], [Numbers]], Fixed(Float)),
    operation!("math/sqrt", 38, [[Float]], Fixed(Float)),
    operation!(
        "sequence/iterate",
        39,
        [[Numbers, Float], [Integer], [Function]],
        Input(0)
    ),
    operation!("text/records", 40, [[Text], [Text], [Text]], Fixed(Records)),
    operation!(
        "records/map",
        42,
        [[Records], [Function]],
        Fixed(TextStream)
    ),
    operation!("bytes/translate", 43, [[Bytes], [ByteTable]], Fixed(Bytes)),
    operation!("sequence/reverse", 44, [[Bytes]], Fixed(Bytes)),
    operation!("io/write-each", 45, [[TextStream]], Fixed(Unit)),
    operation!(
        "bytes/table",
        48,
        [[Text], [Text], [Text]],
        Fixed(ByteTable)
    ),
    operation!("text/wrap", 49, [[Bytes], [Integer], [Text]], Fixed(Text)),
    operation!("bytes/from-text", 51, [[Text]], Fixed(Bytes)),
    operation!("io/read-records", 52, [[Text], [Text]], Fixed(Records)),
    operation!("regex/count-bytes", 53, [[Text], [Text]], Fixed(Integer)),
    operation!(
        "regex/replace-bytes",
        54,
        [[Text], [Text], [Text]],
        Fixed(Text)
    ),
];

pub fn native_sequence_catalog() -> Catalog {
    OPERATIONS
}

/// General grid orchestration and image serialization source operations.
pub fn native_grid_catalog() -> &'static [&'static str] {
    &["io/argument-u64", "grid/map-until", "image/pbm", "io/write"]
}

/// Closed control-flow and fixed-byte operations admitted by the native compiler.
pub fn native_control_catalog() -> &'static [&'static str] {
    &[
        "control/while",
        "control/if",
        "control/select",
        "stats/sum",
        "parallel/reduce-ranges",
        "number/mod",
        "number/max",
        "number/factorial",
        "number/bounded",
        "sequence/iota-bytes",
        "sequence/zero-bytes",
        "sequence/get",
        "sequence/set",
        "sequence/reverse-prefix",
        "sequence/rotate-prefix",
        "io/argument-u64",
        "number/to-f64",
        "bytes/from-text",
        "sequence/f64-values",
        "sequence/cumulative-sum",
        "sequence/length",
        "io/write",
        "io/write-scan",
        "io/print",
        "number/shift-left",
        "heap/null",
        "heap/new-refs",
        "heap/read-refs",
        "memory/with-ref-arena",
        "memory/with-frame",
        "memory/null",
        "memory/allocate",
        "memory/read",
        "memory/with-arena",
        "bigint/new",
        "bigint/assign",
        "bigint/multiply-u64-into",
        "bigint/add-into",
        "bigint/subtract-into",
        "bigint/floor-u64",
        "bigint/floor",
        "bigint/add-multiple-u64",
        "bigint/subtract-multiple-u64",
        "bigint/less-equal",
    ]
}
