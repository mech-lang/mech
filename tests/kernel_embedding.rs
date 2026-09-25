use mech::kernel::{Backend, Error, Kernel, KernelBuilder, Session};

#[path = "../hosts/gpu/tests/support/embedding_cases.rs"]
mod cases;

fn backends() -> Vec<Backend> {
    vec![
        Backend::Scalar,
        Backend::Simd,
        Backend::Aot,
        Backend::AotSimd,
    ]
}

fn aot_simd() -> Option<Backend> {
    Some(Backend::AotSimd)
}
