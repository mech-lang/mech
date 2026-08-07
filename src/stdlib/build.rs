fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // The exhaustive all-feature test binaries exceed ld64's compact-
        // unwind 24-bit function-offset format. DWARF unwind information is
        // still emitted, so exception handling remains available.
        println!("cargo:rustc-link-arg-tests=-Wl,-no_compact_unwind");
    }
}
