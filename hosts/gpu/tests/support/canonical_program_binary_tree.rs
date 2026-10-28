//! The full pinned Roc binary-tree algorithm, using ordinary Mech functions.
use super::artifacts;
use mech_gpu::native_artifact::JitProgramArtifact;

#[path = "canonical_program_binary_tree_differential.rs"]
mod differential;

const KERNEL: &str = concat!(
    include_str!("../fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-binary-tree.mec")
);

#[test]
fn binary_tree_search_and_skip_compile_as_separate_native_walks() {
    for call in [
        "bt-get-matches(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, maximum<u64>, nice<u64>, depth<u64>, cache<[u32]>, pointer<u64>)",
        "bt-skip-byte(tables<[u16]>, hash3<u64>, hash4<u64>, input<[u8]>, base<u64>, position<u64>, nice<u64>, depth<u64>)",
    ] {
        for artifact in artifacts(&format!("{KERNEL}\n{call}\n")).0 {
            let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                .unwrap_or_else(|error| panic!("{call}: {error:?}"));
            let plan = jit.descriptor().plan();
            assert!(plan.iterations().len() >= 4);
            let has_cache = plan.values().iter().any(|ty| matches!(
                ty.definition().body(), mech_core::SchemaBody::Matrix { element, .. }
                    if element.as_ref() == &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32)
            ));
            assert_eq!(has_cache, call.starts_with("bt-get-matches"));
        }
    }
}
