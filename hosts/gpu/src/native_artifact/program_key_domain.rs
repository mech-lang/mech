//! Finite ASCII domains derived from immutable source producers. A domain is
//! a property of every successful output, never an observation of benchmark
//! data. Unknown producers keep the full ASCII representation.
use mech_core::{SchemaBody, ValueData};
use mech_engine::native_artifact::{NativeCollectionPrimitive as Primitive,
    NativeCollectionResult, NativeIterationAction as Action, NativeMatchAction as Scalar,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Domain {
    alphabet: [u64; 2],
    bits: u8,
    // An injective bit window needs only shift-and-mask in generated code.
    // Other finite domains use a compact ordinal table.
    shift: Option<u8>,
}
impl Domain {
    pub fn ascii() -> Self { Self { alphabet: [u64::MAX; 2], bits: 7, shift: Some(0) } }
    pub fn bits(&self) -> u8 { self.bits }
    pub fn shift(&self) -> Option<u8> { self.shift }
    // A bit-window encoder is emitted directly. Other finite alphabets retain
    // the existing lossless representation until a table encoder is available.
    pub fn packing(self) -> Self { if self.shift.is_some() { self } else { Self::ascii() } }
    pub fn contains(&self, byte: u8) -> bool {
        byte < 128 && self.alphabet[usize::from(byte / 64)] & (1 << (byte % 64)) != 0
    }
    /// Every successful producer output consists of one-byte graphemes.
    /// CRLF is the only multi-byte grapheme in ASCII. If either character is
    /// excluded from the complete source alphabet, that pair cannot occur.
    pub fn byte_graphemes(&self) -> bool {
        !self.contains(b'\r') || !self.contains(b'\n')
    }
    pub fn count(&self) -> usize {
        (self.alphabet[0].count_ones() + self.alphabet[1].count_ones()) as usize
    }
    pub fn tables(&self) -> ([u8; 128], [u8; 128]) {
        let mut encode = [u8::MAX; 128];
        let mut decode = [u8::MAX; 128];
        let mut rank = 0;
        for byte in 0..128u8 {
            if self.contains(byte) {
                let code = self.shift.map_or(rank, |shift| (byte >> shift) & ((1 << self.bits) - 1));
                encode[usize::from(byte)] = code;
                decode[usize::from(code)] = byte;
                rank += 1;
            }
        }
        (encode, decode)
    }
    pub(super) fn from_alphabet(alphabet: [u64; 2]) -> Option<Self> {
        let count = alphabet.iter().map(|x| x.count_ones()).sum::<u32>();
        if count == 0 { return None; }
        let bits = (u32::BITS - (count - 1).leading_zeros()).max(1) as u8;
        let mut domain = Self { alphabet, bits, shift: None };
        for shift in 0..=7 - bits {
            let mut seen = [0u64; 2];
            let unique = (0..128u8).filter(|&byte| domain.contains(byte)).all(|byte| {
                let code = (byte >> shift) & ((1 << bits) - 1);
                let flag = 1 << (code % 64);
                let word = &mut seen[usize::from(code / 64)];
                let absent = *word & flag == 0;
                *word |= flag;
                absent
            });
            if unique { domain.shift = Some(shift); break; }
        }
        Some(domain)
    }
}

fn add_text(alphabet: &mut [u64; 2], text: &str) -> Option<()> {
    if !text.is_ascii() { return None; }
    for byte in text.bytes() { alphabet[usize::from(byte / 64)] |= 1 << (byte % 64); }
    Some(())
}

fn producer(plan: &Plan, value: Value) -> Option<&Action> {
    let mut matches = plan.blocks().iter().filter_map(|block| {
        let matches = match block.action() {
            Action::Scalar(Scalar::Copy { output, .. }) | Action::LibraryCall { output, .. }
            | Action::Structural { output, .. } => *output == value,
            Action::TextJoinFinish { collection } => plan.collections().get(*collection as usize)
                .is_some_and(|c| matches!(c.result(), NativeCollectionResult::TextJoin(s) if s.output() == value)),
            _ => false,
        };
        matches.then_some(block.action())
    });
    let result = matches.next()?;
    matches.next().is_none().then_some(result)
}

fn string_alphabet(plan: &Plan, value: Value, pending: &mut Vec<Value>) -> Option<[u64; 2]> {
    if pending.contains(&value) || pending.len() > plan.values().len() { return None; }
    if let Some(constant) = plan.constants().iter().find(|c| c.value() == value) {
        let ValueData::String(text) = constant.constant().data() else { return None; };
        let mut alphabet = [0; 2];
        add_text(&mut alphabet, text)?;
        return Some(alphabet);
    }
    pending.push(value);
    let result = (|| match producer(plan, value)? {
        Action::Scalar(Scalar::Copy { source, .. }) => string_alphabet(plan, *source, pending),
        Action::TextJoinFinish { collection } => {
            let region = plan.string_map_regions().into_iter().find(|r| r.collection() == *collection)?;
            let NativeCollectionResult::TextJoin(join) = plan.collections()[*collection as usize].result()
                else { return None; };
            let mut alphabet = map_alphabet(plan, region.map(), pending)?;
            let separator = string_alphabet(plan, join.separator(), pending)?;
            alphabet[0] |= separator[0]; alphabet[1] |= separator[1];
            Some(alphabet)
        }
        _ => None,
    })();
    pending.pop();
    result
}

fn map_alphabet(plan: &Plan, value: Value, pending: &mut Vec<Value>) -> Option<[u64; 2]> {
    if pending.contains(&value) || pending.len() > plan.values().len() { return None; }
    if let Some(constant) = plan.constants().iter().find(|c| c.value() == value) {
        let ValueData::Map(map) = constant.constant().data() else { return None; };
        let mut alphabet = [0; 2];
        for entry in map.entries() {
            let ValueData::String(text) = entry.value() else { return None; };
            add_text(&mut alphabet, text)?;
        }
        return Some(alphabet);
    }
    pending.push(value);
    let result = (|| match producer(plan, value)? {
        Action::Scalar(Scalar::Copy { source, .. }) => map_alphabet(plan, *source, pending),
        Action::LibraryCall { template, operands, .. } => {
            if !matches!(template.primitive(), Primitive::CompositePack(n) if n == operands.len())
                || operands.len() % 2 != 0
                || !matches!(plan.values().get(value.get() as usize)?.definition().body(), SchemaBody::Map { .. })
            { return None; }
            let mut alphabet = [0; 2];
            for pair in operands.chunks_exact(2) {
                let chars = string_alphabet(plan, pair[1], pending)?;
                alphabet[0] |= chars[0]; alphabet[1] |= chars[1];
            }
            Some(alphabet)
        }
        _ => None,
    })();
    pending.pop();
    result
}

pub(super) fn derive(plan: &Plan, source: Value) -> Option<Domain> {
    Domain::from_alphabet(string_alphabet(plan, source, &mut Vec::new())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_grapheme_proof_covers_ascii_pairs_and_retains_crlf_guard() {
        assert!(!Domain::ascii().byte_graphemes());
        for first in 0..128u8 {
            for second in 0..128u8 {
                let mut alphabet = [0; 2];
                let text = String::from_utf8(vec![first, second]).unwrap();
                add_text(&mut alphabet, &text).unwrap();
                let domain = Domain::from_alphabet(alphabet).unwrap();
                if domain.byte_graphemes() {
                    assert!(mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf(&text));
                } else {
                    assert!(domain.contains(b'\r') && domain.contains(b'\n'));
                    assert!(!mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf("\r\n"));
                }
            }
        }
    }

    #[test]
    fn singleton_nonzero_code_and_unused_codes_remain_distinct() {
        let mut alphabet = [0; 2];
        add_text(&mut alphabet, "a").unwrap();
        let singleton = Domain::from_alphabet(alphabet).unwrap();
        assert_eq!((singleton.bits(), singleton.shift()), (1, Some(0)));
        let (encode, decode) = singleton.packing().tables();
        assert_eq!(encode[usize::from(b'a')], 1);
        assert_eq!(decode[1], b'a');
        assert_eq!(decode[0], u8::MAX);

        add_text(&mut alphabet, "bc").unwrap();
        let three = Domain::from_alphabet(alphabet).unwrap();
        assert_eq!((three.bits(), three.shift()), (2, Some(0)));
        let (encode, decode) = three.packing().tables();
        assert_eq!(&decode[..4], &[u8::MAX, b'a', b'b', b'c']);
        for byte in b"abc" { assert_eq!(decode[usize::from(encode[usize::from(*byte)])], *byte); }
    }

    #[test]
    fn noninjective_bit_windows_fall_back_to_all_ascii_codes() {
        let mut alphabet = [0; 2];
        add_text(&mut alphabet, "\x01\x02\x04\x08").unwrap();
        let sparse = Domain::from_alphabet(alphabet).unwrap();
        assert_eq!((sparse.count(), sparse.bits(), sparse.shift()), (4, 2, None));
        assert_eq!(sparse.packing(), Domain::ascii());
        let (encode, decode) = sparse.packing().tables();
        for byte in 0..128u8 {
            assert_eq!(encode[usize::from(byte)], byte);
            assert_eq!(decode[usize::from(byte)], byte);
        }
    }

    #[test]
    fn finite_domains_are_lossless_and_choose_injective_windows_only() {
        for text in ["ACGT", "ab:!?", "a", "\0a", "ABCXYZ", " \t\n", "\x01\x02\x04\x08"] {
            let mut alphabet = [0; 2]; add_text(&mut alphabet, text).unwrap();
            let domain = Domain::from_alphabet(alphabet).unwrap();
            let (encode, decode) = domain.tables();
            assert_eq!(domain.count(), text.bytes().collect::<std::collections::BTreeSet<_>>().len());
            for byte in 0..128u8 {
                if domain.contains(byte) {
                    assert!(encode[usize::from(byte)] < (1u8 << domain.bits));
                    assert_eq!(decode[usize::from(encode[usize::from(byte)])], byte);
                } else { assert_eq!(encode[usize::from(byte)], u8::MAX); }
            }
        }
        let dna = Domain::from_alphabet([0, (1 << (b'A' - 64)) | (1 << (b'C' - 64))
            | (1 << (b'G' - 64)) | (1 << (b'T' - 64))]).unwrap();
        assert_eq!((dna.bits(), dna.shift()), (2, Some(1)));
        let mut sparse = [0; 2]; add_text(&mut sparse, "\x01\x02\x04\x08").unwrap();
        assert_eq!(Domain::from_alphabet(sparse).unwrap().shift(), None);
        assert_eq!(Domain::ascii().tables(), (core::array::from_fn(|i| i as u8), core::array::from_fn(|i| i as u8)));
    }
}
