//! Host-owned, activated resource boundary for native sequence programs.
//!
//! The emitted arithmetic object receives the same Context ABI in every launch
//! policy. An activated context never falls back to ambient process IO.
use mech_engine::native_sequence::NativeResourceRequirement;

/// A host supplies one already-activated set of exact resource bindings.
/// Implementations must reject requests outside that set before observing or
/// changing any host resource. Implementing this trait is a host-side privilege;
/// it is not exposed as a callable operation in Mech source.
pub trait NativeHostIo {
    fn requirements(&self) -> &[NativeResourceRequirement];
    fn read(&mut self, request: &NativeResourceRequirement) -> Result<Vec<u8>, String>;
    fn send(&mut self, request: &NativeResourceRequirement, text: &[u8]) -> Result<(), String>;
    /// Publish staged effects only after successful native completion.
    fn finish(&mut self, _success: bool) -> Result<(), String> {
        Ok(())
    }
}

pub(super) fn parse_u64(text: &[u8]) -> Result<u64, String> {
    if text.is_empty() || !text.iter().all(u8::is_ascii_digit) {
        return Err("number/parse-u64 expects nonempty unsigned decimal text".into());
    }
    std::str::from_utf8(text)
        .map_err(|_| "number/parse-u64 expects UTF-8 text")?
        .parse()
        .map_err(|_| "number/parse-u64 value exceeds u64::MAX".into())
}

#[cfg(test)]
mod tests {
    use super::parse_u64;

    #[test]
    fn decimal_parsing_is_pure_and_checked() {
        for (text, expected) in [
            ("0", 0),
            ("0001", 1),
            ("5500", 5500),
            ("18446744073709551615", u64::MAX),
        ] {
            assert_eq!(parse_u64(text.as_bytes()).unwrap(), expected);
        }
        for text in [
            "",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "1e3",
            "18446744073709551616",
        ] {
            assert!(parse_u64(text.as_bytes()).is_err(), "{text:?}");
        }
        assert!(parse_u64(&[0xff]).is_err());
    }
}
