#![cfg(feature = "embedding")]

use mech_gpu::embed::{Backend, Error, Kernel, KernelBuilder, Session};

#[path = "support/embedding_cases.rs"]
mod cases;

fn backends() -> Vec<Backend> {
    #[cfg(feature = "aot")]
    return vec![
        Backend::Scalar,
        Backend::Simd,
        Backend::Aot,
        Backend::AotSimd,
    ];
    #[cfg(not(feature = "aot"))]
    vec![Backend::Scalar, Backend::Simd]
}

fn aot_simd() -> Option<Backend> {
    #[cfg(feature = "aot")]
    return Some(Backend::AotSimd);
    #[cfg(not(feature = "aot"))]
    None
}
