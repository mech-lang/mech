use sha2::{Digest, Sha256};

pub(crate) fn transport_package_id(package_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"mech-bundle-nominal-package-v1\0");
    digest.update(package_id.as_bytes());
    let mut result = String::from("sha256:");
    for byte in digest.finalize() {
        use std::fmt::Write as _;
        write!(&mut result, "{byte:02x}").expect("writing to String cannot fail");
    }
    result
}
