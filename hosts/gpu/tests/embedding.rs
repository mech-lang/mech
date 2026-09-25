#![cfg(feature = "embedding")]

use mech_gpu::embed::{Backend, Error, Kernel, KernelBuilder, Session};

#[path = "support/embedding_cases.rs"]
mod cases;

#[cfg(feature = "aot")]
#[path = "support/embedding_bundle_cases.rs"]
mod bundles;

fn backends() -> Vec<Backend> {
    #[cfg(feature = "aot")]
    return vec![
        Backend::Scalar,
        Backend::Simd,
        Backend::Jit,
        Backend::Aot,
        Backend::AotSimd,
    ];
    #[cfg(all(feature = "jit", not(feature = "aot")))]
    return vec![Backend::Scalar, Backend::Simd, Backend::Jit];
    #[cfg(not(feature = "jit"))]
    vec![Backend::Scalar, Backend::Simd]
}

fn aot_simd() -> Option<Backend> {
    #[cfg(feature = "aot")]
    return Some(Backend::AotSimd);
    #[cfg(not(feature = "aot"))]
    None
}
