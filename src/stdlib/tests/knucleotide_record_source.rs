//! Ordinary FASTA record selection through canonical source artifacts.
#![cfg(feature = "full_compiler")]

use mech_core::{ReactiveInstanceId, ResidentValueRef, Value, ValueData};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const FIXTURE: &str =
    include_str!("../../../hosts/gpu/tests/fixtures/native/knucleotide-idiomatic.mec");

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x46415354), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn compile(source: &str) -> ([ProgramArtifact; 2], Vec<usize>) {
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(&document(source), mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let outputs = program
        .document_outputs()
        .iter()
        .filter(|output| output.kind == SourceDocumentOutputKind::Program)
        .map(|output| output.output as usize)
        .collect();
    let original = program.compile_artifact().unwrap();
    assert!(original.requirements().is_empty());
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    ([original, decoded], outputs)
}

// Keep the actual fixture's ordinary program and remove only its provider
// bindings/delivery expressions for this library qualification. Host delivery
// and native compilation require their independent qualification routes.
fn ordinary_fixture() -> String {
    let mut deliveries = 0;
    let mut source = FIXTURE
        .lines()
        .filter(|line| !line.starts_with("+> @"))
        .map(|line| {
            if let Some(value) = line.strip_prefix("@out/text <- ") {
                deliveries += 1;
                format!("delivery-{deliveries} := {value}")
            } else {
                line.replace("@in/text", "signal<string>")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(deliveries, 3);
    source.push_str("(delivery-1, delivery-2, delivery-3)\n");
    source
}

fn record_program() -> String {
    let fixture = ordinary_fixture();
    let (prefix, _) = fixture.split_once("input-lines :=").unwrap();
    format!(
        r#"{prefix}input-lines := text/lines(signal<string>)
bounds := #RecordBounds(input-lines, sequence/length(input-lines))
record := bounds?
  | (first, end), first < end => text/join(input-lines[first..end], "")
  | * => "".
sequence := text/join([uppercase{{char}} | char <- record], "")
(record, sequence)
"#
    )
}

fn check(source: &str, inputs: &[&str], expected: impl Fn(usize, &[Value])) {
    let (artifacts, outputs) = compile(source);
    assert!(!outputs.is_empty());
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x46415354, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (turn, text) in inputs.iter().enumerate() {
            let input = [(*text).to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap_or_else(|error| panic!("turn {turn}, {text:?}: {error:?}"));
            assert!(!instance.has_ready_continuation());
            let values = outputs
                .iter()
                .map(|output| instance.copied_output(*output).unwrap())
                .collect::<Vec<_>>();
            expected(turn, &values);
        }
    }
}

#[test]
fn complete_ordinary_knucleotide_fixture_parses_without_record_or_translation_helpers() {
    document(FIXTURE);
    assert!(!FIXTURE.contains("text/record"));
    assert!(!FIXTURE.contains("bytes/translate"));
}

#[test]
fn record_bounds_handle_descriptions_empty_absent_terminal_headers_and_crlf() {
    let cases = [
        ("", "", ""),
        (">ONE\nAAAA\n", "", ""),
        (">THREE", "", ""),
        (">THREE\n", "", ""),
        (">THREE\n>FOUR\nAAAA\n", "", ""),
        (
            ">ONE\nTT\n>THREE description\ngG\nta\n>FOUR\nCC\n",
            "gGta",
            "GGTA",
        ),
        (
            ">THREE\tdescription\r\ngg\r\nTa\r\n>NEXT\r\nCC\r\n",
            "ggTa",
            "GGTA",
        ),
        (">THREE\nAC\nGT", "ACGT", "ACGT"),
        (">THREEX\nCC\n>THREE real record\nac\n", "ac", "AC"),
        (">THREE\n\nac\n\nGT\n\n>NEXT\n", "acGT", "ACGT"),
        (">THREE\nGG\n>THREE second record\nTT\n", "GG", "GG"),
    ];
    check(
        &record_program(),
        &cases.iter().map(|case| case.0).collect::<Vec<_>>(),
        |turn, outputs| {
            assert_eq!(outputs.len(), 1);
            let ValueData::Tuple(pair) = outputs[0].data() else {
                panic!("{:?}", outputs[0])
            };
            assert_eq!(pair.len(), 2);
            assert!(matches!(&pair[0], ValueData::String(text) if text.as_ref() == cases[turn].1));
            assert!(matches!(&pair[1], ValueData::String(text) if text.as_ref() == cases[turn].2));
        },
    );
}

#[test]
fn ordinary_program_produces_exact_knucleotide_output_after_record_selection() {
    let expected = concat!(
        "G 50.000\nA 25.000\nT 25.000\n\n",
        "GG 33.333\nGT 33.333\nTA 33.333\n\n",
        "1\tGGT\n1\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n",
    );
    check(
        &ordinary_fixture(),
        &[
            ">ONE\nAAAA\n>THREE description\ngG\nta\n>FOUR\nCC\n",
            ">THREE\r\nGGTA\r\n",
            ">THREE\nGGTA",
        ],
        |_, outputs| {
            assert_eq!(outputs.len(), 1);
            let ValueData::Tuple(deliveries) = outputs[0].data() else {
                panic!("{:?}", outputs[0])
            };
            assert_eq!(deliveries.len(), 3);
            let actual = deliveries
                .iter()
                .map(|value| {
                    let ValueData::String(text) = value else {
                        panic!("{value:?}")
                    };
                    text.as_ref()
                })
                .collect::<String>();
            assert_eq!(actual, expected);
        },
    );
}
