use mech_syntax::document::*;
#[path = "support/document_stream.rs"]
mod support;
use support::*;

// A transport-only test adapter. The parser itself accepts valid UTF-8 strings.
#[derive(Default)]
struct Utf8Transport {
    pending: Vec<u8>,
}
impl Utf8Transport {
    fn feed(&mut self, stream: &mut DocumentStream, bytes: &[u8]) -> Result<(), &'static str> {
        self.pending.extend_from_slice(bytes);
        let (valid, malformed) = match core::str::from_utf8(&self.pending) {
            Ok(_) => (self.pending.len(), false),
            Err(error) => (error.valid_up_to(), error.error_len().is_some()),
        };
        if malformed {
            return Err("malformed UTF-8");
        }
        if valid > 0 {
            append(
                stream,
                core::str::from_utf8(&self.pending[..valid]).unwrap(),
                127,
            );
            self.pending.drain(..valid);
        }
        Ok(())
    }
    fn finish(&self) -> Result<(), &'static str> {
        if self.pending.is_empty() {
            Ok(())
        } else {
            Err("incomplete UTF-8")
        }
    }
}
#[test]
fn every_byte_split_preserves_unicode_and_grammar_boundaries() {
    for text in [
        "x := \"é👩‍💻🇺🇸\"\r\ny := 2\r\n",
        "x := 1..=10\ny := -1 + 2\n",
        "#machine -> :next ~> :other => :value\n",
        "x := {a: 1, b: [2, 3]}\ny := [v | v <- xs]\n",
        "Title\n=====\ntext {x + 1}\n",
        "~~~mech\nx := \"a\"\ny := 2\n~~~\n",
        "```text\né👩‍💻\n```\n",
        "╭◉╮\n(◉ ◯ ◉)\n",
    ] {
        for split in 0..=text.len() {
            let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
            let mut transport = Utf8Transport::default();
            transport
                .feed(&mut stream, &text.as_bytes()[..split])
                .unwrap();
            transport
                .feed(&mut stream, &text.as_bytes()[split..])
                .unwrap();
            transport.finish().unwrap();
            equivalent(&finish(&mut stream, 127), text);
        }
        let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
        let mut transport = Utf8Transport::default();
        for byte in text.as_bytes() {
            transport
                .feed(&mut stream, core::slice::from_ref(byte))
                .unwrap();
        }
        transport.finish().unwrap();
        equivalent(&finish(&mut stream, 127), text);
    }
}
#[test]
fn transport_rejects_invalid_or_incomplete_encoding_without_replacement() {
    let mut stream = DocumentStream::new(DocumentId(1), ParseConfig::default());
    let mut transport = Utf8Transport::default();
    transport.feed(&mut stream, b"x := ").unwrap();
    assert_eq!(transport.feed(&mut stream, &[0xff]), Err("malformed UTF-8"));
    assert_eq!(stream.source().to_contiguous_string(), "x := ");
    let mut transport = Utf8Transport::default();
    transport.feed(&mut stream, &[0xf0, 0x9f]).unwrap();
    assert_eq!(transport.finish(), Err("incomplete UTF-8"));
    assert_eq!(stream.source().to_contiguous_string(), "x := ");
}
#[test]
fn deterministic_random_and_scalar_partitions_cover_long_inputs() {
    for text in [
        format!("x := \"{}\"\ny := 2\n", "é👩‍💻".repeat(128)),
        format!("```mech\n{}\n```\n", "x := 1\n".repeat(64)),
    ] {
        let cuts: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(core::iter::once(text.len()))
            .collect();
        for seed in [1u64, 17, 826] {
            let mut random = seed;
            let mut at = 0;
            let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
            while at + 1 < cuts.len() {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let next = (at + 1 + ((random >> 32) % 31) as usize).min(cuts.len() - 1);
                append(&mut stream, &text[cuts[at]..cuts[next]], 1024);
                at = next;
            }
            equivalent(&finish(&mut stream, 1024), &text);
        }
    }
}
