fn main() {
    println!("cargo:rerun-if-env-changed=DEFLATE_MIMALLOC_OBJECT");
    let object = std::env::var("DEFLATE_MIMALLOC_OBJECT")
        .expect("DEFLATE_MIMALLOC_OBJECT must name the pinned reference allocator object");
    assert!(std::path::Path::new(&object).is_file());
    println!("cargo:rerun-if-changed={object}");
    println!("cargo:rustc-link-arg={object}");
}
