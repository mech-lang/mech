use std::time::SystemTime;

/// Returns true if nth bit in array is 1.
#[allow(dead_code)]
pub(crate) fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

#[cfg(not(target_arch = "wasm32"))]
pub fn time_now() -> SystemTime {
    SystemTime::now()
}

#[cfg(target_arch = "wasm32")]
pub fn time_now() -> SystemTime {
    #[cfg(feature = "wasm-bindgen")]
    use js_sys::Date;
    use std::time::Duration;
    #[cfg(not(feature = "wasm-bindgen"))]
    use stdweb::web::Date;

    let offset = Duration::from_millis(Date::now() as u64);
    SystemTime::UNIX_EPOCH + offset
}
