use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

const EXPECTED: &str = "{\"Cargo.toml\":\"9cedb68474e5bd916685f4fd4adc1f9db43cd83e145344b6a14e3bc0f73cb08d\",\"application.mec\":\"7d27b43f9e96bc29a3b6d42cb2abaff562cfdbe44db27b67f8dad5d188f37f2c\",\"factory-profile.json\":\"a2ff1ef171376a65ee7377a98046b68335b5258185eef65b44455a5fe8547074\",\"native-descriptor.bin\":\"619dd8e4ec0d8510072dac24744b4e57737f85ffa20e09651c94f08ad72ec970\",\"native-descriptor.json\":\"a5e2b094aa8f12cb2447fc924d5a9ae479ee07af31eaf868f8052fe323236b4e\",\"object-identity.bin\":\"b55dfab35d07196f5053f99b5f57ea75a12a71c4a07df6c3b1efdce182e9b90b\",\"program.mecb\":\"16c62437e9317eca685d8769484695339037cbaed2b6733a50e3c4de9c332f89\",\"program.o\":\"a95560c7ac2e7904a2dedcb20c67ba68a464491242229c163d2a81a8807a3394\",\"rust-toolchain.toml\":\"19219593d7dba33a75623287af94da940a93fac92eeef99e8fff9b7fd804fcf9\",\"src/main.rs\":\"8915c58fb429d396ef610618ff6b35c4e9fd3fdd72db90d427fdb7752403b3e0\"}";
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn main() {
    assert_eq!(
        std::env::var("HOST").unwrap(),
        std::env::var("TARGET").unwrap(),
        "canonical native object is host-only"
    );
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let expected: BTreeMap<String, String> = serde_json::from_str(EXPECTED).unwrap();
    let manifest_bytes = fs::read(root.join("linked-manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["schema"], "mech.canonical-linked-program-application.v1");
    let mut pins: BTreeMap<String, String> =
        serde_json::from_value(manifest["pins"].clone()).unwrap();
    let script = pins.remove("build.rs").expect("build script pin");
    assert_eq!(
        digest(&fs::read(root.join("build.rs")).unwrap()),
        script,
        "build script changed"
    );
    assert_eq!(pins, expected, "manifest generated-input pins changed");
    for (name, hash) in expected {
        assert_eq!(
            digest(&fs::read(root.join(&name)).unwrap()),
            hash,
            "changed generated input {name}"
        );
        println!("cargo:rerun-if-changed={name}");
    }
    println!("cargo:rerun-if-changed=linked-manifest.json");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg={}", root.join("program.o").display());
}
